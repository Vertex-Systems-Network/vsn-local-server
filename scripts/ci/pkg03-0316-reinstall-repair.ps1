param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness repair.
#
# The exact accepted 03.16 harness source is retained in immutable Git history
# and pinned below by both commit and Git blob SHA. This shim rehydrates those
# exact bytes, replaces only the positively identified NSIS uninstall terminal
# dismissal helper, parses the resulting PowerShell, and executes it with the
# original parameters. Product/runtime/installer behavior and all acceptance
# assertions remain in the pinned base harness.
#
# Windows evidence showed that the uninstall terminal page can expose a Close
# control that is not UIA-invokable under the per-machine lifecycle. The helper
# therefore attempts the real UIA Close first, then posts the dialog-standard
# IDOK command and WM_CLOSE to the already positively identified terminal page.
# Native dismissal is retried on subsequent observations; TerminalRoots is used
# only to deduplicate evidence recording, never to suppress a required retry.

$BaseCommit = 'c754599a42ee44b1bb3b6d41edbf783d2146a985'
$BasePath = 'scripts/ci/pkg03-0316-reinstall-repair.ps1'
$ExpectedBaseBlob = 'aa054f97309407f394bd2a87297d3d6428794711'

$RequiredFrozenTokens = @(
  'MISSING',
  'HASH_MISMATCH',
  'MATCH',
  'VSN-Agent',
  'Stop-Service',
  'nsis-current-user',
  'nsis-per-machine',
  'wix-per-machine',
  '/fa',
  'reinstall-healthy-1',
  'repair-missing',
  'repair-tamper',
  'reinstall-healthy-2',
  'exact_sha256_restored',
  'duplicate_registration_forbidden'
)

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.16 pinned base harness blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}

$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.16 failed to load pinned base harness from Git history.'
}
foreach ($token in $RequiredFrozenTokens) {
  if (-not $source.Contains($token)) {
    throw "03.16 pinned base harness missing frozen token: $token"
  }
}

$old = @'
function Invoke-UninstallTerminalWindowClose([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $rootHandle=[IntPtr]::Zero
  try { $rootHandle=[IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0316NativeUi]::IsWindow($rootHandle)) { return $false }
  $key="${Lifecycle}:${Phase}:terminal-window:$($rootHandle.ToInt64())"
  if (-not $TerminalRoots.Add($key)) { return $true }
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
  [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='native-terminal-window-close';control='proven-uninstall-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiEvidence
  return $true
}
'@.Replace("`r`n", "`n")

$new = @'
function Invoke-UninstallTerminalWindowClose([string]$Lifecycle,[string]$Phase,[System.Windows.Automation.AutomationElement]$Window) {
  $closeCandidates=@()
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name=Get-SafeName $button
      $normalized=($name -replace '&','').Trim()
      $automationId=[string]$button.Current.AutomationId
      $nativeHandle=[int]$button.Current.NativeWindowHandle
      if ($normalized -ne 'Close') { continue }
      if ($nativeHandle -eq 0 -and $automationId -match '^(?i:Close|Minimize|Maximize)$') { continue }
      $closeCandidates += [pscustomobject]@{element=$button;name=$name}
    } catch {}
  }
  foreach ($candidate in $closeCandidates) {
    try {
      $invoke=[System.Windows.Automation.InvokePattern]$candidate.element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
      $invoke.Invoke()
      [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='invoke-terminal-close-button';control=$candidate.name;at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence
      Start-Sleep -Milliseconds 350
      return $true
    } catch {}
  }

  $rootHandle=[IntPtr]::Zero
  try { $rootHandle=[IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0316NativeUi]::IsWindow($rootHandle)) { return $false }
  $key="${Lifecycle}:${Phase}:terminal-window:$($rootHandle.ToInt64())"
  $firstAttempt=$TerminalRoots.Add($key)

  # The terminal page has already been positively identified by
  # Test-UninstallTerminalPage. Post the dialog-standard primary command first;
  # unlike UIA InvokePattern this remains available for the elevated NSIS
  # per-machine terminal window on hosted Windows runners.
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0111,[IntPtr]1,[IntPtr]::Zero)
  Start-Sleep -Milliseconds 250
  if ([Vsn0316NativeUi]::IsWindow($rootHandle)) {
    [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
  }
  if ($firstAttempt) {
    [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='native-terminal-idok-close-fallback';control='proven-uninstall-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence
  }
  return $true
}
'@.Replace("`r`n", "`n")

$count = [regex]::Matches($source, [regex]::Escape($old)).Count
if ($count -ne 1) {
  throw "03.16 terminal helper patch boundary mismatch: expected exactly one match, found $count"
}
$patched = $source.Replace($old, $new)

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeHarness = Join-Path $tempRoot 'pkg03-0316-reinstall-repair-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness, $patched, [Text.UTF8Encoding]::new($false))

$tokens=$null
$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.16 patched runtime harness has $($errors.Count) parse error(s)."
}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
