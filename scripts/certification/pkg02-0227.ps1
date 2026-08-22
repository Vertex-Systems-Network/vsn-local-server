$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$TrackerPath = 'certification/pkg02-usable-local-beta-v1.json'
$EvidenceRoot = Join-Path $PWD 'dist-certification/02.27'
New-Item -ItemType Directory -Force -Path $EvidenceRoot | Out-Null

function Invoke-Checked {
    param([string]$Message, [scriptblock]$Command)
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

$tracker = Get-Content $TrackerPath -Raw | ConvertFrom-Json
if ($tracker.package_id -ne 'PKG-02' -or [int]$tracker.required -ne 27) {
    throw 'PKG-02 tracker schema/package identity is invalid'
}
if ([int]$tracker.done -ne 26 -or $tracker.complete -ne $false -or $tracker.active_task -ne '02.27') {
    throw "02.27 may run only at 26/27 with active_task=02.27; got done=$($tracker.done) active=$($tracker.active_task) complete=$($tracker.complete)"
}

$expected = 1..26 | ForEach-Object { '02.{0:D2}' -f $_ }
foreach ($taskId in $expected) {
    $task = @($tracker.tasks | Where-Object id -eq $taskId)
    if ($task.Count -ne 1) { throw "tracker must contain exactly one $taskId entry" }
    if ($task[0].status -ne 'DONE') { throw "$taskId is not DONE" }
}
$final = @($tracker.tasks | Where-Object id -eq '02.27')
if ($final.Count -ne 1 -or $final[0].status -ne 'IN_PROGRESS') {
    throw '02.27 tracker entry must be IN_PROGRESS during final-gate execution'
}

$before = (git status --porcelain=v1 --untracked-files=all) -join "`n"
if ($LASTEXITCODE -ne 0) { throw 'unable to capture pre-gate repository status' }
if ($before) { throw "02.27 requires a clean checkout before smoke; drift:`n$before" }

rustup toolchain install 1.97.1 --profile minimal --component rustfmt,clippy
if ($LASTEXITCODE -ne 0) { throw 'Rust 1.97.1 toolchain setup failed' }
$env:RUSTUP_TOOLCHAIN = '1.97.1'

Invoke-Checked 'rustfmt failed' { cargo fmt --all -- --check }
Invoke-Checked 'workspace Clippy failed' { cargo clippy --workspace --all-targets --locked -- -D warnings }
Invoke-Checked 'workspace tests failed' { cargo test --workspace --locked }
Invoke-Checked 'release Agent/CLI build failed' { cargo build --locked --release -p vsn-agent -p vsn }

if (-not (Test-Path 'apps/desktop/package-lock.json')) { throw 'Desktop lockfile is missing' }
Push-Location 'apps/desktop'
try {
    Invoke-Checked 'Desktop npm ci failed' { npm ci }
    Invoke-Checked 'Desktop production build failed' { npm run build }
} finally {
    Pop-Location
}

Invoke-Checked 'CLI version smoke failed' { & 'target/release/vsn.exe' version }
Invoke-Checked 'CLI command catalog smoke failed' { & 'target/release/vsn.exe' help }

$after = (git status --porcelain=v1 --untracked-files=all) -join "`n"
if ($LASTEXITCODE -ne 0) { throw 'unable to capture post-gate repository status' }
if ($after -ne $before) {
    throw "02.27 introduced repository drift.`nBefore:`n$before`nAfter:`n$after"
}

$source = (git rev-parse HEAD).Trim()
$evidence = [ordered]@{
    schema_version = 1
    package_id = 'PKG-02'
    task_id = '02.27'
    source_commit = $source
    runner_name = $env:RUNNER_NAME
    runner_environment = $env:RUNNER_ENVIRONMENT
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    rust_version = (& rustc --version).Trim()
    prerequisite_tasks_done = 26
    clean_checkout_before_smoke = $true
    workspace_fmt_clippy_tests_pass = $true
    release_agent_cli_build_pass = $true
    desktop_locked_build_pass = $true
    cli_smoke_pass = $true
    unintended_repository_drift = $false
}
$evidence | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $EvidenceRoot 'evidence.json') -Encoding utf8
Get-FileHash (Join-Path $EvidenceRoot 'evidence.json') -Algorithm SHA256 | ForEach-Object Hash | Set-Content (Join-Path $EvidenceRoot 'evidence.json.sha256')
