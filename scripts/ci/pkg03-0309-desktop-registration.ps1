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
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class Vsn0309NativeUi {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr GetAncestor(IntPtr hWnd, uint gaFlags);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern int GetDlgCtrlID(IntPtr hWnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindow(IntPtr hWnd);
}
'@

$ProductName = 'VSN Dev Platform'
$BundleId = 'dev.vsn.platform'
$UserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot = Join-Path $env:ProgramFiles $ProductName
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()
$ObservationFingerprints = [System.Collections.Generic.HashSet[string]]::new()
$TerminalFallbackRoots = [System.Collections.Generic.HashSet[string]]::new()

function Assert-Condition([bool]$Condition,[string]$Message) {
  if (-not $Condition) { throw $Message }
}

function Get-Sha256([string]$Path) {
  (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) {
  try { ([string]$Element.Current.Name).Trim() } catch { '' }
}

function Write-UiArtifacts {
  New-Item -ItemType Directory -Force $EvidencePath | Out-Null
  ConvertTo-Json -InputObject @($Observations | ForEach-Object { $_ }) -Depth 14 |
    Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
  ConvertTo-Json -InputObject @($Actions | ForEach-Object { $_ }) -Depth 10 |
    Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
}

function Write-FailureEvidence([string]$Phase,[string]$Message) {
  Write-UiArtifacts
  [ordered]@{
    schema_version = 1
    package_id = 'PKG-03'
    task_id = '03.09'
    source_commit = $SourceSha
    diagnostic_only = $true
    failed_phase = $Phase
    message = $Message
    captured_at_utc = [DateTime]::UtcNow.ToString('o')
  } | ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
}

function Get-StartMenuLinks {
  $roots = @(
    [Environment]::GetFolderPath('StartMenu'),
    [Environment]::GetFolderPath('CommonStartMenu')
  ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }
  $links = @()
  foreach ($root in $roots) {
    $links += @(Get-ChildItem -LiteralPath $root -Filter "$ProductName.lnk" -File -Recurse -ErrorAction SilentlyContinue)
  }
  @($links | Sort-Object FullName -Unique)
}

function Get-DesktopLinks {
  $paths = @(
    (Join-Path ([Environment]::GetFolderPath('Desktop')) "$ProductName.lnk"),
    (Join-Path ([Environment]::GetFolderPath('CommonDesktopDirectory')) "$ProductName.lnk")
  ) | Where-Object { $_ }
  @($paths | Where-Object { Test-Path -LiteralPath $_ } | ForEach-Object { Get-Item -LiteralPath $_ })
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
  [pscustomobject][ordered]@{ path=$Path; target=$target; app_user_model_id=$aumid }
}

function Test-ShortcutTarget([object]$Info,[string]$ExpectedExe) {
  if (-not $Info -or -not $Info.target) { return $false }
  try {
    [IO.Path]::GetFullPath($Info.target).TrimEnd('\') -ieq [IO.Path]::GetFullPath($ExpectedExe).TrimEnd('\')
  } catch { $false }
}

function Get-Controls(
  [System.Windows.Automation.AutomationElement]$Window,
  [System.Windows.Automation.ControlType]$Type
) {
  $condition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,$Type
  )
  @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
}

function Get-RelevantWindows([int]$RootProcessId) {
  $snapshot = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId, ParentProcessId)
  $family = [System.Collections.Generic.HashSet[int]]::new()
  [void]$family.Add($RootProcessId)
  do {
    $changed = $false
    foreach ($proc in $snapshot) {
      $processId = [int]$proc.ProcessId
      $parentId = [int]$proc.ParentProcessId
      if ($family.Contains($parentId) -and -not $family.Contains($processId)) {
        [void]$family.Add($processId)
        $changed = $true
      }
    }
  } while ($changed)

  $root = [System.Windows.Automation.AutomationElement]::RootElement
  $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
  $result = @()
  foreach ($element in $all) {
    try {
      $name = [string]$element.Current.Name
      $windowProcessId = [int]$element.Current.ProcessId
      $visible = -not [bool]$element.Current.IsOffscreen
      $handle = [int]$element.Current.NativeWindowHandle
      $titleFallback = $name -match '(?i)VSN Dev Platform|Windows Installer'
      if ($visible -and $handle -ne 0 -and ($family.Contains($windowProcessId) -or $titleFallback)) {
        $result += $element
      }
    } catch {}
  }
  @($result)
}

function Record-Window([string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $controls = @()
  $all = $Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
  foreach ($element in $all) {
    try {
      $name = Get-SafeName $element
      $patterns = @()
      try { $patterns = @($element.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }) } catch {}
      $controls += [pscustomobject][ordered]@{
        control_type = [string]$element.Current.ControlType.ProgrammaticName
        name = $name
        automation_id = [string]$element.Current.AutomationId
        class_name = [string]$element.Current.ClassName
        framework_id = [string]$element.Current.FrameworkId
        enabled = [bool]$element.Current.IsEnabled
        offscreen = [bool]$element.Current.IsOffscreen
        native_window_handle = [int]$element.Current.NativeWindowHandle
        patterns = $patterns
      }
    } catch {}
  }
  $title = Get-SafeName $Window
  $windowProcessId = 0
  try { $windowProcessId = [int]$Window.Current.ProcessId } catch {}
  $summary = @($controls | Where-Object { $_.name } | ForEach-Object { "$($_.control_type):$($_.name)" })
  $fingerprint = "${Phase}|$windowProcessId|$title|$($summary -join '|')"
  if (-not $ObservationFingerprints.Add($fingerprint)) { return }
  [void]$Observations.Add([pscustomobject][ordered]@{
    phase=$Phase; process_id=$windowProcessId; title=$title; controls=$controls; at_utc=[DateTime]::UtcNow.ToString('o')
  })
  Write-UiArtifacts
}

function Set-LaunchOff([System.Windows.Automation.AutomationElement]$Window,[string]$Phase) {
  foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
    $name = Get-SafeName $box
    if ($name -notmatch '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform') { continue }
    try {
      $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
      if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) {
        $toggle.Toggle(); Start-Sleep -Milliseconds 180
      }
      [void]$Actions.Add([pscustomobject][ordered]@{
        phase=$Phase; action='ensure-launch-off'; control=$name; at_utc=[DateTime]::UtcNow.ToString('o')
      })
      Write-UiArtifacts
    } catch {}
  }
}

function Select-DesktopShortcut([System.Windows.Automation.AutomationElement]$Window,[string]$Phase) {
  foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
    $name = Get-SafeName $box
    if ($name -notmatch '(?i)(create\s+)?desktop.*shortcut|shortcut.*desktop') { continue }
    try {
      $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
      if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::On) {
        $toggle.Toggle(); Start-Sleep -Milliseconds 220
      }
      [void]$Actions.Add([pscustomobject][ordered]@{
        phase=$Phase; action='select-desktop-shortcut'; control=$name; at_utc=[DateTime]::UtcNow.ToString('o')
      })
      Write-UiArtifacts
      return $true
    } catch {}
  }
  return $false
}

function Invoke-TerminalFallback(
  [System.Windows.Automation.AutomationElement]$Window,
  [System.Windows.Automation.AutomationElement]$Button,
  [string]$ButtonName,
  [string]$Phase,
  [bool]$Allowed
) {
  if (-not $Allowed) { return }
  try { $buttonHandle = [IntPtr][int]$Button.Current.NativeWindowHandle } catch { return }
  if ($buttonHandle -eq [IntPtr]::Zero -or -not [Vsn0309NativeUi]::IsWindow($buttonHandle)) { return }
  $rootHandle = [Vsn0309NativeUi]::GetAncestor($buttonHandle,[uint32]2)
  if ($rootHandle -eq [IntPtr]::Zero) {
    try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return }
  }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0309NativeUi]::IsWindow($rootHandle)) { return }
  $key = "${Phase}:$($rootHandle.ToInt64())"
  if (-not $TerminalFallbackRoots.Add($key)) { return }
  $controlId = [Vsn0309NativeUi]::GetDlgCtrlID($buttonHandle)
  if ($controlId -gt 0) {
    [void][Vsn0309NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle)
    [void]$Actions.Add([pscustomobject][ordered]@{
      phase=$Phase; action='native-wm-command-fallback'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
    })
    Start-Sleep -Milliseconds 450
  }
  if ([Vsn0309NativeUi]::IsWindow($rootHandle)) {
    [void][Vsn0309NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$Actions.Add([pscustomobject][ordered]@{
      phase=$Phase; action='native-wm-close-terminal-fallback'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
    })
  }
  Write-UiArtifacts
}

function Invoke-Primary(
  [System.Windows.Automation.AutomationElement]$Window,
  [string]$Phase,
  [bool]$TerminalFallbackAllowed
) {
  $priority = if ($Phase -match 'uninstall') {
    @('^Uninstall$','^Remove$','^Next\b','^Yes$','^Finish$','^Close$','^OK$')
  } else {
    @('^Install$','^Next\b','^Finish$','^Close$','^OK$')
  }
  $buttons = @()
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name = Get-SafeName $button
      if ($name) { $buttons += [pscustomobject]@{Element=$button;Name=$name;Norm=($name -replace '&','').Trim()} }
    } catch {}
  }
  foreach ($pattern in $priority) {
    $selected = $buttons | Where-Object { $_.Norm -match "(?i)$pattern" } | Select-Object -First 1
    if ($null -eq $selected) { continue }
    try {
      $invoke = [System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      [void]$Actions.Add([pscustomobject][ordered]@{
        phase=$Phase; action='invoke-button'; control=$selected.Name; at_utc=[DateTime]::UtcNow.ToString('o')
      })
      Write-UiArtifacts
      if ($selected.Norm -match '(?i)^(Finish|Close)$') {
        Start-Sleep -Milliseconds 350
        Invoke-TerminalFallback $Window $selected.Element $selected.Name $Phase $TerminalFallbackAllowed
      }
      return $selected.Norm
    } catch {}
  }
  return $null
}

function Drive-Ui(
  [System.Diagnostics.Process]$RootProcess,
  [string]$Phase,
  [scriptblock]$Completion,
  [bool]$SelectDesktop = $false,
  [int]$TimeoutSeconds = 210
) {
  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible = $false
  $desktopSelected = $false
  $quietCompletePolls = 0
  while ([DateTime]::UtcNow -lt $deadline) {
    $completionNow = [bool](& $Completion)
    $windows = @(Get-RelevantWindows $RootProcess.Id)
    if ($windows.Count -eq 0) {
      if ($completionNow) {
        $quietCompletePolls++
        if ($quietCompletePolls -ge 3) {
          Write-UiArtifacts
          return [pscustomobject]@{visible=$visible;desktop_option_selected=$desktopSelected;terminal_closed=$true}
        }
      } else { $quietCompletePolls = 0 }
      Start-Sleep -Milliseconds 500
      continue
    }

    $visible = $true
    $quietCompletePolls = 0
    foreach ($window in $windows) {
      try { $window.SetFocus() } catch {}
      Record-Window $Phase $window
      if ($SelectDesktop -and -not $desktopSelected) {
        if (Select-DesktopShortcut $window $Phase) { $desktopSelected = $true }
      }
      Set-LaunchOff $window $Phase

      $completionNow = [bool](& $Completion)
      $terminalFallbackAllowed = $completionNow
      if ($Phase -eq 'nsis-install' -and $SelectDesktop -and $desktopSelected) {
        $terminalFallbackAllowed = (
          (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and
          @(Get-StartMenuLinks).Count -gt 0
        )
      }
      if ($Phase -eq 'msi-install') {
        # The stock WiX Finish dialog can remain visible after the installed
        # executable and Start Menu shortcut exist. Closing that terminal UI
        # must not be gated on the Desktop shortcut that is asserted directly
        # after process exit; otherwise a missing Desktop shortcut becomes a
        # timeout instead of a precise contract failure.
        $terminalFallbackAllowed = (
          (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and
          @(Get-StartMenuLinks).Count -gt 0
        )
      }
      [void](Invoke-Primary $window $Phase $terminalFallbackAllowed)
      Start-Sleep -Milliseconds 800
      break
    }
  }
  $message = "Timed out driving $Phase UI."
  Write-FailureEvidence $Phase $message
  throw $message
}

function Wait-ForExitBounded([System.Diagnostics.Process]$Process,[string]$Phase,[int]$TimeoutSeconds=20) {
  Wait-Process -Id $Process.Id -Timeout $TimeoutSeconds -ErrorAction SilentlyContinue
  try { $Process.Refresh() } catch {}
  $exited = $false
  try { $exited = $Process.HasExited } catch { $exited = $true }
  Assert-Condition $exited "$Phase root process did not exit."
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
  $exe = Join-Path $ExpectedRoot 'VSN Dev Platform.exe'
  $start = @(Get-StartMenuLinks)
  $desk = @(Get-DesktopLinks)
  Assert-Condition ($start.Count -ge 1) "$Phase Start Menu shortcut missing."
  if ($RequireDesktop) { Assert-Condition ($desk.Count -ge 1) "$Phase Desktop shortcut missing." }
  $startInfo = @($start | ForEach-Object { Get-ShortcutInfo $_.FullName })
  $deskInfo = @($desk | ForEach-Object { Get-ShortcutInfo $_.FullName })
  Assert-Condition (($startInfo | Where-Object { Test-ShortcutTarget $_ $exe }).Count -ge 1) "$Phase Start Menu target mismatch."
  if ($RequireDesktop) {
    Assert-Condition (($deskInfo | Where-Object { Test-ShortcutTarget $_ $exe }).Count -ge 1) "$Phase Desktop target mismatch."
  }
  $startAumids = @($startInfo | ForEach-Object { $_.app_user_model_id } | Where-Object { $_ })
  if ($RequireStartAumid) {
    Assert-Condition ($startAumids -contains $BundleId) "$Phase Start Menu AppUserModelID mismatch."
  } elseif ($startAumids.Count -gt 0) {
    Assert-Condition ($startAumids -contains $BundleId) "$Phase observed AppUserModelID mismatch."
  }
  [pscustomobject]@{start_menu=$startInfo;desktop=$deskInfo}
}

function Assert-ShortcutCleanup([string]$Phase) {
  Assert-Condition (@(Get-StartMenuLinks).Count -eq 0) "$Phase Start Menu shortcut remains after uninstall."
  Assert-Condition (@(Get-DesktopLinks).Count -eq 0) "$Phase Desktop shortcut remains after uninstall."
}

function Assert-NoCliAgent([string]$Root,[string]$Phase) {
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $Root 'bin\vsn.exe'))) "$Phase illegally placed bin/vsn.exe before 03.10."
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $Root 'bin\vsn-agent.exe'))) "$Phase illegally placed bin/vsn-agent.exe before 03.10."
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
Write-UiArtifacts
$actualHead = (git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"
$SetupPath=(Resolve-Path -LiteralPath $SetupPath).Path
$MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
Assert-Condition ((Get-Item -LiteralPath $SetupPath).Length -gt 0) 'NSIS setup is empty.'
Assert-Condition ((Get-Item -LiteralPath $MsiPath).Length -gt 0) 'MSI is empty.'
Assert-ShortcutCleanup 'preflight'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe'))) 'Current-user install already exists.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe'))) 'Machine install already exists.'

# NSIS current-user positive Desktop shortcut lifecycle.
$nsis=Start-Process -FilePath $SetupPath -PassThru
$nsisInstall=Drive-Ui $nsis 'nsis-install' {
  (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and
  @(Get-StartMenuLinks).Count -gt 0 -and
  @(Get-DesktopLinks).Count -gt 0
} $true
Wait-ForExitBounded $nsis 'NSIS install'
Assert-Condition ($nsis.ExitCode -eq 0) "NSIS install exit $($nsis.ExitCode)"
Assert-Condition $nsisInstall.visible 'No visible NSIS install UI observed.'
Assert-Condition $nsisInstall.desktop_option_selected 'NSIS Desktop shortcut GUI option was not observed/selected.'
$nsisShortcuts=Assert-ShortcutSet 'NSIS' $UserRoot $true $false
Assert-NoCliAgent $UserRoot 'NSIS install'

$uninstaller=Join-Path $UserRoot 'uninstall.exe'
Assert-Condition (Test-Path -LiteralPath $uninstaller) 'NSIS uninstaller missing.'
$nu=Start-Process -FilePath $uninstaller -PassThru
$nsisUninstall=Drive-Ui $nu 'nsis-uninstall' {
  -not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and
  @(Get-StartMenuLinks).Count -eq 0 -and
  @(Get-DesktopLinks).Count -eq 0
}
Wait-ForExitBounded $nu 'NSIS uninstall'
Assert-Condition ($nu.ExitCode -eq 0) "NSIS uninstall exit $($nu.ExitCode)"
Assert-Condition $nsisUninstall.visible 'No visible NSIS uninstall UI observed.'
Assert-ShortcutCleanup 'NSIS'

# MSI/WiX Desktop shortcut and Start Menu registration lifecycle.
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
$msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
$mi=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath)) -PassThru
$msiInstall=Drive-Ui $mi 'msi-install' {
  (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and
  @(Get-StartMenuLinks).Count -gt 0 -and
  @(Get-DesktopLinks).Count -gt 0
}
Wait-ForExitBounded $mi 'MSI install'
Assert-Condition ($mi.ExitCode -eq 0) "MSI install exit $($mi.ExitCode)"
Assert-Condition $msiInstall.visible 'No visible MSI install UI observed.'
$wixShortcuts=Assert-ShortcutSet 'WiX' $MachineRoot $true $true
Assert-NoCliAgent $MachineRoot 'MSI install'

$mu=Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode) -PassThru
$msiUninstall=Drive-Ui $mu 'msi-uninstall' {
  -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and
  @(Get-StartMenuLinks).Count -eq 0 -and
  @(Get-DesktopLinks).Count -eq 0
}
Wait-ForExitBounded $mu 'MSI uninstall'
Assert-Condition ($mu.ExitCode -eq 0) "MSI uninstall exit $($mu.ExitCode)"
Assert-Condition $msiUninstall.visible 'No visible MSI uninstall UI observed.'
Assert-ShortcutCleanup 'WiX'

$tracked=@(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) { $tracked | Write-Host; throw 'Tracked repository drift detected during 03.09 desktop registration lifecycle.' }

Write-UiArtifacts
$evidence=[ordered]@{
  schema_version=1
  package_id='PKG-03'
  task_id='03.09'
  source_commit=$SourceSha
  nsis=[ordered]@{
    setup_sha256=Get-Sha256 $SetupPath
    visible_install_ui_observed=[bool]$nsisInstall.visible
    desktop_shortcut_gui_selected=[bool]$nsisInstall.desktop_option_selected
    start_menu=$nsisShortcuts.start_menu
    desktop=$nsisShortcuts.desktop
    visible_uninstall_ui_observed=[bool]$nsisUninstall.visible
    nsis_start_menu_removed=$true
    nsis_desktop_removed=$true
  }
  wix=[ordered]@{
    msi_sha256=Get-Sha256 $MsiPath
    product_code=$productCode
    visible_install_ui_observed=[bool]$msiInstall.visible
    start_menu=$wixShortcuts.start_menu
    desktop=$wixShortcuts.desktop
    visible_uninstall_ui_observed=[bool]$msiUninstall.visible
    wix_start_menu_removed=$true
    wix_desktop_removed=$true
  }
  application_registration=[ordered]@{
    bundle_identifier=$BundleId
    file_associations_claimed=$false
    deep_links_claimed=$false
  }
  cli_agent_placement_claimed=$false
  service_registration_claimed=$false
  acl_mutation_claimed=$false
  silent_or_passive_deployment_claimed=$false
  signing_claimed=$false
  updater_mutation_claimed=$false
  tracked_repository_drift_zero=$true
}
$evidence | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
