param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

$root = Join-Path $PWD 'dist-self-hosted\02.21'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0221-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null
$server = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$workspace,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

function Start-Agent {
    $script:agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput (Join-Path $root 'agent.stdout.log') -RedirectStandardError (Join-Path $root 'agent.stderr.log') -PassThru -WindowStyle Hidden
    foreach ($i in 1..80) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { return }
        if ($script:agent.HasExited) { throw "Agent exited before readiness with code $($script:agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent did not become ready'
}

function Stop-ProcessSafe($Process) {
    if ($Process -and -not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $Process.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
}

try {
    if (-not $IsWindows) { throw '02.21 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.21 certification requires a GitHub-hosted runner' }
    Write-Host "runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $previewSource = Get-Content 'crates/vsn-preview/src/lib.rs' -Raw
    foreach ($needle in @('MAX_BODY_BYTES','http://127.0.0.1','redirect(reqwest::redirect::Policy::none())','only GET/HEAD preview requests are allowed','body_base64','truncated')) {
        if (-not $previewSource.Contains($needle)) { throw "missing preview invariant: $needle" }
    }
    $ipcSource = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    if (-not $ipcSource.Contains('MAX_FRAME_BYTES: usize = 1024 * 1024')) { throw '02.21 acceptance expects the current 1 MiB IPC frame contract' }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-preview --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'preview clippy failed'
    cargo test --locked --package vsn-preview --package vsn-core
    Assert-LastExit 'preview tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $serverSource = Join-Path $sandbox 'preview_server.rs'
    $portFile = Join-Path $sandbox 'port.txt'
    @'
use std::{env, fs, io::{Read, Write}, net::{TcpListener, TcpStream}};
fn response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], extra: &str) {
    let header = format!("HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n", body.len());
    stream.write_all(header.as_bytes()).unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
}
fn main() {
    let port_file = env::args().nth(1).expect("port file");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    fs::write(&port_file, listener.local_addr().unwrap().port().to_string()).unwrap();
    for incoming in listener.incoming() {
        let mut stream = incoming.unwrap();
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let path = request.split_whitespace().nth(1).unwrap_or("/");
        match path {
            "/small" => response(&mut stream, "200 OK", "text/plain", b"preview-ok", "X-Test: small\r\n"),
            "/binary" => response(&mut stream, "200 OK", "application/octet-stream", &vec![0x5a; 400 * 1024], ""),
            "/large-text" => response(&mut stream, "200 OK", "text/plain; charset=utf-8", &vec![b'x'; 480 * 1024], ""),
            "/oversize" => response(&mut stream, "200 OK", "text/plain", &vec![b'y'; 600 * 1024], ""),
            "/redirect" => response(&mut stream, "302 Found", "text/plain", b"redirect", "Location: http://example.invalid/blocked\r\n"),
            _ => response(&mut stream, "404 Not Found", "text/plain", b"not-found", ""),
        }
    }
}
'@ | Set-Content -LiteralPath $serverSource -Encoding utf8
    rustc $serverSource -O -o (Join-Path $bin 'preview-server.exe')
    Assert-LastExit 'preview fixture server build failed'
    $server = Start-Process -FilePath (Join-Path $bin 'preview-server.exe') -ArgumentList @($portFile) -RedirectStandardOutput (Join-Path $root 'server.stdout.log') -RedirectStandardError (Join-Path $root 'server.stderr.log') -PassThru -WindowStyle Hidden
    foreach ($i in 1..80) {
        if (Test-Path -LiteralPath $portFile) { break }
        if ($server.HasExited) { throw 'preview fixture server exited before readiness' }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Test-Path -LiteralPath $portFile)) { throw 'preview fixture server did not publish port' }
    $port = [int](Get-Content $portFile -Raw).Trim()

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb another Agent' }
    Start-Agent

    $small = & $script:Cli preview fetch $port /small | Out-String | ConvertFrom-Json
    $small | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'small.json') -Encoding utf8
    if ([int]$small.status -ne 200 -or [string]$small.text -ne 'preview-ok' -or $small.truncated -ne $false) { throw 'small text preview contract failed' }
    if ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$small.body_base64)) -ne 'preview-ok') { throw 'small preview base64 mismatch' }

    $binary = & $script:Cli preview fetch $port /binary | Out-String | ConvertFrom-Json
    $binary | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $root 'binary.json') -Encoding utf8
    if ([int]$binary.status -ne 200 -or $null -ne $binary.text -or $binary.truncated -ne $false) { throw 'binary preview metadata failed' }
    if ([Convert]::FromBase64String([string]$binary.body_base64).Length -ne 400 * 1024) { throw 'binary preview length mismatch' }

    $redirect = & $script:Cli preview fetch $port /redirect | Out-String | ConvertFrom-Json
    $redirect | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $root 'redirect.json') -Encoding utf8
    if ([int]$redirect.status -ne 302) { throw 'preview unexpectedly followed redirect away from loopback' }

    # A large text response must remain representable inside the authenticated 1 MiB IPC frame.
    # Keeping both a full base64 body and a full duplicate text string exceeds that budget.
    $largeOut = Join-Path $root 'large-text.json'
    $largeErr = Join-Path $root 'large-text.stderr'
    & $script:Cli preview fetch $port /large-text 1> $largeOut 2> $largeErr
    $largeCode = $LASTEXITCODE
    $largeCode | Set-Content (Join-Path $root 'large-text.exit-code.txt')
    if ($largeCode -ne 0) { throw 'bounded large text preview must remain representable through IPC' }
    if ((Get-Item $largeOut).Length -ge 900000) { throw 'large text preview exceeded frame-safe serialized response budget' }
    $large = Get-Content $largeOut -Raw | ConvertFrom-Json
    if ([int]$large.status -ne 200) { throw 'large text preview returned unexpected status' }

    & $script:Cli preview fetch $port /oversize 1> (Join-Path $root 'oversize.stdout') 2> (Join-Path $root 'oversize.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'Content-Length above 512 KiB must be rejected for direct preview fetch' }
    & $script:Cli preview fetch $port 'https://example.invalid/' 1> (Join-Path $root 'external.stdout') 2> (Join-Path $root 'external.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'external URL preview path unexpectedly succeeded' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.21';
        artifact='loopback-readonly-preview-fetch-windows-github-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_environment=$env:RUNNER_ENVIRONMENT;
        loopback_only_verified=$true; redirect_not_followed=$true; binary_response_verified=$true;
        frame_safe_text_response_verified=$true; oversized_response_rejected=$true; external_target_rejected=$true;
        audit_chain_valid=$true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    Stop-ProcessSafe $agent
    Stop-ProcessSafe $server
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
