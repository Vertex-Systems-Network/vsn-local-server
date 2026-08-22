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

function Get-BytesSha256([byte[]]$Bytes) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-','').ToLowerInvariant()
    }
    finally { $sha.Dispose() }
}

function Get-FileSha256([string]$Path) {
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

$script:Root = Join-Path $PWD 'dist-self-hosted\02.17'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0217-' + [guid]::NewGuid().ToString('N'))
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
    if (-not $IsWindows) { throw "02.17 acceptance requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $script:Root 'runner.txt')

    $filesSource = (Get-Content 'crates/vsn-files/src/lib.rs' -Raw) + "`n" + (Get-Content 'crates/vsn-files/src/lib_base.rs' -Raw)
    foreach ($needle in @(
        'MAX_BINARY_CHUNK_BYTES',
        'MAX_BINARY_FILE_BYTES',
        'binary_upload_status',
        'binary upload has pending recovery',
        'create_new(true)',
        'binary upload partial changed during resume',
        'workspace root itself cannot be used as a binary file destination',
        'binary_metadata_is_link_like',
        'FILE_ATTRIBUTE_REPARSE_POINT',
        'sha256_reader',
        'checksum_mismatch_preserves_existing_destination',
        'precreated_partial_symlink_is_rejected_without_touching_target'
    )) {
        if (-not $filesSource.Contains($needle)) { throw "missing 02.17 source invariant: $needle" }
    }

    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @(
        'pub fn file_read_binary_chunk',
        'pub fn file_write_binary_chunk',
        'pub fn file_abort_binary_upload',
        'pub fn file_binary_upload_status',
        'pub fn file_digest',
        'Permission::FilesRead',
        'Permission::FilesWrite'
    )) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.17 Core boundary invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('files.binary.read','files.binary.write','files.binary.abort','files.binary.status','files.digest')) {
        if (-not $agentSource.Contains($needle)) { throw "missing 02.17 Agent route invariant: $needle" }
    }
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw
    foreach ($needle in @('binary-read','binary-write','binary-abort','binary-status','files.digest')) {
        if (-not $cliSource.Contains($needle)) { throw "missing 02.17 CLI route invariant: $needle" }
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
    Invoke-CliJson @('workspace','add',$workspace) | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $script:Root 'workspace-add.json') -Encoding utf8

    $sourcePath = Join-Path $sandbox 'source.bin'
    $sourceBytes = [byte[]]::new(1200123)
    $random = [System.Random]::new(2170217)
    $random.NextBytes($sourceBytes)
    [System.IO.File]::WriteAllBytes($sourcePath, $sourceBytes)
    $sourceSha = Get-FileSha256 $sourcePath
    $sourceSha | Set-Content (Join-Path $script:Root 'source.sha256')

    $target = Join-Path $workspace 'payload.bin'
    $transfer = 'transfer_main_0217'
    $chunkSize = 524288
    $firstCount = [Math]::Min($chunkSize, $sourceBytes.Length)
    $firstB64 = [Convert]::ToBase64String($sourceBytes, 0, $firstCount)
    $firstWrite = Invoke-CliWithStdin @('files','binary-write',$target,$transfer,'0','false') $firstB64
    $first = $firstWrite.Stdout | ConvertFrom-Json
    $first | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'write-first.json') -Encoding utf8
    if ([uint64]$first.committed_bytes -ne [uint64]$firstCount -or $first.complete -ne $false) { throw 'first binary chunk state mismatch' }

    $status1 = Invoke-CliJson @('files','binary-status',$target,$transfer)
    $status1 | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'status-first.json') -Encoding utf8
    if ([uint64]$status1.committed_bytes -ne [uint64]$firstCount -or $status1.partial_exists -ne $true -or $status1.final_exists -ne $false) {
        throw 'binary upload status after first chunk is inconsistent'
    }

    $secondOffset = $firstCount
    $secondCount = [Math]::Min($chunkSize, $sourceBytes.Length - $secondOffset)
    $secondB64 = [Convert]::ToBase64String($sourceBytes, $secondOffset, $secondCount)
    $mismatch = Invoke-CliWithStdin @('files','binary-write',$target,$transfer,[string]($secondOffset - 1),'false') $secondB64 $false
    $mismatch.Stdout | Set-Content (Join-Path $script:Root 'offset-mismatch.stdout') -Encoding utf8
    $mismatch.Stderr | Set-Content (Join-Path $script:Root 'offset-mismatch.stderr') -Encoding utf8
    $mismatch.ExitCode | Set-Content (Join-Path $script:Root 'offset-mismatch.exit-code.txt')
    if ($mismatch.ExitCode -eq 0 -or $mismatch.Stderr -notmatch 'offset mismatch') { throw 'binary offset mismatch was not rejected explicitly' }
    $statusAfterMismatch = Invoke-CliJson @('files','binary-status',$target,$transfer)
    if ([uint64]$statusAfterMismatch.committed_bytes -ne [uint64]$firstCount) { throw 'offset mismatch changed committed binary bytes' }

    $secondWrite = Invoke-CliWithStdin @('files','binary-write',$target,$transfer,[string]$secondOffset,'false') $secondB64
    $second = $secondWrite.Stdout | ConvertFrom-Json
    $expectedAfterSecond = $firstCount + $secondCount
    if ([uint64]$second.committed_bytes -ne [uint64]$expectedAfterSecond -or $second.complete -ne $false) { throw 'binary resume second chunk mismatch' }
    $status2 = Invoke-CliJson @('files','binary-status',$target,$transfer)
    $status2 | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'status-second.json') -Encoding utf8
    if ([uint64]$status2.committed_bytes -ne [uint64]$expectedAfterSecond) { throw 'binary status lost resumed offset' }

    $finalOffset = $expectedAfterSecond
    $finalCount = $sourceBytes.Length - $finalOffset
    $finalB64 = [Convert]::ToBase64String($sourceBytes, $finalOffset, $finalCount)
    $finalWrite = Invoke-CliWithStdin @('files','binary-write',$target,$transfer,[string]$finalOffset,'true') $finalB64
    $final = $finalWrite.Stdout | ConvertFrom-Json
    $final | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'write-final.json') -Encoding utf8
    if ($final.complete -ne $true -or [uint64]$final.committed_bytes -ne [uint64]$sourceBytes.Length) { throw 'binary finalize did not commit complete payload' }
    if ([string]$final.sha256 -ne $sourceSha) { throw 'finalize SHA-256 does not match source payload' }
    if ((Get-FileSha256 $target) -ne $sourceSha) { throw 'finalized file bytes do not match source payload' }

    $digest = Invoke-CliJson @('files','digest',$target)
    $digest | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'digest.json') -Encoding utf8
    if ([string]$digest.sha256 -ne $sourceSha -or [uint64]$digest.bytes -ne [uint64]$sourceBytes.Length) { throw 'Agent file digest verification mismatch' }

    $reconstructed = [System.IO.MemoryStream]::new()
    try {
        [uint64]$readOffset = 0
        do {
            $chunk = Invoke-CliJson @('files','binary-read',$target,[string]$readOffset,'999999')
            if ([int]$chunk.bytes -gt $chunkSize) { throw "binary read exceeded 512 KiB chunk ceiling: $($chunk.bytes)" }
            $decoded = [Convert]::FromBase64String([string]$chunk.data_b64)
            if ($decoded.Length -ne [int]$chunk.bytes) { throw 'binary read byte count/base64 payload mismatch' }
            if ((Get-BytesSha256 $decoded) -ne [string]$chunk.chunk_sha256) { throw 'binary read chunk SHA-256 mismatch' }
            $reconstructed.Write($decoded, 0, $decoded.Length)
            $readOffset += [uint64]$chunk.bytes
        } while ($chunk.eof -ne $true)
        $rebuilt = $reconstructed.ToArray()
    }
    finally { $reconstructed.Dispose() }
    if ($rebuilt.Length -ne $sourceBytes.Length -or (Get-BytesSha256 $rebuilt) -ne $sourceSha) { throw 'chunked binary read did not reconstruct exact source payload' }

    $beyondOut = Join-Path $script:Root 'read-beyond.stdout'
    $beyondErr = Join-Path $script:Root 'read-beyond.stderr'
    $beyondCode = Invoke-CliCapture @('files','binary-read',$target,[string]($sourceBytes.Length + 1),'1024') $beyondOut $beyondErr
    if ($beyondCode -eq 0) { throw 'binary read accepted offset beyond end of file' }

    $abortTarget = Join-Path $workspace 'abort.bin'
    $abortTransfer = 'transfer_abort_0217'
    $abortCount = 65536
    $abortB64 = [Convert]::ToBase64String($sourceBytes, 0, $abortCount)
    Invoke-CliWithStdin @('files','binary-write',$abortTarget,$abortTransfer,'0','false') $abortB64 | Out-Null
    $abortStatus = Invoke-CliJson @('files','binary-status',$abortTarget,$abortTransfer)
    if ([uint64]$abortStatus.committed_bytes -ne [uint64]$abortCount -or $abortStatus.partial_exists -ne $true) { throw 'abort fixture upload did not persist expected partial' }
    $aborted = Invoke-CliJson @('files','binary-abort',$abortTarget,$abortTransfer)
    $aborted | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $script:Root 'abort.json') -Encoding utf8
    if ($aborted -ne $true) { throw 'binary abort did not remove valid partial upload' }
    $abortAfter = Invoke-CliJson @('files','binary-status',$abortTarget,$abortTransfer)
    if ($abortAfter.partial_exists -ne $false -or $abortAfter.final_exists -ne $false -or [uint64]$abortAfter.committed_bytes -ne 0) { throw 'binary abort left transfer state behind' }

    $oversizedTarget = Join-Path $workspace 'oversized.bin'
    $oversizedTransfer = 'transfer_oversized_0217'
    $oversizedBytes = [byte[]]::new($chunkSize + 1)
    $oversizedB64 = [Convert]::ToBase64String($oversizedBytes)
    $oversized = Invoke-CliWithStdin @('files','binary-write',$oversizedTarget,$oversizedTransfer,'0','false') $oversizedB64 $false
    if ($oversized.ExitCode -eq 0) { throw 'binary upload accepted a chunk above 512 KiB' }
    $oversizedPart = Join-Path $workspace ('.oversized.bin.vsn-upload-' + $oversizedTransfer + '.part')
    if (Test-Path -LiteralPath $oversizedPart) { throw 'oversized rejected chunk left a partial file' }

    $rootTransfer = 'transfer_root_0217'
    $rootWrite = Invoke-CliWithStdin @('files','binary-write',$workspace,$rootTransfer,'0','false') ([Convert]::ToBase64String([byte[]](1,2,3))) $false
    if ($rootWrite.ExitCode -eq 0) { throw 'binary write accepted workspace root as file destination' }
    $rootSiblingLeaks = @(Get-ChildItem -LiteralPath $sandbox -Force | Where-Object { $_.Name -like '.workspace.vsn-*' })
    if ($rootSiblingLeaks.Count -ne 0) { throw 'workspace-root binary write created an out-of-root sibling artifact' }

    $hostileTarget = Join-Path $workspace 'hostile.bin'
    $hostileTransfer = 'transfer_junction_0217'
    $hostilePart = Join-Path $workspace ('.hostile.bin.vsn-upload-' + $hostileTransfer + '.part')
    $hostileOutside = Join-Path $outside 'hostile-part-target'
    New-Item -ItemType Directory -Force -Path $hostileOutside | Out-Null
    $hostileSentinel = Join-Path $hostileOutside 'keep.txt'
    'keep-hostile-target' | Set-Content -LiteralPath $hostileSentinel -Encoding utf8
    $hostileSha = Get-FileSha256 $hostileSentinel
    New-Item -ItemType Junction -Path $hostilePart -Target $hostileOutside | Out-Null
    $hostile = Invoke-CliWithStdin @('files','binary-write',$hostileTarget,$hostileTransfer,'0','false') ([Convert]::ToBase64String([byte[]](9,8,7))) $false
    $hostile.Stderr | Set-Content (Join-Path $script:Root 'hostile-part.stderr') -Encoding utf8
    if ($hostile.ExitCode -eq 0) { throw 'binary upload followed hostile junction partial object' }
    if ((Get-FileSha256 $hostileSentinel) -ne $hostileSha) { throw 'hostile partial attempt changed outside target' }
    if (-not (Test-Path -LiteralPath $hostilePart)) { throw 'hostile partial rejection mutated the junction object' }
    Remove-Item -LiteralPath $hostilePart -Force

    $recoveryTarget = Join-Path $workspace 'recovery.bin'
    $recoveryTransfer = 'transfer_recovery_0217'
    $recoveryBackup = Join-Path $workspace ('.recovery.bin.vsn-backup-' + $recoveryTransfer + '.bak')
    [System.IO.File]::WriteAllBytes($recoveryBackup, [byte[]](11,22,33,44))
    $backupSha = Get-FileSha256 $recoveryBackup
    $statusOut = Join-Path $script:Root 'pending-recovery-status.stdout'
    $statusErr = Join-Path $script:Root 'pending-recovery-status.stderr'
    $statusCode = Invoke-CliCapture @('files','binary-status',$recoveryTarget,$recoveryTransfer) $statusOut $statusErr
    if ($statusCode -eq 0) { throw 'read-only binary status performed or ignored pending recovery' }
    if (-not (Test-Path -LiteralPath $recoveryBackup) -or (Get-FileSha256 $recoveryBackup) -ne $backupSha -or (Test-Path -LiteralPath $recoveryTarget)) {
        throw 'read-only binary status mutated pending recovery state'
    }
    $recoveryAbort = Invoke-CliJson @('files','binary-abort',$recoveryTarget,$recoveryTransfer)
    if ($recoveryAbort -ne $false) { throw 'recovery-only abort incorrectly reported a removed partial' }
    if (-not (Test-Path -LiteralPath $recoveryTarget) -or (Get-FileSha256 $recoveryTarget) -ne $backupSha -or (Test-Path -LiteralPath $recoveryBackup)) {
        throw 'write-authorized abort did not safely recover previous binary destination'
    }

    $chain = Invoke-CliJson @('audit','verify')
    $chain | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.17';
        artifact='resumable-binary-workspace-transfer-windows-source-first-scaffold';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        chunk_ceiling_verified=$true;
        exact_offset_resume_verified=$true;
        offset_mismatch_non_mutating_verified=$true;
        read_only_status_verified=$true;
        abort_verified=$true;
        finalize_sha256_verified=$true;
        agent_digest_verified=$true;
        chunk_digest_and_reconstruction_verified=$true;
        workspace_root_containment_verified=$true;
        hostile_partial_reparse_rejected=$true;
        pending_recovery_write_permission_boundary_verified=$true;
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