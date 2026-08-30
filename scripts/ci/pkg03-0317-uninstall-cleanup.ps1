param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.17'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'pkg03-0313-snapshot.ps1')

# Reuse the accepted 03.13 UI helpers without executing its lifecycle.
$helperSource = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'pkg03-0313-installer-nonmutation.ps1') -Raw
$helperStart = $helperSource.IndexOf('Add-Type -AssemblyName UIAutomationClient')
$helperEnd = $helperSource.LastIndexOf('New-Item -ItemType Directory -Force $EvidencePath | Out-Null')
if ($helperStart -lt 0 -or $helperEnd -le $helperStart) { throw 'Unable to locate accepted 03.13 helper boundary.' }
Invoke-Expression $helperSource.Substring($helperStart,$helperEnd-$helperStart)

$ProductName = 'VSN Dev Platform'
$ServiceName = 'VSN-Agent'
$UserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot = Join-Path $env:ProgramFiles $ProductName
$SecurityRoot = Join-Path $env:ProgramData 'VSN\security'
$IpcKey = Join-Path $SecurityRoot 'ipc.key'
$HkcuKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$HklmNsisKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$SnapshotsPath = Join-Path $EvidencePath 'snapshots'
$PreservationObservations = [System.Collections.Generic.List[object]]::new()
$HarnessCreated = [System.Collections.Generic.List[string]]::new()

function Get-Sha256([string]$Path) {
  (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
}

function Test-PathUnder([string]$Child,[string]$Root) {
  $childFull = [IO.Path]::GetFullPath($Child).TrimEnd('\') + '\'
  $rootFull = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
  return $childFull.StartsWith($rootFull,[StringComparison]::OrdinalIgnoreCase)
}

function Get-PathRecord([string]$Path,[string]$Class) {
  $exists = Test-Path -LiteralPath $Path
  $item = if ($exists) { Get-Item -LiteralPath $Path -Force } else { $null }
  $isFile = $exists -and -not $item.PSIsContainer
  $sddl = $null
  if ($exists) { try { $sddl = [string](Get-Acl -LiteralPath $Path).Sddl } catch {} }
  [pscustomobject][ordered]@{
    class=$Class; path=$Path; exists=[bool]$exists; is_file=[bool]$isFile
    size_bytes=$(if ($isFile) { [int64]$item.Length } else { $null })
    sha256=$(if ($isFile) { Get-Sha256 $Path } else { $null })
    sddl=$sddl
  }
}

function Assert-RecordPreserved([pscustomobject]$Before,[string]$Label) {
  Assert-Condition (Test-Path -LiteralPath $Before.path -PathType Leaf) "$Label preserved marker missing: $($Before.path)"
  $after = Get-PathRecord $Before.path $Before.class
  Assert-Condition ($after.size_bytes -eq $Before.size_bytes) "$Label marker size changed: $($Before.path)"
  Assert-Condition ($after.sha256 -eq $Before.sha256) "$Label marker bytes changed: $($Before.path)"
  Assert-Condition ($after.sddl -eq $Before.sddl) "$Label marker ACL changed: $($Before.path)"
  [void]$PreservationObservations.Add([pscustomobject][ordered]@{lifecycle=$Label;before=$Before;after=$after;preserved=$true})
  return $after
}

function New-Marker([string]$Path,[string]$Class,[string]$Lifecycle) {
  $parent = Split-Path -Parent $Path
  New-Item -ItemType Directory -Force $parent | Out-Null
  $content = "PKG-03|03.17|$Lifecycle|$Class|preserve-v1`n"
  [IO.File]::WriteAllText($Path,$content,[Text.UTF8Encoding]::new($false))
  [void]$HarnessCreated.Add($Path)
  return Get-PathRecord $Path $Class
}

function Get-ShortcutPaths {
  $roots = @(
    [Environment]::GetFolderPath('Desktop'),
    [Environment]::GetFolderPath('CommonDesktopDirectory'),
    [Environment]::GetFolderPath('StartMenu'),
    [Environment]::GetFolderPath('CommonStartMenu')
  ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }
  $paths = @()
  foreach ($root in $roots) {
    $paths += @(Get-ChildItem -LiteralPath $root -Filter 'VSN Dev Platform*.lnk' -File -Recurse -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
  }
  return @($paths | Sort-Object -Unique)
}

function Get-ServiceSnapshot {
  $svc = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue
  if ($null -eq $svc) { return $null }
  [pscustomobject][ordered]@{name=[string]$svc.Name;display_name=[string]$svc.DisplayName;start_name=[string]$svc.StartName;start_mode=[string]$svc.StartMode;path_name=[string]$svc.PathName;state=[string]$svc.State}
}

function Stop-AgentIfPresent([string]$Lifecycle) {
  $service = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
  if ($null -eq $service) { return $false }
  if ($service.Status -ne 'Stopped') {
    Stop-Service -Name $ServiceName -Force -ErrorAction Stop
    $service.WaitForStatus('Stopped',[TimeSpan]::FromSeconds(30))
  }
  $service.Refresh()
  Assert-Condition ($service.Status -eq 'Stopped') "$Lifecycle Agent service did not become quiescent."
  return $true
}

function Test-Pkg0317UninstallTerminalPage([System.Windows.Automation.AutomationElement]$Window) {
  $names = @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { (Get-SafeName $_) -replace '&','' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
  $hasClose = @($names | Where-Object { $_ -match '(?i)^Close$' }).Count -gt 0
  $hasDetails = @($names | Where-Object { $_ -match '(?i)^Show details$' }).Count -gt 0
  $hasDestructive = @($names | Where-Object { $_ -match '(?i)^(Remove|Uninstall)$' }).Count -gt 0
  return $hasClose -and -not $hasDestructive -and ($hasDetails -or $names.Count -le 4)
}

function Close-Pkg0317TerminalWindow([string]$Lifecycle,[System.Windows.Automation.AutomationElement]$Window) {
  $handle = [IntPtr]::Zero
  try { $handle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($handle -eq [IntPtr]::Zero -or -not [Vsn0313NativeUi]::IsWindow($handle)) { return $false }
  [void][Vsn0313NativeUi]::PostMessage($handle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
  [void]$Actions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase='uninstall';action='native-terminal-window-close';control='proven-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiArtifacts
  return $true
}

function Drive-Pkg0317Ui([string]$Lifecycle,[string]$Phase,[System.Diagnostics.Process]$Process,[scriptblock]$Completion,[int]$TimeoutSeconds=300) {
  $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible = $false
  $quiet = 0
  while ([DateTime]::UtcNow -lt $deadline) {
    $complete = [bool](& $Completion)
    $windows = @(Get-RelevantWindows $Process.Id)
    if ($windows.Count -eq 0) {
      $exited = $false
      try { $Process.Refresh(); $exited = $Process.HasExited } catch { $exited = $true }
      if ($complete -and $exited) { $quiet++; if ($quiet -ge 3) { break } } else { $quiet = 0 }
      Start-Sleep -Milliseconds 450
      continue
    }
    $visible = $true; $quiet = 0
    $window = $windows[0]
    try { $window.SetFocus() } catch {}
    Record-Window $Lifecycle $Phase $window
    Set-SafetyCheckboxes $Lifecycle $Phase $window
    if ($Phase -eq 'uninstall' -and $complete -and (Test-Pkg0317UninstallTerminalPage $window)) {
      [void](Close-Pkg0317TerminalWindow $Lifecycle $window)
    } elseif ($Phase -eq 'uninstall' -and $complete) {
      # The required uninstall state is already reached; close only the root
      # installer window, never click another destructive action.
      [void](Close-Pkg0317TerminalWindow $Lifecycle $window)
    } else {
      [void](Invoke-PrimaryButton $Lifecycle $Phase $window $complete)
    }
    Start-Sleep -Milliseconds 700
  }
  Assert-Condition ([bool](& $Completion)) "$Lifecycle $Phase did not reach required state."
  Wait-ForExitBounded $Process "$Lifecycle $Phase" 30
  Assert-Condition ($Process.ExitCode -eq 0) "$Lifecycle $Phase exit code was $($Process.ExitCode), expected 0."
  return [pscustomobject][ordered]@{phase=$Phase;visible_ui_observed=$visible;exit_code=[int]$Process.ExitCode}
}

function Build-ContextProbe {
  $probeDir = Join-Path $EvidencePath 'context-probe-src'
  $src = Join-Path $probeDir 'src'
  New-Item -ItemType Directory -Force $src | Out-Null
  $repo = (Get-Location).Path.Replace('\','/')
  @"
[package]
name = "vsn0317-context-probe"
version = "0.1.0"
edition = "2021"
[dependencies]
vsn-security = { path = "$repo/crates/vsn-security" }
vsn-config = { path = "$repo/crates/vsn-config" }
[workspace]
"@ | Set-Content -LiteralPath (Join-Path $probeDir 'Cargo.toml') -Encoding utf8NoBOM
  @'
use std::{env,fs};
fn main(){
 let out=env::args().nth(1).expect("output");
 let data=vsn_security::data_dir().expect("data");
 let config=vsn_config::default_path().expect("config");
 fs::write(out,format!("data_local={}\nconfig_file={}\nlocal_app_data={}\napp_data={}\nprogram_data={}\n",data.display(),config.display(),env::var("LOCALAPPDATA").unwrap_or_default(),env::var("APPDATA").unwrap_or_default(),env::var("PROGRAMDATA").unwrap_or_default())).expect("write");
}
'@ | Set-Content -LiteralPath (Join-Path $src 'main.rs') -Encoding utf8NoBOM
  $targetRoot = (Resolve-Path 'target').Path
  $oldTarget = $env:CARGO_TARGET_DIR
  try {
    $env:CARGO_TARGET_DIR = $targetRoot
    & cargo build --release --offline --manifest-path (Join-Path $probeDir 'Cargo.toml')
    Assert-Condition ($LASTEXITCODE -eq 0) '03.17 context probe build failed.'
  } finally { $env:CARGO_TARGET_DIR = $oldTarget }
  $exe = Join-Path $targetRoot 'release\vsn0317-context-probe.exe'
  Assert-Condition (Test-Path -LiteralPath $exe -PathType Leaf) '03.17 context probe executable missing.'
  return $exe
}

function Read-ContextProbeOutput([string]$Path,[string]$Context) {
  $map = @{}
  foreach ($line in Get-Content -LiteralPath $Path) {
    $parts = $line -split '=',2
    if ($parts.Count -eq 2) { $map[$parts[0]] = $parts[1] }
  }
  foreach ($key in @('data_local','config_file','local_app_data','app_data','program_data')) {
    Assert-Condition (-not [string]::IsNullOrWhiteSpace([string]$map[$key])) "$Context context probe missing $key."
  }
  [pscustomobject][ordered]@{context=$Context;data_local=[string]$map.data_local;config_file=[string]$map.config_file;local_app_data=[string]$map.local_app_data;app_data=[string]$map.app_data;program_data=[string]$map.program_data}
}

function Invoke-CurrentUserContextProbe([string]$ProbeExe) {
  $out = Join-Path $EvidencePath 'context-current-user.txt'
  & $ProbeExe $out
  Assert-Condition ($LASTEXITCODE -eq 0) 'Current-user context probe failed.'
  return Read-ContextProbeOutput $out 'current-user'
}

function Invoke-LocalServiceContextProbe([string]$ProbeExe,[string]$Lifecycle) {
  $probeName = 'VSN-0317-ContextProbe'
  $out = Join-Path $env:WINDIR ("Temp\vsn-0317-context-" + [guid]::NewGuid().ToString('N') + '.txt')
  $sc = Join-Path $env:SystemRoot 'System32\sc.exe'
  if (Get-CimInstance Win32_Service -Filter "Name='$probeName'" -ErrorAction SilentlyContinue) { & $sc delete $probeName | Out-Null; Start-Sleep -Seconds 1 }
  $binPath = "`"$ProbeExe`" `"$out`""
  $created = (& $sc create $probeName 'binPath=' $binPath 'start=' 'demand' 'obj=' 'NT AUTHORITY\LocalService' 2>&1 | Out-String).Trim()
  Assert-Condition ($LASTEXITCODE -eq 0) "$Lifecycle LocalService probe service create failed: $created"
  try {
    & $sc start $probeName 2>&1 | Out-Null
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    while ([DateTime]::UtcNow -lt $deadline -and -not (Test-Path -LiteralPath $out -PathType Leaf)) { Start-Sleep -Milliseconds 250 }
    Assert-Condition (Test-Path -LiteralPath $out -PathType Leaf) "$Lifecycle LocalService context probe produced no output."
    return Read-ContextProbeOutput $out 'local-service'
  } finally {
    & $sc delete $probeName 2>&1 | Out-Null
    Remove-Item -LiteralPath $out -Force -ErrorAction SilentlyContinue
  }
}

function Get-OwnedRelativePaths {
  $manifest = Get-Content -LiteralPath 'installer/windows/owned-payload.v1.json' -Raw | ConvertFrom-Json
  $paths = @($manifest.owned_files | ForEach-Object { ([string]$_.relative_path).Replace('/','\') })
  Assert-Condition (($paths -join '|') -eq 'VSN Dev Platform.exe|bin\vsn.exe|bin\vsn-agent.exe') 'Owned payload manifest drifted from accepted 03.05/03.10 set.'
  return $paths
}

function Assert-OwnedPresent([string]$Lifecycle,[string]$InstallRoot,[string[]]$Owned) {
  $rows = @()
  foreach ($relative in $Owned) {
    $path = Join-Path $InstallRoot $relative
    Assert-Condition (Test-Path -LiteralPath $path -PathType Leaf) "$Lifecycle expected owned file missing after install: $relative"
    $rows += [pscustomobject][ordered]@{relative_path=$relative;size_bytes=[int64](Get-Item -LiteralPath $path).Length;sha256=Get-Sha256 $path}
  }
  return @($rows)
}

function Assert-OwnedAbsent([string]$Lifecycle,[string]$InstallRoot,[string[]]$Owned,[string]$HarnessJunction) {
  foreach ($relative in $Owned) {
    Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $InstallRoot $relative) -PathType Leaf)) "$Lifecycle uninstall left owned file: $relative"
  }
  $remaining = @()
  if (Test-Path -LiteralPath $InstallRoot -PathType Container) {
    $remaining = @(Get-ChildItem -LiteralPath $InstallRoot -Force -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName })
    $unexpected = @($remaining | Where-Object { $_ -ne $HarnessJunction })
    Assert-Condition ($unexpected.Count -eq 0) "$Lifecycle uninstall left unexpected install-root artifacts: $($unexpected -join ', ')"
  }
  return @($remaining)
}

function Get-SecurityClassification([string]$Lifecycle,[string]$Stage) {
  $dirExists = Test-Path -LiteralPath $SecurityRoot -PathType Container
  $keyExists = Test-Path -LiteralPath $IpcKey -PathType Leaf
  $entries = @()
  if ($dirExists) { $entries = @(Get-ChildItem -LiteralPath $SecurityRoot -Force -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName }) }
  [pscustomobject][ordered]@{
    lifecycle=$Lifecycle;stage=$Stage;directory=$SecurityRoot;directory_exists=[bool]$dirExists;key_exists=[bool]$keyExists
    key_sha256=$(if ($keyExists) { Get-Sha256 $IpcKey } else { $null });entries=$entries
    classification='runtime-security-state';installer_owned_claimed=$false;heuristic_delete_allowed=$false
  }
}

function Reset-RunCreatedSecurity([string]$Lifecycle,[pscustomobject]$BeforeInstall,[pscustomobject]$AfterUninstall) {
  if ($BeforeInstall.directory_exists) { return [pscustomobject]@{reset=$false;reason='preexisting-not-owned-by-harness'} }
  if (-not $AfterUninstall.directory_exists) { return [pscustomobject]@{reset=$false;reason='already-absent'} }
  Assert-Condition ($null -eq (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) "$Lifecycle cannot reset run-created security while service exists."
  # Certification-environment cleanup after evidence capture; not an installer claim.
  Remove-Item -LiteralPath $SecurityRoot -Recurse -Force
  Assert-Condition (-not (Test-Path -LiteralPath $SecurityRoot)) "$Lifecycle failed to reset run-created security fixture."
  return [pscustomobject]@{reset=$true;reason='run-created-fixture-after-evidence'}
}

function New-PreservationFixture([string]$Lifecycle,[string]$InstallRoot,[pscustomobject]$Context) {
  Assert-Condition (-not (Test-PathUnder $Context.data_local $InstallRoot)) "$Lifecycle mutable data root is inside install root."
  Assert-Condition (-not (Test-PathUnder $Context.config_file $InstallRoot)) "$Lifecycle mutable config path is inside install root."
  Assert-Condition (-not (Test-PathUnder $Context.data_local $SecurityRoot)) "$Lifecycle mutable data overlaps machine IPC security."
  $dataMarker = New-Marker (Join-Path $Context.data_local 'pkg03-0317-preserve-data.txt') 'mutable-data' $Lifecycle
  $configDir = Split-Path -Parent $Context.config_file
  $configMarker = New-Marker (Join-Path $configDir 'pkg03-0317-preserve-config.txt') 'mutable-config' $Lifecycle
  $workspaceRoot = Join-Path ([Environment]::GetFolderPath('MyDocuments')) ("VSN-0317-$Lifecycle-workspace")
  $workspaceMarker = New-Marker (Join-Path $workspaceRoot 'project-preserve.txt') 'workspace-project' $Lifecycle
  $outsideRoot = Join-Path (Split-Path -Parent $InstallRoot) ("VSN-0317-$Lifecycle-outside")
  $outsideMarker = New-Marker (Join-Path $outsideRoot 'outside-preserve.txt') 'unrelated-outside-boundary' $Lifecycle
  $junction = Join-Path $InstallRoot 'pkg03-0317-preserve-junction'
  if (Test-Path -LiteralPath $junction) { throw "$Lifecycle junction path unexpectedly exists." }
  New-Item -ItemType Junction -Path $junction -Target $outsideRoot | Out-Null
  [void]$HarnessCreated.Add($junction)
  $junctionItem = Get-Item -LiteralPath $junction -Force
  Assert-Condition (($junctionItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) "$Lifecycle preservation junction is not a reparse point."
  $natural = @()
  if ($Lifecycle -ne 'nsis-current-user') {
    $device = Join-Path $Context.data_local 'security\device.json'
    Assert-Condition (Test-Path -LiteralPath $device -PathType Leaf) "$Lifecycle expected persistent Agent device identity before uninstall."
    $natural += Get-PathRecord $device 'natural-runtime-device-identity'
  }
  [pscustomobject][ordered]@{
    context=$Context;markers=@($dataMarker,$configMarker,$workspaceMarker,$outsideMarker);natural_records=@($natural)
    junction_path=$junction;junction_target=$outsideRoot
  }
}

function Remove-HarnessFixture([pscustomobject]$Fixture,[string]$InstallRoot) {
  foreach ($marker in @($Fixture.markers)) { Remove-Item -LiteralPath $marker.path -Force -ErrorAction SilentlyContinue }
  foreach ($dir in @((Split-Path -Parent $Fixture.markers[2].path),(Split-Path -Parent $Fixture.markers[3].path))) {
    if (Test-Path -LiteralPath $dir -PathType Container) { Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue }
  }
  if (Test-Path -LiteralPath $Fixture.junction_path) { cmd /c rmdir "$($Fixture.junction_path)" 2>$null | Out-Null }
  if (Test-Path -LiteralPath $InstallRoot -PathType Container) {
    $remaining = @(Get-ChildItem -LiteralPath $InstallRoot -Force -ErrorAction SilentlyContinue)
    if ($remaining.Count -eq 0) { Remove-Item -LiteralPath $InstallRoot -Force -ErrorAction SilentlyContinue }
  }
}

function Invoke-PreservationLifecycle(
  [string]$Lifecycle,[string]$InstallRoot,[bool]$Machine,[bool]$Msi,
  [scriptblock]$StartInstall,[scriptblock]$InstallCompletion,
  [scriptblock]$StartUninstall,[scriptblock]$UninstallCompletion,
  [string]$ProbeExe,[string]$MsiLogPath=''
) {
  $baselineSnapshotPath = Join-Path $SnapshotsPath "$Lifecycle-baseline.json"
  $preUninstallSnapshotPath = Join-Path $SnapshotsPath "$Lifecycle-pre-uninstall.json"
  $postUninstallSnapshotPath = Join-Path $SnapshotsPath "$Lifecycle-post-uninstall.json"
  $securityBefore = Get-SecurityClassification $Lifecycle 'before-install'
  $shortcutBefore = @(Get-ShortcutPaths)
  $baseline = Write-Pkg0313Snapshot -Path $baselineSnapshotPath

  $installProcess = & $StartInstall
  $install = Drive-Pkg0317Ui $Lifecycle 'install' $installProcess $InstallCompletion 300
  $owned = @(Get-OwnedRelativePaths)
  $ownedInstalled = @(Assert-OwnedPresent $Lifecycle $InstallRoot $owned)
  $shortcutAfterInstall = @(Get-ShortcutPaths)
  $addedShortcuts = @($shortcutAfterInstall | Where-Object { $_ -notin $shortcutBefore })
  Assert-Condition ($addedShortcuts.Count -gt 0) "$Lifecycle did not create an observable owned shortcut."

  if ($Machine) {
    Assert-Condition ($null -ne (Get-ServiceSnapshot)) "$Lifecycle expected VSN-Agent after install."
    [void](Stop-AgentIfPresent $Lifecycle)
    $context = Invoke-LocalServiceContextProbe $ProbeExe $Lifecycle
  } else {
    Assert-Condition ($null -eq (Get-ServiceSnapshot)) "$Lifecycle current-user install created VSN-Agent."
    Assert-Condition (-not (Test-Path -LiteralPath $SecurityRoot)) "$Lifecycle current-user install created machine security state."
    $context = Invoke-CurrentUserContextProbe $ProbeExe
  }

  $fixture = New-PreservationFixture $Lifecycle $InstallRoot $context
  $preUninstall = Write-Pkg0313Snapshot -Path $preUninstallSnapshotPath
  Assert-Pkg0313SnapshotEqual -BaselinePath $baselineSnapshotPath -CandidatePath $preUninstallSnapshotPath -Label "$Lifecycle install regression"
  $securityPreUninstall = Get-SecurityClassification $Lifecycle 'pre-uninstall'

  $uninstallProcess = & $StartUninstall
  $uninstall = Drive-Pkg0317Ui $Lifecycle 'uninstall' $uninstallProcess $UninstallCompletion 300
  $remaining = @(Assert-OwnedAbsent $Lifecycle $InstallRoot $owned $fixture.junction_path)
  foreach ($shortcut in $addedShortcuts) { Assert-Condition (-not (Test-Path -LiteralPath $shortcut)) "$Lifecycle uninstall left owned shortcut: $shortcut" }
  if ($Machine) { Assert-Condition ($null -eq (Get-ServiceSnapshot)) "$Lifecycle uninstall left VSN-Agent service." }
  else { Assert-Condition ($null -eq (Get-ServiceSnapshot)) "$Lifecycle current-user uninstall created VSN-Agent service." }

  $preserved = @()
  foreach ($marker in @($fixture.markers)) { $preserved += Assert-RecordPreserved $marker $Lifecycle }
  foreach ($natural in @($fixture.natural_records)) { $preserved += Assert-RecordPreserved $natural $Lifecycle }
  $postUninstall = Write-Pkg0313Snapshot -Path $postUninstallSnapshotPath
  Assert-Pkg0313SnapshotEqual -BaselinePath $preUninstallSnapshotPath -CandidatePath $postUninstallSnapshotPath -Label "$Lifecycle uninstall"
  $securityAfter = Get-SecurityClassification $Lifecycle 'post-uninstall'
  if (-not $Machine) {
    Assert-Condition (-not $securityAfter.directory_exists) "$Lifecycle current-user uninstall mutated machine security state."
  }

  $reset = Reset-RunCreatedSecurity $Lifecycle $securityBefore $securityAfter
  $msiLog = $null
  if ($Msi) {
    Assert-Condition (Test-Path -LiteralPath $MsiLogPath -PathType Leaf) "$Lifecycle MSI uninstall log missing."
    Assert-Condition ((Get-Item -LiteralPath $MsiLogPath).Length -gt 0) "$Lifecycle MSI uninstall log is empty."
    $msiLog = [pscustomobject][ordered]@{path=$MsiLogPath;size_bytes=[int64](Get-Item $MsiLogPath).Length;sha256=Get-Sha256 $MsiLogPath}
  }

  Remove-HarnessFixture $fixture $InstallRoot
  return [pscustomobject][ordered]@{
    lifecycle=$Lifecycle;install_root=$InstallRoot;install=$install;uninstall=$uninstall
    owned_installed=$ownedInstalled;owned_payload_absent_after_uninstall=$true
    shortcuts_added=$addedShortcuts;owned_shortcuts_absent_after_uninstall=$true
    service_absent_after_uninstall=$true;preservation_fixture=$fixture;preserved_after_uninstall=$preserved
    install_root_remaining_entries_after_uninstall=$remaining
    security_before_install=$securityBefore;security_pre_uninstall=$securityPreUninstall;security_after_uninstall=$securityAfter;security_harness_reset=$reset
    protected_baseline_sha256=$baseline.sha256;protected_pre_uninstall_sha256=$preUninstall.sha256;protected_post_uninstall_sha256=$postUninstall.sha256
    protected_state_equal_during_install=$true;protected_state_equal_after_uninstall=$true
    msi_uninstall_log=$msiLog
  }
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
New-Item -ItemType Directory -Force $SnapshotsPath | Out-Null
$actualHead = (git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"
$CurrentUserNsisPath = (Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
$PerMachineNsisPath = (Resolve-Path -LiteralPath $PerMachineNsisPath).Path
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
foreach ($package in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)) { Assert-Condition ((Get-Item -LiteralPath $package).Length -gt 0) "Package is empty: $package" }

Assert-Condition (-not (Test-Path -LiteralPath $UserRoot)) 'Current-user install root exists at preflight.'
Assert-Condition (-not (Test-Path -LiteralPath $MachineRoot)) 'Machine install root exists at preflight.'
Assert-Condition (-not (Test-Path -LiteralPath $SecurityRoot)) 'Machine security state exists at preflight.'
Assert-Condition ($null -eq (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)) 'VSN-Agent exists at preflight.'
$probeExe = Build-ContextProbe
$productCode = Get-MsiProperty $MsiPath 'ProductCode'
$msiArpKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productCode"
$msiexec = Join-Path $env:SystemRoot 'System32\msiexec.exe'

# WiX first so native AppSearch cannot inherit current-user installer metadata.
$msiLogPath = Join-Path $EvidencePath 'wix-per-machine-uninstall.log'
$wix = Invoke-PreservationLifecycle 'wix-per-machine' $MachineRoot $true $true `
  { Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $MsiPath)) -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $msiArpKey) } `
  { Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode,'/L*V',('"{0}"' -f $msiLogPath)) -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $msiArpKey) } `
  $probeExe $msiLogPath
Assert-Condition (-not (Test-Path -LiteralPath $msiArpKey)) 'MSI ProductCode registration remains after uninstall.'

$currentUser = Invoke-PreservationLifecycle 'nsis-current-user' $UserRoot $false $false `
  { Start-Process -FilePath $CurrentUserNsisPath -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HkcuKey) } `
  { Start-Process -FilePath (Join-Path $UserRoot 'uninstall.exe') -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HkcuKey) } `
  $probeExe
Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) 'Current-user ARP registration remains after uninstall.'

$perMachine = Invoke-PreservationLifecycle 'nsis-per-machine' $MachineRoot $true $false `
  { Start-Process -FilePath $PerMachineNsisPath -PassThru } `
  { (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $HklmNsisKey) } `
  { Start-Process -FilePath (Join-Path $MachineRoot 'uninstall.exe') -PassThru } `
  { -not (Test-Path -LiteralPath (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $HklmNsisKey) } `
  $probeExe
Assert-Condition (-not (Test-Path -LiteralPath $HklmNsisKey)) 'Per-machine NSIS ARP registration remains after uninstall.'

$tracked = @(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) { $tracked | Write-Host; throw 'Tracked repository drift detected during 03.17 lifecycle.' }

Write-UiArtifacts
@($PreservationObservations) | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath (Join-Path $EvidencePath 'preservation-observations.json') -Encoding utf8NoBOM
$evidence = [ordered]@{
  schema_version=1;package_id='PKG-03';task_id='03.17';source_commit=$SourceSha
  packages=[ordered]@{
    nsis_current_user=[ordered]@{path=$CurrentUserNsisPath;size_bytes=[int64](Get-Item $CurrentUserNsisPath).Length;sha256=Get-Sha256 $CurrentUserNsisPath}
    nsis_per_machine=[ordered]@{path=$PerMachineNsisPath;size_bytes=[int64](Get-Item $PerMachineNsisPath).Length;sha256=Get-Sha256 $PerMachineNsisPath}
    msi=[ordered]@{path=$MsiPath;size_bytes=[int64](Get-Item $MsiPath).Length;sha256=Get-Sha256 $MsiPath;product_code=$productCode}
  }
  lifecycles=@($currentUser,$perMachine,$wix)
  owned_payload_manifest='installer/windows/owned-payload.v1.json'
  owned_payload_cleanup_required=$true;dirty_user_data_preservation_required=$true;workspace_project_preservation_required=$true
  outside_boundary_preservation_required=$true;reparse_escape_deletion_forbidden=$true;network_trust_nonmutation_required=$true
  machine_security_state_classification='runtime-security-state-not-installer-owned-by-03.17'
  machine_security_heuristic_delete_allowed=$false
  rollback_or_recovery_claimed=$false;running_process_coordination_claimed=$false;reboot_semantics_claimed=$false
  silent_or_passive_deployment_claimed=$false;signing_claimed=$false;updater_mutation_claimed=$false
  product_configuration_mutated=$false;installer_template_or_hook_mutated=$false;acl_policy_mutated=$false
  tracked_repository_drift_zero=$true
}
$evidence | ConvertTo-Json -Depth 22 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
$digest = Get-Sha256 (Join-Path $EvidencePath 'evidence.json')
"$digest  evidence.json" | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM
