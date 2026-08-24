param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

$root = Join-Path $PWD 'dist-self-hosted\02.20'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0220-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$outside = Join-Path $sandbox 'outside'
$fixture = Join-Path $workspace 'pty-fixture.exe'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$ipcKeyHashBefore = if ($hadIpcKey) { (Get-FileHash $ipcKey -Algorithm SHA256).Hash } else { $null }
$agent = $null
$script:Cli = $null
$script:AgentExe = $null
$outsideLink = $null
$sessionIds = [System.Collections.Generic.List[string]]::new()
$sessionsCleaned = $false
$script:PtyDsrQuery = "$([char]27)[6n"
$script:PtyDsrResponse = "$([char]27)[1;1R"

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

function Write-PtyRaw([string]$SessionId, [string]$InputText) {
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $script:Cli
    $info.ArgumentList.Add('terminal')
    $info.ArgumentList.Add('pty-write')
    $info.ArgumentList.Add($SessionId)
    $info.UseShellExecute = $false
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.CreateNoWindow = $true
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    if (-not $process.Start()) { throw 'failed to start raw PTY writer' }
    $process.StandardInput.Write($InputText)
    $process.StandardInput.Close()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "raw PTY write failed with exit $($process.ExitCode): $stderr"
    }
    return $stdout
}

function Read-PtyUntilMarker([string]$SessionId, [string]$Marker, [int]$TimeoutMs) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $output = ''
    $dsrResponses = 0
    $lastChunk = $null
    while ($watch.ElapsedMilliseconds -lt $TimeoutMs) {
        $remaining = [Math]::Max(1, $TimeoutMs - [int]$watch.ElapsedMilliseconds)
        $slice = [Math]::Min(1000, $remaining)
        $lastChunk = & $script:Cli terminal pty-read-wait $SessionId $slice | Out-String | ConvertFrom-Json
        Assert-LastExit 'PTY marker read failed'
        $text = [string]$lastChunk.output
        $output += $text
        if ($text.Contains($script:PtyDsrQuery)) {
            Write-PtyRaw $SessionId $script:PtyDsrResponse | Out-Null
            $dsrResponses++
        }
        if ($output.Contains($Marker) -or $lastChunk.running -ne $true) { break }
    }
    $watch.Stop()
    return [pscustomobject]@{
        output = $output
        dsr_responses = $dsrResponses
        elapsed_ms = [uint64]$watch.ElapsedMilliseconds
        running = if ($lastChunk) { [bool]$lastChunk.running } else { $false }
        exit_code = if ($lastChunk) { $lastChunk.exit_code } else { $null }
    }
}

function Initialize-PtyHost([string]$SessionId, [int]$TimeoutMs = 3000) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $output = ''
    $dsrResponses = 0
    $running = $true
    while ($watch.ElapsedMilliseconds -lt $TimeoutMs) {
        $chunk = & $script:Cli terminal pty-read-wait $SessionId 250 | Out-String | ConvertFrom-Json
        Assert-LastExit 'PTY host initialization read failed'
        $running = [bool]$chunk.running
        $text = [string]$chunk.output
        $output += $text
        if ($text.Contains($script:PtyDsrQuery)) {
            Write-PtyRaw $SessionId $script:PtyDsrResponse | Out-Null
            $dsrResponses++
            continue
        }
        if (-not $text -or -not $running) { break }
    }
    $watch.Stop()
    return [pscustomobject]@{
        output = $output
        dsr_responses = $dsrResponses
        elapsed_ms = [uint64]$watch.ElapsedMilliseconds
        running = $running
    }
}

try {
    if (-not $IsWindows) { throw '02.20 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.20 certification requires a GitHub-hosted runner' }
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $actual = (git rev-parse HEAD).Trim()
    if (-not $env:EXPECTED_SHA) { throw 'EXPECTED_SHA is required for exact-head 02.20 certification' }
    if ($actual -ne $env:EXPECTED_SHA) { throw "02.20 source binding mismatch: expected=$env:EXPECTED_SHA actual=$actual" }
    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }
    Write-Host "source=$actual runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH ipc=127.0.0.1:39731"

    $terminal = Get-Content 'crates/vsn-terminal/src/lib.rs' -Raw
    foreach ($needle in @(
        'start_pty_session_with_scrollback',
        'write_pty_session',
        'read_pty_session_wait',
        'resize_pty_session',
        'stop_pty_session',
        'remove_pty_session',
        'list_pty_recovery',
        'read_pty_scrollback',
        'MAX_PTY_SCROLLBACK_BYTES',
        'writer: Arc<Mutex<Box<dyn Write + Send>>>',
        'Arc::clone(&s.writer)',
        'write_pty_recovery_bytes',
        'MOVEFILE_REPLACE_EXISTING'
    )) {
        if (-not $terminal.Contains($needle)) { throw "missing 02.20 PTY invariant: $needle" }
    }
    $ipc = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    foreach ($needle in @('terminal.pty.read-wait','Duration::from_secs(7)','client_response_timeout')) {
        if (-not $ipc.Contains($needle)) { throw "missing 02.20 IPC invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('terminal.pty.start','terminal.pty.write','terminal.pty.read-wait','terminal.pty.resize','terminal.pty.status','terminal.pty.stop','terminal.pty.remove','terminal.pty.scrollback.read','terminal.pty.recovery.list')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent PTY command: $needle" }
    }
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw
    foreach ($needle in @('pty-start','pty-write','pty-read-wait','pty-resize','pty-status','pty-stop','pty-remove','pty-scrollback-read','pty-recovery-list')) {
        if (-not $cliSource.Contains($needle)) { throw "missing CLI PTY command: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-terminal --package vsn-ipc --all-targets -- -D warnings
    Assert-LastExit 'PTY/IPC clippy failed'
    cargo test --locked --package vsn-terminal --package vsn-ipc --package vsn-core
    Assert-LastExit 'PTY/IPC/Core tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $fixtureSource = Join-Path $sandbox 'pty_fixture.rs'
    @'
use std::{env, io::{self, BufRead, Write}, thread, time::Duration};
fn main() {
    if env::args().nth(1).as_deref() == Some("idle") {
        thread::sleep(Duration::from_secs(30));
        return;
    }
    println!("PTY_READY");
    io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines() {
        let line = line.unwrap_or_default();
        if line == "exit" {
            println!("PTY_EXIT");
            io::stdout().flush().unwrap();
            return;
        }
        if line == "burst" {
            let block = "Z".repeat(8192);
            for _ in 0..160 { print!("{block}"); }
            println!("BURST_DONE");
            io::stdout().flush().unwrap();
            continue;
        }
        println!("ECHO:{line}");
        io::stdout().flush().unwrap();
    }
}
'@ | Set-Content -LiteralPath $fixtureSource -Encoding utf8
    rustc $fixtureSource -O -o $fixture
    Assert-LastExit 'PTY fixture build failed'

    Start-Agent
    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    # Containment must fail closed for cwd aliases and programs outside the registered workspace.
    & $script:Cli terminal pty-start $outside $fixture 1> (Join-Path $root 'outside-cwd.stdout') 2> (Join-Path $root 'outside-cwd.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'outside-workspace PTY cwd unexpectedly succeeded' }
    $outsideLink = Join-Path $workspace 'outside-link'
    New-Item -ItemType Junction -Path $outsideLink -Target $outside | Out-Null
    & $script:Cli terminal pty-start $outsideLink $fixture 1> (Join-Path $root 'junction-cwd.stdout') 2> (Join-Path $root 'junction-cwd.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'junction escape PTY cwd unexpectedly succeeded' }
    Remove-Item -LiteralPath $outsideLink -Force
    $outsideLink = $null
    $outsideProgram = Join-Path $outside 'outside-program.exe'
    Copy-Item -LiteralPath $fixture -Destination $outsideProgram -Force
    & $script:Cli terminal pty-start $workspace $outsideProgram 1> (Join-Path $root 'outside-program.stdout') 2> (Join-Path $root 'outside-program.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'outside-workspace PTY program unexpectedly succeeded' }

    # Main interactive PTY lifecycle. ConPTY may first request cursor position (ESC[6n);
    # a real terminal host must answer that control handshake before application output proceeds.
    $started = & $script:Cli terminal pty-start $workspace $fixture | Out-String | ConvertFrom-Json
    $started | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $root 'pty-start.json') -Encoding utf8
    $id = [string]$started.session_id
    if (-not $id -or $started.running -ne $true -or [int]$started.rows -ne 30 -or [int]$started.cols -ne 120) { throw 'PTY start returned unexpected state' }
    $sessionIds.Add($id)

    $ready = Read-PtyUntilMarker $id 'PTY_READY' 5000
    $ready | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-ready.json') -Encoding utf8
    if (-not ([string]$ready.output).Contains('PTY_READY')) { throw 'PTY startup output missing after terminal-host handshake' }

    $listed = @(& $script:Cli terminal pty-list | Out-String | ConvertFrom-Json)
    $listed | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $root 'pty-list.json') -Encoding utf8
    if (-not ($listed | Where-Object { $_.session_id -eq $id })) { throw 'PTY list omitted active session' }

    $recoveryRunning = @(& $script:Cli terminal pty-recovery-list | Out-String | ConvertFrom-Json)
    $recoveryRunning | ConvertTo-Json -Depth 7 | Set-Content (Join-Path $root 'recovery-running.json') -Encoding utf8
    $runningCheckpoint = @($recoveryRunning | Where-Object { $_.session_id -eq $id })
    if ($runningCheckpoint.Count -ne 1 -or [string]$runningCheckpoint[0].state -ne 'running_at_last_checkpoint') { throw 'active PTY recovery checkpoint missing' }

    # Resize must update both live state and the already-existing recovery checkpoint on Windows.
    $resized = & $script:Cli terminal pty-resize $id 40 140 | Out-String | ConvertFrom-Json
    $resized | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-resize.json') -Encoding utf8
    if ([int]$resized.rows -ne 40 -or [int]$resized.cols -ne 140 -or $resized.running -ne $true) { throw 'PTY resize state was not persisted' }
    $status = & $script:Cli terminal pty-status $id | Out-String | ConvertFrom-Json
    $status | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-status.json') -Encoding utf8
    if ([int]$status.rows -ne 40 -or [int]$status.cols -ne 140 -or $status.running -ne $true) { throw 'PTY status lost resized dimensions' }

    $recoveryResized = @(& $script:Cli terminal pty-recovery-list | Out-String | ConvertFrom-Json)
    $resizedCheckpoint = @($recoveryResized | Where-Object { $_.session_id -eq $id })
    $recoveryResized | ConvertTo-Json -Depth 7 | Set-Content (Join-Path $root 'recovery-resized.json') -Encoding utf8
    if ($resizedCheckpoint.Count -ne 1 -or [int]$resizedCheckpoint[0].rows -ne 40 -or [int]$resizedCheckpoint[0].cols -ne 140) { throw 'PTY recovery checkpoint did not reflect resize' }

    "hello-pty`n" | & $script:Cli terminal pty-write $id | Set-Content (Join-Path $root 'pty-write.json') -Encoding utf8
    Assert-LastExit 'PTY input write failed'
    $echo = & $script:Cli terminal pty-read-wait $id 3000 | Out-String | ConvertFrom-Json
    $echo | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-echo.json') -Encoding utf8
    if (-not ([string]$echo.output).Contains('ECHO:hello-pty')) { throw 'PTY interactive echo missing' }

    # Prepare the terminal-host side before measuring a true no-output bounded read-wait.
    $idle = & $script:Cli terminal pty-start $workspace $fixture idle | Out-String | ConvertFrom-Json
    $idleId = [string]$idle.session_id
    if (-not $idleId -or $idle.running -ne $true) { throw 'idle PTY did not start' }
    $sessionIds.Add($idleId)
    $idleStartup = Initialize-PtyHost $idleId 3000
    $idleStartup | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'idle-startup.json') -Encoding utf8
    if ($idleStartup.running -ne $true) { throw 'idle PTY exited during terminal-host initialization' }

    $wait = [Diagnostics.Stopwatch]::StartNew()
    $idleChunk = & $script:Cli terminal pty-read-wait $idleId 3000 | Out-String | ConvertFrom-Json
    $wait.Stop()
    $wait.ElapsedMilliseconds | Set-Content (Join-Path $root 'idle-read-wait-ms.txt')
    $idleChunk | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'idle-read-wait.json') -Encoding utf8
    if ($wait.Elapsed.TotalSeconds -lt 2.5 -or $wait.Elapsed.TotalSeconds -gt 4.5) { throw "bounded PTY long-poll returned outside expected 3s window: $($wait.Elapsed.TotalSeconds)s" }
    if ($idleChunk.running -ne $true -or [string]$idleChunk.output) { throw 'idle PTY long-poll returned unexpected payload/state' }
    $idleStopped = & $script:Cli terminal pty-stop $idleId | Out-String | ConvertFrom-Json
    if ($idleStopped.running -ne $false) { throw 'idle PTY stop failed' }
    & $script:Cli terminal pty-remove $idleId *> $null
    Assert-LastExit 'idle PTY remove failed'
    $sessionIds.Remove($idleId) | Out-Null

    # Live output is bounded while durable scrollback independently retains the larger stream.
    "burst`n" | & $script:Cli terminal pty-write $id *> $null
    Assert-LastExit 'PTY burst command write failed'
    $burstWatch = [Diagnostics.Stopwatch]::StartNew()
    $scrollList = @()
    $scroll = @()
    do {
        Start-Sleep -Milliseconds 100
        $scrollList = @(& $script:Cli terminal pty-scrollback-list | Out-String | ConvertFrom-Json)
        $scroll = @($scrollList | Where-Object { $_.session_id -eq $id })
        if ($scroll.Count -eq 1 -and [uint64]$scroll[0].bytes -ge 1100000) { break }
    } while ($burstWatch.Elapsed.TotalSeconds -lt 5)
    $burstWatch.Stop()

    $burst = & $script:Cli terminal pty-read $id | Out-String | ConvertFrom-Json
    $burst | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-burst.json') -Encoding utf8
    if ([uint64]$burst.dropped_bytes -eq 0) { throw 'PTY bounded live buffer did not report dropped bytes after oversized output' }
    if ([Text.Encoding]::UTF8.GetByteCount([string]$burst.output) -gt 65536) { throw 'PTY live read exceeded CLI 64 KiB read bound' }

    $scrollList | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $root 'scrollback-list.json') -Encoding utf8
    if ($scroll.Count -ne 1 -or [uint64]$scroll[0].bytes -lt 1048576 -or $scroll[0].active -ne $true) { throw 'durable PTY scrollback was not retained independently of live truncation' }
    $scrollBytes = [uint64]$scroll[0].bytes

    $scrollChunk = & $script:Cli terminal pty-scrollback-read $id 0 262144 | Out-String | ConvertFrom-Json
    $scrollChunk | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'scrollback-first.json') -Encoding utf8
    if ([uint64]$scrollChunk.next_offset -eq 0 -or [uint64]$scrollChunk.total_bytes -lt 1048576 -or $scrollChunk.eof -ne $false) { throw 'bounded PTY scrollback read contract failed' }
    $decoded = [Convert]::FromBase64String([string]$scrollChunk.payload_base64)
    if ($decoded.Length -gt 262144) { throw 'PTY scrollback read exceeded 256 KiB bound' }

    # Invalid resize input must fail closed.
    & $script:Cli terminal pty-resize '__missing_pty_0220__' 40 140 1> (Join-Path $root 'invalid-resize.stdout') 2> (Join-Path $root 'invalid-resize.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'missing-session PTY resize unexpectedly succeeded' }
    & $script:Cli terminal pty-resize $id 0 140 1> (Join-Path $root 'zero-resize.stdout') 2> (Join-Path $root 'zero-resize.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'zero-row PTY resize unexpectedly succeeded' }

    $stopped = & $script:Cli terminal pty-stop $id | Out-String | ConvertFrom-Json
    $stopped | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-stop.json') -Encoding utf8
    if ($stopped.running -ne $false) { throw 'PTY stop did not transition to stopped state' }

    $recoveryStopped = @(& $script:Cli terminal pty-recovery-list | Out-String | ConvertFrom-Json)
    $recoveryStopped | ConvertTo-Json -Depth 7 | Set-Content (Join-Path $root 'recovery-stopped.json') -Encoding utf8
    $stoppedCheckpoint = @($recoveryStopped | Where-Object { $_.session_id -eq $id })
    if ($stoppedCheckpoint.Count -ne 1 -or [string]$stoppedCheckpoint[0].state -ne 'stopped' -or [int]$stoppedCheckpoint[0].rows -ne 40 -or [int]$stoppedCheckpoint[0].cols -ne 140) { throw 'stopped PTY recovery checkpoint was not durably updated' }

    & $script:Cli terminal pty-remove $id | Set-Content (Join-Path $root 'pty-remove.json') -Encoding utf8
    Assert-LastExit 'PTY remove failed'
    $sessionIds.Remove($id) | Out-Null
    $afterRemove = @(& $script:Cli terminal pty-list | Out-String | ConvertFrom-Json)
    if (@($afterRemove | Where-Object { $_.session_id -eq $id }).Count -ne 0) { throw 'removed PTY still appears active' }

    & $script:Cli terminal pty-recovery-remove $id | Set-Content (Join-Path $root 'recovery-remove.json') -Encoding utf8
    Assert-LastExit 'PTY recovery metadata removal failed'
    & $script:Cli terminal pty-scrollback-remove $id | Set-Content (Join-Path $root 'scrollback-remove.json') -Encoding utf8
    Assert-LastExit 'PTY scrollback removal failed'

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }
    $auditEvents = @($chain.events).Count

    $remaining = @(& $script:Cli terminal pty-list | Out-String | ConvertFrom-Json)
    if ($remaining.Count -ne 0) { throw 'PTY sessions remain after lifecycle cleanup' }
    $sessionsCleaned = $true

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task_id = '02.20'
        artifact = 'pty-conpty-lifecycle-windows-github-hosted'
        product_version = $candidate.product_version
        candidate_id = $candidate.candidate_id
        source_commit = $actual
        runner_name = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        checks = [ordered]@{
            exact_source = $true
            github_hosted_windows = $true
            required_tests = $true
            pty_start_list = $true
            interactive_write_read = $true
            bounded_read_wait = $true
            resize_status = $true
            bounded_live_output = $true
            durable_bounded_scrollback = $true
            recovery_checkpoint_replacement = $true
            stop_remove = $true
            workspace_and_program_containment = $true
            invalid_resize_fail_closed = $true
            audit_chain_valid = $true
            session_cleanup = $true
        }
        measurements = [ordered]@{
            startup_dsr_responses = [uint64]$ready.dsr_responses
            idle_startup_dsr_responses = [uint64]$idleStartup.dsr_responses
            idle_read_wait_ms = [uint64]$wait.ElapsedMilliseconds
            burst_wait_ms = [uint64]$burstWatch.ElapsedMilliseconds
            live_dropped_bytes = [uint64]$burst.dropped_bytes
            scrollback_bytes = $scrollBytes
            first_scrollback_read_bytes = [uint64]$decoded.Length
            audit_events = [uint64]$auditEvents
        }
    } | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    if ($script:agent -and -not $script:agent.HasExited -and $script:Cli) {
        foreach ($sid in @($sessionIds)) {
            & $script:Cli terminal pty-stop $sid *> $null
            & $script:Cli terminal pty-remove $sid *> $null
        }
        $cleanupRemaining = @(& $script:Cli terminal pty-list | Out-String | ConvertFrom-Json)
        if ($LASTEXITCODE -eq 0 -and $cleanupRemaining.Count -eq 0) {
            $sessionsCleaned = $true
        }
    }
    if ($outsideLink -and (Test-Path -LiteralPath $outsideLink)) {
        Remove-Item -LiteralPath $outsideLink -Force -ErrorAction SilentlyContinue
    }
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
    $ipcRestored = if ($hadIpcKey) {
        (Test-Path -LiteralPath $ipcKey) -and ((Get-FileHash $ipcKey -Algorithm SHA256).Hash -eq $ipcKeyHashBefore)
    } else {
        -not (Test-Path -LiteralPath $ipcKey)
    }
    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force -ErrorAction SilentlyContinue
    }
    [ordered]@{
        agent_stopped = ($null -eq $script:agent)
        localappdata_restored = ($env:LOCALAPPDATA -eq $originalLocalAppData)
        ipc_key_restored = [bool]$ipcRestored
        junction_removed = (-not $outsideLink -or -not (Test-Path -LiteralPath $outsideLink))
        sessions_cleaned = [bool]$sessionsCleaned
        sandbox_removed = (-not (Test-Path -LiteralPath $sandbox))
    } | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $root 'cleanup.json') -Encoding utf8
}