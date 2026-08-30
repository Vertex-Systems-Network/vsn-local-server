param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.18'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness correction. Exact-head evidence already proved
# authority, parser and candidate builds. The frozen harness has three runtime
# environment defects only: a detached-checkout `main:` helper reference, a
# helper-end marker whose first occurrence is inside Write-UiEvidence (therefore
# producing an incomplete function substring), and a $PSScriptRoot snapshot
# reference that becomes invalid after the runtime harness is emitted into
# RUNNER_TEMP. Pin helper authority to canonical SHA, cut at the unique accepted
# 03.15 execution-start boundary, and resolve the snapshot helper from repo root.
# No rollback/recovery acceptance assertion or product/installer behavior changes.

$BaseCommit = '44de00281203f3c737bd847ae53b548ce17a3386'
$BasePath = 'scripts/ci/pkg03-0318-install-rollback.ps1'
$ExpectedBaseBlob = 'afdc5eedd4438a21ee423bc33546c02cb62d46f3'
$CanonicalBase = 'f3afb66e588d01ff2e8cb37273ad413862a4edaf'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.18 pinned harness blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}

$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n","`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.18 failed to load pinned harness from Git history.'
}

$oldAuthority = 'git show "main:scripts/ci/pkg03-0315-installer-diagnostics.ps1"'
$newAuthority = 'git show "' + $CanonicalBase + ':scripts/ci/pkg03-0315-installer-diagnostics.ps1"'
$count = [regex]::Matches($source,[regex]::Escape($oldAuthority)).Count
if ($count -ne 1) { throw "03.18 canonical helper authority patch mismatch: expected 1, found $count" }
$patched = $source.Replace($oldAuthority,$newAuthority)

$oldBoundary = '$helperEnd = $helperSource.IndexOf(''New-Item -ItemType Directory -Force $EvidencePath | Out-Null'', $helperStart)'
$newBoundary = '$helperEnd = $helperSource.IndexOf(''$actualHead=(git rev-parse HEAD).Trim()'', $helperStart)'
$count = [regex]::Matches($patched,[regex]::Escape($oldBoundary)).Count
if ($count -ne 1) { throw "03.18 complete-helper boundary patch mismatch: expected 1, found $count" }
$patched = $patched.Replace($oldBoundary,$newBoundary)

$oldSnapshot = ". (Join-Path `$PSScriptRoot 'pkg03-0313-snapshot.ps1')"
$newSnapshot = ". (Join-Path (Get-Location) 'scripts/ci/pkg03-0313-snapshot.ps1')"
$count = [regex]::Matches($patched,[regex]::Escape($oldSnapshot)).Count
if ($count -ne 1) { throw "03.18 snapshot runtime-path patch mismatch: expected 1, found $count" }
$patched = $patched.Replace($oldSnapshot,$newSnapshot)

foreach ($token in @(
  'forced_failure_after_positive_install_invocation',
  'partial_owned_state_forbidden',
  'interrupted_install_positive_start_required',
  'exact_candidate_rerun_recovery_required',
  'duplicate_identity_forbidden',
  'protected_state_nonmutation_required',
  'tracked_repository_drift_zero'
)) {
  if (-not $patched.Contains($token)) { throw "03.18 patched harness missing frozen acceptance token: $token" }
}

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeHarness = Join-Path $tempRoot 'pkg03-0318-install-rollback-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness,$patched,[Text.UTF8Encoding]::new($false))

$tokens=$null
$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.18 patched runtime harness has $($errors.Count) parse error(s)."
}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
