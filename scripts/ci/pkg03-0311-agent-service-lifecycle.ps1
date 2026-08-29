param(
    [Parameter(Mandatory=$true)][string]$CurrentUserSetupPath,
    [Parameter(Mandatory=$true)][string]$PerMachineSetupPath,
    [Parameter(Mandatory=$true)][string]$MsiPath,
    [Parameter(Mandatory=$true)][string]$SourceSha,
    [string]$EvidenceDir = 'dist-pkg03/03.11'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class Vsn0311NativeUi {
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll", SetLastError=true)] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr GetAncestor(IntPtr hWnd, uint gaFlags);
  [DllImport("user32.dll", SetLastError=true)] public static extern int GetDlgCtrlID(IntPtr hWnd);
  [DllImport("user32.dll")] [return: MarshalAs(UnmanagedType.Bool)] public static extern bool IsWindow(IntPtr hWnd);
}
'@

$ProductName = 'VSN Dev Platform'
$ServiceName = 'VSN-Agent'
$ServiceDisplayName = 'VSN Agent'
$ServiceAccount = 'NT AUTHORITY\LocalService'
$UserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot = Join-Path $env:ProgramFiles $ProductName
$HkcuKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$HklmKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()
$TerminalRoots = [System.Collections.Generic.HashSet[string]]::new()

function Assert-Condition([bool]$Condition,[string]$Message) { if (-not $Condition) { throw $Message } }
function Get-Sha256([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) { try { ([string]$Element.Current.Name).Trim() } catch { '' } }
function Get-Controls([System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.ControlType]$Type) {
    $c = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty,$Type)
    @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants,$c))
}
function Get-RelevantWindows([int]$RootPid) {
    $snapshot = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId,ParentProcessId)
    $family = [System.Collections.Generic.HashSet[int]]::new(); [void]$family.Add($RootPid)
    do {
        $changed=$false
        foreach($p in $snapshot){
            $pidNow=[int]$p.ProcessId;$ppid=[int]$p.ParentProcessId
            if($family.Contains($ppid)-and -not $family.Contains($pidNow)){[void]$family.Add($pidNow);$changed=$true}
        }
    } while($changed)
    $root=[System.Windows.Automation.AutomationElement]::RootElement
    $all=$root.FindAll([System.Windows.Automation.TreeScope]::Children,[System.Windows.Automation.Condition]::TrueCondition)
    $result=@()
    foreach($el in $all){
        try {
            $name=[string]$el.Current.Name;$pidNow=[int]$el.Current.ProcessId;$visible=-not [bool]$el.Current.IsOffscreen;$handle=[int]$el.Current.NativeWindowHandle
            $fallback=$name -match '(?i)VSN Dev Platform|Windows Installer'
            if($visible -and $handle -ne 0 -and ($family.Contains($pidNow)-or $fallback)){ $result += $el }
        } catch {}
    }
    @($result)
}
function Record-Window([string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
    $buttons=@(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
    $checks=@(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
    [void]$Observations.Add([pscustomobject][ordered]@{phase=$Phase;pid=[int]$Window.Current.ProcessId;title=(Get-SafeName $Window);buttons=$buttons;checkboxes=$checks;at_utc=[DateTime]::UtcNow.ToString('o')})
}
function Disable-SafetyCheckboxes([string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
    foreach($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))){
        $name=Get-SafeName $box
        if($name -notmatch '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform|delete.*(app.*data|user.*data)|remove.*(app.*data|user.*data)'){continue}
        try {
            $toggle=[System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
            if($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off){$toggle.Toggle();Start-Sleep -Milliseconds 180}
            [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='ensure-checkbox-off';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
        } catch {}
    }
}
function Invoke-TerminalFallback([string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.AutomationElement]$Button,[string]$ButtonName,[bool]$Allowed) {
    if(-not $Allowed){return}
    try{$buttonHandle=[IntPtr][int]$Button.Current.NativeWindowHandle}catch{return}
    if($buttonHandle -eq [IntPtr]::Zero -or -not [Vsn0311NativeUi]::IsWindow($buttonHandle)){return}
    $rootHandle=[Vsn0311NativeUi]::GetAncestor($buttonHandle,[uint32]2)
    if($rootHandle -eq [IntPtr]::Zero){try{$rootHandle=[IntPtr][int]$Window.Current.NativeWindowHandle}catch{return}}
    if($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0311NativeUi]::IsWindow($rootHandle)){return}
    $key="${Phase}:$($rootHandle.ToInt64())";if(-not $TerminalRoots.Add($key)){return}
    $controlId=[Vsn0311NativeUi]::GetDlgCtrlID($buttonHandle)
    if($controlId -gt 0){[void][Vsn0311NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle);Start-Sleep -Milliseconds 350}
    if([Vsn0311NativeUi]::IsWindow($rootHandle)){[void][Vsn0311NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)}
    [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='terminal-native-fallback';control=$ButtonName;at_utc=[DateTime]::UtcNow.ToString('o')})
}
function Invoke-Primary([string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[bool]$CompletionReached) {
    $priority=if($Phase -match 'uninstall'){@('^Remove$','^Uninstall$','^Next\b','^Yes$','^Finish$','^Close$','^OK$')}else{@('^Install$','^Next\b','^Finish$','^Close$','^OK$')}
    $buttons=@()
    foreach($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))){
        try{if(-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen){continue};$name=Get-SafeName $button;if($name){$buttons += [pscustomobject]@{Element=$button;Name=$name;Norm=($name-replace '&','').Trim()}}}catch{}
    }
    foreach($pattern in $priority){
        $selected=$buttons|Where-Object{$_.Norm -match "(?i)$pattern"}|Select-Object -First 1
        if($null -eq $selected){continue}
        try {
            $invoke=[System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern);$invoke.Invoke()
            [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='invoke-button';control=$selected.Name;at_utc=[DateTime]::UtcNow.ToString('o')})
            if($selected.Norm -match '(?i)^(Finish|Close|OK)$'){Start-Sleep -Milliseconds 300;Invoke-TerminalFallback $Phase $Window $selected.Element $selected.Name $CompletionReached}
            return
        }catch{}
    }
}
function Drive-Ui([System.Diagnostics.Process]$RootProcess,[string]$Phase,[scriptblock]$Completion,[int]$TimeoutSeconds=240) {
    $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds);$visible=$false;$quiet=0
    while([DateTime]::UtcNow -lt $deadline){
        $complete=[bool](& $Completion);$windows=@(Get-RelevantWindows $RootProcess.Id)
        if($windows.Count -eq 0){if($complete){$quiet++;if($quiet -ge 3){return [pscustomobject]@{visible=$visible;terminal_closed=$true}}}else{$quiet=0};Start-Sleep -Milliseconds 500;continue}
        $visible=$true;$quiet=0
        foreach($window in $windows){try{$window.SetFocus()}catch{};Record-Window $Phase $window;Disable-SafetyCheckboxes $Phase $window;Invoke-Primary $Phase $window $complete;Start-Sleep -Milliseconds 700;break}
    }
    throw "Timed out driving $Phase UI."
}
function Observe-ProcessUi([System.Diagnostics.Process]$RootProcess,[string]$Phase,[scriptblock]$Completion,[int]$TimeoutSeconds=300) {
    $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds);$visible=$false;$quiet=0
    while([DateTime]::UtcNow -lt $deadline){
        $complete=[bool](& $Completion)
        try{$RootProcess.Refresh()}catch{}
        $exited=$false;try{$exited=$RootProcess.HasExited}catch{}
        $windows=@(Get-RelevantWindows $RootProcess.Id)
        if($windows.Count -gt 0){
            $visible=$true;$quiet=0
            foreach($window in $windows){Record-Window $Phase $window}
        } elseif($complete -and $exited) {
            $quiet++;if($quiet -ge 3){$RootProcess.WaitForExit();return [pscustomobject]@{visible=$visible;exit_code=[int]$RootProcess.ExitCode;terminal_closed=$true}}
        } else {$quiet=0}
        if($exited -and -not $complete){$RootProcess.WaitForExit();return [pscustomobject]@{visible=$visible;exit_code=[int]$RootProcess.ExitCode;terminal_closed=($windows.Count -eq 0)}}
        Start-Sleep -Milliseconds 500
    }
    throw "Timed out observing $Phase UI."
}
function Get-ServiceSnapshot { Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue }
function Test-ServiceAbsent { $null -eq (Get-ServiceSnapshot) }
function Wait-ServiceState([string]$Expected,[int]$TimeoutSeconds=45) {
    $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while([DateTime]::UtcNow -lt $deadline){$s=Get-ServiceSnapshot;if($Expected -eq 'Absent'){if($null -eq $s){return}}elseif($null -ne $s -and [string]$s.State -eq $Expected){return};Start-Sleep -Milliseconds 500}
    throw "Service $ServiceName did not reach $Expected."
}
function Assert-ServiceContract([string]$Root,[string]$Lane) {
    $svc=Get-ServiceSnapshot;Assert-Condition ($null -ne $svc) "$Lane service is absent."
    $agent=Join-Path $Root 'bin\vsn-agent.exe';$escaped=[regex]::Escape($agent)
    Assert-Condition ([string]$svc.DisplayName -eq $ServiceDisplayName) "$Lane service DisplayName mismatch: $($svc.DisplayName)"
    Assert-Condition ([string]$svc.StartMode -eq 'Auto') "$Lane service StartMode mismatch: $($svc.StartMode)"
    Assert-Condition ([string]$svc.StartName -eq $ServiceAccount) "$Lane service account mismatch: $($svc.StartName)"
    Assert-Condition ([string]$svc.PathName -match ('^"?'+$escaped+'"?\s+--service-run$')) "$Lane service PathName mismatch: $($svc.PathName)"
    Assert-Condition ([string]$svc.State -eq 'Running') "$Lane service is not RUNNING: $($svc.State)"
    [pscustomobject][ordered]@{name=[string]$svc.Name;display_name=[string]$svc.DisplayName;state=[string]$svc.State;start_mode=[string]$svc.StartMode;start_name=[string]$svc.StartName;path_name=[string]$svc.PathName}
}
function Invoke-Agent([string]$Agent,[string]$Verb,[string]$Lane) {
    $output=(& $Agent service $Verb 2>&1 | Out-String).Trim();$code=$LASTEXITCODE
    Assert-Condition ($code -eq 0) "$Lane agent service $Verb failed: exit=$code output=$output"
    [pscustomobject][ordered]@{verb=$Verb;exit_code=$code;output=$output}
}
function Invoke-Ping([string]$Cli,[string]$Lane) {
    $output=(& $Cli ping 2>&1 | Out-String).Trim();$code=$LASTEXITCODE
    Assert-Condition ($code -eq 0) "$Lane installed CLI ping failed: exit=$code output=$output"
    [pscustomobject][ordered]@{exit_code=$code;output=$output}
}
function Exercise-RunningService([string]$Root,[string]$Lane) {
    $agent=Join-Path $Root 'bin\vsn-agent.exe';$cli=Join-Path $Root 'bin\vsn.exe'
    Assert-Condition (Test-Path -LiteralPath $agent -PathType Leaf) "$Lane Agent payload missing."
    Assert-Condition (Test-Path -LiteralPath $cli -PathType Leaf) "$Lane CLI payload missing."
    Wait-ServiceState Running
    $config=Assert-ServiceContract $Root $Lane;$ping1=Invoke-Ping $cli $Lane
    $stop=Invoke-Agent $agent stop $Lane;Wait-ServiceState Stopped
    $start=Invoke-Agent $agent start $Lane;Wait-ServiceState Running
    $ping2=Invoke-Ping $cli $Lane
    [pscustomobject][ordered]@{config=$config;initial_health=$ping1;stop=$stop;start=$start;second_health=$ping2;bounded_transitions=$true}
}
function Probe-StoppedServiceNativeCode([string]$Lane) {
    $sc=Join-Path $env:SystemRoot 'System32\sc.exe'
    $output=(& $sc stop $ServiceName 2>&1 | Out-String).Trim();$code=$LASTEXITCODE
    Assert-Condition ($code -eq 1062) "$Lane expected native ERROR_SERVICE_NOT_ACTIVE 1062, got exit=$code output=$output"
    Wait-ServiceState Stopped
    [pscustomobject][ordered]@{command='sc.exe stop VSN-Agent';exit_code=[int]$code;expected_already_stopped_code=1062;output=$output}
}
function Get-MsiProperty([string]$Path,[string]$Property) {
    $installer=New-Object -ComObject WindowsInstaller.Installer
    $db=$installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
    $view=$db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
    $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null)|Out-Null
    $record=$view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null);if($null -eq $record){throw "MSI property $Property missing."}
    [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}
function Start-UiProcess([string]$File,[string[]]$ProcessArgs=@()) {
    if(-not $ProcessArgs.Count){return (Start-Process -FilePath $File -PassThru)}
    $startInfo=[System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName=$File
    $startInfo.UseShellExecute=$false
    foreach($argument in $ProcessArgs){[void]$startInfo.ArgumentList.Add($argument)}
    $process=[System.Diagnostics.Process]::new()
    $process.StartInfo=$startInfo
    if(-not $process.Start()){throw "Failed to start process: $File"}
    $process
}
function Clear-StaleCurrentUserInstallerLocationMetadata {
    $subKey='Software\Vertex Systems Network\VSN Dev Platform'
    $key=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKey,$true)
    if($null -eq $key){return @()}
    $removed=@()
    try {
        $expected=$UserRoot.TrimEnd([char]'\')
        foreach($name in @('', 'InstallDir')){
            $raw=$key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            if($null -eq $raw){continue}
            $path=([string]$raw).Trim()
            if([string]::IsNullOrWhiteSpace($path)){continue}
            $normalized=$path.TrimEnd([char]'\')
            if($normalized -ine $expected){continue}
            if(Test-Path -LiteralPath $path){continue}
            $key.DeleteValue($name,$false)
            $label=if([string]::IsNullOrEmpty($name)){'(Default)'}else{$name}
            $removed += [pscustomobject][ordered]@{name=$label;stale_path=$path}
        }
    } finally {
        $key.Close()
    }
    @($removed)
}
function Write-Evidence([object]$Evidence) {
    New-Item -ItemType Directory -Force $EvidencePath|Out-Null
    @($Observations)|ConvertTo-Json -Depth 10|Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
    @($Actions)|ConvertTo-Json -Depth 10|Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
    $Evidence|ConvertTo-Json -Depth 14|Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
}

New-Item -ItemType Directory -Force $EvidencePath|Out-Null
$CurrentUserSetupPath=(Resolve-Path $CurrentUserSetupPath).Path;$PerMachineSetupPath=(Resolve-Path $PerMachineSetupPath).Path;$MsiPath=(Resolve-Path $MsiPath).Path
foreach($p in @($CurrentUserSetupPath,$PerMachineSetupPath,$MsiPath)){Assert-Condition ((Get-Item $p).Length -gt 0) "Installer is empty: $p"}
Assert-Condition (Test-ServiceAbsent) 'VSN-Agent unexpectedly exists before 03.11 lifecycle.'
Assert-Condition (-not (Test-Path $UserRoot)) 'Current-user install root exists before test.'
Assert-Condition (-not (Test-Path $MachineRoot)) 'Per-machine install root exists before test.'

$evidence=[ordered]@{schema_version=2;package_id='PKG-03';task_id='03.11';source_commit=$SourceSha;current_user=$null;lane_isolation=$null;per_machine=$null;msi=$null;tracked_repository_drift_zero=$false}
try {
    # Current-user NSIS: payload may install, but machine service must remain absent.
    $cuInstall=Start-UiProcess $CurrentUserSetupPath
    $cuInstallUi=Drive-Ui $cuInstall 'current-user-install' { (Test-Path (Join-Path $UserRoot 'bin\vsn-agent.exe')) -and (Test-Path $HkcuKey) } 240
    Assert-Condition $cuInstallUi.visible 'No visible current-user NSIS install UI observed.'
    Assert-Condition (Test-ServiceAbsent) 'Current-user NSIS registered VSN-Agent.'
    $cuAgent=Join-Path $UserRoot 'bin\vsn-agent.exe';$cuCli=Join-Path $UserRoot 'bin\vsn.exe';$cuUninstall=Join-Path $UserRoot 'uninstall.exe'
    Assert-Condition (Test-Path $cuAgent -PathType Leaf) 'Current-user Agent payload missing.';Assert-Condition (Test-Path $cuCli -PathType Leaf) 'Current-user CLI payload missing.';Assert-Condition (Test-Path $cuUninstall -PathType Leaf) 'Current-user uninstaller missing.'
    $cuUninstallProc=Start-UiProcess $cuUninstall
    $cuUninstallUi=Drive-Ui $cuUninstallProc 'current-user-uninstall' { -not (Test-Path $UserRoot) -and -not (Test-Path $HkcuKey) } 240
    Assert-Condition $cuUninstallUi.visible 'No visible current-user NSIS uninstall UI observed.';Assert-Condition (Test-ServiceAbsent) 'Current-user NSIS mutated service state during uninstall.'
    $evidence.current_user=[ordered]@{setup_sha256=Get-Sha256 $CurrentUserSetupPath;visible_install_ui_observed=[bool]$cuInstallUi.visible;visible_uninstall_ui_observed=[bool]$cuUninstallUi.visible;agent_payload_observed=$true;cli_payload_observed=$true;service_absent_after_install=$true;service_absent_after_uninstall=$true;machine_service_mutation_observed=$false}

    # Keep independent installer lanes isolated without deleting user data or the product metadata key.
    $removedStaleInstallerLocationValues=@(Clear-StaleCurrentUserInstallerLocationMetadata)
    $evidence.lane_isolation=[ordered]@{
        reason='Remove only stale Tauri installer-location metadata between independent certification lanes'
        removed_stale_installer_location_values=$removedStaleInstallerLocationValues
        product_registry_key_deleted=$false
        installer_language_deleted=$false
        user_data_deleted=$false
    }

    # Per-machine NSIS: hook must own service lifecycle, not Agent file ownership.
    $pmInstall=Start-UiProcess $PerMachineSetupPath
    $pmInstallUi=Drive-Ui $pmInstall 'per-machine-install' { (Test-Path (Join-Path $MachineRoot 'bin\vsn-agent.exe')) -and (Test-Path $HklmKey) -and -not (Test-ServiceAbsent) } 300
    Assert-Condition $pmInstallUi.visible 'No visible per-machine NSIS install UI observed.'
    $pmLifecycle=Exercise-RunningService $MachineRoot 'per-machine'
    $pmUninstall=Join-Path $MachineRoot 'uninstall.exe';Assert-Condition (Test-Path $pmUninstall -PathType Leaf) 'Per-machine uninstaller missing.'
    $pmUninstallProc=Start-UiProcess $pmUninstall
    $pmUninstallUi=Drive-Ui $pmUninstallProc 'per-machine-uninstall' { (Test-ServiceAbsent) -and -not (Test-Path $MachineRoot) -and -not (Test-Path $HklmKey) } 300
    Assert-Condition $pmUninstallUi.visible 'No visible per-machine NSIS uninstall UI observed.';Wait-ServiceState Absent
    $evidence.per_machine=[ordered]@{setup_sha256=Get-Sha256 $PerMachineSetupPath;visible_install_ui_observed=[bool]$pmInstallUi.visible;visible_uninstall_ui_observed=[bool]$pmUninstallUi.visible;service=$pmLifecycle;service_absent_after_uninstall=$true;payload_removed_after_uninstall=$true}

    # MSI/WiX: prove install/health and stopped-service removal; live-running uninstall coordination is owned by 03.19.
    $productCode=Get-MsiProperty $MsiPath 'ProductCode';$upgradeCode=Get-MsiProperty $MsiPath 'UpgradeCode'
    $msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
    $msiInstallLog=Join-Path $EvidencePath 'msi-install.log'
    $msiInstallArgs=@('/i',$MsiPath,'/qb!','/norestart','/l*v',$msiInstallLog)
    $msiInstall=Start-UiProcess $msiexec $msiInstallArgs
    $msiInstallUi=Observe-ProcessUi $msiInstall 'msi-install' { (Test-Path (Join-Path $MachineRoot 'bin\vsn-agent.exe')) -and -not (Test-ServiceAbsent) } 300
    Assert-Condition ($msiInstallUi.exit_code -eq 0) "MSI install failed: exit=$($msiInstallUi.exit_code) log=$msiInstallLog"
    Assert-Condition $msiInstallUi.visible 'No visible MSI basic install UI observed.'
    $msiLifecycle=Exercise-RunningService $MachineRoot 'msi'

    $msiAgent=Join-Path $MachineRoot 'bin\vsn-agent.exe'
    $certificationPreUninstallStop=Invoke-Agent $msiAgent stop 'msi-certification-pre-uninstall'
    Wait-ServiceState Stopped
    $stoppedSnapshot=Get-ServiceSnapshot
    Assert-Condition ($null -ne $stoppedSnapshot -and [string]$stoppedSnapshot.State -eq 'Stopped') 'MSI service is not Stopped before uninstall.'
    $nativeStoppedProbe=Probe-StoppedServiceNativeCode 'msi-stopped-service-probe'

    $msiUninstallLog=Join-Path $EvidencePath 'msi-uninstall.log'
    $msiUninstallArgs=@('/x',$productCode,'/qb!','/norestart','/l*v',$msiUninstallLog)
    $msiUninstall=Start-UiProcess $msiexec $msiUninstallArgs
    $msiUninstallUi=Observe-ProcessUi $msiUninstall 'msi-uninstall' { (Test-ServiceAbsent) -and -not (Test-Path $MachineRoot) } 300
    Assert-Condition ($msiUninstallUi.exit_code -eq 0) "MSI uninstall failed: exit=$($msiUninstallUi.exit_code) log=$msiUninstallLog"
    Assert-Condition $msiUninstallUi.visible 'No visible MSI basic uninstall UI observed.';Wait-ServiceState Absent
    $evidence.msi=[ordered]@{
        msi_sha256=Get-Sha256 $MsiPath
        product_code=$productCode
        upgrade_code=$upgradeCode
        visible_install_ui_observed=[bool]$msiInstallUi.visible
        visible_uninstall_ui_observed=[bool]$msiUninstallUi.visible
        install_exit_code=[int]$msiInstallUi.exit_code
        uninstall_exit_code=[int]$msiUninstallUi.exit_code
        install_log='msi-install.log'
        uninstall_log='msi-uninstall.log'
        service=$msiLifecycle
        certification_pre_uninstall_stop=$certificationPreUninstallStop
        service_state_before_uninstall='Stopped'
        native_stopped_service_probe=$nativeStoppedProbe
        live_running_coordination_owner='03.19'
        live_running_uninstall_certified=$false
        service_absent_after_uninstall=$true
        payload_removed_after_uninstall=$true
    }

    $tracked=@(git status --porcelain=v1 --untracked-files=no);if($tracked.Count -ne 0){$tracked|Write-Host;throw 'Tracked repository drift detected during 03.11.'}
    $evidence.tracked_repository_drift_zero=$true
    Write-Evidence $evidence
} catch {
    $evidence.failure=[ordered]@{message=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')}
    Write-Evidence $evidence
    throw
}
