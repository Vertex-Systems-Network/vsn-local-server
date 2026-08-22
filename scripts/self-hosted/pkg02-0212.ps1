param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Write-JsonFile([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 24 -Compress | Set-Content -LiteralPath $Path -Encoding utf8
}

function Invoke-CliCapture([string[]]$Args, [string]$Stdout, [string]$Stderr) {
    & $script:Cli @Args 1> $Stdout 2> $Stderr
    return $LASTEXITCODE
}

function New-FakeRuntime([string]$Name, [string]$Version, [string]$OutputPath) {
    $source = [IO.Path]::ChangeExtension($OutputPath, '.rs')
    @"
fn main() {
    println!("VSN fake $Name $Version");
}
"@ | Set-Content -LiteralPath $source -Encoding utf8
    rustc $source -O -o $OutputPath
    Assert-LastExit "fake $Name $Version artifact build failed"
}

function Get-Sha([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
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

$script:Root = Join-Path $PWD 'dist-self-hosted\02.12'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0212-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$projectA = Join-Path $workspace 'project-a'
$projectB = Join-Path $workspace 'project-b'
$outside = Join-Path $sandbox 'outside'
$outsideSentinel = Join-Path $outside 'do-not-touch.txt'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$script:Agent = $null

New-Item -ItemType Directory -Force -Path $script:Root,$bin,$projectA,$projectB,$outside,$isolatedLocalAppData | Out-Null
'sentinel' | Set-Content -LiteralPath $outsideSentinel -Encoding utf8
$outsideSentinelSha = Get-Sha $outsideSentinel
$env:LOCALAPPDATA = $isolatedLocalAppData

try {
    if (-not $IsWindows) { throw "02.12 acceptance requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $script:Root 'runner.txt')

    $runtimeSource = Get-Content 'crates/vsn-runtime/src/lib.rs' -Raw
    foreach ($needle in @(
        'pub fn uninstall_runtime',
        'validate_registered_runtime_location',
        'expected_runtime_install_dir',
        'pub fn audit_registry',
        'stale_shim',
        'pub fn repair_registry',
        'find_repair_executable',
        'MAX_REPAIR_SCAN_ENTRIES'
    )) {
        if (-not $runtimeSource.Contains($needle)) { throw "missing 02.12 runtime source invariant: $needle" }
    }
    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('pub fn runtime_uninstall','pub fn runtime_repair','pub fn runtime_audit')) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.12 core source invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('runtime.uninstall','runtime.repair','runtime.audit')) {
        if (-not $agentSource.Contains($needle)) { throw "missing 02.12 Agent route invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-runtime --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'runtime/core clippy failed'
    cargo test --locked --package vsn-runtime --package vsn-core
    Assert-LastExit 'runtime/core tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    cargo build --locked --release --package vsn-security --example pkg02_catalog_sign
    Assert-LastExit 'catalog signer fixture build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $signer = Join-Path $PWD 'target\release\examples\pkg02_catalog_sign.exe'

    $node20Artifact = Join-Path $sandbox 'node-20.exe'
    $node22Artifact = Join-Path $sandbox 'node-22.exe'
    $php84Artifact = Join-Path $sandbox 'php-84.exe'
    New-FakeRuntime 'node' '20.0.0' $node20Artifact
    New-FakeRuntime 'node' '22.0.0' $node22Artifact
    New-FakeRuntime 'php' '8.4.0' $php84Artifact

    $catalog = Join-Path $sandbox 'catalog.json'
    $trust = Join-Path $sandbox 'trust.json'
    $catalogValue = [ordered]@{
        schema_version = 1
        provider = 'vsn.pkg02.0212.test'
        runtimes = @(
            [ordered]@{
                runtime='node'; version='20.0.0'; artifacts=@([ordered]@{
                    os='windows'; arch='x86_64'; url=('file://' + $node20Artifact.Replace('\','/'));
                    sha256=(Get-Sha $node20Artifact); archive='binary'; executable_relpath='bin/node.exe'
                })
            },
            [ordered]@{
                runtime='node'; version='22.0.0'; artifacts=@([ordered]@{
                    os='windows'; arch='x86_64'; url=('file://' + $node22Artifact.Replace('\','/'));
                    sha256=(Get-Sha $node22Artifact); archive='binary'; executable_relpath='bin/node.exe'
                })
            },
            [ordered]@{
                runtime='php'; version='8.4.0'; artifacts=@([ordered]@{
                    os='windows'; arch='x86_64'; url=('file://' + $php84Artifact.Replace('\','/'));
                    sha256=(Get-Sha $php84Artifact); archive='binary'; executable_relpath='bin/php.exe'
                })
            }
        )
        signature = $null
    }
    Write-JsonFile $catalog $catalogValue
    $signed = & $signer $catalog | Out-String | ConvertFrom-Json
    Assert-LastExit 'catalog signer fixture failed'
    if (-not $signed.public_key -or -not $signed.signature) { throw 'catalog signer returned incomplete result' }
    $catalogValue.signature = [string]$signed.signature
    Write-JsonFile $catalog $catalogValue
    Write-JsonFile $trust ([ordered]@{ public_keys = @([string]$signed.public_key) })

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent
    & $script:Cli diagnostics | Set-Content (Join-Path $script:Root 'diagnostics.json') -Encoding utf8
    Assert-LastExit 'diagnostics failed'
    $diag = Get-Content (Join-Path $script:Root 'diagnostics.json') -Raw | ConvertFrom-Json
    $runtimeRoot = Join-Path ([string]$diag.data_dir) 'runtimes'
    $registryPath = Join-Path $runtimeRoot 'registry.json'
    $node20Dir = Join-Path $runtimeRoot 'node\20.0.0'
    $node22Dir = Join-Path $runtimeRoot 'node\22.0.0'
    $php84Dir = Join-Path $runtimeRoot 'php\8.4.0'
    $node20Exe = Join-Path $node20Dir 'bin\node.exe'
    $node22Exe = Join-Path $node22Dir 'bin\node.exe'
    $php84Exe = Join-Path $php84Dir 'bin\php.exe'
    $nodeShim = Join-Path $runtimeRoot 'shims\node.cmd'
    $phpShim = Join-Path $runtimeRoot 'shims\php.cmd'

    & $script:Cli workspace add $workspace | Set-Content (Join-Path $script:Root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    foreach ($runtime in @(
        @{Id='node';Version='20.0.0'},
        @{Id='node';Version='22.0.0'},
        @{Id='php';Version='8.4.0'},
        @{Id='node';Version='20.0.0'}
    )) {
        & $script:Cli runtime install-trusted $catalog $trust $runtime.Id $runtime.Version | Out-Null
        Assert-LastExit "install $($runtime.Id)@$($runtime.Version) failed"
    }
    if ((& $nodeShim | Out-String).Trim() -ne 'VSN fake node 20.0.0') { throw 'pre-uninstall node shim does not target node 20' }
    if ((& $phpShim | Out-String).Trim() -ne 'VSN fake php 8.4.0') { throw 'pre-uninstall php shim is invalid' }

    & $script:Cli runtime activate $projectA node 20.0.0 | Out-Null
    Assert-LastExit 'node 20 activation failed'
    & $script:Cli runtime activate $projectB node 22.0.0 | Out-Null
    Assert-LastExit 'node 22 activation failed'
    & $script:Cli runtime activate $projectB php 8.4.0 | Out-Null
    Assert-LastExit 'php activation failed'

    $auditBefore = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $auditBefore | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'audit-before.json') -Encoding utf8
    if ($auditBefore.healthy -ne $true) { throw 'runtime audit is unhealthy before uninstall' }

    $node22ShaBefore = Get-Sha $node22Exe
    $php84ShaBefore = Get-Sha $php84Exe
    & $script:Cli runtime uninstall node 20.0.0 | Set-Content (Join-Path $script:Root 'uninstall-node20.json') -Encoding utf8
    Assert-LastExit 'node 20 uninstall failed'
    if (Test-Path -LiteralPath $node20Dir) { throw 'node 20 install directory survived uninstall' }
    if (-not (Test-Path -LiteralPath $node22Exe -PathType Leaf)) { throw 'node 22 was damaged by node 20 uninstall' }
    if (-not (Test-Path -LiteralPath $php84Exe -PathType Leaf)) { throw 'php was damaged by node 20 uninstall' }
    if ((Get-Sha $node22Exe) -ne $node22ShaBefore) { throw 'node 22 digest changed during node 20 uninstall' }
    if ((Get-Sha $php84Exe) -ne $php84ShaBefore) { throw 'php digest changed during node 20 uninstall' }
    if ((& $nodeShim | Out-String).Trim() -ne 'VSN fake node 22.0.0') { throw 'node shim was not repointed to the surviving node version' }
    if ((& $phpShim | Out-String).Trim() -ne 'VSN fake php 8.4.0') { throw 'php shim changed during unrelated node uninstall' }

    $registryAfterUninstall = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $registryAfterUninstall | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'registry-after-node20-uninstall.json') -Encoding utf8
    if (@($registryAfterUninstall.installed | Where-Object { $_.runtime -eq 'node' -and $_.version -eq '20.0.0' }).Count -ne 0) { throw 'node 20 registry entry survived uninstall' }
    if (@($registryAfterUninstall.installed | Where-Object { $_.runtime -eq 'node' -and $_.version -eq '22.0.0' }).Count -ne 1) { throw 'node 22 registry entry was damaged' }
    $activationText = $registryAfterUninstall.project_activation | ConvertTo-Json -Depth 12 -Compress
    if ($activationText -match '20\.0\.0') { throw 'node 20 project activation survived uninstall' }
    if ($activationText -notmatch '22\.0\.0') { throw 'node 22 project activation was removed by unrelated uninstall' }
    if ($activationText -notmatch '8\.4\.0') { throw 'php project activation was removed by unrelated uninstall' }

    Stop-Agent
    $registryRaw = Get-Content -LiteralPath $registryPath -Raw | ConvertFrom-Json
    $node22Entry = @($registryRaw.installed | Where-Object { $_.runtime -eq 'node' -and $_.version -eq '22.0.0' })[0]
    $node22Entry.install_dir = $outside
    $node22Entry.executable = (Join-Path $outside 'node.exe')
    Write-JsonFile $registryPath $registryRaw
    Start-Agent

    $auditCorrupt = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $auditCorrupt | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'audit-corrupt.json') -Encoding utf8
    if ($auditCorrupt.healthy -ne $false) { throw 'corrupted runtime registry unexpectedly audited healthy' }
    if (-not (@($auditCorrupt.issues | Where-Object { $_.code -eq 'install_dir_escape' }).Count)) { throw 'audit did not flag escaped install directory' }

    $unsafeOut = Join-Path $script:Root 'unsafe-uninstall.stdout'
    $unsafeErr = Join-Path $script:Root 'unsafe-uninstall.stderr'
    $unsafeCode = Invoke-CliCapture @('runtime','uninstall','node','22.0.0') $unsafeOut $unsafeErr
    $unsafeCode | Set-Content (Join-Path $script:Root 'unsafe-uninstall.exit-code.txt')
    if ($unsafeCode -eq 0) { throw 'uninstall accepted a corrupted escaped runtime registration' }
    if ((Get-Sha $outsideSentinel) -ne $outsideSentinelSha) { throw 'unsafe uninstall changed the outside sentinel' }
    if ((Get-Sha $php84Exe) -ne $php84ShaBefore) { throw 'unsafe uninstall damaged unrelated php runtime' }

    & $script:Cli runtime repair | Set-Content (Join-Path $script:Root 'repair.json') -Encoding utf8
    Assert-LastExit 'runtime repair failed'
    $registryAfterRepair = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $registryAfterRepair | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'registry-after-repair.json') -Encoding utf8
    $node22Repaired = @($registryAfterRepair.installed | Where-Object { $_.runtime -eq 'node' -and $_.version -eq '22.0.0' })
    if ($node22Repaired.Count -ne 1) { throw 'repair did not preserve node 22 registration' }
    if ([IO.Path]::GetFullPath([string]$node22Repaired[0].install_dir) -ne [IO.Path]::GetFullPath($node22Dir)) { throw 'repair did not restore canonical node 22 install directory' }
    if ([IO.Path]::GetFullPath([string]$node22Repaired[0].executable) -ne [IO.Path]::GetFullPath($node22Exe)) { throw 'repair did not recover node 22 executable path' }
    if ((Get-Sha $node22Exe) -ne $node22ShaBefore) { throw 'repair changed node 22 payload' }
    if ((Get-Sha $php84Exe) -ne $php84ShaBefore) { throw 'repair changed unrelated php payload' }
    if ((& $nodeShim | Out-String).Trim() -ne 'VSN fake node 22.0.0') { throw 'repair did not restore node shim' }
    if ((& $phpShim | Out-String).Trim() -ne 'VSN fake php 8.4.0') { throw 'repair changed unrelated php shim' }

    $auditAfterRepair = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $auditAfterRepair | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'audit-after-repair.json') -Encoding utf8
    if ($auditAfterRepair.healthy -ne $true) { throw 'runtime audit remained unhealthy after repair' }

    & $script:Cli runtime uninstall node 22.0.0 | Set-Content (Join-Path $script:Root 'uninstall-node22.json') -Encoding utf8
    Assert-LastExit 'node 22 uninstall failed after repair'
    if (Test-Path -LiteralPath $node22Dir) { throw 'node 22 install directory survived uninstall' }
    if (Test-Path -LiteralPath $nodeShim) { throw 'node shim survived removal of the final node version' }
    if (-not (Test-Path -LiteralPath $php84Exe -PathType Leaf)) { throw 'final node uninstall damaged php runtime' }
    if ((Get-Sha $php84Exe) -ne $php84ShaBefore) { throw 'final node uninstall changed php payload' }
    if ((& $phpShim | Out-String).Trim() -ne 'VSN fake php 8.4.0') { throw 'final node uninstall changed php shim' }

    $auditFinal = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $auditFinal | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'audit-final.json') -Encoding utf8
    if ($auditFinal.healthy -ne $true) { throw 'final runtime audit is unhealthy' }
    if ($auditFinal.installed -ne 1) { throw "expected only php runtime after final node uninstall, got $($auditFinal.installed)" }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.12';
        artifact='runtime-uninstall-repair-recovery-windows-source-first-scaffold';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        exact_version_uninstall_verified=$true; unrelated_runtime_preservation_verified=$true;
        activation_cleanup_scoped_to_removed_version=$true; surviving_version_shim_repoint_verified=$true;
        final_version_shim_removal_verified=$true; corrupted_registry_uninstall_fail_closed=$true;
        outside_sentinel_preserved=$true; repair_expected_path_recovery_verified=$true;
        repair_executable_recovery_verified=$true; post_repair_audit_healthy=$true;
        final_audit_healthy=$true; audit_chain_valid=$true
    } | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $script:Root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $script:Root 'evidence.json.sha256')
}
finally {
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
