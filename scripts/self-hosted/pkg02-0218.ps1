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

$script:Root = Join-Path $PWD 'dist-self-hosted\02.18'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0218-' + [guid]::NewGuid().ToString('N'))
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
    if (-not $IsWindows) { throw "02.18 acceptance requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $script:Root 'runner.txt')

    $terminalSource = (Get-Content 'crates/vsn-terminal/src/lib.rs' -Raw) + "`n" + (Get-Content 'crates/vsn-terminal/src/lib_base.rs' -Raw)
    foreach ($needle in @(
        'pub fn execute',
        'DIRECT_MAX_OUTPUT_BYTES',
        'DIRECT_MAX_TIMEOUT_MS',
        'DIRECT_OUTPUT_DRAIN_GRACE',
        'JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE',
        'assign_process_to_job_object',
        'terminate_job_object',
        'process_group(0)',
        'read_direct_output',
        'RecvTimeoutError::Timeout'
    )) {
        if (-not $terminalSource.Contains($needle)) { throw "missing 02.18 terminal source invariant: $needle" }
    }
    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('pub fn terminal_execute','Permission::TerminalExecute','vsn_terminal::execute')) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.18 Core boundary invariant: $needle" }
    }
    $probeSource = Get-Content 'crates/vsn-terminal/examples/pkg02_0218_probe.rs' -Raw
    foreach ($needle in @('helper-parent','helper-child','helper-large-output','descendant-sentinel.txt','768 * 1024')) {
        if (-not $probeSource.Contains($needle)) { throw "missing 02.18 deterministic probe invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-terminal --package vsn-files --package vsn-system --package vsn-core --package vsn-agent --package vsn --all-targets -- -D warnings
    Assert-LastExit 'terminal/files/system/core/agent/cli clippy failed'
    cargo test --locked --package vsn-terminal --package vsn-core --package vsn-agent --package vsn
    Assert-LastExit 'terminal/core/agent/cli tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-terminal --example pkg02_0218_probe
    Assert-LastExit '02.18 direct terminal probe build failed'
    $probe = Join-Path $PWD 'target\release\examples\pkg02_0218_probe.exe'
    & $probe $workspace $outside | Set-Content (Join-Path $script:Root 'direct-exec-probe.json') -Encoding utf8
    Assert-LastExit '02.18 direct terminal probe failed'
    $probeResult = Get-Content (Join-Path $script:Root 'direct-exec-probe.json') -Raw | ConvertFrom-Json
    if ($probeResult.large_output_capture_bytes -ne 524288 -or $probeResult.large_output_truncated -ne $true) { throw 'bounded output probe mismatch' }
    if ($probeResult.large_output_completed_without_timeout -ne $true) { throw 'large output deadlocked or timed out' }
    if ($probeResult.timeout_triggered -ne $true -or $probeResult.descendant_sentinel_absent -ne $true) { throw 'process-tree timeout containment probe mismatch' }
    if ($probeResult.outside_cwd_rejected -ne $true -or $probeResult.outside_absolute_program_rejected -ne $true) { throw 'terminal workspace/program containment probe mismatch' }
    if ([uint64]$probeResult.timeout_duration_ms -gt 4000) { throw 'direct execution exceeded bounded timeout shutdown budget' }

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent

    $workspaceAdd = Invoke-CliJson @('workspace','add',$workspace)
    $workspaceAdd | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'workspace-add.json') -Encoding utf8

    $basic = Invoke-CliJson @('terminal','exec',$workspace,'cmd.exe','/d','/c','echo stdout-marker-0218&&echo stderr-marker-0218 1>&2')
    $basic | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'terminal-basic.json') -Encoding utf8
    if ($basic.timed_out -ne $false -or [string]$basic.stdout -notmatch 'stdout-marker-0218' -or [string]$basic.stderr -notmatch 'stderr-marker-0218') {
        throw 'Agent/CLI direct terminal stdout/stderr contract mismatch'
    }

    $outsideOut = Join-Path $script:Root 'outside-cwd.stdout'
    $outsideErr = Join-Path $script:Root 'outside-cwd.stderr'
    $outsideCode = Invoke-CliCapture @('terminal','exec',$outside,'cmd.exe','/d','/c','echo blocked') $outsideOut $outsideErr
    if ($outsideCode -eq 0) { throw 'Agent/CLI direct terminal accepted outside-workspace cwd' }

    $cmdPath = (Get-Command cmd.exe).Source
    $programOut = Join-Path $script:Root 'outside-program.stdout'
    $programErr = Join-Path $script:Root 'outside-program.stderr'
    $programCode = Invoke-CliCapture @('terminal','exec',$workspace,$cmdPath,'/d','/c','echo blocked') $programOut $programErr
    if ($programCode -eq 0) { throw 'Agent/CLI direct terminal accepted absolute executable outside workspace' }

    $chain = Invoke-CliJson @('audit','verify')
    $chain | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'audit-chain.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain is invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.18';
        artifact='bounded-direct-terminal-execution-windows-source-first-scaffold';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        terminal_execute_permission_boundary_verified=$true;
        workspace_cwd_containment_verified=$true;
        absolute_program_containment_verified=$true;
        bounded_timeout_verified=$true;
        process_tree_termination_verified=$true;
        descendant_pipe_handle_shutdown_verified=$true;
        output_capture_ceiling_verified=$true;
        output_drain_after_capture_ceiling_verified=$true;
        stdout_stderr_contract_verified=$true;
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
