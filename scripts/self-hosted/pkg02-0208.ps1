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

$root = Join-Path $PWD 'dist-self-hosted\02.08'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ("vsn-pkg02-0208-windows-" + [guid]::NewGuid().ToString('N'))
$fakebin = Join-Path $sandbox 'fakebin'
$workspace = Join-Path $sandbox 'workspace'
$projects = Join-Path $workspace 'projects'
$outside = Join-Path $sandbox 'outside'
$vsnLocalRoot = Join-Path $env:LOCALAPPDATA 'VSN'
$backupRoot = Join-Path $env:RUNNER_TEMP ("vsn-pkg02-0208-backup-" + [guid]::NewGuid().ToString('N'))
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadLocalData = Test-Path -LiteralPath $vsnLocalRoot
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$sandbox,$fakebin,$projects,$outside,$backupRoot | Out-Null
if ($hadLocalData) {
    Copy-Item -LiteralPath $vsnLocalRoot -Destination (Join-Path $backupRoot 'VSN') -Recurse -Force
}

try {
    if (-not $IsWindows) { throw '02.08 certification requires Windows' }
    $listener = Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue
    if ($listener) { throw 'TCP 49731 is already in use; refusing to disturb an existing VSN Agent' }

    $rust = (& rustc --version).Trim()
    $cargoVersion = (& cargo --version).Trim()
    if ($rust -notmatch '^rustc 1\.97\.1\b') { throw "expected rustc 1.97.1, got $rust" }
    if ($cargoVersion -notmatch '^cargo 1\.97\.1\b') { throw "expected cargo 1.97.1, got $cargoVersion" }
    @("rust=$rust", "cargo=$cargoVersion", "runner=$env:RUNNER_NAME", "os=$env:RUNNER_OS", "arch=$env:RUNNER_ARCH") | Set-Content (Join-Path $root 'runner.txt')

    Require-Text 'apps/agent/src/main.rs' 'project.bootstrap'
    Require-Text 'apps/cli/src/main.rs' 'cmd == "project" && sub == "bootstrap"'
    Require-Text 'crates/vsn-core/src/lib.rs' 'pub fn project_bootstrap'
    Require-Text 'crates/vsn-core/src/lib.rs' 'Permission::ProjectEdit'
    Require-Text 'crates/vsn-core/src/lib.rs' 'vsn_files::resolve_for_write(&roots, path)'
    Require-Text 'crates/vsn-project/src/lib.rs' 'pub fn execute_bootstrap'
    Require-Text 'crates/vsn-project/src/lib.rs' 'BOOTSTRAP_STDOUT_CAPTURE_BYTES'
    Require-Text 'crates/vsn-project/src/lib.rs' 'BOOTSTRAP_STDERR_CAPTURE_BYTES'

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-project --all-targets -- -D warnings
    Assert-LastExit 'vsn-project clippy failed'
    cargo test --locked --package vsn-project
    Assert-LastExit 'vsn-project tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    (Get-FileHash (Join-Path $bin 'vsn-agent.exe') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'vsn-agent.sha256')
    (Get-FileHash (Join-Path $bin 'vsn.exe') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'vsn.sha256')

    $fakeSource = Join-Path $sandbox 'fake_npm.rs'
    @'
use std::{env, fs, io::{self, Write}, process};
fn main() {
    let cwd = env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|v| v.to_str()).unwrap_or("");
    match name {
        "success-app" => {
            fs::write(cwd.join("package.json"), b"{\"name\":\"success-app\",\"private\":true}\n").unwrap();
            println!("fake npm success");
        }
        "verbose-success-app" => {
            fs::write(cwd.join("package.json"), b"{\"name\":\"verbose-success-app\",\"private\":true}\n").unwrap();
            io::stdout().lock().write_all(&vec![b'x'; 128 * 1024]).unwrap();
        }
        "fail-new" | "fail-existing" => {
            fs::write(cwd.join("partial.txt"), b"partial\n").unwrap();
            eprintln!("controlled bootstrap failure");
            process::exit(42);
        }
        "verbose-failure-app" => {
            fs::write(cwd.join("partial.txt"), b"partial\n").unwrap();
            io::stderr().lock().write_all(&vec![b'e'; 96 * 1024]).unwrap();
            process::exit(42);
        }
        other => {
            eprintln!("unexpected fake npm cwd={other}");
            process::exit(97);
        }
    }
}
'@ | Set-Content -LiteralPath $fakeSource -Encoding utf8
    rustc $fakeSource -O -o (Join-Path $fakebin 'npm.exe')
    Assert-LastExit 'fake npm build failed'

    $junction = Join-Path $workspace 'outside-link'
    New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null
    $env:PATH = "$fakebin;$env:PATH"

    $agentOut = Join-Path $root 'agent.stdout.log'
    $agentErr = Join-Path $root 'agent.stderr.log'
    $agent = Start-Process -FilePath (Join-Path $bin 'vsn-agent.exe') -RedirectStandardOutput $agentOut -RedirectStandardError $agentErr -PassThru -WindowStyle Hidden
    $agent.Id | Set-Content (Join-Path $root 'agent.pid')

    $ready = $false
    for ($i = 0; $i -lt 80; $i++) {
        $ping = & (Join-Path $bin 'vsn.exe') ping 2> (Join-Path $root 'readiness-ping.err')
        if ($LASTEXITCODE -eq 0) {
            $ping | Set-Content (Join-Path $root 'readiness-ping.json') -Encoding utf8
            $ready = $true
            break
        }
        if ($agent.HasExited) {
            throw "Agent exited before readiness with code $($agent.ExitCode)"
        }
        Start-Sleep -Milliseconds 250
    }
    if (-not $ready) { throw 'Agent did not become ready' }

    $workspaceResult = & (Join-Path $bin 'vsn.exe') workspace add $workspace
    Assert-LastExit 'workspace add failed'
    $workspaceResult | Set-Content (Join-Path $root 'workspace-add.json') -Encoding utf8

    function Invoke-Bootstrap([string]$Name, [string]$Destination, [bool]$ExpectSuccess) {
        $stdout = Join-Path $root "$Name.stdout"
        $stderr = Join-Path $root "$Name.stderr"
        $result = & (Join-Path $bin 'vsn.exe') project bootstrap node $Destination 2> $stderr
        $code = $LASTEXITCODE
        $result | Set-Content $stdout -Encoding utf8
        $code | Set-Content (Join-Path $root "$Name.exit-code.txt")
        if ($ExpectSuccess -and $code -ne 0) { throw "$Name expected success, exit=$code" }
        if (-not $ExpectSuccess -and $code -eq 0) { throw "$Name expected failure but returned exit 0" }
        @{ Code=$code; Stdout=$stdout; Stderr=$stderr }
    }

    $success = Join-Path $projects 'success-app'
    $r = Invoke-Bootstrap 'success' $success $true
    if (-not (Test-Path (Join-Path $success 'package.json'))) { throw 'successful bootstrap did not create package.json' }
    $value = Get-Content $r.Stdout -Raw | ConvertFrom-Json
    if ($value.template -ne 'node' -or $value.status_code -ne 0) { throw 'unexpected success result' }
    if ($value.stdout -notmatch 'fake npm success' -or $value.stdout_truncated -ne $false -or $value.stderr_truncated -ne $false) { throw 'unexpected success output contract' }

    $verboseSuccess = Join-Path $projects 'verbose-success-app'
    $r = Invoke-Bootstrap 'verbose-success' $verboseSuccess $true
    $value = Get-Content $r.Stdout -Raw | ConvertFrom-Json
    if ($value.status_code -ne 0 -or $value.stdout_truncated -ne $true -or $value.stderr_truncated -ne $false) { throw 'verbose success was not bounded correctly' }
    if ([Text.Encoding]::UTF8.GetByteCount([string]$value.stdout) -gt 196608) { throw 'verbose success crossed IPC-safe bound' }

    $failNew = Join-Path $projects 'fail-new'
    foreach ($attempt in 1,2) {
        if (Test-Path $failNew) { throw 'fail-new destination exists before retry' }
        $r = Invoke-Bootstrap "fail-new-$attempt" $failNew $false
        if (Test-Path $failNew) { throw 'failed new destination was not rolled back' }
        $err = Get-Content $r.Stderr -Raw
        if ($err -notmatch '^error=' -or $err -notmatch '42' -or $err -notmatch 'controlled bootstrap failure') { throw 'non-zero child error propagation is incomplete' }
    }

    $failExisting = Join-Path $projects 'fail-existing'
    New-Item -ItemType Directory -Path $failExisting | Out-Null
    foreach ($attempt in 1,2) {
        Invoke-Bootstrap "fail-existing-$attempt" $failExisting $false | Out-Null
        if (-not (Test-Path $failExisting -PathType Container)) { throw 'existing empty destination was removed' }
        if ((Get-ChildItem -LiteralPath $failExisting -Force | Measure-Object).Count -ne 0) { throw 'existing empty destination was not restored' }
    }

    $verboseFailure = Join-Path $projects 'verbose-failure-app'
    $r = Invoke-Bootstrap 'verbose-failure' $verboseFailure $false
    if (Test-Path $verboseFailure) { throw 'verbose failed destination was not rolled back' }
    $err = Get-Content $r.Stderr -Raw
    if ($err -notmatch '42' -or $err -notmatch 'stderr truncated') { throw 'verbose failure truncation/error contract failed' }
    if ([Text.Encoding]::UTF8.GetByteCount($err) -ge 131072) { throw 'verbose failure response crossed bound' }

    $nonempty = Join-Path $projects 'nonempty-app'
    New-Item -ItemType Directory -Path $nonempty | Out-Null
    $sentinel = Join-Path $nonempty 'keep.txt'
    'keep' | Set-Content $sentinel
    $before = (Get-FileHash $sentinel -Algorithm SHA256).Hash
    Invoke-Bootstrap 'nonempty' $nonempty $false | Out-Null
    $after = (Get-FileHash $sentinel -Algorithm SHA256).Hash
    if ($before -ne $after) { throw 'non-empty destination sentinel changed' }
    if ((Get-ChildItem -LiteralPath $nonempty -Force | Measure-Object).Count -ne 1) { throw 'non-empty destination was mutated' }

    Invoke-Bootstrap 'outside' (Join-Path $outside 'outside-app') $false | Out-Null
    Invoke-Bootstrap 'junction-escape' (Join-Path $junction 'escape-app') $false | Out-Null
    if (Test-Path (Join-Path $outside 'outside-app')) { throw 'outside workspace bootstrap wrote data' }
    if (Test-Path (Join-Path $outside 'escape-app')) { throw 'junction escape wrote data' }

    $audit = & (Join-Path $bin 'vsn.exe') audit verify
    Assert-LastExit 'audit verify failed'
    $audit | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    $auditValue = $audit | ConvertFrom-Json
    if ($auditValue.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    $evidence = [ordered]@{
        schema_version = 1
        package_id = 'PKG-02'
        task_id = '02.08'
        artifact = 'windows-self-hosted-bounded-retry-safe-project-bootstrap'
        product_version = $candidate.product_version
        candidate_id = $candidate.candidate_id
        source_commit = $env:GITHUB_SHA
        runner_name = $env:RUNNER_NAME
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        rust_version = '1.97.1'
        build_mode = 'locked-release-from-current-source'
        successful_bootstrap_verified = $true
        verbose_success_bounded_and_truncated = $true
        child_nonzero_fails_operator_command = $true
        new_destination_rollback_verified = $true
        existing_empty_destination_restored = $true
        repeat_failure_idempotent = $true
        verbose_failure_bounded_and_rolled_back = $true
        nonempty_destination_preserved = $true
        outside_workspace_rejected = $true
        junction_escape_rejected = $true
        audit_chain_valid = $true
    }
    $evidence | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    if ($agent -and -not $agent.HasExited) {
        Stop-Process -Id $agent.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $agent.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $vsnLocalRoot) {
        Remove-Item -LiteralPath $vsnLocalRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
    if ($hadLocalData) {
        $source = Join-Path $backupRoot 'VSN'
        if (Test-Path -LiteralPath $source) {
            Copy-Item -LiteralPath $source -Destination $vsnLocalRoot -Recurse -Force
        }
    }
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
