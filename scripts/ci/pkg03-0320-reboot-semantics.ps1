param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir='dist-pkg03/03.20'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

function Assert-Condition([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Get-Sha256([string]$Path){return (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()}
function Get-BootIdentity {
  $os=Get-CimInstance Win32_OperatingSystem -ErrorAction Stop
  return ([datetime]$os.LastBootUpTime).ToUniversalTime().ToString('o')
}
function Get-PendingSnapshot {
  param([string]$RegistryPath,[string]$PropertyName)
  $exists=$false;$kind=$null;$values=@()
  try {
    $key=Get-Item -LiteralPath $RegistryPath -ErrorAction Stop
    $names=@($key.GetValueNames())
    if($names -contains $PropertyName){
      $exists=$true
      $kind=[string]$key.GetValueKind($PropertyName)
      $raw=$key.GetValue($PropertyName,$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
      if($null -ne $raw){$values=@($raw|ForEach-Object{[string]$_})}
    }
  } catch { throw "Unable to read pending-reboot registry state: $($_.Exception.Message)" }
  return [pscustomobject][ordered]@{exists=$exists;kind=$kind;values=@($values)}
}
function Restore-PendingSnapshot {
  param([string]$RegistryPath,[string]$PropertyName,[object]$Snapshot)
  if($Snapshot.exists){
    New-ItemProperty -LiteralPath $RegistryPath -Name $PropertyName -PropertyType MultiString -Value @($Snapshot.values) -Force | Out-Null
  }else{
    Remove-ItemProperty -LiteralPath $RegistryPath -Name $PropertyName -ErrorAction SilentlyContinue
  }
}
function Assert-PendingSnapshotEqual {
  param([object]$Expected,[object]$Actual,[string]$Label)
  Assert-Condition ($Expected.exists -eq $Actual.exists) "$Label pending-value existence mismatch."
  Assert-Condition ([string]$Expected.kind -eq [string]$Actual.kind) "$Label pending-value kind mismatch."
  $a=@($Expected.values);$b=@($Actual.values)
  Assert-Condition ($a.Count -eq $b.Count) "$Label pending-value count mismatch."
  for($i=0;$i -lt $a.Count;$i++){Assert-Condition ([string]$a[$i] -ceq [string]$b[$i]) "$Label pending-value order/content mismatch at $i."}
}
function Assert-PendingPrefixPreserved {
  param([string[]]$ExpectedPrefix,[object]$Actual,[string]$Label)
  Assert-Condition $Actual.exists "$Label pending-value disappeared."
  Assert-Condition ([string]$Actual.kind -eq 'MultiString') "$Label pending-value kind changed."
  $values=@($Actual.values)
  Assert-Condition ($values.Count -ge $ExpectedPrefix.Count) "$Label pending-value shortened below protected prefix."
  for($i=0;$i -lt $ExpectedPrefix.Count;$i++){Assert-Condition ([string]$values[$i] -ceq [string]$ExpectedPrefix[$i]) "$Label pending prefix changed at $i."}
}
function Get-MsiProperty([string]$Path,[string]$Property){
  $installer=New-Object -ComObject WindowsInstaller.Installer
  $db=$installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
  $view=$db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
  $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null)|Out-Null
  $record=$view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null)
  if($null -eq $record){throw "MSI property '$Property' not found."}
  return [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}
function Invoke-Msi {
  param([string[]]$Arguments,[string]$Label)
  $p=Start-Process -FilePath (Join-Path $env:SystemRoot 'System32/msiexec.exe') -ArgumentList $Arguments -Wait -PassThru
  $code=[int]$p.ExitCode
  Assert-Condition ($code -ne 1641) "$Label initiated a reboot (1641), forbidden by 03.20."
  Assert-Condition ($code -in @(0,3010)) "$Label returned unexpected MSI exit code $code."
  return $code
}

$actual=(git rev-parse HEAD).Trim();Assert-Condition ($actual -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actual"
$CurrentUserNsisPath=(Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
$PerMachineNsisPath=(Resolve-Path -LiteralPath $PerMachineNsisPath).Path
$MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
foreach($p in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)){Assert-Condition ((Get-Item -LiteralPath $p).Length -gt 0) "Package is empty: $p"}

$out=(New-Item -ItemType Directory -Force -Path $EvidenceDir).FullName
$repoRoot=(Get-Location).Path
$baselineDir=Join-Path $out 'baseline-0319'
$baselineInvokeDir=[IO.Path]::GetRelativePath($repoRoot,$baselineDir)
Assert-Condition (-not [IO.Path]::IsPathRooted($baselineInvokeDir)) 'Inherited 03.19 evidence path must remain repo-relative.'
Assert-Condition ($baselineInvokeDir -notmatch '^\.\.(?:[\\/]|$)') 'Inherited 03.19 evidence path escaped repository root.'
$logsDir=Join-Path $out 'logs';New-Item -ItemType Directory -Force $logsDir|Out-Null
$pendingPath='HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager'
$pendingName='PendingFileRenameOperations'
$original=Get-PendingSnapshot $pendingPath $pendingName
Assert-Condition ((-not $original.exists) -or ([string]$original.kind -eq 'MultiString')) "Pre-existing PendingFileRenameOperations is not MultiString."
$bootBefore=Get-BootIdentity
$probeRoot=Join-Path $env:RUNNER_TEMP ('vsn-pkg03-0320-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force $probeRoot|Out-Null
$probeSource=Join-Path $probeRoot 'pending-source.tmp';$probeDestination=Join-Path $probeRoot 'pending-destination.tmp'
[IO.File]::WriteAllText($probeSource,'PKG-03 03.20 deterministic pending rename probe')
$probePair=@("\??\$probeSource","\??\$probeDestination")
$protectedPrefix=@($original.values)+$probePair
$cleanupRestored=$false
$baselineEvidence=$null
$installCode=$null;$uninstallCode=$null
$installLog=Join-Path $logsDir 'msi-norestart-install.log';$uninstallLog=Join-Path $logsDir 'msi-norestart-uninstall.log'
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
$installReally=$false;$installPending=$false;$uninstallReally=$false;$uninstallPending=$false

try {
  New-ItemProperty -LiteralPath $pendingPath -Name $pendingName -PropertyType MultiString -Value $protectedPrefix -Force | Out-Null
  Assert-PendingPrefixPreserved $protectedPrefix (Get-PendingSnapshot $pendingPath $pendingName) 'after probe injection'
  Assert-Condition ((Get-BootIdentity) -eq $bootBefore) 'Boot session changed while injecting pending-reboot signal.'

  & scripts/ci/pkg03-0319-running-processes.ps1 `
    -CurrentUserNsisPath $CurrentUserNsisPath `
    -PerMachineNsisPath $PerMachineNsisPath `
    -MsiPath $MsiPath `
    -SourceSha $SourceSha `
    -EvidenceDir $baselineInvokeDir
  if($LASTEXITCODE -ne 0){throw "Inherited 03.19 lifecycle returned native exit $LASTEXITCODE"}
  $baselineEvidence=Get-Content -Raw -LiteralPath (Join-Path $baselineDir 'evidence.json')|ConvertFrom-Json
  Assert-Condition ([string]$baselineEvidence.source_commit -eq $SourceSha) 'Inherited 03.19 source binding mismatch.'
  Assert-Condition ($baselineEvidence.harness_pre_kill -eq $false) 'Inherited 03.19 pre-kill widened.'
  Assert-Condition ($baselineEvidence.tracked_repository_drift_zero -eq $true) 'Inherited 03.19 tracked drift is nonzero.'
  Assert-Condition (@($baselineEvidence.lifecycles).Count -eq 3) 'Inherited 03.19 lifecycle count mismatch.'
  foreach($row in @($baselineEvidence.lifecycles)){
    Assert-Condition ($row.protected_state_equal -eq $true) "Inherited 03.19 $($row.lifecycle) protected state mismatch."
    Assert-Condition ($row.operation.outcome -in @('coordinated_completion','deterministic_safe_block')) "Inherited 03.19 $($row.lifecycle) outcome invalid."
  }
  Assert-PendingPrefixPreserved $protectedPrefix (Get-PendingSnapshot $pendingPath $pendingName) 'after inherited 03.19 lifecycle'
  Assert-Condition ((Get-BootIdentity) -eq $bootBefore) 'Boot session changed during inherited 03.19 lifecycle.'

  $installCode=Invoke-Msi -Arguments @('/i',"`"$MsiPath`"",'/qn','/norestart','/L*V',"`"$installLog`"") -Label 'MSI /norestart install'
  Assert-Condition (Test-Path -LiteralPath $installLog) 'MSI /norestart install log missing.'
  $installText=Get-Content -Raw -LiteralPath $installLog
  $installReally=($installText -match '(?im)\bREBOOT\b[^\r\n]*ReallySuppress')
  $installPending=($installText -match '(?im)\bMsiSystemRebootPending\b[^\r\n]*(?:=|value is\s+''?)\s*1\b')
  Assert-Condition $installReally 'MSI install log did not prove REBOOT=ReallySuppress semantics.'
  Assert-Condition $installPending 'MSI install log did not prove MsiSystemRebootPending=1.'
  Assert-PendingPrefixPreserved $protectedPrefix (Get-PendingSnapshot $pendingPath $pendingName) 'after MSI /norestart install'
  Assert-Condition ((Get-BootIdentity) -eq $bootBefore) 'Boot session changed after MSI /norestart install.'

  $uninstallCode=Invoke-Msi -Arguments @('/x',$productCode,'/qn','/norestart','/L*V',"`"$uninstallLog`"") -Label 'MSI /norestart uninstall'
  Assert-Condition (Test-Path -LiteralPath $uninstallLog) 'MSI /norestart uninstall log missing.'
  $uninstallText=Get-Content -Raw -LiteralPath $uninstallLog
  $uninstallReally=($uninstallText -match '(?im)\bREBOOT\b[^\r\n]*ReallySuppress')
  $uninstallPending=($uninstallText -match '(?im)\bMsiSystemRebootPending\b[^\r\n]*(?:=|value is\s+''?)\s*1\b')
  Assert-Condition $uninstallReally 'MSI uninstall log did not prove REBOOT=ReallySuppress semantics.'
  Assert-Condition $uninstallPending 'MSI uninstall log did not prove MsiSystemRebootPending=1.'
  Assert-PendingPrefixPreserved $protectedPrefix (Get-PendingSnapshot $pendingPath $pendingName) 'after MSI /norestart uninstall'
  Assert-Condition ((Get-BootIdentity) -eq $bootBefore) 'Boot session changed after MSI /norestart uninstall.'
} finally {
  try {
    $arp="HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productCode"
    if(Test-Path -LiteralPath $arp){
      $p=Start-Process -FilePath (Join-Path $env:SystemRoot 'System32/msiexec.exe') -ArgumentList @('/x',$productCode,'/qn','/norestart') -Wait -PassThru
      if([int]$p.ExitCode -notin @(0,1605,1614,3010)){Write-Host "Emergency MSI cleanup exit=$($p.ExitCode)"}
    }
  } catch { Write-Host "Emergency MSI cleanup failed: $($_.Exception.Message)" }
  Restore-PendingSnapshot $pendingPath $pendingName $original
  $restored=Get-PendingSnapshot $pendingPath $pendingName
  Assert-PendingSnapshotEqual $original $restored 'final cleanup'
  $cleanupRestored=$true
  Remove-Item -LiteralPath $probeSource,$probeDestination -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $probeRoot -Recurse -Force -ErrorAction SilentlyContinue
}

$bootAfter=Get-BootIdentity
Assert-Condition ($bootAfter -eq $bootBefore) 'Boot-session identity changed during 03.20.'
Assert-Condition $cleanupRestored 'Pending-reboot registry state was not restored.'
$baselineFile=Join-Path $baselineDir 'evidence.json'
$baselineHash=Get-Sha256 $baselineFile
$installLogHash=Get-Sha256 $installLog;$uninstallLogHash=Get-Sha256 $uninstallLog
$tracked=@(git status --porcelain=v1 --untracked-files=no);if($tracked.Count -ne 0){$tracked|Write-Host;throw 'Tracked repository drift detected during 03.20.'}

$evidence=[ordered]@{
  schema_version=1
  package_id='PKG-03'
  task_id='03.20'
  source_commit=$SourceSha
  boot=[ordered]@{before=$bootBefore;after=$bootAfter;identity_equal=$true;restart_initiated=$false}
  pending_reboot=[ordered]@{
    signal='PendingFileRenameOperations'
    original_exists=[bool]$original.exists
    original_kind=$original.kind
    original_values=@($original.values)
    injected_pair=@($probePair)
    protected_prefix_preserved=$true
    msi_system_reboot_pending_observed=($installPending -and $uninstallPending)
    universal_reboot_detector_claimed=$false
    exact_original_state_restored=$true
  }
  exit_contract=[ordered]@{accepted_success_codes=@(0,3010);reboot_initiated_code_forbidden=1641;reboot_initiated_observed=$false}
  msi_norestart=[ordered]@{
    arguments=@('/qn','/norestart','/L*V')
    reboot_property='ReallySuppress'
    install_exit_code=$installCode
    uninstall_exit_code=$uninstallCode
    install_really_suppress_observed=$installReally
    uninstall_really_suppress_observed=$uninstallReally
    install_msi_system_reboot_pending_observed=$installPending
    uninstall_msi_system_reboot_pending_observed=$uninstallPending
    install_log=[ordered]@{path=$installLog;size_bytes=(Get-Item $installLog).Length;sha256=$installLogHash}
    uninstall_log=[ordered]@{path=$uninstallLog;size_bytes=(Get-Item $uninstallLog).Length;sha256=$uninstallLogHash}
    quiet_control_plane_used=$true
    silent_deployment_acceptance_claimed=$false
  }
  inherited_0319=[ordered]@{
    evidence_path=$baselineFile
    evidence_sha256=$baselineHash
    source_commit=[string]$baselineEvidence.source_commit
    lifecycle_count=@($baselineEvidence.lifecycles).Count
    harness_pre_kill=[bool]$baselineEvidence.harness_pre_kill
    tracked_repository_drift_zero=[bool]$baselineEvidence.tracked_repository_drift_zero
  }
  packages=[ordered]@{
    current_user_nsis_sha256=Get-Sha256 $CurrentUserNsisPath
    per_machine_nsis_sha256=Get-Sha256 $PerMachineNsisPath
    msi_sha256=Get-Sha256 $MsiPath
    product_code=$productCode
  }
  cleanup=[ordered]@{pending_registry_restored=$true;probe_files_removed=$true}
  product_or_installer_mutation=$false
  silent_deployment_acceptance_claimed=$false
  signing_claimed=$false
  provenance_claimed=$false
  updater_mutation_claimed=$false
  tracked_repository_drift_zero=$true
}
$evidencePath=Join-Path $out 'evidence.json';$evidence|ConvertTo-Json -Depth 14|Set-Content -LiteralPath $evidencePath -Encoding utf8NoBOM
$digest=Get-Sha256 $evidencePath;"$digest  evidence.json"|Set-Content -LiteralPath (Join-Path $out 'evidence.json.sha256') -Encoding utf8NoBOM
$evidence|ConvertTo-Json -Depth 14
