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
#
# Exact-head run 33549239142 / failure artifact 9818101165 then proved both
# NSIS lanes can complete their bounded safe-block/retry paths while a detached
# prior-lane NSIS terminal window remains visible. The accepted 03.15 helper's
# broad title fallback admitted that stale "VSN Dev Platform Uninstall" window
# into the later MSI lane, so the MSI WelcomeDlg was never driven. Inject a
# task-local window-ownership override only: exact process-family windows are
# authoritative and ordered first; fallback is restricted to MSI/setup titles.
# No installer/product process is killed or otherwise mutated by this filter.

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
$strictWindowFilter=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('__B64__'))
$source=$source.Replace($snapshotDot,$snapshotDot+"`n"+$stableDot+"`n"+$strictWindowFilter)
$assertCount=[regex]::Matches($source,'\bAssert-Pkg0313SnapshotEqual\b').Count
if($assertCount -ne 2){throw "03.19 expected exactly 2 protected-state equality call sites, found $assertCount"}
$source=[regex]::Replace($source,'\bAssert-Pkg0313SnapshotEqual\b','Assert-Pkg0319SnapshotEqual')
foreach($token in @(
  'pkg03-0319-stable-snapshot.ps1',
  'Assert-Pkg0319SnapshotEqual',
  'harness_pre_kill=$false',
  'function Get-RelevantWindows([int]$RootPid)',
  '(?i)^(VSN Dev Platform Setup|Windows Installer)$',
  'return @($owned + $fallback)'
)){
  if(-not $source.Contains($token)){throw "03.19 runtime certification patch missing token: $token"}
}

'@.Replace("`r`n","`n").Replace('__B64__','ZnVuY3Rpb24gR2V0LVJlbGV2YW50V2luZG93cyhbaW50XSRSb290UGlkKSB7CiAgJHNuYXBzaG90ID0gQChHZXQtQ2ltSW5zdGFuY2UgV2luMzJfUHJvY2VzcyAtRXJyb3JBY3Rpb24gU2lsZW50bHlDb250aW51ZSB8IFNlbGVjdC1PYmplY3QgUHJvY2Vzc0lkLFBhcmVudFByb2Nlc3NJZCkKICAkZmFtaWx5ID0gW1N5c3RlbS5Db2xsZWN0aW9ucy5HZW5lcmljLkhhc2hTZXRbaW50XV06Om5ldygpCiAgW3ZvaWRdJGZhbWlseS5BZGQoJFJvb3RQaWQpCiAgZG8gewogICAgJGNoYW5nZWQgPSAkZmFsc2UKICAgIGZvcmVhY2ggKCRwcm9jIGluICRzbmFwc2hvdCkgewogICAgICAkcGlkTm93ID0gW2ludF0kcHJvYy5Qcm9jZXNzSWQKICAgICAgJHBhcmVudCA9IFtpbnRdJHByb2MuUGFyZW50UHJvY2Vzc0lkCiAgICAgIGlmICgkZmFtaWx5LkNvbnRhaW5zKCRwYXJlbnQpIC1hbmQgLW5vdCAkZmFtaWx5LkNvbnRhaW5zKCRwaWROb3cpKSB7CiAgICAgICAgW3ZvaWRdJGZhbWlseS5BZGQoJHBpZE5vdyk7ICRjaGFuZ2VkID0gJHRydWUKICAgICAgfQogICAgfQogIH0gd2hpbGUgKCRjaGFuZ2VkKQoKICAkcm9vdCA9IFtTeXN0ZW0uV2luZG93cy5BdXRvbWF0aW9uLkF1dG9tYXRpb25FbGVtZW50XTo6Um9vdEVsZW1lbnQKICAkYWxsID0gJHJvb3QuRmluZEFsbChbU3lzdGVtLldpbmRvd3MuQXV0b21hdGlvbi5UcmVlU2NvcGVdOjpDaGlsZHJlbixbU3lzdGVtLldpbmRvd3MuQXV0b21hdGlvbi5Db25kaXRpb25dOjpUcnVlQ29uZGl0aW9uKQogICRvd25lZCA9IEAoKQogICRmYWxsYmFjayA9IEAoKQogIGZvcmVhY2ggKCR3aW5kb3cgaW4gJGFsbCkgewogICAgdHJ5IHsKICAgICAgJG5hbWUgPSBbc3RyaW5nXSR3aW5kb3cuQ3VycmVudC5OYW1lCiAgICAgICRwaWROb3cgPSBbaW50XSR3aW5kb3cuQ3VycmVudC5Qcm9jZXNzSWQKICAgICAgJGhhbmRsZSA9IFtpbnRdJHdpbmRvdy5DdXJyZW50Lk5hdGl2ZVdpbmRvd0hhbmRsZQogICAgICAkdmlzaWJsZSA9IC1ub3QgW2Jvb2xdJHdpbmRvdy5DdXJyZW50LklzT2Zmc2NyZWVuCiAgICAgIGlmICgtbm90ICR2aXNpYmxlIC1vciAkaGFuZGxlIC1lcSAwKSB7IGNvbnRpbnVlIH0KICAgICAgaWYgKCRmYW1pbHkuQ29udGFpbnMoJHBpZE5vdykpIHsKICAgICAgICAkb3duZWQgKz0gJHdpbmRvdwogICAgICAgIGNvbnRpbnVlCiAgICAgIH0KICAgICAgaWYgKCRuYW1lIC1tYXRjaCAnKD9pKV4oVlNOIERldiBQbGF0Zm9ybSBTZXR1cHxXaW5kb3dzIEluc3RhbGxlcikkJykgewogICAgICAgICRmYWxsYmFjayArPSAkd2luZG93CiAgICAgIH0KICAgIH0gY2F0Y2gge30KICB9CiAgcmV0dXJuIEAoJG93bmVkICsgJGZhbGxiYWNrKQp9Cg==')

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
