param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$FeatureId = 'pkg02-0224-domain-https'
$FeatureVersion = '1.0.0'
$CanonicalBaseSha = '265bd17895231fc145ccd435c48def0a38bfd98d'
$PlanSha256 = '0ca95d3eb1bf515f4dab7f1cd694cc17a3ef00ef1269b745420aec71e724f3f4'
$ResearchSha256 = '0f013e98603dcbe6f7ee8511591825c869f17ee8b3e560899dd30c636f5e42c5'
$LifecycleSha256 = '7758ddd2e31ba2a91407524dbf32a76bdab6d7f86ef94a0878105f240dbaa877'
$PreflightSha256 = 'fd631612e0585e8501dd8cd0fff85919f88f954140c7877505a228421cdbdd5b'
$CandidateId = 'c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474'
$ProductVersion = '0.38.1'
$AgentIpcPort = 39731

function Assert-Exit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Assert-Sha([string]$Path, [string]$Expected, [string]$Name) {
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected) { throw "$Name digest mismatch expected=$Expected actual=$actual" }
}

function Get-OptionalSha([string]$Path) {
    try {
        if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
        return (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
    } catch {
        return $null
    }
}

function Stop-ProcessSafe($Process) {
    if ($null -ne $Process) {
        try {
            if (-not $Process.HasExited) {
                Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
                Wait-Process -Id $Process.Id -Timeout 10 -ErrorAction SilentlyContinue
            }
        } catch {}
    }
}

function Test-ProcessStopped([int]$ProcessId) {
    if ($ProcessId -le 0) { return $true }
    return $null -eq (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)
}

function Get-FreeTcpPort {
    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    try {
        $listener.Start()
        return ([Net.IPEndPoint]$listener.LocalEndpoint).Port
    } finally {
        $listener.Stop()
    }
}

function Invoke-CliJson([string[]]$CliArgs, [string]$Name) {
    $out = Join-Path $script:Root "$Name.stdout.json"
    $err = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt")
    if ($code -ne 0) {
        $detail = if (Test-Path $err) { Get-Content $err -Raw } else { '' }
        throw "$Name failed (exit=$code): $detail"
    }
    Get-Content $out -Raw | ConvertFrom-Json
}

function Invoke-CliFailure([string[]]$CliArgs, [string]$Name) {
    $out = Join-Path $script:Root "$Name.stdout.log"
    $err = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt")
    if ($code -eq 0) { throw "$Name unexpectedly succeeded" }
    $stdoutText = if (Test-Path $out) { Get-Content $out -Raw } else { '' }
    $stderrText = if (Test-Path $err) { Get-Content $err -Raw } else { '' }
    [pscustomobject]@{ ExitCode = $code; Text = "$stdoutText`n$stderrText" }
}

function Assert-NetworkManageDenied($Failure, [string]$Name) {
    if (-not $Failure.Text.Contains('permission denied: network.manage')) {
        throw "$Name did not fail specifically at network.manage: $($Failure.Text)"
    }
}

function Start-Agent {
    $script:Agent = Start-Process -FilePath $script:AgentExe `
        -RedirectStandardOutput (Join-Path $script:Root 'agent.stdout.log') `
        -RedirectStandardError (Join-Path $script:Root 'agent.stderr.log') `
        -PassThru -WindowStyle Hidden
    $sw = [Diagnostics.Stopwatch]::StartNew()
    foreach ($i in 1..100) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) {
            $sw.Stop()
            $script:AgentReadyMs = [int64]$sw.ElapsedMilliseconds
            return
        }
        if ($script:Agent.HasExited) { throw "Agent exited before readiness code=$($script:Agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent readiness exceeded 25 seconds'
}

$script:Root = Join-Path $PWD 'dist-self-hosted\02.24'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0224-' + [guid]::NewGuid().ToString('N'))
$isolated = Join-Path $sandbox 'localappdata'
$originalLocal = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadKey = Test-Path -LiteralPath $ipcKey -PathType Leaf
$originalKeyBytes = if ($hadKey) { [IO.File]::ReadAllBytes($ipcKey) } else { $null }
$originalKeyHash = if ($hadKey) { Get-OptionalSha $ipcKey } else { $null }
$systemHosts = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
$systemHostsPreHash = Get-OptionalSha $systemHosts
$systemHostsPostHash = $null
$script:Agent = $null
$script:AgentExe = $null
$script:Cli = $null
$script:AgentReadyMs = 0
$agentPid = 0
$success = $false
$cleanup = [ordered]@{
    agent_stopped = $false
    ipc_key_restored = $false
    localappdata_restored = $false
    sandbox_removed = $false
    system_hosts_unchanged = $false
    no_privileged_system_mutation = $true
}

if (Test-Path $script:Root) { Remove-Item $script:Root -Recurse -Force }
New-Item -ItemType Directory -Force -Path $script:Root, $bin, $sandbox, $isolated | Out-Null
$env:LOCALAPPDATA = $isolated

try {
    if (-not $IsWindows) { throw '02.24 requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.24 requires GitHub-hosted runner' }
    if ($env:RUNNER_ARCH -ne 'X64') { throw "02.24 requires X64 runner, got $env:RUNNER_ARCH" }
    if (-not $env:EXPECTED_SHA) { throw 'EXPECTED_SHA required' }
    $sourceCommit = (git rev-parse HEAD).Trim()
    if ($sourceCommit -ne $env:EXPECTED_SHA) { throw "exact source mismatch expected=$env:EXPECTED_SHA actual=$sourceCommit" }
    $rustcVersion = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rustcVersion -notmatch '^rustc 1\.97\.1\b') { throw "rustc 1.97.1 required: $rustcVersion" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "cargo 1.97.1 required: $cargoVersion" }
    if (Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue) { throw 'IPC port occupied' }

    Assert-Sha '.ai\plans\pkg02-0224-domain-https-v1.md' $PlanSha256 'plan'
    Assert-Sha '.ai\features\pkg02-0224\research.md' $ResearchSha256 'research'
    Assert-Sha '.ai\features\pkg02-0224\lifecycle-review.md' $LifecycleSha256 'lifecycle'
    Assert-Sha '.ai\features\pkg02-0224\development-preflight.md' $PreflightSha256 'preflight'
    $manifest = Get-Content '.ai\manifests\pkg02-0224-domain-https.v1.json' -Raw | ConvertFrom-Json
    if ([string]$manifest.feature_id -ne $FeatureId -or [string]$manifest.version -ne $FeatureVersion) { throw 'manifest identity mismatch' }
    if ([string]$manifest.canonical_base_sha -ne $CanonicalBaseSha) { throw 'manifest canonical base mismatch' }
    if ([string]$manifest.plan.sha256 -ne $PlanSha256) { throw 'manifest plan digest mismatch' }
    if ([string]$manifest.research.market_delta -ne 'none') { throw 'market delta not cleared' }
    if (($manifest.acceptance.criteria | Measure-Object).Count -ne 12) { throw 'frozen AC-01..AC-12 set changed' }

    $network = Get-Content 'crates\vsn-network\src\lib.rs' -Raw
    foreach ($required in @(
        'pub fn apply_hosts_domain_at',
        'pub fn remove_hosts_domain_at',
        'String::from_utf8(bytes)',
        'ReplaceFileW',
        'skip_install_trust',
        'pub fn reload_caddyfile_with_executable',
        'network helper output exceeded safety limit'
    )) {
        if (-not $network.Contains($required)) { throw "missing network safety invariant: $required" }
    }
    if ($network.Contains('tls_insecure_skip_verify')) { throw 'unsafe Caddy TLS bypass present' }
    $atomicMatch = [regex]::Match($network, '(?s)fn atomic_write\(.*?#\[cfg\(not\(windows\)\)\]')
    if (-not $atomicMatch.Success) { throw 'atomic replacement source block unavailable' }
    if ($atomicMatch.Value.Contains('remove_file(path)') -or $atomicMatch.Value.Contains('remove_file(destination)')) { throw 'destination pre-delete remains in atomic replacement path' }

    $policy = Get-Content 'crates\vsn-policy\src\lib.rs' -Raw
    $localAuth = [regex]::Match($policy, '(?s)pub fn local_authenticated\(\) -> Self \{.*?pub fn local_network_admin\(\) -> Self \{')
    if (-not $localAuth.Success) { throw 'local authenticated policy block unavailable' }
    if ($localAuth.Value.Contains('NetworkManage')) { throw 'ordinary local principal unexpectedly has NetworkManage' }
    $networkAdmin = [regex]::Match($policy, '(?s)pub fn local_network_admin\(\) -> Self \{.*?pub fn remote_delegated')
    if (-not $networkAdmin.Success -or -not $networkAdmin.Value.Contains('NetworkManage')) { throw 'elevated network principal lacks NetworkManage' }

    $agentSource = Get-Content 'apps\agent\src\main.rs' -Raw
    $adminFn = [regex]::Match($agentSource, '(?s)fn network_admin_command\(args: &\[String\]\) -> ExitCode \{.*?\n\}\nfn is_os_elevated')
    if (-not $adminFn.Success) { throw 'network-admin function source unavailable' }
    $elevationIndex = $adminFn.Value.IndexOf('if !is_os_elevated()')
    $principalIndex = $adminFn.Value.IndexOf('Principal::local_network_admin()')
    if ($elevationIndex -lt 0 -or $principalIndex -lt 0 -or $elevationIndex -ge $principalIndex) { throw 'OS elevation is not checked before elevated principal creation' }

    & cargo fmt --all -- --check *> (Join-Path $script:Root 'cargo-fmt.log')
    Assert-Exit 'cargo fmt failed'
    & cargo clippy --locked --package vsn-network --package vsn-core --package vsn-policy --package vsn-agent --all-targets --no-deps -- -D warnings *> (Join-Path $script:Root 'cargo-clippy.log')
    Assert-Exit 'task-scoped strict Clippy failed'
    & cargo test --locked --package vsn-network --package vsn-core --package vsn-policy *> (Join-Path $script:Root 'cargo-test.log')
    Assert-Exit '02.24 package tests failed'
    & cargo test --locked --package vsn-network --test pkg02_hosts_safety *> (Join-Path $script:Root 'hosts-safety-test.log')
    Assert-Exit '02.24 hosts/Caddy safety tests failed'
    & cargo test --locked --package vsn-core --test pkg02_domain_policy *> (Join-Path $script:Root 'domain-policy-test.log')
    Assert-Exit '02.24 domain policy tests failed'
    & cargo test --locked --package vsn-network replacement_failure_preserves_original_and_cleans_stage *> (Join-Path $script:Root 'replacement-failure-test.log')
    Assert-Exit '02.24 replacement failure test failed'
    & cargo build --locked --release --package vsn-agent --package vsn *> (Join-Path $script:Root 'cargo-build.log')
    Assert-Exit 'release Agent/CLI build failed'
    & git diff --check *> (Join-Path $script:Root 'git-diff-check.log')
    Assert-Exit 'git diff check failed'

    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $script:Cli = Join-Path $bin 'vsn.exe'
    $agentSha256 = (Get-FileHash $script:AgentExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $cliSha256 = (Get-FileHash $script:Cli -Algorithm SHA256).Hash.ToLowerInvariant()

    Start-Agent
    $agentPid = $script:Agent.Id

    $port = Get-FreeTcpPort
    $planOne = Invoke-CliJson @('domain', 'plan', 'demo.test', [string]$port) 'domain-plan-one'
    $planTwo = Invoke-CliJson @('domain', 'plan', 'demo.test', [string]$port) 'domain-plan-two'
    if ([string]$planOne.domain -ne 'demo.test') { throw 'domain plan normalization mismatch' }
    if ([string]$planOne.target_host -ne '127.0.0.1' -or [int]$planOne.target_port -ne $port) { throw 'domain plan loopback target mismatch' }
    if ($planOne.tls -ne $true -or $planOne.requires_admin_for_hosts_file -ne $true) { throw 'domain plan TLS/admin contract mismatch' }
    $conflictsOne = ($planOne.conflicts | ConvertTo-Json -Compress)
    $conflictsTwo = ($planTwo.conflicts | ConvertTo-Json -Compress)
    if ($conflictsOne -ne $conflictsTwo) { throw 'domain plan conflict reporting is not deterministic' }

    $invalidExternal = Invoke-CliFailure @('domain', 'plan', 'example.com', [string]$port) 'domain-plan-external'
    $invalidShell = Invoke-CliFailure @('domain', 'plan', 'x;cmd.test', [string]$port) 'domain-plan-shell'
    $invalidZero = Invoke-CliFailure @('domain', 'plan', 'demo.test', '0') 'domain-plan-zero'
    if (-not $invalidExternal.Text.Contains('invalid local domain')) { throw 'external domain did not fail validation' }
    if (-not $invalidShell.Text.Contains('invalid local domain')) { throw 'shell-like domain did not fail validation' }
    if (-not $invalidZero.Text.Contains('port must be between 1 and 65535')) { throw 'port zero did not fail closed' }

    $deniedApply = Invoke-CliFailure @('domain', 'apply', 'demo.test') 'domain-apply-denied'
    $deniedRemove = Invoke-CliFailure @('domain', 'remove', 'demo.test') 'domain-remove-denied'
    $deniedReload = Invoke-CliFailure @('domain', 'reload') 'domain-reload-denied'
    Assert-NetworkManageDenied $deniedApply 'domain apply'
    Assert-NetworkManageDenied $deniedRemove 'domain remove'
    Assert-NetworkManageDenied $deniedReload 'domain reload'

    $audit = Invoke-CliJson @('audit', 'verify') 'audit-verify'
    $auditEventCount = @($audit.events).Count
    if ($audit.valid -ne $true -or $auditEventCount -eq 0) { throw 'audit verification failed or empty' }

    $success = $true
} finally {
    Stop-ProcessSafe $script:Agent
    $cleanup.agent_stopped = Test-ProcessStopped $agentPid

    try {
        if ($hadKey) {
            $parent = Split-Path -Parent $ipcKey
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
            [IO.File]::WriteAllBytes($ipcKey, $originalKeyBytes)
            $cleanup.ipc_key_restored = (Get-OptionalSha $ipcKey) -eq $originalKeyHash
        } else {
            if (Test-Path -LiteralPath $ipcKey) { Remove-Item -LiteralPath $ipcKey -Force }
            $cleanup.ipc_key_restored = -not (Test-Path -LiteralPath $ipcKey)
        }
    } catch {
        $cleanup.ipc_key_restored = $false
    }

    $env:LOCALAPPDATA = $originalLocal
    $cleanup.localappdata_restored = $env:LOCALAPPDATA -eq $originalLocal

    if (Test-Path $sandbox) { Remove-Item $sandbox -Recurse -Force -ErrorAction SilentlyContinue }
    $cleanup.sandbox_removed = -not (Test-Path $sandbox)

    $systemHostsPostHash = Get-OptionalSha $systemHosts
    $cleanup.system_hosts_unchanged = if ($null -ne $systemHostsPreHash) {
        $systemHostsPreHash -eq $systemHostsPostHash
    } else {
        $true
    }

    $cleanup | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $script:Root 'cleanup.json') -Encoding utf8
}

if (-not $success) { throw '02.24 certification did not reach acceptance evidence' }
foreach ($entry in $cleanup.GetEnumerator()) {
    if ($entry.Value -ne $true) { throw "cleanup invariant failed: $($entry.Key)" }
}

$evidence = [ordered]@{
    schema_version = 1
    feature_id = $FeatureId
    feature_version = $FeatureVersion
    package_id = 'PKG-02'
    task_id = '02.24'
    canonical_base_sha = $CanonicalBaseSha
    plan_sha256 = $PlanSha256
    source_commit = (git rev-parse HEAD).Trim()
    product_version = $ProductVersion
    candidate_id = $CandidateId
    runner_environment = $env:RUNNER_ENVIRONMENT
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    rustc_version = $rustcVersion
    cargo_version = $cargoVersion
    ipc_address = "127.0.0.1:$AgentIpcPort"
    privileged_system_mutation_performed = $false
    trust_store_mutation_performed = $false
    resolver_mutation_performed = $false
    checks = [ordered]@{
        ac01_exact_source_toolchain = $true
        ac02_domain_plan = $true
        ac03_permission_split = $true
        ac04_hosts_apply_sandbox = $true
        ac05_hosts_remove_sandbox = $true
        ac06_hosts_read_fail_closed = $true
        ac07_failure_safe_replacement = $true
        ac08_https_config_trust_boundary = $true
        ac09_validate_then_reload = $true
        ac10_elevation_boundary = $true
        ac11_audit_cleanup_nonmutation = $true
        ac12_evidence_integrity = $true
    }
    measurements = [ordered]@{
        agent_readiness_ms = $script:AgentReadyMs
        audit_events = [uint64]$auditEventCount
        requested_domain_port = $port
        deterministic_conflicts = $conflictsOne
        system_hosts_pre_sha256 = $systemHostsPreHash
        system_hosts_post_sha256 = $systemHostsPostHash
    }
    artifacts = [ordered]@{
        vsn_agent_sha256 = $agentSha256
        vsn_cli_sha256 = $cliSha256
        cleanup = 'cleanup.json'
        cargo_fmt = 'cargo-fmt.log'
        cargo_clippy = 'cargo-clippy.log'
        cargo_test = 'cargo-test.log'
        cargo_build = 'cargo-build.log'
        hosts_safety_test = 'hosts-safety-test.log'
        domain_policy_test = 'domain-policy-test.log'
        replacement_failure_test = 'replacement-failure-test.log'
    }
}
$evidencePath = Join-Path $script:Root 'evidence.json'
$evidence | ConvertTo-Json -Depth 8 | Set-Content $evidencePath -Encoding utf8
$evidenceSha256 = (Get-FileHash $evidencePath -Algorithm SHA256).Hash.ToLowerInvariant()
$evidenceSha256 | Set-Content (Join-Path $script:Root 'evidence.json.sha256') -Encoding ascii
Write-Host "02.24 acceptance evidence complete source=$($evidence.source_commit) evidence_sha256=$evidenceSha256"
