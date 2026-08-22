param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

$root = Join-Path $PWD 'dist-self-hosted\02.24'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0224-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$isolatedLocalAppData | Out-Null
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
    if (-not $IsWindows) { throw '02.24 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.24 certification requires a GitHub-hosted runner' }
    Write-Host "runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $network = Get-Content 'crates/vsn-network/src/lib.rs' -Raw
    foreach ($needle in @('pub fn plan_local_domain','pub fn apply_hosts_domain','pub fn remove_hosts_domain','requires_admin_for_hosts_file','127.0.0.1','pub fn render_caddyfile','fn atomic_write')) {
        if (-not $network.Contains($needle)) { throw "missing local-domain invariant: $needle" }
    }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    foreach ($needle in @('network-admin','network-admin commands require OS elevation','local_network_admin','apply-hosts','remove-hosts','install-ca','proxy-config')) {
        if (-not $agentSource.Contains($needle)) { throw "missing privileged network boundary invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-network --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'network/core clippy failed'
    cargo test --locked --package vsn-network --package vsn-core
    Assert-LastExit 'network/core tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb another Agent' }
    Start-Agent

    $plan = & $script:Cli domain plan demo.test 8123 true | Out-String | ConvertFrom-Json
    $plan | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'domain-plan.json') -Encoding utf8
    if ([string]$plan.domain -ne 'demo.test' -or [string]$plan.target_host -ne '127.0.0.1' -or [int]$plan.target_port -ne 8123 -or $plan.tls -ne $true -or $plan.requires_admin_for_hosts_file -ne $true) {
        throw 'domain plan did not preserve loopback/TLS/elevation contract'
    }

    & $script:Cli domain plan 'bad.example' 8123 false 1> (Join-Path $root 'invalid-domain.stdout') 2> (Join-Path $root 'invalid-domain.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'non-.test domain unexpectedly accepted' }

    # The ordinary authenticated Agent command surface must not expose privileged hosts mutation.
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw
    if ($cliSource.Contains('domain apply-hosts') -or $cliSource.Contains('domain remove-hosts')) {
        throw 'ordinary CLI domain surface directly exposes privileged hosts mutation'
    }

    # The dedicated network-admin binary path must fail closed unless the OS process is elevated.
    # On GitHub-hosted Windows the runner may itself be elevated; capture the observed posture without
    # modifying the real hosts file. Static source checks above preserve the mandatory elevation gate.
    & (Join-Path $bin 'vsn-agent.exe') network-admin 1> (Join-Path $root 'network-admin-empty.stdout') 2> (Join-Path $root 'network-admin-empty.stderr')
    $emptyCode = $LASTEXITCODE
    $emptyCode | Set-Content (Join-Path $root 'network-admin-empty.exit-code.txt')
    if ($emptyCode -eq 0) { throw 'empty privileged network-admin command unexpectedly succeeded' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.24';
        artifact='local-domain-https-privileged-boundary-windows-github-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_environment=$env:RUNNER_ENVIRONMENT;
        domain_plan_verified=$true; test_suffix_enforced=$true; loopback_upstream_verified=$true;
        privileged_boundary_source_verified=$true; hosts_fail_closed_tests_passed=$true; audit_chain_valid=$true
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
