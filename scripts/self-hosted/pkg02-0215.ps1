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

function Set-FakeMode([string]$Mode) {
    $Mode | Set-Content -LiteralPath $script:ModeFile -Encoding ascii
}

$root = Join-Path $PWD 'dist-self-hosted\02.15'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0215-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$fakebin = Join-Path $sandbox 'fakebin'
$modeFile = Join-Path $sandbox 'mode.txt'
$originalLocalAppData = $env:LOCALAPPDATA
$originalPath = $env:PATH
$originalModeFile = $env:VSN_FAKE_CONTAINER_MODE_FILE
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null
$evidence = $null
$acceptanceSucceeded = $false

New-Item -ItemType Directory -Force -Path $root,$bin,$sandbox,$isolatedLocalAppData,$fakebin | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData
$script:Root = $root
$script:Agent = $null
$script:ModeFile = $modeFile

try {
    if (-not $IsWindows) { throw "02.15 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    if ($env:RUNNER_ENVIRONMENT -and $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw "02.15 certification requires a GitHub-hosted runner; got '$env:RUNNER_ENVIRONMENT'" }
    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }

    $container = Get-Content 'crates/vsn-container/src/lib.rs' -Raw
    $core = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw
    $ipcSource = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw

    foreach ($needle in @(
        'BACKEND_PROBE_TIMEOUT',
        'BASELINE_OPERATION_TIMEOUT',
        'BASELINE_LIST_OUTPUT_BYTES',
        'BASELINE_TEXT_OUTPUT_BYTES',
        'BASELINE_ACTION_OUTPUT_BYTES',
        'BASELINE_STATS_OUTPUT_BYTES',
        'BASELINE_MAX_ITEMS',
        'BASELINE_MAX_FIELD_BYTES',
        'fn run_bounded',
        'pub fn detect_all',
        'pub fn list_containers',
        'pub fn container_logs',
        'pub fn container_inspect',
        'pub fn container_stats',
        'pub fn container_action'
    )) {
        if (-not $container.Contains($needle)) { throw "missing 02.15 container invariant: $needle" }
    }
    foreach ($needle in @('Permission::RuntimeView','Permission::RuntimeManage','vsn_container::detect_all','vsn_container::container_action')) {
        if (-not $core.Contains($needle)) { throw "missing 02.15 Core boundary invariant: $needle" }
    }
    foreach ($needle in @('"container.backends"','"container.list"','"container.images"','"container.volumes"','"container.networks"','"container.logs"','"container.inspect"','"container.stats"','"container.action"')) {
        if (-not $agentSource.Contains($needle)) { throw "missing authenticated Agent container route: $needle" }
    }
    foreach ($needle in @('container" && sub == "backends','container" && sub == "list','container" && sub == "logs','container" && sub == "inspect','container" && sub == "stats','"start" | "stop" | "restart" | "pause" | "unpause"')) {
        if (-not $cliSource.Contains($needle)) { throw "missing CLI-to-Agent container route: $needle" }
    }
    if (-not $ipcSource.Contains('const MAX_FRAME_BYTES: usize = 1024 * 1024;')) { throw 'existing 1 MiB IPC frame contract changed unexpectedly' }
    if (-not $ipcSource.Contains('const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);')) { throw 'existing 5-second IPC timeout contract changed unexpectedly' }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-container --package vsn-core --package vsn-agent --package vsn --all-targets --no-deps -- -D warnings
    Assert-LastExit '02.15 path clippy failed'
    cargo test --locked --package vsn-container --package vsn-core
    Assert-LastExit '02.15 package tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $fakeSource = Join-Path $sandbox 'fake_container_backend.rs'
    @'
use std::{env, fs, process, thread, time::Duration};

fn mode() -> String {
    env::var("VSN_FAKE_CONTAINER_MODE_FILE")
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_else(|| "healthy".into())
        .trim()
        .to_string()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = mode();
    if args.first().map(String::as_str) == Some("--version") {
        if mode == "hang-detect" { thread::sleep(Duration::from_secs(10)); }
        println!("Container Engine version 99.0.0, build vsn-test");
        return;
    }
    if args.first().map(String::as_str) == Some("info") {
        if mode == "daemon-down" { eprintln!("daemon unavailable"); process::exit(125); }
        println!("99.0.0");
        return;
    }
    if args.first().map(String::as_str) == Some("ps") {
        if mode == "daemon-down" { eprintln!("daemon unavailable"); process::exit(125); }
        if mode == "huge-read" { print!("{}", "x".repeat(256 * 1024)); return; }
        println!("abc123\tvsn-demo\timage:test\tUp 1 minute\t127.0.0.1:8080->80/tcp");
        return;
    }
    if args.first().map(String::as_str) == Some("image") && args.get(1).map(String::as_str) == Some("ls") {
        println!("img123\timage:test\t10MB"); return;
    }
    if args.first().map(String::as_str) == Some("volume") && args.get(1).map(String::as_str) == Some("ls") {
        println!("vol1\tvol1\tlocal"); return;
    }
    if args.first().map(String::as_str) == Some("network") && args.get(1).map(String::as_str) == Some("ls") {
        println!("net1\tbridge\tbridge"); return;
    }
    if args.first().map(String::as_str) == Some("logs") {
        println!("line-one"); println!("line-two"); return;
    }
    if args.first().map(String::as_str) == Some("inspect") {
        println!("[{{\"Id\":\"abc123\",\"Name\":\"vsn-demo\"}}]"); return;
    }
    if args.first().map(String::as_str) == Some("stats") {
        println!("vsn-demo\t1.00%\t10MiB / 1GiB\t1kB / 2kB\t3kB / 4kB\t5"); return;
    }
    if matches!(args.first().map(String::as_str), Some("start" | "stop" | "restart" | "pause" | "unpause")) {
        if mode == "daemon-down" || mode == "action-fail" { eprintln!("lifecycle failed"); process::exit(42); }
        if mode == "hang-action" { thread::sleep(Duration::from_secs(10)); }
        if mode == "huge-action" { print!("{}", "x".repeat(256 * 1024)); return; }
        println!("{}", args.get(1).map(String::as_str).unwrap_or("vsn-demo")); return;
    }
    eprintln!("unsupported fake container args: {:?}", args);
    process::exit(97);
}
'@ | Set-Content -LiteralPath $fakeSource -Encoding utf8
    rustc $fakeSource -O -o (Join-Path $fakebin 'docker.exe')
    Assert-LastExit 'fake Docker compilation failed'
    Copy-Item (Join-Path $fakebin 'docker.exe') (Join-Path $fakebin 'podman.exe') -Force
    Set-FakeMode 'healthy'
    $env:VSN_FAKE_CONTAINER_MODE_FILE = $modeFile
    $env:PATH = "$fakebin;$originalPath"

    Start-Agent

    Set-FakeMode 'healthy'
    $healthyRaw = & $script:Cli container backends | Out-String
    Assert-LastExit 'healthy backend discovery failed'
    $healthyRaw | Set-Content (Join-Path $root 'backends-healthy.json') -Encoding utf8
    $healthy = @($healthyRaw | ConvertFrom-Json)
    if ($healthy.Count -ne 2 -or [string]$healthy[0].id -ne 'docker' -or [string]$healthy[1].id -ne 'podman') { throw 'Docker/Podman backend ordering changed' }
    foreach ($backend in $healthy) {
        if ($backend.installed -ne $true -or $backend.daemon_reachable -ne $true) { throw "healthy backend discovery failed for $($backend.id)" }
    }

    $listRaw = & $script:Cli container list docker | Out-String
    Assert-LastExit 'container list failed'
    $listRaw | Set-Content (Join-Path $root 'container-list.json') -Encoding utf8
    $list = @($listRaw | ConvertFrom-Json)
    if ($list.Count -ne 1 -or [string]$list[0].name -ne 'vsn-demo') { throw 'container list parsing failed' }
    foreach ($resource in @('images','volumes','networks')) {
        $resourceRaw = & $script:Cli container $resource docker | Out-String
        Assert-LastExit "container $resource failed"
        $resourceRaw | Set-Content (Join-Path $root ("container-$resource.json")) -Encoding utf8
        if (@($resourceRaw | ConvertFrom-Json).Count -ne 1) { throw "container $resource did not return one deterministic resource" }
    }
    $logsRaw = & $script:Cli container logs docker vsn-demo | Out-String
    Assert-LastExit 'container logs failed'
    $logsRaw | Set-Content (Join-Path $root 'container-logs.json') -Encoding utf8
    $inspectRaw = & $script:Cli container inspect docker vsn-demo | Out-String
    Assert-LastExit 'container inspect failed'
    $inspectRaw | Set-Content (Join-Path $root 'container-inspect.json') -Encoding utf8
    $statsRaw = & $script:Cli container stats docker vsn-demo | Out-String
    Assert-LastExit 'container stats failed'
    $statsRaw | Set-Content (Join-Path $root 'container-stats.json') -Encoding utf8
    if ([Text.Encoding]::UTF8.GetByteCount($logsRaw) -ge 900KB -or [Text.Encoding]::UTF8.GetByteCount($inspectRaw) -ge 900KB -or [Text.Encoding]::UTF8.GetByteCount($statsRaw) -ge 900KB) { throw 'container read response exceeded IPC-safe acceptance budget' }

    $startRaw = & $script:Cli container start docker vsn-demo | Out-String
    Assert-LastExit 'healthy lifecycle start failed'
    $startRaw | Set-Content (Join-Path $root 'container-start.json') -Encoding utf8

    $invalidBackendOut = Join-Path $root 'invalid-backend.stdout'
    $invalidBackendErr = Join-Path $root 'invalid-backend.stderr'
    $invalidBackendCode = Invoke-CliCapture -CliArgs @('container','list','invalid') -Stdout $invalidBackendOut -Stderr $invalidBackendErr
    if ($invalidBackendCode -eq 0) { throw 'invalid backend unexpectedly succeeded' }
    $invalidTargetOut = Join-Path $root 'invalid-target.stdout'
    $invalidTargetErr = Join-Path $root 'invalid-target.stderr'
    $invalidTargetCode = Invoke-CliCapture -CliArgs @('container','start','docker','bad target') -Stdout $invalidTargetOut -Stderr $invalidTargetErr
    if ($invalidTargetCode -eq 0) { throw 'invalid container target unexpectedly succeeded' }

    Set-FakeMode 'daemon-down'
    $downRaw = & $script:Cli container backends | Out-String
    Assert-LastExit 'daemon-unavailable backend discovery failed'
    $downRaw | Set-Content (Join-Path $root 'backends-daemon-down.json') -Encoding utf8
    $down = @($downRaw | ConvertFrom-Json)
    foreach ($backend in $down) {
        if ($backend.installed -ne $true -or $backend.daemon_reachable -ne $false) { throw "daemon-unavailable semantics failed for $($backend.id)" }
    }
    $downListCode = Invoke-CliCapture -CliArgs @('container','list','docker') -Stdout (Join-Path $root 'daemon-down-list.stdout') -Stderr (Join-Path $root 'daemon-down-list.stderr')
    if ($downListCode -eq 0) { throw 'container list unexpectedly succeeded while daemon unavailable' }
    $downActionCode = Invoke-CliCapture -CliArgs @('container','start','docker','vsn-demo') -Stdout (Join-Path $root 'daemon-down-action.stdout') -Stderr (Join-Path $root 'daemon-down-action.stderr')
    if ($downActionCode -eq 0) { throw 'container lifecycle unexpectedly succeeded while daemon unavailable' }

    Set-FakeMode 'hang-detect'
    $detectStarted = [Diagnostics.Stopwatch]::StartNew()
    $hungDiscoveryRaw = & $script:Cli container backends | Out-String
    $hungDiscoveryCode = $LASTEXITCODE
    $detectStarted.Stop()
    $discoveryElapsedMs = [int]$detectStarted.ElapsedMilliseconds
    $hungDiscoveryRaw | Set-Content (Join-Path $root 'backends-hung.json') -Encoding utf8
    $discoveryElapsedMs | Set-Content (Join-Path $root 'backends-hung-elapsed-ms.txt')
    if ($hungDiscoveryCode -ne 0) { throw 'bounded discovery must return structured backend state instead of breaking IPC' }
    if ($discoveryElapsedMs -ge 5000) { throw "backend discovery exceeded IPC window: ${discoveryElapsedMs}ms" }
    $hungDiscovery = @($hungDiscoveryRaw | ConvertFrom-Json)
    if ($hungDiscovery | Where-Object { $_.installed -eq $true }) { throw 'timed-out discovery must fail closed as unavailable' }

    Set-FakeMode 'huge-read'
    $readStarted = [Diagnostics.Stopwatch]::StartNew()
    $hugeReadCode = Invoke-CliCapture -CliArgs @('container','list','docker') -Stdout (Join-Path $root 'huge-read.stdout') -Stderr (Join-Path $root 'huge-read.stderr')
    $readStarted.Stop()
    $readElapsedMs = [int]$readStarted.ElapsedMilliseconds
    $readElapsedMs | Set-Content (Join-Path $root 'huge-read-elapsed-ms.txt')
    if ($hugeReadCode -eq 0) { throw 'oversized container read unexpectedly succeeded' }
    if ($readElapsedMs -ge 5000) { throw "oversized container read exceeded IPC window: ${readElapsedMs}ms" }
    & $script:Cli ping *> $null
    Assert-LastExit 'IPC failed after oversized container read'

    Set-FakeMode 'action-fail'
    $failedActionCode = Invoke-CliCapture -CliArgs @('container','restart','docker','vsn-demo') -Stdout (Join-Path $root 'action-fail.stdout') -Stderr (Join-Path $root 'action-fail.stderr')
    if ($failedActionCode -eq 0) { throw 'backend lifecycle failure unexpectedly succeeded' }

    Set-FakeMode 'hang-action'
    $actionStarted = [Diagnostics.Stopwatch]::StartNew()
    $hungActionCode = Invoke-CliCapture -CliArgs @('container','stop','docker','vsn-demo') -Stdout (Join-Path $root 'hang-action.stdout') -Stderr (Join-Path $root 'hang-action.stderr')
    $actionStarted.Stop()
    $actionElapsedMs = [int]$actionStarted.ElapsedMilliseconds
    $actionElapsedMs | Set-Content (Join-Path $root 'hang-action-elapsed-ms.txt')
    if ($hungActionCode -eq 0) { throw 'timed-out lifecycle action unexpectedly succeeded' }
    if ($actionElapsedMs -ge 5000) { throw "container lifecycle timeout exceeded IPC window: ${actionElapsedMs}ms" }
    & $script:Cli ping *> $null
    Assert-LastExit 'IPC failed after lifecycle timeout'

    Set-FakeMode 'huge-action'
    $hugeActionStarted = [Diagnostics.Stopwatch]::StartNew()
    $hugeActionCode = Invoke-CliCapture -CliArgs @('container','restart','docker','vsn-demo') -Stdout (Join-Path $root 'huge-action.stdout') -Stderr (Join-Path $root 'huge-action.stderr')
    $hugeActionStarted.Stop()
    $hugeActionElapsedMs = [int]$hugeActionStarted.ElapsedMilliseconds
    $hugeActionElapsedMs | Set-Content (Join-Path $root 'huge-action-elapsed-ms.txt')
    if ($hugeActionCode -eq 0) { throw 'oversized lifecycle output unexpectedly succeeded' }
    if ($hugeActionElapsedMs -ge 5000) { throw "oversized lifecycle action exceeded IPC window: ${hugeActionElapsedMs}ms" }
    & $script:Cli ping *> $null
    Assert-LastExit 'IPC failed after oversized lifecycle output'

    $auditRaw = & $script:Cli audit verify | Out-String
    Assert-LastExit 'audit verification failed'
    $auditRaw | Set-Content (Join-Path $root 'audit-chain.json') -Encoding utf8
    $auditChain = $auditRaw | ConvertFrom-Json
    if ($auditChain.valid -ne $true) { throw 'audit chain is invalid' }

    $sourceCommit = if ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { (git rev-parse HEAD).Trim() }
    $evidence = [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task = '02.15'
        artifact = 'bounded-container-baseline-windows'
        source_commit = $sourceCommit
        runner = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        os = $env:RUNNER_OS
        arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        rust = $rust
        cargo = $cargoVersion
        discovery_bounded_elapsed_ms = $discoveryElapsedMs
        read_bounded_elapsed_ms = $readElapsedMs
        lifecycle_timeout_elapsed_ms = $actionElapsedMs
        lifecycle_output_elapsed_ms = $hugeActionElapsedMs
        checks = [ordered]@{
            authenticated_agent_cli_boundary = $true
            existing_permissions_enforced = $true
            docker_podman_discovery_order = $true
            healthy_backend_discovery = $true
            unavailable_daemon_reported = $true
            unavailable_daemon_read_rejected = $true
            unavailable_daemon_action_rejected = $true
            normal_container_read = $true
            resource_reads = $true
            logs_inspect_stats_frame_safe = $true
            lifecycle_success = $true
            invalid_backend_rejected = $true
            invalid_target_rejected = $true
            discovery_timeout_bounded = $true
            read_output_bounded = $true
            lifecycle_backend_failure = $true
            lifecycle_timeout_bounded = $true
            lifecycle_output_bounded = $true
            ipc_survived_failures = $true
            audit_chain_valid = $true
            cleanup_attempted = $true
        }
    }
    $acceptanceSucceeded = $true
}
finally {
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    $env:PATH = $originalPath
    if ($null -eq $originalModeFile) {
        Remove-Item Env:VSN_FAKE_CONTAINER_MODE_FILE -ErrorAction SilentlyContinue
    } else {
        $env:VSN_FAKE_CONTAINER_MODE_FILE = $originalModeFile
    }
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force -ErrorAction SilentlyContinue
    }

    $ipcKeyRestored = if ($hadIpcKey) { Test-Path -LiteralPath $ipcKey } else { -not (Test-Path -LiteralPath $ipcKey) }
    $sandboxRemoved = -not (Test-Path -LiteralPath $sandbox)
    if (-not $ipcKeyRestored) { throw 'IPC key state was not restored after 02.15 cleanup' }
    if (-not $sandboxRemoved) { throw '02.15 sandbox still exists after cleanup' }

    if ($acceptanceSucceeded) {
        $evidence.checks['ipc_key_state_restored'] = $ipcKeyRestored
        $evidence.checks['sandbox_removed'] = $sandboxRemoved
        $evidence.checks['cleanup_verified'] = $true
        Write-JsonFile (Join-Path $root 'evidence.json') $evidence
        (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
    }
}
