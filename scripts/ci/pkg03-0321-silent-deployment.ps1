param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir='dist-pkg03/03.21'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

$ProductName='VSN Dev Platform'
$ServiceName='VSN-Agent'
$ServiceDisplayName='VSN Agent'
$ServiceAccount='NT AUTHORITY\LocalService'
$UserRoot=Join-Path $env:LOCALAPPDATA $ProductName
$MachineRoot=Join-Path $env:ProgramFiles $ProductName
$HkcuKey="HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$HklmKey="HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath=(New-Item -ItemType Directory -Force -Path $EvidenceDir).FullName
$MsiInstallLog=Join-Path $EvidencePath 'msi-silent-install.log'
$MsiUninstallLog=Join-Path $EvidencePath 'msi-silent-uninstall.log'
$Operations=[System.Collections.Generic.List[object]]::new()

function Assert-Condition([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Get-Sha256([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()}
function Get-ServiceSnapshot { Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue }
function Test-ServiceAbsent { $null -eq (Get-ServiceSnapshot) }
function Test-ServiceRunning {
  $svc=Get-ServiceSnapshot
  return ($null -ne $svc -and [string]$svc.State -eq 'Running')
}
function Wait-ServiceState([string]$Expected,[int]$TimeoutSeconds=60){
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  while([DateTime]::UtcNow -lt $deadline){
    $s=Get-ServiceSnapshot
    if($Expected -eq 'Absent'){if($null -eq $s){return}}
    elseif($null -ne $s -and [string]$s.State -eq $Expected){return}
    Start-Sleep -Milliseconds 400
  }
  throw "Service $ServiceName did not reach $Expected."
}
function Assert-MachineService([string]$Root,[string]$Lane){
  $svc=Get-ServiceSnapshot
  Assert-Condition ($null -ne $svc) "$Lane service missing."
  $agent=Join-Path $Root 'bin\vsn-agent.exe'
  $escaped=[regex]::Escape($agent)
  Assert-Condition ([string]$svc.DisplayName -eq $ServiceDisplayName) "$Lane service DisplayName mismatch."
  Assert-Condition ([string]$svc.StartMode -eq 'Auto') "$Lane service StartMode mismatch."
  Assert-Condition ([string]$svc.StartName -eq $ServiceAccount) "$Lane service account mismatch."
  Assert-Condition ([string]$svc.PathName -match ('^"?'+$escaped+'"?\s+--service-run$')) "$Lane service PathName mismatch: $($svc.PathName)"
  Assert-Condition ([string]$svc.State -eq 'Running') "$Lane service is not running."
  [pscustomobject][ordered]@{
    name=[string]$svc.Name;display_name=[string]$svc.DisplayName;state=[string]$svc.State
    start_mode=[string]$svc.StartMode;start_name=[string]$svc.StartName;path_name=[string]$svc.PathName
  }
}
function Stop-InstalledService([string]$Root,[string]$Lane){
  $agent=Join-Path $Root 'bin\vsn-agent.exe'
  Assert-Condition (Test-Path -LiteralPath $agent -PathType Leaf) "$Lane Agent payload missing before service stop."
  $output=(& $agent service stop 2>&1 | Out-String).Trim()
  $code=$LASTEXITCODE
  Assert-Condition ($code -eq 0) "$Lane service stop failed: exit=$code output=$output"
  Wait-ServiceState Stopped
  [pscustomobject][ordered]@{exit_code=[int]$code;output=$output}
}
function Invoke-InstalledPing([string]$Root,[string]$Lane){
  $cli=Join-Path $Root 'bin\vsn.exe'
  Assert-Condition (Test-Path -LiteralPath $cli -PathType Leaf) "$Lane CLI payload missing."
  $output=(& $cli ping 2>&1 | Out-String).Trim()
  $code=$LASTEXITCODE
  Assert-Condition ($code -eq 0) "$Lane installed CLI ping failed: exit=$code output=$output"
  [pscustomobject][ordered]@{exit_code=[int]$code;output=$output}
}
function Get-MsiProperty([string]$Path,[string]$Property){
  $installer=New-Object -ComObject WindowsInstaller.Installer
  $db=$installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
  $view=$db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
  $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null)|Out-Null
  $record=$view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null)
  if($null -eq $record){throw "MSI property '$Property' not found."}
  [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}
function Test-MsiArp([string]$ProductCode){
  Test-Path -LiteralPath "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductCode"
}
function Clear-StaleCurrentUserInstallerLocationMetadata {
  $subKey='Software\Vertex Systems Network\VSN Dev Platform'
  $key=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($subKey,$true)
  if($null -eq $key){return @()}
  $removed=@()
  try{
    $expected=$UserRoot.TrimEnd([char]'\')
    foreach($name in @('', 'InstallDir')){
      $raw=$key.GetValue($name,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
      if($null -eq $raw){continue}
      $path=([string]$raw).Trim()
      if([string]::IsNullOrWhiteSpace($path)){continue}
      if($path.TrimEnd([char]'\') -ine $expected){continue}
      if(Test-Path -LiteralPath $path){continue}
      $key.DeleteValue($name,$false)
      $removed += [pscustomobject][ordered]@{name=if([string]::IsNullOrEmpty($name)){'(Default)'}else{$name};stale_path=$path}
    }
  }finally{$key.Close()}
  @($removed)
}
function Update-ProcessFamily([System.Collections.Generic.HashSet[int]]$Family){
  $snapshot=@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId,ParentProcessId)
  do{
    $changed=$false
    foreach($proc in $snapshot){
      $pidNow=[int]$proc.ProcessId;$parent=[int]$proc.ParentProcessId
      if($Family.Contains($parent)-and -not $Family.Contains($pidNow)){
        [void]$Family.Add($pidNow);$changed=$true
      }
    }
  }while($changed)
}
function Get-VisibleFamilyWindows([System.Collections.Generic.HashSet[int]]$Family){
  $rows=@()
  foreach($id in @($Family)){
    try{
      $p=Get-Process -Id $id -ErrorAction Stop
      if($p.MainWindowHandle -ne 0 -and -not [string]::IsNullOrWhiteSpace([string]$p.MainWindowTitle)){
        $rows += [pscustomobject][ordered]@{pid=$id;title=[string]$p.MainWindowTitle;process_name=[string]$p.ProcessName}
      }
    }catch{}
  }
  @($rows)
}
function Get-AliveFamily([System.Collections.Generic.HashSet[int]]$Family){
  $alive=@()
  foreach($id in @($Family)){try{Get-Process -Id $id -ErrorAction Stop|Out-Null;$alive+=$id}catch{}}
  @($alive)
}
function Invoke-BoundedSilentOperation {
  param(
    [string]$Label,
    [string]$FilePath,
    [string[]]$Arguments,
    [scriptblock]$CompletionTest,
    [int[]]$AcceptedExitCodes,
    [int]$TimeoutSeconds=300
  )
  $psi=[System.Diagnostics.ProcessStartInfo]::new()
  $psi.FileName=$FilePath
  $psi.UseShellExecute=$false
  foreach($arg in $Arguments){[void]$psi.ArgumentList.Add($arg)}
  $process=[System.Diagnostics.Process]::new();$process.StartInfo=$psi
  if(-not $process.Start()){throw "$Label failed to start."}
  $family=[System.Collections.Generic.HashSet[int]]::new();[void]$family.Add([int]$process.Id)
  $visible=[System.Collections.Generic.List[object]]::new()
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $completePolls=0
  while([DateTime]::UtcNow -lt $deadline){
    Update-ProcessFamily $family
    foreach($row in @(Get-VisibleFamilyWindows $family)){
      if(-not @($visible|Where-Object{$_.pid -eq $row.pid -and $_.title -eq $row.title}).Count){
        [void]$visible.Add($row)
      }
    }
    $complete=[bool](& $CompletionTest)
    $alive=@(Get-AliveFamily $family)
    if($complete -and $alive.Count -eq 0){$completePolls++;if($completePolls -ge 2){break}}else{$completePolls=0}
    Start-Sleep -Milliseconds 350
  }
  $complete=[bool](& $CompletionTest)
  $alive=@(Get-AliveFamily $family)
  if(-not $complete -or $alive.Count -ne 0){
    foreach($id in $alive){Stop-Process -Id $id -Force -ErrorAction SilentlyContinue}
    throw "$Label timed out or failed required state: complete=$complete alive=$($alive -join ',')"
  }
  $process.WaitForExit()
  $code=[int]$process.ExitCode
  Assert-Condition ($code -in $AcceptedExitCodes) "$Label native exit $code not in accepted set $($AcceptedExitCodes -join ',')."
  Assert-Condition ($code -ne 1641) "$Label initiated reboot (1641), forbidden."
  Assert-Condition ($visible.Count -eq 0) "$Label exposed visible titled installer-family window(s): $(@($visible|ForEach-Object{"$($_.process_name):$($_.title)"}) -join '; ')"
  $record=[pscustomobject][ordered]@{
    label=$Label;file=$FilePath;arguments=@($Arguments);root_pid=[int]$process.Id
    family_pids=@($family);exit_code=$code;bounded_completion=$true
    zero_ui_or_input_events_sent=$true;visible_titled_windows=@($visible)
  }
  [void]$Operations.Add($record)
  $record
}

$actual=(git rev-parse HEAD).Trim()
Assert-Condition ($actual -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actual"
$CurrentUserNsisPath=(Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
$PerMachineNsisPath=(Resolve-Path -LiteralPath $PerMachineNsisPath).Path
$MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
foreach($path in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)){
  Assert-Condition ((Get-Item -LiteralPath $path).Length -gt 0) "Installer is empty: $path"
}
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
$msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
$evidence=[ordered]@{
  schema_version=1;package_id='PKG-03';task_id='03.21';source_commit=$SourceSha
  current_user=$null;lane_isolation=$null;per_machine=$null;msi=$null
  zero_input_contract=[ordered]@{
    ui_automation_events_sent=0;keyboard_events_sent=0;mouse_events_sent=0
    stdin_prompt_answers_sent=0;visible_installer_family_titled_window_observed=$false
    passive_mode_certified=$false
  }
  packages=[ordered]@{
    current_user_nsis_sha256=Get-Sha256 $CurrentUserNsisPath
    per_machine_nsis_sha256=Get-Sha256 $PerMachineNsisPath
    msi_sha256=Get-Sha256 $MsiPath
    product_code=$productCode
  }
  product_or_installer_mutation=$false;signing_claimed=$false;provenance_claimed=$false;updater_mutation_claimed=$false
  tracked_repository_drift_zero=$false
}

try{
  Assert-Condition (Test-ServiceAbsent) 'VSN-Agent unexpectedly exists before 03.21.'
  Assert-Condition (-not (Test-Path $UserRoot)) 'Current-user root exists before 03.21.'
  Assert-Condition (-not (Test-Path $MachineRoot)) 'Machine root exists before 03.21.'
  Assert-Condition (-not (Test-Path $HkcuKey)) 'HKCU ARP exists before 03.21.'
  Assert-Condition (-not (Test-Path $HklmKey)) 'HKLM NSIS ARP exists before 03.21.'
  Assert-Condition (-not (Test-MsiArp $productCode)) 'MSI ProductCode ARP exists before 03.21.'

  # Current-user NSIS: exact uppercase /S, no input, machine service must stay absent.
  $cuInstall=Invoke-BoundedSilentOperation -Label 'current-user-nsis-silent-install' -FilePath $CurrentUserNsisPath -Arguments @('/S') `
    -CompletionTest { (Test-Path (Join-Path $UserRoot 'VSN Dev Platform.exe')) -and (Test-Path (Join-Path $UserRoot 'bin\vsn.exe')) -and (Test-Path (Join-Path $UserRoot 'bin\vsn-agent.exe')) -and (Test-Path $HkcuKey) } `
    -AcceptedExitCodes @(0) -TimeoutSeconds 300
  Assert-Condition (Test-ServiceAbsent) 'Current-user silent NSIS created machine service.'
  Assert-Condition (-not (Test-Path $HklmKey)) 'Current-user silent NSIS created HKLM NSIS ARP.'
  $cuUninstaller=Join-Path $UserRoot 'uninstall.exe';Assert-Condition (Test-Path $cuUninstaller -PathType Leaf) 'Current-user silent uninstaller missing.'
  $cuUninstall=Invoke-BoundedSilentOperation -Label 'current-user-nsis-silent-uninstall' -FilePath $cuUninstaller -Arguments @('/S') `
    -CompletionTest { -not (Test-Path $UserRoot) -and -not (Test-Path $HkcuKey) } -AcceptedExitCodes @(0) -TimeoutSeconds 300
  Assert-Condition (Test-ServiceAbsent) 'Current-user silent uninstall mutated machine service.'
  $staleRemoved=@(Clear-StaleCurrentUserInstallerLocationMetadata)
  $evidence.current_user=[ordered]@{
    install=$cuInstall;uninstall=$cuUninstall;service_absent_after_install=$true;service_absent_after_uninstall=$true
    hkcu_registration_removed=$true;machine_scope_mutation_observed=$false
  }
  $evidence.lane_isolation=[ordered]@{
    removed_only_stale_current_user_installer_location_values=$staleRemoved
    product_registry_key_deleted=$false;user_data_deleted=$false
  }

  # Per-machine NSIS: preserve accepted machine service identity/health, then stop it before silent uninstall.
  $pmInstall=Invoke-BoundedSilentOperation -Label 'per-machine-nsis-silent-install' -FilePath $PerMachineNsisPath -Arguments @('/S') `
    -CompletionTest { (Test-Path (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-Path (Join-Path $MachineRoot 'bin\vsn.exe')) -and (Test-Path (Join-Path $MachineRoot 'bin\vsn-agent.exe')) -and (Test-Path $HklmKey) -and (Test-ServiceRunning) } `
    -AcceptedExitCodes @(0) -TimeoutSeconds 360
  $pmService=Assert-MachineService $MachineRoot 'per-machine-nsis'
  $pmPing=Invoke-InstalledPing $MachineRoot 'per-machine-nsis'
  $pmStop=Stop-InstalledService $MachineRoot 'per-machine-nsis'
  $pmUninstaller=Join-Path $MachineRoot 'uninstall.exe';Assert-Condition (Test-Path $pmUninstaller -PathType Leaf) 'Per-machine silent uninstaller missing.'
  $pmUninstall=Invoke-BoundedSilentOperation -Label 'per-machine-nsis-silent-uninstall' -FilePath $pmUninstaller -Arguments @('/S') `
    -CompletionTest { (Test-ServiceAbsent) -and -not (Test-Path $MachineRoot) -and -not (Test-Path $HklmKey) } -AcceptedExitCodes @(0) -TimeoutSeconds 360
  Wait-ServiceState Absent
  Assert-Condition (-not (Test-Path $HkcuKey)) 'Per-machine silent NSIS created HKCU ARP.'
  $evidence.per_machine=[ordered]@{
    install=$pmInstall;service=$pmService;health=$pmPing;pre_uninstall_stop=$pmStop;uninstall=$pmUninstall
    service_absent_after_uninstall=$true;payload_removed_after_uninstall=$true
    running_resource_uninstall_recertified=$false;running_resource_owner='03.19'
  }

  # MSI/WiX strict silent mode. /quiet is the public equivalent of /qn; /norestart is mandatory.
  $msiInstallArgs=@('/i',$MsiPath,'/quiet','/norestart','/L*V',$MsiInstallLog)
  $msiInstall=Invoke-BoundedSilentOperation -Label 'msi-quiet-install' -FilePath $msiexec -Arguments $msiInstallArgs `
    -CompletionTest { (Test-Path (Join-Path $MachineRoot 'VSN Dev Platform.exe')) -and (Test-MsiArp $productCode) -and (Test-ServiceRunning) } `
    -AcceptedExitCodes @(0,3010) -TimeoutSeconds 360
  $msiService=Assert-MachineService $MachineRoot 'msi'
  $msiPing=Invoke-InstalledPing $MachineRoot 'msi'
  $msiStop=Stop-InstalledService $MachineRoot 'msi'
  $msiUninstallArgs=@('/x',$productCode,'/quiet','/norestart','/L*V',$MsiUninstallLog)
  $msiUninstall=Invoke-BoundedSilentOperation -Label 'msi-quiet-uninstall' -FilePath $msiexec -Arguments $msiUninstallArgs `
    -CompletionTest { (Test-ServiceAbsent) -and -not (Test-Path $MachineRoot) -and -not (Test-MsiArp $productCode) } `
    -AcceptedExitCodes @(0,3010) -TimeoutSeconds 360
  Wait-ServiceState Absent
  Assert-Condition (Test-Path $MsiInstallLog -PathType Leaf) 'MSI silent install log missing.'
  Assert-Condition (Test-Path $MsiUninstallLog -PathType Leaf) 'MSI silent uninstall log missing.'
  $installText=Get-Content -Raw -LiteralPath $MsiInstallLog
  $uninstallText=Get-Content -Raw -LiteralPath $MsiUninstallLog
  $installReally=($installText -match '(?im)\bREBOOT\b[^\r\n]*ReallySuppress')
  $uninstallReally=($uninstallText -match '(?im)\bREBOOT\b[^\r\n]*ReallySuppress')
  Assert-Condition $installReally 'MSI silent install log did not prove REBOOT=ReallySuppress.'
  Assert-Condition $uninstallReally 'MSI silent uninstall log did not prove REBOOT=ReallySuppress.'
  $evidence.msi=[ordered]@{
    install=$msiInstall;service=$msiService;health=$msiPing;pre_uninstall_stop=$msiStop;uninstall=$msiUninstall
    public_silent_switch='/quiet';equivalent_no_ui_switch='/qn';norestart_required=$true
    install_really_suppress_observed=$installReally;uninstall_really_suppress_observed=$uninstallReally
    install_log=[ordered]@{path=$MsiInstallLog;size_bytes=(Get-Item $MsiInstallLog).Length;sha256=Get-Sha256 $MsiInstallLog}
    uninstall_log=[ordered]@{path=$MsiUninstallLog;size_bytes=(Get-Item $MsiUninstallLog).Length;sha256=Get-Sha256 $MsiUninstallLog}
    running_resource_uninstall_recertified=$false;running_resource_owner='03.19';reboot_semantics_owner='03.20'
  }

  foreach($op in @($Operations)){Assert-Condition (@($op.visible_titled_windows).Count -eq 0) "Visible titled window evidence widened for $($op.label)."}
  $tracked=@(git status --porcelain=v1 --untracked-files=no)
  if($tracked.Count -ne 0){$tracked|Write-Host;throw 'Tracked repository drift detected during 03.21.'}
  $evidence.operations=@($Operations)
  $evidence.tracked_repository_drift_zero=$true
  $evidencePath=Join-Path $EvidencePath 'evidence.json'
  $evidence|ConvertTo-Json -Depth 16|Set-Content -LiteralPath $evidencePath -Encoding utf8NoBOM
  $digest=Get-Sha256 $evidencePath
  "$digest  evidence.json"|Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM
  $evidence|ConvertTo-Json -Depth 16
}catch{
  $evidence.failure=[ordered]@{message=$_.Exception.Message;at_utc=[DateTime]::UtcNow.ToString('o')}
  $evidence.operations=@($Operations)
  $evidence|ConvertTo-Json -Depth 16|Set-Content -LiteralPath (Join-Path $EvidencePath 'failure-evidence.json') -Encoding utf8NoBOM
  throw
}finally{
  try{
    if(-not (Test-ServiceAbsent)){
      $agent=Join-Path $MachineRoot 'bin\vsn-agent.exe'
      if(Test-Path $agent){& $agent service stop 2>&1|Out-Null}
      else{& (Join-Path $env:SystemRoot 'System32\sc.exe') stop $ServiceName 2>&1|Out-Null}
      Start-Sleep -Seconds 1
    }
  }catch{}
  try{
    $cu=Join-Path $UserRoot 'uninstall.exe'
    if(Test-Path $cu){Start-Process -FilePath $cu -ArgumentList '/S' -Wait -ErrorAction SilentlyContinue|Out-Null}
  }catch{}
  try{
    $pm=Join-Path $MachineRoot 'uninstall.exe'
    if(Test-Path $pm){Start-Process -FilePath $pm -ArgumentList '/S' -Wait -ErrorAction SilentlyContinue|Out-Null}
  }catch{}
  try{
    if(Test-MsiArp $productCode){Start-Process -FilePath $msiexec -ArgumentList @('/x',$productCode,'/qn','/norestart') -Wait -ErrorAction SilentlyContinue|Out-Null}
  }catch{}
}
