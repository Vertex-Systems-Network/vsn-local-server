param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Stop-ProcessSafe($Process) {
    if ($null -ne $Process -and -not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $Process.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
}

$root = Join-Path $PWD 'dist-self-hosted\02.21'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0221-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$originalIpcHash = if ($hadIpcKey) { (Get-FileHash -LiteralPath $ipcKey -Algorithm SHA256).Hash } else { $null }
$agent = $null
$server = $null
$fixturePort = 0
$success = $false

if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
New-Item -ItemType Directory -Force -Path $root,$bin,$sandbox,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

function Start-Agent {
    $script:agent = Start-Process -FilePath $script:AgentExe `
        -RedirectStandardOutput (Join-Path $root 'agent.stdout.log') `
        -RedirectStandardError (Join-Path $root 'agent.stderr.log') `
        -PassThru -WindowStyle Hidden
    foreach ($i in 1..100) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { return }
        if ($script:agent.HasExited) { throw "Agent exited before readiness with code $($script:agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent did not become ready'
}

function Invoke-CliJson([string[]]$Arguments, [string]$Name) {
    $stdout = Join-Path $root "$Name.stdout.json"
    $stderr = Join-Path $root "$Name.stderr.log"
    & $script:Cli @Arguments 1> $stdout 2> $stderr
    if ($LASTEXITCODE -ne 0) {
        $detail = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
        throw "CLI $Name failed (exit=$LASTEXITCODE): $detail"
    }
    return (Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json)
}

function Invoke-CliExpectFailure([string[]]$Arguments, [string]$Name) {
    $stdout = Join-Path $root "$Name.stdout.log"
    $stderr = Join-Path $root "$Name.stderr.log"
    & $script:Cli @Arguments 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $root "$Name.exit-code.txt")
    if ($code -eq 0) { throw "CLI $Name unexpectedly succeeded" }
}

try {
    if (-not $IsWindows) { throw '02.21 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.21 certification requires a GitHub-hosted runner' }
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }
    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) {
        throw 'VSN Agent IPC port 39731 is occupied; refusing to disturb another Agent'
    }

    $previewSource = Get-Content 'crates/vsn-preview/src/lib.rs' -Raw
    foreach ($needle in @('MAX_BODY_BYTES: u64 = 512 * 1024','http://127.0.0.1','redirect(reqwest::redirect::Policy::none())','only GET/HEAD preview requests are allowed','body_base64','truncated')) {
        if (-not $previewSource.Contains($needle)) { throw "missing direct-preview invariant: $needle" }
    }
    $ipcSource = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    foreach ($needle in @('MAX_FRAME_BYTES: usize = 1024 * 1024','"preview.fetch" => Duration::from_secs(15)','fit_response_payload','encode_response_line')) {
        if (-not $ipcSource.Contains($needle)) { throw "missing IPC preview invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-preview --package vsn-ipc --all-targets -- -D warnings
    Assert-LastExit 'preview/ipc clippy failed'
    cargo test --locked --package vsn-preview --package vsn-ipc --package vsn-core
    Assert-LastExit 'preview/ipc/core tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI release build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $serverSource = Join-Path $sandbox 'preview_server.rs'
    $portFile = Join-Path $sandbox 'port.txt'
    @'
use std::{
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], extra: &str) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn response_without_length(stream: &mut TcpStream, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn main() {
    let port_file = env::args().nth(1).expect("port file");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback fixture");
    fs::write(&port_file, listener.local_addr().unwrap().port().to_string()).unwrap();
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else { continue };
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request.split_whitespace().nth(1).unwrap_or("/");
        match path {
            "/small" => response(&mut stream, "200 OK", "text/plain", b"preview-ok", "X-Test: small\r\n"),
            "/binary" => response(&mut stream, "200 OK", "application/octet-stream", &vec![0x5a; 400 * 1024], ""),
            "/large-text" => response(&mut stream, "200 OK", "text/plain; charset=utf-8", &vec![b'x'; 480 * 1024], ""),
            "/oversize" => response(&mut stream, "200 OK", "text/plain", &vec![b'y'; 600 * 1024], ""),
            "/stream-oversize" => response_without_length(&mut stream, "text/plain", &vec![b'z'; 600 * 1024]),
            "/redirect" => response(&mut stream, "302 Found", "text/plain", b"redirect", "Location: http://example.invalid/blocked\r\n"),
            "/slow" => {
                thread::sleep(Duration::from_secs(6));
                response(&mut stream, "200 OK", "text/plain", b"slow-ok", "");
            }
            _ => response(&mut stream, "404 Not Found", "text/plain", b"not-found", ""),
        }
    }
}
'@ | Set-Content -LiteralPath $serverSource -Encoding utf8

    rustc $serverSource -O -o (Join-Path $bin 'preview-server.exe')
    Assert-LastExit 'preview fixture server build failed'
    $server = Start-Process -FilePath (Join-Path $bin 'preview-server.exe') `
        -ArgumentList @($portFile) `
        -RedirectStandardOutput (Join-Path $root 'server.stdout.log') `
        -RedirectStandardError (Join-Path $root 'server.stderr.log') `
        -PassThru -WindowStyle Hidden
    foreach ($i in 1..100) {
        if (Test-Path -LiteralPath $portFile) { break }
        if ($server.HasExited) { throw 'preview fixture server exited before readiness' }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Test-Path -LiteralPath $portFile)) { throw 'preview fixture server did not publish port' }
    $fixturePort = [int](Get-Content -LiteralPath $portFile -Raw).Trim()

    Start-Agent

    $small = Invoke-CliJson @('preview','fetch',"$fixturePort",'/small') 'small'
    if ([int]$small.status -ne 200 -or [string]$small.text -ne 'preview-ok' -or $small.truncated -ne $false) {
        throw 'small text preview contract failed'
    }
    if ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$small.body_base64)) -ne 'preview-ok') {
        throw 'small preview base64 mismatch'
    }

    $binary = Invoke-CliJson @('preview','fetch',"$fixturePort",'/binary') 'binary'
    $binaryBytes = [Convert]::FromBase64String([string]$binary.body_base64)
    if ([int]$binary.status -ne 200 -or $null -ne $binary.text -or $binary.truncated -ne $false) {
        throw 'binary preview metadata failed'
    }
    if ($binaryBytes.Length -ne 400 * 1024 -or $binaryBytes[0] -ne 0x5a -or $binaryBytes[$binaryBytes.Length - 1] -ne 0x5a) {
        throw 'binary preview body mismatch'
    }

    $large = Invoke-CliJson @('preview','fetch',"$fixturePort",'/large-text') 'large-text'
    $largeBytes = [Convert]::FromBase64String([string]$large.body_base64)
    $largeOutputBytes = (Get-Item -LiteralPath (Join-Path $root 'large-text.stdout.json')).Length
    if ([int]$large.status -ne 200 -or $large.truncated -ne $false) { throw 'large text preview status failed' }
    if ($null -ne $large.text) { throw 'large text preview retained duplicate text instead of frame-safe base64-only representation' }
    if ($largeBytes.Length -ne 480 * 1024 -or $largeBytes[0] -ne [byte][char]'x' -or $largeBytes[$largeBytes.Length - 1] -ne [byte][char]'x') {
        throw 'large text preview body mismatch'
    }
    if ($largeOutputBytes -ge 900000) { throw "large text preview exceeded frame-safe CLI budget: $largeOutputBytes" }

    $streamed = Invoke-CliJson @('preview','fetch',"$fixturePort",'/stream-oversize') 'stream-oversize'
    $streamedBytes = [Convert]::FromBase64String([string]$streamed.body_base64)
    if ([int]$streamed.status -ne 200 -or $streamed.truncated -ne $true) {
        throw 'unknown-length oversized response was not bounded by truncation'
    }
    if ($null -ne $streamed.text) { throw 'truncated large text retained duplicate text' }
    if ($streamedBytes.Length -ne 512 * 1024) { throw 'truncated direct-preview body is not exactly 512 KiB' }

    Invoke-CliExpectFailure @('preview','fetch',"$fixturePort",'/oversize') 'oversize-known-length'
    Invoke-CliExpectFailure @('preview','fetch',"$fixturePort",'https://example.invalid/') 'external-target'

    $redirect = Invoke-CliJson @('preview','fetch',"$fixturePort",'/redirect') 'redirect'
    if ([int]$redirect.status -ne 302) { throw 'preview unexpectedly followed redirect away from loopback' }
    if ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$redirect.body_base64)) -ne 'redirect') {
        throw 'redirect response body mismatch'
    }

    $slowWatch = [Diagnostics.Stopwatch]::StartNew()
    $slow = Invoke-CliJson @('preview','fetch',"$fixturePort",'/slow') 'slow'
    $slowWatch.Stop()
    $slowMs = [int64]$slowWatch.ElapsedMilliseconds
    if ([int]$slow.status -ne 200 -or [string]$slow.text -ne 'slow-ok') { throw 'bounded slow preview failed' }
    if ($slowMs -lt 5500 -or $slowMs -ge 14000) { throw "slow preview timing outside bounded contract: ${slowMs}ms" }

    $chain = Invoke-CliJson @('audit','verify') 'audit'
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }
    $auditEvents = [uint64]$chain.events
    if ($auditEvents -eq 0) { throw 'audit evidence reported zero events' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    $evidence = [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task_id = '02.21'
        artifact = 'loopback-readonly-preview-fetch-windows-github-hosted'
        product_version = $candidate.product_version
        candidate_id = $candidate.candidate_id
        source_commit = $env:GITHUB_SHA
        runner_name = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        fixture_port = $fixturePort
        checks = [ordered]@{
            exact_source = $true
            github_hosted_windows = $true
            required_tests = $true
            loopback_readonly_fetch = $true
            small_text_roundtrip = $true
            binary_response = $true
            frame_safe_large_text = $true
            bounded_unknown_length_truncation = $true
            known_length_oversize_rejected = $true
            external_target_rejected = $true
            redirect_not_followed = $true
            bounded_slow_response = $true
            audit_chain_valid = $true
        }
        measurements = [ordered]@{
            binary_body_bytes = $binaryBytes.Length
            large_text_body_bytes = $largeBytes.Length
            large_text_cli_bytes = $largeOutputBytes
            truncated_body_bytes = $streamedBytes.Length
            slow_response_ms = $slowMs
            audit_events = $auditEvents
        }
    }
    $evidence | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash -LiteralPath (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() |
        Set-Content -LiteralPath (Join-Path $root 'evidence.json.sha256')
    $success = $true
}
finally {
    Stop-ProcessSafe $agent
    Stop-ProcessSafe $server
    $env:LOCALAPPDATA = $originalLocalAppData

    $ipcRestored = $false
    if ($hadIpcKey) {
        if (Test-Path -LiteralPath $ipcKey) {
            $ipcRestored = ((Get-FileHash -LiteralPath $ipcKey -Algorithm SHA256).Hash -eq $originalIpcHash)
        }
    } else {
        if (Test-Path -LiteralPath $ipcKey) {
            Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
        }
        $ipcRestored = -not (Test-Path -LiteralPath $ipcKey)
    }

    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force -ErrorAction SilentlyContinue
    }

    $agentStopped = ($null -eq $agent) -or $agent.HasExited
    $serverStopped = ($null -eq $server) -or $server.HasExited
    $localAppDataRestored = ($env:LOCALAPPDATA -eq $originalLocalAppData)
    $sandboxRemoved = -not (Test-Path -LiteralPath $sandbox)
    $ipcPortFree = -not [bool](Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue)

    [ordered]@{
        certification_completed = $success
        agent_stopped = $agentStopped
        fixture_server_stopped = $serverStopped
        localappdata_restored = $localAppDataRestored
        ipc_key_restored = $ipcRestored
        ipc_port_released = $ipcPortFree
        sandbox_removed = $sandboxRemoved
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $root 'cleanup.json') -Encoding utf8

    if ($success -and (-not $agentStopped -or -not $serverStopped -or -not $localAppDataRestored -or -not $ipcRestored -or -not $ipcPortFree -or -not $sandboxRemoved)) {
        throw '02.21 cleanup verification failed'
    }
}
