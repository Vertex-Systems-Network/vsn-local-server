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
# into the later MSI lane, so the MSI WelcomeDlg was never driven. A first
# task-local process-family filter removed that stale-window cross-lane leak.
#
# Exact-head run 33553660912 / job 100009113224 / artifact 9819298614
# (sha256:611ce9c441520805536aa582e24d41578e1a0983d31bb27cbff0230c4163826b)
# proved the stale-window isolation held through current-user completion and
# per-machine install, but the fresh elevated per-machine uninstaller exposed
# no window to the harness because UAC can detach its UI process from the
# ordinary parent chain. Keep exact process-family windows authoritative and
# admit only exact installer titles whose owning process started at or after
# the current operation's root-process epoch. This admits the fresh elevated
# uninstall window while rejecting prior-lane stale terminal windows.
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
  'Vsn0319WindowEpochByRoot',
  '(?i)^(VSN Dev Platform Setup|VSN Dev Platform Uninstall|Windows Installer)$',
  '$started -ge $epoch',
  'return @($owned + $fallback)'
)){
  if(-not $source.Contains($token)){throw "03.19 runtime certification patch missing token: $token"}
}

'@.Replace("`r`n","`n").Replace('__B64__','JHNjcmlwdDpWc24wMzE5V2luZG93RXBvY2hCeVJvb3QgPSBAe30KCmZ1bmN0aW9uIEdldC1SZWxldmFudFdpbmRvd3MoW2ludF0kUm9vdFBpZCkgewogICRzbmFwc2hvdCA9IEAoR2V0LUNpbUluc3RhbmNlIFdpbjMyX1Byb2Nlc3MgLUVycm9yQWN0aW9uIFNpbGVudGx5Q29udGludWUgfCBTZWxlY3QtT2JqZWN0IFByb2Nlc3NJZCxQYXJlbnRQcm9jZXNzSWQsQ3JlYXRpb25EYXRlKQogICRmYW1pbHkgPSBbU3lzdGVtLkNvbGxlY3Rpb25zLkdlbmVyaWMuSGFzaFNldFtpbnRdXTo6bmV3KCkKICBbdm9pZF0kZmFtaWx5LkFkZCgkUm9vdFBpZCkKICBkbyB7CiAgICAkY2hhbmdlZCA9ICRmYWxzZQogICAgZm9yZWFjaCAoJHByb2MgaW4gJHNuYXBzaG90KSB7CiAgICAgICRwaWROb3cgPSBbaW50XSRwcm9jLlByb2Nlc3NJZAogICAgICAkcGFyZW50ID0gW2ludF0kcHJvYy5QYXJlbnRQcm9jZXNzSWQKICAgICAgaWYgKCRmYW1pbHkuQ29udGFpbnMoJHBhcmVudCkgLWFuZCAtbm90ICRmYW1pbHkuQ29udGFpbnMoJHBpZE5vdykpIHsKICAgICAgICBbdm9pZF0kZmFtaWx5LkFkZCgkcGlkTm93KTsgJGNoYW5nZWQgPSAkdHJ1ZQogICAgICB9CiAgICB9CiAgfSB3aGlsZSAoJGNoYW5nZWQpCgogIGlmICgtbm90ICRzY3JpcHQ6VnNuMDMxOVdpbmRvd0Vwb2NoQnlSb290LkNvbnRhaW5zS2V5KCRSb290UGlkKSkgewogICAgJGVwb2NoID0gW0RhdGVUaW1lXTo6VXRjTm93LkFkZFNlY29uZHMoLTUpCiAgICAkcm9vdFByb2MgPSAkc25hcHNob3QgfCBXaGVyZS1PYmplY3QgeyBbaW50XSRfLlByb2Nlc3NJZCAtZXEgJFJvb3RQaWQgfSB8IFNlbGVjdC1PYmplY3QgLUZpcnN0IDEKICAgIGlmICgkbnVsbCAtbmUgJHJvb3RQcm9jIC1hbmQgJG51bGwgLW5lICRyb290UHJvYy5DcmVhdGlvbkRhdGUpIHsKICAgICAgdHJ5IHsgJGVwb2NoID0gKFtEYXRlVGltZV0kcm9vdFByb2MuQ3JlYXRpb25EYXRlKS5Ub1VuaXZlcnNhbFRpbWUoKS5BZGRTZWNvbmRzKC0xKSB9IGNhdGNoIHt9CiAgICB9CiAgICAkc2NyaXB0OlZzbjAzMTlXaW5kb3dFcG9jaEJ5Um9vdFskUm9vdFBpZF0gPSAkZXBvY2gKICB9CiAgJGVwb2NoID0gW0RhdGVUaW1lXSRzY3JpcHQ6VnNuMDMxOVdpbmRvd0Vwb2NoQnlSb290WyRSb290UGlkXQoKICAkcm9vdCA9IFtTeXN0ZW0uV2luZG93cy5BdXRvbWF0aW9uLkF1dG9tYXRpb25FbGVtZW50XTo6Um9vdEVsZW1lbnQKICAkYWxsID0gJHJvb3QuRmluZEFsbChbU3lzdGVtLldpbmRvd3MuQXV0b21hdGlvbi5UcmVlU2NvcGVdOjpDaGlsZHJlbixbU3lzdGVtLldpbmRvd3MuQXV0b21hdGlvbi5Db25kaXRpb25dOjpUcnVlQ29uZGl0aW9uKQogICRvd25lZCA9IEAoKQogICRmYWxsYmFjayA9IEAoKQogIGZvcmVhY2ggKCR3aW5kb3cgaW4gJGFsbCkgewogICAgdHJ5IHsKICAgICAgJG5hbWUgPSBbc3RyaW5nXSR3aW5kb3cuQ3VycmVudC5OYW1lCiAgICAgICRwaWROb3cgPSBbaW50XSR3aW5kb3cuQ3VycmVudC5Qcm9jZXNzSWQKICAgICAgJGhhbmRsZSA9IFtpbnRdJHdpbmRvdy5DdXJyZW50Lk5hdGl2ZVdpbmRvd0hhbmRsZQogICAgICAkdmlzaWJsZSA9IC1ub3QgW2Jvb2xdJHdpbmRvdy5DdXJyZW50LklzT2Zmc2NyZWVuCiAgICAgIGlmICgtbm90ICR2aXNpYmxlIC1vciAkaGFuZGxlIC1lcSAwKSB7IGNvbnRpbnVlIH0KICAgICAgaWYgKCRmYW1pbHkuQ29udGFpbnMoJHBpZE5vdykpIHsKICAgICAgICAkb3duZWQgKz0gJHdpbmRvdwogICAgICAgIGNvbnRpbnVlCiAgICAgIH0KCiAgICAgIGlmICgkbmFtZSAtbm90bWF0Y2ggJyg/aSleKFZTTiBEZXYgUGxhdGZvcm0gU2V0dXB8VlNOIERldiBQbGF0Zm9ybSBVbmluc3RhbGx8V2luZG93cyBJbnN0YWxsZXIpJCcpIHsgY29udGludWUgfQoKICAgICAgJHN0YXJ0ZWQgPSAkbnVsbAogICAgICAkcHJvY1JvdyA9ICRzbmFwc2hvdCB8IFdoZXJlLU9iamVjdCB7IFtpbnRdJF8uUHJvY2Vzc0lkIC1lcSAkcGlkTm93IH0gfCBTZWxlY3QtT2JqZWN0IC1GaXJzdCAxCiAgICAgIGlmICgkbnVsbCAtbmUgJHByb2NSb3cgLWFuZCAkbnVsbCAtbmUgJHByb2NSb3cuQ3JlYXRpb25EYXRlKSB7CiAgICAgICAgdHJ5IHsgJHN0YXJ0ZWQgPSAoW0RhdGVUaW1lXSRwcm9jUm93LkNyZWF0aW9uRGF0ZSkuVG9Vbml2ZXJzYWxUaW1lKCkgfSBjYXRjaCB7fQogICAgICB9CiAgICAgIGlmICgkbnVsbCAtZXEgJHN0YXJ0ZWQpIHsKICAgICAgICB0cnkgeyAkc3RhcnRlZCA9IChHZXQtUHJvY2VzcyAtSWQgJHBpZE5vdyAtRXJyb3JBY3Rpb24gU3RvcCkuU3RhcnRUaW1lLlRvVW5pdmVyc2FsVGltZSgpIH0gY2F0Y2gge30KICAgICAgfQogICAgICBpZiAoJG51bGwgLW5lICRzdGFydGVkIC1hbmQgJHN0YXJ0ZWQgLWdlICRlcG9jaCkgewogICAgICAgICRmYWxsYmFjayArPSAkd2luZG93CiAgICAgIH0KICAgIH0gY2F0Y2gge30KICB9CiAgcmV0dXJuIEAoJG93bmVkICsgJGZhbGxiYWNrKQp9Cg==')

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
