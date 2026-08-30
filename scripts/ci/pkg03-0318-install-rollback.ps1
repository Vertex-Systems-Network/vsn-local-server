param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.18'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33332277213 / artifact 9738310176 proved the prior successful
# machine-security isolation correction and then exposed a narrower NSIS harness
# defect. The current-user forced-failure candidate advanced through two real
# wizard Next actions and exposed an NSIS error dialog stating "Error opening
# file for writing" for the exact owned target VSN Dev Platform.exe, with
# Abort/Retry/Ignore controls. That is direct evidence the installer entered its
# write/destructive transaction. The generic driver nevertheless waited for a
# literal button named Install, which NSIS does not expose on this path, and timed
# out while the genuine failure dialog remained visible.
#
# This outer shim pins the exact prior harness and changes only positive-start
# classification for NSIS forced-failure: an explicit target-write failure on the
# exact owned Desktop path is accepted as stronger transaction-start evidence and
# Abort is used to terminate that already-failed transaction. MSI still requires
# its existing positive Install path. Failed-attempt residue, security-state
# absence, nonzero exit, rollback, recovery, duplicate identity, protected state,
# final cleanup and zero-drift assertions remain unchanged. Product/installer
# behavior is untouched.
# Frozen witnesses: forced_failure_after_positive_install_invocation
# partial_owned_state_forbidden interrupted_install_positive_start_required
# exact_candidate_rerun_recovery_required duplicate_identity_forbidden
# protected_state_nonmutation_required tracked_repository_drift_zero /fa
# runner-isolation-security-reset-after-successful-machine-lifecycle
# failed_attempt_residue=$false

$PriorCommit='011b3231ec06ca3a1a454fd2451e84ff9b6bfd27'
$PriorPath='scripts/ci/pkg03-0318-install-rollback.ps1'
$ExpectedPriorBlob='965368b1b416ae9cabcbdb31eccb5de21ad8d655'

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
  'failed_attempt_residue=$false','Never cancel a healthy in-progress transaction'
)){
  if(-not $source.Contains($token)){throw "03.18 pinned prior harness missing token: $token"}
}

$marker='$newFailureBranch = @' + [char]39
$start=$source.IndexOf($marker)
if($start -lt 0){throw '03.18 new failure-driver marker missing.'}
$prefix=$source.Substring(0,$start)
$tail=$source.Substring($start)
$old=@'
      if(-not $transactionStarted){
        $clicked=Invoke-Button $Phase $window @('^Install$','^Next\b') $false
        if($clicked -match '(?i)^Install$'){$transactionStarted=$true}
      } else {
'@.Replace("`r`n","`n")
if(([regex]::Matches($tail,[regex]::Escape($old))).Count -ne 1){
  throw '03.18 NSIS positive-start patch boundary mismatch.'
}
$new=@'
      if(-not $transactionStarted){
        # NSIS does not expose a literal Install control on every path. A visible
        # write-error dialog naming the exact owned Desktop target proves the
        # destructive file-write transaction has already started; this is
        # stronger evidence than a navigation-button click and cannot be created
        # by the harness without the installer attempting the owned write.
        $preStartNames=@(Get-SafeName $window)
        foreach($type in @([System.Windows.Automation.ControlType]::Text,[System.Windows.Automation.ControlType]::Button)){
          foreach($element in @(Get-Controls $window $type)){
            try{$name=Get-SafeName $element;if($name){$preStartNames+=$name}}catch{}
          }
        }
        $preStartSurface=($preStartNames -join ' | ')
        $nsisTargetWriteFailure=($Phase -match '(?i)^nsis-') -and ($preStartSurface -match '(?is)Error opening file for writing:\s*.*VSN Dev Platform\.exe.*Click Abort')
        if($nsisTargetWriteFailure){
          $transactionStarted=$true
          [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='positive-transaction-start-target-write-attempt';target='VSN Dev Platform.exe';proof='NSIS Error opening file for writing';at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
          $clicked=Invoke-Button $Phase $window @('^Abort$') $true
          if($clicked){$terminalAction=$true}
        } else {
          $clicked=Invoke-Button $Phase $window @('^Install$','^Next\b') $false
          if($clicked -match '(?i)^Install$'){$transactionStarted=$true}
        }
      } else {
'@.Replace("`r`n","`n")
$patched=$prefix+$tail.Replace($old,$new)
foreach($token in @('positive-transaction-start-target-write-attempt','Error opening file for writing','^nsis-','^Install$')){
  if(-not $patched.Contains($token)){throw "03.18 positive-start patch missing token: $token"}
}

$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$runtime=Join-Path $tempRoot 'pkg03-0318-nsis-positive-start-wrapper-runtime.ps1'
[IO.File]::WriteAllText($runtime,$patched,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtime,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){$errors|ForEach-Object{Write-Host $_.Message};throw "03.18 positive-start wrapper runtime has $($errors.Count) parse error(s)."}

& $runtime `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
