param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.19'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Evidence-bounded outer shim over the exact prior 03.19 wrapper.
# Run 33331234433 / artifact 9737999463 proved current-user running-resource
# handling reaches an explicit NSIS running-process prompt, cancels into a
# coherent deterministic safe block without harness pre-kill, performs operator
# cleanup only after that proof, and then completes retry uninstall. The sole
# failing assertion was protected-state equality because Windows servicing
# independently refreshed AppX package versions embedded only in firewall Group
# display-resource strings for Microsoft.DesktopAppInstaller and
# Microsoft.WindowsFeedbackHub. Inject the task-local stable comparator only;
# shared 03.13 snapshot code and product/installer behavior remain untouched.

$PriorCommit='2359555c0a83f3c83dcd8b0c4514a6f34ecca821'
$PriorPath='scripts/ci/pkg03-0319-running-processes.ps1'
$ExpectedPriorBlob='dffe9f0a97e6c96650435a06e312546693aecc16'
$StableHelper='scripts/ci/pkg03-0319-stable-snapshot.ps1'

$blob=(& git rev-parse "${PriorCommit}:${PriorPath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedPriorBlob){
  throw "03.19 prior-wrapper blob mismatch: expected=$ExpectedPriorBlob actual=$blob"
}
if(-not (Test-Path -LiteralPath $StableHelper -PathType Leaf)){throw '03.19 task-local stable snapshot helper missing.'}
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
if(([regex]::Matches($wrapper,[regex]::Escape($boundary))).Count -ne 1){throw '03.19 semantic-comparator injection boundary mismatch.'}

$semanticPatch=@'
# Inject task-local comparator after the accepted 03.13 snapshot helper is
# loaded. Replace only the two lifecycle equality call sites; snapshot capture
# itself remains the canonical accepted implementation.
$snapshotDot=". (Join-Path (Get-Location) 'scripts/ci/pkg03-0313-snapshot.ps1')"
$stableDot=". (Join-Path (Get-Location) 'scripts/ci/pkg03-0319-stable-snapshot.ps1')"
if(([regex]::Matches($source,[regex]::Escape($snapshotDot))).Count -ne 1){throw '03.19 runtime stable-comparator injection boundary mismatch.'}
$source=$source.Replace($snapshotDot,$snapshotDot+"`n"+$stableDot)
$assertCount=[regex]::Matches($source,'\bAssert-Pkg0313SnapshotEqual\b').Count
if($assertCount -ne 2){throw "03.19 expected exactly 2 protected-state equality call sites, found $assertCount"}
$source=[regex]::Replace($source,'\bAssert-Pkg0313SnapshotEqual\b','Assert-Pkg0319SnapshotEqual')
foreach($token in @('pkg03-0319-stable-snapshot.ps1','Assert-Pkg0319SnapshotEqual','harness_pre_kill=$false')){
  if(-not $source.Contains($token)){throw "03.19 runtime comparator patch missing token: $token"}
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
