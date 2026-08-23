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
$outside = Join-Path $sandbox 'outside'
$fixture = Join-Path $workspace 'pipe-session-fixture.exe'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$ipcKeyHashBefore = if ($hadIpcKey) { (Get-FileHash $ipcKey -Algorithm SHA256).Hash } else { $null }
$agent = $null
$writerProcess = $null
$outsideLink = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$workspace,$outside,$isolatedLocalAppData | Out-Null
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
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.19 certification requires a GitHub-hosted runner' }
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $actual = (git rev-parse HEAD).Trim()
    if ($env:EXPECTED_SHA -and $actual -ne $env:EXPECTED_SHA) { throw "02.19 source binding mismatch: expected=$env:EXPECTED_SHA actual=$actual" }
    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }
    Write-Host "source=$actual runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH ipc=127.0.0.1:39731"

    $terminal = Get-Content 'crates/vsn-terminal/src/lib.rs' -Raw
    foreach ($needle in @('MAX_SESSION_BUFFER','MAX_SESSION_READ','maximum 64 terminal sessions','stdin: Arc<Mutex<ChildStdin>>','Arc::clone(&s.stdin)','pub fn start_session','pub fn write_session','pub fn read_session','pub fn read_session_wait','pub fn session_state','pub fn stop_session','pub fn remove_session','pub fn list_sessions')) {
        if (-not $terminal.Contains($needle)) { throw "missing pipe-session invariant: $needle" }
    }
    $ipc = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    foreach ($needle in @('terminal.session.read-wait','Duration::from_secs(7)','client_response_timeout')) {
        if (-not $ipc.Contains($needle)) { throw "missing pipe-session IPC invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('terminal.session.start','terminal.session.write','terminal.session.read','terminal.session.read-wait','terminal.session.status','terminal.session.stop','terminal.session.remove','terminal.session.list')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent pipe-session command: $needle" }
    }
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw
    foreach ($needle in @('sub == "start"','sub == "write"','sub == "read"','sub == "read-wait"','sub == "status"','sub == "stop"','sub == "remove"','sub == "list"')) {
        if (-not $cliSource.Contains($needle)) { throw "missing CLI pipe-session surface: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-terminal --package vsn-ipc --all-targets -- -D warnings
    Assert-LastExit 'terminal/ipc clippy failed'
    cargo test --locked --package vsn-terminal --package vsn-ipc --package vsn-core
    Assert-LastExit 'terminal/ipc/core tests failed'
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
            out.join().unwrap();
            err.join().unwrap();
        }
        Some("block-stdin") => {
            // Deliberately do not read stdin. A 256 KiB write should backpressure the pipe.
            thread::sleep(Duration::from_secs(8));
        }
        _ => process::exit(97),
    }
}
'@ | Set-Content -LiteralPath $fixtureSource -Encoding utf8
    rustc $fixtureSource -O -o $fixture
    Assert-LastExit 'pipe-session fixture build failed'

    Start-Agent
    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    # Workspace and program containment must remain fail-closed for persistent sessions.
    & $script:Cli terminal start $outside $fixture echo 1> (Join-Path $root 'outside.stdout') 2> (Join-Path $root 'outside.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'outside-workspace session cwd unexpectedly succeeded' }
    $outsideLink = Join-Path $workspace 'outside-link'
    New-Item -ItemType Junction -Path $outsideLink -Target $outside | Out-Null
    & $script:Cli terminal start $outsideLink $fixture echo 1> (Join-Path $root 'junction.stdout') 2> (Join-Path $root 'junction.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'junction outside-workspace session cwd unexpectedly succeeded' }
    Remove-Item -LiteralPath $outsideLink -Force
    $outsideLink = $null
    $outsideProgram = Join-Path $outside 'outside-program.exe'
    Copy-Item -LiteralPath $fixture -Destination $outsideProgram
    & $script:Cli terminal start $workspace $outsideProgram echo 1> (Join-Path $root 'outside-program.stdout') 2> (Join-Path $root 'outside-program.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'absolute session program outside workspace unexpectedly succeeded' }

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
    $echoStatus | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'echo-status.json') -Encoding utf8
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

    # A blocked write to one session must not hold the global registry lock.
    $block = & $script:Cli terminal start $workspace $fixture 'block-stdin' | Out-String | ConvertFrom-Json
    $blockId = [string]$block.session_id
    $inputPath = Join-Path $sandbox 'blocked-input.txt'
    ('Z' * (256 * 1024)) | Set-Content -LiteralPath $inputPath -NoNewline -Encoding ascii
    $writerOut = Join-Path $root 'blocked-writer.stdout'
    $writerErr = Join-Path $root 'blocked-writer.stderr'
    $command = "Get-Content -LiteralPath '$($inputPath.Replace("'","''"))' -Raw | & '$($script:Cli.Replace("'","''"))' terminal write '$($blockId.Replace("'","''"))'"
    $writerProcess = Start-Process pwsh -ArgumentList @('-NoProfile','-Command',$command) -RedirectStandardOutput $writerOut -RedirectStandardError $writerErr -PassThru -WindowStyle Hidden
    Start-Sleep -Milliseconds 500
    if ($writerProcess.HasExited) { throw 'backpressure fixture writer exited before concurrency assertion; test is not valid' }

    $statusWatch = [Diagnostics.Stopwatch]::StartNew()
    & $script:Cli terminal status $echoId 1> (Join-Path $root 'concurrent-status.stdout') 2> (Join-Path $root 'concurrent-status.stderr')
    $statusCode = $LASTEXITCODE
    $statusWatch.Stop()
    $statusCode | Set-Content (Join-Path $root 'concurrent-status.exit-code.txt')
    $statusWatch.ElapsedMilliseconds | Set-Content (Join-Path $root 'concurrent-status-ms.txt')
    if ($statusCode -ne 0) { throw 'unrelated session status failed while another stdin write was backpressured' }
    if ($statusWatch.Elapsed.TotalSeconds -ge 2) { throw "unrelated session status stalled behind stdin write for $($statusWatch.Elapsed.TotalSeconds)s" }

    Wait-Process -Id $writerProcess.Id -Timeout 15 -ErrorAction SilentlyContinue
    if (-not $writerProcess.HasExited) { Stop-Process -Id $writerProcess.Id -Force -ErrorAction SilentlyContinue }
    $writerProcess = $null

    $stopped = & $script:Cli terminal stop $idleId | Out-String | ConvertFrom-Json
    $stopped | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'idle-stop.json') -Encoding utf8
    if ($stopped.running -ne $false) { throw 'terminal stop did not transition idle session to stopped' }
    & $script:Cli terminal remove $idleId | Set-Content (Join-Path $root 'idle-remove.json') -Encoding utf8
    Assert-LastExit 'idle terminal remove failed'

    "exit`n" | & $script:Cli terminal write $echoId *> $null
    Start-Sleep -Milliseconds 250
    & $script:Cli terminal remove $echoId | Set-Content (Join-Path $root 'echo-remove.json') -Encoding utf8
    Assert-LastExit 'echo terminal remove failed'
    & $script:Cli terminal remove $burstId | Set-Content (Join-Path $root 'burst-remove.json') -Encoding utf8
    Assert-LastExit 'burst terminal remove failed'
    & $script:Cli terminal remove $blockId | Set-Content (Join-Path $root 'block-remove.json') -Encoding utf8
    Assert-LastExit 'block terminal remove failed'

    $afterRemove = @(& $script:Cli terminal list | Out-String | ConvertFrom-Json)
    $afterRemove | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'list-after-remove.json') -Encoding utf8
    foreach ($id in @($echoId,$idleId,$burstId,$blockId)) {
        if ($afterRemove | Where-Object { $_.session_id -eq $id }) { throw "removed session remains in list: $id" }
    }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task_id = '02.19'
        artifact = 'persistent-pipe-terminal-sessions-windows-github-hosted'
        product_version = $candidate.product_version
        candidate_id = $candidate.candidate_id
        source_commit = $actual
        runner_name = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        checks = [ordered]@{
            start_write_read_wait_verified = $true
            status_list_verified = $true
            bounded_long_poll_verified = $true
            bounded_output_verified = $true
            dropped_bytes_verified = $true
            cross_session_responsiveness_verified = $true
            workspace_cwd_containment_verified = $true
            junction_cwd_containment_verified = $true
            outside_program_rejected = $true
            stop_remove_verified = $true
            audit_chain_valid = $true
        }
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    if ($writerProcess -and -not $writerProcess.HasExited) {
        Stop-Process -Id $writerProcess.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $writerProcess.Id -Timeout 5 -ErrorAction SilentlyContinue
    }
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if ($outsideLink -and (Test-Path -LiteralPath $outsideLink)) {
        Remove-Item -LiteralPath $outsideLink -Force -ErrorAction SilentlyContinue
    }
    if ($hadIpcKey) {
        if (-not (Test-Path -LiteralPath $ipcKey)) { throw 'pre-existing IPC key disappeared during 02.19 certification' }
        $ipcKeyHashAfter = (Get-FileHash $ipcKey -Algorithm SHA256).Hash
        if ($ipcKeyHashAfter -ne $ipcKeyHashBefore) { throw 'pre-existing IPC key changed during 02.19 certification' }
    } elseif (Test-Path -LiteralPath $ipcKey) {
        Remove-Item -LiteralPath $ipcKey -Force
    }
    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force
    }
    if (Test-Path -LiteralPath $sandbox) { throw '02.19 sandbox cleanup failed' }
    [ordered]@{
        agent_stopped = $true
        localappdata_restored = ($env:LOCALAPPDATA -eq $originalLocalAppData)
        ipc_key_restored = $true
        sandbox_removed = $true
    } | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $root 'cleanup.json') -Encoding utf8
}
