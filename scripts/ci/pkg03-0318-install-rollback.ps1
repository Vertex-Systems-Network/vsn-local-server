param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.18'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33335548668 / job 99321680359 / independently verified
# artifact 9739305919 isolated one certification defect in the current-user NSIS
# interrupted-install lane. NSIS exposes no literal Install control on this path:
# the driver invoked three real Next actions, package-created state reached the
# frozen PositiveStart predicate, and the UI reached Finish. The inherited driver
# evaluated PositiveStart only behind $installInvoked, so it ignored that stronger
# runtime witness and looped until timeout.
#
# Preserve the exact prior harness and every rollback/recovery assertion. For
# NSIS interrupted-install only, allow the already-frozen PositiveStart predicate
# (owned Desktop payload OR ARP registration materialized by the exact candidate)
# to prove transaction start when no literal Install control exists. MSI retains
# the literal Install requirement. Interruption is still injected only after that
# positive witness. Product/runtime/installer behavior is untouched.

$PriorCommit='4cba4bbb8ec5217f7a8767f17bc85e86a272bba7'
$PriorPath='scripts/ci/pkg03-0318-install-rollback.ps1'
$ExpectedPriorBlob='6cb964a09390a2d0889e77bcf1cf88408c35b444'

$blob=(& git rev-parse "${PriorCommit}:${PriorPath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedPriorBlob){
  throw "03.18 prior harness blob mismatch: expected=$ExpectedPriorBlob actual=$blob"
}
$source=(& git show "${PriorCommit}:${PriorPath}"|Out-String).Replace("`r`n","`n")
if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)){throw '03.18 failed to load pinned prior harness.'}
foreach($token in @(
  'forced_failure_after_positive_install_invocation','partial_owned_state_forbidden',
  'interrupted_install_positive_start_required','exact_candidate_rerun_recovery_required',
  'duplicate_identity_forbidden','protected_state_nonmutation_required',
  'tracked_repository_drift_zero','/fa',
  'runner-isolation-security-reset-after-successful-machine-lifecycle',
  'failed_attempt_residue=$false','positive-transaction-start-target-write-attempt'
)){
  if(-not $source.Contains($token)){throw "03.18 pinned prior harness missing token: $token"}
}

$anchor='$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}'
if(([regex]::Matches($source,[regex]::Escape($anchor))).Count -ne 1){
  throw '03.18 interrupted-start insertion boundary mismatch.'
}
$insertion=@'
$oldInterruptedGate='    if($installInvoked -and [bool](& $PositiveStart)){$positive=$true;break}'
if(([regex]::Matches($patched,[regex]::Escape($oldInterruptedGate))).Count -ne 1){
  throw '03.18 interrupted positive-start gate mismatch.'
}
$newInterruptedGate=@(
'    $positiveNow=[bool](& $PositiveStart)',
'    if($installInvoked -and $positiveNow){$positive=$true;break}',
'    if((-not $installInvoked) -and ($Phase -match ''(?i)^nsis-'') -and $positiveNow){',
'      # NSIS may execute the owned write transaction from its final Next action',
'      # without ever exposing a literal Install button. The frozen PositiveStart',
'      # predicate observes package-created owned payload/ARP state.',
'      $installInvoked=$true',
'      $positive=$true',
'      [void]$Actions.Add([pscustomobject][ordered]@{',
'        phase=$Phase',
'        action=''positive-transaction-start-without-literal-install-control''',
'        proof=''frozen-positive-start-owned-payload-or-arp''',
'        at_utc=[DateTime]::UtcNow.ToString(''o'')',
'      })',
'      Write-UiEvidence',
'      break',
'    }'
) -join "`n"
$patched=$patched.Replace($oldInterruptedGate,$newInterruptedGate)
foreach($required in @('positive-transaction-start-without-literal-install-control','frozen-positive-start-owned-payload-or-arp','(?i)^nsis-')){
  if(-not $patched.Contains($required)){throw "03.18 interrupted-start patch missing token: $required"}
}
'@.Replace("`r`n","`n")
$outerPatched=$source.Replace($anchor,$insertion+$anchor)

$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$outerRuntime=Join-Path $tempRoot 'pkg03-0318-interrupted-positive-start-wrapper-runtime.ps1'
[IO.File]::WriteAllText($outerRuntime,$outerPatched,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($outerRuntime,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){$errors|ForEach-Object{Write-Host $_.Message};throw "03.18 outer wrapper has $($errors.Count) parse error(s)."}

& $outerRuntime `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
