param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness repair layered over the exact previous head.
#
# Evidence from exact head e30870f proved that repair semantics and all owned
# payload hashes are correct, but the elevated per-machine NSIS terminal page
# did not complete when the harness posted WM_CLOSE. WM_CLOSE can destroy the
# window without invoking the NSIS terminal Close action/callback, so this shim
# removes that path. It preserves the positive terminal-page detector and every
# acceptance assertion, tries the real UIA Close first (from the pinned source),
# then activates the real native Close control with BM_CLICK when available, and
# finally uses the dialog default Enter action for elevated UIA boundaries.
# No product/runtime/installer behavior or acceptance criteria are changed.

$BaseCommit = 'e30870f65b12deb7b762fd8c1478cffb076c87f4'
$BasePath = 'scripts/ci/pkg03-0316-reinstall-repair.ps1'
$ExpectedBaseBlob = '3130da105fad4d9b7fa94008deb09a889d8d68b5'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.16 previous harness blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}

$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.16 failed to load the pinned previous harness.'
}

$required = @(
  "native-terminal-idok-close-fallback",
  "Invoke-UninstallTerminalWindowClose",
  "Test-UninstallTerminalPage",
  "Assert-Condition ([bool](& `$Completion))",
  "exact_sha256_restored",
  "duplicate_registration_forbidden"
)
foreach ($token in $required) {
  if (-not $source.Contains($token)) { throw "03.16 pinned previous harness missing token: $token" }
}

$old = @'
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

$new = @'
  $rootHandle=[IntPtr]::Zero
  try { $rootHandle=[IntPtr][int]$Window.Current.NativeWindowHandle } catch { return $false }
  if ($rootHandle -eq [IntPtr]::Zero -or -not [Vsn0316NativeUi]::IsWindow($rootHandle)) { return $false }
  $key="${Lifecycle}:${Phase}:terminal-activation:$($rootHandle.ToInt64())"
  $firstAttempt=$TerminalRoots.Add($key)

  # Do not use WM_CLOSE here. The terminal page is already positively identified
  # and must execute the NSIS Close action, not merely destroy the top-level HWND.
  # First try a native BM_CLICK on a real visible Close button. Title-bar controls
  # exposed by UIA have no usable native HWND and are intentionally ignored.
  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
    try {
      if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
      $name=Get-SafeName $button
      if ((($name -replace '&','').Trim()) -ne 'Close') { continue }
      $buttonHandle=[IntPtr][int]$button.Current.NativeWindowHandle
      if ($buttonHandle -eq [IntPtr]::Zero -or -not [Vsn0316NativeUi]::IsWindow($buttonHandle)) { continue }
      [void][Vsn0316NativeUi]::SendMessage($buttonHandle,[uint32]0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)
      if ($firstAttempt) {
        [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='native-terminal-bm-click';control=$name;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiEvidence
      }
      Start-Sleep -Milliseconds 350
      return $true
    } catch {}
  }

  # Elevated NSIS can expose the real terminal button without an invokable UIA
  # pattern/native child HWND. Activate the dialog default button through Enter;
  # this routes through the dialog/NSIS command path rather than WM_CLOSE.
  try { $Window.SetFocus() } catch {}
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0100,[IntPtr]0x0D,[IntPtr]::Zero)
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0101,[IntPtr]0x0D,[IntPtr]::Zero)
  if ($firstAttempt) {
    [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='terminal-default-enter';control='proven-uninstall-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence
  }
  Start-Sleep -Milliseconds 350
  return $true
}
'@.Replace("`r`n", "`n")

$count = [regex]::Matches($source, [regex]::Escape($old)).Count
if ($count -ne 1) {
  throw "03.16 terminal activation patch boundary mismatch: expected exactly one match, found $count"
}
$patched = $source.Replace($old, $new)

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeHarness = Join-Path $tempRoot 'pkg03-0316-reinstall-repair-terminal-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness, $patched, [Text.UTF8Encoding]::new($false))

$tokens=$null
$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.16 patched terminal runtime has $($errors.Count) parse error(s)."
}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
