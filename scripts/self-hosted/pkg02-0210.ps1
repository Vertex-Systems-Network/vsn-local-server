param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Write-JsonFile([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 16 -Compress | Set-Content -LiteralPath $Path -Encoding utf8
}

$root = Join-Path $PWD 'dist-self-hosted\02.10'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0210-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$sandbox,$isolatedLocalAppData | Out-Null
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

function Invoke-RejectVerify([string]$Name, [string]$Catalog, [string]$Trust) {
    $stdout = Join-Path $root "$Name.stdout"
    $stderr = Join-Path $root "$Name.stderr"
    & $script:Cli runtime catalog-verify $Catalog $Trust 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $root "$Name.exit-code.txt")
    if ($code -eq 0) { throw "$Name catalog verification unexpectedly succeeded" }
    if ((Test-Path $stdout) -and (Get-Item $stdout).Length -ne 0) { throw "$Name unexpectedly wrote stdout" }
    if (-not (Select-String -LiteralPath $stderr -SimpleMatch 'error=' -Quiet)) { throw "$Name did not surface operator error" }
}

function Invoke-RejectCatalog([string]$Name, [string]$Catalog) {
    $stdout = Join-Path $root "$Name.stdout"
    $stderr = Join-Path $root "$Name.stderr"
    & $script:Cli runtime catalog $Catalog 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $root "$Name.exit-code.txt")
    if ($code -eq 0) { throw "$Name catalog unexpectedly succeeded" }
    if ((Test-Path $stdout) -and (Get-Item $stdout).Length -ne 0) { throw "$Name unexpectedly wrote stdout" }
    if (-not (Select-String -LiteralPath $stderr -SimpleMatch 'error=' -Quiet)) { throw "$Name did not surface operator error" }
}

function New-TestTar([string]$Path, [string]$Kind) {
    Add-Type -AssemblyName System.Formats.Tar
    $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
    try {
        $writer = [System.Formats.Tar.TarWriter]::new($stream, $false)
        try {
            if ($Kind -eq 'traversal') {
                $entry = [System.Formats.Tar.PaxTarEntry]::new([System.Formats.Tar.TarEntryType]::RegularFile, '../escape.txt')
                $bytes = [Text.Encoding]::UTF8.GetBytes('evil')
                $entry.DataStream = [System.IO.MemoryStream]::new($bytes, $false)
                $writer.WriteEntry($entry)
            }
            elseif ($Kind -eq 'symlink') {
                $entry = [System.Formats.Tar.PaxTarEntry]::new([System.Formats.Tar.TarEntryType]::SymbolicLink, 'bin/node')
                $entry.LinkName = '../../escape'
                $writer.WriteEntry($entry)
            }
            elseif ($Kind -eq 'hardlink') {
                $entry = [System.Formats.Tar.PaxTarEntry]::new([System.Formats.Tar.TarEntryType]::HardLink, 'bin/node')
                $entry.LinkName = '../../escape'
                $writer.WriteEntry($entry)
            }
            else { throw "unknown tar fixture kind: $Kind" }
        }
        finally { $writer.Dispose() }
    }
    finally { $stream.Dispose() }
}

try {
    if (-not $IsWindows) { throw "02.10 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if (-not (Get-Command tar.exe -ErrorAction SilentlyContinue)) { throw 'Windows tar.exe is required' }
    try { Add-Type -AssemblyName System.Formats.Tar } catch { throw 'System.Formats.Tar is required for deterministic malicious archive fixtures' }

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $root 'runner.txt')

    $runtimeSource = Get-Content 'crates/vsn-runtime/src/lib.rs' -Raw
    foreach ($needle in @('pub fn load_catalog_verified','verify_signature','validate_archive_before_extract','reject_extracted_symlinks')) {
        if (-not $runtimeSource.Contains($needle)) { throw "missing trusted catalog/archive source invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('runtime.catalog-verify','runtime.install-trusted')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent runtime invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-runtime --all-targets -- -D warnings
    Assert-LastExit 'vsn-runtime clippy failed'
    cargo test --locked --package vsn-runtime
    Assert-LastExit 'vsn-runtime tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $goodPub = '6kpsY+KcUgq+9VB7Ey7F+ZVHdq6+vnuSQh7qaRRG0iw='
    $badPub = 'E5j2LG0aRXxRumpLXz29L2n8qTIWIY3ImX5Ba9F9k8o='
    $signature = 'rzklnAniLxG8MBJuBTapVslYPvWJzD9Ar2jU8N2ozUuzjf2OtUd8G2vGBeQSKebynyCVjuolNWLPMoDpmlbXBA=='
    $trustGood = Join-Path $sandbox 'trust-good.json'
    $trustBad = Join-Path $sandbox 'trust-bad.json'
    Write-JsonFile $trustGood ([ordered]@{ public_keys=@($goodPub) })
    Write-JsonFile $trustBad ([ordered]@{ public_keys=@($badPub) })

    $safeArtifact = [ordered]@{
        os='linux'; arch='x86_64'; url='https://example.invalid/node.tar.gz';
        sha256=('0' * 64); archive='tar.gz'; executable_relpath='bin/node'
    }
    $catalogGood = Join-Path $sandbox 'catalog-good.json'
    $goodValue = [ordered]@{
        schema_version=1; provider='vsn.test';
        runtimes=@([ordered]@{ runtime='node'; version='20.0.0'; artifacts=@($safeArtifact) });
        signature=$signature
    }
    Write-JsonFile $catalogGood $goodValue

    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent
    & $script:Cli diagnostics | Set-Content (Join-Path $root 'diagnostics.json') -Encoding utf8
    Assert-LastExit 'diagnostics failed'

    $trusted = & $script:Cli runtime catalog-verify $catalogGood $trustGood | Out-String | ConvertFrom-Json
    $trusted | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'trusted-good.json') -Encoding utf8
    if ([string]$trusted.provider -ne 'vsn.test' -or [int]$trusted.releases -ne 1 -or [string]$trusted.signer_public_key -ne $goodPub) {
        throw 'trusted signed catalog verification returned unexpected metadata'
    }

    $tampered = Join-Path $sandbox 'catalog-tampered.json'
    $tamperedValue = Get-Content $catalogGood -Raw | ConvertFrom-Json
    $tamperedValue.runtimes[0].version = '20.0.1'
    Write-JsonFile $tampered $tamperedValue
    $unsigned = Join-Path $sandbox 'catalog-unsigned.json'
    Write-JsonFile $unsigned ([ordered]@{ schema_version=1; provider='vsn.test'; runtimes=@(); signature=$null })
    $unknown = Join-Path $sandbox 'catalog-unknown-field.json'
    $unknownValue = Get-Content $catalogGood -Raw | ConvertFrom-Json
    $unknownValue | Add-Member -NotePropertyName future_policy -NotePropertyValue 'must-not-be-silently-ignored'
    Write-JsonFile $unknown $unknownValue

    Invoke-RejectVerify 'untrusted' $catalogGood $trustBad
    Invoke-RejectVerify 'tampered' $tampered $trustGood
    Invoke-RejectVerify 'unsigned' $unsigned $trustGood
    Invoke-RejectVerify 'unknown-field' $unknown $trustGood

    $duplicateRelease = Join-Path $sandbox 'duplicate-release.json'
    Write-JsonFile $duplicateRelease ([ordered]@{
        schema_version=1; provider='vsn.test';
        runtimes=@(
            [ordered]@{runtime='node';version='20.0.0';artifacts=@($safeArtifact)},
            [ordered]@{runtime='node';version='20.0.0';artifacts=@($safeArtifact)}
        ); signature=$null
    })
    $duplicateTarget = Join-Path $sandbox 'duplicate-target.json'
    Write-JsonFile $duplicateTarget ([ordered]@{
        schema_version=1; provider='vsn.test';
        runtimes=@([ordered]@{runtime='node';version='20.0.0';artifacts=@($safeArtifact,$safeArtifact)}); signature=$null
    })
    $unsafeOther = Join-Path $sandbox 'unsafe-other-platform.json'
    $unsafeWindows = [ordered]@{ os='windows'; arch='x86_64'; url='http://insecure.invalid/a.zip'; sha256=('0' * 64); archive='zip'; executable_relpath='../escape.exe' }
    Write-JsonFile $unsafeOther ([ordered]@{
        schema_version=1; provider='vsn.test';
        runtimes=@([ordered]@{runtime='node';version='20.0.0';artifacts=@($safeArtifact,$unsafeWindows)}); signature=$null
    })
    $badArchive = Join-Path $sandbox 'unsupported-archive.json'
    $rarArtifact = [ordered]@{ os='linux'; arch='x86_64'; url='https://example.invalid/a.rar'; sha256=('0' * 64); archive='rar'; executable_relpath='bin/node' }
    Write-JsonFile $badArchive ([ordered]@{
        schema_version=1; provider='vsn.test';
        runtimes=@([ordered]@{runtime='node';version='20.0.0';artifacts=@($rarArtifact)}); signature=$null
    })

    Invoke-RejectCatalog 'duplicate-release' $duplicateRelease
    Invoke-RejectCatalog 'duplicate-target' $duplicateTarget
    Invoke-RejectCatalog 'unsafe-other-platform' $unsafeOther
    Invoke-RejectCatalog 'unsupported-archive' $badArchive

    $archiveCases = @(
        @{ Version='20.0.1'; Kind='traversal' },
        @{ Version='20.0.2'; Kind='symlink' },
        @{ Version='20.0.3'; Kind='hardlink' }
    )
    foreach ($case in $archiveCases) {
        $archive = Join-Path $sandbox ($case.Version + '.tar')
        New-TestTar $archive $case.Kind
        $digest = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
        $forward = $archive.Replace('\','/')
        $artifact = [ordered]@{
            os='windows'; arch='x86_64'; url=('file://' + $forward); sha256=$digest;
            archive='tar'; executable_relpath='bin/node'
        }
        $catalog = Join-Path $sandbox ($case.Version + '-catalog.json')
        Write-JsonFile $catalog ([ordered]@{
            schema_version=1; provider='vsn.test';
            runtimes=@([ordered]@{runtime='node';version=$case.Version;artifacts=@($artifact)}); signature=$null
        })
        $stdout = Join-Path $root ('archive-' + $case.Version + '.stdout')
        $stderr = Join-Path $root ('archive-' + $case.Version + '.stderr')
        & $script:Cli runtime install $catalog node $case.Version 1> $stdout 2> $stderr
        $code = $LASTEXITCODE
        $code | Set-Content (Join-Path $root ('archive-' + $case.Version + '.exit-code.txt'))
        if ($code -eq 0) { throw "malicious $($case.Kind) archive unexpectedly installed" }
        if ((Test-Path $stdout) -and (Get-Item $stdout).Length -ne 0) { throw "malicious $($case.Kind) archive unexpectedly wrote stdout" }
    }

    $diag = Get-Content (Join-Path $root 'diagnostics.json') -Raw | ConvertFrom-Json
    $runtimeRoot = Join-Path ([string]$diag.data_dir) 'runtimes'
    foreach ($candidate in @(
        (Join-Path $runtimeRoot 'node\escape.txt'),
        (Join-Path $runtimeRoot 'node\escape'),
        (Join-Path $sandbox 'escape.txt'),
        (Join-Path $sandbox 'escape')
    )) {
        if (Test-Path -LiteralPath $candidate) { throw "archive escaped managed root: $candidate" }
    }

    $audit = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $audit | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'audit-chain.json') -Encoding utf8
    if ($audit.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.10';
        artifact='trusted-runtime-catalog-archive-safety-windows-self-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        trusted_signature_acceptance=$true; untrusted_tampered_unsigned_rejected=$true;
        unknown_schema_fields_rejected=$true; duplicate_release_and_target_rejected=$true;
        all_artifact_metadata_validated=$true; unsupported_archive_rejected=$true;
        traversal_symlink_hardlink_archives_rejected=$true; managed_root_escape_absent=$true; audit_chain_valid=$true
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
