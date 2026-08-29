param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$ExpectedHashesJson,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.14'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Vsn0314NativeUi {
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
$UiObservations = [System.Collections.Generic.List[object]]::new()
$UiActions = [System.Collections.Generic.List[object]]::new()
$IntegrityObservations = [System.Collections.Generic.List[object]]::new()
$TerminalRoots = [System.Collections.Generic.HashSet[string]]::new()
$BackupRoot = Join-Path $env:RUNNER_TEMP ("pkg03-0314-{0}" -f ([Guid]::NewGuid().ToString('N')))

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
  try { ([string]$Element.Current.Name).Trim() } catch { '' }
}

function Get-Controls([System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.ControlType]$Type) {
  $condition = [System.Windows.Automation.PropertyCondition]::new(
    [System.Windows.Automation.AutomationElement]::ControlTypeProperty,$Type
  )
  @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition))
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
        [void]$family.Add($pidNow)
        $changed = $true
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
  [void]$UiObservations.Add([pscustomobject][ordered]@{
    lifecycle=$Lifecycle
    phase=$Phase
    process_id=$(try { [int]$Window.Current.ProcessId } catch { 0 })
    title=Get-SafeName $Window
    buttons=$buttons
    checkboxes=$checks
    at_utc=[DateTime]::UtcNow.ToString('o')
  })
  Write-UiEvidence
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
      [void]$UiActions.Add([pscustomobject][ordered]@{
        lifecycle=$Lifecycle;phase=$Phase;action='ensure-safety-checkbox-off';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')
      })
      Write-UiEvidence
    } catch {}
  }
}

function Invoke-NativeTerminal(
  [string]$Lifecycle,
  [string]$Phase,
  [System.Windows.Automation.AutomationElement]$Window,
  [System.Windows.Automation.AutomationElement]$Button,
  [string]$Name
) {
  $buttonHandle = [IntPtr]::Zero
  try { $buttonHandle = [IntPtr][int]$Button.Current.NativeWindowHandle } catch {}
  if ($buttonHandle -eq [IntPtr]::Zero -or -not [Vsn0314NativeUi]::IsWindow($buttonHandle)) { return $false }
  $rootHandle = [Vsn0314NativeUi]::GetAncestor($buttonHandle,[uint32]2)
  if ($rootHandle -eq [IntPtr]::Zero) {
    try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0314NativeUi]::IsWindow($rootHandle)) { return $false }
  $key = "${Lifecycle}:${Phase}:$($rootHandle.ToInt64())"
  if (-not $TerminalRoots.Add($key)) { return $false }

  $acted = $false
  $controlId = [Vsn0314NativeUi]::GetDlgCtrlID($buttonHandle)
  if ($controlId -gt 0) {
    [void][Vsn0314NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle)
    [void]$UiActions.Add([pscustomobject][ordered]@{
      lifecycle=$Lifecycle;phase=$Phase;action='native-terminal-command';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')
    })
    Write-UiEvidence
    $acted = $true
    Start-Sleep -Milliseconds 350
  }
  if ([Vsn0314NativeUi]::IsWindow($rootHandle)) {
    [void][Vsn0314NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$UiActions.Add([pscustomobject][ordered]@{
      lifecycle=$Lifecycle;phase=$Phase;action='native-terminal-close';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')
    })
    Write-UiEvidence
    $acted = $true
  }
  $acted
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
        $candidates += [pscustomobject]@{element=$button;name=$name;norm=($name -replace '&','').Trim()}
      }
    } catch {}
  }

  foreach ($pattern in $priority) {
    $selected = $candidates | Where-Object { $_.norm -match "(?i)$pattern" } | Select-Object -First 1
    if ($null -eq $selected) { continue }

    if ($CompletionReached -and $selected.norm -match '(?i)^(Finish|OK|Close)$') {
      $nativeHandled = Invoke-NativeTerminal $Lifecycle $Phase $Window $selected.element $selected.name
      if ($nativeHandled) { return $selected.norm }
    }

    try {
      $invoke = [System.Windows.Automation.InvokePattern]$selected.element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      [void]$UiActions.Add([pscustomobject][ordered]@{
        lifecycle=$Lifecycle;phase=$Phase;action='invoke-button';control=$selected.name;at_utc=[DateTime]::UtcNow.ToString('o')
      })
      Write-UiEvidence
      return $selected.norm
    } catch {}
  }
  $null
}

function Drive-SuccessUi(
  [string]$Lifecycle,
  [string]$Phase,
  [System.Diagnostics.Process]$Process,
  [scriptblock]$Completion,
  [int]$TimeoutSeconds=210
) {
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible=$false
  $quiet=0
  while ([DateTime]::UtcNow -lt $deadline) {
    $complete=[bool](& $Completion)
    $windows=@(Get-RelevantWindows $Process.Id)
    if ($windows.Count -eq 0) {
      if ($complete) {
        $quiet++
        if ($quiet -ge 3) { return [pscustomobject]@{visible=$visible;completion=$true} }
      } else { $quiet=0 }
      Start-Sleep -Milliseconds 450
      continue
    }
    $visible=$true
    $quiet=0
    $window=$windows[0]
    try { $window.SetFocus() } catch {}
    Record-Window $Lifecycle $Phase $window
    Set-SafetyCheckboxes $Lifecycle $Phase $window
    $complete=[bool](& $Completion)
    [void](Invoke-PrimaryButton $Lifecycle $Phase $window $complete)
    Start-Sleep -Milliseconds 700
  }
  Write-UiEvidence
  throw "Timed out driving 03.14 $Lifecycle $Phase UI."
}

function Wait-ProcessExit([System.Diagnostics.Process]$Process,[string]$Label,[int]$Seconds=30) {
  $ok=$Process.WaitForExit($Seconds*1000)
  Assert-Condition $ok "$Label root process did not exit within $Seconds seconds."
  $Process.Refresh()
  [int]$Process.ExitCode
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

function Get-IntegrityObservation(
  [string]$Lifecycle,
  [string]$Stage,
  [string]$InstallRoot,
  [pscustomobject]$Expected
) {
  $relative=[string]$Expected.relative_path
  $path=Join-Path $InstallRoot ($relative -replace '/', '\')
  $exists=Test-Path -LiteralPath $path -PathType Leaf
  $observedHash=$null
  $classification='MISSING'
  if ($exists) {
    $observedHash=Get-Sha256 $path
    $classification = if ($observedHash -eq [string]$Expected.sha256) { 'MATCH' } else { 'HASH_MISMATCH' }
  }
  $record=[pscustomobject][ordered]@{
    lifecycle=$Lifecycle
    stage=$Stage
    install_root=$InstallRoot
    relative_path=$relative
    expected_sha256=[string]$Expected.sha256
    expected_size_bytes=[int64]$Expected.size_bytes
    observed_exists=[bool]$exists
    observed_sha256=$observedHash
    observed_size_bytes=$(if ($exists) { [int64](Get-Item -LiteralPath $path).Length } else { $null })
    classification=$classification
    repair_required=($classification -ne 'MATCH')
    source_commit=$SourceSha
    at_utc=[DateTime]::UtcNow.ToString('o')
  }
  [void]$IntegrityObservations.Add($record)
  Write-UiEvidence
  $record
}

function Assert-AllMatch([string]$Lifecycle,[string]$Stage,[string]$InstallRoot,[object[]]$ExpectedOwned) {
  $records=@()
  foreach ($expected in $ExpectedOwned) {
    $record=Get-IntegrityObservation $Lifecycle $Stage $InstallRoot $expected
    Assert-Condition ($record.classification -eq 'MATCH') "$Lifecycle $Stage $($expected.relative_path) is not MATCH: $($record.classification)"
    $records += $record
  }
  @($records)
}

function Invoke-IntegrityProbe(
  [string]$Lifecycle,
  [string]$InstallRoot,
  [pscustomobject]$Expected,
  [string]$Probe
) {
  $relative=[string]$Expected.relative_path
  $path=Join-Path $InstallRoot ($relative -replace '/', '\')
  Assert-Condition (Test-Path -LiteralPath $path -PathType Leaf) "$Lifecycle probe source missing: $relative"
  Assert-Condition ((Get-Sha256 $path) -eq [string]$Expected.sha256) "$Lifecycle probe source is not exact expected bytes: $relative"

  $safe=($Lifecycle + '-' + $relative -replace '[^A-Za-z0-9._-]','_')
  $backup=Join-Path $BackupRoot ($safe + '.bak')
  New-Item -ItemType Directory -Force (Split-Path -Parent $backup) | Out-Null
  Copy-Item -LiteralPath $path -Destination $backup -Force
  Assert-Condition ((Get-Sha256 $backup) -eq [string]$Expected.sha256) "$Lifecycle backup hash mismatch: $relative"

  try {
    if ($Probe -eq 'missing') {
      Remove-Item -LiteralPath $path -Force
      $observed=Get-IntegrityObservation $Lifecycle 'probe-missing' $InstallRoot $Expected
      Assert-Condition ($observed.classification -eq 'MISSING') "$Lifecycle missing probe misclassified $relative as $($observed.classification)"
    } elseif ($Probe -eq 'tamper') {
      $stream=[IO.File]::Open($path,[IO.FileMode]::Append,[IO.FileAccess]::Write,[IO.FileShare]::Read)
      try { $stream.WriteByte(0x31) } finally { $stream.Dispose() }
      $observed=Get-IntegrityObservation $Lifecycle 'probe-tamper' $InstallRoot $Expected
      Assert-Condition ($observed.classification -eq 'HASH_MISMATCH') "$Lifecycle tamper probe misclassified $relative as $($observed.classification)"
    } else {
      throw "Unknown integrity probe: $Probe"
    }
  } finally {
    Copy-Item -LiteralPath $backup -Destination $path -Force
  }

  $restored=Get-IntegrityObservation $Lifecycle ("restored-after-{0}" -f $Probe) $InstallRoot $Expected
  Assert-Condition ($restored.classification -eq 'MATCH') "$Lifecycle restoration did not return $relative to MATCH"
  [pscustomobject][ordered]@{
    relative_path=$relative
    probe=$Probe
    observed_classification=$observed.classification
    repair_required=[bool]$observed.repair_required
    restored_classification=$restored.classification
  }
}

function Invoke-ProbeMatrix(
  [string]$Lifecycle,
  [string]$InstallRoot,
  [object[]]$ExpectedOwned,
  [string[]]$ProbeRelativePaths
) {
  $results=@()
  foreach ($relative in $ProbeRelativePaths) {
    $expected=$ExpectedOwned | Where-Object { $_.relative_path -eq $relative } | Select-Object -First 1
    Assert-Condition ($null -ne $expected) "$Lifecycle expected file missing from contract: $relative"
    foreach ($probe in @('missing','tamper')) {
      $results += Invoke-IntegrityProbe $Lifecycle $InstallRoot $expected $probe
    }
  }
  @($results)
}

function Assert-OwnedAbsent([string]$Lifecycle,[string]$InstallRoot,[object[]]$ExpectedOwned) {
  foreach ($expected in $ExpectedOwned) {
    $path=Join-Path $InstallRoot (([string]$expected.relative_path) -replace '/', '\')
    Assert-Condition (-not (Test-Path -LiteralPath $path -PathType Leaf)) "$Lifecycle uninstall left owned payload: $($expected.relative_path)"
  }
}

function Invoke-IntegrityLifecycle(
  [string]$Lifecycle,
  [string]$InstallRoot,
  [object[]]$ExpectedOwned,
  [string[]]$ProbeRelativePaths,
  [scriptblock]$StartInstall,
  [scriptblock]$InstallCompletion,
  [scriptblock]$StartUninstall,
  [scriptblock]$UninstallCompletion
) {
  $installProcess=& $StartInstall
  $installUi=Drive-SuccessUi $Lifecycle 'install' $installProcess $InstallCompletion
  $installExit=Wait-ProcessExit $installProcess "$Lifecycle install"
  Assert-Condition ($installExit -eq 0) "$Lifecycle install exit code $installExit"
  Assert-Condition $installUi.visible "$Lifecycle install did not expose visible UI"

  $healthy=Assert-AllMatch $Lifecycle 'post-install-healthy' $InstallRoot $ExpectedOwned
  $probes=Invoke-ProbeMatrix $Lifecycle $InstallRoot $ExpectedOwned $ProbeRelativePaths
  [void](Assert-AllMatch $Lifecycle 'post-probe-restored' $InstallRoot $ExpectedOwned)

  $uninstallProcess=& $StartUninstall
  $uninstallUi=Drive-SuccessUi $Lifecycle 'uninstall' $uninstallProcess $UninstallCompletion
  $uninstallExit=Wait-ProcessExit $uninstallProcess "$Lifecycle uninstall"
  Assert-Condition ($uninstallExit -eq 0) "$Lifecycle uninstall exit code $uninstallExit"
  Assert-Condition $uninstallUi.visible "$Lifecycle uninstall did not expose visible UI"
  Assert-OwnedAbsent $Lifecycle $InstallRoot $ExpectedOwned

  [pscustomobject][ordered]@{
    lifecycle=$Lifecycle
    install_root=$InstallRoot
    visible_install_ui_observed=[bool]$installUi.visible
    visible_uninstall_ui_observed=[bool]$uninstallUi.visible
    install_exit_code=$installExit
    uninstall_exit_code=$uninstallExit
    healthy_post_install=@($healthy | ForEach-Object { [pscustomobject]@{relative_path=$_.relative_path;classification=$_.classification;sha256=$_.observed_sha256} })
    probes=@($probes)
    owned_payload_absent_after_uninstall=$true
  }
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
New-Item -ItemType Directory -Force $BackupRoot | Out-Null
try {
  $actualHead=(git rev-parse HEAD).Trim()
  Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"

  $expectedPath=(Resolve-Path -LiteralPath $ExpectedHashesJson).Path
  $expectedContract=Get-Content -Raw -LiteralPath $expectedPath | ConvertFrom-Json
  Assert-Condition ([string]$expectedContract.source_commit -eq $SourceSha) 'Expected-hash source commit mismatch.'
  $ExpectedOwned=@($expectedContract.owned)
  $expectedPaths=@($ExpectedOwned | ForEach-Object { [string]$_.relative_path })
  $requiredPaths=@('VSN Dev Platform.exe','bin\vsn.exe','bin\vsn-agent.exe')
  Assert-Condition ($expectedPaths.Count -eq 3) "Expected exactly 3 owned executable hashes; got $($expectedPaths.Count)."
  foreach ($required in $requiredPaths) {
    Assert-Condition ($required -in $expectedPaths) "Expected-hash contract missing $required"
  }
  foreach ($expected in $ExpectedOwned) {
    Assert-Condition ([string]$expected.sha256 -match '^[0-9a-f]{64}$') "Invalid expected SHA-256: $($expected.relative_path)"
    Assert-Condition ([int64]$expected.size_bytes -gt 0) "Invalid expected size: $($expected.relative_path)"
  }

  $CurrentUserNsisPath=(Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
  $PerMachineNsisPath=(Resolve-Path -LiteralPath $PerMachineNsisPath).Path
  $MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
  foreach ($package in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)) {
    Assert-Condition ((Get-Item -LiteralPath $package).Length -gt 0) "Installer package empty: $package"
  }

  Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) 'HKCU installer registration already exists before 03.14.'
  Assert-Condition (-not (Test-Path -LiteralPath $HklmNsisKey)) 'HKLM NSIS installer registration already exists before 03.14.'

  $productCode=Get-MsiProperty $MsiPath 'ProductCode'
  $msiArpKey="HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productCode"
  Assert-Condition (-not (Test-Path -LiteralPath $msiArpKey)) 'MSI registration already exists before 03.14.'
  $msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'

  # Run WiX first so AppSearch cannot inherit a current-user installation hint.
  $wix=Invoke-IntegrityLifecycle 'wix-per-machine' $MachineRoot $ExpectedOwned @('VSN Dev Platform.exe','bin\vsn.exe') `
    { Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath)) -PassThru } `
    { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $msiArpKey) } `
    { Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode) -PassThru } `
    { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $msiArpKey) }
  Assert-Condition (-not (Test-Path -LiteralPath $msiArpKey)) 'MSI registration remains after 03.14 WiX lifecycle.'

  $currentUser=Invoke-IntegrityLifecycle 'nsis-current-user' $UserRoot $ExpectedOwned @('VSN Dev Platform.exe','bin\vsn.exe','bin\vsn-agent.exe') `
    { Start-Process -FilePath $CurrentUserNsisPath -PassThru } `
    { (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HkcuKey) } `
    { Start-Process -FilePath (Join-Path $UserRoot 'uninstall.exe') -PassThru } `
    { -not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HkcuKey) }
  Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) 'HKCU installer registration remains after 03.14 current-user lifecycle.'

  $perMachine=Invoke-IntegrityLifecycle 'nsis-per-machine' $MachineRoot $ExpectedOwned @('VSN Dev Platform.exe','bin\vsn.exe') `
    { Start-Process -FilePath $PerMachineNsisPath -PassThru } `
    { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HklmNsisKey) } `
    { Start-Process -FilePath (Join-Path $MachineRoot 'uninstall.exe') -PassThru } `
    { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HklmNsisKey) }
  Assert-Condition (-not (Test-Path -LiteralPath $HklmNsisKey)) 'HKLM NSIS registration remains after 03.14 per-machine lifecycle.'

  $tracked=@(git status --porcelain=v1 --untracked-files=no)
  if ($tracked.Count -ne 0) { $tracked | Write-Host; throw 'Tracked repository drift detected during 03.14 lifecycle.' }

  Write-UiEvidence
  $evidence=[ordered]@{
    schema_version=1
    package_id='PKG-03'
    task_id='03.14'
    source_commit=$SourceSha
    owned_relative_paths=$requiredPaths
    classification_contract=@('MATCH','MISSING','HASH_MISMATCH')
    expected=@($ExpectedOwned)
    lifecycles=@($currentUser,$perMachine,$wix)
    integrity_observation_count=$IntegrityObservations.Count
    current_user_agent_destructive_probe=$true
    machine_agent_destructive_probe=$false
    repair_execution_claimed=$false
    reinstall_execution_claimed=$false
    self_healing_claimed=$false
    service_coordination_claimed=$false
    product_configuration_mutated=$false
    acl_mutation_claimed=$false
    firewall_hosts_dns_trust_mutation_claimed=$false
    path_environment_mutation_claimed=$false
    signing_claimed=$false
    updater_mutation_claimed=$false
    tracked_repository_drift_zero=$true
    msi_product_code=$productCode
  }
  $evidence | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
  Write-UiEvidence
} finally {
  Remove-Item -LiteralPath $BackupRoot -Recurse -Force -ErrorAction SilentlyContinue
}
