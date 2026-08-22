param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Write-JsonFile([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 20 -Compress | Set-Content -LiteralPath $Path -Encoding utf8
}

function Invoke-CliCapture([string[]]$CliArgs, [string]$Stdout, [string]$Stderr) {
    & $script:Cli @CliArgs 1> $Stdout 2> $Stderr
    return $LASTEXITCODE
}

function Assert-FileHash([string]$Path, [string]$Expected, [string]$Message) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "$Message (missing: $Path)" }
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected) { throw "$Message (expected=$Expected actual=$actual)" }
}

$root = Join-Path $PWD 'dist-self-hosted\02.12'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0212-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$project = Join-Path $workspace 'project-a'
$outside = Join-Path $sandbox 'outside'
$nodeArtifact = Join-Path $sandbox 'node-test.exe'
$pythonArtifact = Join-Path $sandbox 'python-test.exe'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$project,$outside,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

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

try {
    if (-not $IsWindows) { throw "02.12 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    if ($env:RUNNER_ENVIRONMENT -and $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw "02.12 certification requires a GitHub-hosted runner; got '$env:RUNNER_ENVIRONMENT'" }
    Write-Host "selected runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "environment=$env:RUNNER_ENVIRONMENT", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $root 'runner.txt')

    $runtimeSource = Get-Content 'crates/vsn-runtime/src/lib.rs' -Raw
    foreach ($needle in @('fn runtime_root_for_registry','fn uninstall_tombstone_path','fn validate_managed_install_path','duplicate target registrations','pub fn uninstall_runtime','pub fn repair_registry')) {
        if (-not $runtimeSource.Contains($needle)) { throw "missing 02.12 runtime source invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('runtime.uninstall','runtime.repair','runtime.audit')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent runtime invariant: $needle" }
    }
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw
    foreach ($needle in @('sub == "uninstall"','sub == "repair"','sub == "audit"')) {
        if (-not $cliSource.Contains($needle)) { throw "missing CLI runtime invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-runtime --package vsn-security --package vsn-core --all-targets --no-deps -- -D warnings
    Assert-LastExit 'runtime/security/core clippy failed'
    cargo test --locked --package vsn-runtime --package vsn-security --package vsn-core
    Assert-LastExit 'runtime/security/core tests failed'
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

    $nodeSource = Join-Path $sandbox 'fake_node.rs'
    @'
fn main() {
    println!("VSN fake node 20.0.0");
}
'@ | Set-Content -LiteralPath $nodeSource -Encoding utf8
    rustc $nodeSource -O -o $nodeArtifact
    Assert-LastExit 'fake node artifact build failed'

    $pythonSource = Join-Path $sandbox 'fake_python.rs'
    @'
fn main() {
    println!("VSN fake python 3.12.0");
}
'@ | Set-Content -LiteralPath $pythonSource -Encoding utf8
    rustc $pythonSource -O -o $pythonArtifact
    Assert-LastExit 'fake python artifact build failed'

    $nodeSha = (Get-FileHash -LiteralPath $nodeArtifact -Algorithm SHA256).Hash.ToLowerInvariant()
    $pythonSha = (Get-FileHash -LiteralPath $pythonArtifact -Algorithm SHA256).Hash.ToLowerInvariant()

    $catalog = Join-Path $sandbox 'catalog.json'
    $trust = Join-Path $sandbox 'trust.json'
    $nodeForward = $nodeArtifact.Replace('\','/')
    $pythonForward = $pythonArtifact.Replace('\','/')
    $catalogValue = [ordered]@{
        schema_version = 1
        provider = 'vsn.pkg02.test'
        runtimes = @(
            [ordered]@{
                runtime = 'node'
                version = '20.0.0'
                artifacts = @([ordered]@{
                    os = 'windows'
                    arch = 'x86_64'
                    url = ('file://' + $nodeForward)
                    sha256 = $nodeSha
                    archive = 'binary'
                    executable_relpath = 'bin/node.exe'
                })
            },
            [ordered]@{
                runtime = 'python'
                version = '3.12.0'
                artifacts = @([ordered]@{
                    os = 'windows'
                    arch = 'x86_64'
                    url = ('file://' + $pythonForward)
                    sha256 = $pythonSha
                    archive = 'binary'
                    executable_relpath = 'bin/python.exe'
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

    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent

    & $script:Cli diagnostics | Set-Content (Join-Path $root 'diagnostics.json') -Encoding utf8
    Assert-LastExit 'diagnostics failed'
    $diag = Get-Content (Join-Path $root 'diagnostics.json') -Raw | ConvertFrom-Json
    $runtimeRoot = Join-Path ([string]$diag.data_dir) 'runtimes'
    $registry = Join-Path $runtimeRoot 'registry.json'
    $nodeInstall = Join-Path $runtimeRoot 'node\20.0.0'
    $pythonInstall = Join-Path $runtimeRoot 'python\3.12.0'
    $nodeExe = Join-Path $nodeInstall 'bin\node.exe'
    $pythonExe = Join-Path $pythonInstall 'bin\python.exe'
    $nodeShim = Join-Path $runtimeRoot 'shims\node.cmd'
    $pythonShim = Join-Path $runtimeRoot 'shims\python.cmd'

    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    & $script:Cli runtime install-trusted $catalog $trust node 20.0.0 | Set-Content (Join-Path $root 'install-node.json') -Encoding utf8
    Assert-LastExit 'trusted node install failed'
    & $script:Cli runtime install-trusted $catalog $trust python 3.12.0 | Set-Content (Join-Path $root 'install-python.json') -Encoding utf8
    Assert-LastExit 'trusted python install failed'
    Assert-FileHash $nodeExe $nodeSha 'installed node digest mismatch'
    Assert-FileHash $pythonExe $pythonSha 'installed python digest mismatch'
    if ((& $nodeShim | Out-String).Trim() -ne 'VSN fake node 20.0.0') { throw 'node shim returned unexpected output' }
    if ((& $pythonShim | Out-String).Trim() -ne 'VSN fake python 3.12.0') { throw 'python shim returned unexpected output' }

    & $script:Cli runtime activate $project node 20.0.0 | Set-Content (Join-Path $root 'activate-node.json') -Encoding utf8
    Assert-LastExit 'node activation failed'
    & $script:Cli runtime activate $project python 3.12.0 | Set-Content (Join-Path $root 'activate-python.json') -Encoding utf8
    Assert-LastExit 'python activation failed'

    $initialRegistry = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $initialRegistry | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'registry-before-uninstall.json') -Encoding utf8
    if (@($initialRegistry.installed).Count -ne 2) { throw 'expected two installed runtimes before uninstall' }
    $activationEntry = @($initialRegistry.project_activation.PSObject.Properties | Where-Object { [string]$_.Value.node -eq '20.0.0' -and [string]$_.Value.python -eq '3.12.0' })
    if ($activationEntry.Count -ne 1) { throw 'expected node and python to share one project activation entry' }
    $projectKey = [string]$activationEntry[0].Name

    $pythonExeHash = (Get-FileHash -LiteralPath $pythonExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $pythonShimHash = (Get-FileHash -LiteralPath $pythonShim -Algorithm SHA256).Hash.ToLowerInvariant()

    & $script:Cli runtime uninstall node 20.0.0 | Set-Content (Join-Path $root 'uninstall-node.json') -Encoding utf8
    Assert-LastExit 'clean node uninstall failed'
    if (Test-Path -LiteralPath $nodeInstall) { throw 'node install directory survived uninstall' }
    if (Test-Path -LiteralPath $nodeShim) { throw 'node shim survived uninstall' }
    Assert-FileHash $pythonExe $pythonExeHash 'node uninstall changed sibling python executable'
    Assert-FileHash $pythonShim $pythonShimHash 'node uninstall changed sibling python shim'
    if ((& $pythonShim | Out-String).Trim() -ne 'VSN fake python 3.12.0') { throw 'node uninstall broke sibling python shim execution' }

    $afterNode = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $afterNode | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'registry-after-node-uninstall.json') -Encoding utf8
    if (@($afterNode.installed | Where-Object { $_.runtime -eq 'node' }).Count -ne 0) { throw 'node registration survived uninstall' }
    if (@($afterNode.installed | Where-Object { $_.runtime -eq 'python' -and $_.version -eq '3.12.0' }).Count -ne 1) { throw 'python sibling registration was damaged' }
    $afterNodeProject = $afterNode.project_activation.PSObject.Properties[$projectKey]
    if (-not $afterNodeProject -or [string]$afterNodeProject.Value.python -ne '3.12.0' -or $afterNodeProject.Value.PSObject.Properties['node']) { throw 'node uninstall did not prune only the target activation' }

    $repeatOut = Join-Path $root 'repeat-uninstall.stdout'
    $repeatErr = Join-Path $root 'repeat-uninstall.stderr'
    $repeatCode = Invoke-CliCapture -CliArgs @('runtime','uninstall','node','20.0.0') -Stdout $repeatOut -Stderr $repeatErr
    $repeatCode | Set-Content (Join-Path $root 'repeat-uninstall.exit-code.txt')
    if ($repeatCode -eq 0) { throw 'repeat uninstall unexpectedly succeeded' }
    Assert-FileHash $pythonExe $pythonExeHash 'repeat node uninstall changed sibling python executable'
    Assert-FileHash $pythonShim $pythonShimHash 'repeat node uninstall changed sibling python shim'

    $outsideSentinel = Join-Path $outside 'keep-node.exe'
    [IO.File]::WriteAllBytes($outsideSentinel, [byte[]](1..64))
    $outsideHash = (Get-FileHash -LiteralPath $outsideSentinel -Algorithm SHA256).Hash.ToLowerInvariant()

    $tamperedRegistry = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $unsafeNode = [pscustomobject]@{
        runtime = 'node'
        version = '20.0.0'
        install_dir = $outside
        executable = $outsideSentinel
        source_sha256 = ('0' * 64)
    }
    $tamperedRegistry.installed = @($tamperedRegistry.installed) + $unsafeNode
    $tamperedProject = $tamperedRegistry.project_activation.PSObject.Properties[$projectKey]
    if (-not $tamperedProject) { throw 'project activation disappeared before tamper case' }
    $tamperedProject.Value | Add-Member -NotePropertyName node -NotePropertyValue '20.0.0' -Force
    $tamperedRegistry | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $registry -Encoding utf8

    $auditBeforeRepair = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $auditBeforeRepair | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'audit-before-repair.json') -Encoding utf8
    if ($auditBeforeRepair.healthy -eq $true) { throw 'tampered outside registration was not detected by audit' }

    $unsafeOut = Join-Path $root 'unsafe-uninstall.stdout'
    $unsafeErr = Join-Path $root 'unsafe-uninstall.stderr'
    $unsafeCode = Invoke-CliCapture -CliArgs @('runtime','uninstall','node','20.0.0') -Stdout $unsafeOut -Stderr $unsafeErr
    $unsafeCode | Set-Content (Join-Path $root 'unsafe-uninstall.exit-code.txt')
    if ($unsafeCode -eq 0) { throw 'unsafe outside registry uninstall unexpectedly succeeded' }
    Assert-FileHash $outsideSentinel $outsideHash 'unsafe uninstall changed outside sentinel'
    Assert-FileHash $pythonExe $pythonExeHash 'unsafe uninstall changed sibling python executable'
    Assert-FileHash $pythonShim $pythonShimHash 'unsafe uninstall changed sibling python shim'

    & $script:Cli runtime repair | Set-Content (Join-Path $root 'repair-unsafe.json') -Encoding utf8
    Assert-LastExit 'runtime repair failed for unsafe registration'
    Assert-FileHash $outsideSentinel $outsideHash 'repair changed outside sentinel'
    Assert-FileHash $pythonExe $pythonExeHash 'repair changed healthy python executable'
    Assert-FileHash $pythonShim $pythonShimHash 'repair changed healthy python shim'

    $afterRepair = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $afterRepair | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'registry-after-repair.json') -Encoding utf8
    if (@($afterRepair.installed | Where-Object { $_.runtime -eq 'node' }).Count -ne 0) { throw 'repair left unsafe node registration behind' }
    if (@($afterRepair.installed | Where-Object { $_.runtime -eq 'python' -and $_.version -eq '3.12.0' }).Count -ne 1) { throw 'repair damaged healthy python registration' }
    $afterRepairProject = $afterRepair.project_activation.PSObject.Properties[$projectKey]
    if (-not $afterRepairProject -or [string]$afterRepairProject.Value.python -ne '3.12.0' -or $afterRepairProject.Value.PSObject.Properties['node']) { throw 'repair did not prune dangling node activation while preserving python' }
    $auditAfterRepair = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $auditAfterRepair | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'audit-after-repair.json') -Encoding utf8
    if ($auditAfterRepair.healthy -ne $true) { throw 'runtime audit remained unhealthy after repair' }
    if ((& $pythonShim | Out-String).Trim() -ne 'VSN fake python 3.12.0') { throw 'repair broke healthy python shim execution' }

    # Duplicate target metadata must also fail before any destructive mutation.
    $duplicateRegistry = Get-Content -LiteralPath $registry -Raw | ConvertFrom-Json
    $pythonRegistration = @($duplicateRegistry.installed | Where-Object { $_.runtime -eq 'python' -and $_.version -eq '3.12.0' })[0]
    $duplicateRegistry.installed = @($duplicateRegistry.installed) + $pythonRegistration
    $duplicateRegistry | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $registry -Encoding utf8
    $duplicateOut = Join-Path $root 'duplicate-uninstall.stdout'
    $duplicateErr = Join-Path $root 'duplicate-uninstall.stderr'
    $duplicateCode = Invoke-CliCapture -CliArgs @('runtime','uninstall','python','3.12.0') -Stdout $duplicateOut -Stderr $duplicateErr
    $duplicateCode | Set-Content (Join-Path $root 'duplicate-uninstall.exit-code.txt')
    if ($duplicateCode -eq 0) { throw 'duplicate target uninstall unexpectedly succeeded' }
    Assert-FileHash $pythonExe $pythonExeHash 'duplicate uninstall changed python executable'
    Assert-FileHash $pythonShim $pythonShimHash 'duplicate uninstall changed python shim'

    & $script:Cli runtime repair | Set-Content (Join-Path $root 'repair-duplicate.json') -Encoding utf8
    Assert-LastExit 'runtime repair failed for duplicate registration'
    $afterDuplicateRepair = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $afterDuplicateRepair | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'registry-after-duplicate-repair.json') -Encoding utf8
    if (@($afterDuplicateRepair.installed | Where-Object { $_.runtime -eq 'python' -and $_.version -eq '3.12.0' }).Count -ne 1) { throw 'duplicate repair did not preserve exactly one healthy python registration' }
    $auditAfterDuplicate = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $auditAfterDuplicate | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'audit-after-duplicate-repair.json') -Encoding utf8
    if ($auditAfterDuplicate.healthy -ne $true) { throw 'audit unhealthy after duplicate repair' }
    Assert-FileHash $pythonExe $pythonExeHash 'duplicate repair changed python executable'
    Assert-FileHash $pythonShim $pythonShimHash 'duplicate repair changed python shim'

    & $script:Cli runtime uninstall python 3.12.0 | Set-Content (Join-Path $root 'uninstall-python-final.json') -Encoding utf8
    Assert-LastExit 'final python uninstall failed'
    if (Test-Path -LiteralPath $pythonInstall) { throw 'python install directory survived final uninstall' }
    if (Test-Path -LiteralPath $pythonShim) { throw 'python shim survived final uninstall' }
    $finalRegistry = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $finalRegistry | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'registry-final.json') -Encoding utf8
    if (@($finalRegistry.installed).Count -ne 0) { throw 'final registry is not empty after clean teardown' }
    if (@($finalRegistry.project_activation.PSObject.Properties).Count -ne 0) { throw 'final project activation registry is not empty' }
    $finalAudit = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $finalAudit | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'audit-final.json') -Encoding utf8
    if ($finalAudit.healthy -ne $true) { throw 'final runtime audit is unhealthy' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $sourceCommit = if ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { (git rev-parse HEAD).Trim() }
    $evidence = [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task = '02.12'
        artifact = 'runtime-uninstall-repair-recovery-windows'
        source_commit = $sourceCommit
        runner = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        os = $env:RUNNER_OS
        arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        rust = $rust
        cargo = $cargoVersion
        checks = [ordered]@{
            clean_target_uninstall = $true
            target_shim_removed = $true
            sibling_runtime_preserved = $true
            repeat_uninstall_rejected = $true
            unsafe_registry_uninstall_rejected = $true
            outside_sentinel_preserved = $true
            repair_removed_unsafe_registration = $true
            repair_pruned_dangling_activation = $true
            post_repair_audit_healthy = $true
            duplicate_target_uninstall_rejected = $true
            duplicate_repair_preserved_one_healthy_registration = $true
            final_clean_teardown = $true
            final_runtime_audit_healthy = $true
            audit_chain_valid = $true
        }
    }
    Write-JsonFile (Join-Path $root 'evidence.json') $evidence
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force -ErrorAction SilentlyContinue
    }
}
