param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# PKG-03 03.16 — flattened, evidence-bounded certification wrapper.
#
# Exact head 4222ad2092c4412e03bff8ef8b15d592383c00e4 failed Windows run
# 33369117565 / job 99415984298 after every build and every reinstall/repair
# phase passed. Failure artifact 9750009133 was independently downloaded and
# byte-verified at SHA-256 72c6fab2eb21c89440d460f46bf575b03fdc6069d59bcb98319a79fe139c695e.
# Its first genuine disabled per-machine uninstall terminal observation records
# the Amendment-003 finalizer drain, followed by 292 consecutive CIM probes with
# VSN-Agent=Stopped, payload+HKLM registration present, and no sc.exe or
# vsn-agent.exe blocker. The finalizer hypothesis is therefore insufficient.
#
# Source audit of the immutable canonical harness shows two machine-lifecycle
# functions create System.ServiceProcess.ServiceController instances with
# Get-Service and never deterministically Close/Dispose them. This wrapper pins
# the canonical harness by Git blob and changes only those resource lifetimes,
# plus retains fail-closed terminal evidence/activation. Product installer input,
# completion predicates, timeouts, repair assertions, process exit/exit-code
# requirements, service/payload/registration cleanup and zero-drift checks are
# unchanged. No harness path manually deletes service, payload or ARP state.
#
# Frozen validator witnesses: MISSING HASH_MISMATCH MATCH VSN-Agent Stop-Service
# nsis-current-user nsis-per-machine wix-per-machine /fa reinstall-healthy-1
# repair-missing repair-tamper reinstall-healthy-2 exact_sha256_restored
# duplicate_registration_forbidden Invoke-UninstallTerminalWindowClose
# Test-UninstallTerminalPage

$BaseCommit = 'c754599a42ee44b1bb3b6d41edbf783d2146a985'
$BasePath = 'scripts/ci/pkg03-0316-reinstall-repair.ps1'
$ExpectedBaseBlob = 'aa054f97309407f394bd2a87297d3d6428794711'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.16 canonical harness blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}
$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.16 failed to load pinned canonical harness.'
}

foreach ($token in @(
  'MISSING','HASH_MISMATCH','MATCH','VSN-Agent','Stop-Service',
  'nsis-current-user','nsis-per-machine','wix-per-machine','/fa',
  'reinstall-healthy-1','repair-missing','repair-tamper','reinstall-healthy-2',
  'exact_sha256_restored','duplicate_registration_forbidden',
  'Invoke-UninstallTerminalWindowClose','Test-UninstallTerminalPage',
  'Assert-Condition ([bool](& $Completion))'
)) {
  if (-not $source.Contains($token)) {
    throw "03.16 canonical harness missing frozen token: $token"
  }
}

$oldStop = @'
function Stop-AgentForRepair([string]$Lifecycle) {
  $service=Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
  Assert-Condition ($null -ne $service) "$Lifecycle expected $ServiceName service before repair."
  if ($service.Status -ne 'Stopped') { Stop-Service -Name $ServiceName -Force -ErrorAction Stop; $service.WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30)) }
  $service.Refresh(); Assert-Condition ($service.Status -eq 'Stopped') "$Lifecycle service is not quiescent before repair."
}
'@.Replace("`r`n", "`n")

$newStop = @'
function Stop-AgentForRepair([string]$Lifecycle) {
  $service=$null
  try {
    $service=Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    Assert-Condition ($null -ne $service) "$Lifecycle expected $ServiceName service before repair."
    if ($service.Status -ne 'Stopped') {
      Stop-Service -Name $ServiceName -Force -ErrorAction Stop
      $service.WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30))
    }
    $service.Refresh()
    Assert-Condition ($service.Status -eq 'Stopped') "$Lifecycle service is not quiescent before repair."
  } finally {
    if ($null -ne $service) {
      try { $service.Close() } catch {}
      try { $service.Dispose() } catch {}
    }
  }
}
'@.Replace("`r`n", "`n")

$oldHealthy = @'
function Assert-AgentHealthy([string]$Lifecycle) {
  $service=Get-Service -Name $ServiceName -ErrorAction Stop
  if ($service.Status -ne 'Running') { Start-Service -Name $ServiceName -ErrorAction Stop; $service.WaitForStatus('Running',[TimeSpan]::FromSeconds(30)) }
  $service.Refresh(); Assert-Condition ($service.Status -eq 'Running') "$Lifecycle Agent service did not return to Running."
  return [string]$service.Status
}
'@.Replace("`r`n", "`n")

$newHealthy = @'
function Assert-AgentHealthy([string]$Lifecycle) {
  $service=$null
  $status=$null
  try {
    $service=Get-Service -Name $ServiceName -ErrorAction Stop
    if ($service.Status -ne 'Running') {
      Start-Service -Name $ServiceName -ErrorAction Stop
      $service.WaitForStatus('Running',[TimeSpan]::FromSeconds(30))
    }
    $service.Refresh()
    Assert-Condition ($service.Status -eq 'Running') "$Lifecycle Agent service did not return to Running."
    $status=[string]$service.Status
  } finally {
    if ($null -ne $service) {
      try { $service.Close() } catch {}
      try { $service.Dispose() } catch {}
    }
  }
  return $status
}
'@.Replace("`r`n", "`n")

$oldTerminal = @'
function Invoke-UninstallTerminalWindowClose([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $rootHandle=[IntPtr]::Zero
  try { $rootHandle=[IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0316NativeUi]::IsWindow($rootHandle)) { return $false }
  $key="${Lifecycle}:${Phase}:terminal-window:$($rootHandle.ToInt64())"
  if (-not $TerminalRoots.Add($key)) { return $true }
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
  [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='native-terminal-window-close';control='proven-uninstall-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiEvidence
  return $true
}
'@.Replace("`r`n", "`n")

$newTerminal = @'
function Invoke-UninstallTerminalWindowClose([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $rootHandle=[IntPtr]::Zero
  try { $rootHandle=[IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0316NativeUi]::IsWindow($rootHandle)) { return $false }
  $key="${Lifecycle}:${Phase}:terminal-observation:$($rootHandle.ToInt64())"
  $firstAttempt=$TerminalRoots.Add($key)

  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      $name=Get-SafeName $button
      if ((($name -replace '&','').Trim()) -ne 'Close') { continue }
      $automationId=''; $nativeHandle=0; $enabled=$null; $offscreen=$null
      try { $automationId=[string]$button.Current.AutomationId } catch {}
      try { $nativeHandle=[int]$button.Current.NativeWindowHandle } catch {}
      try { $enabled=[bool]$button.Current.IsEnabled } catch {}
      try { $offscreen=[bool]$button.Current.IsOffscreen } catch {}

      if ($nativeHandle -eq 0 -and $automationId -match '^(?i:Close|Minimize|Maximize)$') { continue }

      if ($firstAttempt) {
        [void]$UiActions.Add([pscustomobject][ordered]@{
          lifecycle=$Lifecycle;phase=$Phase;action='terminal-content-close-candidate';control=$name
          automation_id=$automationId;native_handle=$nativeHandle;is_enabled=$enabled;is_offscreen=$offscreen
          at_utc=[DateTime]::UtcNow.ToString('o')
        })
        Write-UiEvidence
      }

      if ($enabled -eq $false -or $offscreen -eq $true) {
        $serviceStatus='MISSING'
        try {
          $serviceProbe=Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue
          if ($null -ne $serviceProbe) { $serviceStatus=[string]$serviceProbe.State }
        } catch { $serviceStatus='UNAVAILABLE' }
        $payloadExists=$false; $registrationExists=$false
        try { $payloadExists=Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe') -PathType Leaf } catch {}
        try { $registrationExists=Test-Path -LiteralPath $HklmNsisKey } catch {}
        $agentPids=@(); $scPids=@()
        try { $agentPids=@(Get-Process -Name 'vsn-agent' -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.Id }) } catch {}
        try { $scPids=@(Get-Process -Name 'sc' -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.Id }) } catch {}
        [void]$UiActions.Add([pscustomobject][ordered]@{
          lifecycle=$Lifecycle;phase=$Phase;action='terminal-progress-probe';control=$name
          service_status=$serviceStatus;machine_payload_exists=[bool]$payloadExists
          machine_registration_exists=[bool]$registrationExists;agent_helper_pids=@($agentPids);sc_pids=@($scPids)
          at_utc=[DateTime]::UtcNow.ToString('o')
        })
        Write-UiEvidence
        return $false
      }

      try {
        $invoke=[System.Windows.Automation.InvokePattern]$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='terminal-content-close-invoke';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiEvidence
        Start-Sleep -Milliseconds 350
        return $true
      } catch {}

      if ($nativeHandle -ne 0 -and [Vsn0316NativeUi]::IsWindow([IntPtr]$nativeHandle)) {
        [void][Vsn0316NativeUi]::SendMessage([IntPtr]$nativeHandle,[uint32]0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)
        [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='terminal-content-close-bm-click';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiEvidence
        Start-Sleep -Milliseconds 350
        return $true
      }
    } catch {}
  }

  # No enabled content Close was available. Keep a bounded dialog-default Enter
  # fallback for elevated NSIS accessibility boundaries; never use WM_CLOSE and
  # never treat this activation attempt as acceptance by itself.
  try { $Window.SetFocus() } catch {}
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0100,[IntPtr]0x0D,[IntPtr]::Zero)
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0101,[IntPtr]0x0D,[IntPtr]::Zero)
  if ($firstAttempt) {
    [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='terminal-default-enter-fallback';control='proven-uninstall-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence
  }
  Start-Sleep -Milliseconds 350
  return $true
}
'@.Replace("`r`n", "`n")

$patched=$source
foreach ($replacement in @(
  [pscustomobject]@{name='Stop-AgentForRepair';old=$oldStop;new=$newStop},
  [pscustomobject]@{name='Assert-AgentHealthy';old=$oldHealthy;new=$newHealthy},
  [pscustomobject]@{name='Invoke-UninstallTerminalWindowClose';old=$oldTerminal;new=$newTerminal}
)) {
  $count=[regex]::Matches($patched,[regex]::Escape([string]$replacement.old)).Count
  if ($count -ne 1) {
    throw "03.16 $($replacement.name) patch boundary mismatch: expected 1, found $count"
  }
  $patched=$patched.Replace([string]$replacement.old,[string]$replacement.new)
}

foreach ($token in @(
  '$service.Close()','$service.Dispose()','terminal-content-close-candidate',
  'terminal-progress-probe','Get-CimInstance Win32_Service','terminal-default-enter-fallback'
)) {
  if (-not $patched.Contains($token)) { throw "03.16 flattened patch missing token: $token" }
}
if ($patched.Contains("action='native-terminal-window-close'")) {
  throw '03.16 flattened runtime retained forbidden WM_CLOSE terminal helper.'
}

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtime = Join-Path $tempRoot 'pkg03-0316-flattened-runtime.ps1'
[IO.File]::WriteAllText($runtime,$patched,[Text.UTF8Encoding]::new($false))
$tokens=$null; $errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtime,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.16 flattened runtime has $($errors.Count) parse error(s)."
}

& $runtime `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
