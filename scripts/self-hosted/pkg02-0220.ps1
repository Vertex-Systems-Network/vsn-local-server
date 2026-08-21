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
$agent = $null

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
    if (-not $IsWindows) { throw '02.20 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.20 certification requires a GitHub-hosted runner' }
    Write-Host "runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $terminal = Get-Content 'crates/vsn-terminal/src/lib.rs' -Raw
    foreach ($needle in @('start_pty_session_with_scrollback','write_pty_session','read_pty_session_wait','resize_pty_session','stop_pty_session','remove_pty_session','list_pty_recovery','read_pty_scrollback','MAX_PTY_SCROLLBACK_BYTES','write_pty_recovery')) {
        if (-not $terminal.Contains($needle)) { throw "missing PTY/ConPTY invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('terminal.pty.start','terminal.pty.write','terminal.pty.read-wait','terminal.pty.resize','terminal.pty.status','terminal.pty.stop','terminal.pty.remove','terminal.pty.scrollback.read','terminal.pty.recovery.list')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent PTY command: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-terminal --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'PTY clippy failed'
    cargo test --locked --package vsn-terminal --package vsn-core
    Assert-LastExit 'PTY tests failed'
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
use std::io::{self, BufRead, Write};
fn main() {
    println!("PTY_READY");
    io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
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

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb another Agent' }
    Start-Agent
    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    $started = & $script:Cli terminal pty-start $workspace $fixture | Out-String | ConvertFrom-Json
    $started | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $root 'pty-start.json') -Encoding utf8
    $id = [string]$started.session_id
    if (-not $id -or $started.running -ne $true -or [int]$started.rows -ne 30 -or [int]$started.cols -ne 120) { throw 'PTY start returned unexpected state' }

    $ready = & $script:Cli terminal pty-read-wait $id 3000 | Out-String | ConvertFrom-Json
    $ready | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-ready.json') -Encoding utf8
    if (-not ([string]$ready.output).Contains('PTY_READY')) { throw 'PTY startup output missing' }

    $recoveryRunning = & $script:Cli terminal pty-recovery-list | Out-String | ConvertFrom-Json
    $recoveryRunning | ConvertTo-Json -Depth 7 | Set-Content (Join-Path $root 'recovery-running.json') -Encoding utf8
    $runningCheckpoint = @($recoveryRunning | Where-Object { $_.session_id -eq $id })
    if ($runningCheckpoint.Count -ne 1 -or [string]$runningCheckpoint[0].state -ne 'running_at_last_checkpoint') { throw 'active PTY recovery checkpoint missing' }

    # On Windows this requires atomic replacement of the already-existing recovery JSON.
    # A plain fs::rename(tmp, existing.json) fails instead of updating the checkpoint.
    $resized = & $script:Cli terminal pty-resize $id 40 140 | Out-String | ConvertFrom-Json
    $resized | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-resize.json') -Encoding utf8
    if ([int]$resized.rows -ne 40 -or [int]$resized.cols -ne 140 -or $resized.running -ne $true) { throw 'PTY resize state was not persisted' }

    $status = & $script:Cli terminal pty-status $id | Out-String | ConvertFrom-Json
    if ([int]$status.rows -ne 40 -or [int]$status.cols -ne 140) { throw 'PTY status lost resized dimensions' }

    "hello`n" | & $script:Cli terminal pty-write $id | Set-Content (Join-Path $root 'pty-write.json') -Encoding utf8
    Assert-LastExit 'PTY input write failed'
    $echo = & $script:Cli terminal pty-read-wait $id 3000 | Out-String | ConvertFrom-Json
    $echo | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-echo.json') -Encoding utf8
    if (-not ([string]$echo.output).Contains('ECHO:hello')) { throw 'PTY interactive echo missing' }

    "burst`n" | & $script:Cli terminal pty-write $id *> $null
    Assert-LastExit 'PTY burst command write failed'
    Start-Sleep -Milliseconds 700
    $burst = & $script:Cli terminal pty-read $id | Out-String | ConvertFrom-Json
    $burst | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-burst.json') -Encoding utf8
    if ([uint64]$burst.dropped_bytes -eq 0) { throw 'PTY bounded live buffer did not report dropped bytes after oversized output' }

    $scrollList = & $script:Cli terminal pty-scrollback-list | Out-String | ConvertFrom-Json
    $scrollList | ConvertTo-Json -Depth 6 | Set-Content (Join-Path $root 'scrollback-list.json') -Encoding utf8
    $scroll = @($scrollList | Where-Object { $_.session_id -eq $id })
    if ($scroll.Count -ne 1 -or [uint64]$scroll[0].bytes -lt 1048576 -or $scroll[0].active -ne $true) { throw 'durable PTY scrollback was not retained independently of live buffer truncation' }

    $scrollChunk = & $script:Cli terminal pty-scrollback-read $id 0 262144 | Out-String | ConvertFrom-Json
    $scrollChunk | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'scrollback-first.json') -Encoding utf8
    if ([uint64]$scrollChunk.next_offset -eq 0 -or [uint64]$scrollChunk.total_bytes -lt 1048576 -or $scrollChunk.eof -ne $false) { throw 'bounded PTY scrollback read contract failed' }
    $decoded = [Convert]::FromBase64String([string]$scrollChunk.payload_base64)
    if ($decoded.Length -gt 262144) { throw 'PTY scrollback read exceeded 256 KiB bound' }

    $stopped = & $script:Cli terminal pty-stop $id | Out-String | ConvertFrom-Json
    $stopped | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'pty-stop.json') -Encoding utf8
    if ($stopped.running -ne $false) { throw 'PTY stop did not transition to stopped state' }

    $recoveryStopped = & $script:Cli terminal pty-recovery-list | Out-String | ConvertFrom-Json
    $recoveryStopped | ConvertTo-Json -Depth 7 | Set-Content (Join-Path $root 'recovery-stopped.json') -Encoding utf8
    $stoppedCheckpoint = @($recoveryStopped | Where-Object { $_.session_id -eq $id })
    if ($stoppedCheckpoint.Count -ne 1 -or [string]$stoppedCheckpoint[0].state -ne 'stopped') { throw 'stopped PTY recovery checkpoint was not updated' }

    & $script:Cli terminal pty-remove $id | Set-Content (Join-Path $root 'pty-remove.json') -Encoding utf8
    Assert-LastExit 'PTY remove failed'
    $afterRemove = & $script:Cli terminal pty-list | Out-String | ConvertFrom-Json
    if (@($afterRemove | Where-Object { $_.session_id -eq $id }).Count -ne 0) { throw 'removed PTY still appears active' }

    & $script:Cli terminal pty-recovery-remove $id | Set-Content (Join-Path $root 'recovery-remove.json') -Encoding utf8
    Assert-LastExit 'PTY recovery metadata removal failed'
    & $script:Cli terminal pty-scrollback-remove $id | Set-Content (Join-Path $root 'scrollback-remove.json') -Encoding utf8
    Assert-LastExit 'PTY scrollback removal failed'

    & $script:Cli terminal pty-start $outside $fixture 1> (Join-Path $root 'outside.stdout') 2> (Join-Path $root 'outside.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'outside-workspace PTY start unexpectedly succeeded' }
    & $script:Cli terminal pty-resize '__missing_pty_0220__' 0 120 1> (Join-Path $root 'invalid-resize.stdout') 2> (Join-Path $root 'invalid-resize.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'invalid PTY resize unexpectedly succeeded' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.20';
        artifact='pty-conpty-lifecycle-windows-github-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_environment=$env:RUNNER_ENVIRONMENT;
        pty_start_verified=$true; interactive_write_read_verified=$true; resize_verified=$true;
        bounded_live_buffer_verified=$true; durable_scrollback_verified=$true; recovery_checkpoint_updates_verified=$true;
        stop_remove_verified=$true; workspace_containment_verified=$true; audit_chain_valid=$true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
