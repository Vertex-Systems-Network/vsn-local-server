param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.13'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'pkg03-0313-snapshot.ps1')

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Vsn0313NativeUi {
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
$HklmKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$SnapshotsPath = Join-Path $EvidencePath 'snapshots'
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()
$TerminalFallbackRoots = [System.Collections.Generic.HashSet[string]]::new()

function Assert-Condition([bool]$Condition,[string]$Message) {
  if (-not $Condition) { throw $Message }
}

function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) {
  try { ([string]$Element.Current.Name).Trim() } catch { '' }
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
  $snapshot = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId,ParentProcessId)
  $family = [System.Collections.Generic.HashSet[int]]::new()
  [void]$family.Add($RootProcessId)
  do {
    $changed = $false
    foreach ($process in $snapshot) {
      $processId = [int]$process.ProcessId
      $parentId = [int]$process.ParentProcessId
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

function Record-Window([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $buttons = @(
    Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) |
      ForEach-Object { Get-SafeName $_ } | Where-Object { $_ }
  )
  $checks = @(
    Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox) |
      ForEach-Object { Get-SafeName $_ } | Where-Object { $_ }
  )
  [void]$Observations.Add([pscustomobject][ordered]@{
    lifecycle = $Lifecycle
    phase = $Phase
    process_id = [int]$Window.Current.ProcessId
    title = Get-SafeName $Window
    buttons = $buttons
    checkboxes = $checks
    at_utc = [DateTime]::UtcNow.ToString('o')
  })
  Write-UiArtifacts
}

function Set-SafetyCheckboxes([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
    $name = Get-SafeName $box
    $mustBeOff = (
      ($Phase -eq 'install' -and $name -match '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform') -or
      ($Phase -eq 'uninstall' -and $name -match '(?i)delete.*(app.*data|data)|remove.*(app.*data|user.*data)')
    )
    if (-not $mustBeOff) { continue }
    try {
      $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
      if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) {
        $toggle.Toggle()
        Start-Sleep -Milliseconds 180
      }
      [void]$Actions.Add([pscustomobject][ordered]@{
        lifecycle=$Lifecycle; phase=$Phase; action='ensure-safety-checkbox-off'; control=$name; at_utc=[DateTime]::UtcNow.ToString('o')
      })
      Write-UiArtifacts
    } catch {}
  }
}

function Invoke-TerminalFallback(
  [string]$Lifecycle,
  [string]$Phase,
  [System.Windows.Automation.AutomationElement]$Window,
  [System.Windows.Automation.AutomationElement]$Button,
  [string]$ButtonName,
  [bool]$CompletionReached
) {
  if (-not $CompletionReached) { return $false }
  try { $buttonHandle = [IntPtr][int]$Button.Current.NativeWindowHandle } catch { return $false }
  if ($buttonHandle -eq [IntPtr]::Zero -or -not [Vsn0313NativeUi]::IsWindow($buttonHandle)) { return $false }
  $rootHandle = [Vsn0313NativeUi]::GetAncestor($buttonHandle,[uint32]2)
  if ($rootHandle -eq [IntPtr]::Zero) {
    try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0313NativeUi]::IsWindow($rootHandle)) { return $false }
  $key = "${Lifecycle}:${Phase}:$($rootHandle.ToInt64())"
  if (-not $TerminalFallbackRoots.Add($key)) { return $false }

  $acted = $false
  $controlId = [Vsn0313NativeUi]::GetDlgCtrlID($buttonHandle)
  if ($controlId -gt 0) {
    [void][Vsn0313NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle)
    [void]$Actions.Add([pscustomobject][ordered]@{
      lifecycle=$Lifecycle; phase=$Phase; action='native-wm-command-terminal'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
    })
    Write-UiArtifacts
    $acted = $true
    Start-Sleep -Milliseconds 350
  }
  if ([Vsn0313NativeUi]::IsWindow($rootHandle)) {
    [void][Vsn0313NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$Actions.Add([pscustomobject][ordered]@{
      lifecycle=$Lifecycle; phase=$Phase; action='native-wm-close-terminal'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
    })
    Write-UiArtifacts
    $acted = $true
  }
  return $acted
}

function Invoke-PrimaryButton(
  [string]$Lifecycle,
  [string]$Phase,
  [System.Windows.Automation.AutomationElement]$Window,
  [bool]$CompletionReached
) {
  $priority = if ($Phase -eq 'install') {
    @('^Install$','^Next\b','^Finish$','^OK$','^Close$')
  } else {
    @('^Remove$','^Uninstall$','^Next\b','^Yes$','^Finish$','^OK$','^Close$')
  }
  $candidates = @()
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name = Get-SafeName $button
      $automationId = [string]$button.Current.AutomationId
      $nativeHandle = [int]$button.Current.NativeWindowHandle
      if ($nativeHandle -eq 0 -and $automationId -match '^(?i:Close|Minimize|Maximize)$') { continue }
      if ($name) {
        $candidates += [pscustomobject]@{ Element=$button; Name=$name; Normalized=($name -replace '&','').Trim() }
      }
    } catch {}
  }
  foreach ($pattern in $priority) {
    $selected = $candidates | Where-Object { $_.Normalized -match "(?i)$pattern" } | Select-Object -First 1
    if ($null -eq $selected) { continue }

    # WiX Finish/Close is itself the terminal wizard affordance. Prefer the
    # native HWND path when one exists, but do not treat a zero/invalid HWND as
    # success. Newer Windows runner images can expose the button through UIA
    # without a native child HWND, so fall through to InvokePattern in that case.
    if ($Lifecycle -eq 'wix-per-machine' -and $selected.Normalized -match '(?i)^(Finish|Close)$') {
      $nativeHandled = Invoke-TerminalFallback $Lifecycle $Phase $Window $selected.Element $selected.Name $true
      if ($nativeHandled) { return $selected.Normalized }
    }

    try {
      $invoke = [System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      [void]$Actions.Add([pscustomobject][ordered]@{
        lifecycle=$Lifecycle; phase=$Phase; action='invoke-button'; control=$selected.Name; at_utc=[DateTime]::UtcNow.ToString('o')
      })
      Write-UiArtifacts
      if ($selected.Normalized -match '(?i)^(Finish|Close|OK)$') {
        Start-Sleep -Milliseconds 250
        [void](Invoke-TerminalFallback $Lifecycle $Phase $Window $selected.Element $selected.Name $CompletionReached)
      }
      return $selected.Normalized
    } catch {}
  }
  return $null
}

function Drive-InstallerUi(
  [string]$Lifecycle,
  [string]$Phase,
  [System.Diagnostics.Process]$RootProcess,
  [scriptblock]$Completion,
  [int]$TimeoutSeconds = 210
) {
  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible = $false
  $quietCompletePolls = 0
  while ([DateTime]::UtcNow -lt $deadline) {
    $completionNow = [bool](& $Completion)
    $windows = @(Get-RelevantWindows $RootProcess.Id)
    if ($windows.Count -eq 0) {
      if ($completionNow) {
        $quietCompletePolls++
        if ($quietCompletePolls -ge 3) {
          return [pscustomobject]@{ visible=$visible; completion=$true; terminal_closed=$true }
        }
      } else {
        $quietCompletePolls = 0
      }
      Start-Sleep -Milliseconds 450
      continue
    }

    $visible = $true
    $quietCompletePolls = 0
    $window = $windows[0]
    try { $window.SetFocus() } catch {}
    Record-Window $Lifecycle $Phase $window
    Set-SafetyCheckboxes $Lifecycle $Phase $window
    $completionNow = [bool](& $Completion)
    [void](Invoke-PrimaryButton $Lifecycle $Phase $window $completionNow)
    Start-Sleep -Milliseconds 700
  }
  Write-UiArtifacts
  throw "Timed out driving 03.13 $Lifecycle $Phase UI."
}

function Wait-ForExitBounded([System.Diagnostics.Process]$Process,[string]$Label,[int]$TimeoutSeconds=30) {
  $exited = $Process.WaitForExit($TimeoutSeconds * 1000)
  Assert-Condition $exited "$Label root process did not exit within $TimeoutSeconds seconds."
  $Process.Refresh()
}

function Get-MsiProperty([string]$Path,[string]$Property) {
  $installer = New-Object -ComObject WindowsInstaller.Installer
  $db = $installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
  $view = $db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
  $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null) | Out-Null
  $record = $view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null)
  if ($null -eq $record) { throw "MSI property $Property missing" }
  [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}

function Write-UiArtifacts {
  ConvertTo-Json -InputObject @($Observations | ForEach-Object { $_ }) -Depth 10 |
    Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
  ConvertTo-Json -InputObject @($Actions | ForEach-Object { $_ }) -Depth 8 |
    Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
}

function Assert-CleanUserInstallState([string]$Label) {
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe'))) "$Label current-user executable already exists."
  Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) "$Label HKCU uninstall registration already exists."
}

function Assert-CleanMachineInstallState([string]$Label) {
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe'))) "$Label machine executable already exists."
  Assert-Condition (-not (Test-Path -LiteralPath $HklmKey)) "$Label HKLM NSIS uninstall registration already exists."
}

function Invoke-ProtectedLifecycle(
  [string]$Lifecycle,
  [scriptblock]$StartInstall,
  [scriptblock]$InstallCompletion,
  [scriptblock]$StartUninstall,
  [scriptblock]$UninstallCompletion
) {
  $baselinePath = Join-Path $SnapshotsPath "$Lifecycle-baseline.json"
  $postInstallPath = Join-Path $SnapshotsPath "$Lifecycle-post-install.json"
  $postUninstallPath = Join-Path $SnapshotsPath "$Lifecycle-post-uninstall.json"

  $baseline = Write-Pkg0313Snapshot -Path $baselinePath
  $installProcess = & $StartInstall
  $installUi = Drive-InstallerUi $Lifecycle 'install' $installProcess $InstallCompletion
  Wait-ForExitBounded $installProcess "$Lifecycle install"
  Assert-Condition ($installProcess.ExitCode -eq 0) "$Lifecycle install exited with code $($installProcess.ExitCode)."
  Assert-Condition $installUi.visible "$Lifecycle install did not expose visible UI."

  $postInstall = Write-Pkg0313Snapshot -Path $postInstallPath
  Assert-Pkg0313SnapshotEqual -BaselinePath $baselinePath -CandidatePath $postInstallPath -Label "$Lifecycle install"

  $uninstallProcess = & $StartUninstall
  $uninstallUi = Drive-InstallerUi $Lifecycle 'uninstall' $uninstallProcess $UninstallCompletion
  Wait-ForExitBounded $uninstallProcess "$Lifecycle uninstall"
  Assert-Condition ($uninstallProcess.ExitCode -eq 0) "$Lifecycle uninstall exited with code $($uninstallProcess.ExitCode)."
  Assert-Condition $uninstallUi.visible "$Lifecycle uninstall did not expose visible UI."

  $postUninstall = Write-Pkg0313Snapshot -Path $postUninstallPath
  Assert-Pkg0313SnapshotEqual -BaselinePath $baselinePath -CandidatePath $postUninstallPath -Label "$Lifecycle uninstall"

  Write-UiArtifacts
  return [pscustomobject][ordered]@{
    lifecycle = $Lifecycle
    visible_install_ui_observed = [bool]$installUi.visible
    visible_uninstall_ui_observed = [bool]$uninstallUi.visible
    install_exit_code = [int]$installProcess.ExitCode
    uninstall_exit_code = [int]$uninstallProcess.ExitCode
    baseline_sha256 = $baseline.sha256
    post_install_sha256 = $postInstall.sha256
    post_uninstall_sha256 = $postUninstall.sha256
    protected_state_equal_after_install = $true
    protected_state_equal_after_uninstall = $true
    application_launch_disabled = $true
  }
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
New-Item -ItemType Directory -Force $SnapshotsPath | Out-Null
$actualHead = (git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"

$CurrentUserNsisPath = (Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
$PerMachineNsisPath = (Resolve-Path -LiteralPath $PerMachineNsisPath).Path
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
foreach ($package in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)) {
  Assert-Condition ((Get-Item -LiteralPath $package).Length -gt 0) "Installer package is empty: $package"
}

Assert-CleanUserInstallState 'preflight'
Assert-CleanMachineInstallState 'preflight'

# Run WiX first on the fresh runner so stock AppSearch cannot inherit a
# current-user install-directory hint from the NSIS lifecycle.
$productCode = Get-MsiProperty $MsiPath 'ProductCode'
$msiArpKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productCode"
Assert-Condition (-not (Test-Path -LiteralPath $msiArpKey)) 'MSI ProductCode ARP entry already exists before lifecycle.'
$msiexec = Join-Path $env:SystemRoot 'System32\msiexec.exe'
$wix = Invoke-ProtectedLifecycle 'wix-per-machine' `
  { Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath)) -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $msiArpKey) } `
  { Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode) -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $msiArpKey) }
Assert-Condition (-not (Test-Path -LiteralPath $msiArpKey)) 'MSI ProductCode ARP entry remains after lifecycle.'
Assert-CleanMachineInstallState 'after WiX lifecycle'

$currentUser = Invoke-ProtectedLifecycle 'nsis-current-user' `
  { Start-Process -FilePath $CurrentUserNsisPath -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath (Join-Path $UserRoot 'uninstall.exe')) -and (Test-Path -LiteralPath $HkcuKey) } `
  { Start-Process -FilePath (Join-Path $UserRoot 'uninstall.exe') -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HkcuKey) }
Assert-CleanUserInstallState 'after current-user lifecycle'

$perMachine = Invoke-ProtectedLifecycle 'nsis-per-machine' `
  { Start-Process -FilePath $PerMachineNsisPath -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath (Join-Path $MachineRoot 'uninstall.exe')) -and (Test-Path -LiteralPath $HklmKey) } `
  { Start-Process -FilePath (Join-Path $MachineRoot 'uninstall.exe') -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HklmKey) }
Assert-CleanMachineInstallState 'after per-machine lifecycle'

$tracked = @(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) {
  $tracked | Write-Host
  throw 'Tracked repository drift detected during 03.13 non-mutation lifecycle.'
}

Write-UiArtifacts
$evidence = [ordered]@{
  schema_version = 1
  package_id = 'PKG-03'
  task_id = '03.13'
  source_commit = $SourceSha
  protected_surfaces = @('firewall','hosts','resolver','trust')
  snapshot_policy = [ordered]@{
    fail_closed = $true
    automatic_repair = $false
    application_launch_disabled = $true
    snapshots_per_lifecycle = @('baseline','post-install','post-uninstall')
  }
  lifecycles = @($currentUser,$perMachine,$wix)
  msi_product_code = $productCode
  firewall_mutation_claimed = $false
  hosts_mutation_claimed = $false
  resolver_mutation_claimed = $false
  trust_store_mutation_claimed = $false
  product_configuration_mutated = $false
  service_registration_claimed = $false
  acl_mutation_claimed = $false
  signing_claimed = $false
  updater_mutation_claimed = $false
  tracked_repository_drift_zero = $true
}
$evidence | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
