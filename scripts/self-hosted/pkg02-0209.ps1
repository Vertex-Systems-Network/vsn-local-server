param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Require-Text([string]$Path, [string]$Pattern) {
    if (-not (Select-String -LiteralPath $Path -SimpleMatch $Pattern -Quiet)) {
        throw "missing source invariant '$Pattern' in $Path"
    }
}

function Invoke-Cli([string[]]$Args, [string]$StdoutPath, [string]$StderrPath) {
    $output = & $script:Cli @Args 2> $StderrPath
    $code = $LASTEXITCODE
    $output | Set-Content -LiteralPath $StdoutPath -Encoding utf8
    return $code
}

$root = Join-Path $PWD 'dist-self-hosted\02.09'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ("vsn-pkg02-0209-windows-" + [guid]::NewGuid().ToString('N'))
$fakebin = Join-Path $sandbox 'fakebin'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$sandbox,$fakebin,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

try {
    if ($env:RUNNER_NAME -ne 'LOCAL-WIN-02') { throw "LOCAL-WIN-02 required, got '$env:RUNNER_NAME'" }
    if (-not $IsWindows) { throw '02.09 certification requires Windows' }
    $listener = Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue
    if ($listener) { throw 'TCP 49731 is already in use; refusing to disturb an existing VSN Agent' }

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $root 'runner.txt')

    Require-Text 'crates/vsn-runtime/src/lib.rs' 'pub fn detect_all'
    Require-Text 'crates/vsn-runtime/src/lib.rs' 'pub fn load_registry'
    Require-Text 'crates/vsn-runtime/src/lib.rs' 'pub fn audit_registry'
    Require-Text 'apps/agent/src/main.rs' 'runtime.list'
    Require-Text 'apps/agent/src/main.rs' 'runtime.registry'
    Require-Text 'apps/agent/src/main.rs' 'runtime.audit'
    Require-Text 'apps/agent/src/main.rs' 'runtime.conformance'

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
    $agentExe = Join-Path $bin 'vsn-agent.exe'

    $fakeSource = Join-Path $sandbox 'fake_runtime.rs'
    @'
use std::{env, thread, time::Duration};
fn main() {
    let exe = env::current_exe().expect("exe");
    let stem = exe.file_stem().and_then(|v| v.to_str()).unwrap_or("").to_ascii_lowercase();
    match stem.as_str() {
        "php" => println!("PHP 99.0.0 (cli)"),
        "node" | "java" | "deno" => {
            thread::sleep(Duration::from_secs(30));
            println!("hostile probe unexpectedly completed");
        }
        other => { eprintln!("unexpected fake runtime {other}"); std::process::exit(97); }
    }
}
'@ | Set-Content -LiteralPath $fakeSource -Encoding utf8
    rustc $fakeSource -O -o (Join-Path $fakebin 'fake-runtime.exe')
    Assert-LastExit 'fake runtime build failed'
    foreach ($name in 'php','node','java','deno') {
        Copy-Item (Join-Path $fakebin 'fake-runtime.exe') (Join-Path $fakebin "$name.exe") -Force
    }
    $env:PATH = "$fakebin;$env:PATH"

    function Start-Agent {
        $agentOut = Join-Path $root 'agent.stdout.log'
        $agentErr = Join-Path $root 'agent.stderr.log'
        $script:agent = Start-Process -FilePath $agentExe -RedirectStandardOutput $agentOut -RedirectStandardError $agentErr -PassThru -WindowStyle Hidden
        $script:agent.Id | Set-Content (Join-Path $root 'agent.pid')
        $ready = $false
        for ($i = 0; $i -lt 80; $i++) {
            $ping = & $script:Cli ping 2> (Join-Path $root 'readiness-ping.err')
            if ($LASTEXITCODE -eq 0) {
                $ping | Set-Content (Join-Path $root 'readiness-ping.json') -Encoding utf8
                $ready = $true
                break
            }
            if ($script:agent.HasExited) { throw "Agent exited before readiness with code $($script:agent.ExitCode)" }
            Start-Sleep -Milliseconds 250
        }
        if (-not $ready) { throw 'Agent did not become ready' }
    }

    function Stop-Agent {
        if ($script:agent -and -not $script:agent.HasExited) {
            Stop-Process -Id $script:agent.Id -Force
            Wait-Process -Id $script:agent.Id -Timeout 10 -ErrorAction SilentlyContinue
        }
        $script:agent = $null
    }

    Start-Agent
    $diagnostics = & $script:Cli diagnostics
    Assert-LastExit 'diagnostics failed'
    $diagnostics | Set-Content (Join-Path $root 'diagnostics.json') -Encoding utf8
    $conformance = & $script:Cli runtime conformance
    Assert-LastExit 'runtime conformance failed'
    $conformance | Set-Content (Join-Path $root 'conformance.json') -Encoding utf8
    $conformanceValue = $conformance | ConvertFrom-Json
    if ($conformanceValue.valid -ne $true) { throw 'runtime provider conformance is invalid' }

    $runtimeErr = Join-Path $root 'runtime-list.err'
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $runtimeList = & $script:Cli runtime list 2> $runtimeErr
    $runtimeCode = $LASTEXITCODE
    $watch.Stop()
    $runtimeCode | Set-Content (Join-Path $root 'runtime-list.exit-code.txt')
    $watch.ElapsedMilliseconds | Set-Content (Join-Path $root 'runtime-list.elapsed-ms.txt')
    if ($runtimeCode -ne 0) { throw "runtime list failed with exit $runtimeCode" }
    if ($watch.Elapsed.TotalSeconds -ge 8) { throw "runtime list exceeded aggregate 8s bound: $($watch.Elapsed.TotalSeconds)s" }
    $runtimeList | Set-Content (Join-Path $root 'runtime-list.json') -Encoding utf8
    $items = @($runtimeList | ConvertFrom-Json)
    $expected = @('php','node','python','go','rust','java','dotnet','ruby','bun','deno')
    $ids = @($items | ForEach-Object { $_.id })
    if (($ids -join ',') -ne ($expected -join ',')) { throw "runtime inventory order mismatch: $($ids -join ',')" }
    if (($ids | Select-Object -Unique).Count -ne $ids.Count) { throw 'runtime inventory contains duplicate ids' }
    $php = $items | Where-Object id -eq 'php'
    if (-not $php.installed -or -not ([string]$php.version).StartsWith('PHP 99.0.0')) { throw 'fake PHP runtime was not detected' }
    foreach ($id in 'node','java','deno') {
        $item = $items | Where-Object id -eq $id
        if ($item.installed) { throw "$id hostile probe should time out and report unavailable" }
    }

    $diagnosticsValue = $diagnostics | ConvertFrom-Json
    $runtimeRoot = Join-Path ([string]$diagnosticsValue.data_dir) 'runtimes'
    $registry = Join-Path $runtimeRoot 'registry.json'
    New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
    Stop-Agent

    '{not-json' | Set-Content -LiteralPath $registry -Encoding utf8
    Start-Agent
    $malformedOut = Join-Path $root 'malformed-registry.stdout'
    $malformedErr = Join-Path $root 'malformed-registry.stderr'
    $malformedCode = Invoke-Cli @('runtime','registry') $malformedOut $malformedErr
    $malformedCode | Set-Content (Join-Path $root 'malformed-registry.exit-code.txt')
    if ($malformedCode -eq 0) { throw 'malformed registry must fail closed' }
    if ((Get-Item $malformedOut).Length -ne 0) { throw 'malformed registry unexpectedly wrote stdout' }
    if ((Get-Content $malformedErr -Raw) -notmatch 'error=') { throw 'malformed registry did not surface operator error' }
    Stop-Agent

    $outside = Join-Path $sandbox 'outside-runtime'
    New-Item -ItemType Directory -Force -Path $outside | Out-Null
    $evil = Join-Path $outside 'evil.exe'
    'sentinel' | Set-Content -LiteralPath $evil
    $installed = @(
        [ordered]@{ runtime='node'; version='20.0.0'; install_dir=$outside; executable=$evil; source_sha256=('0' * 64) },
        [ordered]@{ runtime='node'; version='20.0.0'; install_dir=$outside; executable=$evil; source_sha256=('0' * 64) },
        [ordered]@{ runtime='unknown-runtime'; version='1.0.0'; install_dir=$outside; executable=$evil; source_sha256=('0' * 64) }
    )
    $activation = @{}
    $activation[(Join-Path $sandbox 'project')] = @{ 'missing-runtime' = '9.9.9' }
    [ordered]@{ installed=$installed; project_activation=$activation } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $registry -Encoding utf8

    Start-Agent
    $registryJson = & $script:Cli runtime registry
    Assert-LastExit 'runtime registry failed on syntactically valid tampered registry'
    $registryJson | Set-Content (Join-Path $root 'tampered-registry.json') -Encoding utf8
    $auditJson = & $script:Cli runtime audit
    Assert-LastExit 'runtime audit failed'
    $auditJson | Set-Content (Join-Path $root 'tampered-audit.json') -Encoding utf8
    $audit = $auditJson | ConvertFrom-Json
    if ($audit.healthy -ne $false) { throw 'tampered registry must audit unhealthy' }
    $codes = @($audit.issues | ForEach-Object { $_.code })
    foreach ($required in 'duplicate_registration','unknown_runtime','install_dir_escape','dangling_activation') {
        if ($codes -notcontains $required) { throw "missing audit issue code $required" }
    }
    $chainJson = & $script:Cli audit verify
    Assert-LastExit 'audit chain verify failed'
    $chainJson | Set-Content (Join-Path $root 'audit-chain.json') -Encoding utf8
    $chain = $chainJson | ConvertFrom-Json
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task_id = '02.09'
        artifact = 'local-win-02-bounded-runtime-inventory-registry-audit'
        product_version = $candidate.product_version
        candidate_id = $candidate.candidate_id
        source_commit = $env:GITHUB_SHA
        runner_name = $env:RUNNER_NAME
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        runtime_probe_timeout_verified = $true
        aggregate_inventory_latency_bounded = $true
        multiple_hostile_probes_isolated = $true
        provider_inventory_exact_unique = $true
        malformed_registry_fail_closed = $true
        duplicate_registration_detected = $true
        unknown_runtime_detected = $true
        install_root_escape_detected = $true
        dangling_activation_detected = $true
        audit_chain_valid = $true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    if ($agent -and -not $agent.HasExited) {
        Stop-Process -Id $agent.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $agent.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
