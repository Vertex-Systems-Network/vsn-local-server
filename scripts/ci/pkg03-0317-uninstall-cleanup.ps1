param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.17'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness correction layered over the exact prior 03.17
# head. Exact Windows evidence proved the uninstall cleanup/preservation lane
# reaches the NSIS terminal page but WM_CLOSE does not execute the terminal
# Close action. Preserve every cleanup, preservation, snapshot, context, exit and
# protected-state assertion from the pinned harness; replace only terminal-page
# activation with the real command path: UIA Invoke -> native BM_CLICK -> dialog
# default Enter. Product/runtime/installer behavior is unchanged.

$BaseCommit = '1b43875914cf06f368a8483207c61b5f08bd4190'
$BasePath = 'scripts/ci/pkg03-0317-uninstall-cleanup.ps1'
$ExpectedBaseBlob = '623204eb7f63c41b769fa323a0e43742fabb741d'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.17 pinned base harness blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}

$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.17 failed to load pinned base harness from Git history.'
}

foreach ($token in @(
  'Test-Pkg0317UninstallTerminalPage',
  'Close-Pkg0317TerminalWindow',
  'Assert-RecordPreserved',
  'Assert-Pkg0313SnapshotEqual',
  'context-current-user',
  'local-service',
  'tracked_repository_drift_zero'
)) {
  if (-not $source.Contains($token)) { throw "03.17 pinned harness missing frozen token: $token" }
}

$old = @'
function Close-Pkg0317TerminalWindow([string]$Lifecycle,[System.Windows.Automation.AutomationElement]$Window) {
  $handle = [IntPtr]::Zero
  try { $handle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($handle -eq [IntPtr]::Zero -or -not [Vsn0313NativeUi]::IsWindow($handle)) { return $false }
  [void][Vsn0313NativeUi]::PostMessage($handle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
  [void]$Actions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase='uninstall';action='native-terminal-window-close';control='proven-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiArtifacts
  return $true
}
'@.Replace("`r`n", "`n")

$new = @'
function Close-Pkg0317TerminalWindow([string]$Lifecycle,[System.Windows.Automation.AutomationElement]$Window) {
  # The caller reaches this helper only after the uninstall completion predicate
  # is true. Never click Remove/Uninstall again and never destroy the root with
  # WM_CLOSE; execute the real terminal Close command so NSIS can finish.
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name = Get-SafeName $button
      if ((($name -replace '&','').Trim()) -ne 'Close') { continue }
      $automationId = [string]$button.Current.AutomationId
      $native = [IntPtr][int]$button.Current.NativeWindowHandle
      if ($native -eq [IntPtr]::Zero -and $automationId -match '^(?i:Close|Minimize|Maximize)$') { continue }

      try {
        $invoke = [System.Windows.Automation.InvokePattern]$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        [void]$Actions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase='uninstall';action='invoke-terminal-close-button';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiArtifacts
        Start-Sleep -Milliseconds 350
        return $true
      } catch {}

      if ($native -ne [IntPtr]::Zero -and [Vsn0313NativeUi]::IsWindow($native)) {
        [void][Vsn0313NativeUi]::SendMessage($native,[uint32]0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)
        [void]$Actions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase='uninstall';action='native-terminal-bm-click';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiArtifacts
        Start-Sleep -Milliseconds 350
        return $true
      }
    } catch {}
  }

  $handle = [IntPtr]::Zero
  try { $handle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($handle -eq [IntPtr]::Zero -or -not [Vsn0313NativeUi]::IsWindow($handle)) { return $false }
  try { $Window.SetFocus() } catch {}
  [void][Vsn0313NativeUi]::PostMessage($handle,[uint32]0x0100,[IntPtr]0x0D,[IntPtr]::Zero)
  [void][Vsn0313NativeUi]::PostMessage($handle,[uint32]0x0101,[IntPtr]0x0D,[IntPtr]::Zero)
  [void]$Actions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase='uninstall';action='terminal-default-enter';control='proven-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiArtifacts
  Start-Sleep -Milliseconds 350
  return $true
}
'@.Replace("`r`n", "`n")

$count = [regex]::Matches($source,[regex]::Escape($old)).Count
if ($count -ne 1) { throw "03.17 terminal helper patch boundary mismatch: expected 1, found $count" }
$patched = $source.Replace($old,$new)

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeHarness = Join-Path $tempRoot 'pkg03-0317-uninstall-cleanup-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness,$patched,[Text.UTF8Encoding]::new($false))

$tokens=$null
$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.17 patched runtime harness has $($errors.Count) parse error(s)."
}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
