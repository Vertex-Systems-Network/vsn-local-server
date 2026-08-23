param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Invoke-BinaryWrite(
    [string]$Path,
    [string]$TransferId,
    [uint64]$Offset,
    [bool]$Finalize,
    [byte[]]$Bytes,
    [string]$OutputPath,
    [string]$ErrorPath
) {
    $b64 = [Convert]::ToBase64String($Bytes)
    $b64 | & $script:Cli files binary-write $Path $TransferId $Offset ($Finalize.ToString().ToLowerInvariant()) 1> $OutputPath 2> $ErrorPath
    return $LASTEXITCODE
}

$root = Join-Path $PWD 'dist-self-hosted\02.17'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0217-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$outside = Join-Path $sandbox 'outside'
$source = Join-Path $sandbox 'source.bin'
$target = Join-Path $workspace 'payload.bin'
$reconstructed = Join-Path $sandbox 'reconstructed.bin'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$ipcKeyHashBefore = if ($hadIpcKey) { (Get-FileHash $ipcKey -Algorithm SHA256).Hash } else { $null }
$agent = $null
$outsideLink = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$workspace,$outside,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

function Start-Agent {
    $script:agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput (Join-Path $root 'agent.stdout.log') -RedirectStandardError (Join-Path $root 'agent.stderr.log') -PassThru -WindowStyle Hidden
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
    if (-not $IsWindows) { throw "02.17 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.17 certification requires a GitHub-hosted runner' }
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $actual = (git rev-parse HEAD).Trim()
    if ($env:EXPECTED_SHA -and $actual -ne $env:EXPECTED_SHA) { throw "02.17 source binding mismatch: expected=$env:EXPECTED_SHA actual=$actual" }
    Write-Host "source=$actual runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH ipc=127.0.0.1:39731"

    $files = Get-Content 'crates/vsn-files/src/lib.rs' -Raw
    foreach ($needle in @('MAX_BINARY_CHUNK_BYTES','MAX_BINARY_FILE_BYTES','pub fn read_binary_chunk','pub fn write_binary_chunk','pub fn abort_binary_upload','pub fn binary_upload_status','pub fn file_digest','recover_binary_replace','staged_replace')) {
        if (-not $files.Contains($needle)) { throw "missing binary-transfer invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('files.binary.read','files.binary.write','files.binary.abort','files.binary.status','files.digest')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent transfer command: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-files --all-targets -- -D warnings
    Assert-LastExit 'binary transfer clippy failed'
    cargo test --locked --package vsn-files --package vsn-core
    Assert-LastExit 'binary transfer tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $payload = New-Object byte[] 1049003
    for ($i = 0; $i -lt $payload.Length; $i++) { $payload[$i] = [byte]($i % 251) }
    [IO.File]::WriteAllBytes($source, $payload)
    $expectedSha = (Get-FileHash $source -Algorithm SHA256).Hash.ToLowerInvariant()

    if (Get-NetTCPConnection -LocalPort 39731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 39731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent
    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    $abortTransfer = 'transfer_abort_0217'
    $firstLength = 300000
    $first = New-Object byte[] $firstLength
    [Array]::Copy($payload, 0, $first, 0, $firstLength)
    $code = Invoke-BinaryWrite $target $abortTransfer 0 $false $first (Join-Path $root 'abort-first.json') (Join-Path $root 'abort-first.err')
    if ($code -ne 0) { throw 'initial resumable chunk failed' }
    $status = & $script:Cli files binary-status $target $abortTransfer | Out-String | ConvertFrom-Json
    $status | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'status-after-first.json') -Encoding utf8
    if ([uint64]$status.committed_bytes -ne $firstLength -or $status.partial_exists -ne $true -or $status.final_exists -ne $false) {
        throw 'binary status did not report first committed chunk'
    }

    $probe = New-Object byte[] 16
    [Array]::Copy($payload, $firstLength, $probe, 0, 16)
    $wrongCode = Invoke-BinaryWrite $target $abortTransfer ($firstLength - 1) $false $probe (Join-Path $root 'wrong-offset.stdout') (Join-Path $root 'wrong-offset.stderr')
    $wrongCode | Set-Content (Join-Path $root 'wrong-offset.exit-code.txt')
    if ($wrongCode -eq 0) { throw 'wrong binary offset unexpectedly succeeded' }
    $statusAfterWrong = & $script:Cli files binary-status $target $abortTransfer | Out-String | ConvertFrom-Json
    if ([uint64]$statusAfterWrong.committed_bytes -ne $firstLength) { throw 'wrong offset advanced committed bytes' }

    & $script:Cli files binary-abort $target $abortTransfer | Set-Content (Join-Path $root 'abort.json') -Encoding utf8
    Assert-LastExit 'binary abort failed'
    $statusAfterAbort = & $script:Cli files binary-status $target $abortTransfer | Out-String | ConvertFrom-Json
    $statusAfterAbort | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'status-after-abort.json') -Encoding utf8
    if ($statusAfterAbort.partial_exists -ne $false -or [uint64]$statusAfterAbort.committed_bytes -ne 0) { throw 'abort did not discard partial transfer' }

    $restartBytes = New-Object byte[] 32
    [Array]::Copy($payload, 0, $restartBytes, 0, $restartBytes.Length)
    $restartCode = Invoke-BinaryWrite $target $abortTransfer 0 $false $restartBytes (Join-Path $root 'restart-after-abort.json') (Join-Path $root 'restart-after-abort.err')
    if ($restartCode -ne 0) { throw 'transfer did not restart cleanly after abort' }
    & $script:Cli files binary-abort $target $abortTransfer | Set-Content (Join-Path $root 'restart-abort.json') -Encoding utf8
    Assert-LastExit 'cleanup abort after restart failed'

    $transfer = 'transfer_finalize_0217'
    $offset = 0
    $chunkSize = 400000
    $partIndex = 0
    while ($offset -lt $payload.Length) {
        $remaining = $payload.Length - $offset
        $length = [Math]::Min($chunkSize, $remaining)
        $chunk = New-Object byte[] $length
        [Array]::Copy($payload, $offset, $chunk, 0, $length)
        $finalize = ($offset + $length) -eq $payload.Length
        $out = Join-Path $root ("upload-{0:D2}.json" -f $partIndex)
        $err = Join-Path $root ("upload-{0:D2}.err" -f $partIndex)
        $code = Invoke-BinaryWrite $target $transfer ([uint64]$offset) $finalize $chunk $out $err
        if ($code -ne 0) { throw "binary chunk $partIndex failed" }
        $result = Get-Content $out -Raw | ConvertFrom-Json
        if ([uint64]$result.committed_bytes -ne ($offset + $length)) { throw "chunk $partIndex reported wrong committed bytes" }
        if ($finalize) {
            if ($result.complete -ne $true) { throw 'final chunk did not report complete' }
            if ([string]$result.sha256 -ne $expectedSha) { throw 'finalize returned wrong SHA-256' }
        } elseif ($result.complete -ne $false) {
            throw 'non-final chunk incorrectly reported complete'
        }
        $offset += $length
        $partIndex++
    }

    $finalStatus = & $script:Cli files binary-status $target $transfer | Out-String | ConvertFrom-Json
    $finalStatus | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'status-final.json') -Encoding utf8
    if ($finalStatus.partial_exists -ne $false -or $finalStatus.final_exists -ne $true) { throw 'final status is inconsistent' }
    $digest = & $script:Cli files digest $target | Out-String | ConvertFrom-Json
    $digest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'digest.json') -Encoding utf8
    if ([string]$digest.sha256 -ne $expectedSha -or [uint64]$digest.bytes -ne $payload.Length) { throw 'Agent digest disagrees with source payload' }

    $output = [IO.File]::Open($reconstructed, [IO.FileMode]::Create, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
        $readOffset = 0
        do {
            $chunkJson = & $script:Cli files binary-read $target $readOffset 262144 | Out-String | ConvertFrom-Json
            $decoded = [Convert]::FromBase64String([string]$chunkJson.data_b64)
            if ([uint64]$chunkJson.offset -ne $readOffset -or [int]$chunkJson.bytes -ne $decoded.Length) { throw 'binary-read metadata mismatch' }
            $chunkDigest = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($decoded)).ToLowerInvariant()
            if ([string]$chunkJson.chunk_sha256 -ne $chunkDigest) { throw 'binary-read chunk SHA-256 mismatch' }
            $output.Write($decoded, 0, $decoded.Length)
            $readOffset += $decoded.Length
        } while ($chunkJson.eof -ne $true)
    }
    finally { $output.Dispose() }
    if ((Get-FileHash $reconstructed -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedSha) { throw 'chunked download reconstruction digest mismatch' }

    $outsideTarget = Join-Path $outside 'escape.bin'
    $outsideCode = Invoke-BinaryWrite $outsideTarget 'transfer_outside_0217' 0 $true ([byte[]](1,2,3)) (Join-Path $root 'outside.stdout') (Join-Path $root 'outside.stderr')
    if ($outsideCode -eq 0 -or (Test-Path -LiteralPath $outsideTarget)) { throw 'outside-workspace binary write escaped containment' }

    [IO.File]::WriteAllBytes((Join-Path $outside 'outside-source.bin'), [byte[]](9,8,7,6))
    $outsideLink = Join-Path $workspace 'outside-link'
    New-Item -ItemType Junction -Path $outsideLink -Target $outside | Out-Null
    $junctionTarget = Join-Path $outsideLink 'junction-escape.bin'
    $junctionWriteCode = Invoke-BinaryWrite $junctionTarget 'transfer_junction_0217' 0 $true ([byte[]](4,5,6)) (Join-Path $root 'junction-write.stdout') (Join-Path $root 'junction-write.stderr')
    if ($junctionWriteCode -eq 0 -or (Test-Path -LiteralPath (Join-Path $outside 'junction-escape.bin'))) { throw 'junction outside binary write escaped containment' }
    & $script:Cli files binary-read (Join-Path $outsideLink 'outside-source.bin') 0 16 1> (Join-Path $root 'junction-read.stdout') 2> (Join-Path $root 'junction-read.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'junction outside binary read unexpectedly succeeded' }
    Remove-Item -LiteralPath $outsideLink -Force
    $outsideLink = $null

    $badIdCode = Invoke-BinaryWrite (Join-Path $workspace 'bad-id.bin') '../../bad' 0 $true ([byte[]](1,2,3)) (Join-Path $root 'bad-id.stdout') (Join-Path $root 'bad-id.stderr')
    if ($badIdCode -eq 0) { throw 'unsafe transfer id unexpectedly succeeded' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task_id = '02.17'
        artifact = 'resumable-binary-workspace-transfer-windows-github-hosted'
        product_version = $candidate.product_version
        candidate_id = $candidate.candidate_id
        source_commit = $actual
        runner_name = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        ipc_address = '127.0.0.1:39731'
        checks = [ordered]@{
            exact_offset_enforced = $true
            status_verified = $true
            abort_verified = $true
            restart_after_abort_verified = $true
            multi_chunk_resume_verified = $true
            finalize_sha256_verified = $true
            agent_digest_verified = $true
            chunked_download_verified = $true
            chunk_sha256_verified = $true
            direct_workspace_containment_verified = $true
            junction_workspace_containment_verified = $true
            unsafe_transfer_id_rejected = $true
            crash_recovery_tests_verified = $true
            audit_chain_valid = $true
        }
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if ($outsideLink -and (Test-Path -LiteralPath $outsideLink)) {
        Remove-Item -LiteralPath $outsideLink -Force -ErrorAction SilentlyContinue
    }
    if ($hadIpcKey) {
        if (-not (Test-Path -LiteralPath $ipcKey)) { throw 'pre-existing IPC key disappeared during 02.17 certification' }
        $ipcKeyHashAfter = (Get-FileHash $ipcKey -Algorithm SHA256).Hash
        if ($ipcKeyHashAfter -ne $ipcKeyHashBefore) { throw 'pre-existing IPC key changed during 02.17 certification' }
    } elseif (Test-Path -LiteralPath $ipcKey) {
        Remove-Item -LiteralPath $ipcKey -Force
    }
    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force
    }
    if (Test-Path -LiteralPath $sandbox) { throw '02.17 sandbox cleanup failed' }
    [ordered]@{
        agent_stopped = $true
        localappdata_restored = ($env:LOCALAPPDATA -eq $originalLocalAppData)
        ipc_key_restored = $true
        sandbox_removed = $true
    } | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $root 'cleanup.json') -Encoding utf8
}