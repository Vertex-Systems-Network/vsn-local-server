param(
    [switch]$StrictAdvisories,
    [switch]$StrictUnusedDependencies,
    [switch]$StrictNpmAudit
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Command
    )
    Write-Host "==> $Name"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit $LASTEXITCODE"
    }
}

function Invoke-ReportOnly {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Command,
        [Parameter(Mandatory = $true)][bool]$Strict
    )
    Write-Host "==> $Name"
    & $Command
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        if ($Strict) { throw "$Name failed with exit $code" }
        Write-Warning "$Name reported issues (exit=$code); this check is report-only unless its strict switch is enabled."
    }
}

$rustc = (& rustc --version).Trim()
$cargo = (& cargo --version).Trim()
if ($rustc -notmatch '^rustc 1\.97\.1\b') { throw "Rust 1.97.1 is required; got: $rustc" }
if ($cargo -notmatch '^cargo 1\.97\.1\b') { throw "Cargo 1.97.1 is required; got: $cargo" }

Invoke-Checked 'Rust format' { cargo fmt --all -- --check }
Invoke-Checked 'Rust Clippy' { cargo clippy --workspace --all-targets --locked -- -D warnings }
Invoke-Checked 'Rust workspace tests' { cargo test --workspace --locked }

if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    Invoke-Checked 'Cargo dependency bans/sources' { cargo deny --all-features --locked check bans sources }
    Invoke-ReportOnly 'RustSec advisories' { cargo deny --all-features --locked check advisories } $StrictAdvisories.IsPresent
} else {
    throw 'cargo-deny is required. Install the repository-pinned/current approved cargo-deny before running the local quality gate.'
}

if (Get-Command cargo-machete -ErrorAction SilentlyContinue) {
    Invoke-ReportOnly 'Unused Rust dependencies' { cargo machete } $StrictUnusedDependencies.IsPresent
} else {
    throw 'cargo-machete is required. Install the repository-approved cargo-machete before running the local quality gate.'
}

if (Get-Command actionlint -ErrorAction SilentlyContinue) {
    Invoke-Checked 'GitHub Actions lint' { actionlint -color }
} else {
    Write-Warning 'actionlint is not installed locally; CI still enforces workflow linting with the pinned actionlint container.'
}

Push-Location 'apps/desktop'
try {
    Invoke-Checked 'Desktop locked install' { npm ci }
    Invoke-Checked 'Desktop strict TypeScript' { npm run typecheck }
    Invoke-ReportOnly 'Desktop production dependency audit' { npm audit --omit=dev --audit-level=high } $StrictNpmAudit.IsPresent
    Invoke-Checked 'Desktop production build' { npm run build }
}
finally {
    Pop-Location
}

Write-Host 'Code quality gate completed.'
