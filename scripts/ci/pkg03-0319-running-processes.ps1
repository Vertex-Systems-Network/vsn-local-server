param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.19'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Evidence-bounded outer shim over the exact previously accepted 03.19 wrapper.
# Run 33331234433 / artifact 9737999463 proved the current-user lifecycle itself
# reached the explicit NSIS running-process prompt, cancelled into a coherent
# deterministic safe block without pre-killing Desktop/CLI, performed operator
# cleanup only after that proof, and completed retry uninstall. The sole failure
# was protected-state equality after Windows independently refreshed localized
# AppX firewall Group resource strings for Microsoft.DesktopAppInstaller and
# Microsoft.WindowsFeedbackHub. Rule count and stable rule semantics were
# unchanged. Normalize only the four-part package version embedded in those two
# exact inbox resource-display strings; every other firewall field and all
# hosts/resolver/trust state remain strict. Product/installer behavior and the
# shared accepted 03.13 helper are unchanged.

$PriorCommit='2359555c0a83f3c83dcd8b0c4514a6f34ecca821'
$PriorPath='scripts/ci/pkg03-0319-running-processes.ps1'
$ExpectedPriorBlob='dffe9f0a97e6c96650435a06e312546693aecc16'

$blob=(& git rev-parse "${PriorCommit}:${PriorPath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedPriorBlob){
  throw "03.19 prior-wrapper blob mismatch: expected=$ExpectedPriorBlob actual=$blob"
}
$wrapper=(& git show "${PriorCommit}:${PriorPath}"|Out-String).Replace("`r`n","`n")
if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($wrapper)){throw '03.19 failed to load pinned prior wrapper.'}
foreach($token in @(
  'harness_pre_kill=$false',
  'operator-cleanup-after-proven-block',
  '\bis running\b[\s\S]*\bkill\b',
  'QueryFullProcessImageName',
  'native-terminal-bm-click'
)){
  if(-not $wrapper.Contains($token)){throw "03.19 pinned prior wrapper missing evidence token: $token"}
}

$boundary='$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}'
if(([regex]::Matches($wrapper,[regex]::Escape($boundary))).Count -ne 1){
  throw '03.19 outer semantic-snapshot injection boundary mismatch.'
}

$semanticPatch=@'
# Run 33331234433 / artifact 9737999463: Windows Store servicing refreshed only
# the package-version component inside firewall Group display-resource strings
# for two Microsoft inbox apps during the bounded lifecycle. Preserve strict
# protected-state semantics while excluding only that evidenced volatile display
# metadata. No VSN firewall rule or functional firewall field is normalized.
$stableSnapshotHelper=@'
function ConvertTo-Pkg0319StableProtectedSnapshot([object]$Snapshot,[string]$Side){
  $allowedPackages=@('Microsoft.DesktopAppInstaller','Microsoft.WindowsFeedbackHub')
  foreach($rule in @($Snapshot.firewall.rules)){
    $group=[string]$rule.group
    foreach($package in $allowedPackages){
      $pattern='^@\{'+[regex]::Escape($package)+'_(?<version>\d+(?:\.\d+){3})_(?<tail>[^}]+\?ms-resource://.+)\}$'
      if($group -match $pattern){
        $rule.group='@{'+$package+'_<package-version>_'+$Matches.tail+'}'
        break
      }
    }
  }
  return $Snapshot
}
function Assert-Pkg0319SnapshotEqual([string]$BaselinePath,[string]$CandidatePath,[string]$Label){
  $baseline=Get-Content -LiteralPath $BaselinePath -Raw | ConvertFrom-Json -Depth 100
  $candidate=Get-Content -LiteralPath $CandidatePath -Raw | ConvertFrom-Json -Depth 100

  # Bind rule identity/count before normalization so normalization can never hide
  # rule insertion/deletion or identity drift.
  $key={param($r) ([string]$r.name)+'|'+([string]$r.direction)+'|'+([string]$r.action)+'|'+([string]$r.profile)+'|'+([string]$r.owner)}
  $bKeys=@($baseline.firewall.rules|ForEach-Object{& $key $_}|Sort-Object)
  $cKeys=@($candidate.firewall.rules|ForEach-Object{& $key $_}|Sort-Object)
  if($bKeys.Count -ne $cKeys.Count -or (($bKeys -join "`n") -cne ($cKeys -join "`n"))){
    throw "03.19 protected firewall rule identity/count changed during $Label."
  }

  $changedGroups=@()
  $candidateByKey=@{}
  foreach($r in @($candidate.firewall.rules)){$candidateByKey[(& $key $r)]=$r}
  foreach($r in @($baseline.firewall.rules)){
    $k=& $key $r
    if($candidateByKey.ContainsKey($k) -and ([string]$r.group -cne [string]$candidateByKey[$k].group)){
      $changedGroups += [pscustomobject][ordered]@{rule=$r.name;baseline_group=[string]$r.group;candidate_group=[string]$candidateByKey[$k].group}
    }
  }

  [void](ConvertTo-Pkg0319StableProtectedSnapshot $baseline 'baseline')
  [void](ConvertTo-Pkg0319StableProtectedSnapshot $candidate 'candidate')
  $bJson=$baseline|ConvertTo-Json -Depth 100 -Compress
  $cJson=$candidate|ConvertTo-Json -Depth 100 -Compress
  if($bJson -cne $cJson){
    throw "03.19 protected Windows state changed beyond the two evidenced inbox firewall Group package-version strings during $Label. baseline=$BaselinePath candidate=$CandidatePath"
  }

  [void]$Actions.Add([pscustomobject][ordered]@{
    phase=$Label
    action='protected-state-semantic-equality'
    normalized_scope='firewall.rules.group package-version only'
    allowed_packages=@('Microsoft.DesktopAppInstaller','Microsoft.WindowsFeedbackHub')
    changed_group_records=$changedGroups
    rule_identity_count=$bKeys.Count
    all_other_protected_state_equal=$true
    at_utc=[DateTime]::UtcNow.ToString('o')
  })
  Write-UiEvidence
}
'@
$stableSnapshotHelper=$stableSnapshotHelper.Replace("`r`n","`n")
$snapshotDot=". (Join-Path (Get-Location) 'scripts/ci/pkg03-0313-snapshot.ps1')"
if(([regex]::Matches($source,[regex]::Escape($snapshotDot))).Count -ne 1){throw '03.19 runtime snapshot-helper injection boundary mismatch.'}
$source=$source.Replace($snapshotDot,$snapshotDot+"`n"+$stableSnapshotHelper)
$assertCount=[regex]::Matches($source,'\bAssert-Pkg0313SnapshotEqual\b').Count
if($assertCount -ne 2){throw "03.19 expected exactly 2 protected-state assertion calls, found $assertCount"}
$source=[regex]::Replace($source,'\bAssert-Pkg0313SnapshotEqual\b','Assert-Pkg0319SnapshotEqual')
foreach($token in @('protected-state-semantic-equality','Microsoft.DesktopAppInstaller','Microsoft.WindowsFeedbackHub','all_other_protected_state_equal=$true')){
  if(-not $source.Contains($token)){throw "03.19 semantic snapshot patch missing token: $token"}
}

'@.Replace("`r`n","`n")

$patchedWrapper=$wrapper.Replace($boundary,$semanticPatch+$boundary)
$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$runtimeWrapper=Join-Path $tempRoot 'pkg03-0319-running-processes-wrapper-runtime.ps1'
[IO.File]::WriteAllText($runtimeWrapper,$patchedWrapper,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeWrapper,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){$errors|ForEach-Object{Write-Host $_.Message};throw "03.19 outer runtime wrapper has $($errors.Count) parse error(s)."}

& $runtimeWrapper `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
