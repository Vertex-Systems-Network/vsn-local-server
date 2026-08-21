param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Invoke-PreviewRequest($Value, [string]$Stdout, [string]$Stderr) {
    $json = $Value | ConvertTo-Json -Depth 12 -Compress
    $json | & $script:Cli preview request 1> $Stdout 2> $Stderr
    return $LASTEXITCODE
}

$root = Join-Path $PWD 'dist-self-hosted\02.22'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0222-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null
$server = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$isolatedLocalAppData | Out-Null
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
    if (-not $IsWindows) { throw '02.22 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.22 certification requires a GitHub-hosted runner' }
    Write-Host "runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $preview = Get-Content 'crates/vsn-preview/src/lib.rs' -Raw
    foreach ($needle in @('MAX_PROXY_REQUEST_BODY','MAX_PROXY_RESPONSE_BODY','allowed_request_header','allowed_response_header','POST','PUT','PATCH','DELETE','OPTIONS','http://127.0.0.1')) {
        if (-not $preview.Contains($needle)) { throw "missing advanced preview invariant: $needle" }
    }
    $ipc = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    if (-not $ipc.Contains('MAX_FRAME_BYTES: usize = 1024 * 1024')) { throw '02.22 acceptance expects the current 1 MiB IPC frame contract' }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-preview --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'advanced preview clippy failed'
    cargo test --locked --package vsn-preview --package vsn-core
    Assert-LastExit 'advanced preview tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $serverSource = Join-Path $sandbox 'advanced_preview_server.rs'
    $portFile = Join-Path $sandbox 'port.txt'
    @'
use std::{env, fs, io::{Read, Write}, net::{TcpListener, TcpStream}};
fn send(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], extra: &str) {
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Secret-Internal: must-be-filtered\r\nX-Frame-Options: DENY\r\n{extra}Connection: close\r\n\r\n", body.len());
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
}
fn main() {
    let port_file = env::args().nth(1).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    fs::write(&port_file, listener.local_addr().unwrap().port().to_string()).unwrap();
    for incoming in listener.incoming() {
        let mut stream = incoming.unwrap();
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
        let head = String::from_utf8_lossy(&bytes[..end]);
        let first = head.lines().next().unwrap_or("GET / HTTP/1.1");
        let mut parts = first.split_whitespace();
        let method = parts.next().unwrap_or("GET");
        let path = parts.next().unwrap_or("/");
        let body_len = bytes.len().saturating_sub(end);
        let x_vsn = head.lines().find_map(|line| line.to_ascii_lowercase().strip_prefix("x-vsn-test:").map(|_| line.split_once(':').map(|v| v.1.trim().to_string()).unwrap_or_default())).unwrap_or_default();
        match path {
            "/echo" => {
                let body = format!("{{\"method\":\"{method}\",\"body_len\":{body_len},\"x_vsn_test\":\"{x_vsn}\"}}");
                send(&mut stream, "200 OK", "application/json", body.as_bytes(), "Set-Cookie: local=1\r\n");
            }
            "/large-binary" => send(&mut stream, "200 OK", "application/octet-stream", &vec![0x41; 768 * 1024], ""),
            _ => send(&mut stream, "404 Not Found", "text/plain", b"not-found", ""),
        }
    }
}
'@ | Set-Content -LiteralPath $serverSource -Encoding utf8
    rustc $serverSource -O -o (Join-Path $bin 'advanced-preview-server.exe')
    Assert-LastExit 'advanced preview server build failed'
    $server = Start-Process -FilePath (Join-Path $bin 'advanced-preview-server.exe') -ArgumentList @($portFile) -RedirectStandardOutput (Join-Path $root 'server.stdout.log') -RedirectStandardError (Join-Path $root 'server.stderr.log') -PassThru -WindowStyle Hidden
    foreach ($i in 1..80) {
        if (Test-Path -LiteralPath $portFile) { break }
        if ($server.HasExited) { throw 'advanced preview server exited before readiness' }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Test-Path -LiteralPath $portFile)) { throw 'advanced preview server did not publish port' }
    $port = [int](Get-Content $portFile -Raw).Trim()

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb another Agent' }
    Start-Agent

    $smallBody = New-Object byte[] (400 * 1024)
    [Array]::Fill[byte]($smallBody, [byte]0x51)
    $smallRequest = [ordered]@{
        port=$port; path='/echo'; method='POST';
        headers=[ordered]@{'content-type'='application/octet-stream';'x-vsn-test'='alpha'};
        body_base64=[Convert]::ToBase64String($smallBody)
    }
    $smallOut = Join-Path $root 'post-small.json'
    $smallErr = Join-Path $root 'post-small.stderr'
    if ((Invoke-PreviewRequest $smallRequest $smallOut $smallErr) -ne 0) { throw 'bounded POST preview failed' }
    $small = Get-Content $smallOut -Raw | ConvertFrom-Json
    $echo = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$small.body_base64)) | ConvertFrom-Json
    if ([string]$echo.method -ne 'POST' -or [int]$echo.body_len -ne $smallBody.Length -or [string]$echo.x_vsn_test -ne 'alpha') { throw 'POST method/body/header forwarding contract failed' }
    if ($small.headers.PSObject.Properties.Name -contains 'x-secret-internal') { throw 'unapproved response header leaked through preview proxy' }
    if (-not ($small.headers.PSObject.Properties.Name -contains 'x-frame-options')) { throw 'approved response security header was lost' }

    $deleteRequest = [ordered]@{port=$port;path='/echo';method='DELETE';headers=@{};body_base64=$null}
    $deleteOut = Join-Path $root 'delete.json'
    if ((Invoke-PreviewRequest $deleteRequest $deleteOut (Join-Path $root 'delete.stderr')) -ne 0) { throw 'allowed DELETE preview failed' }
    $delete = Get-Content $deleteOut -Raw | ConvertFrom-Json
    $deleteEcho = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String([string]$delete.body_base64)) | ConvertFrom-Json
    if ([string]$deleteEcho.method -ne 'DELETE') { throw 'DELETE method was not preserved' }

    $badHeader = [ordered]@{port=$port;path='/echo';method='GET';headers=[ordered]@{'host'='example.invalid'};body_base64=$null}
    if ((Invoke-PreviewRequest $badHeader (Join-Path $root 'bad-header.stdout') (Join-Path $root 'bad-header.stderr')) -eq 0) { throw 'disallowed Host header unexpectedly passed' }
    $getBody = [ordered]@{port=$port;path='/echo';method='GET';headers=@{};body_base64=[Convert]::ToBase64String([byte[]](1,2,3))}
    if ((Invoke-PreviewRequest $getBody (Join-Path $root 'get-body.stdout') (Join-Path $root 'get-body.stderr')) -eq 0) { throw 'GET request body unexpectedly passed' }
    $external = [ordered]@{port=$port;path='//example.invalid/x';method='POST';headers=@{};body_base64=$null}
    if ((Invoke-PreviewRequest $external (Join-Path $root 'external.stdout') (Join-Path $root 'external.stderr')) -eq 0) { throw 'network-path preview target unexpectedly passed' }

    # Payloads that cannot fit the authenticated 1 MiB request frame must fail at the preview
    # validation boundary rather than leaking a lower-level frame overflow from IPC transport.
    $tooLargeForFrame = New-Object byte[] (768 * 1024)
    $frameRequest = [ordered]@{port=$port;path='/echo';method='POST';headers=@{};body_base64=[Convert]::ToBase64String($tooLargeForFrame)}
    $frameErr = Join-Path $root 'frame-request.stderr'
    $frameCode = Invoke-PreviewRequest $frameRequest (Join-Path $root 'frame-request.stdout') $frameErr
    if ($frameCode -eq 0) { throw 'request exceeding frame-safe preview budget unexpectedly succeeded' }
    $frameErrorText = Get-Content $frameErr -Raw
    if ($frameErrorText -match 'frame exceeds maximum size') { throw 'preview request body limit is not aligned with IPC frame budget' }

    # Likewise, an oversized loopback response must be truncated/bounded before serialization.
    $largeRequest = [ordered]@{port=$port;path='/large-binary';method='GET';headers=@{};body_base64=$null}
    $largeOut = Join-Path $root 'large-response.json'
    $largeErr = Join-Path $root 'large-response.stderr'
    $largeCode = Invoke-PreviewRequest $largeRequest $largeOut $largeErr
    if ($largeCode -ne 0) { throw 'large loopback response must be bounded before IPC serialization' }
    if ((Get-Item $largeOut).Length -ge 900000) { throw 'advanced preview response exceeded frame-safe serialized budget' }
    $large = Get-Content $largeOut -Raw | ConvertFrom-Json
    if ($large.truncated -ne $true) { throw 'large advanced preview response did not report truncation' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.22';
        artifact='advanced-loopback-preview-requests-windows-github-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_environment=$env:RUNNER_ENVIRONMENT;
        allowed_mutation_methods_verified=$true; request_header_filter_verified=$true; response_header_filter_verified=$true;
        loopback_boundary_verified=$true; frame_safe_request_limit_verified=$true; frame_safe_response_limit_verified=$true;
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
