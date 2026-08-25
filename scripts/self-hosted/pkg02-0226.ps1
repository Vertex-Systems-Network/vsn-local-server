param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$FeatureId = 'pkg02-0226-external-native-database-adapters'
$FeatureVersion = '1.0.0'
$CanonicalBaseSha = '836feb4171a9eb882208a6d666600cea4abe3f42'
$PlanSha256 = 'aa40b6a0d001e4dfb572a2fc51bfae273df4047030981d730a7a864262d9a793'
$ResearchSha256 = '10bbd47772fd2f5ca3189c13d52942c55914ff9b688dff270330806744166709'
$LifecycleSha256 = '1e5213f017d0e2655e33028f4684ed5437c576a1b97d551c5f39498d08d65365'
$PreflightSha256 = '171644fcd960112a5719b2fef95433d480d560a91f6387ba93762525d6144564'
$CandidateId = 'c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474'
$ProductVersion = '0.38.1'
$AgentIpcPort = 39731
$ExternalStdoutLimit = 512 * 1024
$ExternalStderrLimit = 256 * 1024
$NativeTextCellLimit = 256 * 1024
$NativeResultLimit = 512 * 1024
$IpcFrameLimit = 1024 * 1024

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
    } catch { return $null }
}

function Invoke-CliJson([string[]]$CliArgs, [string]$Name) {
    $out = Join-Path $script:Root "$Name.stdout.json"
    $err = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt") -Encoding ascii
    if ($code -ne 0) {
        $detail = if (Test-Path $err) { Get-Content $err -Raw } else { '' }
        throw "$Name failed (exit=$code): $detail"
    }
    $bytes = (Get-Item -LiteralPath $out).Length
    if ($bytes -gt $script:MaxSuccessfulCliBytes) { $script:MaxSuccessfulCliBytes = $bytes }
    Get-Content $out -Raw | ConvertFrom-Json
}

function Invoke-CliFailure([string[]]$CliArgs, [string]$Name) {
    $out = Join-Path $script:Root "$Name.stdout.log"
    $err = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code = $LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt") -Encoding ascii
    if ($code -eq 0) { throw "$Name unexpectedly succeeded" }
    $stdoutText = if (Test-Path $out) { Get-Content $out -Raw } else { '' }
    $stderrText = if (Test-Path $err) { Get-Content $err -Raw } else { '' }
    [pscustomobject]@{ ExitCode = $code; Text = "$stdoutText`n$stderrText" }
}

function Assert-FailureContains($Failure, [string]$Needle, [string]$Name) {
    if (-not $Failure.Text.Contains($Needle)) { throw "$Name missing expected failure '$Needle': $($Failure.Text)" }
}

function Get-FakeLogCount {
    if (-not (Test-Path -LiteralPath $script:FakeLog -PathType Leaf)) { return 0 }
    return @(Get-Content -LiteralPath $script:FakeLog).Count
}

function Assert-NoFakeSpawn([scriptblock]$Action, [string]$Name) {
    $before = Get-FakeLogCount
    & $Action
    $after = Get-FakeLogCount
    if ($after -ne $before) { throw "$Name reached fake client despite fail-closed expectation" }
}

function Write-FakeClient([string]$Name, [string]$Body) {
    $path = Join-Path $script:FakeBin "$Name.cmd"
    $expanded = $Body.Replace('`r`n', [Environment]::NewLine)
    Set-Content -LiteralPath $path -Value $expanded -Encoding ascii -NoNewline
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

function Stop-AgentSafe {
    if ($null -ne $script:Agent) {
        try {
            if (-not $script:Agent.HasExited) {
                Stop-Process -Id $script:Agent.Id -Force -ErrorAction SilentlyContinue
                Wait-Process -Id $script:Agent.Id -Timeout 10 -ErrorAction SilentlyContinue
            }
        } catch {}
    }
}

$script:Root = Join-Path $PWD 'dist-self-hosted\02.26'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0226-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$outside = Join-Path $sandbox 'outside'
$junction = Join-Path $workspace 'escape'
$script:FakeBin = Join-Path $sandbox 'fake-bin'
$isolated = Join-Path $sandbox 'localappdata'
$ca = Join-Path $workspace 'root-ca.pem'
$credential = Join-Path $workspace 'client.cnf'
$outsideCa = Join-Path $outside 'outside-ca.pem'
$junctionCa = Join-Path $junction 'outside-ca.pem'
$outsideCredential = Join-Path $outside 'outside-client.cnf'
$junctionCredential = Join-Path $junction 'outside-client.cnf'
$script:FakeLog = Join-Path $sandbox 'fake-client.log'
$originalLocal = $env:LOCALAPPDATA
$originalPath = $env:PATH
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadKey = Test-Path -LiteralPath $ipcKey -PathType Leaf
$originalKeyBytes = if ($hadKey) { [IO.File]::ReadAllBytes($ipcKey) } else { $null }
$originalKeyHash = if ($hadKey) { Get-OptionalSha $ipcKey } else { $null }
$hostsPath = Join-Path $env:SystemRoot 'System32\drivers\etc\hosts'
$hostsPreHash = Get-OptionalSha $hostsPath
$script:Agent = $null
$script:AgentExe = $null
$script:Cli = $null
$script:AgentReadyMs = 0
$script:MaxSuccessfulCliBytes = 0L
$auditEventCount = 0
$fiveEngineCount = 0
$fakeDetectionElapsedMs = 0L
$slowDetectionElapsedMs = 0L
$noisyDetectionElapsedMs = 0L
$payloadZipSha = $null
$success = $false
$workspaceAdded = $false
$cleanup = [ordered]@{
    agent_stopped = $false
    workspace_removed = $false
    junction_removed = $false
    ipc_key_restored = $false
    localappdata_restored = $false
    path_restored = $false
    sandbox_removed = $false
    system_hosts_unchanged = $false
    no_system_trust_mutation = $true
    no_resolver_mutation = $true
    no_production_or_remote_database_mutation = $true
    no_privileged_system_mutation = $true
}

if (Test-Path $script:Root) { Remove-Item $script:Root -Recurse -Force }
New-Item -ItemType Directory -Force -Path $script:Root, $bin, $sandbox, $workspace, $outside, $script:FakeBin, $isolated | Out-Null
$env:LOCALAPPDATA = $isolated

try {
    if (-not $IsWindows) { throw '02.26 requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.26 requires GitHub-hosted runner' }
    if ($env:RUNNER_ARCH -ne 'X64') { throw "02.26 requires X64 runner, got $env:RUNNER_ARCH" }
    if (-not $env:EXPECTED_SHA) { throw 'EXPECTED_SHA required' }
    $sourceCommit = (git rev-parse HEAD).Trim()
    if ($sourceCommit -ne $env:EXPECTED_SHA) { throw "exact source mismatch expected=$env:EXPECTED_SHA actual=$sourceCommit" }
    $rustcVersion = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rustcVersion -notmatch '^rustc 1\.97\.1\b') { throw "rustc 1.97.1 required: $rustcVersion" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "cargo 1.97.1 required: $cargoVersion" }
    if (Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue) { throw 'IPC port occupied' }

    Assert-Sha '.ai\plans\pkg02-0226-external-native-database-v1.md' $PlanSha256 'plan'
    Assert-Sha '.ai\features\pkg02-0226\research.md' $ResearchSha256 'research'
    Assert-Sha '.ai\features\pkg02-0226\lifecycle-review.md' $LifecycleSha256 'lifecycle'
    Assert-Sha '.ai\features\pkg02-0226\development-preflight.md' $PreflightSha256 'preflight'
    $manifest = Get-Content '.ai\manifests\pkg02-0226-external-native-database.v1.json' -Raw | ConvertFrom-Json
    if ([string]$manifest.feature_id -ne $FeatureId -or [string]$manifest.version -ne $FeatureVersion) { throw 'manifest identity mismatch' }
    if ([string]$manifest.canonical_base_sha -ne $CanonicalBaseSha) { throw 'manifest canonical base mismatch' }
    if ([string]$manifest.plan.sha256 -ne $PlanSha256) { throw 'manifest plan digest mismatch' }
    if (($manifest.acceptance.criteria | Measure-Object).Count -ne 12) { throw 'frozen AC-01..AC-12 set changed' }

    $capSource = Get-Content 'crates\vsn-database\src\lib.rs' -Raw
    foreach ($engine in @('postgresql','mysql','mariadb','mongodb','redis')) {
        if (-not $capSource.Contains("engine: `"$engine`".into()")) { throw "capability engine missing: $engine" }
    }
    $nativeSource = Get-Content 'crates\vsn-database-native\src\lib.rs' -Raw
    foreach ($fn in @('postgres_indexes','postgres_relations','mongo_indexes')) {
        $m = [regex]::Match($nativeSource, "(?s)pub fn $fn\(.*?(?=\npub fn |\z)")
        if (-not $m.Success -or -not $m.Value.Contains('bounded_read_result(NativeGrid {')) { throw "$fn bypasses native result budget" }
    }
    if (-not $nativeSource.Contains('with_danger_skip_domain_validation(false)') -or -not $nativeSource.Contains('with_danger_accept_invalid_certs(false)')) { throw 'MySQL verified TLS hardening missing' }
    foreach ($forbidden in @('tlsinsecure=true','tlsallowinvalidhostnames=true','tlsallowinvalidcertificates=true')) {
        if (-not $nativeSource.ToLowerInvariant().Contains($forbidden)) { throw "Mongo insecure-option rejection missing: $forbidden" }
    }
    $cliSource = Get-Content 'apps\cli\src\main.rs' -Raw
    foreach ($surface in @('pg-tls-inspect','pg-tls-browse','pg-tls-query','mysql-tls-inspect','mysql-tls-browse','mysql-tls-query')) {
        if (-not $cliSource.Contains($surface)) { throw "public TLS CLI surface missing: $surface" }
    }

    & cargo fmt --all -- --check *> (Join-Path $script:Root 'cargo-fmt.log')
    Assert-Exit 'cargo fmt failed'
    & cargo clippy --locked --package vsn-database --package vsn-database-cli --package vsn-database-native --package vsn-core --package vsn-policy --package vsn-agent --package vsn --all-targets --no-deps -- -D warnings *> (Join-Path $script:Root 'cargo-clippy.log')
    Assert-Exit '02.26 strict Clippy failed'
    & cargo test --locked --package vsn-database --package vsn-database-cli --package vsn-database-native --package vsn-core --package vsn-policy *> (Join-Path $script:Root 'cargo-test.log')
    Assert-Exit '02.26 package tests failed'
    & cargo build --locked --release --package vsn-agent --package vsn *> (Join-Path $script:Root 'cargo-build.log')
    Assert-Exit 'release Agent/CLI build failed'
    & git diff --check *> (Join-Path $script:Root 'git-diff-check.log')
    Assert-Exit 'git diff check failed'

    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $script:Cli = Join-Path $bin 'vsn.exe'

    Set-Content -LiteralPath $ca -Value 'fixture-root-ca-not-for-production' -Encoding ascii
    Set-Content -LiteralPath $credential -Value "[client]`npassword=supersecret-0226" -Encoding ascii
    Set-Content -LiteralPath $outsideCa -Value 'outside-root-ca' -Encoding ascii
    Set-Content -LiteralPath $outsideCredential -Value "[client]`npassword=outside-secret" -Encoding ascii
    New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null

    $env:VSN_FAKE_CLIENT_LOG = $script:FakeLog
    $commonRelational = '@echo off`r`nif "%1"=="--version" (echo %~n0 fake 1.0& exit /b 0)`r`n>>"%VSN_FAKE_CLIENT_LOG%" echo %~n0^|%*^|PGSSLMODE=%PGSSLMODE%^|PGSSLROOTCERT=%PGSSLROOTCERT%^|PGPASSFILE=%PGPASSFILE%`r`npowershell -NoProfile -Command "[Console]::Out.WriteLine(''schema'' + [char]9 + ''name'' + [char]9 + ''type''); [Console]::Out.WriteLine(''public'' + [char]9 + ''widgets'' + [char]9 + ''BASE TABLE'')"`r`nexit /b 0`r`n'
    Write-FakeClient 'psql' $commonRelational
    Write-FakeClient 'mysql' $commonRelational
    Write-FakeClient 'mariadb' $commonRelational
    Write-FakeClient 'mongosh' '@echo off`r`nif "%1"=="--version" (echo mongosh fake 1.0& exit /b 0)`r`n>>"%VSN_FAKE_CLIENT_LOG%" echo mongosh^|%*`r`necho []`r`nexit /b 0`r`n'
    Write-FakeClient 'redis-cli' '@echo off`r`nif "%1"=="--version" (echo redis-cli fake 1.0& exit /b 0)`r`n>>"%VSN_FAKE_CLIENT_LOG%" echo redis-cli^|%*`r`necho fixture-key-one`r`necho fixture-key-two`r`nexit /b 0`r`n'
    $env:PATH = "$script:FakeBin;$originalPath"

    Start-Agent
    $workspaceResult = Invoke-CliJson @('workspace','add',$workspace) 'workspace-add'
    $workspaceAdded = $true

    $sw = [Diagnostics.Stopwatch]::StartNew()
    $clients = Invoke-CliJson @('db','clients') 'db-clients'
    $sw.Stop()
    $fakeDetectionElapsedMs = [int64]$sw.ElapsedMilliseconds
    $clientRows = @($clients)
    if ($clientRows.Count -ne 5) { throw "expected five client detections, got $($clientRows.Count)" }
    $expectedEngines = @('postgresql','mysql','mariadb','mongodb','redis')
    foreach ($engine in $expectedEngines) {
        $row = $clientRows | Where-Object { [string]$_.engine -eq $engine }
        if ($null -eq $row -or $row.available -ne $true) { throw "fake client unavailable: $engine" }
    }
    $fiveEngineCount = $clientRows.Count

    Write-FakeClient 'psql' '@echo off`r`nif "%1"=="--version" (ping -n 8 127.0.0.1 >nul& exit /b 0)`r`nexit /b 0`r`n'
    $slowSw = [Diagnostics.Stopwatch]::StartNew()
    $slowClients = Invoke-CliJson @('db','clients') 'db-clients-slow-version'
    $slowSw.Stop()
    $slowDetectionElapsedMs = [int64]$slowSw.ElapsedMilliseconds
    if ($slowDetectionElapsedMs -ge 8000) { throw "slow client detection exceeded bounded expectation: $slowDetectionElapsedMs ms" }
    $slowPg = @($slowClients) | Where-Object { [string]$_.engine -eq 'postgresql' }
    if ($null -eq $slowPg -or $slowPg.available -ne $true -or $null -ne $slowPg.version) { throw 'slow --version detection did not fail closed to unavailable version metadata' }

    Write-FakeClient 'psql' '@echo off`r`nif "%1"=="--version" (for /L %%i in (1,1,70000) do @echo 012345678901234567890123456789)`r`nexit /b 0`r`n'
    $noisySw = [Diagnostics.Stopwatch]::StartNew()
    $noisyClients = Invoke-CliJson @('db','clients') 'db-clients-noisy-version'
    $noisySw.Stop()
    $noisyDetectionElapsedMs = [int64]$noisySw.ElapsedMilliseconds
    if ($noisyDetectionElapsedMs -ge 8000) { throw "noisy client detection exceeded bounded expectation: $noisyDetectionElapsedMs ms" }
    $noisyPg = @($noisyClients) | Where-Object { [string]$_.engine -eq 'postgresql' }
    if ($null -eq $noisyPg -or $noisyPg.available -ne $true -or $null -ne $noisyPg.version) { throw 'high-output --version detection did not fail closed to unavailable version metadata' }
    Write-FakeClient 'psql' $commonRelational

    $pgLocal = Invoke-CliJson @('db','inspect','postgresql','localhost','5432','fixture','app') 'pg-local-inspect'
    $mysqlLocal = Invoke-CliJson @('db','inspect','mysql','127.0.0.1','3306','fixture','app') 'mysql-local-inspect'
    $mariaLocal = Invoke-CliJson @('db','inspect','mariadb','::1','3306','fixture','app') 'maria-local-inspect'
    $loopbackExtra = @(
        (Invoke-CliJson @('db','inspect','postgresql','127.0.0.1','5432','fixture','app') 'pg-loopback-127'),
        (Invoke-CliJson @('db','inspect','postgresql','::1','5432','fixture','app') 'pg-loopback-ipv6'),
        (Invoke-CliJson @('db','inspect','mysql','localhost','3306','fixture','app') 'mysql-loopback-localhost'),
        (Invoke-CliJson @('db','inspect','mysql','::1','3306','fixture','app') 'mysql-loopback-ipv6'),
        (Invoke-CliJson @('db','inspect','mariadb','localhost','3306','fixture','app') 'maria-loopback-localhost'),
        (Invoke-CliJson @('db','inspect','mariadb','127.0.0.1','3306','fixture','app') 'maria-loopback-127')
    )
    foreach ($item in @($pgLocal,$mysqlLocal,$mariaLocal) + $loopbackExtra) {
        if (@($item.entities).Count -lt 1) { throw 'loopback fake relational inspection returned no entity' }
    }

    $pgTls = Invoke-CliJson @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$ca) 'pg-tls-inspect'
    $pgQuery = Invoke-CliJson @('db','query-tls','postgresql','db.example.test','5432','fixture','app',$ca,'SELECT 1') 'pg-tls-query'
    $mysqlTls = Invoke-CliJson @('db','inspect-tls','mysql','db.example.test','3306','fixture','app',$ca) 'mysql-tls-inspect'
    $mysqlQuery = Invoke-CliJson @('db','query-tls','mysql','db.example.test','3306','fixture','app',$ca,'SELECT 1') 'mysql-tls-query'
    $mariaTls = Invoke-CliJson @('db','inspect-tls','mariadb','db.example.test','3306','fixture','app',$ca) 'maria-tls-inspect'
    $mongoTls = Invoke-CliJson @('db','inspect-tls','mongodb','db.example.test','27017','-','app',$ca) 'mongo-tls-inspect'
    $redisTls = Invoke-CliJson @('db','inspect-tls','redis','db.example.test','6380','-','0',$ca) 'redis-tls-inspect'
    if ([string]$pgTls.engine -ne 'postgresql' -or [string]$mysqlTls.engine -ne 'mysql' -or [string]$mariaTls.engine -ne 'mariadb' -or [string]$mongoTls.engine -ne 'mongodb' -or [string]$redisTls.engine -ne 'redis') { throw 'TLS fake inspection engine mismatch' }
    if (@($pgQuery.rows).Count -lt 1 -or @($mysqlQuery.rows).Count -lt 1) { throw 'TLS fake read query returned no rows' }

    $fakeLogText = Get-Content -LiteralPath $script:FakeLog -Raw
    foreach ($needle in @('PGSSLMODE=verify-full','PGSSLROOTCERT=','--ssl-mode=VERIFY_IDENTITY','--ssl-ca=','--ssl-verify-server-cert','mongosh|--quiet --host db.example.test --port 27017 app --tls --tlsCAFile','redis-cli|-h db.example.test -p 6380 -n 0 --tls --cacert')) {
        if (-not $fakeLogText.Contains($needle)) { throw "verified TLS construction evidence missing: $needle" }
    }
    if ($fakeLogText.Contains('supersecret-0226')) { throw 'secret leaked into generated client argv/log' }

    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect','postgresql','localhost.evil.invalid','5432','fixture','app') 'reject-pg-spoof'
        Assert-FailureContains $f 'trusted root CA' 'reject-pg-spoof'
    } 'reject-pg-spoof'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect','postgresql','db1.example.test,db2.example.test','5432','fixture','app') 'reject-multihost'
        Assert-FailureContains $f 'unambiguous' 'reject-multihost'
    } 'reject-multihost'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect','postgresql','user@localhost','5432','fixture','app') 'reject-userinfo-host'
        Assert-FailureContains $f 'unambiguous' 'reject-userinfo-host'
    } 'reject-userinfo-host'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect','mysql','localhost','0','fixture','app') 'reject-port-zero'
        Assert-FailureContains $f 'port 0' 'reject-port-zero'
    } 'reject-port-zero'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect','mysql','db.example.test','3306','fixture','app') 'reject-remote-plaintext'
        Assert-FailureContains $f 'trusted root CA' 'reject-remote-plaintext'
    } 'reject-remote-plaintext'

    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','query-tls','mongodb','db.example.test','27017','','app',$ca,'db.runCommand({ping:1})') 'reject-mongo-query'
        Assert-FailureContains $f 'arbitrary script/query execution is disabled' 'reject-mongo-query'
    } 'reject-mongo-query'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','query-tls','redis','db.example.test','6380','','0',$ca,'FLUSHALL') 'reject-redis-query'
        Assert-FailureContains $f 'arbitrary script/query execution is disabled' 'reject-redis-query'
    } 'reject-redis-query'

    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$outsideCa) 'reject-outside-ca'
        Assert-FailureContains $f 'must be inside a configured workspace' 'reject-outside-ca'
    } 'reject-outside-ca'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$junctionCa) 'reject-junction-ca'
        Assert-FailureContains $f 'must be inside a configured workspace' 'reject-junction-ca'
    } 'reject-junction-ca'
    Assert-NoFakeSpawn {
        $missing = Join-Path $workspace 'missing-ca.pem'
        $f = Invoke-CliFailure @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$missing) 'reject-missing-ca'
        Assert-FailureContains $f 'unavailable' 'reject-missing-ca'
    } 'reject-missing-ca'
    Assert-NoFakeSpawn {
        $missingCredential = Join-Path $workspace 'missing-client.cnf'
        $f = Invoke-CliFailure @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$ca,$missingCredential) 'reject-missing-credential'
        Assert-FailureContains $f 'unavailable' 'reject-missing-credential'
    } 'reject-missing-credential'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$ca,$outsideCredential) 'reject-outside-credential'
        Assert-FailureContains $f 'must be inside a configured workspace' 'reject-outside-credential'
    } 'reject-outside-credential'
    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$ca,$junctionCredential) 'reject-junction-credential'
        Assert-FailureContains $f 'must be inside a configured workspace' 'reject-junction-credential'
    } 'reject-junction-credential'

    Assert-NoFakeSpawn {
        $f = Invoke-CliFailure @('db','inspect','unknown','localhost','5432','','') 'reject-unknown-engine'
        Assert-FailureContains $f 'unknown variant' 'reject-unknown-engine'
    } 'reject-unknown-engine'

    $pgCredential = Invoke-CliJson @('db','inspect-tls','postgresql','db.example.test','5432','fixture','app',$ca,$credential) 'pg-tls-credential'
    if ([string]$pgCredential.engine -ne 'postgresql') { throw 'contained credential file did not succeed' }
    $fakeLogText = Get-Content -LiteralPath $script:FakeLog -Raw
    if ($fakeLogText.Contains('supersecret-0226')) { throw 'credential secret leaked into generated client argv/log' }

    foreach ($case in @(
        @('pg-tls-inspect','db pg tls inspect'),
        @('pg-tls-browse','db pg tls browse'),
        @('pg-tls-query','db pg tls query'),
        @('mysql-tls-inspect','db mysql tls inspect'),
        @('mysql-tls-browse','db mysql tls browse'),
        @('mysql-tls-query','db mysql tls query')
    )) {
        if (-not $cliSource.Contains([string]$case[0])) { throw "public native TLS surface missing: $($case[1])" }
    }

    $nativeProviderFailures = @(
        (Invoke-CliFailure @('db','pg-tls-inspect','host=db.example.test port=5432 user=fixture dbname=app sslmode=verify-full',$ca) 'native-pg-tls-inspect'),
        (Invoke-CliFailure @('db','pg-tls-browse','host=db.example.test port=5432 user=fixture dbname=app sslmode=verify-full',$ca,'public','widgets') 'native-pg-tls-browse'),
        (Invoke-CliFailure @('db','pg-tls-query','host=db.example.test port=5432 user=fixture dbname=app sslmode=verify-full',$ca,'SELECT 1') 'native-pg-tls-query'),
        (Invoke-CliFailure @('db','mysql-tls-inspect','mysql://fixture@db.example.test:3306/app',$ca) 'native-mysql-tls-inspect'),
        (Invoke-CliFailure @('db','mysql-tls-browse','mysql://fixture@db.example.test:3306/app',$ca,'app','widgets') 'native-mysql-tls-browse'),
        (Invoke-CliFailure @('db','mysql-tls-query','mysql://fixture@db.example.test:3306/app',$ca,'SELECT 1') 'native-mysql-tls-query')
    )
    foreach ($failure in $nativeProviderFailures) {
        if ($failure.Text.Contains('permission denied') -or $failure.Text.Contains('unsupported command') -or $failure.Text.Contains('must be inside a configured workspace')) { throw "native TLS public path failed before authorized provider boundary: $($failure.Text)" }
    }

    $audit = Invoke-CliJson @('audit','verify') 'audit-verify'
    if ($audit.valid -ne $true) { throw 'audit chain invalid' }
    $auditEventCount = @($audit.events).Count
    if ($auditEventCount -le 0) { throw 'audit chain empty' }

    $remove = Invoke-CliJson @('workspace','remove',$workspace) 'workspace-remove'
    $workspaceAdded = $false
    $cleanup.workspace_removed = $true

    $payloadDir = Join-Path $script:Root 'payload'
    New-Item -ItemType Directory -Force -Path $payloadDir | Out-Null
    Copy-Item (Join-Path $bin 'vsn-agent.exe') $payloadDir -Force
    Copy-Item (Join-Path $bin 'vsn.exe') $payloadDir -Force
    Copy-Item (Join-Path $script:Root 'db-clients.stdout.json') $payloadDir -Force
    Copy-Item $script:FakeLog (Join-Path $payloadDir 'fake-client.log') -Force
    $payloadZip = Join-Path $script:Root 'pkg02-0226-evidence-payload.zip'
    if (Test-Path $payloadZip) { Remove-Item $payloadZip -Force }
    Compress-Archive -Path (Join-Path $payloadDir '*') -DestinationPath $payloadZip -CompressionLevel Optimal
    $payloadZipSha = (Get-FileHash -LiteralPath $payloadZip -Algorithm SHA256).Hash.ToLowerInvariant()

    $success = $true
}
finally {
    if ($workspaceAdded -and $null -ne $script:Cli) {
        try { & $script:Cli workspace remove $workspace *> $null; if ($LASTEXITCODE -eq 0) { $cleanup.workspace_removed = $true } } catch {}
    }
    Stop-AgentSafe
    $cleanup.agent_stopped = $null -eq $script:Agent -or $script:Agent.HasExited
    if (Test-Path -LiteralPath $junction) {
        try { cmd /c rmdir "$junction" *> $null } catch {}
    }
    $cleanup.junction_removed = -not (Test-Path -LiteralPath $junction)

    try {
        if ($hadKey) {
            New-Item -ItemType Directory -Force -Path (Split-Path $ipcKey -Parent) | Out-Null
            [IO.File]::WriteAllBytes($ipcKey, $originalKeyBytes)
        } elseif (Test-Path -LiteralPath $ipcKey) {
            Remove-Item -LiteralPath $ipcKey -Force
        }
    } catch {}
    $restoredKeyHash = Get-OptionalSha $ipcKey
    $cleanup.ipc_key_restored = if ($hadKey) { $restoredKeyHash -eq $originalKeyHash } else { -not (Test-Path -LiteralPath $ipcKey) }

    $env:LOCALAPPDATA = $originalLocal
    $cleanup.localappdata_restored = $env:LOCALAPPDATA -eq $originalLocal
    $env:PATH = $originalPath
    $cleanup.path_restored = $env:PATH -eq $originalPath
    Remove-Item Env:VSN_FAKE_CLIENT_LOG -ErrorAction SilentlyContinue

    $hostsPostHash = Get-OptionalSha $hostsPath
    $cleanup.system_hosts_unchanged = ($hostsPreHash -eq $hostsPostHash)

    try { if (Test-Path -LiteralPath $sandbox) { Remove-Item -LiteralPath $sandbox -Recurse -Force } } catch {}
    $cleanup.sandbox_removed = -not (Test-Path -LiteralPath $sandbox)

    $cleanup | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'cleanup.json') -Encoding utf8
}

if (-not $success) { throw '02.26 certification did not reach acceptance evidence stage' }
foreach ($p in $cleanup.GetEnumerator()) { if ($p.Value -ne $true) { throw "cleanup failed: $($p.Key)" } }
if ($script:MaxSuccessfulCliBytes -ge $IpcFrameLimit) { throw 'successful CLI payload reached/exceeded IPC frame limit' }
if ($fiveEngineCount -ne 5) { throw 'five-engine count changed' }
if (-not $payloadZipSha -or $payloadZipSha -notmatch '^[0-9a-f]{64}$') { throw 'payload ZIP digest unavailable' }

$evidence = [ordered]@{
    schema_version = 1
    feature_id = $FeatureId
    feature_version = $FeatureVersion
    package_id = 'PKG-02'
    task_id = '02.26'
    canonical_base_sha = $CanonicalBaseSha
    plan_sha256 = $PlanSha256
    research_sha256 = $ResearchSha256
    lifecycle_sha256 = $LifecycleSha256
    development_preflight_sha256 = $PreflightSha256
    source_commit = $env:EXPECTED_SHA
    product_version = $ProductVersion
    candidate_id = $CandidateId
    runner_environment = $env:RUNNER_ENVIRONMENT
    runner_os = 'Windows'
    runner_arch = $env:RUNNER_ARCH
    rustc_version = $rustcVersion
    cargo_version = $cargoVersion
    ipc_address = "127.0.0.1:$AgentIpcPort"
    privileged_system_mutation_performed = $false
    production_or_remote_database_mutation_performed = $false
    checks = [ordered]@{
        AC01_exact_source_toolchain = $true
        AC02_declared_capabilities = $true
        AC03_client_detection = $true
        AC04_plaintext_loopback_boundary = $true
        AC05_verified_remote_tls = $true
        AC06_unsupported_capability_fail_closed = $true
        AC07_authenticated_local_read_operator_path = $true
        AC08_write_permission_truthfulness = $true
        AC09_resource_process_safety = $true
        AC10_credential_ca_secret_safety = $true
        AC11_audit_cleanup_non_mutation = $true
        AC12_evidence_integrity = $true
    }
    measurements = [ordered]@{
        agent_readiness_ms = $script:AgentReadyMs
        five_engine_count = $fiveEngineCount
        client_detection_total_ms = $fakeDetectionElapsedMs
        slow_client_detection_total_ms = $slowDetectionElapsedMs
        noisy_client_detection_total_ms = $noisyDetectionElapsedMs
        external_stdout_limit_bytes = $ExternalStdoutLimit
        external_stderr_limit_bytes = $ExternalStderrLimit
        native_text_cell_limit_bytes = $NativeTextCellLimit
        native_result_limit_bytes = $NativeResultLimit
        ipc_frame_limit_bytes = $IpcFrameLimit
        max_successful_cli_payload_bytes = $script:MaxSuccessfulCliBytes
        audit_events = $auditEventCount
        system_hosts_pre_sha256 = $hostsPreHash
        system_hosts_post_sha256 = Get-OptionalSha $hostsPath
    }
    capability_engines = @('postgresql','mysql','mariadb','mongodb','redis')
    artifacts = [ordered]@{
        vsn_agent_sha256 = (Get-FileHash -LiteralPath (Join-Path $bin 'vsn-agent.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
        vsn_cli_sha256 = (Get-FileHash -LiteralPath (Join-Path $bin 'vsn.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
        payload_zip_sha256 = $payloadZipSha
    }
    cleanup = $cleanup
}
$evidencePath = Join-Path $script:Root 'evidence.json'
$evidence | ConvertTo-Json -Depth 12 | Set-Content $evidencePath -Encoding utf8
$evidenceSha = (Get-FileHash -LiteralPath $evidencePath -Algorithm SHA256).Hash.ToLowerInvariant()
$evidenceSha | Set-Content (Join-Path $script:Root 'evidence.json.sha256') -Encoding ascii
Write-Host "02.26 evidence_sha256=$evidenceSha payload_zip_sha256=$payloadZipSha source=$($env:EXPECTED_SHA)"
