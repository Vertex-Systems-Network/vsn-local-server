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

function Get-ServiceState([string]$Name, [string]$EvidenceName) {
    $state = Invoke-CliJson @('service','status',$Name)
    $state | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root $EvidenceName) -Encoding utf8
    return $state
}

$script:Root = Join-Path $PWD 'dist-self-hosted\02.13'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0213-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$script:Agent = $null
$script:ServiceCreated = $false
$serviceName = 'VSN-PKG02-0213'

New-Item -ItemType Directory -Force -Path $script:Root,$bin,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

try {
    if (-not $IsWindows) { throw "02.13 acceptance requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $script:Root 'runner.txt')

    $systemSource = Get-Content 'crates/vsn-system/src/lib.rs' -Raw
    foreach ($needle in @(
        'validate_managed_service_name',
        'windows_start_service',
        'windows_stop_service',
        'wait_for_windows_service_state',
        'SERVICE_TRANSITION_TIMEOUT',
        'windows_native_service_name',
        'if name == "VSN-Agent"',
        '"VSNAgent"'
    )) {
        if (-not $systemSource.Contains($needle)) { throw "missing 02.13 system source invariant: $needle" }
    }
    if ($systemSource.Contains('from_millis(500)')) { throw 'fixed 500ms Windows service restart sleep is still present' }

    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('Permission::ServiceView','Permission::ServiceManage','if !name.starts_with("VSN-")')) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.13 Core permission/namespace invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    if (-not $agentSource.Contains('const SERVICE_NAME: &str = "VSNAgent"')) { throw 'Windows Agent native service identity drifted' }
    $wixSource = Get-Content 'packaging/windows/VSN.wxs' -Raw
    if (-not $wixSource.Contains('Name="VSNAgent"')) { throw 'Windows MSI native service identity drifted' }
    $fixtureSource = Get-Content 'apps/agent/examples/pkg02_service_fixture.rs' -Raw
    if (-not $fixtureSource.Contains('VSN-PKG02-0213')) { throw '02.13 service fixture identity missing' }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-system --package vsn-core --package vsn-agent --all-targets -- -D warnings
    Assert-LastExit 'service/core/agent clippy failed'
    cargo test --locked --package vsn-system --package vsn-core --package vsn-agent
    Assert-LastExit 'service/core/agent tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    cargo build --locked --release --package vsn-agent --example pkg02_service_fixture
    Assert-LastExit 'Windows service fixture build failed'

    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    Copy-Item 'target\release\examples\pkg02_service_fixture.exe' (Join-Path $bin 'pkg02_service_fixture.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $fixtureExe = Join-Path $bin 'pkg02_service_fixture.exe'

    & sc.exe query $serviceName *> $null
    if ($LASTEXITCODE -eq 0) { throw "refusing to disturb pre-existing service $serviceName" }

    $quotedFixture = '"' + $fixtureExe + '"'
    & sc.exe create $serviceName 'binPath=' $quotedFixture 'start=' 'demand' 'DisplayName=' 'VSN PKG02 02.13 Fixture' | Set-Content (Join-Path $script:Root 'sc-create.txt') -Encoding utf8
    Assert-LastExit 'fixture service create failed'
    $script:ServiceCreated = $true
    & sc.exe qc $serviceName | Set-Content (Join-Path $script:Root 'sc-qc.txt') -Encoding utf8
    Assert-LastExit 'fixture service configuration query failed'

    Start-Agent
    $conformance = Invoke-CliJson @('service','conformance')
    $conformance | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'conformance.json') -Encoding utf8
    if ($conformance.valid -ne $true) { throw 'native service provider conformance is invalid' }

    $initial = Get-ServiceState $serviceName 'state-initial.json'
    if ($initial.state -ne 'stopped') { throw "fixture expected stopped initially, got $($initial.state)" }

    $start = Invoke-CliJson @('service','start',$serviceName)
    $start | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'action-start.json') -Encoding utf8
    if ($start.state -ne 'running') { throw "start did not converge to running, got $($start.state)" }
    $startAgain = Invoke-CliJson @('service','start',$serviceName)
    $startAgain | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'action-start-idempotent.json') -Encoding utf8
    if ($startAgain.state -ne 'running') { throw 'idempotent start did not remain running' }

    $restart = Invoke-CliJson @('service','restart',$serviceName)
    $restart | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'action-restart.json') -Encoding utf8
    if ($restart.state -ne 'running') { throw "restart did not converge to running, got $($restart.state)" }
    $afterRestart = Get-ServiceState $serviceName 'state-after-restart.json'
    if ($afterRestart.state -ne 'running') { throw 'post-restart status is not running' }

    $stop = Invoke-CliJson @('service','stop',$serviceName)
    $stop | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'action-stop.json') -Encoding utf8
    if ($stop.state -ne 'stopped') { throw "stop did not converge to stopped, got $($stop.state)" }
    $stopAgain = Invoke-CliJson @('service','stop',$serviceName)
    $stopAgain | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'action-stop-idempotent.json') -Encoding utf8
    if ($stopAgain.state -ne 'stopped') { throw 'idempotent stop did not remain stopped' }

    $statusOut = Join-Path $script:Root 'outside-status.stdout'
    $statusErr = Join-Path $script:Root 'outside-status.stderr'
    $statusCode = Invoke-CliCapture @('service','status','Spooler') $statusOut $statusErr
    $statusCode | Set-Content (Join-Path $script:Root 'outside-status.exit-code.txt')
    if ($statusCode -eq 0) { throw 'service status escaped the VSN-managed namespace boundary' }

    $actionOut = Join-Path $script:Root 'outside-action.stdout'
    $actionErr = Join-Path $script:Root 'outside-action.stderr'
    $actionCode = Invoke-CliCapture @('service','start','Spooler') $actionOut $actionErr
    $actionCode | Set-Content (Join-Path $script:Root 'outside-action.exit-code.txt')
    if ($actionCode -eq 0) { throw 'service mutation escaped the VSN-managed namespace boundary' }

    $aliasEvidence = [ordered]@{
        public_name = 'VSN-Agent'
        windows_native_name = 'VSNAgent'
        public_namespace_allowed = $coreSource.Contains('if !name.starts_with("VSN-")')
        compatibility_mapping_present = $systemSource.Contains('windows_native_service_name') -and $systemSource.Contains('if name == "VSN-Agent"')
        agent_native_identity_present = $agentSource.Contains('const SERVICE_NAME: &str = "VSNAgent"')
        wix_native_identity_present = $wixSource.Contains('Name="VSNAgent"')
    }
    $aliasEvidence | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'agent-service-alias.json') -Encoding utf8
    if (-not ($aliasEvidence.public_namespace_allowed -and $aliasEvidence.compatibility_mapping_present -and $aliasEvidence.agent_native_identity_present -and $aliasEvidence.wix_native_identity_present)) {
        throw 'VSN Agent public/native service alias contract is incomplete'
    }

    $chain = Invoke-CliJson @('audit','verify')
    $chain | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.13';
        artifact='managed-os-service-lifecycle-windows-source-first-scaffold';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        service_view_manage_permission_split_verified=$true;
        managed_namespace_status_boundary_verified=$true;
        managed_namespace_mutation_boundary_verified=$true;
        disposable_real_scm_service_verified=$true;
        start_convergence_verified=$true; start_idempotence_verified=$true;
        restart_stop_then_start_convergence_verified=$true;
        stop_convergence_verified=$true; stop_idempotence_verified=$true;
        fixed_restart_sleep_absent=$true;
        windows_agent_public_legacy_alias_contract_verified=$true;
        audit_chain_valid=$true
    } | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $script:Root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $script:Root 'evidence.json.sha256')
}
finally {
    Stop-Agent
    if ($script:ServiceCreated) {
        & sc.exe stop $serviceName *> $null
        for ($i = 0; $i -lt 40; $i++) {
            $query = & sc.exe query $serviceName 2>&1 | Out-String
            if ($query -match 'STOPPED' -or $LASTEXITCODE -ne 0) { break }
            Start-Sleep -Milliseconds 250
        }
        & sc.exe delete $serviceName *> $null
    }
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
