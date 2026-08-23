param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Write-JsonFile([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath $Path -Encoding utf8
}

function Invoke-CliCapture([string[]]$CliArgs, [string]$Stdout, [string]$Stderr) {
    & $script:Cli @CliArgs 1> $Stdout 2> $Stderr
    return $LASTEXITCODE
}

function Start-Agent {
    $agentOut = Join-Path $script:Root 'agent.stdout.log'
    $agentErr = Join-Path $script:Root 'agent.stderr.log'
    $script:Agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput $agentOut -RedirectStandardError $agentErr -PassThru -WindowStyle Hidden
    $script:Agent.Id | Set-Content (Join-Path $script:Root 'agent.pid')
    $ready = $false
    foreach ($i in 1..80) {
        $script:Agent.Refresh()
        if ($script:Agent.HasExited) { throw "Agent exited before readiness with code $($script:Agent.ExitCode)" }
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        Start-Sleep -Milliseconds 250
    }
    if (-not $ready) { throw 'Agent did not become ready' }
}

function Stop-Agent {
    if ($script:Agent -and -not $script:Agent.HasExited) {
        Stop-Process -Id $script:Agent.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $script:Agent.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
    $script:Agent = $null
}

$root = Join-Path $PWD 'dist-self-hosted\02.14'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0214-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$outsideDir = Join-Path $sandbox 'outside'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null
$evidence = $null
$acceptanceSucceeded = $false
$exclusiveLogHandle = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$isolatedLocalAppData,$outsideDir | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData
$script:Root = $root
$script:Agent = $null

try {
    if (-not $IsWindows) { throw "02.14 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    if ($env:RUNNER_ENVIRONMENT -and $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw "02.14 certification requires a GitHub-hosted runner; got '$env:RUNNER_ENVIRONMENT'" }
    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }

    $core = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    $system = Get-Content 'crates/vsn-system/src/lib.rs' -Raw
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw

    foreach ($needle in @(
        'Permission::MachineView',
        'Permission::NetworkView',
        'Permission::ServiceView',
        'Permission::FilesRead',
        'baseline log access is restricted to VSN-owned data'
    )) {
        if (-not $core.Contains($needle)) { throw "missing 02.14 Core boundary invariant: $needle" }
    }
    foreach ($needle in @(
        'DIAGNOSTIC_COMMAND_TIMEOUT',
        'MAX_PROCESS_ITEMS',
        'MAX_PORT_ITEMS',
        'MAX_TCP_TIMEOUT_MS',
        'MAX_LOG_BYTES',
        'MAX_LOG_LINES',
        'MAX_LOG_RESPONSE_BYTES',
        'run_bounded_command',
        'pub fn list_processes',
        'pub fn process_metrics',
        'pub fn list_ports',
        'pub fn port_conflicts',
        'pub fn tcp_health',
        'pub fn tail_log'
    )) {
        if (-not $system.Contains($needle)) { throw "missing 02.14 system invariant: $needle" }
    }
    foreach ($needle in @('"process.list"','"process.metrics"','"port.list"','"port.check"','"health.tcp"','"log.tail"')) {
        if (-not $agentSource.Contains($needle)) { throw "missing authenticated Agent route: $needle" }
    }
    foreach ($needle in @('process" && sub == "list','process" && sub == "metrics','port" && sub == "list','port" && sub == "check','health" && sub == "tcp','log" && sub == "tail')) {
        if (-not $cliSource.Contains($needle)) { throw "missing CLI-to-Agent route: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-system --package vsn-core --package vsn-agent --package vsn --all-targets --no-deps -- -D warnings
    Assert-LastExit '02.14 path clippy failed'
    cargo test --locked --package vsn-system --package vsn-core
    Assert-LastExit '02.14 package tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    Start-Agent

    $diagnosticsRaw = & $script:Cli diagnostics | Out-String
    Assert-LastExit 'diagnostics command failed'
    $diagnosticsRaw | Set-Content (Join-Path $root 'diagnostics.json') -Encoding utf8
    $diagnostics = $diagnosticsRaw | ConvertFrom-Json
    $dataDir = [string]$diagnostics.data_dir
    if (-not $dataDir) { throw 'diagnostics did not expose VSN data_dir' }

    $processRaw = & $script:Cli process list | Out-String
    Assert-LastExit 'process list failed'
    $processRaw | Set-Content (Join-Path $root 'process-list.json') -Encoding utf8
    $processes = @($processRaw | ConvertFrom-Json)
    if ($processes.Count -lt 1) { throw 'process snapshot is empty' }
    if ($processes.Count -gt 512) { throw "process snapshot exceeded 512 entries: $($processes.Count)" }
    if ([Text.Encoding]::UTF8.GetByteCount($processRaw) -ge 900KB) { throw 'process snapshot exceeded frame-safe acceptance budget' }
    $lastPid = 0
    foreach ($process in $processes) {
        if ([uint32]$process.pid -lt [uint32]$lastPid) { throw 'process snapshot is not deterministically sorted by pid' }
        $lastPid = [uint32]$process.pid
    }

    $metricsRaw = & $script:Cli process metrics $PID | Out-String
    Assert-LastExit 'process metrics failed'
    $metricsRaw | Set-Content (Join-Path $root 'process-metrics.json') -Encoding utf8
    $metrics = $metricsRaw | ConvertFrom-Json
    if ([uint32]$metrics.pid -ne [uint32]$PID) { throw 'process metrics returned wrong pid' }
    if (-not $metrics.memory_bytes -or [uint64]$metrics.memory_bytes -eq 0) { throw 'process metrics missing memory usage' }

    $zeroPortOut = Join-Path $root 'port-zero.stdout'
    $zeroPortErr = Join-Path $root 'port-zero.stderr'
    $zeroPortCode = Invoke-CliCapture -CliArgs @('port','check','0') -Stdout $zeroPortOut -Stderr $zeroPortErr
    $zeroPortCode | Set-Content (Join-Path $root 'port-zero.exit-code.txt')
    if ($zeroPortCode -eq 0) { throw 'port 0 check unexpectedly succeeded' }

    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try {
        $port = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
        $portRaw = & $script:Cli port list | Out-String
        Assert-LastExit 'port list failed'
        $portRaw | Set-Content (Join-Path $root 'port-list.json') -Encoding utf8
        $ports = @($portRaw | ConvertFrom-Json)
        if ($ports.Count -gt 2048) { throw "port snapshot exceeded 2048 entries: $($ports.Count)" }
        if ([Text.Encoding]::UTF8.GetByteCount($portRaw) -ge 900KB) { throw 'port snapshot exceeded frame-safe acceptance budget' }
        if (-not ($ports | Where-Object { [int]$_.port -eq $port })) { throw "port list did not include loopback listener $port" }

        $conflictsRaw = & $script:Cli port check $port | Out-String
        Assert-LastExit 'port conflict check failed'
        $conflictsRaw | Set-Content (Join-Path $root 'port-check.json') -Encoding utf8
        $conflicts = @($conflictsRaw | ConvertFrom-Json)
        if (-not ($conflicts | Where-Object { [int]$_.port -eq $port })) { throw 'port check missed active loopback conflict' }

        $healthyRaw = & $script:Cli health tcp 127.0.0.1 $port | Out-String
        Assert-LastExit 'healthy TCP check failed'
        $healthyRaw | Set-Content (Join-Path $root 'tcp-healthy.json') -Encoding utf8
        $healthy = $healthyRaw | ConvertFrom-Json
        if ($healthy.healthy -ne $true) { throw 'TCP health did not report active loopback listener healthy' }
    }
    finally {
        $listener.Stop()
    }

    $unhealthyRaw = & $script:Cli health tcp 127.0.0.1 $port | Out-String
    Assert-LastExit 'closed-port TCP check command failed'
    $unhealthyRaw | Set-Content (Join-Path $root 'tcp-unhealthy.json') -Encoding utf8
    $unhealthy = $unhealthyRaw | ConvertFrom-Json
    if ($unhealthy.healthy -ne $false) { throw 'TCP health did not report closed loopback listener unhealthy' }

    $timeoutStarted = [DateTime]::UtcNow
    $boundedRaw = & $script:Cli health tcp 203.0.113.1 9 | Out-String
    Assert-LastExit 'bounded TCP failure check command failed'
    $boundedElapsedMs = [int]([DateTime]::UtcNow - $timeoutStarted).TotalMilliseconds
    $boundedRaw | Set-Content (Join-Path $root 'tcp-bounded-failure.json') -Encoding utf8
    $boundedHealth = $boundedRaw | ConvertFrom-Json
    if ($boundedHealth.healthy -ne $false) { throw 'TEST-NET TCP endpoint unexpectedly reported healthy' }
    if ($boundedElapsedMs -ge 5000) { throw "TCP health exceeded bounded acceptance time: ${boundedElapsedMs}ms" }
    $boundedElapsedMs | Set-Content (Join-Path $root 'tcp-bounded-elapsed-ms.txt')

    $logDir = Join-Path $dataDir 'logs'
    New-Item -ItemType Directory -Force -Path $logDir | Out-Null
    $log = Join-Path $logDir 'pkg02-0214.log'
    $stream = [System.IO.File]::Open($log,[System.IO.FileMode]::Create,[System.IO.FileAccess]::Write,[System.IO.FileShare]::Read)
    try {
        $buffer = New-Object byte[] (1024 * 1024)
        [Array]::Fill[byte]($buffer, [byte]120)
        foreach ($i in 1..9) { $stream.Write($buffer,0,$buffer.Length) }
        $newline = [Text.Encoding]::UTF8.GetBytes("`n")
        $stream.Write($newline,0,$newline.Length)
        foreach ($i in 1..6000) {
            $lineBytes = [Text.Encoding]::UTF8.GetBytes(("tail-{0:D4}`n" -f $i))
            $stream.Write($lineBytes,0,$lineBytes.Length)
        }
    }
    finally {
        $stream.Dispose()
    }

    $tailRaw = & $script:Cli log tail $log 6000 | Out-String
    Assert-LastExit 'bounded log tail failed'
    $tailRaw | Set-Content (Join-Path $root 'log-tail.json') -Encoding utf8
    $tail = @($tailRaw | ConvertFrom-Json)
    if ($tail.Count -ne 5000) { throw "log tail did not enforce 5000-line cap: $($tail.Count)" }
    if ([string]$tail[0] -ne 'tail-1001' -or [string]$tail[-1] -ne 'tail-6000') { throw 'bounded log tail returned unexpected trailing window' }

    $hugeLog = Join-Path $logDir 'pkg02-0214-huge-line.log'
    ([string]('z' * (700 * 1024)) + "`n") | Set-Content -LiteralPath $hugeLog -NoNewline -Encoding utf8
    $hugeRaw = & $script:Cli log tail $hugeLog 10 | Out-String
    Assert-LastExit 'huge-line bounded log tail failed'
    $hugeRaw | Set-Content (Join-Path $root 'log-tail-huge-line.json') -Encoding utf8
    $hugeTail = @($hugeRaw | ConvertFrom-Json)
    if ($hugeTail.Count -ne 1 -or -not ([string]$hugeTail[0]).StartsWith('[truncated] ')) { throw 'huge log line was not explicitly bounded' }
    if ([Text.Encoding]::UTF8.GetByteCount($hugeRaw) -ge 600KB) { throw 'huge-line log response exceeded frame-safe acceptance budget' }

    $missingOut = Join-Path $root 'missing-log.stdout'
    $missingErr = Join-Path $root 'missing-log.stderr'
    $missingCode = Invoke-CliCapture -CliArgs @('log','tail',(Join-Path $logDir 'missing.log'),'10') -Stdout $missingOut -Stderr $missingErr
    $missingCode | Set-Content (Join-Path $root 'missing-log.exit-code.txt')
    if ($missingCode -eq 0) { throw 'missing log unexpectedly succeeded' }

    $unreadable = Join-Path $logDir 'exclusive.log'
    'exclusive-content' | Set-Content -LiteralPath $unreadable -Encoding utf8
    $exclusiveLogHandle = [System.IO.File]::Open($unreadable,[System.IO.FileMode]::Open,[System.IO.FileAccess]::ReadWrite,[System.IO.FileShare]::None)
    try {
        $unreadableOut = Join-Path $root 'unreadable-log.stdout'
        $unreadableErr = Join-Path $root 'unreadable-log.stderr'
        $unreadableCode = Invoke-CliCapture -CliArgs @('log','tail',$unreadable,'10') -Stdout $unreadableOut -Stderr $unreadableErr
        $unreadableCode | Set-Content (Join-Path $root 'unreadable-log.exit-code.txt')
        if ($unreadableCode -eq 0) { throw 'exclusively locked log unexpectedly succeeded' }
    }
    finally {
        $exclusiveLogHandle.Dispose()
        $exclusiveLogHandle = $null
    }

    $outside = Join-Path $outsideDir 'outside.log'
    'must-not-be-readable-through-vsn-log-tail' | Set-Content -LiteralPath $outside -Encoding utf8
    $outsideOut = Join-Path $root 'outside-log.stdout'
    $outsideErr = Join-Path $root 'outside-log.stderr'
    $outsideCode = Invoke-CliCapture -CliArgs @('log','tail',$outside,'10') -Stdout $outsideOut -Stderr $outsideErr
    $outsideCode | Set-Content (Join-Path $root 'outside-log.exit-code.txt')
    if ($outsideCode -eq 0) { throw 'outside VSN data log path unexpectedly succeeded' }

    $junction = Join-Path $logDir 'outside-link'
    New-Item -ItemType Junction -Path $junction -Target $outsideDir | Out-Null
    $junctionOut = Join-Path $root 'junction-log.stdout'
    $junctionErr = Join-Path $root 'junction-log.stderr'
    $junctionCode = Invoke-CliCapture -CliArgs @('log','tail',(Join-Path $junction 'outside.log'),'10') -Stdout $junctionOut -Stderr $junctionErr
    $junctionCode | Set-Content (Join-Path $root 'junction-log.exit-code.txt')
    if ($junctionCode -eq 0) { throw 'junction escape log path unexpectedly succeeded' }

    $auditRaw = & $script:Cli audit verify | Out-String
    Assert-LastExit 'audit verification failed'
    $auditRaw | Set-Content (Join-Path $root 'audit-chain.json') -Encoding utf8
    $auditChain = $auditRaw | ConvertFrom-Json
    if ($auditChain.valid -ne $true) { throw 'audit chain is invalid' }

    $sourceCommit = if ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { (git rev-parse HEAD).Trim() }
    $evidence = [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task = '02.14'
        artifact = 'bounded-local-diagnostics-windows'
        source_commit = $sourceCommit
        runner = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        os = $env:RUNNER_OS
        arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        rust = $rust
        cargo = $cargoVersion
        tcp_bounded_elapsed_ms = $boundedElapsedMs
        checks = [ordered]@{
            authenticated_agent_cli_boundary = $true
            existing_permissions_enforced = $true
            process_snapshot_nonempty = $true
            process_snapshot_bounded = $true
            process_metrics_valid = $true
            port_snapshot_bounded = $true
            active_port_discovered = $true
            active_port_conflict_detected = $true
            invalid_port_rejected = $true
            tcp_success = $true
            tcp_failure = $true
            tcp_timeout_bounded = $true
            log_read_window_bounded = $true
            log_line_count_bounded = $true
            log_response_frame_bounded = $true
            missing_log_rejected = $true
            unreadable_log_rejected = $true
            outside_log_rejected = $true
            junction_escape_log_rejected = $true
            audit_chain_valid = $true
            cleanup_attempted = $true
        }
    }
    $acceptanceSucceeded = $true
}
finally {
    if ($exclusiveLogHandle) {
        $exclusiveLogHandle.Dispose()
        $exclusiveLogHandle = $null
    }
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force -ErrorAction SilentlyContinue
    }

    $ipcKeyRestored = if ($hadIpcKey) { Test-Path -LiteralPath $ipcKey } else { -not (Test-Path -LiteralPath $ipcKey) }
    $sandboxRemoved = -not (Test-Path -LiteralPath $sandbox)
    if (-not $ipcKeyRestored) { throw 'IPC key state was not restored after 02.14 cleanup' }
    if (-not $sandboxRemoved) { throw '02.14 sandbox still exists after cleanup' }

    if ($acceptanceSucceeded) {
        $evidence.checks['ipc_key_state_restored'] = $ipcKeyRestored
        $evidence.checks['sandbox_removed'] = $sandboxRemoved
        $evidence.checks['cleanup_verified'] = $true
        Write-JsonFile (Join-Path $root 'evidence.json') $evidence
        (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
    }
}
