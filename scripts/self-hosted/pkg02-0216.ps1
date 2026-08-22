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

function Invoke-CliWithStdin([string[]]$Args, [string]$InputText, [bool]$ExpectSuccess = $true) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $script:Cli
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    foreach ($arg in $Args) { [void]$psi.ArgumentList.Add($arg) }
    $process = [System.Diagnostics.Process]::Start($psi)
    try {
        $process.StandardInput.Write($InputText)
        $process.StandardInput.Close()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        $result = [pscustomobject]@{ ExitCode=$process.ExitCode; Stdout=$stdout; Stderr=$stderr }
        if ($ExpectSuccess -and $process.ExitCode -ne 0) {
            throw "CLI failed: $($Args -join ' ') (exit=$($process.ExitCode)) stderr=$stderr"
        }
        return $result
    }
    finally { $process.Dispose() }
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

$script:Root = Join-Path $PWD 'dist-self-hosted\02.16'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0216-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$outside = Join-Path $sandbox 'outside'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$script:Agent = $null

New-Item -ItemType Directory -Force -Path $script:Root,$bin,$workspace,$outside,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

try {
    if (-not $IsWindows) { throw "02.16 acceptance requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $script:Root 'runner.txt')

    $filesSource = Get-Content 'crates/vsn-files/src/lib.rs' -Raw
    foreach ($needle in @(
        'MAX_TEXT_BYTES',
        'MAX_DIRECTORY_ENTRIES',
        'TEXT_TRANSACTION_COUNTER',
        'create_new(true)',
        'staged_replace(&tmp, &path, &transaction_id)',
        'workspace root itself cannot be mutated',
        'resolve_existing_for_mutation',
        'metadata_is_link_like',
        'FILE_ATTRIBUTE_REPARSE_POINT',
        'take(MAX_TEXT_BYTES + 1)'
    )) {
        if (-not $filesSource.Contains($needle)) { throw "missing 02.16 source invariant: $needle" }
    }

    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('Permission::FilesRead','Permission::FilesWrite','vsn_files::resolve_existing')) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.16 Core boundary invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-files --package vsn-core --package vsn-agent --package vsn --all-targets -- -D warnings
    Assert-LastExit 'files/core/agent/cli clippy failed'
    cargo test --locked --package vsn-files --package vsn-core --package vsn-agent --package vsn
    Assert-LastExit 'files/core/agent/cli tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    Start-Agent

    $added = Invoke-CliJson @('workspace','add',$workspace)
    $added | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'workspace-add.json') -Encoding utf8
    $roots = @(Invoke-CliJson @('workspace','list'))
    $roots | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'workspace-roots.json') -Encoding utf8
    $canonicalWorkspace = (Get-Item -LiteralPath $workspace).FullName
    if (-not ($roots | Where-Object { [string]$_ -eq $canonicalWorkspace })) { throw 'workspace registration did not persist' }

    $docs = Join-Path $workspace 'docs'
    $mkdir = Invoke-CliJson @('files','mkdir',$docs)
    $mkdir | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'mkdir.json') -Encoding utf8
    if ([string]$mkdir.operation -ne 'mkdir' -or $mkdir.is_dir -ne $true) { throw 'mkdir result mismatch' }

    $note = Join-Path $docs 'note.txt'
    $firstContent = "alpha`nβeta`nمرحبا"
    $write1 = Invoke-CliWithStdin @('files','write',$note) $firstContent
    $write1.Stdout | Set-Content (Join-Path $script:Root 'write-create.stdout') -Encoding utf8
    $created = $write1.Stdout | ConvertFrom-Json
    if ($created.created -ne $true) { throw 'initial text write was not reported as created' }
    if ([uint64]$created.bytes -ne [System.Text.Encoding]::UTF8.GetByteCount($firstContent)) { throw 'initial text byte count mismatch' }

    $read1 = Invoke-CliJson @('files','read',$note)
    $read1 | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'read-created.json') -Encoding utf8
    if ([string]$read1.content -ne $firstContent) { throw 'Unicode text read did not exactly match write payload' }

    $secondContent = "replacement`nΩ`n終"
    $write2 = Invoke-CliWithStdin @('files','write',$note) $secondContent
    $replaced = $write2.Stdout | ConvertFrom-Json
    $replaced | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'write-replace.json') -Encoding utf8
    if ($replaced.created -ne $false) { throw 'overwrite was incorrectly reported as create' }
    $read2 = Invoke-CliJson @('files','read',$note)
    if ([string]$read2.content -ne $secondContent) { throw 'transactional text replacement did not persist exact content' }
    $transactionLeaks = @(Get-ChildItem -LiteralPath $docs -Force | Where-Object { $_.Name -match '\.vsn-(upload|backup)-text-' })
    if ($transactionLeaks.Count -ne 0) { throw 'text transaction left staging/backup artifacts after successful replacement' }

    $alphaDir = Join-Path $workspace 'alpha-dir'
    Invoke-CliJson @('files','mkdir',$alphaDir) | Out-Null
    $rootList = @(Invoke-CliJson @('files','list',$workspace))
    $rootList | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'list-root.json') -Encoding utf8
    $dirNames = @($rootList | Where-Object { $_.is_dir -eq $true } | ForEach-Object { [string]$_.name })
    if ($dirNames.Count -lt 2 -or $dirNames[0] -ne 'alpha-dir' -or $dirNames[1] -ne 'docs') { throw 'directory list ordering is not deterministic directories-first/name-sorted' }

    $moved = Join-Path $docs 'moved.txt'
    $move = Invoke-CliJson @('files','move',$note,$moved)
    $move | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'move.json') -Encoding utf8
    if ([string]$move.operation -ne 'move' -or -not (Test-Path -LiteralPath $moved) -or (Test-Path -LiteralPath $note)) { throw 'workspace move did not preserve expected source/destination state' }
    $movedRead = Invoke-CliJson @('files','read',$moved)
    if ([string]$movedRead.content -ne $secondContent) { throw 'moved file content changed' }

    $rootWrite = Invoke-CliWithStdin @('files','write',$workspace) 'blocked-root-write' $false
    $rootWrite.Stdout | Set-Content (Join-Path $script:Root 'root-write.stdout') -Encoding utf8
    $rootWrite.Stderr | Set-Content (Join-Path $script:Root 'root-write.stderr') -Encoding utf8
    $rootWrite.ExitCode | Set-Content (Join-Path $script:Root 'root-write.exit-code.txt')
    if ($rootWrite.ExitCode -eq 0) { throw 'workspace root write unexpectedly succeeded' }
    $siblingLeaks = @(Get-ChildItem -LiteralPath $sandbox -Force -File | Where-Object { $_.Name -match '^workspace\..*vsn-' })
    if ($siblingLeaks.Count -ne 0) { throw 'workspace root write created an out-of-root sibling transaction artifact' }

    $rootMoveOut = Join-Path $script:Root 'root-move.stdout'
    $rootMoveErr = Join-Path $script:Root 'root-move.stderr'
    $rootMoveCode = Invoke-CliCapture @('files','move',$workspace,(Join-Path $sandbox 'workspace-moved')) $rootMoveOut $rootMoveErr
    $rootMoveCode | Set-Content (Join-Path $script:Root 'root-move.exit-code.txt')
    if ($rootMoveCode -eq 0 -or -not (Test-Path -LiteralPath $workspace)) { throw 'workspace root move protection failed' }

    $rootDeleteOut = Join-Path $script:Root 'root-delete.stdout'
    $rootDeleteErr = Join-Path $script:Root 'root-delete.stderr'
    $rootDeleteCode = Invoke-CliCapture @('files','delete',$workspace,'true') $rootDeleteOut $rootDeleteErr
    $rootDeleteCode | Set-Content (Join-Path $script:Root 'root-delete.exit-code.txt')
    if ($rootDeleteCode -eq 0 -or -not (Test-Path -LiteralPath $workspace)) { throw 'workspace root delete protection failed' }

    $outsideFile = Join-Path $outside 'outside.txt'
    'outside-preserve' | Set-Content -LiteralPath $outsideFile -Encoding utf8
    foreach ($case in @(
        @{Name='outside-read'; Args=@('files','read',$outsideFile)},
        @{Name='outside-mkdir'; Args=@('files','mkdir',(Join-Path $outside 'new-dir'))}
    )) {
        $stdout = Join-Path $script:Root ($case.Name + '.stdout')
        $stderr = Join-Path $script:Root ($case.Name + '.stderr')
        $code = Invoke-CliCapture $case.Args $stdout $stderr
        $code | Set-Content (Join-Path $script:Root ($case.Name + '.exit-code.txt'))
        if ($code -eq 0) { throw "$($case.Name) escaped workspace containment" }
    }
    $outsideWrite = Invoke-CliWithStdin @('files','write',$outsideFile) 'outside-overwrite' $false
    $outsideWrite.Stderr | Set-Content (Join-Path $script:Root 'outside-write.stderr') -Encoding utf8
    if ($outsideWrite.ExitCode -eq 0) { throw 'outside write escaped workspace containment' }
    if ((Get-Content -LiteralPath $outsideFile -Raw).Trim() -ne 'outside-preserve') { throw 'outside write attempt changed unrelated file' }

    $junction = Join-Path $workspace 'outside-junction'
    New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null
    $junctionList = @(Invoke-CliJson @('files','list',$workspace))
    $junctionEntry = $junctionList | Where-Object { [string]$_.name -eq 'outside-junction' }
    if (-not $junctionEntry) { throw 'junction entry is missing from safe directory listing' }
    if ($junctionEntry.is_dir -eq $true -or [uint64]$junctionEntry.size -ne 0) { throw 'directory listing followed junction target metadata' }

    $junctionReadOut = Join-Path $script:Root 'junction-read.stdout'
    $junctionReadErr = Join-Path $script:Root 'junction-read.stderr'
    $junctionReadCode = Invoke-CliCapture @('files','read',(Join-Path $junction 'outside.txt')) $junctionReadOut $junctionReadErr
    if ($junctionReadCode -eq 0) { throw 'junction traversal escaped workspace read containment' }
    $junctionWrite = Invoke-CliWithStdin @('files','write',(Join-Path $junction 'new.txt')) 'junction-write' $false
    if ($junctionWrite.ExitCode -eq 0 -or (Test-Path -LiteralPath (Join-Path $outside 'new.txt'))) { throw 'junction traversal escaped workspace write containment' }
    $junctionDeleteOut = Join-Path $script:Root 'junction-delete.stdout'
    $junctionDeleteErr = Join-Path $script:Root 'junction-delete.stderr'
    $junctionDeleteCode = Invoke-CliCapture @('files','delete',$junction,'true') $junctionDeleteOut $junctionDeleteErr
    if ($junctionDeleteCode -eq 0 -or -not (Test-Path -LiteralPath $junction)) { throw 'junction mutation was not rejected fail-closed' }

    $largeText = Join-Path $docs 'too-large.txt'
    [System.IO.File]::WriteAllBytes($largeText, [byte[]]::new(1024 * 1024 + 1))
    $largeOut = Join-Path $script:Root 'large-read.stdout'
    $largeErr = Join-Path $script:Root 'large-read.stderr'
    $largeCode = Invoke-CliCapture @('files','read',$largeText) $largeOut $largeErr
    $largeCode | Set-Content (Join-Path $script:Root 'large-read.exit-code.txt')
    if ($largeCode -eq 0) { throw 'text read exceeded 1 MiB bound' }

    $deleteFile = Invoke-CliJson @('files','delete',$moved,'false')
    $deleteFile | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'delete-file.json') -Encoding utf8
    if (Test-Path -LiteralPath $moved) { throw 'file delete did not remove target' }
    Remove-Item -LiteralPath $junction -Force
    $deleteDocs = Invoke-CliJson @('files','delete',$docs,'true')
    $deleteDocs | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'delete-directory.json') -Encoding utf8
    if (Test-Path -LiteralPath $docs) { throw 'recursive directory delete did not remove target' }

    $chain = Invoke-CliJson @('audit','verify')
    $chain | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.16';
        artifact='workspace-text-file-operations-windows-source-first-scaffold';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        workspace_registration_verified=$true;
        list_mkdir_verified=$true;
        exact_unicode_read_write_verified=$true;
        transactional_overwrite_verified=$true;
        move_delete_verified=$true;
        workspace_root_protection_verified=$true;
        outside_workspace_containment_verified=$true;
        junction_escape_and_mutation_rejected=$true;
        text_read_size_bound_verified=$true;
        audit_chain_valid=$true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $script:Root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $script:Root 'evidence.json.sha256')
}
finally {
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
