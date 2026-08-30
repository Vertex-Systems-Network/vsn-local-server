param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.18'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Reuse the canonically accepted 03.15 UI/exit-code primitives, but only the
# helper section. 03.18 owns the failure/interruption semantics below.
$helperSource = (& git show "main:scripts/ci/pkg03-0315-installer-diagnostics.ps1" | Out-String).Replace("`r`n","`n")
$helperStart = $helperSource.IndexOf('Set-StrictMode -Version Latest')
$helperEnd = $helperSource.IndexOf('New-Item -ItemType Directory -Force $EvidencePath | Out-Null', $helperStart)
if ($helperStart -lt 0 -or $helperEnd -le $helperStart) { throw 'Unable to locate accepted 03.15 helper boundary.' }
Invoke-Expression $helperSource.Substring($helperStart, $helperEnd - $helperStart)

. (Join-Path $PSScriptRoot 'pkg03-0313-snapshot.ps1')

$SecurityDir = Join-Path $env:ProgramData 'VSN\security'
$StartMenuUser = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$StartMenuCommon = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs'
$ServiceName = 'VSN-Agent'
$ExpectedOwned = @('VSN Dev Platform.exe','bin\vsn.exe','bin\vsn-agent.exe')
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$SnapshotsPath = Join-Path $EvidencePath 'snapshots'
$FailureLogsPath = Join-Path $EvidencePath 'failure-logs'
New-Item -ItemType Directory -Force $EvidencePath,$SnapshotsPath,$FailureLogsPath | Out-Null
Write-UiEvidence

function Test-ServiceAbsent {
  return $null -eq (Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue)
}

function Get-ServiceEvidence {
  $svc = Get-CimInstance Win32_Service -Filter "Name='$ServiceName'" -ErrorAction SilentlyContinue
  if ($null -eq $svc) { return $null }
  return [pscustomobject][ordered]@{
    name=[string]$svc.Name
    display_name=[string]$svc.DisplayName
    start_name=[string]$svc.StartName
    start_mode=[string]$svc.StartMode
    state=[string]$svc.State
    path_name=[string]$svc.PathName
  }
}

function Get-InstallInventory([string]$Root,[string]$ArpPath,[string]$Label) {
  $entries = @()
  if (Test-Path -LiteralPath $Root) {
    foreach ($item in @(Get-ChildItem -LiteralPath $Root -Force -Recurse -ErrorAction SilentlyContinue)) {
      $relative = $item.FullName.Substring($Root.TrimEnd('\').Length).TrimStart('\')
      $entries += [pscustomobject][ordered]@{relative=$relative;kind=$(if($item.PSIsContainer){'directory'}else{'file'});size=$(if($item.PSIsContainer){0}else{[long]$item.Length})}
    }
  }
  return [pscustomobject][ordered]@{
    label=$Label
    root=$Root
    root_exists=(Test-Path -LiteralPath $Root)
    entries=$entries
    arp_path=$ArpPath
    arp_exists=(Test-Path -LiteralPath $ArpPath)
    service=Get-ServiceEvidence
    security_dir_exists=(Test-Path -LiteralPath $SecurityDir)
  }
}

function Assert-PreflightClean([string]$Root,[string]$ArpPath,[string]$Label) {
  Assert-Condition (-not (Test-Path -LiteralPath $Root)) "$Label install root already exists."
  Assert-Condition (-not (Test-Path -LiteralPath $ArpPath)) "$Label ARP state already exists."
  Assert-Condition (Test-ServiceAbsent) "$Label service already exists."
  Assert-Condition (-not (Test-Path -LiteralPath $SecurityDir)) "$Label machine security state already exists."
}

function New-FailureCollision([string]$Root) {
  New-Item -ItemType Directory -Force $Root | Out-Null
  $collision = Join-Path $Root 'VSN Dev Platform.exe'
  New-Item -ItemType Directory -Force $collision | Out-Null
  $marker = Join-Path $collision '03.18-owned-sentinel.txt'
  [IO.File]::WriteAllText($marker, "03.18 collision sentinel`n", [Text.UTF8Encoding]::new($false))
  return [pscustomobject][ordered]@{root=$Root;collision=$collision;marker=$marker;marker_sha256=Get-Sha256 $marker}
}

function Assert-FailureContainsOnlyProbe([object]$Probe,[string]$ArpPath,[string]$Label) {
  Assert-Condition (Test-Path -LiteralPath $Probe.marker -PathType Leaf) "$Label failure probe marker was removed."
  Assert-Condition ((Get-Sha256 $Probe.marker) -eq $Probe.marker_sha256) "$Label failure probe marker changed."
  $all = @(Get-ChildItem -LiteralPath $Probe.root -Force -Recurse -ErrorAction SilentlyContinue)
  $allowed = @(
    [IO.Path]::GetFullPath($Probe.collision).TrimEnd('\'),
    [IO.Path]::GetFullPath($Probe.marker)
  )
  $unexpected = @()
  foreach ($item in $all) {
    $full = [IO.Path]::GetFullPath($item.FullName).TrimEnd('\')
    if ($full -notin $allowed) { $unexpected += $full }
  }
  Assert-Condition ($unexpected.Count -eq 0) "$Label left unauthorized partial install state: $($unexpected -join ', ')"
  Assert-Condition (-not (Test-Path -LiteralPath $ArpPath)) "$Label left ARP registration after failed install."
  Assert-Condition (Test-ServiceAbsent) "$Label left VSN-Agent after failed install."
  Assert-Condition (-not (Test-Path -LiteralPath $SecurityDir)) "$Label left machine security state after failed install."
  return Get-InstallInventory $Probe.root $ArpPath "$Label-post-failure"
}

function Remove-FailureProbe([object]$Probe) {
  Assert-Condition (Test-Path -LiteralPath $Probe.marker -PathType Leaf) 'Cannot remove missing failure probe marker.'
  Remove-Item -LiteralPath $Probe.marker -Force
  Remove-Item -LiteralPath $Probe.collision -Force
  Remove-Item -LiteralPath $Probe.root -Force
}

function Get-ProcessFamily([int]$RootPid) {
  $snapshot = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Select-Object ProcessId,ParentProcessId)
  $family = [System.Collections.Generic.HashSet[int]]::new(); [void]$family.Add($RootPid)
  do {
    $changed=$false
    foreach($row in $snapshot){
      $pidNow=[int]$row.ProcessId; $parent=[int]$row.ParentProcessId
      if($family.Contains($parent) -and -not $family.Contains($pidNow)){[void]$family.Add($pidNow);$changed=$true}
    }
  } while($changed)
  return @($family)
}

function Stop-ObservedInstaller([System.Diagnostics.Process]$Process,[int[]]$ObservedWindowPids,[string]$Label) {
  $ids = [System.Collections.Generic.HashSet[int]]::new()
  foreach($id in @(Get-ProcessFamily $Process.Id)){[void]$ids.Add([int]$id)}
  foreach($id in $ObservedWindowPids){if($id -gt 0){[void]$ids.Add($id)}}
  $rows=@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object { $ids.Contains([int]$_.ProcessId) })
  foreach($row in @($rows | Sort-Object ProcessId -Descending)){
    try { Stop-Process -Id ([int]$row.ProcessId) -Force -ErrorAction Stop } catch {}
  }
  Start-Sleep -Seconds 2
  $alive=@(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object { $ids.Contains([int]$_.ProcessId) })
  Assert-Condition ($alive.Count -eq 0) "$Label installer process family remained alive after interruption."
  [void]$Actions.Add([pscustomobject][ordered]@{phase=$Label;action='force-interrupt-observed-installer';process_ids=@($ids);at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiEvidence
  return @($ids)
}

function Invoke-ExpectedInstallFailure(
  [System.Diagnostics.Process]$Process,
  [string]$Phase,
  [int]$TimeoutSeconds=120
) {
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible=$false; $transactionStarted=$false; $terminalAction=$false; $observedPids=[System.Collections.Generic.HashSet[int]]::new()
  while([DateTime]::UtcNow -lt $deadline){
    $hasExited=$false
    try{$hasExited=$Process.HasExited}catch{$hasExited=$true}
    $windows=@(Get-RelevantWindows $Process.Id)
    foreach($window in $windows){
      $visible=$true
      try{[void]$observedPids.Add([int]$window.Current.ProcessId)}catch{}
      Record-Window $Phase $window
      Set-LaunchOff $Phase $window
      if(-not $transactionStarted){
        $clicked=Invoke-Button $Phase $window @('^Install$','^Next\b') $false
        if($clicked -match '(?i)^Install$'){$transactionStarted=$true}
      } else {
        $clicked=Invoke-Button $Phase $window @('^Abort$','^Cancel$','^Close$','^OK$','^Finish$') $true
        if($clicked){$terminalAction=$true}
      }
    }
    if($hasExited -and $windows.Count -eq 0){break}
    Start-Sleep -Milliseconds 350
  }
  Assert-Condition $visible "$Phase did not expose installer UI."
  Assert-Condition $transactionStarted "$Phase never positively invoked the install transaction."
  try{$Process.Refresh()}catch{}
  $exited=$false; try{$exited=$Process.HasExited}catch{$exited=$true}
  Assert-Condition $exited "$Phase did not terminate after deterministic failure."
  $exit=[int]$Process.ExitCode
  Assert-Condition ($exit -ne 0) "$Phase unexpectedly returned success; deterministic failure was not proven."
  return [pscustomobject][ordered]@{phase=$Phase;visible_ui=$visible;transaction_started=$transactionStarted;terminal_action_observed=$terminalAction;exit_code=$exit;expected_nonzero=$true;window_process_ids=@($observedPids)}
}

function Invoke-InterruptedInstall(
  [System.Diagnostics.Process]$Process,
  [string]$Phase,
  [scriptblock]$PositiveStart,
  [int]$TimeoutSeconds=120
) {
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  $visible=$false; $installInvoked=$false; $positive=$false; $observedPids=[System.Collections.Generic.HashSet[int]]::new()
  while([DateTime]::UtcNow -lt $deadline){
    try{if($Process.HasExited){throw "$Phase installer exited before interruption could be injected."}}catch{if($_.Exception.Message -like '*before interruption*'){throw}}
    $windows=@(Get-RelevantWindows $Process.Id)
    foreach($window in $windows){
      $visible=$true
      try{[void]$observedPids.Add([int]$window.Current.ProcessId)}catch{}
      Record-Window $Phase $window
      Set-LaunchOff $Phase $window
      if(-not $installInvoked){
        $clicked=Invoke-Button $Phase $window @('^Install$','^Next\b') $false
        if($clicked -match '(?i)^Install$'){$installInvoked=$true}
      }
    }
    if($installInvoked -and [bool](& $PositiveStart)){$positive=$true;break}
    Start-Sleep -Milliseconds 120
  }
  Assert-Condition $visible "$Phase did not expose installer UI."
  Assert-Condition $installInvoked "$Phase never invoked Install."
  Assert-Condition $positive "$Phase never reached a positive transaction-start observation."
  $killed=Stop-ObservedInstaller $Process @($observedPids) $Phase
  return [pscustomobject][ordered]@{phase=$Phase;visible_ui=$visible;install_invoked=$true;positive_transaction_start=$true;interruption_injected=$true;killed_process_ids=$killed}
}

function Get-OwnedHashes([string]$Root,[string]$Label) {
  $rows=@()
  foreach($relative in $ExpectedOwned){
    $path=Join-Path $Root $relative
    Assert-Condition (Test-Path -LiteralPath $path -PathType Leaf) "$Label missing owned payload: $relative"
    $rows += [pscustomobject][ordered]@{relative_path=$relative;size_bytes=[long](Get-Item $path).Length;sha256=Get-Sha256 $path}
  }
  return $rows
}

function Assert-SingleIdentity([string]$Root,[string]$ArpPath,[bool]$Machine,[string]$Label) {
  Assert-Condition (Test-Path -LiteralPath (Join-Path $Root 'VSN Dev Platform.exe') -PathType Leaf) "$Label desktop payload missing."
  Assert-Condition (Test-Path -LiteralPath $ArpPath) "$Label ARP identity missing."
  if($Machine){
    $svc=Get-ServiceEvidence
    Assert-Condition ($null -ne $svc) "$Label VSN-Agent service missing."
    Assert-Condition ($svc.name -eq $ServiceName) "$Label service identity drifted."
    Assert-Condition ($svc.start_name -match '(?i)LocalService') "$Label service account drifted: $($svc.start_name)"
  } else {
    Assert-Condition (Test-ServiceAbsent) "$Label current-user recovery created machine service."
    Assert-Condition (-not (Test-Path -LiteralPath $SecurityDir)) "$Label current-user recovery created machine security state."
  }
  return [pscustomobject][ordered]@{root=$Root;arp=$ArpPath;machine=$Machine;service=Get-ServiceEvidence;owned=@(Get-OwnedHashes $Root $Label);single_identity=$true}
}

function Invoke-NsisFailureAndRecovery([string]$Setup,[string]$Root,[string]$ArpPath,[bool]$Machine,[string]$Name) {
  Assert-PreflightClean $Root $ArpPath "$Name preflight"
  $baselinePath=Join-Path $SnapshotsPath "$Name-baseline.json"; [void](Write-Pkg0313Snapshot -Path $baselinePath)
  $sentinel=Join-Path $EvidencePath "$Name-outside-sentinel.txt"; [IO.File]::WriteAllText($sentinel,"$Name outside sentinel`n",[Text.UTF8Encoding]::new($false)); $sentinelHash=Get-Sha256 $sentinel

  $probe=New-FailureCollision $Root
  $p=Start-Process -FilePath $Setup -PassThru
  $failure=Invoke-ExpectedInstallFailure $p "$Name-forced-failure"
  $postFailure=Assert-FailureContainsOnlyProbe $probe $ArpPath $Name
  Assert-Condition ((Get-Sha256 $sentinel) -eq $sentinelHash) "$Name outside sentinel changed during failure."
  $snap=Join-Path $SnapshotsPath "$Name-post-failure.json"; [void](Write-Pkg0313Snapshot -Path $snap); Assert-Pkg0313SnapshotEqual -BaselinePath $baselinePath -CandidatePath $snap -Label "$Name forced failure"
  Remove-FailureProbe $probe
  Assert-PreflightClean $Root $ArpPath "$Name post-failure cleanup"

  $p=Start-Process -FilePath $Setup -PassThru
  $interrupted=Invoke-InterruptedInstall $p "$Name-interrupted-install" { (Test-Path -LiteralPath (Join-Path $Root 'VSN Dev Platform.exe')) -or (Test-Path -LiteralPath $ArpPath) }
  $residue=Get-InstallInventory $Root $ArpPath "$Name-post-interruption"

  $p=Start-Process -FilePath $Setup -PassThru
  $recovery=Drive-SuccessUi $p "$Name-exact-candidate-recovery" { (Test-Path -LiteralPath (Join-Path $Root 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $ArpPath) }
  Assert-Condition ($recovery.exit_code -eq 0) "$Name recovery exit code was $($recovery.exit_code)."
  $identity=Assert-SingleIdentity $Root $ArpPath $Machine "$Name recovered"

  $uninstaller=Join-Path $Root 'uninstall.exe'; Assert-Condition (Test-Path -LiteralPath $uninstaller -PathType Leaf) "$Name uninstaller missing after recovery."
  $p=Start-Process -FilePath $uninstaller -PassThru
  $cleanup=Drive-SuccessUi $p "$Name-final-uninstall" { -not (Test-Path -LiteralPath (Join-Path $Root 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $ArpPath) } $true
  Assert-Condition ($cleanup.exit_code -eq 0) "$Name cleanup exit code was $($cleanup.exit_code)."
  Assert-Condition (-not (Test-Path -LiteralPath $Root)) "$Name install root remains after final uninstall."
  Assert-Condition (Test-ServiceAbsent) "$Name service remains after final uninstall."
  Assert-Condition ((Get-Sha256 $sentinel) -eq $sentinelHash) "$Name outside sentinel changed after recovery lifecycle."
  $finalSnap=Join-Path $SnapshotsPath "$Name-final.json"; [void](Write-Pkg0313Snapshot -Path $finalSnap); Assert-Pkg0313SnapshotEqual -BaselinePath $baselinePath -CandidatePath $finalSnap -Label "$Name final cleanup"

  return [pscustomobject][ordered]@{lifecycle=$Name;failure=$failure;post_failure=$postFailure;interruption=$interrupted;post_interruption=$residue;recovery=$recovery;recovered_identity=$identity;cleanup=$cleanup;outside_sentinel_sha256=$sentinelHash;protected_state_restored=$true}
}

function Invoke-MsiFailureAndRecovery([string]$Package,[string]$ProductCode) {
  $name='wix-per-machine'; $root=$MachineRoot; $arp=Get-MsiArp $ProductCode; $msiexec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
  Assert-PreflightClean $root $arp "$name preflight"
  $baselinePath=Join-Path $SnapshotsPath "$name-baseline.json"; [void](Write-Pkg0313Snapshot -Path $baselinePath)
  $sentinel=Join-Path $EvidencePath "$name-outside-sentinel.txt"; [IO.File]::WriteAllText($sentinel,"$name outside sentinel`n",[Text.UTF8Encoding]::new($false)); $sentinelHash=Get-Sha256 $sentinel

  $probe=New-FailureCollision $root
  $failureLog=Join-Path $FailureLogsPath 'msi-forced-failure.log'
  $p=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $Package),'/L*V',('"{0}"' -f $failureLog)) -PassThru
  $failure=Invoke-ExpectedInstallFailure $p 'wix-per-machine-forced-failure'
  $failureLogEvidence=Get-LogEvidence $failureLog
  $postFailure=Assert-FailureContainsOnlyProbe $probe $arp $name
  Assert-Condition ((Get-Sha256 $sentinel) -eq $sentinelHash) 'MSI outside sentinel changed during failure.'
  $snap=Join-Path $SnapshotsPath "$name-post-failure.json"; [void](Write-Pkg0313Snapshot -Path $snap); Assert-Pkg0313SnapshotEqual -BaselinePath $baselinePath -CandidatePath $snap -Label 'MSI forced failure'
  Remove-FailureProbe $probe
  Assert-PreflightClean $root $arp "$name post-failure cleanup"

  $interruptLog=Join-Path $FailureLogsPath 'msi-interrupted-install.log'
  $p=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $Package),'/L*V',('"{0}"' -f $interruptLog)) -PassThru
  $interrupted=Invoke-InterruptedInstall $p 'wix-per-machine-interrupted-install' { (Test-Path -LiteralPath (Join-Path $root 'VSN Dev Platform.exe')) -or (Test-Path -LiteralPath $arp) }
  Start-Sleep -Seconds 2
  $residue=Get-InstallInventory $root $arp 'wix-per-machine-post-interruption'
  $interruptLogEvidence=$(if(Test-Path -LiteralPath $interruptLog){Get-LogEvidence $interruptLog}else{$null})

  $recoveryLog=Join-Path $FailureLogsPath 'msi-recovery.log'
  $p=Start-Process -FilePath $msiexec -ArgumentList @('/i',('"{0}"' -f $Package),'/L*V',('"{0}"' -f $recoveryLog)) -PassThru
  $recovery=Drive-SuccessUi $p 'wix-per-machine-exact-candidate-recovery' { (Test-Path -LiteralPath (Join-Path $root 'VSN Dev Platform.exe')) -and (Test-Path -LiteralPath $arp) }
  Assert-Condition ($recovery.exit_code -eq 0) "MSI recovery exit code was $($recovery.exit_code)."
  $recoveryLogEvidence=Get-LogEvidence $recoveryLog
  $identity=Assert-SingleIdentity $root $arp $true 'MSI recovered'

  $cleanupLog=Join-Path $FailureLogsPath 'msi-final-uninstall.log'
  $p=Start-Process -FilePath $msiexec -ArgumentList @('/x',$ProductCode,'/L*V',('"{0}"' -f $cleanupLog)) -PassThru
  $cleanup=Drive-SuccessUi $p 'wix-per-machine-final-uninstall' { -not (Test-Path -LiteralPath (Join-Path $root 'VSN Dev Platform.exe')) -and -not (Test-Path -LiteralPath $arp) } $true
  Assert-Condition ($cleanup.exit_code -eq 0) "MSI cleanup exit code was $($cleanup.exit_code)."
  $cleanupLogEvidence=Get-LogEvidence $cleanupLog
  Assert-Condition (-not (Test-Path -LiteralPath $root)) 'MSI install root remains after final uninstall.'
  Assert-Condition (Test-ServiceAbsent) 'MSI service remains after final uninstall.'
  Assert-Condition ((Get-Sha256 $sentinel) -eq $sentinelHash) 'MSI outside sentinel changed after lifecycle.'
  $finalSnap=Join-Path $SnapshotsPath "$name-final.json"; [void](Write-Pkg0313Snapshot -Path $finalSnap); Assert-Pkg0313SnapshotEqual -BaselinePath $baselinePath -CandidatePath $finalSnap -Label 'MSI final cleanup'

  return [pscustomobject][ordered]@{lifecycle=$name;failure=$failure;failure_log=$failureLogEvidence;post_failure=$postFailure;interruption=$interrupted;interruption_log=$interruptLogEvidence;post_interruption=$residue;recovery=$recovery;recovery_log=$recoveryLogEvidence;recovered_identity=$identity;cleanup=$cleanup;cleanup_log=$cleanupLogEvidence;outside_sentinel_sha256=$sentinelHash;protected_state_restored=$true}
}

$actualHead=(git rev-parse HEAD).Trim(); Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"
$CurrentUserNsisPath=(Resolve-Path -LiteralPath $CurrentUserNsisPath).Path
$PerMachineNsisPath=(Resolve-Path -LiteralPath $PerMachineNsisPath).Path
$MsiPath=(Resolve-Path -LiteralPath $MsiPath).Path
foreach($package in @($CurrentUserNsisPath,$PerMachineNsisPath,$MsiPath)){Assert-Condition ((Get-Item $package).Length -gt 0) "Empty installer package: $package"}
$productCode=Get-MsiProperty $MsiPath 'ProductCode'
Assert-PreflightClean $UserRoot $HkcuKey 'global current-user preflight'
Assert-PreflightClean $MachineRoot (Get-MsiArp $productCode) 'global machine preflight'
Assert-Condition (-not (Test-Path -LiteralPath $HklmNsisKey)) 'NSIS machine ARP unexpectedly exists at preflight.'

# Run MSI first to avoid inheriting NSIS AppSearch state; every lifecycle returns
# to a clean product-owned state before the next one begins.
$wix=Invoke-MsiFailureAndRecovery $MsiPath $productCode
$current=Invoke-NsisFailureAndRecovery $CurrentUserNsisPath $UserRoot $HkcuKey $false 'nsis-current-user'
$machine=Invoke-NsisFailureAndRecovery $PerMachineNsisPath $MachineRoot $HklmNsisKey $true 'nsis-per-machine'

$tracked=@(git status --porcelain=v1 --untracked-files=no)
if($tracked.Count -ne 0){$tracked|Write-Host;throw 'Tracked repository drift detected during 03.18 lifecycle.'}
Write-UiEvidence

$evidence=[ordered]@{
  schema_version=1
  package_id='PKG-03'
  task_id='03.18'
  source_commit=$SourceSha
  packages=[ordered]@{
    nsis_current_user=[ordered]@{path=$CurrentUserNsisPath;size_bytes=[long](Get-Item $CurrentUserNsisPath).Length;sha256=Get-Sha256 $CurrentUserNsisPath}
    nsis_per_machine=[ordered]@{path=$PerMachineNsisPath;size_bytes=[long](Get-Item $PerMachineNsisPath).Length;sha256=Get-Sha256 $PerMachineNsisPath}
    msi=[ordered]@{path=$MsiPath;size_bytes=[long](Get-Item $MsiPath).Length;sha256=Get-Sha256 $MsiPath;product_code=$productCode}
  }
  lifecycles=@($current,$machine,$wix)
  forced_failure_probe='desktop-path-directory-collision'
  forced_failure_after_positive_install_invocation=$true
  partial_owned_state_forbidden=$true
  interrupted_install_positive_start_required=$true
  exact_candidate_rerun_recovery_required=$true
  duplicate_identity_forbidden=$true
  protected_state_nonmutation_required=$true
  running_process_coordination_claimed=$false
  reboot_semantics_claimed=$false
  silent_or_passive_deployment_claimed=$false
  signing_claimed=$false
  updater_mutation_claimed=$false
  product_or_installer_mutation=$false
  tracked_repository_drift_zero=$true
}
$evidenceFile=Join-Path $EvidencePath 'evidence.json'
$evidence|ConvertTo-Json -Depth 20|Set-Content -LiteralPath $evidenceFile -Encoding utf8NoBOM
$digest=Get-Sha256 $evidenceFile
"$digest  evidence.json"|Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM
$evidence|ConvertTo-Json -Depth 20
