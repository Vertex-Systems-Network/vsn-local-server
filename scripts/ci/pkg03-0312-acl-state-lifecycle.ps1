param(
    [Parameter(Mandatory=$true)][string]$CurrentUserSetupPath,
    [Parameter(Mandatory=$true)][string]$PerMachineSetupPath,
    [Parameter(Mandatory=$true)][string]$MsiPath,
    [Parameter(Mandatory=$true)][string]$SourceSha,
    [string]$EvidenceDir='dist-pkg03/03.12'
)

# Reuse the accepted 03.11 UI/service harness helpers verbatim at runtime.
# The 03.12 validator pins that source to the accepted canonical base.
$helperSource=Get-Content -LiteralPath 'scripts/ci/pkg03-0311-agent-service-lifecycle.ps1' -Raw
$helperStart=$helperSource.IndexOf('Set-StrictMode -Version Latest')
$helperEnd=$helperSource.IndexOf('New-Item -ItemType Directory -Force $EvidencePath|Out-Null')
if($helperStart -lt 0 -or $helperEnd -le $helperStart){throw 'Unable to locate accepted 03.11 helper boundary.'}
Invoke-Expression $helperSource.Substring($helperStart,$helperEnd-$helperStart)

$SecurityDir=Join-Path $env:ProgramData 'VSN\security'
$IpcKey=Join-Path $SecurityDir 'ipc.key'
$Harness0311Sha=(Get-FileHash -LiteralPath 'scripts/ci/pkg03-0311-agent-service-lifecycle.ps1' -Algorithm SHA256).Hash.ToLowerInvariant()

function Get-CurrentUserSid { [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value }
function Get-AclEvidence([string]$Path) {
    $acl=Get-Acl -LiteralPath $Path
    $rules=@()
    foreach($rule in @($acl.GetAccessRules($true,$false,[System.Security.Principal.SecurityIdentifier]))){
        $rules += [pscustomobject][ordered]@{
            sid=[string]$rule.IdentityReference.Value
            type=[string]$rule.AccessControlType
            rights=[string]$rule.FileSystemRights
            rights_mask=[int64]$rule.FileSystemRights
            inheritance=[string]$rule.InheritanceFlags
            propagation=[string]$rule.PropagationFlags
            inherited=[bool]$rule.IsInherited
        }
    }
    [pscustomobject][ordered]@{
        path=(Resolve-Path -LiteralPath $Path).Path
        owner=[string]$acl.Owner
        inheritance_protected=[bool]$acl.AreAccessRulesProtected
        sddl=$acl.GetSecurityDescriptorSddlForm([System.Security.AccessControl.AccessControlSections]::Access)
        rules=$rules
    }
}
function Get-AllowMask([object[]]$Rules,[string]$Sid) {
    [int64]$mask=0
    foreach($rule in @($Rules)){if($rule.sid -eq $Sid -and $rule.type -eq 'Allow'){$mask=$mask -bor [int64]$rule.rights_mask}}
    $mask
}
function Assert-Full([int64]$Mask,[string]$Label) {
    $full=[int64][System.Security.AccessControl.FileSystemRights]::FullControl
    Assert-Condition (($Mask -band $full) -eq $full) "$Label missing FullControl."
}
function Assert-ReadOnly([int64]$Mask,[string]$Label) {
    $read=[int64][System.Security.AccessControl.FileSystemRights]::Read
    $danger=[int64][System.Security.AccessControl.FileSystemRights]::Write -bor [int64][System.Security.AccessControl.FileSystemRights]::Delete -bor [int64][System.Security.AccessControl.FileSystemRights]::ChangePermissions -bor [int64][System.Security.AccessControl.FileSystemRights]::TakeOwnership
    Assert-Condition (($Mask -band $read) -eq $read) "$Label missing read rights."
    Assert-Condition (($Mask -band $danger) -eq 0) "$Label unexpectedly has write/delete/ACL-owner rights."
}
function Assert-IpcAclContract([string]$CreatorSid,[string]$Lane) {
    Assert-Condition (Test-Path -LiteralPath $SecurityDir -PathType Container) "$Lane security directory missing."
    Assert-Condition (Test-Path -LiteralPath $IpcKey -PathType Leaf) "$Lane ipc.key missing."
    $directory=Get-AclEvidence $SecurityDir
    $key=Get-AclEvidence $IpcKey
    Assert-Condition $directory.inheritance_protected "$Lane directory inheritance is enabled."
    Assert-Condition $key.inheritance_protected "$Lane key inheritance is enabled."
    $expected=@('S-1-5-18','S-1-5-32-544','S-1-5-19',$CreatorSid)
    foreach($entry in @($directory.rules)+@($key.rules)){
        Assert-Condition (-not [bool]$entry.inherited) "$Lane inherited ACE observed."
        if($entry.type -eq 'Deny'){throw "$Lane unexpected explicit deny SID $($entry.sid)."}
        if($entry.type -eq 'Allow' -and $entry.sid -notin $expected){throw "$Lane unexpected explicit allow SID $($entry.sid)."}
    }
    Assert-Full (Get-AllowMask $directory.rules 'S-1-5-18') "$Lane directory SYSTEM"
    Assert-Full (Get-AllowMask $directory.rules 'S-1-5-32-544') "$Lane directory Administrators"
    Assert-ReadOnly (Get-AllowMask $directory.rules 'S-1-5-19') "$Lane directory LocalService"
    Assert-Full (Get-AllowMask $directory.rules $CreatorSid) "$Lane directory creator"
    Assert-Full (Get-AllowMask $key.rules 'S-1-5-18') "$Lane key SYSTEM"
    Assert-Full (Get-AllowMask $key.rules 'S-1-5-32-544') "$Lane key Administrators"
    Assert-ReadOnly (Get-AllowMask $key.rules 'S-1-5-19') "$Lane key LocalService"
    Assert-ReadOnly (Get-AllowMask $key.rules $CreatorSid) "$Lane key creator"
    [pscustomobject][ordered]@{lane=$Lane;creator_sid=$CreatorSid;directory=$directory;key=$key;contract_passed=$true}
}
function Test-PathUnder([string]$Child,[string]$Root) {
    $childFull=[System.IO.Path]::GetFullPath($Child).TrimEnd('\')+'\'
    $rootFull=[System.IO.Path]::GetFullPath($Root).TrimEnd('\')+'\'
    $childFull.StartsWith($rootFull,[System.StringComparison]::OrdinalIgnoreCase)
}
function Invoke-LocalServiceProjectDirsProbe([string]$Lane) {
    $probeName='VSN-0312-ContextProbe'
    $probeDir=Join-Path $EvidencePath 'context-probe-src'
    $src=Join-Path $probeDir 'src'
    New-Item -ItemType Directory -Force $src|Out-Null
    $repo=(Get-Location).Path.Replace('\','/')
    @"
[package]
name = "vsn0312-context-probe"
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
    $targetRoot=(Resolve-Path 'target').Path
    $oldTarget=$env:CARGO_TARGET_DIR
    try {
        $env:CARGO_TARGET_DIR=$targetRoot
        & cargo build --release --offline --manifest-path (Join-Path $probeDir 'Cargo.toml')
        Assert-Condition ($LASTEXITCODE -eq 0) "$Lane ProjectDirs probe build failed."
    } finally {$env:CARGO_TARGET_DIR=$oldTarget}
    $probeExe=Join-Path $targetRoot 'release\vsn0312-context-probe.exe'
    $outputPath=Join-Path $env:WINDIR ("Temp\vsn-0312-context-"+[guid]::NewGuid().ToString('N')+".txt")
    $sc=Join-Path $env:SystemRoot 'System32\sc.exe'
    if(Get-CimInstance Win32_Service -Filter "Name='$probeName'" -ErrorAction SilentlyContinue){& $sc delete $probeName|Out-Null;Start-Sleep -Seconds 1}
    $binPath="`"$probeExe`" `"$outputPath`""
    $create=(& $sc create $probeName 'binPath=' $binPath 'start=' 'demand' 'obj=' 'NT AUTHORITY\LocalService' 2>&1|Out-String).Trim()
    Assert-Condition ($LASTEXITCODE -eq 0) "$Lane probe service create failed: $create"
    $start=$null
    try {
        $start=Start-Process -FilePath $sc -ArgumentList @('start',$probeName) -PassThru -WindowStyle Hidden
        $deadline=[DateTime]::UtcNow.AddSeconds(30)
        while([DateTime]::UtcNow -lt $deadline -and -not (Test-Path -LiteralPath $outputPath)){Start-Sleep -Milliseconds 250}
        Assert-Condition (Test-Path -LiteralPath $outputPath -PathType Leaf) "$Lane LocalService probe produced no output."
        $map=@{}
        foreach($line in Get-Content -LiteralPath $outputPath){$p=$line -split '=',2;if($p.Count -eq 2){$map[$p[0]]=$p[1]}}
        foreach($k in @('data_local','config_file','local_app_data','app_data','program_data')){Assert-Condition (-not [string]::IsNullOrWhiteSpace([string]$map[$k])) "$Lane probe missing $k."}
        [pscustomobject][ordered]@{lane=$Lane;service_account='NT AUTHORITY\LocalService';data_local=[string]$map.data_local;config_file=[string]$map.config_file;local_app_data=[string]$map.local_app_data;app_data=[string]$map.app_data;program_data=[string]$map.program_data}
    } finally {
        if($start){try{$start.Refresh();if(-not $start.HasExited){Stop-Process -Id $start.Id -Force -ErrorAction SilentlyContinue}}catch{}}
        & $sc delete $probeName 2>&1|Out-Null
        Remove-Item -LiteralPath $outputPath -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $probeDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
function Assert-MutableSeparation([object]$Probe,[string]$Lane) {
    Assert-Condition (-not (Test-PathUnder $Probe.data_local $MachineRoot)) "$Lane data_local is inside install root."
    Assert-Condition (-not (Test-PathUnder $Probe.config_file $MachineRoot)) "$Lane config is inside install root."
    Assert-Condition (-not (Test-PathUnder $Probe.data_local $SecurityDir)) "$Lane mutable data overlaps machine IPC security."
    $device=Join-Path $Probe.data_local 'security\device.json'
    Assert-Condition (Test-Path -LiteralPath $device -PathType Leaf) "$Lane Agent device identity missing at observed LocalService data root."
    [pscustomobject][ordered]@{data_local=$Probe.data_local;config_file=$Probe.config_file;device_identity_path=$device;data_outside_install_root=$true;config_outside_install_root=$true;machine_ipc_separate=$true}
}
function Reset-RunCreatedSecurityFixture {
    Assert-Condition (Test-ServiceAbsent) 'Cannot reset IPC fixture while VSN-Agent exists.'
    Assert-Condition (Test-Path -LiteralPath $IpcKey -PathType Leaf) 'Expected run-created IPC key before reset.'
    Remove-Item -LiteralPath $SecurityDir -Recurse -Force
    Assert-Condition (-not (Test-Path $SecurityDir)) 'Failed to reset run-created IPC fixture.'
}
function Write-Evidence0312([object]$Evidence) {
    New-Item -ItemType Directory -Force $EvidencePath|Out-Null
    @($Observations)|ConvertTo-Json -Depth 10|Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
    @($Actions)|ConvertTo-Json -Depth 10|Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
    $Evidence|ConvertTo-Json -Depth 20|Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
}

New-Item -ItemType Directory -Force $EvidencePath|Out-Null
$CurrentUserSetupPath=(Resolve-Path $CurrentUserSetupPath).Path
$PerMachineSetupPath=(Resolve-Path $PerMachineSetupPath).Path
$MsiPath=(Resolve-Path $MsiPath).Path
Assert-Condition (Test-ServiceAbsent) 'VSN-Agent unexpectedly exists before 03.12.'
Assert-Condition (-not (Test-Path $UserRoot)) 'Current-user install root already exists.'
Assert-Condition (-not (Test-Path $MachineRoot)) 'Per-machine install root already exists.'
Assert-Condition (-not (Test-Path $SecurityDir)) 'Machine IPC security state already exists.'
$creatorSid=Get-CurrentUserSid
$evidence=[ordered]@{schema_version=1;package_id='PKG-03';task_id='03.12';source_commit=$SourceSha;authoritative_integration='03.11 service install -> vsn_core::provision_local_ipc';helper_0311_sha256=$Harness0311Sha;duplicate_acl_writer_added=$false;current_user=$null;lane_isolation=$null;per_machine=$null;msi=$null;comprehensive_uninstall_preservation_owner='03.17';tracked_repository_drift_zero=$false}

try {
    $cuInstall=Start-UiProcess $CurrentUserSetupPath
    $cuUi=Drive-Ui $cuInstall 'current-user-install' {(Test-Path (Join-Path $UserRoot 'bin\vsn-agent.exe')) -and (Test-Path $HkcuKey)} 240
    Assert-Condition $cuUi.visible 'No visible current-user NSIS install UI.'
    Assert-Condition (Test-ServiceAbsent) 'Current-user install created service.'
    Assert-Condition (-not (Test-Path $SecurityDir)) 'Current-user install created machine IPC security.'
    $cuUninstall=Join-Path $UserRoot 'uninstall.exe'
    $cuUninstallProc=Start-UiProcess $cuUninstall
    $cuUnUi=Drive-Ui $cuUninstallProc 'current-user-uninstall' {-not (Test-Path $UserRoot) -and -not (Test-Path $HkcuKey)} 240
    Assert-Condition $cuUnUi.visible 'No visible current-user NSIS uninstall UI.'
    Assert-Condition (-not (Test-Path $SecurityDir)) 'Current-user uninstall created machine IPC security.'
    $evidence.current_user=[ordered]@{setup_sha256=Get-Sha256 $CurrentUserSetupPath;visible_install_ui_observed=[bool]$cuUi.visible;visible_uninstall_ui_observed=[bool]$cuUnUi.visible;machine_security_created_by_current_user_install=$false;machine_security_created_by_current_user_uninstall=$false;service_absent_after_uninstall=$true}
    $evidence.lane_isolation=[ordered]@{stale_current_user_installer_location_values_removed=@(Clear-StaleCurrentUserInstallerLocationMetadata);arbitrary_user_data_deleted=$false;security_fixture_reset_before_msi=$false}

    $pmInstall=Start-UiProcess $PerMachineSetupPath
    $pmUi=Drive-Ui $pmInstall 'per-machine-install' {(Test-Path (Join-Path $MachineRoot 'bin\vsn-agent.exe')) -and (Test-Path $HklmKey) -and -not (Test-ServiceAbsent)} 300
    Assert-Condition $pmUi.visible 'No visible per-machine NSIS install UI.'
    $pmLifecycle=Exercise-RunningService $MachineRoot 'per-machine'
    $pmAcl=Assert-IpcAclContract $creatorSid 'per-machine'
    $pmProbe=Invoke-LocalServiceProjectDirsProbe 'per-machine'
    $pmSep=Assert-MutableSeparation $pmProbe 'per-machine'
    $pmUninstall=Join-Path $MachineRoot 'uninstall.exe'
    $pmUnProc=Start-UiProcess $pmUninstall
    $pmUnUi=Drive-Ui $pmUnProc 'per-machine-uninstall' {(Test-ServiceAbsent) -and -not (Test-Path $MachineRoot) -and -not (Test-Path $HklmKey)} 300
    Assert-Condition $pmUnUi.visible 'No visible per-machine NSIS uninstall UI.'
    Wait-ServiceState Absent
    Assert-Condition (Test-Path -LiteralPath $IpcKey -PathType Leaf) 'Per-machine uninstall removed IPC key; 03.17 owns preservation.'
    $evidence.per_machine=[ordered]@{setup_sha256=Get-Sha256 $PerMachineSetupPath;visible_install_ui_observed=[bool]$pmUi.visible;visible_uninstall_ui_observed=[bool]$pmUnUi.visible;service=$pmLifecycle;ipc_acl=$pmAcl;localservice_projectdirs=$pmProbe;separation=$pmSep;service_absent_after_uninstall=$true;payload_removed_after_uninstall=$true;ipc_security_persisted_after_uninstall_observed=$true;comprehensive_preservation_certified=$false}

    Reset-RunCreatedSecurityFixture
    $evidence.lane_isolation.security_fixture_reset_before_msi=$true

    $productCode=Get-MsiProperty $MsiPath 'ProductCode'
    $upgradeCode=Get-MsiProperty $MsiPath 'UpgradeCode'
    $msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
    $msiInstallLog=Join-Path $EvidencePath 'msi-install.log'
    $msiInstall=Start-UiProcess $msiexec @('/i',$MsiPath,'/qb!','/norestart','/l*v',$msiInstallLog)
    $msiUi=Observe-ProcessUi $msiInstall 'msi-install' {(Test-Path (Join-Path $MachineRoot 'bin\vsn-agent.exe')) -and -not (Test-ServiceAbsent)} 300
    Assert-Condition ($msiUi.exit_code -eq 0) "MSI install failed: $($msiUi.exit_code)"
    Assert-Condition $msiUi.visible 'No visible MSI basic install UI.'
    $msiLifecycle=Exercise-RunningService $MachineRoot 'msi'
    $msiAcl=Assert-IpcAclContract $creatorSid 'msi'
    $msiProbe=Invoke-LocalServiceProjectDirsProbe 'msi'
    $msiSep=Assert-MutableSeparation $msiProbe 'msi'
    Assert-Condition ($pmProbe.data_local -ieq $msiProbe.data_local) 'NSIS/MSI LocalService data path differs.'
    Assert-Condition ($pmProbe.config_file -ieq $msiProbe.config_file) 'NSIS/MSI LocalService config path differs.'
    $stop=Invoke-Agent (Join-Path $MachineRoot 'bin\vsn-agent.exe') stop 'msi-certification-pre-uninstall'
    Wait-ServiceState Stopped
    $msiUninstallLog=Join-Path $EvidencePath 'msi-uninstall.log'
    $msiUninstall=Start-UiProcess $msiexec @('/x',$productCode,'/qb!','/norestart','/l*v',$msiUninstallLog)
    $msiUnUi=Observe-ProcessUi $msiUninstall 'msi-uninstall' {(Test-ServiceAbsent) -and -not (Test-Path $MachineRoot)} 300
    Assert-Condition ($msiUnUi.exit_code -eq 0) "MSI uninstall failed: $($msiUnUi.exit_code)"
    Assert-Condition $msiUnUi.visible 'No visible MSI basic uninstall UI.'
    Wait-ServiceState Absent
    Assert-Condition (Test-Path -LiteralPath $IpcKey -PathType Leaf) 'MSI uninstall removed IPC key; 03.17 owns preservation.'
    $evidence.msi=[ordered]@{msi_sha256=Get-Sha256 $MsiPath;product_code=$productCode;upgrade_code=$upgradeCode;visible_install_ui_observed=[bool]$msiUi.visible;visible_uninstall_ui_observed=[bool]$msiUnUi.visible;install_exit_code=[int]$msiUi.exit_code;uninstall_exit_code=[int]$msiUnUi.exit_code;install_log='msi-install.log';uninstall_log='msi-uninstall.log';service=$msiLifecycle;ipc_acl=$msiAcl;localservice_projectdirs=$msiProbe;separation=$msiSep;certification_pre_uninstall_stop=$stop;service_absent_after_uninstall=$true;payload_removed_after_uninstall=$true;ipc_security_persisted_after_uninstall_observed=$true;comprehensive_preservation_certified=$false}

    $tracked=@(git status --porcelain=v1 --untracked-files=no)
    if($tracked.Count){$tracked|Write-Host;throw 'Tracked repository drift detected during 03.12.'}
    $evidence.tracked_repository_drift_zero=$true
    Write-Evidence0312 $evidence
} catch {
    $evidence.failure=[ordered]@{message=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')}
    Write-Evidence0312 $evidence
    throw
}
