param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33349384632 / artifact 9743480159 independently proved that
# current-user and per-machine NSIS both completed healthy reinstall, MISSING
# repair, HASH_MISMATCH repair, exact SHA256 restoration and the second healthy
# pass. Per-machine uninstall then remained incomplete for the full timeout with
# VSN-Agent=Stopped, the machine payload still present, HKLM registration still
# present, and the genuine content Close disabled. This is no longer treated as
# terminal-UI-only evidence. Record the process presence of the nested
# vsn-agent.exe service helper and sc.exe child while teardown is stalled so the
# next artifact can distinguish the blocking layer before any new product change
# control is considered. Completion predicate, process exit, exit code, service,
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
  # Exact-head run 33349384632 proved the real per-machine NSIS content Close
  # remains disabled while teardown itself is incomplete: service stopped,
  # payload present and HKLM registration present. Keep activation fail-closed
  # and bind each not-ready observation to live teardown plus helper-process
  # presence. This is evidence only and cannot satisfy or bypass acceptance.
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
        # The real NSIS content Close has a nonzero native child HWND. Never
        # activate it while disabled. Instead record live teardown state on every
        # retry; this is evidence only and cannot satisfy or bypass acceptance.
        if ($nativeHandle -ne 0 -and $isEnabled -eq $false -and $isOffscreen -ne $true) {
          $serviceStatus='MISSING'
          try {
            $serviceProbe=Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
            if ($null -ne $serviceProbe) { $serviceStatus=[string]$serviceProbe.Status }
          } catch { $serviceStatus='UNAVAILABLE' }
          $payloadExists=$false; $registrationExists=$false
          try { $payloadExists=Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe') -PathType Leaf } catch {}
          try { $registrationExists=Test-Path -LiteralPath $HklmNsisKey } catch {}
          $agentHelperPids=@(); $scPids=@()
          try { $agentHelperPids=@(Get-Process -Name 'vsn-agent' -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.Id }) } catch {}
          try { $scPids=@(Get-Process -Name 'sc' -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.Id }) } catch {}
          [void]$UiActions.Add([pscustomobject][ordered]@{
            lifecycle=$Lifecycle;phase=$Phase;action='uia-terminal-progress-probe';control=$name
            automation_id=$automationId;native_handle=$nativeHandle;is_enabled=$isEnabled
            service_status=$serviceStatus;machine_payload_exists=[bool]$payloadExists
            machine_registration_exists=[bool]$registrationExists
            agent_helper_pids=@($agentHelperPids);sc_pids=@($scPids)
            at_utc=[DateTime]::UtcNow.ToString('o')
          })
          Write-UiEvidence
          return $false
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
  'uia-terminal-close-state-skipped','uia-terminal-progress-probe',
  'service_status','machine_payload_exists','machine_registration_exists',
  'agent_helper_pids','sc_pids',
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
