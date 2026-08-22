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

$script:Root = Join-Path $PWD 'dist-self-hosted\02.15'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0215-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$modeFile = Join-Path $sandbox 'container-mode.txt'
$originalLocalAppData = $env:LOCALAPPDATA
$originalPath = $env:PATH
$originalModeFile = $env:VSN_PKG02_0215_MODE_FILE
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$script:Agent = $null

New-Item -ItemType Directory -Force -Path $script:Root,$bin,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

try {
    if (-not $IsWindows) { throw "02.15 acceptance requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $script:Root 'runner.txt')

    $containerSource = Get-Content 'crates/vsn-container/src/lib.rs' -Raw
    foreach ($needle in @(
        'BACKEND_VERSION_TIMEOUT',
        'BACKEND_INFO_TIMEOUT',
        'BASELINE_READ_TIMEOUT',
        'BASELINE_ACTION_TIMEOUT',
        'BACKEND_PROBE_OUTPUT_BYTES',
        'BASELINE_LIST_OUTPUT_BYTES',
        'BASELINE_LOG_OUTPUT_BYTES',
        'vsn-container-detect-docker',
        'vsn-container-detect-podman',
        'run_bounded(',
        'container command timed out after',
        'container command output exceeded safety limit'
    )) {
        if (-not $containerSource.Contains($needle)) { throw "missing 02.15 container source invariant: $needle" }
    }
    if ($containerSource.Contains('.output()')) { throw 'unbounded Command::output remains in vsn-container baseline' }

    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('Permission::RuntimeView','Permission::RuntimeManage')) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.15 Core permission invariant: $needle" }
    }
    $fixtureSource = Get-Content 'crates/vsn-container/examples/pkg02_container_fixture.rs' -Raw
    foreach ($needle in @('daemon-down','hang','flood','Docker version 99.0.0-vsn-fixture')) {
        if (-not $fixtureSource.Contains($needle)) { throw "missing 02.15 fixture mode: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-container --package vsn-core --package vsn-agent --package vsn --all-targets -- -D warnings
    Assert-LastExit 'container/core/agent/cli clippy failed'
    cargo test --locked --package vsn-container --package vsn-core --package vsn-agent --package vsn
    Assert-LastExit 'container/core/agent/cli tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    cargo build --locked --release --package vsn-container --example pkg02_container_fixture
    Assert-LastExit 'container backend fixture build failed'

    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    Copy-Item 'target\release\examples\pkg02_container_fixture.exe' (Join-Path $bin 'docker.exe') -Force
    Copy-Item 'target\release\examples\pkg02_container_fixture.exe' (Join-Path $bin 'podman.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    'reachable' | Set-Content -LiteralPath $modeFile -Encoding ascii
    $env:VSN_PKG02_0215_MODE_FILE = $modeFile
    $env:PATH = "$bin;$originalPath"
    if ((Get-Command docker.exe -ErrorAction Stop).Source -ne (Join-Path $bin 'docker.exe')) { throw 'fake docker fixture is not first on PATH' }
    if ((Get-Command podman.exe -ErrorAction Stop).Source -ne (Join-Path $bin 'podman.exe')) { throw 'fake podman fixture is not first on PATH' }

    Start-Agent

    $backends = @(Invoke-CliJson @('container','backends'))
    $backends | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'backends-reachable.json') -Encoding utf8
    if ($backends.Count -ne 2) { throw "expected two baseline backends, got $($backends.Count)" }
    if (($backends | ForEach-Object { [string]$_.id }) -join ',' -ne 'docker,podman') { throw 'backend discovery order drifted' }
    foreach ($backend in $backends) {
        if ($backend.installed -ne $true) { throw "$($backend.id) fixture was not detected as installed" }
        if ($backend.daemon_reachable -ne $true) { throw "$($backend.id) fixture daemon was not detected as reachable" }
        if ([string]$backend.version -notmatch '99\.0\.0-vsn-fixture') { throw "$($backend.id) fixture version was not captured" }
    }

    $containers = @(Invoke-CliJson @('container','list','docker'))
    $containers | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'containers.json') -Encoding utf8
    if ($containers.Count -ne 1 -or [string]$containers[0].id -ne 'abc123' -or [string]$containers[0].name -ne 'vsn-fixture') {
        throw 'container list fixture row mismatch'
    }

    $images = @(Invoke-CliJson @('container','images','docker'))
    $volumes = @(Invoke-CliJson @('container','volumes','docker'))
    $networks = @(Invoke-CliJson @('container','networks','docker'))
    $images | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'images.json') -Encoding utf8
    $volumes | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'volumes.json') -Encoding utf8
    $networks | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'networks.json') -Encoding utf8
    if ($images.Count -ne 1 -or [string]$images[0].id -ne 'img123') { throw 'image inventory mismatch' }
    if ($volumes.Count -ne 1 -or [string]$volumes[0].id -ne 'vol123') { throw 'volume inventory mismatch' }
    if ($networks.Count -ne 1 -or [string]$networks[0].id -ne 'net123') { throw 'network inventory mismatch' }

    $logs = [string](Invoke-CliJson @('container','logs','docker','abc123'))
    $logs | Set-Content (Join-Path $script:Root 'logs.txt') -Encoding utf8
    if ($logs -notmatch 'fixture stdout log' -or $logs -notmatch 'fixture stderr log') { throw 'bounded container logs lost stdout/stderr fixture content' }

    $inspect = [string](Invoke-CliJson @('container','inspect','docker','abc123'))
    $inspect | Set-Content (Join-Path $script:Root 'inspect.txt') -Encoding utf8
    if ($inspect -notmatch 'abc123') { throw 'container inspect fixture response mismatch' }

    $stats = Invoke-CliJson @('container','stats','docker','abc123')
    $stats | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'stats.json') -Encoding utf8
    if ([string]$stats.name -ne 'vsn-fixture' -or [string]$stats.cpu_percent -ne '0.10%' -or [string]$stats.pids -ne '2') {
        throw 'container stats fixture response mismatch'
    }

    foreach ($action in @('start','restart','stop','pause','unpause')) {
        $result = Invoke-CliJson @('container',$action,'docker','abc123')
        $result | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root ("action-$action.json")) -Encoding utf8
        if ([string]$result.backend -ne 'docker' -or [string]$result.target -ne 'abc123' -or [string]$result.action -ne $action) {
            throw "container $action lifecycle result mismatch"
        }
    }

    $unsupportedOut = Join-Path $script:Root 'unsupported-backend.stdout'
    $unsupportedErr = Join-Path $script:Root 'unsupported-backend.stderr'
    $unsupportedCode = Invoke-CliCapture @('container','list','sh') $unsupportedOut $unsupportedErr
    $unsupportedCode | Set-Content (Join-Path $script:Root 'unsupported-backend.exit-code.txt')
    if ($unsupportedCode -eq 0) { throw 'unsupported backend bypassed allowlist' }

    'daemon-down' | Set-Content -LiteralPath $modeFile -Encoding ascii
    $down = @(Invoke-CliJson @('container','backends'))
    $down | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'backends-daemon-down.json') -Encoding utf8
    foreach ($backend in $down) {
        if ($backend.installed -ne $true) { throw "$($backend.id) lost installed=true when daemon became unavailable" }
        if ($backend.daemon_reachable -ne $false) { throw "$($backend.id) daemon-down state was not reported" }
    }
    $downOut = Join-Path $script:Root 'daemon-down-list.stdout'
    $downErr = Join-Path $script:Root 'daemon-down-list.stderr'
    $downCode = Invoke-CliCapture @('container','list','docker') $downOut $downErr
    $downCode | Set-Content (Join-Path $script:Root 'daemon-down-list.exit-code.txt')
    if ($downCode -eq 0) { throw 'container list unexpectedly succeeded with unavailable daemon' }
    if ((Get-Content $downErr -Raw) -notmatch 'fixture daemon unavailable') { throw 'unavailable daemon error is not actionable' }

    'hang' | Set-Content -LiteralPath $modeFile -Encoding ascii
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $hung = @(Invoke-CliJson @('container','backends'))
    $watch.Stop()
    $hung | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'backends-timeout.json') -Encoding utf8
    $watch.ElapsedMilliseconds | Set-Content (Join-Path $script:Root 'backend-timeout-elapsed-ms.txt')
    if ($watch.Elapsed.TotalSeconds -ge 9) { throw "parallel backend discovery exceeded bounded latency: $($watch.Elapsed.TotalSeconds)s" }
    foreach ($backend in $hung) {
        if ($backend.installed -ne $true -or $backend.daemon_reachable -ne $false) { throw "$($backend.id) timeout did not degrade to installed/unreachable" }
    }

    'flood' | Set-Content -LiteralPath $modeFile -Encoding ascii
    $floodOut = Join-Path $script:Root 'flood-logs.stdout'
    $floodErr = Join-Path $script:Root 'flood-logs.stderr'
    $floodCode = Invoke-CliCapture @('container','logs','docker','abc123') $floodOut $floodErr
    $floodCode | Set-Content (Join-Path $script:Root 'flood-logs.exit-code.txt')
    if ($floodCode -eq 0) { throw 'oversized container log output bypassed safety limit' }
    if ((Get-Content $floodErr -Raw) -notmatch 'output exceeded safety limit') { throw 'container log output-limit failure is not explicit' }

    'reachable' | Set-Content -LiteralPath $modeFile -Encoding ascii
    $chain = Invoke-CliJson @('audit','verify')
    $chain | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.15';
        artifact='docker-podman-baseline-windows-source-first-scaffold';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        backend_discovery_order_verified=$true;
        parallel_discovery_timeout_verified=$true;
        daemon_reachable_state_verified=$true;
        daemon_unavailable_state_verified=$true;
        container_inventory_verified=$true;
        image_volume_network_inventory_verified=$true;
        bounded_logs_verified=$true;
        inspect_stats_verified=$true;
        lifecycle_actions_verified=$true;
        unsupported_backend_rejected=$true;
        oversized_output_rejected=$true;
        audit_chain_valid=$true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $script:Root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $script:Root 'evidence.json.sha256')
}
finally {
    Stop-Agent
    $env:LOCALAPPDATA = $originalLocalAppData
    $env:PATH = $originalPath
    $env:VSN_PKG02_0215_MODE_FILE = $originalModeFile
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}