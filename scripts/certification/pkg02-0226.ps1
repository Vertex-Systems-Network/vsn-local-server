$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Task = '02.26'
$EvidenceRoot = Join-Path $PWD 'dist-certification/02.26'
New-Item -ItemType Directory -Force -Path $EvidenceRoot | Out-Null

function Invoke-Checked {
    param([string]$Message, [scriptblock]$Command)
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

rustup toolchain install 1.97.1 --profile minimal --component rustfmt,clippy
if ($LASTEXITCODE -ne 0) { throw 'Rust 1.97.1 toolchain setup failed' }
$env:RUSTUP_TOOLCHAIN = '1.97.1'

Invoke-Checked 'rustfmt failed' { cargo fmt --all -- --check }
Invoke-Checked 'database native Clippy failed' { cargo clippy -p vsn-database-native --all-targets --locked -- -D warnings }
Invoke-Checked 'database CLI Clippy failed' { cargo clippy -p vsn-database-cli --all-targets --locked -- -D warnings }
Invoke-Checked 'database native tests failed' { cargo test -p vsn-database-native --locked }
Invoke-Checked 'database CLI tests failed' { cargo test -p vsn-database-cli --locked }
Invoke-Checked '02.26 transport-boundary regression failed' { cargo test -p vsn-database-native --locked --test pkg02_0226_transport_boundaries }

$cliSource = Get-Content 'crates/vsn-database-cli/src/lib.rs' -Raw
foreach ($engine in @('Postgresql','Mysql','Mariadb','Mongo','Redis')) {
    if (-not $cliSource.Contains("Engine::$engine")) { throw "client detection is missing $engine" }
}
foreach ($client in @('psql','mysql','mariadb','mongosh','redis-cli')) {
    if (-not $cliSource.Contains("\"$client\"")) { throw "client mapping is missing $client" }
}

$nativeSource = Get-Content 'crates/vsn-database-native/src/lib.rs' -Raw
foreach ($needle in @('postgres_tls_inspect','mysql_tls_inspect','rediss://','mongodb+srv://')) {
    if (-not $nativeSource.Contains($needle)) { throw "declared TLS/fail-closed capability marker missing: $needle" }
}

$source = (git rev-parse HEAD).Trim()
$evidence = [ordered]@{
    schema_version = 1
    package_id = 'PKG-02'
    task_id = $Task
    source_commit = $source
    runner_name = $env:RUNNER_NAME
    runner_environment = $env:RUNNER_ENVIRONMENT
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    rust_version = (& rustc --version).Trim()
    client_detection_declares_all_beta_engines = $true
    native_loopback_spoof_regressions_pass = $true
    declared_tls_profiles_present = $true
    unsupported_capabilities_fail_closed = $true
}
$evidence | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $EvidenceRoot 'evidence.json') -Encoding utf8
Get-FileHash (Join-Path $EvidenceRoot 'evidence.json') -Algorithm SHA256 | ForEach-Object Hash | Set-Content (Join-Path $EvidenceRoot 'evidence.json.sha256')
