param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Invoke-Mutation([string]$Subcommand, [string]$DbPath, [string]$Entity, $Request, [string]$Out, [string]$Err) {
    $json = $Request | ConvertTo-Json -Depth 10 -Compress
    $json | & $script:Cli db $Subcommand $DbPath $Entity 1> $Out 2> $Err
    return $LASTEXITCODE
}

$root = Join-Path $PWD 'dist-self-hosted\02.25'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0225-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$outside = Join-Path $sandbox 'outside'
$insideDb = Join-Path $workspace 'studio.db'
$outsideDb = Join-Path $outside 'outside.db'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$workspace,$outside,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

function Start-Agent {
    $script:agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput (Join-Path $root 'agent.stdout.log') -RedirectStandardError (Join-Path $root 'agent.stderr.log') -PassThru -WindowStyle Hidden
    foreach ($i in 1..80) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { return }
        if ($script:agent.HasExited) { throw "Agent exited before readiness with code $($script:agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent did not become ready'
}

function Stop-Agent {
    if ($script:agent -and -not $script:agent.HasExited) {
        Stop-Process -Id $script:agent.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $script:agent.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
    $script:agent = $null
}

try {
    if (-not $IsWindows) { throw '02.25 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.25 certification requires a GitHub-hosted runner' }
    Write-Host "runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $sqlite = Get-Content 'crates/vsn-database-sqlite/src/lib.rs' -Raw
    foreach ($needle in @('SqliteProvider','fn query','fn browse','fn insert','fn update','fn delete','fn list_indexes','fn list_relations','fn statistics','validate_read_query')) {
        if (-not $sqlite.Contains($needle)) { throw "missing SQLite Studio invariant: $needle" }
    }
    $core = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('pub fn sqlite_inspect','pub fn sqlite_query','pub fn sqlite_browse','pub fn sqlite_insert','pub fn sqlite_update','pub fn sqlite_delete')) {
        if (-not $core.Contains($needle)) { throw "missing Core SQLite command: $needle" }
    }
    $ipc = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    if (-not $ipc.Contains('MAX_FRAME_BYTES: usize = 1024 * 1024')) { throw '02.25 acceptance expects current 1 MiB IPC frame contract' }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-database-sqlite --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'SQLite/core clippy failed'
    cargo test --locked --package vsn-database-sqlite --package vsn-core
    Assert-LastExit 'SQLite/core tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn --package vsn-database-sqlite --example pkg02_fixture
    Assert-LastExit 'Agent/CLI/SQLite fixture build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $fixture = Join-Path $PWD 'target\release\examples\pkg02_fixture.exe'
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    & $fixture $insideDb *> $null
    Assert-LastExit 'inside SQLite fixture creation failed'
    & $fixture $outsideDb *> $null
    Assert-LastExit 'outside SQLite fixture creation failed'

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb another Agent' }
    Start-Agent
    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    $inspect = & $script:Cli db sqlite-inspect $insideDb | Out-String | ConvertFrom-Json
    $inspect | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $root 'inspect.json') -Encoding utf8
    $inspectText = $inspect | ConvertTo-Json -Depth 10 -Compress
    if (-not $inspectText.Contains('users') -or -not $inspectText.Contains('teams')) { throw 'SQLite inspect missed expected entities' }

    $browse = & $script:Cli db sqlite-browse $insideDb users | Out-String | ConvertFrom-Json
    $browse | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'browse.json') -Encoding utf8
    if ([uint64]$browse.total_rows -lt 2 -or -not (@($browse.columns) -contains 'email')) { throw 'SQLite browse contract failed' }

    $query = & $script:Cli db sqlite-query $insideDb "SELECT id,name,email FROM users WHERE name='Alice'" | Out-String | ConvertFrom-Json
    $query | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'query.json') -Encoding utf8
    if ([int]$query.row_count -ne 1 -or [string]$query.rows[0].email -ne 'alice@example.test') { throw 'safe SQLite SELECT failed' }

    & $script:Cli db sqlite-query $insideDb 'DELETE FROM users' 1> (Join-Path $root 'unsafe-query.stdout') 2> (Join-Path $root 'unsafe-query.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'mutating SQL passed read-query boundary' }

    $indexes = & $script:Cli db sqlite-indexes $insideDb users | Out-String | ConvertFrom-Json
    $indexes | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'indexes.json') -Encoding utf8
    if (-not (($indexes | ConvertTo-Json -Depth 8 -Compress).Contains('idx_users_name'))) { throw 'SQLite indexes missed deterministic index' }

    $relations = & $script:Cli db sqlite-relations $insideDb users | Out-String | ConvertFrom-Json
    $relations | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'relations.json') -Encoding utf8
    if (-not (($relations | ConvertTo-Json -Depth 8 -Compress).Contains('teams'))) { throw 'SQLite relations missed foreign key' }

    $stats = & $script:Cli db sqlite-stats $insideDb users | Out-String | ConvertFrom-Json
    $stats | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'stats.json') -Encoding utf8
    if ([uint64]$stats.row_count -lt 2) { throw 'SQLite stats row count invalid' }

    $insertReq = [ordered]@{values=[ordered]@{team_id=1;name='Bob';email='bob@example.test';note='created'};filter=@{}}
    $insertOut = Join-Path $root 'insert.json'
    if ((Invoke-Mutation 'sqlite-insert' $insideDb 'users' $insertReq $insertOut (Join-Path $root 'insert.stderr')) -ne 0) { throw 'SQLite structured insert failed' }
    $insert = Get-Content $insertOut -Raw | ConvertFrom-Json
    if ([uint64]$insert.affected_rows -ne 1) { throw 'SQLite insert affected unexpected rows' }

    $updateReq = [ordered]@{values=[ordered]@{note='updated'};filter=[ordered]@{email='bob@example.test'}}
    $updateOut = Join-Path $root 'update.json'
    if ((Invoke-Mutation 'sqlite-update' $insideDb 'users' $updateReq $updateOut (Join-Path $root 'update.stderr')) -ne 0) { throw 'SQLite structured update failed' }
    if ([uint64](Get-Content $updateOut -Raw | ConvertFrom-Json).affected_rows -ne 1) { throw 'SQLite update affected unexpected rows' }

    $deleteReq = [ordered]@{values=@{};filter=[ordered]@{email='bob@example.test'}}
    $deleteOut = Join-Path $root 'delete.json'
    if ((Invoke-Mutation 'sqlite-delete' $insideDb 'users' $deleteReq $deleteOut (Join-Path $root 'delete.stderr')) -ne 0) { throw 'SQLite structured delete failed' }
    if ([uint64](Get-Content $deleteOut -Raw | ConvertFrom-Json).affected_rows -ne 1) { throw 'SQLite delete affected unexpected rows' }

    $emptyFilterReq = [ordered]@{values=[ordered]@{note='unsafe'};filter=@{}}
    if ((Invoke-Mutation 'sqlite-update' $insideDb 'users' $emptyFilterReq (Join-Path $root 'empty-filter.stdout') (Join-Path $root 'empty-filter.stderr')) -eq 0) { throw 'SQLite update without filter unexpectedly succeeded' }

    # Registered workspace boundaries must contain every SQLite database path.
    & $script:Cli db sqlite-inspect $outsideDb 1> (Join-Path $root 'outside-inspect.stdout') 2> (Join-Path $root 'outside-inspect.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'outside-workspace SQLite database was exposed through Agent' }

    # Large provider results must be rejected/truncated before the 1 MiB IPC frame boundary.
    $largeOut = Join-Path $root 'large-query.stdout'
    $largeErr = Join-Path $root 'large-query.stderr'
    & $script:Cli db sqlite-query $insideDb "SELECT note FROM users WHERE name='Large'" 1> $largeOut 2> $largeErr
    $largeCode = $LASTEXITCODE
    $largeCode | Set-Content (Join-Path $root 'large-query.exit-code.txt')
    if ($largeCode -eq 0 -and (Get-Item $largeOut).Length -ge 900000) { throw 'SQLite result exceeded frame-safe serialized budget' }
    if ($largeCode -ne 0) {
        $errorText = Get-Content $largeErr -Raw
        if ($errorText -match 'frame exceeds maximum size') { throw 'SQLite provider result limit is not aligned with IPC frame budget' }
    }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.25';
        artifact='sqlite-database-studio-windows-github-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_environment=$env:RUNNER_ENVIRONMENT;
        inspect_browse_query_verified=$true; indexes_relations_stats_verified=$true; structured_crud_verified=$true;
        mutation_safety_verified=$true; workspace_containment_verified=$true; frame_safe_result_verified=$true; audit_chain_valid=$true
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
