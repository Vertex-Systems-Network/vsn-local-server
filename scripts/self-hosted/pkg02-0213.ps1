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

$root = Join-Path $PWD 'dist-self-hosted\02.13'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0213-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$service = 'VSN-PKG02-0213'
$other = 'OTHER-PKG02-0213'
$agent = $null
$evidence = $null
$acceptanceSucceeded = $false

New-Item -ItemType Directory -Force -Path $root,$bin,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

function Remove-TestService([string]$Name) {
    & sc.exe stop $Name *> $null
    Start-Sleep -Milliseconds 300
    & sc.exe delete $Name *> $null
}

function Start-Agent {
    $agentOut = Join-Path $root 'agent.stdout.log'
    $agentErr = Join-Path $root 'agent.stderr.log'
    $script:agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput $agentOut -RedirectStandardError $agentErr -PassThru -WindowStyle Hidden
    $script:agent.Id | Set-Content (Join-Path $root 'agent.pid')
    $ready = $false
    foreach ($i in 1..80) {
        $script:agent.Refresh()
        if ($script:agent.HasExited) { throw "Agent exited before readiness with code $($script:agent.ExitCode)" }
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { $ready = $true; break }
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

function Get-ServiceViaVsn([string]$Name) {
    $raw = & $script:Cli service status $Name
    Assert-LastExit "service status failed for $Name"
    return ($raw | Out-String | ConvertFrom-Json)
}

function Wait-ServiceState([string]$Name, [string]$Expected, [int]$Seconds = 20) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        $state = Get-ServiceViaVsn $Name
        if ([string]$state.state -eq $Expected) { return $state }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "$Name did not reach state $Expected"
}

function Wait-ServiceAbsent([string]$Name, [int]$Seconds = 20) {
    $deadline = [DateTime]::UtcNow.AddSeconds($Seconds)
    do {
        $existing = Get-Service -Name $Name -ErrorAction SilentlyContinue
        if (-not $existing) { return }
        Start-Sleep -Milliseconds 250
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "$Name still exists after cleanup"
}

try {
    if (-not $IsWindows) { throw "02.13 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    if ($env:RUNNER_ENVIRONMENT -and $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw "02.13 certification requires a GitHub-hosted runner; got '$env:RUNNER_ENVIRONMENT'" }
    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }

    $core = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    $system = Get-Content 'crates/vsn-system/src/lib.rs' -Raw
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('Permission::ServiceView','Permission::ServiceManage','only VSN-managed OS services','name.starts_with("VSN-")')) {
        if (-not $core.Contains($needle)) { throw "missing service boundary invariant: $needle" }
    }
    foreach ($needle in @('pub fn service_state','pub fn service_action','wait_for_windows_service_state','Duration::from_secs(15)')) {
        if (-not $system.Contains($needle)) { throw "missing service lifecycle invariant: $needle" }
    }
    if ($system.Contains('std::thread::sleep(Duration::from_millis(500));')) { throw 'fixed 500ms Windows restart race is still present' }
    if ($agentSource -notmatch 'const SERVICE_NAME:\s*&str\s*=\s*"VSN-[^"]+"') { throw 'production Windows Agent service name is outside VSN-* namespace' }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-system --package vsn-core --package vsn-agent --all-targets --no-deps -- -D warnings
    Assert-LastExit 'service-path clippy failed'
    cargo test --locked --package vsn-system --package vsn-core
    Assert-LastExit 'service-path tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    cargo build --locked --release --package vsn-agent --example pkg02_service_fixture
    Assert-LastExit 'SCM fixture build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    Copy-Item 'target\release\examples\pkg02_service_fixture.exe' (Join-Path $bin 'pkg02_service_fixture.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $fixture = Join-Path $bin 'pkg02_service_fixture.exe'
    (Get-FileHash $fixture -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'fixture.sha256')

    Remove-TestService $service
    Remove-TestService $other

    & sc.exe create $service 'binPath=' ('"' + $fixture + '"') 'start=' 'demand' 'DisplayName=' 'VSN PKG02 02.13 Fixture' | Set-Content (Join-Path $root 'sc-create-vsn.txt')
    Assert-LastExit 'unable to create VSN test service on GitHub-hosted Windows runner'
    & sc.exe create $other 'binPath=' ('"' + $fixture + '"') 'start=' 'demand' 'DisplayName=' 'Non-VSN PKG02 Boundary Fixture' | Set-Content (Join-Path $root 'sc-create-other.txt')
    Assert-LastExit 'unable to create non-VSN boundary service'

    Start-Agent

    $initial = Get-ServiceViaVsn $service
    $initial | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'initial-vsn.json') -Encoding utf8
    if ([string]$initial.state -ne 'stopped') { throw "expected initial stopped state, got $($initial.state)" }
    $otherInitial = Get-ServiceViaVsn $other
    $otherInitial | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'initial-other.json') -Encoding utf8
    if ([string]$otherInitial.name -ne $other -or [string]$otherInitial.state -ne 'stopped') { throw 'ServiceView failed for non-VSN boundary fixture' }

    $otherStartOut = Join-Path $root 'start-other.stdout'
    $otherStartErr = Join-Path $root 'start-other.stderr'
    $otherStartCode = Invoke-CliCapture -CliArgs @('service','start',$other) -Stdout $otherStartOut -Stderr $otherStartErr
    $otherStartCode | Set-Content (Join-Path $root 'start-other.exit-code.txt')
    if ($otherStartCode -eq 0) { throw 'non-VSN service mutation unexpectedly succeeded' }
    if ([string](Get-ServiceViaVsn $other).state -ne 'stopped') { throw 'non-VSN service changed state despite mutation rejection' }

    & $script:Cli service start $service | Set-Content (Join-Path $root 'start-vsn.json') -Encoding utf8
    Assert-LastExit 'VSN service start failed'
    Wait-ServiceState $service 'running' | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'running-after-start.json') -Encoding utf8

    $restartStarted = [DateTime]::UtcNow
    & $script:Cli service restart $service | Set-Content (Join-Path $root 'restart-vsn.json') -Encoding utf8
    Assert-LastExit 'VSN service restart failed against delayed-stop fixture'
    $restartElapsedMs = [int]([DateTime]::UtcNow - $restartStarted).TotalMilliseconds
    $restartElapsedMs | Set-Content (Join-Path $root 'restart-elapsed-ms.txt')
    if ($restartElapsedMs -lt 1500) { throw "restart returned too quickly to have honored delayed SCM stop: ${restartElapsedMs}ms" }
    Wait-ServiceState $service 'running' | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'running-after-restart.json') -Encoding utf8

    & $script:Cli service stop $service | Set-Content (Join-Path $root 'stop-vsn.json') -Encoding utf8
    Assert-LastExit 'VSN service stop failed'
    Wait-ServiceState $service 'stopped' | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'stopped-after-stop.json') -Encoding utf8

    $unsafeOut = Join-Path $root 'unsafe-name.stdout'
    $unsafeErr = Join-Path $root 'unsafe-name.stderr'
    $unsafeCode = Invoke-CliCapture -CliArgs @('service','start','VSN-Bad&whoami') -Stdout $unsafeOut -Stderr $unsafeErr
    $unsafeCode | Set-Content (Join-Path $root 'unsafe-name.exit-code.txt')
    if ($unsafeCode -eq 0) { throw 'unsafe service name unexpectedly succeeded' }

    $conformance = & $script:Cli service conformance | Out-String | ConvertFrom-Json
    $conformance | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'conformance.json') -Encoding utf8
    if ($conformance.valid -ne $true) { throw 'native service provider conformance is invalid' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $sourceCommit = if ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { (git rev-parse HEAD).Trim() }
    $evidence = [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task = '02.13'
        artifact = 'vsn-managed-service-lifecycle-windows'
        source_commit = $sourceCommit
        runner = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        os = $env:RUNNER_OS
        arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        rust = $rust
        cargo = $cargoVersion
        checks = [ordered]@{
            production_agent_service_in_vsn_namespace = $true
            service_view_vsn_fixture = $true
            service_view_non_vsn_fixture = $true
            non_vsn_mutation_rejected = $true
            vsn_start_success = $true
            delayed_restart_waited_for_stopped = $true
            vsn_restart_success = $true
            vsn_stop_success = $true
            unsafe_service_name_rejected = $true
            provider_conformance_valid = $true
            audit_chain_valid = $true
            cleanup_attempted = $true
        }
        restart_elapsed_ms = $restartElapsedMs
    }
    $acceptanceSucceeded = $true
}
finally {
    Stop-Agent
    foreach ($name in @($service,$other)) {
        & sc.exe stop $name *> $null
        Start-Sleep -Seconds 3
        & sc.exe delete $name *> $null
        Wait-ServiceAbsent $name
    }
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force -ErrorAction SilentlyContinue
    }

    $ipcKeyRestored = if ($hadIpcKey) { Test-Path -LiteralPath $ipcKey } else { -not (Test-Path -LiteralPath $ipcKey) }
    $sandboxRemoved = -not (Test-Path -LiteralPath $sandbox)
    if (-not $ipcKeyRestored) { throw 'IPC key state was not restored after 02.13 cleanup' }
    if (-not $sandboxRemoved) { throw '02.13 sandbox still exists after cleanup' }

    if ($acceptanceSucceeded) {
        $evidence.checks['vsn_service_removed'] = $true
        $evidence.checks['non_vsn_service_removed'] = $true
        $evidence.checks['ipc_key_state_restored'] = $ipcKeyRestored
        $evidence.checks['sandbox_removed'] = $sandboxRemoved
        $evidence.checks['cleanup_verified'] = $true
        Write-JsonFile (Join-Path $root 'evidence.json') $evidence
        (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
    }
}
