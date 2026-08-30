param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.19'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Reuse accepted 03.15 UI/exit primitives only. 03.19 overrides terminal
# dismissal for the current Windows runner and owns all running-resource logic.
$helperSource=(& git show "main:scripts/ci/pkg03-0315-installer-diagnostics.ps1"|Out-String).Replace("`r`n","`n")
$helperStart=$helperSource.IndexOf('Set-StrictMode -Version Latest')
$helperEnd=$helperSource.IndexOf('New-Item -ItemType Directory -Force $EvidencePath | Out-Null',$helperStart)
if($helperStart -lt 0 -or $helperEnd -le $helperStart){throw 'Unable to locate accepted 03.15 helper boundary.'}
Invoke-Expression $helperSource.Substring($helperStart,$helperEnd-$helperStart)
. (Join-Path $PSScriptRoot 'pkg03-0313-snapshot.ps1')

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Vsn0319Process {
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
  public struct STARTUPINFO { public Int32 cb; public string lpReserved; public string lpDesktop; public string lpTitle; public Int32 dwX; public Int32 dwY; public Int32 dwXSize; public Int32 dwYSize; public Int32 dwXCountChars; public Int32 dwYCountChars; public Int32 dwFillAttribute; public Int32 dwFlags; public Int16 wShowWindow; public Int16 cbReserved2; public IntPtr lpReserved2; public IntPtr hStdInput; public IntPtr hStdOutput; public IntPtr hStdError; }
  [StructLayout(LayoutKind.Sequential)]
  public struct PROCESS_INFORMATION { public IntPtr hProcess; public IntPtr hThread; public Int32 dwProcessId; public Int32 dwThreadId; }
  [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
  public static extern bool CreateProcess(string app, string commandLine, IntPtr pa, IntPtr ta, bool inherit, uint flags, IntPtr env, string cwd, ref STARTUPINFO si, out PROCESS_INFORMATION pi);
  [DllImport("kernel32.dll", SetLastError=true)] public static extern bool CloseHandle(IntPtr h);
}
'@

$SecurityDir=Join-Path $env:ProgramData 'VSN\security'
$ServiceName='VSN-Agent'
$ExpectedOwned=@('VSN Dev Platform.exe','bin\vsn.exe','bin\vsn-agent.exe')
$EvidencePath=Join-Path (Get-Location) $EvidenceDir
$SnapshotsPath=Join-Path $EvidencePath 'snapshots'
$LogsPath=Join-Path $EvidencePath 'msi-logs'
New-Item -ItemType Directory -Force $EvidencePath,$SnapshotsPath,$LogsPath|Out-Null
Write-UiEvidence

# Current Windows images can expose a real wizard Close button while WM_CLOSE
# alone is ignored. Invoke the actual terminal affordance first, native close
# only as fallback. This changes certification UI handling, not product state.
function Invoke-NativeTerminal([string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.AutomationElement]$Button,[string]$Name){
  try{
    $invoke=[System.Windows.Automation.InvokePattern]$Button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $invoke.Invoke()
    [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='invoke-real-terminal-control';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence;Start-Sleep -Milliseconds 350;return
  }catch{}
  $root=[IntPtr]::Zero;try{$root=[IntPtr][int]$Window.Current.NativeWindowHandle}catch{return}
  if($root -ne [IntPtr]::Zero -and [Vsn0315NativeUi]::IsWindow($root)){
    [void][Vsn0315NativeUi]::PostMessage($root,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='native-terminal-close-fallback';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')});Write-UiEvidence
  }
}

function Test-ServiceAbsent { return $null -eq (Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue) }
function Get-ServiceEvidence {
  $s=Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue
  if($null -eq $s){return $null}
  [pscustomobject][ordered]@{name=[string]$s.Name;display_name=[string]$s.DisplayName;start_name=[string]$s.StartName;start_mode=[string]$s.StartMode;state=[string]$s.State;path_name=[string]$s.PathName}
}
function Wait-ServiceRunning([string]$Label){
  $deadline=[DateTime]::UtcNow.AddSeconds(30)
  while([DateTime]::UtcNow -lt $deadline){$s=Get-ServiceEvidence;if($null -ne $s -and $s.state -eq 'Running'){return $s};Start-Sleep -Milliseconds 300}
  throw "$Label VSN-Agent did not reach Running."
}
function Get-OwnedHashes([string]$Root,[string]$Label){
  $rows=@();foreach($rel in $ExpectedOwned){$p=Join-Path $Root $rel;Assert-Condition (Test-Path -LiteralPath $p -PathType Leaf) "$Label missing $rel";$rows+=[pscustomobject][ordered]@{relative_path=$rel;path=(Resolve-Path $p).Path;size_bytes=[long](Get-Item $p).Length;sha256=Get-Sha256 $p}}
  return $rows
}
function Assert-OwnedHashesEqual([object[]]$Expected,[string]$Root,[string]$Label){
  $actual=@(Get-OwnedHashes $Root $Label);foreach($row in $Expected){$match=$actual|Where-Object relative_path -eq $row.relative_path|Select-Object -First 1;Assert-Condition ($null -ne $match -and $match.sha256 -eq $row.sha256) "$Label payload hash changed: $($row.relative_path)"};return $actual
}
function Get-ProcessEvidence([int]$Pid,[string]$ExpectedPath,[string]$Role,[string]$ExecutionState){
  $p=Get-Process -Id $Pid -ErrorAction Stop
  $cim=Get-CimInstance Win32_Process -Filter "ProcessId=$Pid" -ErrorAction Stop
  $actual=[IO.Path]::GetFullPath([string]$cim.ExecutablePath)
  $expected=[IO.Path]::GetFullPath($ExpectedPath)
  Assert-Condition ($actual -eq $expected) "$Role image mismatch: expected=$expected actual=$actual"
  Assert-Condition (-not $p.HasExited) "$Role process exited before installer invocation."
  return [pscustomobject][ordered]@{role=$Role;pid=$Pid;path=$actual;sha256=Get-Sha256 $actual;execution_state=$ExecutionState;alive=$true}
}
function Start-SuspendedExact([string]$Path,[string]$Role){
  $si=New-Object Vsn0319Process+STARTUPINFO;$si.cb=[Runtime.InteropServices.Marshal]::SizeOf($si);$pi=New-Object Vsn0319Process+PROCESS_INFORMATION
  $cmd='"'+$Path+'"'
  $ok=[Vsn0319Process]::CreateProcess($Path,$cmd,[IntPtr]::Zero,[IntPtr]::Zero,$false,[uint32]0x00000004,[IntPtr]::Zero,(Split-Path $Path),[ref]$si,[ref]$pi)
  if(-not $ok){throw "$Role CreateProcess(CREATE_SUSPENDED) failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"}
  [void][Vsn0319Process]::CloseHandle($pi.hThread);[void][Vsn0319Process]::CloseHandle($pi.hProcess)
  return [int]$pi.dwProcessId
}
function Test-ProcessAlive([int]$Pid){try{$p=Get-Process -Id $Pid -ErrorAction Stop;return -not $p.HasExited}catch{return $false}}
function Stop-OperatorResources([object]$Resources,[bool]$Machine,[string]$Phase){
  $before=[ordered]@{desktop_alive=(Test-ProcessAlive $Resources.desktop.pid);cli_alive=(Test-ProcessAlive $Resources.cli.pid);service=Get-ServiceEvidence}
  foreach($pid in @([int]$Resources.desktop.pid,[int]$Resources.cli.pid)){if(Test-ProcessAlive $pid){Stop-Process -Id $pid -Force -ErrorAction Stop}}
  if($Machine -and -not (Test-ServiceAbsent)){Stop-Service -Name $ServiceName -Force -ErrorAction Stop;Start-Sleep -Milliseconds 500}
  [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='operator-cleanup-after-proven-block';desktop_pid=$Resources.desktop.pid;cli_pid=$Resources.cli.pid;service_stop=$Machine;at_utc=[DateTime]::UtcNow.ToString('o')});Write-UiEvidence
  return [pscustomobject][ordered]@{before=$before;desktop_alive_after=(Test-ProcessAlive $Resources.desktop.pid);cli_alive_after=(Test-ProcessAlive $Resources.cli.pid);service_after=Get-ServiceEvidence}
}
function Start-RunningResources([string]$Root,[bool]$Machine,[string]$Phase){
  $desktopPath=Join-Path $Root 'VSN Dev Platform.exe';$cliPath=Join-Path $Root 'bin\vsn.exe'
  $desktop=Start-Process -FilePath $desktopPath -PassThru;Start-Sleep -Seconds 2
  Assert-Condition (-not $desktop.HasExited) "$Phase Desktop did not remain running."
  $cliPid=Start-SuspendedExact $cliPath "$Phase CLI"
  if($Machine){if(Test-ServiceAbsent){throw "$Phase VSN-Agent service missing."};$s=Get-ServiceEvidence;if($s.state -ne 'Running'){Start-Service -Name $ServiceName -ErrorAction Stop};$s=Wait-ServiceRunning $Phase}else{Assert-Condition (Test-ServiceAbsent) "$Phase current-user unexpectedly has machine service.";$s=$null}
  $d=Get-ProcessEvidence $desktop.Id $desktopPath 'desktop' 'running';$c=Get-ProcessEvidence $cliPid $cliPath 'cli' 'create-suspended-deterministic-file-in-use'
  [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='establish-running-resources';desktop_pid=$d.pid;cli_pid=$c.pid;agent_running=$Machine;at_utc=[DateTime]::UtcNow.ToString('o')});Write-UiEvidence
  return [pscustomobject][ordered]@{desktop=$d;cli=$c;service=$s;harness_pre_kill=$false}
}
function Get-WindowText([System.Windows.Automation.AutomationElement]$Window){
  $names=@(Get-SafeName $Window)
  foreach($type in @([System.Windows.Automation.ControlType]::Text,[System.Windows.Automation.ControlType]::Button,[System.Windows.Automation.ControlType]::RadioButton)){
    foreach($e in @(Get-Controls $Window $type)){try{$n=Get-SafeName $e;if($n){$names+=$n}}catch{}}
  }
  return ($names -join ' | ')
}
function Get-InstalledCoherence([string]$Root,[string]$Arp,[object[]]$Expected,[bool]$Machine,[string]$Label){
  Assert-Condition (Test-Path -LiteralPath $Arp) "$Label ARP identity missing."
  $hashes=Assert-OwnedHashesEqual $Expected $Root $Label
  $svc=Get-ServiceEvidence
  if($Machine){Assert-Condition ($null -ne $svc) "$Label service missing.";Assert-Condition ($svc.name -eq $ServiceName -and $svc.start_name -match '(?i)LocalService') "$Label service identity drifted."}
  else{Assert-Condition (Test-ServiceAbsent) "$Label current-user created machine service."}
  return [pscustomobject][ordered]@{installed=$true;arp_present=$true;owned=$hashes;service=$svc;coherent=$true}
}
function Invoke-RunningOperation(
  [System.Diagnostics.Process]$Installer,
  [string]$Phase,
  [string]$Root,
  [string]$Arp,
  [object[]]$ExpectedHashes,
  [object]$Resources,
  [bool]$Machine,
  [int]$TimeoutSeconds=120
){
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds);$visible=$false;$operationInvoked=$false;$coordinationObserved=$false;$blockText=@();$cancelRequested=$false
  while([DateTime]::UtcNow -lt $deadline){
    $windows=@(Get-RelevantWindows $Installer.Id)
    foreach($window in $windows){
      $visible=$true;Record-Window $Phase $window;$text=Get-WindowText $window
      if($text -match '(?i)(files? in use|application.+in use|applications?.+running|close.+applications?|restart manager|MsiRMFilesInUse|retry.+cancel|abort.+retry.+ignore)'){$coordinationObserved=$true;$blockText+=$text}
      if(-not $operationInvoked){$clicked=Invoke-Button $Phase $window @('^Uninstall$','^Remove$','^Next\b','^Yes$') $false;if($clicked -match '(?i)^(Uninstall|Remove)$'){$operationInvoked=$true}}
      elseif($coordinationObserved -and -not $cancelRequested){$clicked=Invoke-Button $Phase $window @('^Cancel$','^Abort$','^Close$') $false;if($clicked){$cancelRequested=$true}}
      elseif($cancelRequested){[void](Invoke-Button $Phase $window @('^Yes$','^OK$','^Close$','^Finish$') $true)}
    }
    $exited=$false;try{$Installer.Refresh();$exited=$Installer.HasExited}catch{$exited=$true}
    if($exited -and $windows.Count -eq 0){break}
    Start-Sleep -Milliseconds 300
  }
  Assert-Condition $visible "$Phase did not expose visible installer UI."
  Assert-Condition $operationInvoked "$Phase never invoked uninstall/remove."
  $exited=$false;try{$Installer.Refresh();$exited=$Installer.HasExited}catch{$exited=$true}
  Assert-Condition $exited "$Phase exceeded bounded runtime; indefinite hang is forbidden."
  $exit=[int]$Installer.ExitCode
  $desktopAlive=Test-ProcessAlive $Resources.desktop.pid;$cliAlive=Test-ProcessAlive $Resources.cli.pid
  $stillInstalled=(Test-Path -LiteralPath $Arp) -and (Test-Path -LiteralPath (Join-Path $Root 'VSN Dev Platform.exe') -PathType Leaf)
  if($exit -eq 0 -and -not $stillInstalled){
    Assert-Condition $coordinationObserved "$Phase completed after running resources without observable coordination; silent handling is forbidden."
    Assert-Condition (-not $desktopAlive -and -not $cliAlive) "$Phase removed payload while product process remained alive."
    if($Machine){Assert-Condition (Test-ServiceAbsent) "$Phase completed uninstall but service remains."}
    return [pscustomobject][ordered]@{phase=$Phase;outcome='coordinated_completion';exit_code=$exit;coordination_observed=$true;block_text=@($blockText|Sort-Object -Unique);desktop_alive_after=$desktopAlive;cli_alive_after=$cliAlive;harness_pre_kill=$false}
  }
  Assert-Condition $coordinationObserved "$Phase non-success was not tied to an explicit running-resource coordination/block observation."
  Assert-Condition $stillInstalled "$Phase blocked only after destructive package mutation."
  Assert-Condition ($desktopAlive -and $cliAlive) "$Phase silently terminated a product process before deterministic block."
  $coherence=Get-InstalledCoherence $Root $Arp $ExpectedHashes $Machine "$Phase safe block"
  return [pscustomobject][ordered]@{phase=$Phase;outcome='deterministic_safe_block';exit_code=$exit;coordination_observed=$true;block_text=@($blockText|Sort-Object -Unique);desktop_alive_after=$desktopAlive;cli_alive_after=$cliAlive;installed_coherence=$coherence;harness_pre_kill=$false}
}
function Get-RestartManagerEvidence([string]$LogPath){
  $log=Get-LogEvidence $LogPath;$matches=@(Select-String -LiteralPath $LogPath -Pattern 'Restart Manager|MsiRMFilesInUse|FilesInUse|MSIRESTARTMANAGERCONTROL|RM session' -CaseSensitive:$false -ErrorAction SilentlyContinue|ForEach-Object{$_.Line.Trim()}|Select-Object -Unique)
  Assert-Condition ($matches.Count -gt 0) 'MSI log contains no Restart Manager/files-in-use evidence.'
  return [pscustomobject][ordered]@{log=$log;matches=$matches;restart_manager_evidence=$true}
}
function Reset-RunCreatedSecurity([string]$Phase){
  if(-not (Test-Path -LiteralPath $SecurityDir)){return [pscustomobject][ordered]@{existed=$false;reset=$false}}
  Assert-Condition (Test-ServiceAbsent) "$Phase cannot reset security while service exists."
  $rows=@();foreach($item in @(Get-ChildItem -LiteralPath $SecurityDir -Recurse -Force -File -ErrorAction SilentlyContinue)){$rows+=[pscustomobject][ordered]@{path=$item.FullName;sha256=Get-Sha256 $item.FullName;size_bytes=[long]$item.Length}}
  Remove-Item -LiteralPath $SecurityDir -Recurse -Force;Assert-Condition (-not (Test-Path -LiteralPath $SecurityDir)) "$Phase security reset failed."
  [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='certification-lane-reset-run-created-security';files=$rows;at_utc=[DateTime]::UtcNow.ToString('o')});Write-UiEvidence
  return [pscustomobject][ordered]@{existed=$true;reset=$true;files=$rows}
}
function Invoke-NsisLifecycle([string]$Setup,[string]$Root,[string]$Arp,[bool]$Machine,[string]$Name){
  Assert-Condition (-not (Test-Path -LiteralPath $Root)) "$Name root exists at preflight.";Assert-Condition (-not (Test-Path -LiteralPath $Arp)) "$Name ARP exists at preflight."
  $baseline=Join-Path $SnapshotsPath "$Name-baseline.json";[void](Write-Pkg0313Snapshot -Path $baseline)
  $p=Start-Process -FilePath $Setup -PassThru;$install=Drive-SuccessUi $p "$Name-install" {(Test-Path -LiteralPath (Join-Path $Root 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $Arp)};Assert-Condition ($install.exit_code -eq 0) "$Name install failed."
  $hashes=@(Get-OwnedHashes $Root "$Name installed")
  $resources=Start-RunningResources $Root $Machine "$Name-running"
  $uninstaller=Join-Path $Root 'uninstall.exe';Assert-Condition (Test-Path -LiteralPath $uninstaller -PathType Leaf) "$Name uninstaller missing."
  $opProc=Start-Process -FilePath $uninstaller -PassThru;$operation=Invoke-RunningOperation $opProc "$Name-running-uninstall" $Root $Arp $hashes $resources $Machine
  $operatorCleanup=$null;$retry=$null
  if($operation.outcome -eq 'deterministic_safe_block'){
    $operatorCleanup=Stop-OperatorResources $resources $Machine "$Name-operator-cleanup"
    $p=Start-Process -FilePath $uninstaller -PassThru;$retry=Drive-SuccessUi $p "$Name-retry-uninstall" {-not (Test-Path -LiteralPath (Join-Path $Root 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $Arp)} $true;Assert-Condition ($retry.exit_code -eq 0) "$Name retry uninstall failed."
  }
  Assert-Condition (-not (Test-Path -LiteralPath $Arp)) "$Name ARP remains after accepted outcome.";Assert-Condition (Test-ServiceAbsent) "$Name service remains after accepted outcome."
  $securityReset=$(if($Machine){Reset-RunCreatedSecurity "$Name-final"}else{[pscustomobject][ordered]@{existed=$false;reset=$false}})
  $final=Join-Path $SnapshotsPath "$Name-final.json";[void](Write-Pkg0313Snapshot -Path $final);Assert-Pkg0313SnapshotEqual -BaselinePath $baseline -CandidatePath $final -Label "$Name running-resource lifecycle"
  return [pscustomobject][ordered]@{lifecycle=$Name;install=$install;initial_owned=$hashes;resources=$resources;operation=$operation;operator_cleanup=$operatorCleanup;retry=$retry;security_lane_reset=$securityReset;protected_state_equal=$true}
}
function Invoke-MsiLifecycle([string]$Package,[string]$ProductCode){
  $name='wix-per-machine';$root=$MachineRoot;$arp=Get-MsiArp $ProductCode;$msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
  Assert-Condition (-not (Test-Path -LiteralPath $root)) 'MSI root exists at preflight.';Assert-Condition (-not (Test-Path -LiteralPath $arp)) 'MSI ARP exists at preflight.'
  $baseline=Join-Path $SnapshotsPath "$name-baseline.json";[void](Write-Pkg0313Snapshot -Path $baseline)
  $installLog=Join-Path $LogsPath 'msi-install.log';$p=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $Package),'/L*V',('"{0}"' -f $installLog)) -PassThru;$install=Drive-SuccessUi $p 'wix-per-machine-install' {(Test-Path -LiteralPath (Join-Path $root 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $arp)};Assert-Condition ($install.exit_code -eq 0) 'MSI install failed.';[void](Get-LogEvidence $installLog)
  $hashes=@(Get-OwnedHashes $root 'MSI installed');$resources=Start-RunningResources $root $true 'wix-per-machine-running'
  $runningLog=Join-Path $LogsPath 'msi-running-uninstall.log';$opProc=Start-Process -FilePath $msiexec -ArgumentList @('/x',$ProductCode,'/L*V',('"{0}"' -f $runningLog)) -PassThru;$operation=Invoke-RunningOperation $opProc 'wix-per-machine-running-uninstall' $root $arp $hashes $resources $true;$rm=Get-RestartManagerEvidence $runningLog
  $operatorCleanup=$null;$retry=$null;$retryLogEvidence=$null
  if($operation.outcome -eq 'deterministic_safe_block'){
    $operatorCleanup=Stop-OperatorResources $resources $true 'wix-per-machine-operator-cleanup'
    $retryLog=Join-Path $LogsPath 'msi-retry-uninstall.log';$p=Start-Process -FilePath $msiexec -ArgumentList @('/x',$ProductCode,'/L*V',('"{0}"' -f $retryLog)) -PassThru;$retry=Drive-SuccessUi $p 'wix-per-machine-retry-uninstall' {-not (Test-Path -LiteralPath (Join-Path $root 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $arp)} $true;Assert-Condition ($retry.exit_code -eq 0) 'MSI retry uninstall failed.';$retryLogEvidence=Get-LogEvidence $retryLog
  }
  Assert-Condition (-not (Test-Path -LiteralPath $arp)) 'MSI ARP remains after accepted outcome.';Assert-Condition (Test-ServiceAbsent) 'MSI service remains after accepted outcome.'
  $securityReset=Reset-RunCreatedSecurity 'wix-per-machine-final';$final=Join-Path $SnapshotsPath "$name-final.json";[void](Write-Pkg0313Snapshot -Path $final);Assert-Pkg0313SnapshotEqual -BaselinePath $baseline -CandidatePath $final -Label 'MSI running-resource lifecycle'
  return [pscustomobject][ordered]@{lifecycle=$name;install=$install;initial_owned=$hashes;resources=$resources;operation=$operation;restart_manager=$rm;operator_cleanup=$operatorCleanup;retry=$retry;retry_log=$retryLogEvidence;security_lane_reset=$securityReset;protected_state_equal=$true}
}

$actual=(git rev-parse HEAD).Trim();Assert-Condition ($actual -eq $SourceSha) "Source SHA mismatch expected=$SourceSha actual=$actual"
$CurrentUserNsisPath=(Resolve-Path $CurrentUserNsisPath).Path;$PerMachineNsisPath=(Resolve-Path $PerMachineNsisPath).Path;$MsiPath=(Resolve-Path $MsiPath).Path
foreach($pkg in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)){Assert-Condition ((Get-Item $pkg).Length -gt 0) "Empty package: $pkg"}
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
Assert-Condition (-not (Test-Path -LiteralPath $UserRoot)) 'Current-user root exists at preflight.';Assert-Condition (-not (Test-Path -LiteralPath $MachineRoot)) 'Machine root exists at preflight.';Assert-Condition (Test-ServiceAbsent) 'Service exists at preflight.';Assert-Condition (-not (Test-Path -LiteralPath $SecurityDir)) 'Machine security exists at preflight.'

$current=Invoke-NsisLifecycle $CurrentUserNsisPath $UserRoot $HkcuKey $false 'nsis-current-user'
$machine=Invoke-NsisLifecycle $PerMachineNsisPath $MachineRoot $HklmNsisKey $true 'nsis-per-machine'
$wix=Invoke-MsiLifecycle $MsiPath $productCode

$tracked=@(git status --porcelain=v1 --untracked-files=no);if($tracked.Count -ne 0){$tracked|Write-Host;throw 'Tracked repository drift detected during 03.19 lifecycle.'};Write-UiEvidence
$evidence=[ordered]@{
  schema_version=1;package_id='PKG-03';task_id='03.19';source_commit=$SourceSha
  packages=[ordered]@{nsis_current_user=[ordered]@{path=$CurrentUserNsisPath;sha256=Get-Sha256 $CurrentUserNsisPath};nsis_per_machine=[ordered]@{path=$PerMachineNsisPath;sha256=Get-Sha256 $PerMachineNsisPath};msi=[ordered]@{path=$MsiPath;sha256=Get-Sha256 $MsiPath;product_code=$productCode}}
  lifecycles=@($current,$machine,$wix)
  harness_pre_kill=$false
  installer_coordination_or_safe_block_required=$true
  silent_force_kill_forbidden=$true
  indefinite_hang_forbidden=$true
  partial_package_state_forbidden=$true
  msi_restart_manager_evidence_required=$true
  service_identity_invariant_required=$true
  reboot_semantics_claimed=$false;silent_or_passive_deployment_claimed=$false;signing_claimed=$false;updater_mutation_claimed=$false;product_or_installer_mutation=$false
  tracked_repository_drift_zero=$true
}
$file=Join-Path $EvidencePath 'evidence.json';$evidence|ConvertTo-Json -Depth 20|Set-Content -LiteralPath $file -Encoding utf8NoBOM;$digest=Get-Sha256 $file;"$digest  evidence.json"|Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM;$evidence|ConvertTo-Json -Depth 20
