param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Invoke-CliJson([string[]]$Args) {
    $text = & $script:Cli @Args | Out-String
    Assert-LastExit "CLI failed: $($Args -join ' ')"
    return ($text | ConvertFrom-Json)
}

function Invoke-CliCapture([string[]]$Args, [string]$Stdout, [string]$Stderr) {
    & $script:Cli @Args 1> $Stdout 2> $Stderr
    return $LASTEXITCODE
}

function Start-Agent {
    $agentOut = Join-Path $script:Root 'agent.stdout.log'
    $agentErr = Join-Path $script:Root 'agent.stderr.log'
    $script:Agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput $agentOut -RedirectStandardError $agentErr -PassThru -WindowStyle Hidden
    $script:Agent.Id | Set-Content (Join-Path $script:Root 'agent.pid')
    $ready = $false
    foreach ($i in 1..80) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
        if ($script:Agent.HasExited) { throw "Agent exited before readiness with code $($script:Agent.ExitCode)" }
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

$script:Root = Join-Path $PWD 'dist-self-hosted\02.14'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0214-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$script:Agent = $null
$listener = $null
$client = $null

New-Item -ItemType Directory -Force -Path $script:Root,$bin,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

try {
    if (-not $IsWindows) { throw "02.14 acceptance requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $script:Root 'runner.txt')

    $systemSource = Get-Content 'crates/vsn-system/src/lib.rs' -Raw
    foreach ($needle in @(
        'MAX_PROCESS_SNAPSHOT_ITEMS',
        'MAX_PORT_SNAPSHOT_ITEMS',
        'MAX_TCP_HEALTH_TIMEOUT_MS',
        'MAX_LOG_BYTES',
        'MAX_LOG_LINES',
        'parse_windows_netstat',
        'LISTENING',
        'local_health_addresses',
        'is_loopback()',
        'read_until(b''\n''',
        'symlink_metadata',
        'CpuPercent'
    )) {
        if (-not $systemSource.Contains($needle)) { throw "missing 02.14 system source invariant: $needle" }
    }

    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @(
        'Permission::MachineView',
        'Permission::NetworkView',
        'Permission::FilesRead',
        'baseline log access is restricted to VSN-owned data'
    )) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.14 Core boundary invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-system --package vsn-core --package vsn-agent --package vsn --all-targets -- -D warnings
    Assert-LastExit 'diagnostics/core/agent/cli clippy failed'
    cargo test --locked --package vsn-system --package vsn-core --package vsn-agent --package vsn
    Assert-LastExit 'diagnostics/core/agent/cli tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    Start-Agent

    $diagnostics = Invoke-CliJson @('diagnostics')
    $diagnostics | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'diagnostics.json') -Encoding utf8
    $dataDir = [string]$diagnostics.data_dir
    if ([string]::IsNullOrWhiteSpace($dataDir)) { throw 'diagnostics did not expose VSN data_dir' }
    New-Item -ItemType Directory -Force -Path $dataDir | Out-Null

    $processes = @(Invoke-CliJson @('process','list'))
    $processes | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'process-list.json') -Encoding utf8
    if ($processes.Count -lt 1) { throw 'process snapshot is empty' }
    if ($processes.Count -gt 4096) { throw "process snapshot exceeded bound: $($processes.Count)" }
    if (-not ($processes | Where-Object { [uint32]$_.pid -eq [uint32]$script:Agent.Id })) {
        throw "process snapshot does not contain Agent pid $($script:Agent.Id)"
    }
    $processPids = @($processes | ForEach-Object { [uint32]$_.pid })
    $sortedPids = @($processPids | Sort-Object)
    if (($processPids -join ',') -ne ($sortedPids -join ',')) { throw 'process snapshot is not deterministically PID-sorted' }

    $metrics = Invoke-CliJson @('process','metrics',[string]$script:Agent.Id)
    $metrics | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'process-metrics.json') -Encoding utf8
    if ([uint32]$metrics.pid -ne [uint32]$script:Agent.Id) { throw 'process metrics PID binding mismatch' }
    if ($null -eq $metrics.cpu_percent) { throw 'Windows process CPU metric is missing' }
    if ([double]$metrics.cpu_percent -lt 0 -or [double]$metrics.cpu_percent -gt 100) { throw "CPU metric is outside bounded range: $($metrics.cpu_percent)" }
    if ([uint64]$metrics.memory_bytes -lt 1) { throw 'Windows process memory metric is missing' }

    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    $port | Set-Content (Join-Path $script:Root 'listener-port.txt')

    $ports = @(Invoke-CliJson @('port','list'))
    $ports | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'port-list.json') -Encoding utf8
    if ($ports.Count -gt 8192) { throw "port snapshot exceeded bound: $($ports.Count)" }
    if (-not ($ports | Where-Object { [int]$_.port -eq $port -and [string]$_.state -eq 'LISTEN' })) {
        throw "listener $port is missing from port snapshot"
    }

    $conflicts = @(Invoke-CliJson @('port','check',[string]$port))
    $conflicts | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'port-check-listener.json') -Encoding utf8
    if ($conflicts.Count -lt 1) { throw 'listener port conflict was not detected' }
    if ($conflicts | Where-Object { [string]$_.state -ne 'LISTEN' }) { throw 'port conflict includes non-listening sockets' }

    $client = [System.Net.Sockets.TcpClient]::new()
    $client.Connect('127.0.0.1', $port)
    $clientPort = ([System.Net.IPEndPoint]$client.Client.LocalEndPoint).Port
    $clientPort | Set-Content (Join-Path $script:Root 'established-client-port.txt')
    Start-Sleep -Milliseconds 150
    $clientPortConflicts = @(Invoke-CliJson @('port','check',[string]$clientPort))
    $clientPortConflicts | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'port-check-established-client.json') -Encoding utf8
    if ($clientPortConflicts.Count -ne 0) { throw 'established outbound/client socket was incorrectly reported as a listening conflict' }

    $health = Invoke-CliJson @('health','tcp','127.0.0.1',[string]$port)
    $health | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'tcp-health-loopback.json') -Encoding utf8
    if ($health.healthy -ne $true) { throw 'loopback TCP health check failed against active listener' }

    $blockedHealth = Invoke-CliJson @('health','tcp','192.0.2.1','443')
    $blockedHealth | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'tcp-health-non-loopback.json') -Encoding utf8
    if ($blockedHealth.healthy -eq $true) { throw 'non-loopback TCP health target was not rejected' }
    if ([string]$blockedHealth.detail -notmatch 'loopback') { throw 'non-loopback TCP health rejection is not explicit' }

    $logsDir = Join-Path $dataDir 'logs'
    New-Item -ItemType Directory -Force -Path $logsDir | Out-Null
    $boundedLog = Join-Path $logsDir 'pkg02-0214-bounded.log'
    0..5024 | ForEach-Object { "line-$_" } | Set-Content -LiteralPath $boundedLog -Encoding utf8
    $tail = @(Invoke-CliJson @('log','tail',$boundedLog,'999999'))
    $tail | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $script:Root 'log-tail-bounded.json') -Encoding utf8
    if ($tail.Count -ne 5000) { throw "bounded log tail returned $($tail.Count) lines instead of 5000" }
    if ([string]$tail[-1] -ne 'line-5024') { throw 'bounded log tail did not preserve the newest line' }

    $unicodeLog = Join-Path $logsDir 'pkg02-0214-unicode-large.log'
    $payload = ([string][char]0x00E9) * 2200
    $writer = [System.IO.StreamWriter]::new($unicodeLog, $false, [System.Text.UTF8Encoding]::new($false))
    try {
        0..2099 | ForEach-Object { $writer.WriteLine("unicode-$_-$payload") }
    }
    finally { $writer.Dispose() }
    if ((Get-Item -LiteralPath $unicodeLog).Length -le 8MB) { throw 'Unicode log fixture did not cross the 8 MiB tail window' }
    $unicodeTail = @(Invoke-CliJson @('log','tail',$unicodeLog,'3'))
    $unicodeTail | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $script:Root 'log-tail-unicode-large.json') -Encoding utf8
    if ($unicodeTail.Count -ne 3) { throw 'large Unicode log tail did not return requested final lines' }
    if ([string]$unicodeTail[-1] -notmatch '^unicode-2099-') { throw 'large Unicode log tail lost final-line integrity' }

    $outsideLog = Join-Path $sandbox 'outside.log'
    'outside VSN data root' | Set-Content -LiteralPath $outsideLog -Encoding utf8
    $outsideOut = Join-Path $script:Root 'outside-log.stdout'
    $outsideErr = Join-Path $script:Root 'outside-log.stderr'
    $outsideCode = Invoke-CliCapture @('log','tail',$outsideLog,'10') $outsideOut $outsideErr
    $outsideCode | Set-Content (Join-Path $script:Root 'outside-log.exit-code.txt')
    if ($outsideCode -eq 0) { throw 'log tail escaped VSN-owned data containment' }

    $chain = Invoke-CliJson @('audit','verify')
    $chain | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.14';
        artifact='local-diagnostics-windows-source-first-scaffold';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        process_snapshot_bounded_sorted_verified=$true;
        process_cpu_memory_metrics_verified=$true;
        listening_port_inventory_verified=$true;
        established_socket_false_conflict_rejected=$true;
        loopback_tcp_health_verified=$true;
        non_loopback_tcp_health_rejected=$true;
        log_tail_line_bound_verified=$true;
        log_tail_byte_window_unicode_verified=$true;
        log_tail_vsn_data_containment_verified=$true;
        audit_chain_valid=$true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $script:Root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $script:Root 'evidence.json.sha256')
}
finally {
    if ($client) { $client.Dispose() }
    if ($listener) { $listener.Stop() }
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}