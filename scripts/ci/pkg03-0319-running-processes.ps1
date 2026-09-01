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
# proved stale-window isolation through current-user completion and per-machine
# install, but UAC-detached uninstall UI was outside the ordinary parent chain.
#
# Exact-head run 33557730196 / job 100022538016 / artifact 9820669466
# (sha256:c881cd42810492578019a1967d12a436ba25f26d8b3b7c9d34c38323dd29b89e)
# then proved process-start epoch admission is not reliable enough: all three
# package builds passed and an orphan NSIS `Un` process existed, but the harness
# recorded zero uninstall-window observations before bounded timeout.
#
# Exact-head run 33560591254 / job 100031841295 / artifact 9821884579
# (sha256:b53601b2ca403593b80fa755a97e6dfd1105d69f7bdbaeefea806086c00177cb)
# proved current-user running-uninstall enters an evidenced safe block but leaves
# its terminal "Uninstallation Aborted" HWND open for the full 120-second bound.
# Runner cleanup later found that orphan `Un` process plus the per-machine `Un`
# process. The subsequent per-machine root handoff exited with zero UI
# observations. Close only a proven terminal installer window via the already
# accepted 03.19 WM_CLOSE fallback; do not kill or mutate product processes.
#
# Keep exact process-family windows authoritative. For detached uninstall UI,
# admit a non-terminal "VSN Dev Platform Uninstall" HWND once per root operation
# and retain that same handle as it transitions to terminal state. A prior-lane
# stale terminal window is never admitted into a later operation. Exact Setup
# and Windows Installer fallback remains bounded to those exact titles.
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
  'Vsn0319AdmittedUninstallHandlesByRoot',
  '(?i)^(VSN Dev Platform Setup|Windows Installer)$',
  '(?i)^VSN Dev Platform Uninstall$',
  'terminalStale',
  'Uninstallation Complete|Uninstallation Aborted',
  'function Invoke-Button(',
  'terminal-window-fallback',
  'Invoke-NativeTerminal $Phase $Window $null',
  'return @($owned + $fallback)'
)){
  if(-not $source.Contains($token)){throw "03.19 runtime certification patch missing token: $token"}
}

'@.Replace("`r`n","`n").Replace('__B64__','JHNjcmlwdDpWc24wMzE5QWRtaXR0ZWRVbmluc3RhbGxIYW5kbGVzQnlSb290ID0gQHt9CgpmdW5jdGlvbiBHZXQtUmVsZXZhbnRXaW5kb3dzKFtpbnRdJFJvb3RQaWQpIHsKICAkc25hcHNob3QgPSBAKEdldC1DaW1JbnN0YW5jZSBXaW4zMl9Qcm9jZXNzIC1FcnJvckFjdGlvbiBTaWxlbnRseUNvbnRpbnVlIHwgU2VsZWN0LU9iamVjdCBQcm9jZXNzSWQsUGFyZW50UHJvY2Vzc0lkKQogICRmYW1pbHkgPSBbU3lzdGVtLkNvbGxlY3Rpb25zLkdlbmVyaWMuSGFzaFNldFtpbnRdXTo6bmV3KCkKICBbdm9pZF0kZmFtaWx5LkFkZCgkUm9vdFBpZCkKICBkbyB7CiAgICAkY2hhbmdlZCA9ICRmYWxzZQogICAgZm9yZWFjaCAoJHByb2MgaW4gJHNuYXBzaG90KSB7CiAgICAgICRwaWROb3cgPSBbaW50XSRwcm9jLlByb2Nlc3NJZAogICAgICAkcGFyZW50ID0gW2ludF0kcHJvYy5QYXJlbnRQcm9jZXNzSWQKICAgICAgaWYgKCRmYW1pbHkuQ29udGFpbnMoJHBhcmVudCkgLWFuZCAtbm90ICRmYW1pbHkuQ29udGFpbnMoJHBpZE5vdykpIHsKICAgICAgICBbdm9pZF0kZmFtaWx5LkFkZCgkcGlkTm93KTsgJGNoYW5nZWQgPSAkdHJ1ZQogICAgICB9CiAgICB9CiAgfSB3aGlsZSAoJGNoYW5nZWQpCgogIGlmICgtbm90ICRzY3JpcHQ6VnNuMDMxOUFkbWl0dGVkVW5pbnN0YWxsSGFuZGxlc0J5Um9vdC5Db250YWluc0tleSgkUm9vdFBpZCkpIHsKICAgICRzY3JpcHQ6VnNuMDMxOUFkbWl0dGVkVW5pbnN0YWxsSGFuZGxlc0J5Um9vdFskUm9vdFBpZF0gPSBbU3lzdGVtLkNvbGxlY3Rpb25zLkdlbmVyaWMuSGFzaFNldFtpbnRdXTo6bmV3KCkKICB9CiAgJGFkbWl0dGVkVW5pbnN0YWxsSGFuZGxlcyA9ICRzY3JpcHQ6VnNuMDMxOUFkbWl0dGVkVW5pbnN0YWxsSGFuZGxlc0J5Um9vdFskUm9vdFBpZF0KCiAgJHJvb3QgPSBbU3lzdGVtLldpbmRvd3MuQXV0b21hdGlvbi5BdXRvbWF0aW9uRWxlbWVudF06OlJvb3RFbGVtZW50CiAgJGFsbCA9ICRyb290LkZpbmRBbGwoW1N5c3RlbS5XaW5kb3dzLkF1dG9tYXRpb24uVHJlZVNjb3BlXTo6Q2hpbGRyZW4sW1N5c3RlbS5XaW5kb3dzLkF1dG9tYXRpb24uQ29uZGl0aW9uXTo6VHJ1ZUNvbmRpdGlvbikKICAkb3duZWQgPSBAKCkKICAkZmFsbGJhY2sgPSBAKCkKICBmb3JlYWNoICgkd2luZG93IGluICRhbGwpIHsKICAgIHRyeSB7CiAgICAgICRuYW1lID0gW3N0cmluZ10kd2luZG93LkN1cnJlbnQuTmFtZQogICAgICAkcGlkTm93ID0gW2ludF0kd2luZG93LkN1cnJlbnQuUHJvY2Vzc0lkCiAgICAgICRoYW5kbGUgPSBbaW50XSR3aW5kb3cuQ3VycmVudC5OYXRpdmVXaW5kb3dIYW5kbGUKICAgICAgJHZpc2libGUgPSAtbm90IFtib29sXSR3aW5kb3cuQ3VycmVudC5Jc09mZnNjcmVlbgogICAgICBpZiAoLW5vdCAkdmlzaWJsZSAtb3IgJGhhbmRsZSAtZXEgMCkgeyBjb250aW51ZSB9CiAgICAgIGlmICgkZmFtaWx5LkNvbnRhaW5zKCRwaWROb3cpKSB7CiAgICAgICAgJG93bmVkICs9ICR3aW5kb3cKICAgICAgICBjb250aW51ZQogICAgICB9CgogICAgICBpZiAoJG5hbWUgLW1hdGNoICcoP2kpXihWU04gRGV2IFBsYXRmb3JtIFNldHVwfFdpbmRvd3MgSW5zdGFsbGVyKSQnKSB7CiAgICAgICAgJGZhbGxiYWNrICs9ICR3aW5kb3cKICAgICAgICBjb250aW51ZQogICAgICB9CiAgICAgIGlmICgkbmFtZSAtbm90bWF0Y2ggJyg/aSleVlNOIERldiBQbGF0Zm9ybSBVbmluc3RhbGwkJykgeyBjb250aW51ZSB9CgogICAgICBpZiAoJGFkbWl0dGVkVW5pbnN0YWxsSGFuZGxlcy5Db250YWlucygkaGFuZGxlKSkgewogICAgICAgICRmYWxsYmFjayArPSAkd2luZG93CiAgICAgICAgY29udGludWUKICAgICAgfQoKICAgICAgJHRleHQgPSAkbmFtZQogICAgICB0cnkgeyAkdGV4dCA9IEdldC1XaW5kb3dUZXh0ICR3aW5kb3cgfSBjYXRjaCB7fQogICAgICAkdGVybWluYWxTdGFsZSA9ICR0ZXh0IC1tYXRjaCAnKD9pKShVbmluc3RhbGxhdGlvbiBDb21wbGV0ZXxVbmluc3RhbGxhdGlvbiBBYm9ydGVkfFVuaW5zdGFsbCB3YXMgbm90IGNvbXBsZXRlZCBzdWNjZXNzZnVsbHl8aGFzIGJlZW4gdW5pbnN0YWxsZWR8Q2xpY2sgRmluaXNoIHRvIGNsb3NlIFVuaW5zdGFsbCknCiAgICAgIGlmICgtbm90ICR0ZXJtaW5hbFN0YWxlKSB7CiAgICAgICAgW3ZvaWRdJGFkbWl0dGVkVW5pbnN0YWxsSGFuZGxlcy5BZGQoJGhhbmRsZSkKICAgICAgICAkZmFsbGJhY2sgKz0gJHdpbmRvdwogICAgICB9CiAgICB9IGNhdGNoIHt9CiAgfQogIHJldHVybiBAKCRvd25lZCArICRmYWxsYmFjaykKfQoKZnVuY3Rpb24gSW52b2tlLUJ1dHRvbigKICBbc3RyaW5nXSRQaGFzZSwKICBbU3lzdGVtLldpbmRvd3MuQXV0b21hdGlvbi5BdXRvbWF0aW9uRWxlbWVudF0kV2luZG93LAogIFtzdHJpbmdbXV0kUGF0dGVybnMsCiAgW2Jvb2xdJFRlcm1pbmFsUmVhZHkgPSAkZmFsc2UKKSB7CiAgJGJ1dHRvbnMgPSBAKCkKICBmb3JlYWNoICgkYnV0dG9uIGluIEAoR2V0LUNvbnRyb2xzICRXaW5kb3cgKFtTeXN0ZW0uV2luZG93cy5BdXRvbWF0aW9uLkNvbnRyb2xUeXBlXTo6QnV0dG9uKSkpIHsKICAgIHRyeSB7CiAgICAgIGlmICgtbm90IFtib29sXSRidXR0b24uQ3VycmVudC5Jc0VuYWJsZWQgLW9yIFtib29sXSRidXR0b24uQ3VycmVudC5Jc09mZnNjcmVlbikgeyBjb250aW51ZSB9CiAgICAgICRhdXRvbWF0aW9uSWQgPSBbc3RyaW5nXSRidXR0b24uQ3VycmVudC5BdXRvbWF0aW9uSWQKICAgICAgJG5hdGl2ZUhhbmRsZSA9IFtpbnRdJGJ1dHRvbi5DdXJyZW50Lk5hdGl2ZVdpbmRvd0hhbmRsZQogICAgICBpZiAoJG5hdGl2ZUhhbmRsZSAtZXEgMCAtYW5kICRhdXRvbWF0aW9uSWQgLW1hdGNoICdeKENsb3NlfE1pbmltaXplfE1heGltaXplKSQnKSB7IGNvbnRpbnVlIH0KICAgICAgJG5hbWUgPSBHZXQtU2FmZU5hbWUgJGJ1dHRvbgogICAgICBpZiAoJG5hbWUpIHsgJGJ1dHRvbnMgKz0gW3BzY3VzdG9tb2JqZWN0XUB7ZWxlbWVudD0kYnV0dG9uO25hbWU9JG5hbWU7bm9ybT0oJG5hbWUgLXJlcGxhY2UgJyYnLCcnKS5UcmltKCl9IH0KICAgIH0gY2F0Y2gge30KICB9CiAgZm9yZWFjaCAoJHBhdHRlcm4gaW4gJFBhdHRlcm5zKSB7CiAgICAkc2VsZWN0ZWQgPSAkYnV0dG9ucyB8IFdoZXJlLU9iamVjdCB7ICRfLm5vcm0gLW1hdGNoICIoP2kpJHBhdHRlcm4iIH0gfCBTZWxlY3QtT2JqZWN0IC1GaXJzdCAxCiAgICBpZiAoJG51bGwgLWVxICRzZWxlY3RlZCkgeyBjb250aW51ZSB9CiAgICBpZiAoJFRlcm1pbmFsUmVhZHkgLWFuZCAkc2VsZWN0ZWQubm9ybSAtbWF0Y2ggJyg/aSleKEZpbmlzaHxDbG9zZXxPSykkJykgewogICAgICBJbnZva2UtTmF0aXZlVGVybWluYWwgJFBoYXNlICRXaW5kb3cgJHNlbGVjdGVkLmVsZW1lbnQgJHNlbGVjdGVkLm5hbWUKICAgICAgcmV0dXJuICRzZWxlY3RlZC5ub3JtCiAgICB9CiAgICB0cnkgewogICAgICAkaW52b2tlID0gW1N5c3RlbS5XaW5kb3dzLkF1dG9tYXRpb24uSW52b2tlUGF0dGVybl0kc2VsZWN0ZWQuZWxlbWVudC5HZXRDdXJyZW50UGF0dGVybihbU3lzdGVtLldpbmRvd3MuQXV0b21hdGlvbi5JbnZva2VQYXR0ZXJuXTo6UGF0dGVybikKICAgICAgJGludm9rZS5JbnZva2UoKQogICAgICBbdm9pZF0kQWN0aW9ucy5BZGQoW3BzY3VzdG9tb2JqZWN0XVtvcmRlcmVkXUB7cGhhc2U9JFBoYXNlO2FjdGlvbj0naW52b2tlLWJ1dHRvbic7Y29udHJvbD0kc2VsZWN0ZWQubmFtZTthdF91dGM9W0RhdGVUaW1lXTo6VXRjTm93LlRvU3RyaW5nKCdvJyl9KQogICAgICBXcml0ZS1VaUV2aWRlbmNlCiAgICAgIHJldHVybiAkc2VsZWN0ZWQubm9ybQogICAgfSBjYXRjaCB7fQogIH0KCiAgaWYgKCRUZXJtaW5hbFJlYWR5KSB7CiAgICAkdGVybWluYWxUZXh0ID0gJycKICAgIHRyeSB7ICR0ZXJtaW5hbFRleHQgPSBHZXQtV2luZG93VGV4dCAkV2luZG93IH0gY2F0Y2gge30KICAgIGlmICgkdGVybWluYWxUZXh0IC1tYXRjaCAnKD9pKShVbmluc3RhbGxhdGlvbiBDb21wbGV0ZXxVbmluc3RhbGxhdGlvbiBBYm9ydGVkfFVuaW5zdGFsbCB3YXMgbm90IGNvbXBsZXRlZCBzdWNjZXNzZnVsbHl8aGFzIGJlZW4gdW5pbnN0YWxsZWR8Q2xpY2sgRmluaXNoIHRvIGNsb3NlIFVuaW5zdGFsbCknKSB7CiAgICAgIEludm9rZS1OYXRpdmVUZXJtaW5hbCAkUGhhc2UgJFdpbmRvdyAkbnVsbCAndGVybWluYWwtd2luZG93LWZhbGxiYWNrJwogICAgICByZXR1cm4gJ0Nsb3NlJwogICAgfQogIH0KICByZXR1cm4gJG51bGwKfQo=')

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
