param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$FeatureId = 'pkg02-0227-fresh-state-local-beta-final-gate'
$FeatureVersion = '1.0.0'
$CanonicalBaseSha = 'e6e981f106ff3685ab1694261991e5e97a3b738d'
$PlanSha256 = 'c8ced3e3b0b636f702d2c1a7608ac798827dd6808cf45ddc23d820d7af14ef8c'
$ResearchSha256 = '341b772f57db3ba3560add8cfc88ec85c418eed24058e0463dd01371ef277e9e'
$LifecycleSha256 = '9d57aeffc1a2bdd7ca5668deed21409ebae5bcf739284b102b1fbfdd56bb82bc'
$PreflightSha256 = 'd778ced2e1d8b074aa0fd8776384a1b8bba07c6bfa8318867dd4957ad869f654'
$ProductVersion = '0.38.1'
$CandidateId = 'c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474'
$AgentIpcPort = 39731
$TrackerPath = 'certification\pkg02-usable-local-beta-v1.json'
$ManifestPath = '.ai\manifests\pkg02-0227-fresh-state-local-beta-final-gate.v1.json'
$PlanPath = '.ai\plans\pkg02-0227-fresh-state-local-beta-final-gate-v1.md'

function Assert-Exit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Get-Sha([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-OptionalSha([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $null }
    Get-Sha $Path
}

function Assert-Sha([string]$Path, [string]$Expected, [string]$Name) {
    $actual = Get-Sha $Path
    if ($actual -ne $Expected) { throw "$Name digest mismatch expected=$Expected actual=$actual" }
}

function Write-Json([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Path -Encoding utf8
}

function Get-GitStatusText {
    $lines = @(git status --porcelain=v1 --untracked-files=all)
    Assert-Exit 'git status failed'
    $evidenceRoot = 'dist-self-hosted/02.27'
    $evidencePrefix = "$evidenceRoot/"
    $filtered = foreach ($line in $lines) {
        $path = if ($line.Length -ge 4) { $line.Substring(3).Trim() } else { '' }
        if ($path -eq $evidenceRoot -or $path.StartsWith($evidencePrefix, [StringComparison]::Ordinal)) { continue }
        $line
    }
    @($filtered) -join "`n"
}

function Get-BoundHashes([string[]]$Paths) {
    $result = [ordered]@{}
    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "bound file missing: $path" }
        $result[$path.Replace('\','/')] = Get-Sha $path
    }
    [pscustomobject]$result
}

function Assert-MapsEqual($Before, $After, [string]$Name) {
    $beforeKeys = @($Before.PSObject.Properties.Name)
    $afterKeys = @($After.PSObject.Properties.Name)
    if (($beforeKeys -join "`n") -ne ($afterKeys -join "`n")) { throw "$Name keys changed" }
    foreach ($key in $beforeKeys) {
        if ([string]$Before.$key -ne [string]$After.$key) {
            throw "$Name mismatch for $key before=$($Before.$key) after=$($After.$key)"
        }
    }
}

function Invoke-CliJson([string[]]$CliArgs, [string]$Name) {
    $out = Join-Path $script:Root "$Name.json"
    $err = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $script:Root "$Name.exit-code.txt") -Encoding ascii
    if ($code -ne 0) {
        $detail = if (Test-Path -LiteralPath $err) { Get-Content -LiteralPath $err -Raw } else { '' }
        throw "$Name failed (exit=$code): $detail"
    }
    Get-Content -LiteralPath $out -Raw | ConvertFrom-Json -NoEnumerate
}

function Invoke-CliFailure([string[]]$CliArgs, [string]$Name) {
    $out = Join-Path $script:Root "$Name.stdout.log"
    $err = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $script:Root "$Name.exit-code.txt") -Encoding ascii
    if ($code -eq 0) { throw "$Name unexpectedly succeeded" }
    [pscustomobject]@{
        ExitCode = $code
        Stdout = if (Test-Path -LiteralPath $out) { Get-Content -LiteralPath $out -Raw } else { '' }
        Stderr = if (Test-Path -LiteralPath $err) { Get-Content -LiteralPath $err -Raw } else { '' }
    }
}

function Start-Agent {
    $script:Agent = Start-Process -FilePath $script:AgentExe `
        -RedirectStandardOutput (Join-Path $script:Root 'agent.stdout.log') `
        -RedirectStandardError (Join-Path $script:Root 'agent.stderr.log') `
        -PassThru -WindowStyle Hidden
    $timer = [Diagnostics.Stopwatch]::StartNew()
    foreach ($attempt in 1..100) {
        & $script:Cli ping 1> (Join-Path $script:Root 'readiness-ping.json') 2> (Join-Path $script:Root 'readiness-ping.stderr.log')
        if ($LASTEXITCODE -eq 0) {
            $timer.Stop()
            $script:AgentReadyMs = [int64]$timer.ElapsedMilliseconds
            return
        }
        if ($script:Agent.HasExited) { throw "Agent exited before readiness code=$($script:Agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent readiness exceeded 25 seconds'
}

function Stop-AgentSafe {
    if ($null -eq $script:Agent) { return $true }
    try {
        if (-not $script:Agent.HasExited) {
            Stop-Process -Id $script:Agent.Id -Force -ErrorAction Stop
            Wait-Process -Id $script:Agent.Id -Timeout 10 -ErrorAction SilentlyContinue
        }
        return $script:Agent.HasExited
    } catch {
        return $false
    }
}

function Run-DesktopBridgeTest([string]$TestName, [string]$LogName) {
    & cargo test --locked --package vsn-desktop $TestName -- --ignored *> (Join-Path $script:Root $LogName)
    Assert-Exit "Desktop bridge test failed: $TestName"
}

function Set-OpenSslBuildEnvironment {
    $roots = [Collections.Generic.List[string]]::new()
    $command = Get-Command openssl.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        $binDir = Split-Path -Parent $command.Source
        $roots.Add((Split-Path -Parent $binDir))
    }
    foreach ($candidate in @('C:\Program Files\OpenSSL', 'C:\Program Files\OpenSSL-Win64')) {
        $roots.Add($candidate)
    }

    $selectedRoot = $null
    $selectedLib = $null
    foreach ($candidate in @($roots | Select-Object -Unique)) {
        $includeDir = Join-Path $candidate 'include'
        if (-not (Test-Path -LiteralPath (Join-Path $includeDir 'openssl\ssl.h') -PathType Leaf)) { continue }

        foreach ($libDir in @(
            (Join-Path $candidate 'lib'),
            (Join-Path $candidate 'lib\VC\x64\MD'),
            (Join-Path $candidate 'lib\VC\x64\MT')
        )) {
            if (
                (Test-Path -LiteralPath (Join-Path $libDir 'libssl.lib') -PathType Leaf) -and
                (Test-Path -LiteralPath (Join-Path $libDir 'libcrypto.lib') -PathType Leaf)
            ) {
                $selectedRoot = $candidate
                $selectedLib = $libDir
                break
            }
        }
        if ($null -ne $selectedRoot) { break }
    }

    if ($null -eq $selectedRoot -or $null -eq $selectedLib) {
        throw 'GitHub-hosted Windows image exposes OpenSSL but its development headers/libraries could not be located without installing or mutating the runner'
    }

    $selectedInclude = Join-Path $selectedRoot 'include'
    $env:OPENSSL_DIR = $selectedRoot
    $env:OPENSSL_INCLUDE_DIR = $selectedInclude
    $env:OPENSSL_LIB_DIR = $selectedLib

    $selectedExe = Join-Path $selectedRoot 'bin\openssl.exe'
    $version = if (Test-Path -LiteralPath $selectedExe -PathType Leaf) {
        (& $selectedExe version).Trim()
    } else {
        (& openssl version).Trim()
    }
    if ($LASTEXITCODE -ne 0) { throw 'OpenSSL version probe failed' }

    [pscustomobject]@{
        version = $version
        root = $selectedRoot
        include_dir = $selectedInclude
        lib_dir = $selectedLib
        source = 'preinstalled_github_hosted_runner'
        privileged_install_performed = $false
    }
}

$script:Root = Join-Path $PWD 'dist-self-hosted\02.27'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0227-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$isolatedLocal = Join-Path $sandbox 'localappdata'
$overviewOut = Join-Path $sandbox 'overview-state'
$script:Agent = $null
$script:AgentExe = $null
$script:Cli = $null
$script:AgentReadyMs = 0L
$workspaceRegistered = $false
$runFailure = $null
$runSucceeded = $false

$originalLocal = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$ipcKeyParent = Split-Path -Parent $ipcKey
$hadIpcKey = Test-Path -LiteralPath $ipcKey -PathType Leaf
$originalIpcKeyBytes = if ($hadIpcKey) { [IO.File]::ReadAllBytes($ipcKey) } else { $null }
$originalIpcKeySha = if ($hadIpcKey) { Get-Sha $ipcKey } else { $null }
$hostsPath = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
$hostsPreSha = Get-OptionalSha $hostsPath
$tauriGeneratedWindowsSchema = 'apps\desktop\src-tauri\gen\schemas\windows-schema.json'

$cleanup = [ordered]@{
    agent_stopped = $false
    workspace_removed = $false
    ipc_key_restored = $false
    localappdata_restored = $false
    sandbox_removed = $false
    tauri_generated_windows_schema_removed = $false
    system_hosts_unchanged = $false
    no_system_trust_mutation = $true
    no_resolver_mutation = $true
    no_production_or_remote_database_mutation = $true
    no_privileged_system_mutation = $true
}

if (Test-Path -LiteralPath $script:Root) { Remove-Item -LiteralPath $script:Root -Recurse -Force }
New-Item -ItemType Directory -Force -Path $script:Root, $bin, $sandbox, $workspace, $isolatedLocal | Out-Null

$trackedTauriSchema = @(git ls-files -- $tauriGeneratedWindowsSchema)
Assert-Exit 'unable to inspect tracked Tauri generated schema state'
if ($trackedTauriSchema.Count -ne 0) { throw 'Tauri generated Windows schema unexpectedly became tracked; refusing cleanup' }

$boundPaths = @(
    'Cargo.lock',
    'apps\desktop\package-lock.json',
    $TrackerPath,
    'docs\MASTER-EXECUTION-STATUS.json',
    'docs\MASTER-EXECUTION-PLAN.md',
    'docs\release-candidate-current.json',
    'rust-toolchain.toml',
    $PlanPath,
    $ManifestPath
)

$preStatus = Get-GitStatusText
if ($preStatus) { throw "02.27 requires a clean checkout before execution:`n$preStatus" }
$preStatus | Set-Content -LiteralPath (Join-Path $script:Root 'repository-status-pre.txt') -Encoding utf8
$boundPre = Get-BoundHashes $boundPaths
Write-Json (Join-Path $script:Root 'bound-state-pre.json') $boundPre

try {
    if (-not $IsWindows) { throw '02.27 requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.27 requires GitHub-hosted runner' }
    if ($env:RUNNER_OS -ne 'Windows') { throw "02.27 requires runner_os=Windows, got $env:RUNNER_OS" }
    if ($env:RUNNER_ARCH -ne 'X64') { throw "02.27 requires X64 runner, got $env:RUNNER_ARCH" }
    if (-not $env:EXPECTED_SHA) { throw 'EXPECTED_SHA required' }

    $sourceCommit = (git rev-parse HEAD).Trim()
    Assert-Exit 'git rev-parse failed'
    if ($sourceCommit -ne $env:EXPECTED_SHA) { throw "exact source mismatch expected=$env:EXPECTED_SHA actual=$sourceCommit" }

    $rustcVersion = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rustcVersion -notmatch '^rustc 1\.97\.1\b') { throw "rustc 1.97.1 required: $rustcVersion" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "cargo 1.97.1 required: $cargoVersion" }
    if (Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue) { throw 'IPC port 39731 occupied before gate' }

    $opensslBuild = Set-OpenSslBuildEnvironment
    Write-Json (Join-Path $script:Root 'openssl-build-environment.json') $opensslBuild

    Assert-Sha $PlanPath $PlanSha256 'plan'
    Assert-Sha '.ai\features\pkg02-0227\research.md' $ResearchSha256 'research'
    Assert-Sha '.ai\features\pkg02-0227\lifecycle-review.md' $LifecycleSha256 'lifecycle review'
    Assert-Sha '.ai\features\pkg02-0227\development-preflight.md' $PreflightSha256 'development preflight'

    $manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    if ([string]$manifest.feature_id -ne $FeatureId -or [string]$manifest.version -ne $FeatureVersion) { throw 'manifest identity mismatch' }
    if ([string]$manifest.canonical_base_sha -ne $CanonicalBaseSha) { throw 'manifest canonical base mismatch' }
    if ([string]$manifest.plan.sha256 -ne $PlanSha256) { throw 'manifest plan digest mismatch' }
    if (($manifest.acceptance.criteria | Measure-Object).Count -ne 12) { throw 'frozen AC-01..AC-12 set changed' }
    if (($manifest.acceptance.required_regressions | Measure-Object).Count -ne 18) { throw 'frozen 18-gate regression matrix changed' }
    Write-Json (Join-Path $script:Root 'required-regressions.json') @($manifest.acceptance.required_regressions)

    $candidate = Get-Content -LiteralPath 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    if ([string]$candidate.product_version -ne $ProductVersion) { throw "product version changed: $($candidate.product_version)" }
    if ([string]$candidate.candidate_id -ne $CandidateId) { throw "candidate changed: $($candidate.candidate_id)" }

    $tracker = Get-Content -LiteralPath $TrackerPath -Raw | ConvertFrom-Json
    if ([string]$tracker.package_id -ne 'PKG-02' -or [int]$tracker.required -ne 27) { throw 'PKG-02 tracker identity/denominator mismatch' }
    if ([int]$tracker.done -ne 26 -or [double]$tracker.percent -ne 96.30 -or $tracker.complete -ne $false -or [string]$tracker.active_task -ne '02.27') {
        throw "tracker entry state mismatch done=$($tracker.done) percent=$($tracker.percent) complete=$($tracker.complete) active=$($tracker.active_task)"
    }
    if (($tracker.tasks | Measure-Object).Count -ne 27) { throw 'tracker must contain exactly 27 task entries' }

    $prerequisites = @()
    foreach ($index in 1..27) {
        $id = '02.{0:D2}' -f $index
        $matches = @($tracker.tasks | Where-Object { [string]$_.id -eq $id })
        if ($matches.Count -ne 1) { throw "tracker must contain exactly one $id entry" }
        $task = $matches[0]
        if ($index -le 26) {
            if ([string]$task.status -ne 'DONE') { throw "$id is not DONE" }
            $hasEvidence = $null -ne $task.PSObject.Properties['evidence'] -and -not [string]::IsNullOrWhiteSpace([string]$task.evidence)
            if ($index -ge 2 -and -not $hasEvidence) { throw "$id is missing accepted evidence in the current tracker schema" }
            $evidenceDigest = $null
            if ($hasEvidence) {
                $bytes = [Text.Encoding]::UTF8.GetBytes([string]$task.evidence)
                $evidenceDigest = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
            }
            $prerequisites += [pscustomobject]@{
                id = $id
                status = [string]$task.status
                evidence_present = $hasEvidence
                evidence_text_sha256 = $evidenceDigest
            }
        } elseif ([string]$task.status -ne 'IN_PROGRESS') {
            throw '02.27 must be the sole IN_PROGRESS task'
        }
    }
    Write-Json (Join-Path $script:Root 'prerequisite-evidence-summary.json') $prerequisites

    $allowedChangedFiles = @(
        '.ai/features/pkg02-0227/development-preflight.md',
        '.ai/features/pkg02-0227/lifecycle-review.md',
        '.ai/features/pkg02-0227/research.md',
        '.ai/manifests/pkg02-0227-fresh-state-local-beta-final-gate.v1.json',
        '.ai/plans/pkg02-0227-fresh-state-local-beta-final-gate-v1.md',
        '.github/workflows/pkg02-0227-fresh-state-final-gate.yml',
        'apps/desktop/src-tauri/icons/icon.ico',
        'crates/vsn-extension/src/lib.rs',
        'scripts/self-hosted/pkg02-0227.ps1'
    ) | Sort-Object
    $actualChangedFiles = @(git diff --name-only "$CanonicalBaseSha...HEAD") | ForEach-Object { $_.Trim() } | Where-Object { $_ } | Sort-Object
    Assert-Exit 'unable to inspect implementation scope'
    if (($actualChangedFiles -join "`n") -ne ($allowedChangedFiles -join "`n")) {
        throw "02.27 scope differs from frozen planning/certification scope plus the AC-04-proven product fixes.`nExpected:`n$($allowedChangedFiles -join "`n")`nActual:`n$($actualChangedFiles -join "`n")"
    }
    $actualChangedFiles | Set-Content -LiteralPath (Join-Path $script:Root 'changed-files.txt') -Encoding utf8

    & git diff --check "$CanonicalBaseSha...HEAD" *> (Join-Path $script:Root 'git-diff-check-range.log')
    Assert-Exit 'committed git diff --check failed'
    & git diff --check *> (Join-Path $script:Root 'git-diff-check.log')
    Assert-Exit 'git diff --check failed'

    & cargo fmt --all -- --check *> (Join-Path $script:Root 'cargo-fmt.log')
    Assert-Exit 'cargo fmt failed'
    & cargo clippy --workspace --all-targets --locked -- -D warnings *> (Join-Path $script:Root 'cargo-clippy.log')
    Assert-Exit 'workspace Clippy failed'
    & cargo test --workspace --locked *> (Join-Path $script:Root 'cargo-test.log')
    Assert-Exit 'workspace tests failed'
    & cargo build --locked --release --package vsn-agent --package vsn *> (Join-Path $script:Root 'cargo-build.log')
    Assert-Exit 'release Agent/CLI build failed'

    Copy-Item -LiteralPath 'target\release\vsn-agent.exe' -Destination (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item -LiteralPath 'target\release\vsn.exe' -Destination (Join-Path $bin 'vsn.exe') -Force
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $script:Cli = Join-Path $bin 'vsn.exe'

    $desktopLockBefore = Get-Sha 'apps\desktop\package-lock.json'
    Push-Location 'apps\desktop'
    try {
        & npm ci *> (Join-Path $script:Root 'desktop-npm-ci.log')
        Assert-Exit 'Desktop npm ci failed'
        & npm run build *> (Join-Path $script:Root 'desktop-build.log')
        Assert-Exit 'Desktop production build failed'

        if (Test-Path -LiteralPath $overviewOut) { Remove-Item -LiteralPath $overviewOut -Recurse -Force }
        & '.\node_modules\.bin\tsc.cmd' `
            'src\agentOverviewState.ts' `
            'tests\agentOverviewState.acceptance.ts' `
            '--target' 'ES2022' `
            '--module' 'ES2022' `
            '--moduleResolution' 'Bundler' `
            '--skipLibCheck' `
            '--outDir' $overviewOut *> (Join-Path $script:Root 'desktop-overview-tsc.log')
        Assert-Exit 'Desktop Overview state compile failed'
        '{"type":"module"}' | Set-Content -LiteralPath (Join-Path $overviewOut 'package.json') -Encoding ascii
        & node (Join-Path $overviewOut 'tests\agentOverviewState.acceptance.js') *> (Join-Path $script:Root 'desktop-overview-state.log')
        Assert-Exit 'Desktop Overview state acceptance failed'
    } finally {
        Pop-Location
    }
    $desktopLockAfter = Get-Sha 'apps\desktop\package-lock.json'
    if ($desktopLockBefore -ne $desktopLockAfter) { throw 'Desktop package-lock changed during locked build' }

    $env:LOCALAPPDATA = $isolatedLocal
    if (Test-Path -LiteralPath $ipcKey -PathType Leaf) { Remove-Item -LiteralPath $ipcKey -Force }

    $offlineBefore = Invoke-CliFailure @('status') 'offline-status-before-agent'
    if (-not $offlineBefore.Stderr.Contains('hint=ensure vsn-agent is running and the authenticated local IPC channel is available')) {
        throw 'offline CLI failure did not preserve the operator hint'
    }
    Run-DesktopBridgeTest 'desktop_bridge_reports_agent_unavailable' 'desktop-bridge-offline-before.log'

    Start-Agent
    $ping = Invoke-CliJson @('ping') 'ping'
    $status = Invoke-CliJson @('status') 'status'
    $machine = Invoke-CliJson @('machine') 'machine'
    $security = Invoke-CliJson @('security') 'security'
    $config = Invoke-CliJson @('config','show') 'config-show'
    $auditBeforeWorkspace = Invoke-CliJson @('audit','verify') 'audit-before-workspace'

    if ($ping.pong -ne $true -or [string]$ping.version -ne $ProductVersion) { throw 'authenticated ping mismatch' }
    if ($status.health.healthy -ne $true -or $status.security.ipc_secret_ready -ne $true) { throw 'authenticated status mismatch' }
    if ([string]::IsNullOrWhiteSpace([string]$machine.device_id) -or ([string]$machine.device_id).Length -lt 8) { throw 'machine device_id missing/invalid' }
    if ($security.device_identity_ready -ne $true -or $security.ipc_secret_ready -ne $true) { throw 'security status mismatch' }
    if ([int]$config.version -ne 3 -or $config.remote.enabled -ne $false) { throw 'config show mismatch' }
    if ($auditBeforeWorkspace.valid -ne $true) { throw 'audit chain invalid before workspace smoke' }

    $workspaceInitial = Invoke-CliJson @('workspace','list') 'workspace-initial'
    if (@($workspaceInitial).Count -ne 0) { throw 'isolated workspace list must start empty' }
    $workspaceAdd = Invoke-CliJson @('workspace','add',$workspace) 'workspace-add'
    $workspaceRegistered = $true
    $workspaceAfterAdd = Invoke-CliJson @('workspace','list') 'workspace-after-add'
    if (@($workspaceAfterAdd).Count -ne 1) { throw 'workspace registration did not produce exactly one root' }
    if (@($workspaceAdd.workspace_roots).Count -ne 1) { throw 'workspace add response did not contain exactly one root' }

    Run-DesktopBridgeTest 'desktop_bridge_uses_authenticated_agent' 'desktop-bridge-online.log'

    $workspaceRemove = Invoke-CliJson @('workspace','remove',$workspace) 'workspace-remove'
    $workspaceRegistered = $false
    if (@($workspaceRemove.workspace_roots).Count -ne 0) { throw 'workspace remove response is not empty' }
    $workspaceAfterRemove = Invoke-CliJson @('workspace','list') 'workspace-after-remove'
    if (@($workspaceAfterRemove).Count -ne 0) { throw 'workspace root persisted after remove' }

    $audit = Invoke-CliJson @('audit','verify') 'audit'
    if ($audit.valid -ne $true -or [int]$audit.events -lt 8) { throw 'final audit chain invalid or unexpectedly empty' }

    if (-not (Stop-AgentSafe)) { throw 'Agent did not stop after integrated smoke' }
    $cleanup.agent_stopped = $true

    $offlineAfter = Invoke-CliFailure @('status') 'offline-status-after-agent'
    if (-not $offlineAfter.Stderr.Contains('hint=ensure vsn-agent is running and the authenticated local IPC channel is available')) {
        throw 'post-online offline CLI failure did not preserve the operator hint'
    }
    Run-DesktopBridgeTest 'desktop_bridge_reports_agent_unavailable' 'desktop-bridge-offline-after.log'

    $runSucceeded = $true
} catch {
    $runFailure = $_
} finally {
    if ($workspaceRegistered -and $null -ne $script:Cli -and $null -ne $script:Agent -and -not $script:Agent.HasExited) {
        try {
            & $script:Cli workspace remove $workspace *> (Join-Path $script:Root 'cleanup-workspace-remove.log')
            if ($LASTEXITCODE -eq 0) { $workspaceRegistered = $false }
        } catch {}
    }
    $cleanup.workspace_removed = -not $workspaceRegistered

    if (-not $cleanup.agent_stopped) { $cleanup.agent_stopped = Stop-AgentSafe }

    try {
        if ($hadIpcKey) {
            New-Item -ItemType Directory -Force -Path $ipcKeyParent | Out-Null
            [IO.File]::WriteAllBytes($ipcKey, $originalIpcKeyBytes)
            $cleanup.ipc_key_restored = (Get-OptionalSha $ipcKey) -eq $originalIpcKeySha
        } else {
            if (Test-Path -LiteralPath $ipcKey -PathType Leaf) { Remove-Item -LiteralPath $ipcKey -Force }
            $cleanup.ipc_key_restored = -not (Test-Path -LiteralPath $ipcKey -PathType Leaf)
        }
    } catch { $cleanup.ipc_key_restored = $false }

    try {
        $env:LOCALAPPDATA = $originalLocal
        $cleanup.localappdata_restored = ($env:LOCALAPPDATA -eq $originalLocal)
    } catch { $cleanup.localappdata_restored = $false }

    try {
        if (Test-Path -LiteralPath $sandbox) { Remove-Item -LiteralPath $sandbox -Recurse -Force }
        $cleanup.sandbox_removed = -not (Test-Path -LiteralPath $sandbox)
    } catch { $cleanup.sandbox_removed = $false }

    try {
        if (Test-Path -LiteralPath $tauriGeneratedWindowsSchema -PathType Leaf) {
            Remove-Item -LiteralPath $tauriGeneratedWindowsSchema -Force
        }
        $cleanup.tauri_generated_windows_schema_removed = -not (Test-Path -LiteralPath $tauriGeneratedWindowsSchema -PathType Leaf)
    } catch { $cleanup.tauri_generated_windows_schema_removed = $false }

    $hostsPostSha = Get-OptionalSha $hostsPath
    $cleanup.system_hosts_unchanged = ($hostsPreSha -eq $hostsPostSha)
    Write-Json (Join-Path $script:Root 'cleanup.json') $cleanup
}

if ($null -ne $runFailure) { throw $runFailure }
if (-not $runSucceeded) { throw '02.27 execution did not reach success' }
foreach ($property in $cleanup.GetEnumerator()) {
    if ($property.Value -ne $true) { throw "cleanup invariant failed: $($property.Key)" }
}

$postStatus = Get-GitStatusText
$postStatus | Set-Content -LiteralPath (Join-Path $script:Root 'repository-status-post.txt') -Encoding utf8
if ($postStatus -ne $preStatus) { throw "repository status drifted.`nBefore:`n$preStatus`nAfter:`n$postStatus" }
$boundPost = Get-BoundHashes $boundPaths
Write-Json (Join-Path $script:Root 'bound-state-post.json') $boundPost
Assert-MapsEqual $boundPre $boundPost 'bound tracked state'

& git diff --check "$CanonicalBaseSha...HEAD" *> (Join-Path $script:Root 'git-diff-check-final-range.log')
Assert-Exit 'final committed git diff --check failed'
& git diff --check *> (Join-Path $script:Root 'git-diff-check-final.log')
Assert-Exit 'final git diff --check failed'

$agentSha = Get-Sha (Join-Path $bin 'vsn-agent.exe')
$cliSha = Get-Sha (Join-Path $bin 'vsn.exe')
$evidence = [ordered]@{
    schema_version = 1
    package_id = 'PKG-02'
    task_id = '02.27'
    feature_id = $FeatureId
    feature_version = $FeatureVersion
    canonical_base_sha = $CanonicalBaseSha
    plan_sha256 = $PlanSha256
    source_commit = $env:EXPECTED_SHA
    product_version = $ProductVersion
    candidate_id = $CandidateId
    runner_environment = $env:RUNNER_ENVIRONMENT
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    rust_version = $rustcVersion
    cargo_version = $cargoVersion
    openssl_build_environment = $opensslBuild
    ipc_address = "127.0.0.1:$AgentIpcPort"
    prerequisite_tasks_done = 26
    prerequisite_evidence_entries = @($prerequisites | Where-Object { $_.evidence_present }).Count
    required_regressions = @($manifest.acceptance.required_regressions)
    artifacts = [ordered]@{
        vsn_agent_sha256 = $agentSha
        vsn_cli_sha256 = $cliSha
        desktop_package_lock_sha256 = $desktopLockAfter
    }
    measurements = [ordered]@{
        agent_ready_ms = $script:AgentReadyMs
        audit_events = [int]$audit.events
        changed_file_count = $actualChangedFiles.Count
        bound_file_count = $boundPaths.Count
    }
    checks = [ordered]@{
        exact_source_runner_toolchain_binding = $true
        canonical_prerequisite_chain = $true
        fresh_checkout_and_tracked_state_baseline = $true
        locked_rust_product_verification = $true
        locked_desktop_verification = $true
        authenticated_agent_cli_integrated_smoke = $true
        desktop_authenticated_bridge_online_offline = $true
        deterministic_overview_states = $true
        accepted_local_capability_regression_manifest_bound = $true
        fail_closed_offline_behavior_preserved = $true
        permission_and_product_scope_preserved = $true
        cleanup_and_non_mutation = $true
        predecessor_evidence_chain_bound = $true
        zero_unintended_repository_and_lock_drift = $true
    }
    cleanup = $cleanup
    privileged_system_mutation_performed = $false
    production_or_remote_database_mutation_performed = $false
}

$evidencePath = Join-Path $script:Root 'evidence.json'
Write-Json $evidencePath $evidence
$evidenceSha = Get-Sha $evidencePath
$evidenceSha | Set-Content -LiteralPath (Join-Path $script:Root 'evidence.json.sha256') -Encoding ascii
Write-Host "02.27 evidence_sha256=$evidenceSha source=$($env:EXPECTED_SHA) agent_sha256=$agentSha cli_sha256=$cliSha"
