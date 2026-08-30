param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded wrapper correction over exact head 8f9d20f.
#
# Run 33312261071 proved authority/parser and all three package builds are green,
# then stopped before lifecycle execution because the outer wrapper required a
# completion-assertion literal from e30870f even though that assertion lives in
# e30870f's pinned canonical base c754599. This wrapper keeps the guard, but binds
# it to the artifact that actually owns it. The exact previous wrapper and exact
# nested base are both immutable blob-pinned before any replacement is made.
# Product/runtime/installer behavior and all repair acceptance assertions remain
# unchanged.

$PreviousCommit = '8f9d20f5c4b3f6d5055424e43c5712e3e315adbc'
$PreviousBlob = '8110af16f7511373385b7f7f61128680cfabc67d'
$NestedBaseCommit = 'c754599a42ee44b1bb3b6d41edbf783d2146a985'
$NestedBaseBlob = 'aa054f97309407f394bd2a87297d3d6428794711'
$BasePath = 'scripts/ci/pkg03-0316-reinstall-repair.ps1'

$previousObserved = (& git rev-parse "${PreviousCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $previousObserved -ne $PreviousBlob) {
  throw "03.16 previous wrapper blob mismatch: expected=$PreviousBlob actual=$previousObserved"
}
$source = (& git show "${PreviousCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.16 failed to load the pinned previous wrapper.'
}

# These are stable source-level witnesses required by the frozen validator and
# by the accepted repair lifecycle. They must still exist in the previous exact
# wrapper before this compatibility correction is applied.
$RequiredFrozenTokens = @(
  'MISSING',
  'HASH_MISMATCH',
  'MATCH',
  'VSN-Agent',
  'Stop-Service',
  'nsis-current-user',
  'nsis-per-machine',
  'wix-per-machine',
  '/fa',
  'reinstall-healthy-1',
  'repair-missing',
  'repair-tamper',
  'reinstall-healthy-2',
  'exact_sha256_restored',
  'duplicate_registration_forbidden',
  'native-terminal-idok-close-fallback',
  'Invoke-UninstallTerminalWindowClose',
  'Test-UninstallTerminalPage'
)
foreach ($token in $RequiredFrozenTokens) {
  if (-not $source.Contains($token)) {
    throw "03.16 pinned previous wrapper missing frozen token: $token"
  }
}

# The completion-state assertion is owned by the canonical executable harness
# nested under e30870f, not by e30870f itself. Verify that exact nested Git blob
# and assertion directly so the acceptance guard is preserved rather than
# deleted or weakened.
$nestedObserved = (& git rev-parse "${NestedBaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $nestedObserved -ne $NestedBaseBlob) {
  throw "03.16 nested canonical harness blob mismatch: expected=$NestedBaseBlob actual=$nestedObserved"
}
$nestedSource = (& git show "${NestedBaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($nestedSource)) {
  throw '03.16 failed to load nested canonical harness.'
}
$CompletionAssertion = 'Assert-Condition ([bool](& $Completion))'
if (-not $nestedSource.Contains($CompletionAssertion)) {
  throw '03.16 nested canonical harness no longer contains the required completion-state assertion.'
}

# 8f9d20f incorrectly asks e30870f itself to contain the nested assertion. The
# stale literal is replaced only in that wrapper-level token list. The nested
# assertion has already been independently blob-bound above and remains in the
# executable canonical harness.
$StaleNestedToken = 'Assert-Condition ([bool](& `$Completion))'
$count = [regex]::Matches($source, [regex]::Escape($StaleNestedToken)).Count
if ($count -ne 1) {
  throw "03.16 stale nested-token patch boundary mismatch: expected exactly one match, found $count"
}
$patched = $source.Replace($StaleNestedToken, 'duplicate_registration_forbidden')

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeHarness = Join-Path $tempRoot 'pkg03-0316-reinstall-repair-nested-guard-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness, $patched, [Text.UTF8Encoding]::new($false))

$tokens=$null
$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.16 nested-guard runtime has $($errors.Count) parse error(s)."
}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
