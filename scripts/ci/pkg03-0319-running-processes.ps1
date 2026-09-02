param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.19'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Evidence-bounded outer shim over the exact previous 03.19 certification
# wrapper. Exact-head run 33565504847 / job 100047642349 / failure artifact
# 9823708728 (sha256:338e82faa3147a3d6680a3e10c5a3f12278d2eeee20500f86302d5b27850c380)
# proved both NSIS lanes complete their deterministic safe-block / operator-
# cleanup / retry lifecycles and WiX installation establishes the intended live
# Desktop, CLI and Agent resources. The WiX uninstall then invoked its real
# confirmation Yes control and reached the native Restart Manager files-in-use
# dialog, but the frozen state machine did not mark that Yes confirmation as the
# uninstall operation having started. Consequently the coordination branch that
# drives non-destructive Cancel was unreachable and the run exhausted its bound.
#
# Patch only that certification state transition: for the operation-start gate,
# treat the already-invoked WiX Yes confirmation as equivalent to NSIS
# Uninstall/Remove. Product/installer behavior, running product processes,
# Restart Manager policy, safe-block assertions, operator cleanup ordering,
# snapshot semantics and all accepted window filtering remain unchanged.

$PriorCommit='53d7005e2b093906e800b93fccb993a6e87b6c53'
$PriorPath='scripts/ci/pkg03-0319-running-processes.ps1'
$ExpectedPriorBlob='a6bf69613df33fa28618842b5708aa517cdada00'

$blob=(& git rev-parse "${PriorCommit}:${PriorPath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedPriorBlob){
  throw "03.19 previous outer-wrapper blob mismatch: expected=$ExpectedPriorBlob actual=$blob"
}
$outer=(& git show "${PriorCommit}:${PriorPath}"|Out-String).Replace("`r`n","`n")
if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($outer)){
  throw '03.19 failed to load pinned previous outer wrapper.'
}
foreach($token in @(
  'harness_pre_kill=$false',
  'operator-cleanup-after-proven-block',
  'Vsn0319AdmittedUninstallHandlesByRoot',
  'terminal-window-fallback',
  '$patchedWrapper=$wrapper.Replace($boundary,$semanticPatch+$boundary)'
)){
  if(-not $outer.Contains($token)){throw "03.19 previous outer wrapper missing evidence token: $token"}
}

$boundary='$patchedWrapper=$wrapper.Replace($boundary,$semanticPatch+$boundary)'
if(([regex]::Matches($outer,[regex]::Escape($boundary))).Count -ne 1){
  throw '03.19 WiX operation-state injection boundary mismatch.'
}

$statePatch=@'
# Exact-head 33565504847 proved WiX starts uninstall through a confirmation Yes
# before Restart Manager presents its files-in-use dialog. The frozen harness
# only considered NSIS Uninstall/Remove as operation-start controls, leaving the
# already-evidenced coordination Cancel branch unreachable. Extend only that
# state transition; do not click Restart Manager OK, terminate product
# processes, or weaken the deterministic safe-block assertions.
$operationStartOld='if($clicked -match ''(?i)^(Uninstall|Remove)$''){$operationInvoked=$true}'
$operationStartNew='if($clicked -match ''(?i)^(Uninstall|Remove|Yes)$''){$operationInvoked=$true}'
if(([regex]::Matches($patchedWrapper,[regex]::Escape($operationStartOld))).Count -ne 1){
  throw '03.19 expected exactly one frozen operation-start gate.'
}
$patchedWrapper=$patchedWrapper.Replace($operationStartOld,$operationStartNew)
if(-not $patchedWrapper.Contains($operationStartNew)){
  throw '03.19 WiX Yes operation-start certification patch was not applied.'
}
'@.Replace("`r`n","`n")

$patchedOuter=$outer.Replace($boundary,$boundary+"`n"+$statePatch)
$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$runtimeOuter=Join-Path $tempRoot 'pkg03-0319-running-processes-outer-statefix.ps1'
[IO.File]::WriteAllText($runtimeOuter,$patchedOuter,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeOuter,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){
  $errors|ForEach-Object{Write-Host $_.Message}
  throw "03.19 state-fix outer wrapper has $($errors.Count) parse error(s)."
}

& $runtimeOuter `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
