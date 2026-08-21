param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

$root = Join-Path $PWD 'dist-self-hosted\02.19'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0219-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$fixture = Join-Path $workspace 'pipe-session-fixture.exe'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null
$writerProcess = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$workspace,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

function Start-Agent {
    $script:agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput (Join-Path $root 'agent.stdout.log') -RedirectStandardError (Join-Path $root 'agent.stderr.log') -PassThru -WindowStyle Hidden
    $script:agent.Id | Set-Content (Join-Path $root 'agent.pid')
    $ready = $false
    foreach ($i in 1..80) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        if ($script:agent.HasExited) { throw "Agent exited before readiness with code $($script:agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    if (-not $ready) { throw 'Agent did not become ready' }
}

function Stop-Agent {
    if ($script:agent -and -not $script:agent.HasExited) {
        Stop-Process -Id $script:agent.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $script:agent.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
    $script:agent = $null
}

try {
    if (-not $IsWindows) { throw "02.19 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $terminal = Get-Content 'crates/vsn-terminal/src/lib.rs' -Raw
    foreach ($needle in @('MAX_SESSION_BUFFER','MAX_SESSION_READ','maximum 64 terminal sessions','pub fn start_session','pub fn write_session','pub fn read_session','pub fn read_session_wait','pub fn session_state','pub fn stop_session','pub fn remove_session','pub fn list_sessions')) {
        if (-not $terminal.Contains($needle)) { throw "missing pipe-session invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('terminal.session.start','terminal.session.write','terminal.session.read','terminal.session.status','terminal.session.stop','terminal.session.remove','terminal.session.list')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent pipe-session command: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-terminal --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'terminal clippy failed'
    cargo test --locked --package vsn-terminal --package vsn-core
    Assert-LastExit 'terminal tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $fixtureSource = Join-Path $sandbox 'pipe_session_fixture.rs'
    @'
use std::{env, io::{self, BufRead, Write}, process, thread, time::Duration};
fn main() {
    match env::args().nth(1).as_deref() {
        Some("echo") => {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                let line = line.unwrap_or_default();
                if line == "exit" { break; }
                println!("OUT:{line}");
                eprintln!("ERR:{line}");
                io::stdout().flush().ok();
                io::stderr().flush().ok();
            }
        }
        Some("idle") => thread::sleep(Duration::from_secs(30)),
        Some("burst") => {
            let out = thread::spawn(|| {
                let mut s = io::stdout().lock();
                let block = vec![b'O'; 8192];
                for _ in 0..180 { s.write_all(&block).unwrap(); }
                s.flush().unwrap();
            });
            let err = thread::spawn(|| {
                let mut s = io::stderr().lock();
                let block = vec![b'E'; 8192];
                for _ in 0..180 { s.write_all(&block).unwrap(); }
                s.flush().unwrap();
            });
            out.join().unwrap(); err.join().unwrap();
        }
        Some("block-stdin") => {
            // Intentionally never read stdin while alive. This creates pipe backpressure.
            thread::sleep(Duration::from_secs(8));
        }
        _ => process::exit(97),
    }
}
'@ | Set-Content -LiteralPath $fixtureSource -Encoding utf8
    rustc $fixtureSource -O -o $fixture
    Assert-LastExit 'pipe-session fixture build failed'

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent
    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    $echo = & $script:Cli terminal start $workspace $fixture echo | Out-String | ConvertFrom-Json
    $echo | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'echo-start.json') -Encoding utf8
    $echoId = [string]$echo.session_id
    if (-not $echoId -or $echo.running -ne $true) { throw 'echo session did not start' }

    "hello-session`n" | & $script:Cli terminal write $echoId | Set-Content (Join-Path $root 'echo-write.json') -Encoding utf8
    Assert-LastExit 'echo session stdin write failed'
    $echoChunk = & $script:Cli terminal read-wait $echoId | Out-String | ConvertFrom-Json
    $echoChunk | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'echo-read.json') -Encoding utf8
    if (-not ([string]$echoChunk.stdout).Contains('OUT:hello-session') -or -not ([string]$echoChunk.stderr).Contains('ERR:hello-session')) { throw 'pipe session did not preserve stdout/stderr interaction' }

    $listed = @(& $script:Cli terminal list | Out-String | ConvertFrom-Json)
    $listed | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'list.json') -Encoding utf8
    if (-not ($listed | Where-Object { $_.session_id -eq $echoId })) { throw 'session list omitted active echo session' }
    $echoStatus = & $script:Cli terminal status $echoId | Out-String | ConvertFrom-Json
    if ($echoStatus.running -ne $true) { throw 'active echo session status is not running' }

    $idle = & $script:Cli terminal start $workspace $fixture idle | Out-String | ConvertFrom-Json
    $idleId = [string]$idle.session_id
    $wait = [Diagnostics.Stopwatch]::StartNew()
    $idleChunk = & $script:Cli terminal read-wait $idleId | Out-String | ConvertFrom-Json
    $wait.Stop()
    $wait.ElapsedMilliseconds | Set-Content (Join-Path $root 'idle-read-wait-ms.txt')
    $idleChunk | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'idle-read-wait.json') -Encoding utf8
    if ($wait.Elapsed.TotalSeconds -lt 2.5 -or $wait.Elapsed.TotalSeconds -gt 4.5) { throw "bounded long-poll returned outside expected 3s window: $($wait.Elapsed.TotalSeconds)s" }
    if ($idleChunk.running -ne $true -or $idleChunk.stdout -or $idleChunk.stderr) { throw 'idle long-poll returned unexpected payload/state' }

    $burst = & $script:Cli terminal start $workspace $fixture burst | Out-String | ConvertFrom-Json
    $burstId = [string]$burst.session_id
    Start-Sleep -Seconds 2
    $burstChunk = & $script:Cli terminal read $burstId | Out-String | ConvertFrom-Json
    $burstChunk | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'burst-read.json') -Encoding utf8
    if ([uint64]$burstChunk.stdout_dropped_bytes -eq 0 -or [uint64]$burstChunk.stderr_dropped_bytes -eq 0) { throw 'bounded ring buffers did not report dropped bytes after oversized output' }
    if ([Text.Encoding]::UTF8.GetByteCount([string]$burstChunk.stdout) -gt 262144 -or [Text.Encoding]::UTF8.GetByteCount([string]$burstChunk.stderr) -gt 262144) { throw 'session read exceeded 256 KiB per-stream cap' }

    # Concurrency safety: a non-reading child can backpressure stdin. A write to that session
    # must not hold the global session registry lock and stall unrelated status/lifecycle calls.
    $block = & $script:Cli terminal start $workspace $fixture 'block-stdin' | Out-String | ConvertFrom-Json
    $blockId = [string]$block.session_id
    $inputPath = Join-Path $sandbox 'blocked-input.txt'
    ('Z' * (256 * 1024)) | Set-Content -LiteralPath $inputPath -NoNewline -Encoding ascii
    $writerOut = Join-Path $root 'blocked-writer.stdout'
    $writerErr = Join-Path $root 'blocked-writer.stderr'
    $command = "Get-Content -LiteralPath '$($inputPath.Replace("'","''"))' -Raw | & '$($script:Cli.Replace("'","''"))' terminal write '$($blockId.Replace("'","''"))'"
    $writerProcess = Start-Process pwsh -ArgumentList @('-NoProfile','-Command',$command) -RedirectStandardOutput $writerOut -RedirectStandardError $writerErr -PassThru -WindowStyle Hidden
    Start-Sleep -Milliseconds 500

    $statusWatch = [Diagnostics.Stopwatch]::StartNew()
    & $script:Cli terminal status $echoId 1> (Join-Path $root 'concurrent-status.stdout') 2> (Join-Path $root 'concurrent-status.stderr')
    $statusCode = $LASTEXITCODE
    $statusWatch.Stop()
    $statusCode | Set-Content (Join-Path $root 'concurrent-status.exit-code.txt')
    $statusWatch.ElapsedMilliseconds | Set-Content (Join-Path $root 'concurrent-status-ms.txt')
    if ($statusCode -ne 0) { throw 'unrelated session status was blocked/failed by backpressured stdin write' }
    if ($statusWatch.Elapsed.TotalSeconds -ge 2) { throw "unrelated session status stalled behind stdin write for $($statusWatch.Elapsed.TotalSeconds)s" }

    Wait-Process -Id $writerProcess.Id -Timeout 15 -ErrorAction SilentlyContinue
    if (-not $writerProcess.HasExited) { Stop-Process -Id $writerProcess.Id -Force -ErrorAction SilentlyContinue }
    $writerProcess = $null

    "exit`n" | & $script:Cli terminal write $echoId *> $null
    Start-Sleep -Milliseconds 250
    $stopped = & $script:Cli terminal stop $idleId | Out-String | ConvertFrom-Json
    $stopped | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'idle-stop.json') -Encoding utf8
    if ($stopped.running -ne $false) { throw 'terminal stop did not transition idle session to stopped' }
    & $script:Cli terminal remove $idleId | Set-Content (Join-Path $root 'idle-remove.json') -Encoding utf8
    Assert-LastExit 'terminal remove failed'
    $afterRemove = @(& $script:Cli terminal list | Out-String | ConvertFrom-Json)
    if ($afterRemove | Where-Object { $_.session_id -eq $idleId }) { throw 'removed session remains in list' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.19';
        artifact='persistent-pipe-terminal-sessions-windows-self-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        multi_request_interaction_verified=$true; stdout_stderr_verified=$true; bounded_wait_verified=$true;
        bounded_ring_buffer_verified=$true; lifecycle_verified=$true; cross_session_concurrency_verified=$true;
        audit_chain_valid=$true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    if ($writerProcess -and -not $writerProcess.HasExited) { Stop-Process -Id $writerProcess.Id -Force -ErrorAction SilentlyContinue }
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
