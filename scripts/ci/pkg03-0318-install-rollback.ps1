param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.18'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness correction. Exact-head run 33309553743 proved
# authority, parser and all candidate builds pass, then failed before rollback
# execution because the harness referenced the local ref `main` in a detached
# checkout. Pin the reused, already accepted 03.15 helper source to the frozen
# canonical activation SHA instead. No helper semantics or 03.18 acceptance
# assertions are changed.

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

$old = 'git show "main:scripts/ci/pkg03-0315-installer-diagnostics.ps1"'
$new = 'git show "' + $CanonicalBase + ':scripts/ci/pkg03-0315-installer-diagnostics.ps1"'
$count = [regex]::Matches($source,[regex]::Escape($old)).Count
if ($count -ne 1) { throw "03.18 canonical helper patch boundary mismatch: expected 1, found $count" }
$patched = $source.Replace($old,$new)

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
