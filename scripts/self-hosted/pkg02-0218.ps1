param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

$root = Join-Path $PWD 'dist-self-hosted\02.18'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0218-' + [guid]::NewGuid().ToString('N'))
$workspace = Join-Path $sandbox 'workspace'
$outside = Join-Path $sandbox 'outside'
$fixture = Join-Path $workspace 'terminal-fixture.exe'
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

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
    if (-not $IsWindows) { throw "02.18 certification requires Windows; selected runner '$env:RUNNER_NAME' is incompatible" }
    Write-Host "selected runner=$env:RUNNER_NAME os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $terminal = Get-Content 'crates/vsn-terminal/src/lib.rs' -Raw
    foreach ($needle in @('MAX_OUTPUT_BYTES','MAX_TIMEOUT_MS','pub fn execute','read_bounded','resolve_program','timeout_ms.clamp')) {
        if (-not $terminal.Contains($needle)) { throw "missing terminal invariant: $needle" }
    }
    $ipc = Get-Content 'crates/vsn-ipc/src/lib.rs' -Raw
    if (-not $ipc.Contains('MAX_FRAME_BYTES: usize = 1024 * 1024')) { throw '02.18 acceptance expects the current 1 MiB IPC frame contract' }
    $agentSource = Get-Content 'apps/agent/src/main.rs' -Raw
    if (-not $agentSource.Contains('terminal.exec')) { throw 'Agent terminal.exec command missing' }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-terminal --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'terminal clippy failed'
    cargo test --locked --package vsn-terminal --package vsn-core
    Assert-LastExit 'terminal tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $fixtureSource = Join-Path $sandbox 'terminal_fixture.rs'
    @'
use std::{env, io::{self, Write}, process, thread, time::Duration};
fn main() {
    match env::args().nth(1).as_deref() {
        Some("exit") => {
            println!("stdout-marker");
            eprintln!("stderr-marker");
            process::exit(7);
        }
        Some("huge") => {
            let out = thread::spawn(|| -> io::Result<()> {
                let mut stdout = io::stdout().lock();
                let block = vec![b'O'; 8192];
                for _ in 0..90 { stdout.write_all(&block)?; }
                stdout.flush()
            });
            let err = thread::spawn(|| -> io::Result<()> {
                let mut stderr = io::stderr().lock();
                let block = vec![b'E'; 8192];
                for _ in 0..90 { stderr.write_all(&block)?; }
                stderr.flush()
            });
            if out.join().ok().and_then(Result::ok).is_none() || err.join().ok().and_then(Result::ok).is_none() {
                process::exit(73);
            }
        }
        Some("sleep") => {
            thread::sleep(Duration::from_secs(35));
            println!("sleep unexpectedly completed without timeout");
        }
        _ => process::exit(97),
    }
}
'@ | Set-Content -LiteralPath $fixtureSource -Encoding utf8
    rustc $fixtureSource -O -o $fixture
    Assert-LastExit 'terminal fixture build failed'

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb an existing VSN Agent' }
    Start-Agent
    & $script:Cli workspace add $workspace | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8
    Assert-LastExit 'workspace add failed'

    $exitResult = & $script:Cli terminal exec $workspace $fixture exit | Out-String | ConvertFrom-Json
    $exitResult | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'exit-result.json') -Encoding utf8
    if ([int]$exitResult.exit_code -ne 7 -or $exitResult.timed_out -ne $false) { throw 'direct exec did not preserve child exit status' }
    if (-not ([string]$exitResult.stdout).Contains('stdout-marker') -or -not ([string]$exitResult.stderr).Contains('stderr-marker')) { throw 'direct exec lost stdout/stderr markers' }

    $hugeOut = Join-Path $root 'huge-result.json'
    $hugeErr = Join-Path $root 'huge-cli.stderr'
    & $script:Cli terminal exec $workspace $fixture huge 1> $hugeOut 2> $hugeErr
    $hugeCode = $LASTEXITCODE
    $hugeCode | Set-Content (Join-Path $root 'huge-cli.exit-code.txt')
    if ($hugeCode -ne 0) { throw 'bounded high-output command must remain representable through IPC' }
    if ((Get-Item $hugeOut).Length -ge 900000) { throw 'terminal result exceeded the frame-safe response budget' }
    $huge = Get-Content $hugeOut -Raw | ConvertFrom-Json
    if ([int]$huge.exit_code -ne 0 -or $huge.timed_out -ne $false) { throw 'output truncation altered child process semantics' }
    if ($huge.stdout_truncated -ne $true -or $huge.stderr_truncated -ne $true) { throw 'high-output command did not report both truncation flags' }

    $watch = [Diagnostics.Stopwatch]::StartNew()
    $timeout = & $script:Cli terminal exec $workspace $fixture sleep | Out-String | ConvertFrom-Json
    $watch.Stop()
    $timeout | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'timeout-result.json') -Encoding utf8
    $watch.ElapsedMilliseconds | Set-Content (Join-Path $root 'timeout-elapsed-ms.txt')
    if ($timeout.timed_out -ne $true) { throw 'long-running direct command did not time out' }
    if ($watch.Elapsed.TotalSeconds -lt 29 -or $watch.Elapsed.TotalSeconds -gt 34) { throw "direct timeout outside expected 30s bound: $($watch.Elapsed.TotalSeconds)s" }

    & $script:Cli terminal exec $workspace '__vsn_missing_command_0218__' 1> (Join-Path $root 'missing.stdout') 2> (Join-Path $root 'missing.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'missing command unexpectedly succeeded' }
    & $script:Cli terminal exec $outside $fixture exit 1> (Join-Path $root 'outside.stdout') 2> (Join-Path $root 'outside.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'outside-workspace cwd unexpectedly succeeded' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.18';
        artifact='bounded-direct-terminal-execution-windows-self-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_os=$env:RUNNER_OS; runner_arch=$env:RUNNER_ARCH;
        exit_status_verified=$true; stdout_stderr_verified=$true; frame_safe_output_verified=$true;
        truncation_does_not_change_child_semantics=$true; timeout_verified=$true; invalid_command_rejected=$true;
        workspace_cwd_containment_verified=$true; audit_chain_valid=$true
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
