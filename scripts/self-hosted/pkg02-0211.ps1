param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Write-JsonFile([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 16 -Compress | Set-Content -LiteralPath $Path -Encoding utf8
}

function Invoke-CliCapture([string[]]$Args, [string]$Stdout, [string]$Stderr) {
    & $script:Cli @Args 1> $Stdout 2> $Stderr
    return $LASTEXITCODE
}

$root = Join-Path $PWD 'dist-self-hosted\02.11'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0211-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$project = Join-Path $workspace 'project-a'
$outside = Join-Path $sandbox 'outside'
$artifact = Join-Path $sandbox 'node-test.exe'
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
    if (-not $IsWindows) { throw "02.11 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $root 'runner.txt')

    $core = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('pub fn runtime_install_trusted','load_catalog_verified','install_from_artifact','register_runtime','write_shim','pub fn runtime_activate')) {
        if (-not $core.Contains($needle)) { throw "missing install/activation source invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('runtime.install-trusted','runtime.activate')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent runtime invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-runtime --package vsn-security --package vsn-core --all-targets -- -D warnings
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

    $fakeSource = Join-Path $sandbox 'fake_node.rs'
    @'
fn main() {
    println!("VSN fake node 20.0.0");
}
'@ | Set-Content -LiteralPath $fakeSource -Encoding utf8
    rustc $fakeSource -O -o $artifact
    Assert-LastExit 'fake node artifact build failed'
    $artifactSha = (Get-FileHash $artifact -Algorithm SHA256).Hash.ToLowerInvariant()

    $catalog = Join-Path $sandbox 'catalog.json'
    $trust = Join-Path $sandbox 'trust.json'
    $forwardArtifact = $artifact.Replace('\','/')
    $catalogValue = [ordered]@{
        schema_version = 1
        provider = 'vsn.pkg02.test'
        runtimes = @([ordered]@{
            runtime = 'node'
            version = '20.0.0'
            artifacts = @([ordered]@{
                os = 'windows'
                arch = 'x86_64'
                url = ('file://' + $forwardArtifact)
                sha256 = $artifactSha
                archive = 'binary'
                executable_relpath = 'bin/node.exe'
            })
        })
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
    & $script:Cli diagnostics | Set-Content (Join-Path $root 'diagnostics.json') -Encoding utf8
    Assert-LastExit 'diagnostics failed'
    $diag = Get-Content (Join-Path $root 'diagnostics.json') -Raw | ConvertFrom-Json
    $runtimeRoot = Join-Path ([string]$diag.data_dir) 'runtimes'
    $registry = Join-Path $runtimeRoot 'registry.json'
    $installDir = Join-Path $runtimeRoot 'node\20.0.0'
    $shimDir = Join-Path $runtimeRoot 'shims'
    $shim = Join-Path $shimDir 'node.cmd'

    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    # Force the final shim stage to fail. A correct transaction must roll back both the
    # installed directory and the registry entry rather than exposing partial state.
    New-Item -ItemType Directory -Force -Path $shimDir | Out-Null
    New-Item -ItemType Directory -Force -Path $shim | Out-Null
    $failureOut = Join-Path $root 'transaction-failure.stdout'
    $failureErr = Join-Path $root 'transaction-failure.stderr'
    $failureCode = Invoke-CliCapture @('runtime','install-trusted',$catalog,$trust,'node','20.0.0') $failureOut $failureErr
    $failureCode | Set-Content (Join-Path $root 'transaction-failure.exit-code.txt')
    if ($failureCode -eq 0) { throw 'forced shim failure unexpectedly succeeded' }
    if (Test-Path -LiteralPath $installDir) { throw 'transaction failure left installed runtime directory behind' }
    if (Test-Path -LiteralPath $registry) {
        $registryValue = Get-Content $registry -Raw | ConvertFrom-Json
        if (@($registryValue.installed | Where-Object { $_.runtime -eq 'node' -and $_.version -eq '20.0.0' }).Count -ne 0) {
            throw 'transaction failure left runtime registry entry behind'
        }
    }
    Remove-Item -LiteralPath $shim -Recurse -Force
    $tmpShim = Join-Path $shimDir '.node.cmd.tmp'
    if (Test-Path -LiteralPath $tmpShim) { Remove-Item -LiteralPath $tmpShim -Force }

    & $script:Cli runtime install-trusted $catalog $trust node 20.0.0 | Set-Content (Join-Path $root 'install.json') -Encoding utf8
    Assert-LastExit 'trusted runtime install failed'
    $installedExe = Join-Path $installDir 'bin\node.exe'
    if (-not (Test-Path -LiteralPath $installedExe -PathType Leaf)) { throw 'installed runtime executable missing' }
    if (-not (Test-Path -LiteralPath $shim -PathType Leaf)) { throw 'runtime shim missing' }
    $shimOutput = (& $shim | Out-String).Trim()
    if ($shimOutput -ne 'VSN fake node 20.0.0') { throw "runtime shim returned unexpected output: $shimOutput" }
    $installedSha = (Get-FileHash $installedExe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($installedSha -ne $artifactSha) { throw 'installed runtime digest mismatch' }

    & $script:Cli runtime install-trusted $catalog $trust node 20.0.0 | Set-Content (Join-Path $root 'reinstall.json') -Encoding utf8
    Assert-LastExit 'idempotent trusted reinstall failed'
    $registryValue = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $registryValue | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'registry-after-install.json') -Encoding utf8
    $matches = @($registryValue.installed | Where-Object { $_.runtime -eq 'node' -and $_.version -eq '20.0.0' })
    if ($matches.Count -ne 1) { throw "expected one node@20.0.0 registration, got $($matches.Count)" }
    $runtimeAudit = & $script:Cli runtime audit | Out-String | ConvertFrom-Json
    $runtimeAudit | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'audit-after-install.json') -Encoding utf8
    if ($runtimeAudit.healthy -ne $true) { throw 'runtime audit unhealthy after trusted install' }

    & $script:Cli runtime activate $project node 20.0.0 | Set-Content (Join-Path $root 'activate.json') -Encoding utf8
    Assert-LastExit 'contained project activation failed'
    $projectCanonical = (Get-Item -LiteralPath $project).FullName
    $registryAfterActivate = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $registryAfterActivate | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'registry-after-activate.json') -Encoding utf8
    $activeVersion = $registryAfterActivate.project_activation.PSObject.Properties[$projectCanonical].Value.node
    if ([string]$activeVersion -ne '20.0.0') { throw 'project activation was not persisted' }

    Stop-Agent
    Start-Agent
    $registryAfterRestart = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $registryAfterRestart | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'registry-after-restart.json') -Encoding utf8
    $persistedVersion = $registryAfterRestart.project_activation.PSObject.Properties[$projectCanonical].Value.node
    if ([string]$persistedVersion -ne '20.0.0') { throw 'project activation did not survive Agent restart' }

    $outsideLink = Join-Path $workspace 'outside-link'
    New-Item -ItemType Junction -Path $outsideLink -Target $outside | Out-Null
    foreach ($case in @(
        @{Name='outside'; Path=$outside},
        @{Name='junction'; Path=$outsideLink},
        @{Name='nonexistent'; Path=(Join-Path $sandbox 'nonexistent-project')}
    )) {
        $stdout = Join-Path $root ($case.Name + '.stdout')
        $stderr = Join-Path $root ($case.Name + '.stderr')
        $code = Invoke-CliCapture @('runtime','activate',[string]$case.Path,'node','20.0.0') $stdout $stderr
        $code | Set-Content (Join-Path $root ($case.Name + '.exit-code.txt'))
        if ($code -eq 0) { throw "$($case.Name) activation must be rejected" }
    }

    'tampered artifact' | Set-Content -LiteralPath $artifact -Encoding utf8
    $tamperedOut = Join-Path $root 'tampered.stdout'
    $tamperedErr = Join-Path $root 'tampered.stderr'
    $tamperedCode = Invoke-CliCapture @('runtime','install-trusted',$catalog,$trust,'node','20.0.0') $tamperedOut $tamperedErr
    $tamperedCode | Set-Content (Join-Path $root 'tampered.exit-code.txt')
    if ($tamperedCode -eq 0) { throw 'tampered artifact unexpectedly installed' }
    if ((Get-FileHash $installedExe -Algorithm SHA256).Hash.ToLowerInvariant() -ne $installedSha) { throw 'digest failure damaged existing accepted runtime' }
    if ((& $shim | Out-String).Trim() -ne 'VSN fake node 20.0.0') { throw 'digest failure damaged existing shim' }
    $registryAfterTamper = & $script:Cli runtime registry | Out-String | ConvertFrom-Json
    $registryAfterTamper | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'registry-after-tamper.json') -Encoding utf8
    if (@($registryAfterTamper.installed | Where-Object { $_.runtime -eq 'node' -and $_.version -eq '20.0.0' }).Count -ne 1) {
        throw 'digest failure changed registry cardinality'
    }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.11';
        artifact='transactional-trusted-runtime-install-activation-windows-self-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        deterministic_test_signer=$true; trusted_install_verified=$true; shim_verified=$true;
        idempotent_reinstall_verified=$true; transactional_failure_rollback_verified=$true;
        workspace_activation_containment_verified=$true; activation_persistence_verified=$true;
        digest_failure_preserves_existing_runtime=$true; audit_chain_valid=$true
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
