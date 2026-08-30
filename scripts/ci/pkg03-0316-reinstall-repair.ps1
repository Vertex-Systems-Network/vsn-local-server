param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Vsn0316NativeUi {
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll", SetLastError=true)] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr GetAncestor(IntPtr hWnd, uint gaFlags);
  [DllImport("user32.dll", SetLastError=true)] public static extern int GetDlgCtrlID(IntPtr hWnd);
  [DllImport("user32.dll")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool IsWindow(IntPtr hWnd);
}
'@

$ProductName = 'VSN Dev Platform'
$ServiceName = 'VSN-Agent'
$UserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot = Join-Path $env:ProgramFiles $ProductName
$MachineSecurityRoot = Join-Path $env:ProgramData 'VSN\security'
$MachineSecurityKey = Join-Path $MachineSecurityRoot 'ipc.key'
$HkcuKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$HklmNsisKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$UiObservations = [System.Collections.Generic.List[object]]::new()
$UiActions = [System.Collections.Generic.List[object]]::new()
$IntegrityObservations = [System.Collections.Generic.List[object]]::new()
$TerminalRoots = [System.Collections.Generic.HashSet[string]]::new()

function Assert-Condition([bool]$Condition,[string]$Message) {
  if (-not $Condition) { throw $Message }
}

function Get-Sha256([string]$Path) {
  (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
}

function Write-UiEvidence {
  New-Item -ItemType Directory -Force $EvidencePath | Out-Null
  @($UiObservations) | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
  @($UiActions) | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
  @($IntegrityObservations) | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $EvidencePath 'integrity-observations.json') -Encoding utf8NoBOM
}

function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) {
  try { return ([string]$Element.Current.Name).Trim() } catch { return '' }
}

function Get-Controls([System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.ControlType]$Type) {
  $condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$Type)
  return @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
}

function Get-RelevantWindows([int]$RootPid) {
  $snapshot=@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId,ParentProcessId)
  $family=[System.Collections.Generic.HashSet[int]]::new(); [void]$family.Add($RootPid)
  do {
    $changed=$false
    foreach ($proc in $snapshot) {
      $pidNow=[int]$proc.ProcessId; $parent=[int]$proc.ParentProcessId
      if ($family.Contains($parent) -and -not $family.Contains($pidNow)) { [void]$family.Add($pidNow); $changed=$true }
    }
  } while ($changed)
  $root=[System.Windows.Automation.AutomationElement]::RootElement
  $all=$root.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
  $result=@()
  foreach ($window in $all) {
    try {
      $name=[string]$window.Current.Name; $pidNow=[int]$window.Current.ProcessId; $handle=[int]$window.Current.NativeWindowHandle
      $visible=-not [bool]$window.Current.IsOffscreen
      if ($visible -and $handle -ne 0 -and ($family.Contains($pidNow) -or $name -match '(?i)VSN Dev Platform|Windows Installer')) { $result += $window }
    } catch {}
  }
  return @($result)
}

function Record-Window([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $buttons=@(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
  [void]$UiObservations.Add([pscustomobject][ordered]@{
    lifecycle=$Lifecycle; phase=$Phase; title=Get-SafeName $Window; buttons=$buttons
    pid=$(try { [int]$Window.Current.ProcessId } catch { 0 }); at_utc=[DateTime]::UtcNow.ToString('o')
  })
  Write-UiEvidence
}

function Set-SafetyCheckboxes([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
    $name=Get-SafeName $box
    if ($name -notmatch '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform|delete.*(app.*data|data)|remove.*(app.*data|user.*data)') { continue }
    try {
      $toggle=[System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
      if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) { $toggle.Toggle(); Start-Sleep -Milliseconds 180 }
      [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='ensure-safety-checkbox-off';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence
    } catch {}
  }
}

function Invoke-NativeTerminal([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.AutomationElement]$Button,[string]$Name) {
  $buttonHandle=[IntPtr]::Zero
  try { $buttonHandle=[IntPtr][int]$Button.Current.NativeWindowHandle } catch {}
  $rootHandle=[IntPtr]::Zero
  if ($buttonHandle -ne [IntPtr]::Zero -and [Vsn0316NativeUi]::IsWindow($buttonHandle)) { $rootHandle=[Vsn0316NativeUi]::GetAncestor($buttonHandle,[uint32]2) }
  if ($rootHandle -eq [IntPtr]::Zero) { try { $rootHandle=[IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false } }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0316NativeUi]::IsWindow($rootHandle)) { return $false }
  $key="${Lifecycle}:${Phase}:$($rootHandle.ToInt64())"
  if (-not $TerminalRoots.Add($key)) { return $false }
  if ($buttonHandle -ne [IntPtr]::Zero -and [Vsn0316NativeUi]::IsWindow($buttonHandle)) {
    $controlId=[Vsn0316NativeUi]::GetDlgCtrlID($buttonHandle)
    if ($controlId -gt 0) { [void][Vsn0316NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle); Start-Sleep -Milliseconds 300 }
  }
  if ([Vsn0316NativeUi]::IsWindow($rootHandle)) { [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero) }
  [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='native-terminal';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiEvidence
  return $true
}

function Test-UninstallTerminalPage([System.Windows.Automation.AutomationElement]$Window) {
  $names=@(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { (Get-SafeName $_) -replace '&','' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
  $hasClose=@($names | Where-Object { $_ -match '(?i)^Close$' }).Count -gt 0
  $hasDetails=@($names | Where-Object { $_ -match '(?i)^Show details$' }).Count -gt 0
  $hasBack=@($names | Where-Object { $_ -match '(?i)^< Back$' }).Count -gt 0
  $hasDestructiveAction=@($names | Where-Object { $_ -match '(?i)^(Remove|Uninstall)$' }).Count -gt 0
  return $hasClose -and $hasDetails -and $hasBack -and -not $hasDestructiveAction
}

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

function Invoke-PrimaryButton([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[bool]$CompletionReached,[bool]$Maintenance=$false) {
  $priority = if ($Maintenance) {
    @('^Reinstall$','^Repair$','^Install$','^Next\b','^Yes$','^Finish$','^OK$','^Close$')
  } elseif ($Phase -eq 'uninstall') {
    @('^Remove$','^Uninstall$','^Next\b','^Yes$','^Finish$','^OK$','^Close$')
  } else {
    @('^Install$','^Next\b','^Yes$','^Finish$','^OK$','^Close$')
  }
  $candidates=@()
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name=Get-SafeName $button; $automationId=[string]$button.Current.AutomationId; $nativeHandle=[int]$button.Current.NativeWindowHandle
      if ($nativeHandle -eq 0 -and $automationId -match '^(?i:Close|Minimize|Maximize)$') { continue }
      if ($name) { $candidates += [pscustomobject]@{element=$button;name=$name;norm=($name -replace '&','').Trim()} }
    } catch {}
  }
  foreach ($pattern in $priority) {
    $selected=$candidates | Where-Object { $_.norm -match "(?i)$pattern" } | Select-Object -First 1
    if ($null -eq $selected) { continue }
    if (($CompletionReached -or $Maintenance) -and $selected.norm -match '(?i)^(Finish|OK|Close)$') {
      if (Invoke-NativeTerminal $Lifecycle $Phase $Window $selected.element $selected.name) { return $selected.norm }
    }
    try {
      $invoke=[System.Windows.Automation.InvokePattern]$selected.element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='invoke-button';control=$selected.name;at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence
      return $selected.norm
    } catch {}
  }
  return $null
}

function Drive-SuccessUi([string]$Lifecycle,[string]$Phase,[System.Diagnostics.Process]$Process,[scriptblock]$Completion,[bool]$Maintenance=$false,[int]$TimeoutSeconds=240) {
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds); $visible=$false; $quiet=0
  while ([DateTime]::UtcNow -lt $deadline) {
    $complete=[bool](& $Completion)
    $windows=@(Get-RelevantWindows $Process.Id)
    if ($windows.Count -eq 0) {
      $exited=$false; try { $Process.Refresh(); $exited=$Process.HasExited } catch { $exited=$true }
      if ($complete -and $exited) { $quiet++; if ($quiet -ge 3) { break } } else { $quiet=0 }
      Start-Sleep -Milliseconds 450; continue
    }
    $visible=$true; $quiet=0; $window=$windows[0]
    try { $window.SetFocus() } catch {}
    Record-Window $Lifecycle $Phase $window
    Set-SafetyCheckboxes $Lifecycle $Phase $window
    $terminalPage=($Phase -eq 'uninstall') -and (Test-UninstallTerminalPage $window)
    if ($terminalPage) {
      [void](Invoke-UninstallTerminalWindowClose $Lifecycle $Phase $window)
    } else {
      [void](Invoke-PrimaryButton $Lifecycle $Phase $window $complete $Maintenance)
    }
    Start-Sleep -Milliseconds 700
  }
  Assert-Condition ([bool](& $Completion)) "$Lifecycle $Phase did not reach required state."
  $ok=$Process.WaitForExit(30000); Assert-Condition $ok "$Lifecycle $Phase root process did not exit."
  $Process.Refresh(); $exit=[int]$Process.ExitCode
  Assert-Condition ($exit -eq 0) "$Lifecycle $Phase exit code was $exit, expected 0."
  return [pscustomobject][ordered]@{phase=$Phase;visible_ui_observed=$visible;exit_code=$exit}
}

function Get-MsiProperty([string]$Path,[string]$Property) {
  $installer=New-Object -ComObject WindowsInstaller.Installer
  $db=$installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
  $view=$db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
  $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null)|Out-Null
  $record=$view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null)
  if ($null -eq $record) { throw "MSI property '$Property' not found." }
  return [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}

function Get-OwnedExpected([string]$InstallRoot) {
  $records=@()
  foreach ($relative in @('VSN Dev Platform.exe','bin\vsn.exe','bin\vsn-agent.exe')) {
    $path=Join-Path $InstallRoot $relative
    Assert-Condition (Test-Path -LiteralPath $path -PathType Leaf) "Expected installed owned payload missing: $relative"
    $records += [pscustomobject][ordered]@{relative_path=$relative;size_bytes=[int64](Get-Item -LiteralPath $path).Length;sha256=Get-Sha256 $path}
  }
  return @($records)
}

function Get-IntegrityObservation([string]$Lifecycle,[string]$Stage,[string]$InstallRoot,[pscustomobject]$Expected) {
  $path=Join-Path $InstallRoot ([string]$Expected.relative_path)
  $exists=Test-Path -LiteralPath $path -PathType Leaf; $observedHash=$null; $classification='MISSING'
  if ($exists) { $observedHash=Get-Sha256 $path; $classification=if ($observedHash -eq [string]$Expected.sha256) { 'MATCH' } else { 'HASH_MISMATCH' } }
  $record=[pscustomobject][ordered]@{
    lifecycle=$Lifecycle;stage=$Stage;relative_path=[string]$Expected.relative_path;expected_sha256=[string]$Expected.sha256
    observed_exists=[bool]$exists;observed_sha256=$observedHash;classification=$classification;repair_required=($classification -ne 'MATCH')
    at_utc=[DateTime]::UtcNow.ToString('o')
  }
  [void]$IntegrityObservations.Add($record); Write-UiEvidence; return $record
}

function Assert-AllMatch([string]$Lifecycle,[string]$Stage,[string]$InstallRoot,[object[]]$ExpectedOwned) {
  foreach ($expected in $ExpectedOwned) {
    $r=Get-IntegrityObservation $Lifecycle $Stage $InstallRoot $expected
    Assert-Condition ($r.classification -eq 'MATCH') "$Lifecycle $Stage $($expected.relative_path) is $($r.classification), expected MATCH."
  }
}

function Get-ProductRegistrationKeys {
  $found=@()
  foreach ($root in @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall','HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall')) {
    if (-not (Test-Path -LiteralPath $root)) { continue }
    foreach ($key in @(Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue)) {
      try { if ([string](Get-ItemProperty -LiteralPath $key.PSPath -Name DisplayName -ErrorAction Stop).DisplayName -eq $ProductName) { $found += $key.PSPath } } catch {}
    }
  }
  return @($found | Sort-Object -Unique)
}

function Get-ShortcutPaths {
  $roots=@(
    [Environment]::GetFolderPath('Desktop'),
    [Environment]::GetFolderPath('CommonDesktopDirectory'),
    [Environment]::GetFolderPath('StartMenu'),
    [Environment]::GetFolderPath('CommonStartMenu')
  ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }
  $paths=@()
  foreach ($root in $roots) { $paths += @(Get-ChildItem -LiteralPath $root -Filter 'VSN Dev Platform*.lnk' -File -Recurse -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName }) }
  return @($paths | Sort-Object -Unique)
}

function Get-ServiceSnapshot {
  $svc=Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue
  if ($null -eq $svc) { return $null }
  return [pscustomobject][ordered]@{name=[string]$svc.Name;display_name=[string]$svc.DisplayName;start_name=[string]$svc.StartName;start_mode=[string]$svc.StartMode;path_name=[string]$svc.PathName;state=[string]$svc.State}
}

function Get-SecuritySnapshot {
  $dirExists=Test-Path -LiteralPath $MachineSecurityRoot -PathType Container
  $keyExists=Test-Path -LiteralPath $MachineSecurityKey -PathType Leaf
  return [pscustomobject][ordered]@{
    directory_exists=[bool]$dirExists; key_exists=[bool]$keyExists
    directory_sddl=$(if ($dirExists) { [string](Get-Acl -LiteralPath $MachineSecurityRoot).Sddl } else { $null })
    key_sddl=$(if ($keyExists) { [string](Get-Acl -LiteralPath $MachineSecurityKey).Sddl } else { $null })
  }
}

function Get-IdentitySnapshot([string]$InstallRoot) {
  return [pscustomobject][ordered]@{
    install_root=$InstallRoot
    registration_keys=@(Get-ProductRegistrationKeys)
    shortcuts=@(Get-ShortcutPaths)
    service=Get-ServiceSnapshot
    security=Get-SecuritySnapshot
  }
}

function Assert-IdentityStable([string]$Lifecycle,[pscustomobject]$Baseline,[pscustomobject]$Current,[bool]$Machine) {
  Assert-Condition ($Baseline.install_root -eq $Current.install_root) "$Lifecycle install root changed during repair."
  Assert-Condition ((@($Baseline.registration_keys) -join '|') -eq (@($Current.registration_keys) -join '|')) "$Lifecycle registration cardinality/identity changed during repair."
  Assert-Condition ((@($Baseline.shortcuts) -join '|') -eq (@($Current.shortcuts) -join '|')) "$Lifecycle shortcut cardinality/identity changed during repair."
  if ($Machine) {
    Assert-Condition ($null -ne $Current.service) "$Lifecycle Agent service disappeared during repair."
    foreach ($field in @('name','display_name','start_name','start_mode','path_name')) {
      Assert-Condition ([string]$Baseline.service.$field -eq [string]$Current.service.$field) "$Lifecycle service identity/config changed: $field"
    }
    Assert-Condition ([string]$Baseline.security.directory_sddl -eq [string]$Current.security.directory_sddl) "$Lifecycle security directory ACL changed during repair."
    Assert-Condition ([string]$Baseline.security.key_sddl -eq [string]$Current.security.key_sddl) "$Lifecycle IPC key ACL changed during repair."
  } else {
    Assert-Condition ($null -eq $Current.service) "$Lifecycle illegally created machine Agent service."
    Assert-Condition (-not $Current.security.directory_exists -and -not $Current.security.key_exists) "$Lifecycle illegally created machine security state."
  }
}

function Stop-AgentForRepair([string]$Lifecycle) {
  $service=Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
  Assert-Condition ($null -ne $service) "$Lifecycle expected $ServiceName service before repair."
  if ($service.Status -ne 'Stopped') { Stop-Service -Name $ServiceName -Force -ErrorAction Stop; $service.WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30)) }
  $service.Refresh(); Assert-Condition ($service.Status -eq 'Stopped') "$Lifecycle service is not quiescent before repair."
}

function Assert-AgentHealthy([string]$Lifecycle) {
  $service=Get-Service -Name $ServiceName -ErrorAction Stop
  if ($service.Status -ne 'Running') { Start-Service -Name $ServiceName -ErrorAction Stop; $service.WaitForStatus('Running',[TimeSpan]::FromSeconds(30)) }
  $service.Refresh(); Assert-Condition ($service.Status -eq 'Running') "$Lifecycle Agent service did not return to Running."
  return [string]$service.Status
}

function Invoke-NsisMaintenance([string]$Lifecycle,[string]$Phase,[string]$SetupPath,[scriptblock]$Completion) {
  $p=Start-Process -FilePath $SetupPath -PassThru
  return Drive-SuccessUi $Lifecycle $Phase $p $Completion $true 300
}

function Invoke-MsiRepair([string]$Lifecycle,[string]$Phase,[string]$ProductCode,[string]$LogPath) {
  $msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
  $args=@('/fa',$ProductCode,'/L*V',('"{0}"' -f $LogPath))
  $p=Start-Process -FilePath $msiexec -ArgumentList $args -PassThru -Wait
  Assert-Condition ($p.ExitCode -eq 0) "$Lifecycle $Phase MSI /fa repair exit code was $($p.ExitCode), expected 0."
  Assert-Condition (Test-Path -LiteralPath $LogPath -PathType Leaf) "$Lifecycle $Phase MSI repair log missing."
  Assert-Condition ((Get-Item -LiteralPath $LogPath).Length -gt 0) "$Lifecycle $Phase MSI repair log is empty."
  return [pscustomobject][ordered]@{phase=$Phase;exit_code=[int]$p.ExitCode;log=[pscustomobject][ordered]@{path=$LogPath;size_bytes=[int64](Get-Item $LogPath).Length;sha256=Get-Sha256 $LogPath}}
}

function Set-Probe([string]$Lifecycle,[string]$InstallRoot,[pscustomobject]$Expected,[string]$Probe) {
  $path=Join-Path $InstallRoot ([string]$Expected.relative_path)
  Assert-Condition (Test-Path -LiteralPath $path -PathType Leaf) "$Lifecycle probe target missing before mutation: $($Expected.relative_path)"
  Assert-Condition ((Get-Sha256 $path) -eq [string]$Expected.sha256) "$Lifecycle probe target was not MATCH before mutation."
  if ($Probe -eq 'missing') { Remove-Item -LiteralPath $path -Force }
  elseif ($Probe -eq 'tamper') { [IO.File]::AppendAllText($path,'PKG03-0316-TAMPER',[Text.Encoding]::UTF8) }
  else { throw "Unknown probe: $Probe" }
  $r=Get-IntegrityObservation $Lifecycle $(if ($Probe -eq 'missing') {'repair-missing-pre'} else {'repair-tamper-pre'}) $InstallRoot $Expected
  $wanted=if ($Probe -eq 'missing') {'MISSING'} else {'HASH_MISMATCH'}
  Assert-Condition ($r.classification -eq $wanted) "$Lifecycle $Probe probe classified $($r.classification), expected $wanted."
  return $r
}

function Invoke-ReinstallRepairLifecycle(
  [string]$Lifecycle,
  [string]$InstallRoot,
  [string]$SetupPath,
  [bool]$Machine,
  [bool]$Msi,
  [string]$ProductCode,
  [scriptblock]$StartInitialInstall,
  [scriptblock]$InitialInstallCompletion,
  [scriptblock]$StartUninstall,
  [scriptblock]$UninstallCompletion
) {
  $installProcess=& $StartInitialInstall
  $initial=Drive-SuccessUi $Lifecycle 'initial-install' $installProcess $InitialInstallCompletion $false 300
  $expected=@(Get-OwnedExpected $InstallRoot)
  Assert-AllMatch $Lifecycle 'initial-match' $InstallRoot $expected
  $baseline=Get-IdentitySnapshot $InstallRoot
  Assert-Condition (@($baseline.registration_keys).Count -eq 1) "$Lifecycle expected exactly one product registration after initial install."
  if ($Machine) {
    Assert-Condition ($null -ne $baseline.service) "$Lifecycle expected machine Agent service after initial install."
    [void](Assert-AgentHealthy $Lifecycle)
    $baseline=Get-IdentitySnapshot $InstallRoot
  } else {
    Assert-Condition ($null -eq $baseline.service) "$Lifecycle current-user install created Agent service."
    Assert-Condition (-not $baseline.security.directory_exists -and -not $baseline.security.key_exists) "$Lifecycle current-user install created machine security state."
  }

  $repairRecords=@()
  if ($Machine) { Stop-AgentForRepair $Lifecycle }
  if ($Msi) {
    $log=Join-Path $EvidencePath "$Lifecycle-reinstall-healthy-1.log"
    $action=Invoke-MsiRepair $Lifecycle 'reinstall-healthy-1' $ProductCode $log
  } else {
    $action=Invoke-NsisMaintenance $Lifecycle 'reinstall-healthy-1' $SetupPath { Test-Path -LiteralPath (Join-Path $InstallRoot 'VSN Dev Platform.exe') }
  }
  Assert-AllMatch $Lifecycle 'reinstall-healthy-1-post' $InstallRoot $expected
  if ($Machine) { [void](Assert-AgentHealthy $Lifecycle) }
  $snap=Get-IdentitySnapshot $InstallRoot; Assert-IdentityStable $Lifecycle $baseline $snap $Machine
  $repairRecords += [pscustomobject][ordered]@{phase='reinstall-healthy-1';action=$action;all_match=$true;identity_stable=$true}

  $missing=$expected | Where-Object { $_.relative_path -eq 'VSN Dev Platform.exe' } | Select-Object -First 1
  if ($Machine) { Stop-AgentForRepair $Lifecycle }
  $preMissing=Set-Probe $Lifecycle $InstallRoot $missing 'missing'
  if ($Msi) {
    $log=Join-Path $EvidencePath "$Lifecycle-repair-missing.log"
    $action=Invoke-MsiRepair $Lifecycle 'repair-missing' $ProductCode $log
  } else {
    $action=Invoke-NsisMaintenance $Lifecycle 'repair-missing' $SetupPath { Test-Path -LiteralPath (Join-Path $InstallRoot ([string]$missing.relative_path)) }
  }
  $postMissing=Get-IntegrityObservation $Lifecycle 'repair-missing-post' $InstallRoot $missing
  Assert-Condition ($postMissing.classification -eq 'MATCH') "$Lifecycle missing-file repair did not restore exact bytes."
  if ($Machine) { [void](Assert-AgentHealthy $Lifecycle) }
  $snap=Get-IdentitySnapshot $InstallRoot; Assert-IdentityStable $Lifecycle $baseline $snap $Machine
  $repairRecords += [pscustomobject][ordered]@{phase='repair-missing';pre_classification=$preMissing.classification;post_classification=$postMissing.classification;action=$action;exact_sha256_restored=$true;identity_stable=$true}

  $tamper=$expected | Where-Object { $_.relative_path -eq 'bin\vsn.exe' } | Select-Object -First 1
  if ($Machine) { Stop-AgentForRepair $Lifecycle }
  $preTamper=Set-Probe $Lifecycle $InstallRoot $tamper 'tamper'
  if ($Msi) {
    $log=Join-Path $EvidencePath "$Lifecycle-repair-tamper.log"
    $action=Invoke-MsiRepair $Lifecycle 'repair-tamper' $ProductCode $log
  } else {
    $action=Invoke-NsisMaintenance $Lifecycle 'repair-tamper' $SetupPath { (Test-Path -LiteralPath (Join-Path $InstallRoot ([string]$tamper.relative_path))) -and ((Get-Sha256 (Join-Path $InstallRoot ([string]$tamper.relative_path))) -eq [string]$tamper.sha256) }
  }
  $postTamper=Get-IntegrityObservation $Lifecycle 'repair-tamper-post' $InstallRoot $tamper
  Assert-Condition ($postTamper.classification -eq 'MATCH') "$Lifecycle tampered-file repair did not restore exact bytes."
  if ($Machine) { [void](Assert-AgentHealthy $Lifecycle) }
  $snap=Get-IdentitySnapshot $InstallRoot; Assert-IdentityStable $Lifecycle $baseline $snap $Machine
  $repairRecords += [pscustomobject][ordered]@{phase='repair-tamper';pre_classification=$preTamper.classification;post_classification=$postTamper.classification;action=$action;exact_sha256_restored=$true;identity_stable=$true}

  if ($Machine) { Stop-AgentForRepair $Lifecycle }
  if ($Msi) {
    $log=Join-Path $EvidencePath "$Lifecycle-reinstall-healthy-2.log"
    $action=Invoke-MsiRepair $Lifecycle 'reinstall-healthy-2' $ProductCode $log
  } else {
    $action=Invoke-NsisMaintenance $Lifecycle 'reinstall-healthy-2' $SetupPath { Test-Path -LiteralPath (Join-Path $InstallRoot 'VSN Dev Platform.exe') }
  }
  Assert-AllMatch $Lifecycle 'reinstall-healthy-2-post' $InstallRoot $expected
  if ($Machine) { [void](Assert-AgentHealthy $Lifecycle) }
  $snap=Get-IdentitySnapshot $InstallRoot; Assert-IdentityStable $Lifecycle $baseline $snap $Machine
  $repairRecords += [pscustomobject][ordered]@{phase='reinstall-healthy-2';action=$action;all_match=$true;identity_stable=$true}

  if ($Machine) { Stop-AgentForRepair $Lifecycle }
  $uninstallProcess=& $StartUninstall
  $uninstall=Drive-SuccessUi $Lifecycle 'uninstall' $uninstallProcess $UninstallCompletion $false 300
  foreach ($owned in $expected) { Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $InstallRoot ([string]$owned.relative_path)) -PathType Leaf)) "$Lifecycle uninstall left owned payload $($owned.relative_path)." }

  return [pscustomobject][ordered]@{
    lifecycle=$Lifecycle;install_root=$InstallRoot;initial_install=$initial;expected_owned=$expected;baseline_identity=$baseline
    repairs=@($repairRecords);uninstall=$uninstall;exact_sha256_restored=$true;duplicate_registration_forbidden=$true
    machine_agent_destructive_probe=$false;service_quiescent_before_machine_repairs=$Machine
  }
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
Write-UiEvidence
$actualHead=(git rev-parse HEAD).Trim(); Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"
$CurrentUserNsisPath=(Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
$PerMachineNsisPath=(Resolve-Path -LiteralPath $PerMachineNsisPath).Path
$MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
foreach ($path in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)) { Assert-Condition ((Get-Item -LiteralPath $path).Length -gt 0) "Package is empty: $path" }
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
$msiArpKey="HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productCode"
$msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'

Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) 'Current-user ARP state exists at preflight.'
Assert-Condition (-not (Test-Path -LiteralPath $HklmNsisKey)) 'Machine NSIS ARP state exists at preflight.'
Assert-Condition (-not (Test-Path -LiteralPath $msiArpKey)) 'MSI ARP state exists at preflight.'
Assert-Condition ($null -eq (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) 'Agent service exists before current-user lifecycle.'
Assert-Condition (-not (Test-Path -LiteralPath $MachineSecurityRoot)) 'Machine security state exists before current-user lifecycle.'

$currentUser=Invoke-ReinstallRepairLifecycle 'nsis-current-user' $UserRoot $CurrentUserNsisPath $false $false '' `
  { Start-Process -FilePath $CurrentUserNsisPath -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HkcuKey) } `
  { Start-Process -FilePath (Join-Path $UserRoot 'uninstall.exe') -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HkcuKey) }
Assert-Condition ($null -eq (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) 'Current-user repair lifecycle created Agent service.'
Assert-Condition (-not (Test-Path -LiteralPath $MachineSecurityRoot)) 'Current-user repair lifecycle created machine security state.'

$perMachine=Invoke-ReinstallRepairLifecycle 'nsis-per-machine' $MachineRoot $PerMachineNsisPath $true $false '' `
  { Start-Process -FilePath $PerMachineNsisPath -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HklmNsisKey) } `
  { Start-Process -FilePath (Join-Path $MachineRoot 'uninstall.exe') -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HklmNsisKey) }
Assert-Condition ($null -eq (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) 'Per-machine NSIS uninstall left Agent service.'

$wix=Invoke-ReinstallRepairLifecycle 'wix-per-machine' $MachineRoot $MsiPath $true $true $productCode `
  { Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath)) -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $msiArpKey) } `
  { Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode) -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $msiArpKey) }
Assert-Condition ($null -eq (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) 'MSI uninstall left Agent service.'

$tracked=@(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) { $tracked | Write-Host; throw 'Tracked repository drift detected during 03.16 repair lifecycle.' }

Write-UiEvidence
$evidence=[ordered]@{
  schema_version=1;package_id='PKG-03';task_id='03.16';source_commit=$SourceSha
  packages=[ordered]@{
    nsis_current_user=[ordered]@{path=$CurrentUserNsisPath;size_bytes=[int64](Get-Item $CurrentUserNsisPath).Length;sha256=Get-Sha256 $CurrentUserNsisPath}
    nsis_per_machine=[ordered]@{path=$PerMachineNsisPath;size_bytes=[int64](Get-Item $PerMachineNsisPath).Length;sha256=Get-Sha256 $PerMachineNsisPath}
    msi=[ordered]@{path=$MsiPath;size_bytes=[int64](Get-Item $MsiPath).Length;sha256=Get-Sha256 $MsiPath;product_code=$productCode}
  }
  lifecycles=@($currentUser,$perMachine,$wix)
  classification_contract=@('MATCH','MISSING','HASH_MISMATCH')
  exact_sha256_restoration_required=$true
  healthy_idempotent_reinstall_required=$true
  missing_file_repair_required=$true
  tampered_file_repair_required=$true
  second_healthy_pass_required=$true
  duplicate_registration_forbidden=$true
  current_user_machine_service_forbidden=$true
  per_machine_agent_destructive_probe=$false
  service_quiescent_before_machine_repairs=$true
  msi_repair_mode='/fa'
  msi_verbose_repair_logs=$true
  running_process_coordination_claimed=$false
  dirty_user_data_uninstall_claimed=$false
  rollback_or_recovery_claimed=$false
  reboot_semantics_claimed=$false
  silent_or_passive_deployment_claimed=$false
  signing_claimed=$false
  updater_mutation_claimed=$false
  product_configuration_mutated=$false
  acl_policy_mutated=$false
  tracked_repository_drift_zero=$true
}
$evidence | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM