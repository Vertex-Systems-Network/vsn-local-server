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
        'DIRECT_MAX_RESULT_JSON_BYTES',
        'JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE',
        'assign_process_to_job_object',
        'terminate_job_object',
        'process_group(0)',
        'read_direct_output',
        'RecvTimeoutError::Timeout',
        'enforce_direct_result_budget'
    )) {
        if (-not $terminalSource.Contains($needle)) { throw "missing 02.18 terminal source invariant: $needle" }
    }

    $ipcSource = (Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw) + "`n" + (Get-Content 'crates/vsn-ipc/src/lib_base.rs' -Raw)
    foreach ($needle in @(
        'MAX_FRAME_BYTES: usize = 1024 * 1024',
        'TERMINAL_EXEC_CALL_READ_TIMEOUT',
        'Duration::from_secs(35)',
        'pub fn call_with_timeout',
        'command == "terminal.exec"',
        'MAX_CALL_READ_TIMEOUT'
    )) {
        if (-not $ipcSource.Contains($needle)) { throw "missing 02.18 IPC transport invariant: $needle" }
    }

    $coreSource = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('pub fn terminal_execute','Permission::TerminalExecute','vsn_terminal::execute')) {
        if (-not $coreSource.Contains($needle)) { throw "missing 02.18 Core boundary invariant: $needle" }
    }
    $cliSource = Get-Content 'apps/cli/src/main.rs' -Raw
    foreach ($needle in @('terminal" && sub == "exec','"terminal.exec"','"timeout_ms":30000')) {
        if (-not $cliSource.Contains($needle)) { throw "missing 02.18 CLI direct-exec invariant: $needle" }
    }
    $probeSource = Get-Content 'crates/vsn-terminal/examples/pkg02_0218_probe.rs' -Raw
    foreach ($needle in @(
        'helper-parent',
        'helper-child',
        'helper-exit',
        'helper-transport-delay',
        'helper-large-output',
        'descendant-sentinel.txt',
        'FRAME_SAFE_RESULT_BYTES'
    )) {
        if (-not $probeSource.Contains($needle)) { throw "missing 02.18 deterministic probe invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-terminal --package vsn-files --package vsn-system --package vsn-ipc --package vsn-core --package vsn-agent --package vsn --all-targets -- -D warnings
    Assert-LastExit 'terminal/files/system/ipc/core/agent/cli clippy failed'
    cargo test --locked --package vsn-terminal --package vsn-ipc --package vsn-core --package vsn-agent --package vsn
    Assert-LastExit 'terminal/ipc/core/agent/cli tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-terminal --example pkg02_0218_probe
    Assert-LastExit '02.18 direct terminal probe build failed'
    $probe = Join-Path $PWD 'target\release\examples\pkg02_0218_probe.exe'
    & $probe $workspace $outside | Set-Content (Join-Path $script:Root 'direct-exec-probe.json') -Encoding utf8
    Assert-LastExit '02.18 direct terminal probe failed'
    $probeResult = Get-Content (Join-Path $script:Root 'direct-exec-probe.json') -Raw | ConvertFrom-Json
    if ($probeResult.exit_status_verified -ne $true -or $probeResult.stdout_stderr_verified -ne $true) { throw 'direct-exec exit/stdout/stderr probe mismatch' }
    if ($probeResult.large_stdout_truncated -ne $true -or $probeResult.large_stderr_truncated -ne $true) { throw 'dual-stream truncation probe mismatch' }
    if ($probeResult.frame_safe_result_verified -ne $true -or [uint64]$probeResult.large_result_json_bytes -gt 786432) { throw 'frame-safe result probe mismatch' }
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
    $fixture = Join-Path $workspace 'pkg02-0218-probe.exe'
    if (-not (Test-Path -LiteralPath $fixture -PathType Leaf)) { throw '02.18 workspace probe fixture was not preserved for Agent/CLI acceptance' }

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent

    $workspaceAdd = Invoke-CliJson @('workspace','add',$workspace)
    $workspaceAdd | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'workspace-add.json') -Encoding utf8

    $exitResult = Invoke-CliJson @('terminal','exec',$workspace,$fixture,'helper-exit')
    $exitResult | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'terminal-exit.json') -Encoding utf8
    if ([int]$exitResult.exit_code -ne 7 -or $exitResult.timed_out -ne $false) { throw 'Agent/CLI direct exec did not preserve child exit status' }
    if ([string]$exitResult.stdout -notmatch 'stdout-marker-0218' -or [string]$exitResult.stderr -notmatch 'stderr-marker-0218') {
        throw 'Agent/CLI direct terminal stdout/stderr contract mismatch'
    }

    $hugeOut = Join-Path $script:Root 'terminal-huge.json'
    $hugeErr = Join-Path $script:Root 'terminal-huge.stderr'
    $hugeCode = Invoke-CliCapture @('terminal','exec',$workspace,$fixture,'helper-large-output') $hugeOut $hugeErr
    if ($hugeCode -ne 0) { throw 'frame-safe high-output direct command failed through Agent/CLI' }
    if ((Get-Item -LiteralPath $hugeOut).Length -ge 900000) { throw 'terminal CLI result exceeded conservative frame-safe response budget' }
    $huge = Get-Content -LiteralPath $hugeOut -Raw | ConvertFrom-Json
    if ([int]$huge.exit_code -ne 0 -or $huge.timed_out -ne $false) { throw 'output truncation altered direct child semantics' }
    if ($huge.stdout_truncated -ne $true -or $huge.stderr_truncated -ne $true) { throw 'high-output direct command did not report both truncation flags' }

    $transportWatch = [Diagnostics.Stopwatch]::StartNew()
    $transport = Invoke-CliJson @('terminal','exec',$workspace,$fixture,'helper-transport-delay')
    $transportWatch.Stop()
    $transport | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $script:Root 'terminal-transport-delay.json') -Encoding utf8
    $transportWatch.ElapsedMilliseconds | Set-Content (Join-Path $script:Root 'terminal-transport-delay-ms.txt')
    if ([int]$transport.exit_code -ne 0 -or $transport.timed_out -ne $false -or [string]$transport.stdout -notmatch 'transport-delay-complete-0218') {
        throw 'terminal.exec did not survive a valid command beyond the old 5-second IPC read timeout'
    }
    if ($transportWatch.Elapsed.TotalSeconds -lt 5.5 -or $transportWatch.Elapsed.TotalSeconds -gt 12) { throw "terminal transport delay outside expected bounded window: $($transportWatch.Elapsed.TotalSeconds)s" }

    $missingOut = Join-Path $script:Root 'missing-command.stdout'
    $missingErr = Join-Path $script:Root 'missing-command.stderr'
    $missingCode = Invoke-CliCapture @('terminal','exec',$workspace,'__vsn_missing_command_0218__') $missingOut $missingErr
    if ($missingCode -eq 0) { throw 'missing direct command unexpectedly succeeded' }

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
        exit_status_verified=$true;
        stdout_stderr_verified=$true;
        frame_safe_output_verified=$true;
        truncation_does_not_change_child_semantics=$true;
        workspace_cwd_containment_verified=$true;
        absolute_program_containment_verified=$true;
        invalid_command_rejected=$true;
        bounded_timeout_verified=$true;
        process_tree_termination_verified=$true;
        descendant_pipe_handle_shutdown_verified=$true;
        output_drain_after_capture_ceiling_verified=$true;
        ipc_transport_timeout_alignment_verified=$true;
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
