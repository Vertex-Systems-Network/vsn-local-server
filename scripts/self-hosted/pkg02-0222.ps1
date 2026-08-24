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

function Invoke-PreviewRequest($Value, [string]$Name, [bool]$ExpectSuccess = $true) {
    $stdout = Join-Path $root "$Name.stdout.json"
    $stderr = Join-Path $root "$Name.stderr.log"
    $json = $Value | ConvertTo-Json -Depth 20 -Compress
    $json | & $script:Cli preview request 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $root "$Name.exit-code.txt")
    if ($ExpectSuccess) {
        if ($code -ne 0) {
            $detail = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
            throw "preview request $Name failed (exit=$code): $detail"
        }
        return (Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json)
    }
    if ($code -eq 0) { throw "preview request $Name unexpectedly succeeded" }
    return [pscustomobject]@{
        ExitCode = $code
        Stdout = if (Test-Path -LiteralPath $stdout) { Get-Content -LiteralPath $stdout -Raw } else { '' }
        Stderr = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
    }
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

$root = Join-Path $PWD 'dist-self-hosted\02.22'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0222-' + [guid]::NewGuid().ToString('N'))
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

try {
    if (-not $IsWindows) { throw '02.22 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.22 certification requires a GitHub-hosted runner' }
    if (-not $env:EXPECTED_SHA) { throw 'EXPECTED_SHA is required for exact-source evidence binding' }
    if ((git rev-parse HEAD).Trim() -ne $env:EXPECTED_SHA) { throw '02.22 checkout does not match EXPECTED_SHA' }
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }
    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) {
        throw 'VSN Agent IPC port 39731 is occupied; refusing to disturb another Agent'
    }

    $previewSource = Get-Content 'crates/vsn-preview/src/lib.rs' -Raw
    foreach ($needle in @(
        'MAX_PROXY_REQUEST_BODY',
        'MAX_PROXY_RESPONSE_BODY',
        '"GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"',
        'allowed_request_header',
        'allowed_response_header',
        'http://127.0.0.1',
        'redirect(reqwest::redirect::Policy::none())',
        'path.starts_with("//")'
    )) {
        if (-not $previewSource.Contains($needle)) { throw "missing advanced-preview invariant: $needle" }
    }

    $ipcSource = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    foreach ($needle in @(
        'MAX_FRAME_BYTES: usize = 1024 * 1024',
        'PREVIEW_FRAME_SAFE_BODY_BYTES: usize = 480 * 1024',
        'PREVIEW_REQUEST_PARAM_BUDGET: usize = 800 * 1024',
        '"preview.request" => Duration::from_secs(23)',
        'PreviewRequestRejected',
        'validate_outbound_payload',
        'encode_request_line',
        'fit_response_payload'
    )) {
        if (-not $ipcSource.Contains($needle)) { throw "missing IPC advanced-preview invariant: $needle" }
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

    $serverSource = Join-Path $sandbox 'advanced_preview_server.rs'
    $portFile = Join-Path $sandbox 'port.txt'
    @'
use std::{
    env, fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn send(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], extra: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Secret-Internal: must-be-filtered\r\nX-Frame-Options: DENY\r\n{extra}Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn read_request(stream: &mut TcpStream) -> (String, String, usize, String) {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 8192];
    let mut header_end = None;
    let mut content_length = 0usize;
    loop {
        let n = stream.read(&mut buf).unwrap_or(0);
        if n == 0 { break; }
        bytes.extend_from_slice(&buf[..n]);
        if header_end.is_none() {
            if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                header_end = Some(pos + 4);
                let headers = String::from_utf8_lossy(&bytes[..pos + 4]).to_ascii_lowercase();
                for line in headers.lines() {
                    if let Some(value) = line.strip_prefix("content-length:") {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
            }
        }
        if let Some(end) = header_end {
            if bytes.len() >= end + content_length { break; }
        }
    }
    let end = header_end.unwrap_or(bytes.len());
    let head = String::from_utf8_lossy(&bytes[..end]).to_string();
    let first = head.lines().next().unwrap_or("GET / HTTP/1.1");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let body_len = bytes.len().saturating_sub(end);
    let x_vsn = head.lines().find_map(|line| {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("x-vsn-test:") {
            line.split_once(':').map(|v| v.1.trim().to_string())
        } else {
            None
        }
    }).unwrap_or_default();
    (method, path, body_len, x_vsn)
}

fn main() {
    let port_file = env::args().nth(1).expect("port file");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback fixture");
    fs::write(&port_file, listener.local_addr().unwrap().port().to_string()).unwrap();
    for incoming in listener.incoming() {
        let Ok(mut stream) = incoming else { continue };
        let (method, path, body_len, x_vsn) = read_request(&mut stream);
        match path.as_str() {
            "/echo" => {
                let body = format!("{{\"method\":\"{method}\",\"body_len\":{body_len},\"x_vsn_test\":\"{x_vsn}\"}}");
                send(&mut stream, "200 OK", "application/json", body.as_bytes(), "Set-Cookie: local=1\r\n");
            }
            "/large-binary" => send(&mut stream, "200 OK", "application/octet-stream", &vec![0x41; 768 * 1024], ""),
            "/slow" => {
                thread::sleep(Duration::from_secs(6));
                send(&mut stream, "200 OK", "text/plain", b"slow-advanced-ok", "");
            }
            _ => send(&mut stream, "404 Not Found", "text/plain", b"not-found", ""),
        }
    }
}
'@ | Set-Content -LiteralPath $serverSource -Encoding utf8

    rustc $serverSource -O -o (Join-Path $bin 'advanced-preview-server.exe')
    Assert-LastExit 'advanced preview fixture build failed'
    $server = Start-Process -FilePath (Join-Path $bin 'advanced-preview-server.exe') `
        -ArgumentList @($portFile) `
        -RedirectStandardOutput (Join-Path $root 'server.stdout.log') `
        -RedirectStandardError (Join-Path $root 'server.stderr.log') `
        -PassThru -WindowStyle Hidden
    foreach ($i in 1..100) {
        if (Test-Path -LiteralPath $portFile) { break }
        if ($server.HasExited) { throw 'advanced preview fixture exited before readiness' }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Test-Path -LiteralPath $portFile)) { throw 'advanced preview fixture did not publish port' }
    $fixturePort = [int](Get-Content -LiteralPath $portFile -Raw).Trim()

    Start-Agent

    $postBody = New-Object byte[] (400 * 1024)
    [Array]::Fill[byte]($postBody, [byte]0x51)
    $postRequest = [ordered]@{
        port = $fixturePort
        path = '/echo'
        method = 'POST'
        headers = [ordered]@{'content-type'='application/octet-stream';'x-vsn-test'='alpha'}
        body_base64 = [Convert]::ToBase64String($postBody)
    }
    $post = Invoke-PreviewRequest $postRequest 'post-small'
    if ([int]$post.status -ne 200 -or $post.truncated -ne $false) { throw 'bounded POST status contract failed' }
    $postEcho = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$post.body_base64)) | ConvertFrom-Json
    if ([string]$postEcho.method -ne 'POST' -or [int]$postEcho.body_len -ne $postBody.Length -or [string]$postEcho.x_vsn_test -ne 'alpha') {
        throw 'POST method/body/header forwarding contract failed'
    }
    if ($post.headers.PSObject.Properties.Name -contains 'x-secret-internal') { throw 'unapproved response header leaked through preview proxy' }
    if (-not ($post.headers.PSObject.Properties.Name -contains 'x-frame-options')) { throw 'approved response security header was lost' }
    if (-not ($post.headers.PSObject.Properties.Name -contains 'set-cookie')) { throw 'approved Set-Cookie response header was lost' }

    $deleteRequest = [ordered]@{port=$fixturePort;path='/echo';method='DELETE';headers=@{};body_base64=$null}
    $delete = Invoke-PreviewRequest $deleteRequest 'delete'
    $deleteEcho = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$delete.body_base64)) | ConvertFrom-Json
    if ([string]$deleteEcho.method -ne 'DELETE') { throw 'DELETE method was not preserved' }

    $badHeader = [ordered]@{port=$fixturePort;path='/echo';method='GET';headers=[ordered]@{'host'='example.invalid'};body_base64=$null}
    $badHeaderFailure = Invoke-PreviewRequest $badHeader 'bad-header' $false
    if ($badHeaderFailure.Stderr -match 'frame exceeds maximum size') { throw 'disallowed header leaked into lower-level IPC frame error' }

    $getBody = [ordered]@{port=$fixturePort;path='/echo';method='GET';headers=@{};body_base64=[Convert]::ToBase64String([byte[]](1,2,3))}
    $getBodyFailure = Invoke-PreviewRequest $getBody 'get-body' $false
    if ($getBodyFailure.Stderr -match 'frame exceeds maximum size') { throw 'GET body rejection leaked into lower-level IPC frame error' }

    $external = [ordered]@{port=$fixturePort;path='//example.invalid/x';method='POST';headers=@{};body_base64=$null}
    $externalFailure = Invoke-PreviewRequest $external 'external-target' $false
    if ($externalFailure.Stderr -match 'frame exceeds maximum size') { throw 'external target rejection leaked into lower-level IPC frame error' }

    $tooLarge = New-Object byte[] (768 * 1024)
    [Array]::Fill[byte]($tooLarge, [byte]0x52)
    $tooLargeRequest = [ordered]@{port=$fixturePort;path='/echo';method='POST';headers=@{};body_base64=[Convert]::ToBase64String($tooLarge)}
    $tooLargeFailure = Invoke-PreviewRequest $tooLargeRequest 'oversize-request' $false
    if ($tooLargeFailure.Stderr -match 'frame exceeds maximum size') { throw 'oversized preview request leaked a lower-level IPC frame overflow' }
    if ($tooLargeFailure.Stderr -notmatch 'preview request rejected') { throw 'oversized preview request did not fail at preview validation boundary' }

    $largeRequest = [ordered]@{port=$fixturePort;path='/large-binary';method='GET';headers=@{};body_base64=$null}
    $large = Invoke-PreviewRequest $largeRequest 'large-response'
    $largeBytes = [Convert]::FromBase64String([string]$large.body_base64)
    $largeOutputBytes = (Get-Item -LiteralPath (Join-Path $root 'large-response.stdout.json')).Length
    if ([int]$large.status -ne 200 -or $large.truncated -ne $true) { throw 'large advanced preview response was not bounded by truncation' }
    if ($null -ne $large.text) { throw 'large advanced preview response retained duplicate text' }
    if ($largeBytes.Length -ne 480 * 1024) { throw "advanced preview response body is not exactly 480 KiB: $($largeBytes.Length)" }
    if ($largeOutputBytes -ge 900000) { throw "advanced preview response exceeded frame-safe CLI budget: $largeOutputBytes" }
    if ($largeBytes[0] -ne 0x41 -or $largeBytes[$largeBytes.Length - 1] -ne 0x41) { throw 'large advanced preview body mismatch' }

    $slowRequest = [ordered]@{port=$fixturePort;path='/slow';method='GET';headers=@{};body_base64=$null}
    $slowWatch = [Diagnostics.Stopwatch]::StartNew()
    $slow = Invoke-PreviewRequest $slowRequest 'slow'
    $slowWatch.Stop()
    $slowMs = [int64]$slowWatch.ElapsedMilliseconds
    if ([int]$slow.status -ne 200) { throw 'bounded slow advanced preview failed' }
    $slowText = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$slow.body_base64))
    if ($slowText -ne 'slow-advanced-ok') { throw 'slow advanced preview body mismatch' }
    if ($slowMs -lt 5500 -or $slowMs -ge 22000) { throw "slow advanced preview timing outside bounded contract: ${slowMs}ms" }

    $chain = Invoke-CliJson @('audit','verify') 'audit'
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }
    $auditEvents = [uint64]$chain.events
    if ($auditEvents -eq 0) { throw 'audit evidence reported zero events' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    $evidence = [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task_id = '02.22'
        artifact = 'advanced-loopback-preview-requests-windows-github-hosted'
        product_version = $candidate.product_version
        candidate_id = $candidate.candidate_id
        source_commit = $env:EXPECTED_SHA
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
            allowed_mutation_methods = $true
            request_body_bounded = $true
            response_body_bounded = $true
            request_header_filter = $true
            response_header_filter = $true
            loopback_only_boundary = $true
            oversized_request_preflight_rejected = $true
            large_response_frame_safe = $true
            bounded_slow_response = $true
            audit_chain_valid = $true
        }
        measurements = [ordered]@{
            post_body_bytes = $postBody.Length
            large_response_body_bytes = $largeBytes.Length
            large_response_cli_bytes = $largeOutputBytes
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
        throw '02.22 cleanup verification failed'
    }
}
