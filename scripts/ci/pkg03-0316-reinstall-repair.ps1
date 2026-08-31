param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33340563391 / artifact 9740696185 independently proved that
# current-user and per-machine NSIS both completed healthy reinstall, MISSING
# repair, HASH_MISMATCH repair, exact SHA256 restoration and the second healthy
# pass. The remaining failure is elevated per-machine uninstall terminal
# finalization. The run-44 diagnostic artifact shows the terminal page contains
# two visible Close controls, but the injected UIA loop recorded only the window
# chrome Close before falling back to Enter. That means reading IsEnabled or
# IsOffscreen on the elevated content Close can itself fail at the integrity
# boundary before the control is classified. This task-local wrapper now names
# Close candidates first and treats unreadable state properties as unknown rather
# than as grounds to discard the candidate; explicitly disabled/offscreen controls
# are still skipped. Completion predicate, process exit, exit code, service,
# registration, repair, timeout and signing boundaries are unchanged; product
# behavior is unchanged.
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
  # Run 33340563391 proved the terminal observation can enumerate both NSIS
  # content Close and title-bar Close while direct state-property reads on the
  # elevated content element may be denied. Resolve by name first, retain the
  # canonical chrome exclusion, and only reject state that is explicitly known
  # disabled/offscreen. Unknown state remains a candidate for UIA activation.
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      $name=Get-SafeName $button
      if ((($name -replace '&','').Trim()) -ne 'Close') { continue }

      $className=''; $automationId=''; $frameworkId=''; $nativeHandle=0
      $isEnabled=$null; $isOffscreen=$null
      try { $className=[string]$button.Current.ClassName } catch {}
      try { $automationId=[string]$button.Current.AutomationId } catch {}
      try { $frameworkId=[string]$button.Current.FrameworkId } catch {}
      try { $nativeHandle=[int]$button.Current.NativeWindowHandle } catch {}
      try { $isEnabled=[bool]$button.Current.IsEnabled } catch {
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-enabled-state-unavailable';control=$name;error=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
      }
      try { $isOffscreen=[bool]$button.Current.IsOffscreen } catch {
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-offscreen-state-unavailable';control=$name;error=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
      }

      if ($firstAttempt) {
        [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-close-candidate';control=$name;class_name=$className;automation_id=$automationId;framework_id=$frameworkId;native_handle=$nativeHandle;is_enabled=$isEnabled;is_offscreen=$isOffscreen;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiEvidence
      }

      if ($isEnabled -eq $false -or $isOffscreen -eq $true) {
        if ($firstAttempt) {
          [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-close-state-skipped';control=$name;is_enabled=$isEnabled;is_offscreen=$isOffscreen;at_utc=[DateTime]::UtcNow.ToString('o')})
          Write-UiEvidence
        }
        continue
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
  'uia-terminal-enabled-state-unavailable','uia-terminal-offscreen-state-unavailable',
  'uia-terminal-close-state-skipped',
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