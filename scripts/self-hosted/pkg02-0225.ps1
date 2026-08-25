param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$FeatureId = 'pkg02-0225-sqlite-database-studio'
$FeatureVersion = '1.0.0'
$CanonicalBaseSha = 'd9b2cd0272c6f1e37119dfa7ea09fbd83dbf1842'
$PlanSha256 = 'b7245eec0e8a88ab5f464a86a62db76d71f3ea21ceff465e6335e066be9665df'
$ResearchSha256 = '19a02e45b42b2f2e7403a274207afa14b968ff91ff5db8708605f9b735939d4b'
$LifecycleSha256 = '4103e88568c88e0f1c540aa3a5c79e56d6696382f068d7f6387cf53ca9c9f0da'
$PreflightSha256 = '13d77df56dfe26f9fe5cb705bc8db82f0663b3801227b09944bbfad2f467f10a'
$CandidateId = 'c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474'
$ProductVersion = '0.38.1'
$AgentIpcPort = 39731
$MaxReadResultBytes = 512 * 1024
$MaxTextCellBytes = 256 * 1024
$MaxIpcBytes = 1024 * 1024

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

function Invoke-CliJson([string[]]$CliArgs, [string]$Name, [AllowNull()][string]$StdinText = $null) {
    $out = Join-Path $script:Root "$Name.stdout.json"
    $err = Join-Path $script:Root "$Name.stderr.log"
    if ($null -eq $StdinText) {
        & $script:Cli @CliArgs 1> $out 2> $err
    } else {
        $StdinText | & $script:Cli @CliArgs 1> $out 2> $err
    }
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt") -Encoding ascii
    if ($code -ne 0) {
        $detail = if (Test-Path $err) { Get-Content $err -Raw } else { '' }
        throw "$Name failed (exit=$code): $detail"
    }
    Get-Content $out -Raw | ConvertFrom-Json
}

function Invoke-CliFailure([string[]]$CliArgs, [string]$Name, [AllowNull()][string]$StdinText = $null) {
    $out = Join-Path $script:Root "$Name.stdout.log"
    $err = Join-Path $script:Root "$Name.stderr.log"
    if ($null -eq $StdinText) {
        & $script:Cli @CliArgs 1> $out 2> $err
    } else {
        $StdinText | & $script:Cli @CliArgs 1> $out 2> $err
    }
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt") -Encoding ascii
    if ($code -eq 0) { throw "$Name unexpectedly succeeded" }
    $stdoutText = if (Test-Path $out) { Get-Content $out -Raw } else { '' }
    $stderrText = if (Test-Path $err) { Get-Content $err -Raw } else { '' }
    [pscustomobject]@{ ExitCode = $code; Text = "$stdoutText`n$stderrText" }
}

function Assert-FailureContains($Failure, [string]$Needle, [string]$Name) {
    if (-not $Failure.Text.Contains($Needle)) {
        throw "$Name did not contain '$Needle': $($Failure.Text)"
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

function Assert-SqlitePathDenied([string]$DbPath, [string]$Prefix) {
    $insert = [ordered]@{ values = [ordered]@{ team_id = 1; name = 'Blocked'; email = 'blocked@example.test'; note = 'no-write' }; filter = [ordered]@{} } | ConvertTo-Json -Depth 8 -Compress
    $update = [ordered]@{ values = [ordered]@{ note = 'no-write' }; filter = [ordered]@{ email = 'alice@example.test' } } | ConvertTo-Json -Depth 8 -Compress
    $delete = [ordered]@{ values = [ordered]@{}; filter = [ordered]@{ email = 'alice@example.test' } } | ConvertTo-Json -Depth 8 -Compress

    $failures = @(
        (Invoke-CliFailure @('db','sqlite-inspect',$DbPath) "$Prefix-inspect"),
        (Invoke-CliFailure @('db','sqlite-query',$DbPath,'SELECT name FROM users') "$Prefix-query"),
        (Invoke-CliFailure @('db','sqlite-browse',$DbPath,'users') "$Prefix-browse"),
        (Invoke-CliFailure @('db','sqlite-indexes',$DbPath,'users') "$Prefix-indexes"),
        (Invoke-CliFailure @('db','sqlite-relations',$DbPath,'users') "$Prefix-relations"),
        (Invoke-CliFailure @('db','sqlite-stats',$DbPath,'users') "$Prefix-stats"),
        (Invoke-CliFailure @('db','sqlite-insert',$DbPath,'users') "$Prefix-insert" $insert),
        (Invoke-CliFailure @('db','sqlite-update',$DbPath,'users') "$Prefix-update" $update),
        (Invoke-CliFailure @('db','sqlite-delete',$DbPath,'users') "$Prefix-delete" $delete)
    )
    foreach ($failure in $failures) {
        if (-not $failure.Text.Contains('workspace')) {
            throw "$Prefix SQLite path denial did not fail at workspace containment: $($failure.Text)"
        }
    }
}

$script:Root = Join-Path $PWD 'dist-self-hosted\02.25'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0225-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$outside = Join-Path $sandbox 'outside'
$junction = Join-Path $workspace 'escape'
$insideDb = Join-Path $workspace 'studio.db'
$outsideDb = Join-Path $outside 'outside.db'
$invalidDb = Join-Path $workspace 'invalid.db'
$missingDb = Join-Path $workspace 'missing.db'
$isolated = Join-Path $sandbox 'localappdata'
$originalLocal = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadKey = Test-Path -LiteralPath $ipcKey -PathType Leaf
$originalKeyBytes = if ($hadKey) { [IO.File]::ReadAllBytes($ipcKey) } else { $null }
$originalKeyHash = if ($hadKey) { Get-OptionalSha $ipcKey } else { $null }
$script:Agent = $null
$script:AgentExe = $null
$script:Cli = $null
$script:AgentReadyMs = 0
$agentPid = 0
$outsidePreHash = $null
$outsidePostHash = $null
$maxSuccessfulCliBytes = 0
$auditEventCount = 0
$success = $false
$cleanup = [ordered]@{
    agent_stopped = $false
    junction_removed = $false
    ipc_key_restored = $false
    localappdata_restored = $false
    sandbox_removed = $false
    outside_database_unchanged = $false
    no_privileged_system_mutation = $true
}

if (Test-Path $script:Root) { Remove-Item $script:Root -Recurse -Force }
New-Item -ItemType Directory -Force -Path $script:Root, $bin, $sandbox, $workspace, $outside, $isolated | Out-Null
$env:LOCALAPPDATA = $isolated

try {
    if (-not $IsWindows) { throw '02.25 requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.25 requires GitHub-hosted runner' }
    if ($env:RUNNER_ARCH -ne 'X64') { throw "02.25 requires X64 runner, got $env:RUNNER_ARCH" }
    if (-not $env:EXPECTED_SHA) { throw 'EXPECTED_SHA required' }
    $sourceCommit = (git rev-parse HEAD).Trim()
    if ($sourceCommit -ne $env:EXPECTED_SHA) { throw "exact source mismatch expected=$env:EXPECTED_SHA actual=$sourceCommit" }
    $rustcVersion = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rustcVersion -notmatch '^rustc 1\.97\.1\b') { throw "rustc 1.97.1 required: $rustcVersion" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "cargo 1.97.1 required: $cargoVersion" }
    if (Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue) { throw 'IPC port occupied' }

    Assert-Sha '.ai\plans\pkg02-0225-sqlite-database-studio-v1.md' $PlanSha256 'plan'
    Assert-Sha '.ai\features\pkg02-0225\research.md' $ResearchSha256 'research'
    Assert-Sha '.ai\features\pkg02-0225\lifecycle-review.md' $LifecycleSha256 'lifecycle'
    Assert-Sha '.ai\features\pkg02-0225\development-preflight.md' $PreflightSha256 'preflight'
    $manifest = Get-Content '.ai\manifests\pkg02-0225-sqlite-database-studio.v1.json' -Raw | ConvertFrom-Json
    if ([string]$manifest.feature_id -ne $FeatureId -or [string]$manifest.version -ne $FeatureVersion) { throw 'manifest identity mismatch' }
    if ([string]$manifest.canonical_base_sha -ne $CanonicalBaseSha) { throw 'manifest canonical base mismatch' }
    if ([string]$manifest.plan.sha256 -ne $PlanSha256) { throw 'manifest plan digest mismatch' }
    if ([string]$manifest.research.market_delta -ne 'none') { throw 'market delta not cleared' }
    if (($manifest.acceptance.criteria | Measure-Object).Count -ne 12) { throw 'frozen AC-01..AC-12 set changed' }

    $core = Get-Content 'crates\vsn-core\src\lib.rs' -Raw
    if (-not $core.Contains('fn resolve_sqlite_path(path: &Path)')) { throw 'shared SQLite workspace resolver missing' }
    $resolveCalls = ([regex]::Matches($core, 'let path = resolve_sqlite_path\(path\)\?;')).Count
    if ($resolveCalls -ne 9) { throw "expected 9 SQLite containment calls, got $resolveCalls" }
    $sqlite = Get-Content 'crates\vsn-database-sqlite\src\lib.rs' -Raw
    if (-not $sqlite.Contains('MAX_READ_RESULT_BYTES: usize = 512 * 1024')) { throw '512 KiB SQLite result budget missing' }
    if (-not $sqlite.Contains('MAX_TEXT_CELL_BYTES: usize = 256 * 1024')) { throw '256 KiB SQLite text-cell budget missing' }
    if (-not $sqlite.Contains('serialized JSON safety limit')) { throw 'serialized JSON result accounting missing' }
    $ipc = Get-Content 'crates\vsn-ipc\src\lib.rs' -Raw
    if (-not $ipc.Contains('MAX_FRAME_BYTES: usize = 1024 * 1024')) { throw '1 MiB IPC contract changed' }

    $policy = Get-Content 'crates\vsn-policy\src\lib.rs' -Raw
    $localAuth = [regex]::Match($policy, '(?s)pub fn local_authenticated\(\) -> Self \{.*?pub fn local_network_admin\(\) -> Self \{')
    if (-not $localAuth.Success) { throw 'local authenticated policy block unavailable' }
    foreach ($permission in @('DatabaseView','DatabaseQuery','DatabaseWrite')) {
        if (-not $localAuth.Value.Contains($permission)) { throw "ordinary local principal lacks $permission" }
    }
    if ($localAuth.Value.Contains('DatabaseDestructive')) { throw 'ordinary local principal unexpectedly has DatabaseDestructive' }

    & cargo fmt --all -- --check *> (Join-Path $script:Root 'cargo-fmt.log')
    Assert-Exit 'cargo fmt failed'
    & cargo clippy --locked --package vsn-database-sqlite --package vsn-database --package vsn-core --package vsn-policy --package vsn-agent --all-targets --no-deps -- -D warnings *> (Join-Path $script:Root 'cargo-clippy.log')
    Assert-Exit '02.25 strict Clippy failed'
    & cargo test --locked --package vsn-database-sqlite --package vsn-database --package vsn-core --package vsn-policy *> (Join-Path $script:Root 'cargo-test.log')
    Assert-Exit '02.25 package tests failed'
    & cargo build --locked --release --package vsn-agent --package vsn --package vsn-database-sqlite --example pkg02_fixture *> (Join-Path $script:Root 'cargo-build.log')
    Assert-Exit 'frozen release build command failed'
    & cargo build --locked --release --package vsn-agent --package vsn *> (Join-Path $script:Root 'cargo-agent-cli-build.log')
    Assert-Exit 'release Agent/CLI build failed'
    & git diff --check *> (Join-Path $script:Root 'git-diff-check.log')
    Assert-Exit 'git diff check failed'

    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    Copy-Item 'target\release\examples\pkg02_fixture.exe' (Join-Path $bin 'pkg02_fixture.exe') -Force
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $script:Cli = Join-Path $bin 'vsn.exe'
    $fixture = Join-Path $bin 'pkg02_fixture.exe'
    $agentSha256 = (Get-FileHash $script:AgentExe -Algorithm SHA256).Hash.ToLowerInvariant()
    $cliSha256 = (Get-FileHash $script:Cli -Algorithm SHA256).Hash.ToLowerInvariant()
    $fixtureSha256 = (Get-FileHash $fixture -Algorithm SHA256).Hash.ToLowerInvariant()

    & $fixture $insideDb *> (Join-Path $script:Root 'fixture-inside.log')
    Assert-Exit 'inside fixture creation failed'
    & $fixture $outsideDb *> (Join-Path $script:Root 'fixture-outside.log')
    Assert-Exit 'outside fixture creation failed'
    'not a sqlite database' | Set-Content -LiteralPath $invalidDb -Encoding ascii
    $outsidePreHash = (Get-FileHash $outsideDb -Algorithm SHA256).Hash.ToLowerInvariant()

    Start-Agent
    $agentPid = $script:Agent.Id
    $workspaceAdd = Invoke-CliJson @('workspace','add',$workspace) 'workspace-add'

    # AC-02 inspect and fail-closed invalid/missing paths.
    $inspect = Invoke-CliJson @('db','sqlite-inspect',$insideDb) 'inspect'
    $inspectText = $inspect | ConvertTo-Json -Depth 12 -Compress
    if (-not $inspectText.Contains('"users"') -or -not $inspectText.Contains('"teams"')) { throw 'inspect missed deterministic entities' }
    $usersEntity = @($inspect.entities | Where-Object { [string]$_.entity.name -eq 'users' })
    $teamsEntity = @($inspect.entities | Where-Object { [string]$_.entity.name -eq 'teams' })
    if ($usersEntity.Count -ne 1 -or $teamsEntity.Count -ne 1) { throw 'inspect entity identity mismatch' }
    $expectedUserTypes = [ordered]@{ id = 'integer'; team_id = 'relation'; name = 'text'; email = 'text'; note = 'text' }
    foreach ($expected in $expectedUserTypes.GetEnumerator()) {
        $field = @($usersEntity[0].entity.fields | Where-Object { [string]$_.name -eq [string]$expected.Key })
        if ($field.Count -ne 1 -or [string]$field[0].field_type -ne [string]$expected.Value) {
            throw "inspect users.$($expected.Key) type mismatch expected=$($expected.Value)"
        }
    }
    $teamId = @($usersEntity[0].entity.fields | Where-Object { [string]$_.name -eq 'team_id' })[0]
    if ([string]$teamId.relation_target -ne 'teams') { throw 'inspect team_id relation target mismatch' }
    $expectedTeamTypes = [ordered]@{ id = 'integer'; name = 'text' }
    foreach ($expected in $expectedTeamTypes.GetEnumerator()) {
        $field = @($teamsEntity[0].entity.fields | Where-Object { [string]$_.name -eq [string]$expected.Key })
        if ($field.Count -ne 1 -or [string]$field[0].field_type -ne [string]$expected.Value) {
            throw "inspect teams.$($expected.Key) type mismatch expected=$($expected.Value)"
        }
    }
    Invoke-CliFailure @('db','sqlite-inspect',$missingDb) 'inspect-missing' | Out-Null
    Invoke-CliFailure @('db','sqlite-inspect',$invalidDb) 'inspect-invalid' | Out-Null

    # AC-03 deterministic browse with requested page/order controls.
    $browse = Invoke-CliJson @('db','sqlite-browse',$insideDb,'users','1','1','name','false') 'browse-page'
    if ([uint64]$browse.total_rows -ne 3 -or [uint32]$browse.limit -ne 1 -or [uint64]$browse.offset -ne 1) { throw 'browse page metadata mismatch' }
    if (@($browse.rows).Count -ne 1 -or [string]$browse.rows[0].name -ne 'Charlie') { throw 'browse deterministic ordering mismatch' }
    foreach ($column in @('id','team_id','name','email','note')) {
        if (-not (@($browse.columns) -contains $column)) { throw "browse missing column $column" }
    }

    # AC-04 safe query allow/deny matrix.
    $select = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT id,name,email FROM users WHERE name='Alice'") 'query-select'
    if ([uint64]$select.row_count -ne 1 -or [string]$select.rows[0].email -ne 'alice@example.test') { throw 'SELECT roundtrip failed' }
    Invoke-CliJson @('db','sqlite-query',$insideDb,'EXPLAIN SELECT id FROM users') 'query-explain' | Out-Null
    Invoke-CliJson @('db','sqlite-query',$insideDb,'PRAGMA table_info(users)') 'query-pragma-read' | Out-Null
    $unsafeSql = [ordered]@{
        delete = 'DELETE FROM users'
        update = "UPDATE users SET note='x'"
        insert = "INSERT INTO users(team_id,name,email,note) VALUES(1,'X','x@example.test','x')"
        ddl = 'CREATE TABLE nope(id INTEGER)'
        multiple = 'SELECT 1; SELECT 2'
        cte = 'WITH x AS (SELECT 1) SELECT * FROM x'
        pragma = 'PRAGMA foreign_keys=OFF'
    }
    foreach ($case in $unsafeSql.GetEnumerator()) {
        Invoke-CliFailure @('db','sqlite-query',$insideDb,[string]$case.Value) "query-deny-$($case.Key)" | Out-Null
    }
    $aliceAfterUnsafe = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT note FROM users WHERE email='alice@example.test'") 'alice-after-unsafe-query'
    if ([string]$aliceAfterUnsafe.rows[0].note -ne 'seed-alice') { throw 'unsafe query path mutated Alice' }

    # AC-05 indexes, relations and statistics.
    $indexes = Invoke-CliJson @('db','sqlite-indexes',$insideDb,'users') 'indexes'
    if (-not (($indexes | ConvertTo-Json -Depth 10 -Compress).Contains('idx_users_name'))) { throw 'deterministic index missing' }
    $relations = Invoke-CliJson @('db','sqlite-relations',$insideDb,'users') 'relations'
    $relationsText = $relations | ConvertTo-Json -Depth 10 -Compress
    if (-not $relationsText.Contains('teams') -or -not $relationsText.Contains('team_id') -or -not $relationsText.Contains('id')) { throw 'deterministic foreign key missing' }
    $stats = Invoke-CliJson @('db','sqlite-stats',$insideDb,'users') 'stats'
    if ([uint64]$stats.row_count -ne 3 -or [uint64]$stats.storage_bytes -eq 0) { throw 'SQLite statistics mismatch' }

    # AC-06 structured insert and negatives.
    $insertReq = [ordered]@{ values = [ordered]@{ team_id = 1; name = 'Bob'; email = 'bob@example.test'; note = 'created' }; filter = [ordered]@{} } | ConvertTo-Json -Depth 8 -Compress
    $insert = Invoke-CliJson @('db','sqlite-insert',$insideDb,'users') 'insert-bob' $insertReq
    if ([uint64]$insert.affected_rows -ne 1) { throw 'insert affected unexpected rows' }
    $bob = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT name,note FROM users WHERE email='bob@example.test'") 'insert-bob-check'
    if ([uint64]$bob.row_count -ne 1 -or [string]$bob.rows[0].name -ne 'Bob') { throw 'inserted row unavailable' }
    $emptyInsert = [ordered]@{ values = [ordered]@{}; filter = [ordered]@{} } | ConvertTo-Json -Depth 8 -Compress
    $emptyInsertFailure = Invoke-CliFailure @('db','sqlite-insert',$insideDb,'users') 'insert-empty-denied' $emptyInsert
    Assert-FailureContains $emptyInsertFailure 'insert requires at least one value' 'empty insert'
    $unsafeEntity = ('e' * 256) -join ''
    $unsafeEntityFailure = Invoke-CliFailure @('db','sqlite-insert',$insideDb,$unsafeEntity) 'insert-unsafe-entity' $insertReq
    Assert-FailureContains $unsafeEntityFailure 'unsafe SQLite identifier' 'unsafe insert entity'
    $unsafeField = ('f' * 256) -join ''
    $unsafeValues = [ordered]@{}
    $unsafeValues[$unsafeField] = 'x'
    $unsafeFieldReq = [ordered]@{ values = $unsafeValues; filter = [ordered]@{} } | ConvertTo-Json -Depth 8 -Compress
    $unsafeFieldFailure = Invoke-CliFailure @('db','sqlite-insert',$insideDb,'users') 'insert-unsafe-field' $unsafeFieldReq
    Assert-FailureContains $unsafeFieldFailure 'unsafe SQLite identifier' 'unsafe insert field'
    $invalidFieldReq = [ordered]@{ values = [ordered]@{ missing_field = 'x' }; filter = [ordered]@{} } | ConvertTo-Json -Depth 8 -Compress
    $invalidFieldFailure = Invoke-CliFailure @('db','sqlite-insert',$insideDb,'users') 'insert-invalid-field' $invalidFieldReq
    Assert-FailureContains $invalidFieldFailure 'no column named missing_field' 'invalid insert field'
    $afterInsertNegatives = Invoke-CliJson @('db','sqlite-query',$insideDb,'SELECT COUNT(*) AS total FROM users') 'insert-negatives-count'
    if ([uint64]$afterInsertNegatives.rows[0].total -ne 4) { throw 'insert negative cases caused unintended mutation' }

    # AC-07 structured update and non-empty filter safety.
    $updateReq = [ordered]@{ values = [ordered]@{ note = 'updated' }; filter = [ordered]@{ email = 'bob@example.test' } } | ConvertTo-Json -Depth 8 -Compress
    $updated = Invoke-CliJson @('db','sqlite-update',$insideDb,'users') 'update-bob' $updateReq
    if ([uint64]$updated.affected_rows -ne 1) { throw 'update affected unexpected rows' }
    $emptyUpdate = [ordered]@{ values = [ordered]@{ note = 'unsafe' }; filter = [ordered]@{} } | ConvertTo-Json -Depth 8 -Compress
    Invoke-CliFailure @('db','sqlite-update',$insideDb,'users') 'update-empty-filter-denied' $emptyUpdate | Out-Null
    $bobUpdated = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT note FROM users WHERE email='bob@example.test'") 'update-bob-check'
    if ([string]$bobUpdated.rows[0].note -ne 'updated') { throw 'Bob update not visible' }
    $aliceAfterUpdate = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT note FROM users WHERE email='alice@example.test'") 'alice-after-update'
    if ([string]$aliceAfterUpdate.rows[0].note -ne 'seed-alice') { throw 'update changed unrelated Alice row' }

    # AC-08 structured delete and non-empty filter safety.
    $deleteReq = [ordered]@{ values = [ordered]@{}; filter = [ordered]@{ email = 'bob@example.test' } } | ConvertTo-Json -Depth 8 -Compress
    $deleted = Invoke-CliJson @('db','sqlite-delete',$insideDb,'users') 'delete-bob' $deleteReq
    if ([uint64]$deleted.affected_rows -ne 1) { throw 'delete affected unexpected rows' }
    $emptyDelete = [ordered]@{ values = [ordered]@{}; filter = [ordered]@{} } | ConvertTo-Json -Depth 8 -Compress
    Invoke-CliFailure @('db','sqlite-delete',$insideDb,'users') 'delete-empty-filter-denied' $emptyDelete | Out-Null
    $bobDeleted = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT id FROM users WHERE email='bob@example.test'") 'delete-bob-check'
    if ([uint64]$bobDeleted.row_count -ne 0) { throw 'Bob still exists after delete' }
    $aliceAfterDelete = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT note FROM users WHERE email='alice@example.test'") 'alice-after-delete'
    if ([string]$aliceAfterDelete.rows[0].note -ne 'seed-alice') { throw 'delete changed unrelated Alice row' }

    # AC-09 every SQLite operation must reject direct outside and junction escape paths.
    Assert-SqlitePathDenied $outsideDb 'outside-direct'
    & cmd.exe /d /c mklink /J $junction $outside *> (Join-Path $script:Root 'junction-create.log')
    Assert-Exit 'junction creation failed'
    $escapedDb = Join-Path $junction 'outside.db'
    Assert-SqlitePathDenied $escapedDb 'outside-junction'
    $outsidePostHash = (Get-FileHash $outsideDb -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($outsidePostHash -ne $outsidePreHash) { throw 'outside database bytes changed during containment negatives' }
    $cleanup.outside_database_unchanged = $true

    # AC-10 oversized cell truncation, aggregate result denial and real CLI payload measurement.
    $large = Invoke-CliJson @('db','sqlite-query',$insideDb,"SELECT note FROM users WHERE name='Large'") 'large-cell'
    $largeCell = $large.rows[0].note
    if ([string]$largeCell.type -ne 'text' -or $largeCell.truncated -ne $true -or [uint64]$largeCell.bytes -ne (300 * 1024)) { throw 'oversized text cell was not explicit truncation metadata' }
    $bulkFour = Invoke-CliJson @('db','sqlite-query',$insideDb,'SELECT payload FROM bulk WHERE id <= 4 ORDER BY id') 'bulk-four-query'
    if ([uint64]$bulkFour.row_count -ne 4) { throw 'bounded successful query row count mismatch' }
    $bulkBrowseFour = Invoke-CliJson @('db','sqlite-browse',$insideDb,'bulk','4','0','id','false') 'bulk-four-browse'
    if (@($bulkBrowseFour.rows).Count -ne 4) { throw 'bounded successful browse row count mismatch' }
    $aggregateQuery = Invoke-CliFailure @('db','sqlite-query',$insideDb,'SELECT payload FROM bulk ORDER BY id') 'bulk-six-query-denied'
    Assert-FailureContains $aggregateQuery '512 KiB serialized JSON safety limit' 'aggregate query'
    $aggregateBrowse = Invoke-CliFailure @('db','sqlite-browse',$insideDb,'bulk','6','0','id','false') 'bulk-six-browse-denied'
    Assert-FailureContains $aggregateBrowse '512 KiB serialized JSON safety limit' 'aggregate browse'
    $sizes = @(
        (Get-Item (Join-Path $script:Root 'large-cell.stdout.json')).Length,
        (Get-Item (Join-Path $script:Root 'bulk-four-query.stdout.json')).Length,
        (Get-Item (Join-Path $script:Root 'bulk-four-browse.stdout.json')).Length
    )
    $maxSuccessfulCliBytes = [uint64](($sizes | Measure-Object -Maximum).Maximum)
    if ($maxSuccessfulCliBytes -ge $MaxIpcBytes) { throw "successful CLI payload exceeded 1 MiB: $maxSuccessfulCliBytes" }

    # AC-11 audit + policy proof and orderly workspace cleanup.
    $audit = Invoke-CliJson @('audit','verify') 'audit-verify'
    $auditEventCount = [uint64]$audit.events
    if ($audit.valid -ne $true -or $auditEventCount -eq 0) { throw 'audit verification failed or empty' }
    Invoke-CliJson @('workspace','remove',$workspace) 'workspace-remove' | Out-Null

    $success = $true
} finally {
    Stop-ProcessSafe $script:Agent
    $cleanup.agent_stopped = Test-ProcessStopped $agentPid

    try {
        if (Test-Path -LiteralPath $junction) {
            & cmd.exe /d /c rmdir $junction *> $null
        }
        $cleanup.junction_removed = -not (Test-Path -LiteralPath $junction)
    } catch {
        $cleanup.junction_removed = $false
    }

    if ($null -eq $outsidePostHash -and (Test-Path -LiteralPath $outsideDb -PathType Leaf)) {
        try { $outsidePostHash = (Get-FileHash $outsideDb -Algorithm SHA256).Hash.ToLowerInvariant() } catch {}
    }
    if ($null -ne $outsidePreHash -and $outsidePostHash -eq $outsidePreHash) {
        $cleanup.outside_database_unchanged = $true
    }

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
    $cleanup | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $script:Root 'cleanup.json') -Encoding utf8
}

if (-not $success) { throw '02.25 certification did not reach acceptance evidence' }
foreach ($entry in $cleanup.GetEnumerator()) {
    if ($entry.Value -ne $true) { throw "cleanup invariant failed: $($entry.Key)" }
}

$evidence = [ordered]@{
    schema_version = 1
    feature_id = $FeatureId
    feature_version = $FeatureVersion
    package_id = 'PKG-02'
    task_id = '02.25'
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
    checks = [ordered]@{
        ac01_exact_source_toolchain = $true
        ac02_inspect = $true
        ac03_browse = $true
        ac04_safe_query = $true
        ac05_indexes_relations_statistics = $true
        ac06_structured_insert = $true
        ac07_structured_update = $true
        ac08_structured_delete = $true
        ac09_workspace_containment = $true
        ac10_frame_resource_safety = $true
        ac11_policy_audit_cleanup = $true
        ac12_evidence_integrity = $true
    }
    measurements = [ordered]@{
        agent_readiness_ms = $script:AgentReadyMs
        audit_events = $auditEventCount
        provider_result_limit_bytes = $MaxReadResultBytes
        text_cell_limit_bytes = $MaxTextCellBytes
        ipc_frame_limit_bytes = $MaxIpcBytes
        max_successful_cli_payload_bytes = $maxSuccessfulCliBytes
        outside_pre_sha256 = $outsidePreHash
        outside_post_sha256 = $outsidePostHash
    }
    artifacts = [ordered]@{
        vsn_agent_sha256 = $agentSha256
        vsn_cli_sha256 = $cliSha256
        fixture_sha256 = $fixtureSha256
        cleanup = 'cleanup.json'
        cargo_fmt = 'cargo-fmt.log'
        cargo_clippy = 'cargo-clippy.log'
        cargo_test = 'cargo-test.log'
        cargo_build = 'cargo-build.log'
        cargo_agent_cli_build = 'cargo-agent-cli-build.log'
        git_diff_check = 'git-diff-check.log'
    }
}
$evidencePath = Join-Path $script:Root 'evidence.json'
$evidence | ConvertTo-Json -Depth 8 | Set-Content $evidencePath -Encoding utf8
$evidenceSha256 = (Get-FileHash $evidencePath -Algorithm SHA256).Hash.ToLowerInvariant()
$evidenceSha256 | Set-Content (Join-Path $script:Root 'evidence.json.sha256') -Encoding ascii
Write-Host "02.25 acceptance evidence complete source=$($evidence.source_commit) evidence_sha256=$evidenceSha256"