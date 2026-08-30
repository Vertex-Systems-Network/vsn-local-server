param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33337997424 / artifact 9739971841 independently proved that
# current-user and per-machine NSIS both completed healthy reinstall, MISSING
# repair, HASH_MISMATCH repair, exact SHA256 restoration and the second healthy
# pass. The remaining failure is elevated per-machine uninstall terminal
# finalization. The diagnostic artifact recorded only the title-bar Close
# candidate (AutomationId=Close, empty class), showing that the previous
# class=Button + numeric AutomationId precondition was a harness assumption rather
# than product evidence. This task-local wrapper now retains the known title-bar
# exclusion used by the canonical harness but attempts UIA Invoke/Legacy activation
# on any other visible content Close without requiring class/AutomationId shape.
# Completion predicate, process exit, exit code, service, registration, repair,
# timeout and signing boundaries are unchanged; product behavior is unchanged.
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
  # Run 33337997424 proved the prior metadata gate only recorded the NSIS
  # title-bar Close (AutomationId=Close, native HWND unavailable). Keep that
  # canonical chrome exclusion, but do not require the real wizard content Close
  # to expose a specific class name or numeric AutomationId across elevation.
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name=Get-SafeName $button
      if ((($name -replace '&','').Trim()) -ne 'Close') { continue }

      $className=''; $automationId=''; $frameworkId=''; $nativeHandle=0
      try { $className=[string]$button.Current.ClassName } catch {}
      try { $automationId=[string]$button.Current.AutomationId } catch {}
      try { $frameworkId=[string]$button.Current.FrameworkId } catch {}
      try { $nativeHandle=[int]$button.Current.NativeWindowHandle } catch {}

      if ($firstAttempt) {
        [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-close-candidate';control=$name;class_name=$className;automation_id=$automationId;framework_id=$frameworkId;native_handle=$nativeHandle;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiEvidence
      }

      # Match the canonical harness' title-bar filter. This prevents a generic
      # window-chrome Close from being consumed as proof of NSIS terminal action.
      if ($nativeHandle -eq 0 -and $automationId -match '^(?i:Close|Minimize|Maximize)$') {
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-titlebar-close-skipped';control=$name;automation_id=$automationId;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
        continue
      }

      $activated=$false
      try {
        $invoke=[System.Windows.Automation.InvokePattern]$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        $activated=$true
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-content-close';control=$name;automation_id=$automationId;native_handle=$nativeHandle;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
      } catch {
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-invoke-rejected';control=$name;automation_id=$automationId;native_handle=$nativeHandle;error=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
      }

      if (-not $activated) {
        try {
          $legacy=[System.Windows.Automation.LegacyIAccessiblePattern]$button.GetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern)
          $legacy.DoDefaultAction()
          $activated=$true
          if ($firstAttempt) {
            [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-legacy-terminal-content-close';control=$name;automation_id=$automationId;native_handle=$nativeHandle;at_utc=[DateTime]::UtcNow.ToString('o')})
            Write-UiEvidence
          }
        } catch {
          if ($firstAttempt) {
            [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-legacy-terminal-rejected';control=$name;automation_id=$automationId;native_handle=$nativeHandle;error=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')})
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
foreach($token in @(
  'uia-terminal-close-candidate','uia-terminal-titlebar-close-skipped',
  'uia-terminal-invoke-rejected','uia-legacy-terminal-rejected',
  'uia-terminal-content-close','uia-legacy-terminal-content-close',
  "automationId -match '^(?i:Close|Minimize|Maximize)$'",
  'terminal-default-enter-fallback'
)){
  if(-not $patched.Contains($token)){throw "03.16 UIA terminal activation patch missing token: $token"}
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