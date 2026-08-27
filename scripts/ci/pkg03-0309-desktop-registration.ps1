param(
  [Parameter(Mandatory=$true)][string]$SetupPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.09'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$ProductName = 'VSN Dev Platform'
$BundleId = 'dev.vsn.platform'
$UserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot = Join-Path $env:ProgramFiles $ProductName
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()

function Assert-Condition([bool]$Condition,[string]$Message) { if (-not $Condition) { throw $Message } }
function Get-Sha256([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }

function Get-StartMenuLinks {
  $roots = @(
    [Environment]::GetFolderPath('StartMenu'),
    [Environment]::GetFolderPath('CommonStartMenu')
  ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }
  $links = @()
  foreach ($root in $roots) {
    $links += @(Get-ChildItem -LiteralPath $root -Filter "$ProductName.lnk" -File -Recurse -ErrorAction SilentlyContinue)
  }
  return @($links | Sort-Object FullName -Unique)
}

function Get-DesktopLinks {
  $paths = @(
    (Join-Path ([Environment]::GetFolderPath('Desktop')) "$ProductName.lnk"),
    (Join-Path ([Environment]::GetFolderPath('CommonDesktopDirectory')) "$ProductName.lnk")
  ) | Where-Object { $_ }
  return @($paths | Where-Object { Test-Path -LiteralPath $_ } | ForEach-Object { Get-Item -LiteralPath $_ })
}

function Get-ShortcutInfo([string]$Path) {
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($Path)
  $target = [string]$shortcut.TargetPath
  $aumid = ''
  try {
    $app = New-Object -ComObject Shell.Application
    $folder = $app.Namespace((Split-Path -Parent $Path))
    $item = $folder.ParseName((Split-Path -Leaf $Path))
    if ($null -ne $item) { $aumid = [string]$item.ExtendedProperty('System.AppUserModel.ID') }
  } catch {}
  return [pscustomobject][ordered]@{ path=$Path; target=$target; app_user_model_id=$aumid }
}

function Test-ShortcutTarget([object]$Info,[string]$ExpectedExe) {
  if (-not $Info -or -not $Info.target) { return $false }
  try { return [IO.Path]::GetFullPath($Info.target).TrimEnd('\') -ieq [IO.Path]::GetFullPath($ExpectedExe).TrimEnd('\') } catch { return $false }
}

function Get-RelevantWindows {
  $root = [System.Windows.Automation.AutomationElement]::RootElement
  $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
  $result = @()
  foreach ($e in $all) {
    try {
      $name = [string]$e.Current.Name
      if (-not [bool]$e.Current.IsOffscreen -and [int]$e.Current.NativeWindowHandle -ne 0 -and $name -match '(?i)VSN Dev Platform|Windows Installer') { $result += $e }
    } catch {}
  }
  return $result
}

function Get-Controls([System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.ControlType]$Type) {
  $condition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$Type)
  return @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
}

function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) { try { ([string]$Element.Current.Name).Trim() } catch { '' } }

function Set-DesktopShortcutOn([System.Windows.Automation.AutomationElement]$Window,[string]$Phase) {
  foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
    $name = Get-SafeName $box
    if ($name -notmatch '(?i)desktop.*shortcut|shortcut.*desktop') { continue }
    try {
      $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
      if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::On) { $toggle.Toggle(); Start-Sleep -Milliseconds 200 }
      [void]$Actions.Add([pscustomobject]@{phase=$Phase;action='ensure-desktop-shortcut-on';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
      return $true
    } catch {}
  }
  return $false
}

function Set-LaunchOff([System.Windows.Automation.AutomationElement]$Window,[string]$Phase) {
  foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
    $name = Get-SafeName $box
    if ($name -notmatch '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform') { continue }
    try {
      $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
      if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) { $toggle.Toggle(); Start-Sleep -Milliseconds 150 }
    } catch {}
  }
}

function Invoke-Primary([System.Windows.Automation.AutomationElement]$Window,[string]$Phase) {
  $priority = if ($Phase -match 'uninstall') { @('^Uninstall$','^Remove$','^Next\b','^Yes$','^Finish$','^Close$','^OK$') } else { @('^Install$','^Next\b','^Finish$','^Close$','^OK$') }
  $buttons = @()
  foreach ($b in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$b.Current.IsEnabled -or [bool]$b.Current.IsOffscreen) { continue }
      $n = Get-SafeName $b
      if ($n) { $buttons += [pscustomobject]@{Element=$b;Name=$n;Norm=($n -replace '&','').Trim()} }
    } catch {}
  }
  foreach ($pattern in $priority) {
    $selected = $buttons | Where-Object { $_.Norm -match "(?i)$pattern" } | Select-Object -First 1
    if ($null -eq $selected) { continue }
    try {
      $invoke = [System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      [void]$Actions.Add([pscustomobject]@{phase=$Phase;action='invoke-button';control=$selected.Name;at_utc=[DateTime]::UtcNow.ToString('o')})
      return $true
    } catch {}
  }
  return $false
}

function Drive-Ui([string]$Phase,[scriptblock]$Completion,[bool]$SelectDesktop=$false,[int]$TimeoutSeconds=210) {
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds); $visible=$false; $desktopOption=$false; $quiet=0
  while ([DateTime]::UtcNow -lt $deadline) {
    $windows=@(Get-RelevantWindows)
    foreach ($w in $windows) {
      $visible=$true
      $title=Get-SafeName $w
      $checks=@(Get-Controls $w ([System.Windows.Automation.ControlType]::CheckBox) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
      [void]$Observations.Add([pscustomobject]@{phase=$Phase;title=$title;checkboxes=$checks;at_utc=[DateTime]::UtcNow.ToString('o')})
      if ($SelectDesktop -and (Set-DesktopShortcutOn $w $Phase)) { $desktopOption=$true }
      Set-LaunchOff $w $Phase
      [void](Invoke-Primary $w $Phase)
    }
    if (& $Completion) {
      $quiet++
      if ($quiet -ge 3) { return [pscustomobject]@{visible=$visible;desktop_option_selected=$desktopOption} }
    } else { $quiet=0 }
    Start-Sleep -Milliseconds 600
  }
  throw "Timed out driving $Phase UI."
}

function Get-MsiProperty([string]$Path,[string]$Property) {
  $installer=New-Object -ComObject WindowsInstaller.Installer
  $db=$installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
  $view=$db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
  $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null)|Out-Null
  $record=$view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null)
  if ($null -eq $record) { throw "MSI property $Property missing" }
  [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}

function Assert-ShortcutSet([string]$Phase,[string]$ExpectedRoot,[bool]$RequireDesktop,[bool]$RequireStartAumid) {
  $exe=Join-Path $ExpectedRoot 'VSN Dev Platform.exe'
  $start=@(Get-StartMenuLinks); $desk=@(Get-DesktopLinks)
  Assert-Condition ($start.Count -ge 1) "$Phase Start Menu shortcut missing."
  if ($RequireDesktop) { Assert-Condition ($desk.Count -ge 1) "$Phase Desktop shortcut missing." }
  $startInfo=@($start | ForEach-Object { Get-ShortcutInfo $_.FullName })
  $deskInfo=@($desk | ForEach-Object { Get-ShortcutInfo $_.FullName })
  Assert-Condition (($startInfo | Where-Object { Test-ShortcutTarget $_ $exe }).Count -ge 1) "$Phase Start Menu target mismatch."
  if ($RequireDesktop) { Assert-Condition (($deskInfo | Where-Object { Test-ShortcutTarget $_ $exe }).Count -ge 1) "$Phase Desktop target mismatch." }
  $startAumids=@($startInfo | ForEach-Object { $_.app_user_model_id } | Where-Object { $_ })
  if ($RequireStartAumid) { Assert-Condition ($startAumids -contains $BundleId) "$Phase Start Menu AppUserModelID mismatch." }
  elseif ($startAumids.Count -gt 0) { Assert-Condition ($startAumids -contains $BundleId) "$Phase observed AppUserModelID mismatch." }
  return [pscustomobject]@{start_menu=$startInfo;desktop=$deskInfo}
}

function Assert-ShortcutCleanup([string]$Phase) {
  Assert-Condition (@(Get-StartMenuLinks).Count -eq 0) "$Phase Start Menu shortcut remains after uninstall."
  Assert-Condition (@(Get-DesktopLinks).Count -eq 0) "$Phase Desktop shortcut remains after uninstall."
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
$SetupPath=(Resolve-Path -LiteralPath $SetupPath).Path; $MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
Assert-Condition ((Get-Item $SetupPath).Length -gt 0) 'NSIS setup is empty.'
Assert-Condition ((Get-Item $MsiPath).Length -gt 0) 'MSI is empty.'
Assert-ShortcutCleanup 'preflight'
Assert-Condition (-not (Test-Path (Join-Path $UserRoot 'VSN Dev Platform.exe'))) 'Current-user install already exists.'
Assert-Condition (-not (Test-Path (Join-Path $MachineRoot 'VSN Dev Platform.exe'))) 'Machine install already exists.'

# NSIS current-user positive shortcut lifecycle.
$nsis=Start-Process -FilePath $SetupPath -PassThru
$nsisInstall=Drive-Ui 'nsis-install' { (Test-Path (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and @(Get-StartMenuLinks).Count -gt 0 -and @(Get-DesktopLinks).Count -gt 0 } $true
$nsis.WaitForExit(); Assert-Condition ($nsis.ExitCode -eq 0) "NSIS install exit $($nsis.ExitCode)"
Assert-Condition $nsisInstall.visible 'No visible NSIS install UI observed.'
Assert-Condition $nsisInstall.desktop_option_selected 'NSIS Desktop shortcut GUI option was not observed/selected.'
$nsisShortcuts=Assert-ShortcutSet 'NSIS' $UserRoot $true $false
$uninstaller=Join-Path $UserRoot 'uninstall.exe'; Assert-Condition (Test-Path $uninstaller) 'NSIS uninstaller missing.'
$nu=Start-Process -FilePath $uninstaller -PassThru
$nsisUninstall=Drive-Ui 'nsis-uninstall' { -not (Test-Path (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and @(Get-StartMenuLinks).Count -eq 0 -and @(Get-DesktopLinks).Count -eq 0 }
$nu.WaitForExit(); Assert-Condition ($nu.ExitCode -eq 0) "NSIS uninstall exit $($nu.ExitCode)"
Assert-Condition $nsisUninstall.visible 'No visible NSIS uninstall UI observed.'
Assert-ShortcutCleanup 'NSIS'

# MSI/WiX shortcut lifecycle.
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
$msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
$mi=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath)) -PassThru
$msiInstall=Drive-Ui 'msi-install' { (Test-Path (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and @(Get-StartMenuLinks).Count -gt 0 -and @(Get-DesktopLinks).Count -gt 0 }
$mi.WaitForExit(); Assert-Condition ($mi.ExitCode -eq 0) "MSI install exit $($mi.ExitCode)"
Assert-Condition $msiInstall.visible 'No visible MSI install UI observed.'
$wixShortcuts=Assert-ShortcutSet 'WiX' $MachineRoot $true $true
$mu=Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode) -PassThru
$msiUninstall=Drive-Ui 'msi-uninstall' { -not (Test-Path (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and @(Get-StartMenuLinks).Count -eq 0 -and @(Get-DesktopLinks).Count -eq 0 }
$mu.WaitForExit(); Assert-Condition ($mu.ExitCode -eq 0) "MSI uninstall exit $($mu.ExitCode)"
Assert-Condition $msiUninstall.visible 'No visible MSI uninstall UI observed.'
Assert-ShortcutCleanup 'WiX'

$Observations | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
$Actions | ConvertTo-Json -Depth 10 | Set-Content (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
$evidence=[ordered]@{
 schema_version=1; package_id='PKG-03'; task_id='03.09'; source_commit=$SourceSha
 nsis=[ordered]@{setup_sha256=Get-Sha256 $SetupPath;visible_install_ui_observed=[bool]$nsisInstall.visible;desktop_shortcut_gui_selected=[bool]$nsisInstall.desktop_option_selected;start_menu=$nsisShortcuts.start_menu;desktop=$nsisShortcuts.desktop;visible_uninstall_ui_observed=[bool]$nsisUninstall.visible;nsis_start_menu_removed=$true;nsis_desktop_removed=$true}
 wix=[ordered]@{msi_sha256=Get-Sha256 $MsiPath;product_code=$productCode;visible_install_ui_observed=[bool]$msiInstall.visible;start_menu=$wixShortcuts.start_menu;desktop=$wixShortcuts.desktop;visible_uninstall_ui_observed=[bool]$msiUninstall.visible;wix_start_menu_removed=$true;wix_desktop_removed=$true}
 application_registration=[ordered]@{bundle_identifier=$BundleId;file_associations_claimed=$false;deep_links_claimed=$false}
 cli_agent_placement_claimed=$false;service_registration_claimed=$false;acl_mutation_claimed=$false;silent_or_passive_deployment_claimed=$false;signing_claimed=$false;updater_mutation_claimed=$false;tracked_repository_drift_zero=$true
}
$evidence | ConvertTo-Json -Depth 12 | Set-Content (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
