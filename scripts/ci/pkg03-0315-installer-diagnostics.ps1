param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.15'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Vsn0315NativeUi {
  [DllImport("user32.dll", SetLastError=true)]
  public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll", SetLastError=true)]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll", SetLastError=true)]
  public static extern IntPtr GetAncestor(IntPtr hWnd, uint gaFlags);
  [DllImport("user32.dll", SetLastError=true)]
  public static extern int GetDlgCtrlID(IntPtr hWnd);
  [DllImport("user32.dll")]
  [return: MarshalAs(UnmanagedType.Bool)]
  public static extern bool IsWindow(IntPtr hWnd);
}
'@

$ProductName = 'VSN Dev Platform'
$UserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot = Join-Path $env:ProgramFiles $ProductName
$HkcuKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$HklmNsisKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()
$TerminalRoots = [System.Collections.Generic.HashSet[string]]::new()

function Assert-Condition([bool]$Condition,[string]$Message) {
  if (-not $Condition) { throw $Message }
}

function Get-Sha256([string]$Path) {
  (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
}

function Write-UiEvidence {
  New-Item -ItemType Directory -Force $EvidencePath | Out-Null
  @($Observations) | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
  @($Actions) | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
}

function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) {
  try { return ([string]$Element.Current.Name).Trim() } catch { return '' }
}

function Get-Controls([System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.ControlType]$Type) {
  $condition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$Type)
  return @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
}

function Get-RelevantWindows([int]$RootPid) {
  $snapshot = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId,ParentProcessId)
  $family = [System.Collections.Generic.HashSet[int]]::new()
  [void]$family.Add($RootPid)
  do {
    $changed = $false
    foreach ($proc in $snapshot) {
      $pidNow = [int]$proc.ProcessId
      $parent = [int]$proc.ParentProcessId
      if ($family.Contains($parent) -and -not $family.Contains($pidNow)) {
        [void]$family.Add($pidNow); $changed = $true
      }
    }
  } while ($changed)

  $root = [System.Windows.Automation.AutomationElement]::RootElement
  $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
  $result = @()
  foreach ($window in $all) {
    try {
      $name = [string]$window.Current.Name
      $pidNow = [int]$window.Current.ProcessId
      $handle = [int]$window.Current.NativeWindowHandle
      $visible = -not [bool]$window.Current.IsOffscreen
      $titleFallback = $name -match '(?i)VSN Dev Platform|Windows Installer'
      if ($visible -and $handle -ne 0 -and ($family.Contains($pidNow) -or $titleFallback)) { $result += $window }
    } catch {}
  }
  return @($result)
}

function Record-Window([string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $controls = @()
  $all = $Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
  foreach ($element in $all) {
    try {
      $patterns = @()
      try { $patterns = @($element.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }) } catch {}
      $controls += [pscustomobject][ordered]@{
        control_type = [string]$element.Current.ControlType.ProgrammaticName
        name = Get-SafeName $element
        automation_id = [string]$element.Current.AutomationId
        class_name = [string]$element.Current.ClassName
        enabled = [bool]$element.Current.IsEnabled
        offscreen = [bool]$element.Current.IsOffscreen
        native_window_handle = [int]$element.Current.NativeWindowHandle
        patterns = $patterns
      }
    } catch {}
  }
  [void]$Observations.Add([pscustomobject][ordered]@{
    phase=$Phase
    title=Get-SafeName $Window
    pid=$(try { [int]$Window.Current.ProcessId } catch { 0 })
    controls=$controls
    at_utc=[DateTime]::UtcNow.ToString('o')
  })
  Write-UiEvidence
}

function Set-LaunchOff([string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
    $name = Get-SafeName $box
    if ($name -notmatch '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform') { continue }
    try {
      $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
      if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) { $toggle.Toggle(); Start-Sleep -Milliseconds 180 }
      [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='ensure-launch-off';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence
    } catch {}
  }
}

function Invoke-NativeTerminal([string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.AutomationElement]$Button,[string]$Name) {
  $buttonHandle = [IntPtr]::Zero
  try { $buttonHandle = [IntPtr][int]$Button.Current.NativeWindowHandle } catch {}
  $rootHandle = [IntPtr]::Zero
  if ($buttonHandle -ne [IntPtr]::Zero -and [Vsn0315NativeUi]::IsWindow($buttonHandle)) {
    $rootHandle = [Vsn0315NativeUi]::GetAncestor($buttonHandle,[uint32]2)
  }
  if ($rootHandle -eq [IntPtr]::Zero) {
    try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return }
  }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0315NativeUi]::IsWindow($rootHandle)) { return }
  $key = "${Phase}:$($rootHandle.ToInt64())"
  if (-not $TerminalRoots.Add($key)) { return }

  if ($buttonHandle -ne [IntPtr]::Zero -and [Vsn0315NativeUi]::IsWindow($buttonHandle)) {
    $controlId = [Vsn0315NativeUi]::GetDlgCtrlID($buttonHandle)
    if ($controlId -gt 0) {
      [void][Vsn0315NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle)
      [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='native-terminal-command';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence
      Start-Sleep -Milliseconds 350
    }
  }
  if ([Vsn0315NativeUi]::IsWindow($rootHandle)) {
    [void][Vsn0315NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='native-terminal-close';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence
  }
}

function Invoke-Button(
  [string]$Phase,
  [System.Windows.Automation.AutomationElement]$Window,
  [string[]]$Patterns,
  [bool]$TerminalReady = $false
) {
  $buttons = @()
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name = Get-SafeName $button
      if ($name) { $buttons += [pscustomobject]@{element=$button;name=$name;norm=($name -replace '&','').Trim()} }
    } catch {}
  }
  foreach ($pattern in $Patterns) {
    $selected = $buttons | Where-Object { $_.norm -match "(?i)$pattern" } | Select-Object -First 1
    if ($null -eq $selected) { continue }
    if ($TerminalReady -and $selected.norm -match '(?i)^(Finish|Close|OK)$') {
      Invoke-NativeTerminal $Phase $Window $selected.element $selected.name
      return $selected.norm
    }
    try {
      $invoke = [System.Windows.Automation.InvokePattern]$selected.element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='invoke-button';control=$selected.name;at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence
      return $selected.norm
    } catch {}
  }
  return $null
}

function Wait-ProcessExit([System.Diagnostics.Process]$Process,[string]$Phase,[int]$Seconds=25) {
  Wait-Process -Id $Process.Id -Timeout $Seconds -ErrorAction SilentlyContinue
  try { $Process.Refresh() } catch {}
  $exited=$false
  try { $exited=$Process.HasExited } catch { $exited=$true }
  Assert-Condition $exited "$Phase process did not exit."
  return [int]$Process.ExitCode
}

function Drive-SuccessUi(
  [System.Diagnostics.Process]$Process,
  [string]$Phase,
  [scriptblock]$Completion,
  [bool]$Uninstall=$false,
  [int]$TimeoutSeconds=210
) {
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible=$false
  $quiet=0
  while ([DateTime]::UtcNow -lt $deadline) {
    $complete=[bool](& $Completion)
    $windows=@(Get-RelevantWindows $Process.Id)
    if ($windows.Count -eq 0) {
      if ($complete) { $quiet++; if ($quiet -ge 3) { break } } else { $quiet=0 }
      Start-Sleep -Milliseconds 450
      continue
    }
    $visible=$true; $quiet=0
    foreach ($window in $windows) {
      try { $window.SetFocus() } catch {}
      Record-Window $Phase $window
      Set-LaunchOff $Phase $window
      $patterns = if ($Uninstall) { @('^Uninstall$','^Remove$','^Next\b','^Yes$','^Finish$','^Close$','^OK$') } else { @('^Install$','^Next\b','^Finish$','^Close$','^OK$') }
      [void](Invoke-Button $Phase $window $patterns ([bool](& $Completion)))
      Start-Sleep -Milliseconds 700
      break
    }
  }
  Assert-Condition $visible "$Phase did not expose visible installer UI."
  Assert-Condition ([bool](& $Completion)) "$Phase did not reach its required state."
  $exit=Wait-ProcessExit $Process $Phase
  return [pscustomobject][ordered]@{phase=$Phase;visible_ui=$visible;exit_code=$exit}
}

function Drive-CancelUi([System.Diagnostics.Process]$Process,[string]$Phase,[int]$ExpectedExit,[int]$TimeoutSeconds=90) {
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible=$false
  $cancelRequested=$false
  $confirmRequested=$false
  while ([DateTime]::UtcNow -lt $deadline) {
    try { if ($Process.HasExited) { break } } catch { break }
    $windows=@(Get-RelevantWindows $Process.Id)
    if ($windows.Count -eq 0) { Start-Sleep -Milliseconds 350; continue }
    $visible=$true
    foreach ($window in $windows) { Record-Window $Phase $window }

    if ($cancelRequested) {
      foreach ($window in $windows) {
        $clicked=Invoke-Button $Phase $window @('^Yes$','^OK$') $false
        if ($clicked) { $confirmRequested=$true; break }
      }
    } else {
      foreach ($window in $windows) {
        $clicked=Invoke-Button $Phase $window @('^Cancel$') $false
        if ($clicked) { $cancelRequested=$true; break }
      }
    }
    Start-Sleep -Milliseconds 500
  }
  Assert-Condition $visible "$Phase did not expose visible UI."
  Assert-Condition $cancelRequested "$Phase never invoked Cancel."
  $exit=Wait-ProcessExit $Process $Phase 20
  Assert-Condition ($exit -eq $ExpectedExit) "$Phase exit code mismatch: expected=$ExpectedExit actual=$exit"
  return [pscustomobject][ordered]@{
    phase=$Phase
    visible_ui=$visible
    cancel_confirmed=$true
    confirmation_ui_observed=$confirmRequested
    direct_exit_after_cancel=(-not $confirmRequested)
    exit_code=$exit
    expected_exit_code=$ExpectedExit
  }
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

function Get-MsiArp([string]$ProductCode) {
  return "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductCode"
}

function Assert-UserClean([string]$Label) {
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe'))) "$Label left current-user executable."
  Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) "$Label left current-user ARP state."
}

function Assert-MachineClean([string]$Label,[string]$MsiProductCode='') {
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe'))) "$Label left machine executable."
  Assert-Condition (-not (Test-Path -LiteralPath $HklmNsisKey)) "$Label left NSIS machine ARP state."
  if ($MsiProductCode) { Assert-Condition (-not (Test-Path -LiteralPath (Get-MsiArp $MsiProductCode))) "$Label left MSI ProductCode ARP state." }
}

function Get-LogEvidence([string]$Path) {
  Assert-Condition (Test-Path -LiteralPath $Path -PathType Leaf) "MSI diagnostic log missing: $Path"
  $item=Get-Item -LiteralPath $Path
  Assert-Condition ($item.Length -gt 0) "MSI diagnostic log empty: $Path"
  return [pscustomobject][ordered]@{path=$Path;size_bytes=[long]$item.Length;sha256=Get-Sha256 $Path}
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
Write-UiEvidence
$actualHead=(git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"
$CurrentUserNsisPath=(Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
$PerMachineNsisPath=(Resolve-Path -LiteralPath $PerMachineNsisPath).Path
$MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
foreach ($path in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)) {
  Assert-Condition ((Get-Item -LiteralPath $path).Length -gt 0) "Package is empty: $path"
}
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
$msiArp=Get-MsiArp $productCode
Assert-UserClean 'preflight'
Assert-MachineClean 'preflight' $productCode

# nsis-current-user-success
$p=Start-Process -FilePath $CurrentUserNsisPath -PassThru
$nsisUserInstall=Drive-SuccessUi $p 'nsis-current-user-success-install' {
  (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HkcuKey)
}
Assert-Condition ($nsisUserInstall.exit_code -eq 0) "Current-user NSIS setup exit code was $($nsisUserInstall.exit_code), expected 0."
$u=Join-Path $UserRoot 'uninstall.exe'; Assert-Condition (Test-Path -LiteralPath $u) 'Current-user uninstaller missing.'
$p=Start-Process -FilePath $u -PassThru
$nsisUserUninstall=Drive-SuccessUi $p 'nsis-current-user-success-uninstall' {
  -not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HkcuKey)
} $true
Assert-Condition ($nsisUserUninstall.exit_code -eq 0) "Current-user NSIS uninstall exit code was $($nsisUserUninstall.exit_code), expected 0."
Assert-UserClean 'current-user NSIS success cleanup'

# nsis-per-machine-success
$p=Start-Process -FilePath $PerMachineNsisPath -PassThru
$nsisMachineInstall=Drive-SuccessUi $p 'nsis-per-machine-success-install' {
  (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HklmNsisKey)
}
Assert-Condition ($nsisMachineInstall.exit_code -eq 0) "Per-machine NSIS setup exit code was $($nsisMachineInstall.exit_code), expected 0."
$u=Join-Path $MachineRoot 'uninstall.exe'; Assert-Condition (Test-Path -LiteralPath $u) 'Per-machine NSIS uninstaller missing.'
$p=Start-Process -FilePath $u -PassThru
$nsisMachineUninstall=Drive-SuccessUi $p 'nsis-per-machine-success-uninstall' {
  -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HklmNsisKey)
} $true
Assert-Condition ($nsisMachineUninstall.exit_code -eq 0) "Per-machine NSIS uninstall exit code was $($nsisMachineUninstall.exit_code), expected 0."
Assert-MachineClean 'per-machine NSIS success cleanup' $productCode

# nsis-setup-cancel: documented setup user-abort code is 1.
Assert-UserClean 'NSIS cancellation preflight'
$p=Start-Process -FilePath $CurrentUserNsisPath -PassThru
$nsisCancel=Drive-CancelUi $p 'nsis-setup-cancel' 1
Assert-UserClean 'NSIS setup cancellation'

# msi-install-success / msi-uninstall-success with native /L*V logs.
$msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
$msiInstallLog=Join-Path $EvidencePath 'msi-install-success.log'
$msiUninstallLog=Join-Path $EvidencePath 'msi-uninstall-success.log'
$msiCancelLog=Join-Path $EvidencePath 'msi-install-cancel.log'
$p=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath),'/L*V',('"{0}"' -f $msiInstallLog)) -PassThru
$msiInstall=Drive-SuccessUi $p 'msi-install-success' {
  (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $msiArp)
}
Assert-Condition ($msiInstall.exit_code -eq 0) "MSI install exit code was $($msiInstall.exit_code), expected 0."
$msiInstallLogEvidence=Get-LogEvidence $msiInstallLog

$p=Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode,'/L*V',('"{0}"' -f $msiUninstallLog)) -PassThru
$msiUninstall=Drive-SuccessUi $p 'msi-uninstall-success' {
  -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $msiArp)
} $true
Assert-Condition ($msiUninstall.exit_code -eq 0) "MSI uninstall exit code was $($msiUninstall.exit_code), expected 0."
$msiUninstallLogEvidence=Get-LogEvidence $msiUninstallLog
Assert-MachineClean 'MSI success cleanup' $productCode

# msi-install-cancel: Windows Installer ERROR_INSTALL_USEREXIT = 1602.
$p=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath),'/L*V',('"{0}"' -f $msiCancelLog)) -PassThru
$msiCancel=Drive-CancelUi $p 'msi-install-cancel' 1602
$msiCancelLogEvidence=Get-LogEvidence $msiCancelLog
Assert-MachineClean 'MSI installation cancellation' $productCode

$tracked=@(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) { $tracked | Write-Host; throw 'Tracked repository drift detected during 03.15 diagnostics lifecycle.' }
Write-UiEvidence

$evidence=[ordered]@{
  schema_version=1
  package_id='PKG-03'
  task_id='03.15'
  source_commit=$SourceSha
  packages=[ordered]@{
    nsis_current_user=[ordered]@{path=$CurrentUserNsisPath;size_bytes=(Get-Item $CurrentUserNsisPath).Length;sha256=Get-Sha256 $CurrentUserNsisPath}
    nsis_per_machine=[ordered]@{path=$PerMachineNsisPath;size_bytes=(Get-Item $PerMachineNsisPath).Length;sha256=Get-Sha256 $PerMachineNsisPath}
    msi=[ordered]@{path=$MsiPath;size_bytes=(Get-Item $MsiPath).Length;sha256=Get-Sha256 $MsiPath;product_code=$productCode}
  }
  operations=[ordered]@{
    nsis_current_user_success=[ordered]@{install=$nsisUserInstall;uninstall=$nsisUserUninstall;expected_exit_code=0}
    nsis_per_machine_success=[ordered]@{install=$nsisMachineInstall;uninstall=$nsisMachineUninstall;expected_exit_code=0}
    nsis_setup_cancel=[ordered]@{result=$nsisCancel;expected_exit_code = 1;clean_state=$true}
    msi_install_success=[ordered]@{result=$msiInstall;expected_exit_code=0;log=$msiInstallLogEvidence}
    msi_uninstall_success=[ordered]@{result=$msiUninstall;expected_exit_code=0;log=$msiUninstallLogEvidence}
    msi_install_cancel=[ordered]@{result=$msiCancel;expected_exit_code=1602;log=$msiCancelLogEvidence;clean_state=$true}
  }
  ui_observations_file='ui-observations.json'
  ui_actions_file='ui-actions.json'
  msi_logging_switch='/L*V'
  nsis_native_persistent_log_claimed=$false
  nsis_uninstaller_cancel_exit_code_claimed=$false
  silent_or_passive_deployment_claimed=$false
  reboot_semantics_claimed=$false
  repair_or_reinstall_claimed=$false
  rollback_or_recovery_claimed=$false
  tracked_repository_drift_zero=$true
}
$evidenceFile=Join-Path $EvidencePath 'evidence.json'
$evidence | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath $evidenceFile -Encoding utf8NoBOM
$evidenceHash=Get-Sha256 $evidenceFile
"$evidenceHash  evidence.json" | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM
$evidence | ConvertTo-Json -Depth 14
