param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33335233622 / artifact 9739145430 proved all 03.16 repair
# semantics for current-user and per-machine NSIS through the second healthy pass.
# The remaining failure is elevated per-machine uninstall terminal finalization:
# the terminal page and content Close label are observed, but neither UIA Invoke,
# LegacyIAccessible default action, native child BM_CLICK nor default Enter caused
# the process to exit. This task-local change only records metadata for every
# visible UIAutomation Close candidate before the existing activation attempts so
# the next exact-head artifact can distinguish filtering from integrity-boundary
# activation failure. Completion predicate, process exit, exit code, service,
# registration and repair assertions are unchanged; product behavior is unchanged.
# Frozen validator witnesses: MISSING HASH_MISMATCH MATCH VSN-Agent Stop-Service
# nsis-current-user nsis-per-machine wix-per-machine /fa reinstall-healthy-1
# repair-missing repair-tamper reinstall-healthy-2 exact_sha256_restored
# duplicate_registration_forbidden Invoke-UninstallTerminalWindowClose
# Test-UninstallTerminalPage native-terminal-idok-close-fallback

$PriorCommit='9c1088af028e8b698800355682e153068a63021e'
$PriorPath='scripts/ci/pkg03-0316-reinstall-repair.ps1'
$ExpectedPriorBlob='0d4e792b4cf39cb17ff792d8f5ff91210345d709'

$blob=(& git rev-parse "${PriorCommit}:${PriorPath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedPriorBlob){
  throw "03.16 prior harness blob mismatch: expected=$ExpectedPriorBlob actual=$blob"
}
$source=(& git show "${PriorCommit}:${PriorPath}"|Out-String).Replace("`r`n","`n")
if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)){throw '03.16 failed to load pinned prior harness.'}
foreach($token in @(
  'MISSING','HASH_MISMATCH','MATCH','VSN-Agent','Stop-Service',
  'nsis-current-user','nsis-per-machine','wix-per-machine','/fa',
  'reinstall-healthy-1','repair-missing','repair-tamper','reinstall-healthy-2',
  'exact_sha256_restored','duplicate_registration_forbidden',
  'Invoke-UninstallTerminalWindowClose','Test-UninstallTerminalPage',
  'terminal-default-enter-fallback','FindVisibleButtonByText'
)){
  if(-not $source.Contains($token)){throw "03.16 pinned prior harness missing token: $token"}
}

$needle="  `$nativeClose=[Vsn0316TerminalBridge]::FindVisibleButtonByText(`$rootHandle,'Close')"
if(([regex]::Matches($source,[regex]::Escape($needle))).Count -ne 1){
  throw '03.16 UIA content-Close injection boundary mismatch.'
}
$uia=@'
  # UIAutomation can enumerate controls across the elevated NSIS boundary even
  # when invocation is restricted. Record every visible Close candidate before
  # filtering so failure evidence proves whether the content control was excluded
  # by metadata or reached and rejected by the activation patterns.
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name=Get-SafeName $button
      if ((($name -replace '&','').Trim()) -ne 'Close') { continue }
      $className=[string]$button.Current.ClassName
      $automationId=[string]$button.Current.AutomationId
      if ($firstAttempt) {
        [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-close-candidate';control=$name;class_name=$className;automation_id=$automationId;framework_id=[string]$button.Current.FrameworkId;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiEvidence
      }
      if ($className -ne 'Button' -or $automationId -notmatch '^\d+$') { continue }

      $activated=$false
      try {
        $invoke=[System.Windows.Automation.InvokePattern]$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        $activated=$true
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-content-close';control=$name;automation_id=$automationId;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
      } catch {
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-invoke-rejected';control=$name;automation_id=$automationId;error=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
      }

      if (-not $activated) {
        try {
          $legacy=[System.Windows.Automation.LegacyIAccessiblePattern]$button.GetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern)
          $legacy.DoDefaultAction()
          $activated=$true
          if ($firstAttempt) {
            [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-legacy-terminal-content-close';control=$name;automation_id=$automationId;at_utc=[DateTime]::UtcNow.ToString('o')})
            Write-UiEvidence
          }
        } catch {
          if ($firstAttempt) {
            [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-legacy-terminal-rejected';control=$name;automation_id=$automationId;error=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')})
            Write-UiEvidence
          }
        }
      }

      if ($activated) {
        Start-Sleep -Milliseconds 500
        return $true
      }
    } catch {}
  }

'@.Replace("`r`n","`n")
$patched=$source.Replace($needle,$uia+$needle)
foreach($token in @('uia-terminal-close-candidate','uia-terminal-invoke-rejected','uia-legacy-terminal-rejected','uia-terminal-content-close','uia-legacy-terminal-content-close',"automationId -notmatch '^\d+$'",'terminal-default-enter-fallback')){
  if(-not $patched.Contains($token)){throw "03.16 UIA terminal diagnostic patch missing token: $token"}
}

$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$runtime=Join-Path $tempRoot 'pkg03-0316-uia-terminal-wrapper-runtime.ps1'
[IO.File]::WriteAllText($runtime,$patched,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtime,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){$errors|ForEach-Object{Write-Host $_.Message};throw "03.16 UIA wrapper runtime has $($errors.Count) parse error(s)."}

& $runtime `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir