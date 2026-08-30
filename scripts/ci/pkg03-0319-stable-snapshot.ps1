Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Task-local semantic comparator for PKG-03 03.19 only.
# Exact-head run 33331234433 / artifact 9737999463 showed Windows servicing
# changed only the four-part AppX package version embedded in Firewall Rule
# Group display-resource strings for these two Microsoft inbox apps:
# - Microsoft.DesktopAppInstaller
# - Microsoft.WindowsFeedbackHub
# Rule count, rule identity, direction, action, profile and owner were unchanged.
# Normalize only that evidenced volatile display metadata. Every other firewall
# field and all hosts/resolver/trust state remain strict.

function ConvertTo-Pkg0319StableProtectedSnapshot([object]$Snapshot) {
  $allowedPackages=@('Microsoft.DesktopAppInstaller','Microsoft.WindowsFeedbackHub')
  foreach($rule in @($Snapshot.firewall.rules)) {
    $group=[string]$rule.group
    foreach($package in $allowedPackages) {
      $pattern='^@\{'+[regex]::Escape($package)+'_(?<version>\d+(?:\.\d+){3})_(?<tail>[^}]+\?ms-resource://.+)\}$'
      if($group -match $pattern) {
        $rule.group='@{'+$package+'_<package-version>_'+$Matches.tail+'}'
        break
      }
    }
  }
  return $Snapshot
}

function Assert-Pkg0319SnapshotEqual([string]$BaselinePath,[string]$CandidatePath,[string]$Label) {
  $baseline=Get-Content -LiteralPath $BaselinePath -Raw | ConvertFrom-Json -Depth 100
  $candidate=Get-Content -LiteralPath $CandidatePath -Raw | ConvertFrom-Json -Depth 100

  $ruleKey={
    param($r)
    ([string]$r.name)+'|'+([string]$r.direction)+'|'+([string]$r.action)+'|'+([string]$r.profile)+'|'+([string]$r.owner)
  }
  $bKeys=@($baseline.firewall.rules | ForEach-Object { & $ruleKey $_ } | Sort-Object)
  $cKeys=@($candidate.firewall.rules | ForEach-Object { & $ruleKey $_ } | Sort-Object)
  if($bKeys.Count -ne $cKeys.Count -or (($bKeys -join "`n") -cne ($cKeys -join "`n"))) {
    throw "03.19 protected firewall rule identity/count changed during $Label."
  }

  $candidateByKey=@{}
  foreach($r in @($candidate.firewall.rules)) {
    $k=& $ruleKey $r
    $candidateByKey[$k]=$r
  }
  $changedGroups=@()
  foreach($r in @($baseline.firewall.rules)) {
    $k=& $ruleKey $r
    if($candidateByKey.ContainsKey($k) -and ([string]$r.group -cne [string]$candidateByKey[$k].group)) {
      $changedGroups += [pscustomobject][ordered]@{
        rule=[string]$r.name
        baseline_group=[string]$r.group
        candidate_group=[string]$candidateByKey[$k].group
      }
    }
  }

  [void](ConvertTo-Pkg0319StableProtectedSnapshot $baseline)
  [void](ConvertTo-Pkg0319StableProtectedSnapshot $candidate)
  $bJson=$baseline | ConvertTo-Json -Depth 100 -Compress
  $cJson=$candidate | ConvertTo-Json -Depth 100 -Compress
  if($bJson -cne $cJson) {
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
