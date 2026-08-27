param(
  [Parameter(Mandatory=$true)][string]$SetupPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.10',
  [string]$StageManifest = 'target/pkg03/03.10/stage.json'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Vsn0310NativeUi {
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
$ExpectedVersion = '0.38.1'
$UserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot = Join-Path $env:ProgramFiles $ProductName
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()
$Fingerprints = [System.Collections.Generic.HashSet[string]]::new()
$TerminalRoots = [System.Collections.Generic.HashSet[string]]::new()

function Assert-Condition([bool]$Condition,[string]$Message) {
  if (-not $Condition) { throw $Message }
}

function Get-Sha256([string]$Path) {
  (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-PathSnapshot {
  [pscustomobject][ordered]@{
    process = [Environment]::GetEnvironmentVariable('Path','Process')
    user = [Environment]::GetEnvironmentVariable('Path','User')
    machine = [Environment]::GetEnvironmentVariable('Path','Machine')
  }
}

function Assert-PathSnapshot([object]$Expected,[string]$Phase) {
  $actual = Get-PathSnapshot
  Assert-Condition ($actual.process -eq $Expected.process) "$Phase changed process PATH."
  Assert-Condition ($actual.user -eq $Expected.user) "$Phase changed user PATH."
  Assert-Condition ($actual.machine -eq $Expected.machine) "$Phase changed machine PATH."
}

function Write-UiEvidence {
  New-Item -ItemType Directory -Force $EvidencePath | Out-Null
  ConvertTo-Json -InputObject @($Observations | ForEach-Object { $_ }) -Depth 12 |
    Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
  ConvertTo-Json -InputObject @($Actions | ForEach-Object { $_ }) -Depth 8 |
    Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
}

function Write-FailureEvidence([string]$Phase,[string]$Message) {
  Write-UiEvidence
  [ordered]@{
    schema_version=1; package_id='PKG-03'; task_id='03.10'; source_commit=$SourceSha
    diagnostic_only=$true; failed_phase=$Phase; message=$Message
    captured_at_utc=[DateTime]::UtcNow.ToString('o')
  } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
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
        [void]$family.Add($processId); $changed = $true
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
  $buttons = @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
  $checks = @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
  $title = Get-SafeName $Window
  $windowProcessId = 0
  try { $windowProcessId = [int]$Window.Current.ProcessId } catch {}
  $fingerprint = "${Phase}|$windowProcessId|$title|$($buttons -join '|')|$($checks -join '|')"
  if (-not $Fingerprints.Add($fingerprint)) { return }
  [void]$Observations.Add([pscustomobject][ordered]@{
    phase=$Phase; process_id=$windowProcessId; title=$title; buttons=$buttons; checkboxes=$checks
    at_utc=[DateTime]::UtcNow.ToString('o')
  })
  Write-UiEvidence
}

function Disable-Launch([System.Windows.Automation.AutomationElement]$Window,[string]$Phase) {
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
      Write-UiEvidence
    } catch {}
  }
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
  if ($buttonHandle -eq [IntPtr]::Zero -or -not [Vsn0310NativeUi]::IsWindow($buttonHandle)) { return }
  $rootHandle = [Vsn0310NativeUi]::GetAncestor($buttonHandle,[uint32]2)
  if ($rootHandle -eq [IntPtr]::Zero) {
    try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return }
  }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0310NativeUi]::IsWindow($rootHandle)) { return }
  $key = "${Phase}:$($rootHandle.ToInt64())"
  if (-not $TerminalRoots.Add($key)) { return }
  $controlId = [Vsn0310NativeUi]::GetDlgCtrlID($buttonHandle)
  if ($controlId -gt 0) {
    [void][Vsn0310NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle)
    [void]$Actions.Add([pscustomobject][ordered]@{
      phase=$Phase; action='native-wm-command-fallback'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
    })
    Start-Sleep -Milliseconds 400
  }
  if ([Vsn0310NativeUi]::IsWindow($rootHandle)) {
    [void][Vsn0310NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$Actions.Add([pscustomobject][ordered]@{
      phase=$Phase; action='native-wm-close-terminal-fallback'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
    })
  }
  Write-UiEvidence
}

function Invoke-Primary([System.Windows.Automation.AutomationElement]$Window,[string]$Phase,[bool]$CompletionReached) {
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
      Write-UiEvidence
      if ($selected.Norm -match '(?i)^(Finish|Close)$') {
        Start-Sleep -Milliseconds 350
        Invoke-TerminalFallback $Window $selected.Element $selected.Name $Phase $CompletionReached
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
  [int]$TimeoutSeconds = 210
) {
  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible = $false
  $quietCompletePolls = 0
  while ([DateTime]::UtcNow -lt $deadline) {
    $complete = [bool](& $Completion)
    $windows = @(Get-RelevantWindows $RootProcess.Id)
    if ($windows.Count -eq 0) {
      if ($complete) {
        $quietCompletePolls++
        if ($quietCompletePolls -ge 3) {
          return [pscustomobject]@{visible=$visible;terminal_closed=$true}
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
      Disable-Launch $window $Phase
      $complete = [bool](& $Completion)
      [void](Invoke-Primary $window $Phase $complete)
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
  $installer = New-Object -ComObject WindowsInstaller.Installer
  $db = $installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
  $view = $db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
  $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null) | Out-Null
  $record = $view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null)
  if ($null -eq $record) { throw "MSI property $Property missing." }
  [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}

function Get-PayloadState([string]$Root,[object]$Stage) {
  $cli = Join-Path $Root 'bin\vsn.exe'
  $agent = Join-Path $Root 'bin\vsn-agent.exe'
  [pscustomobject][ordered]@{
    root=$Root
    cli_path=$cli
    agent_path=$agent
    cli_present=(Test-Path -LiteralPath $cli -PathType Leaf)
    agent_present=(Test-Path -LiteralPath $agent -PathType Leaf)
    cli_sha256=$(if (Test-Path -LiteralPath $cli -PathType Leaf) { Get-Sha256 $cli } else { $null })
    agent_sha256=$(if (Test-Path -LiteralPath $agent -PathType Leaf) { Get-Sha256 $agent } else { $null })
    expected_cli_sha256=[string]$Stage.cli.sha256
    expected_agent_sha256=[string]$Stage.agent.sha256
  }
}

function Assert-PayloadInstalled([string]$Root,[object]$Stage,[string]$Phase) {
  $state = Get-PayloadState $Root $Stage
  Assert-Condition $state.cli_present "$Phase missing bin\vsn.exe."
  Assert-Condition $state.agent_present "$Phase missing bin\vsn-agent.exe."
  Assert-Condition ($state.cli_sha256 -eq $state.expected_cli_sha256) "$Phase vsn.exe hash mismatch."
  Assert-Condition ($state.agent_sha256 -eq $state.expected_agent_sha256) "$Phase vsn-agent.exe hash mismatch."
  $state
}

function Assert-PayloadRemoved([string]$Root,[string]$Phase) {
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $Root 'bin\vsn.exe'))) "$Phase left bin\vsn.exe after uninstall."
  Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $Root 'bin\vsn-agent.exe'))) "$Phase left bin\vsn-agent.exe after uninstall."
}

function Invoke-CapturedProbe(
  [string]$FilePath,
  [string[]]$Arguments,
  [string]$Name,
  [string]$ExpectedOutput,
  [int]$TimeoutSeconds = 40
) {
  $stdout = Join-Path $EvidencePath "$Name.stdout.txt"
  $stderr = Join-Path $EvidencePath "$Name.stderr.txt"
  Remove-Item -LiteralPath $stdout,$stderr -Force -ErrorAction SilentlyContinue
  $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
  Wait-Process -Id $process.Id -Timeout $TimeoutSeconds -ErrorAction SilentlyContinue
  try { $process.Refresh() } catch {}
  $exited = $false
  try { $exited = $process.HasExited } catch { $exited = $true }
  Assert-Condition $exited "$Name launch probe did not exit within $TimeoutSeconds seconds."
  Assert-Condition ($process.ExitCode -eq 0) "$Name launch probe exit code $($process.ExitCode)."
  $out = if (Test-Path -LiteralPath $stdout) { Get-Content -LiteralPath $stdout -Raw } else { '' }
  $err = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
  Assert-Condition ($out -match [regex]::Escape($ExpectedOutput)) "$Name launch probe did not emit expected identity text."
  $outText = if ($null -eq $out) { '' } else { ([string]$out).Trim() }
  $errText = if ($null -eq $err) { '' } else { ([string]$err).Trim() }
  [pscustomobject][ordered]@{
    executable=$FilePath
    arguments=$Arguments
    exit_code=$process.ExitCode
    expected_output=$ExpectedOutput
    stdout=$outText
    stderr=$errText
  }
}

function Invoke-PayloadProbes([string]$Root,[string]$Prefix) {
  $cli = Invoke-CapturedProbe (Join-Path $Root 'bin\vsn.exe') @('--version') "$Prefix-cli" 'vsn 0.38.1' 20
  $agent = Invoke-CapturedProbe (Join-Path $Root 'bin\vsn-agent.exe') @('--once') "$Prefix-agent" 'VSN Agent 0.38.1' 40
  [pscustomobject][ordered]@{cli=$cli;agent=$agent}
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
Write-UiEvidence
$actualHead = (git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"
$SetupPath = (Resolve-Path -LiteralPath $SetupPath).Path
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
$StageManifest = (Resolve-Path -LiteralPath $StageManifest).Path
$stageDocument = Get-Content -LiteralPath $StageManifest -Raw | ConvertFrom-Json
Assert-Condition ([string]$stageDocument.source_commit -eq $SourceSha) 'Stage manifest is not bound to exact source head.'
$stageById = @{}
foreach ($entry in @($stageDocument.staged)) { $stageById[[string]$entry.id] = $entry }
Assert-Condition ($stageById.ContainsKey('cli') -and $stageById.ContainsKey('agent')) 'Stage manifest must contain CLI and Agent.'
$stage = [pscustomobject]@{cli=$stageById.cli;agent=$stageById.agent}
Assert-Condition ((Get-Sha256 (Join-Path (Get-Location) 'target/pkg03/03.10/vsn.exe')) -eq [string]$stage.cli.sha256) 'Staged CLI hash drifted after bundle build.'
Assert-Condition ((Get-Sha256 (Join-Path (Get-Location) 'target/pkg03/03.10/vsn-agent.exe')) -eq [string]$stage.agent.sha256) 'Staged Agent hash drifted after bundle build.'

Assert-PayloadRemoved $UserRoot 'preflight current-user'
Assert-PayloadRemoved $MachineRoot 'preflight machine'
$pathBaseline = Get-PathSnapshot

# MSI/WiX per-machine placement, discovery, launch and cleanup.
# Run WiX first on the fresh runner so stock AppSearch cannot inherit a
# current-user install-directory hint from the NSIS lifecycle.
$productCode = Get-MsiProperty $MsiPath 'ProductCode'
$msiexec = Join-Path $env:SystemRoot 'System32\msiexec.exe'
$mi = Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath)) -PassThru
$msiInstall = Drive-Ui $mi 'msi-install' {
  (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and
  (Test-Path -LiteralPath (Join-Path $MachineRoot 'bin\vsn.exe')) -and
  (Test-Path -LiteralPath (Join-Path $MachineRoot 'bin\vsn-agent.exe'))
}
Wait-ForExitBounded $mi 'MSI install'
Assert-Condition ($mi.ExitCode -eq 0) "MSI install exit $($mi.ExitCode)."
Assert-Condition $msiInstall.visible 'No visible MSI install UI observed.'
$wixPayload = Assert-PayloadInstalled $MachineRoot $stage 'MSI install'
Assert-PathSnapshot $pathBaseline 'MSI install'
$wixProbes = Invoke-PayloadProbes $MachineRoot 'wix'

$mu = Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode) -PassThru
$msiUninstall = Drive-Ui $mu 'msi-uninstall' {
  -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and
  -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'bin\vsn.exe')) -and
  -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'bin\vsn-agent.exe'))
}
Wait-ForExitBounded $mu 'MSI uninstall'
Assert-Condition ($mu.ExitCode -eq 0) "MSI uninstall exit $($mu.ExitCode)."
Assert-Condition $msiUninstall.visible 'No visible MSI uninstall UI observed.'
Assert-PayloadRemoved $MachineRoot 'MSI uninstall'
Assert-PathSnapshot $pathBaseline 'MSI uninstall'

# NSIS current-user placement, discovery, launch and cleanup.
$nsis = Start-Process -FilePath $SetupPath -PassThru
$nsisInstall = Drive-Ui $nsis 'nsis-install' {
  (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and
  (Test-Path -LiteralPath (Join-Path $UserRoot 'bin\vsn.exe')) -and
  (Test-Path -LiteralPath (Join-Path $UserRoot 'bin\vsn-agent.exe'))
}
Wait-ForExitBounded $nsis 'NSIS install'
Assert-Condition ($nsis.ExitCode -eq 0) "NSIS install exit $($nsis.ExitCode)."
Assert-Condition $nsisInstall.visible 'No visible NSIS install UI observed.'
$nsisPayload = Assert-PayloadInstalled $UserRoot $stage 'NSIS install'
Assert-PathSnapshot $pathBaseline 'NSIS install'
$nsisProbes = Invoke-PayloadProbes $UserRoot 'nsis'

$uninstaller = Join-Path $UserRoot 'uninstall.exe'
Assert-Condition (Test-Path -LiteralPath $uninstaller -PathType Leaf) 'NSIS uninstaller missing.'
$nu = Start-Process -FilePath $uninstaller -PassThru
$nsisUninstall = Drive-Ui $nu 'nsis-uninstall' {
  -not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and
  -not (Test-Path -LiteralPath (Join-Path $UserRoot 'bin\vsn.exe')) -and
  -not (Test-Path -LiteralPath (Join-Path $UserRoot 'bin\vsn-agent.exe'))
}
Wait-ForExitBounded $nu 'NSIS uninstall'
Assert-Condition ($nu.ExitCode -eq 0) "NSIS uninstall exit $($nu.ExitCode)."
Assert-Condition $nsisUninstall.visible 'No visible NSIS uninstall UI observed.'
Assert-PayloadRemoved $UserRoot 'NSIS uninstall'
Assert-PathSnapshot $pathBaseline 'NSIS uninstall'

$tracked = @(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) { $tracked | Write-Host; throw 'Tracked repository drift detected during 03.10 lifecycle.' }

Write-UiEvidence
$evidence = [ordered]@{
  schema_version=1
  package_id='PKG-03'
  task_id='03.10'
  source_commit=$SourceSha
  staged=[ordered]@{
    manifest_sha256=Get-Sha256 $StageManifest
    cli_sha256=[string]$stage.cli.sha256
    agent_sha256=[string]$stage.agent.sha256
  }
  nsis=[ordered]@{
    setup_sha256=Get-Sha256 $SetupPath
    visible_install_ui_observed=[bool]$nsisInstall.visible
    install_root=$UserRoot
    payload=$nsisPayload
    launch=$nsisProbes
    visible_uninstall_ui_observed=[bool]$nsisUninstall.visible
    cli_removed=$true
    agent_removed=$true
  }
  wix=[ordered]@{
    msi_sha256=Get-Sha256 $MsiPath
    product_code=$productCode
    visible_install_ui_observed=[bool]$msiInstall.visible
    install_root=$MachineRoot
    payload=$wixPayload
    launch=$wixProbes
    visible_uninstall_ui_observed=[bool]$msiUninstall.visible
    cli_removed=$true
    agent_removed=$true
  }
  discovery=[ordered]@{
    rule='<install-root>\\bin\\<binary-name>'
    cli_relative_path='bin\\vsn.exe'
    agent_relative_path='bin\\vsn-agent.exe'
  }
  service_registration_claimed=$false
  path_environment_mutation_claimed=$false
  acl_mutation_claimed=$false
  silent_or_passive_deployment_claimed=$false
  signing_claimed=$false
  updater_mutation_claimed=$false
  tracked_repository_drift_zero=$true
}
$evidence | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
