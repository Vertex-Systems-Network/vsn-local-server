param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# PKG-03 03.16 — Amendment 009 diagnostic overlay.
#
# Exact A008 head db80a67555d614dfdaaff87a74a50ffd1ca150de crossed the
# previously blocked nsis-per-machine uninstall boundary in GitHub-hosted run
# 33429865150 / job 99612339620, then failed at the newly exposed
# wix-per-machine initial-install completion boundary. Failure artifact
# 9772949341 was independently byte-verified at SHA-256
# 727ecab6981eda25e2e0255603aed2c14abc3683e1e2b512fc6c32f052e0773c.
# UI evidence records the real WiX Install action followed immediately by an OK
# dialog and Finish, while Program Files payload + MSI ARP never became present.
#
# This overlay changes no product input, completion predicate, timeout, UI mode,
# lifecycle action, repair assertion or acceptance rule. It pins the exact A008
# certification wrapper and adds only verbose Windows Installer logging to the
# initial WiX install invocation so the next failure artifact contains the native
# MSI/custom-action cause instead of requiring a speculative product mutation.
#
# Frozen validator witnesses retained by this overlay:
# MISSING HASH_MISMATCH MATCH VSN-Agent Stop-Service
# nsis-current-user nsis-per-machine wix-per-machine /fa
# reinstall-healthy-1 repair-missing repair-tamper reinstall-healthy-2
# exact_sha256_restored duplicate_registration_forbidden

$BaseCommit = 'db80a67555d614dfdaaff87a74a50ffd1ca150de'
$BasePath = 'scripts/ci/pkg03-0316-reinstall-repair.ps1'
$ExpectedBaseBlob = 'a681400d8ba5668241420103fa6cb37c538108fc'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.16 A009 base wrapper blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}
$wrapper = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($wrapper)) {
  throw '03.16 A009 failed to load pinned A008 wrapper.'
}

$anchor = '$patched=$source'
$anchorCount = [regex]::Matches($wrapper,[regex]::Escape($anchor)).Count
if ($anchorCount -ne 1) {
  throw "03.16 A009 injection boundary mismatch: expected 1, found $anchorCount"
}

$diagnosticPatch = @'
$oldWixInitial=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('ICB7IFN0YXJ0LVByb2Nlc3MgLUZpbGVQYXRoICRtc2lleGVjIC1Bcmd1bWVudExpc3QgQCgnL2knLCgnInswfSInIC1mICRNc2lQYXRoKSkgLVBhc3NUaHJ1IH0='))
$newWixInitial=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('ICB7ICRpbml0aWFsTG9nPUpvaW4tUGF0aCAkRXZpZGVuY2VQYXRoICd3aXgtcGVyLW1hY2hpbmUtaW5pdGlhbC1pbnN0YWxsLmxvZyc7IFN0YXJ0LVByb2Nlc3MgLUZpbGVQYXRoICRtc2lleGVjIC1Bcmd1bWVudExpc3QgQCgnL2knLCgnInswfSInIC1mICRNc2lQYXRoKSwnL2wqdicsKCciezB9IicgLWYgJGluaXRpYWxMb2cpKSAtUGFzc1RocnUgfQ=='))
$wixInitialCount=[regex]::Matches($source,[regex]::Escape($oldWixInitial)).Count
if ($wixInitialCount -ne 1) {
  throw "03.16 A009 WiX initial-install boundary mismatch: expected 1, found $wixInitialCount"
}
$source=$source.Replace($oldWixInitial,$newWixInitial)
if (-not $source.Contains('wix-per-machine-initial-install.log') -or -not $source.Contains('/l*v')) {
  throw '03.16 A009 verbose initial-install logging was not injected.'
}
$patched=$source
'@.Replace("`r`n", "`n")

$patchedWrapper = $wrapper.Replace($anchor,$diagnosticPatch)
$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtime = Join-Path $tempRoot 'pkg03-0316-a009-wrapper.ps1'
[IO.File]::WriteAllText($runtime,$patchedWrapper,[Text.UTF8Encoding]::new($false))
$tokens=$null; $errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtime,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.16 A009 wrapper has $($errors.Count) parse error(s)."
}

& $runtime `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
