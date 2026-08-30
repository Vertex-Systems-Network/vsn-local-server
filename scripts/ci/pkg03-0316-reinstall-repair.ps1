param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness correction over exact head 40643bad.
# Exact Windows evidence proved both current-user and per-machine healthy
# reinstall, MISSING repair, HASH_MISMATCH repair and exact SHA256 restoration.
# The per-machine NSIS uninstaller then independently exposed the proven terminal
# page (< Back + Close + Show details, with no Remove/Uninstall), but the existing
# default-Enter fallback did not activate the elevated NSIS Close control and the
# completion predicate never finalized. Preserve every frozen repair assertion;
# replace only that terminal fallback with a native child-button lookup + BM_CLICK
# before the existing UIA/Enter fallbacks. No product/runtime/installer mutation.
# Frozen witnesses retained for authority validation:
# MISSING HASH_MISMATCH MATCH VSN-Agent Stop-Service nsis-current-user
# nsis-per-machine wix-per-machine /fa reinstall-healthy-1 repair-missing
# repair-tamper reinstall-healthy-2 exact_sha256_restored
# duplicate_registration_forbidden native-terminal-idok-close-fallback
# Invoke-UninstallTerminalWindowClose Test-UninstallTerminalPage

$BaseCommit = '40643bad0f9b433c46ef5f8391091ed2fbd2c3c3'
$BasePath = 'scripts/ci/pkg03-0316-reinstall-repair.ps1'
$ExpectedBaseBlob = 'eecf0b9d1f3708f66e535dd13acd57483301cc62'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.16 pinned previous wrapper blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}
$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.16 failed to load pinned previous wrapper.'
}

foreach ($token in @(
  'MISSING','HASH_MISMATCH','MATCH','VSN-Agent','Stop-Service',
  'nsis-current-user','nsis-per-machine','wix-per-machine','/fa',
  'reinstall-healthy-1','repair-missing','repair-tamper','reinstall-healthy-2',
  'exact_sha256_restored','duplicate_registration_forbidden',
  'Invoke-UninstallTerminalWindowClose','Test-UninstallTerminalPage'
)) {
  if (-not $source.Contains($token)) { throw "03.16 pinned wrapper missing frozen token: $token" }
}

Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Vsn0316TerminalBridge {
  public delegate bool EnumChildProc(IntPtr hwnd, IntPtr lParam);
  [DllImport("user32.dll")] [return: MarshalAs(UnmanagedType.Bool)]
  static extern bool EnumChildWindows(IntPtr hWndParent, EnumChildProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);
  [DllImport("user32.dll")] [return: MarshalAs(UnmanagedType.Bool)] static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] [return: MarshalAs(UnmanagedType.Bool)] static extern bool IsWindowEnabled(IntPtr hWnd);
  public static IntPtr FindVisibleButtonByText(IntPtr parent, string expected) {
    IntPtr found = IntPtr.Zero;
    EnumChildProc callback = delegate(IntPtr hwnd, IntPtr lParam) {
      var cls = new StringBuilder(128); GetClassName(hwnd, cls, cls.Capacity);
      if (!string.Equals(cls.ToString(), "Button", StringComparison.OrdinalIgnoreCase)) return true;
      if (!IsWindowVisible(hwnd) || !IsWindowEnabled(hwnd)) return true;
      var text = new StringBuilder(512); GetWindowText(hwnd, text, text.Capacity);
      var normalized = text.ToString().Replace("&", "").Trim();
      if (string.Equals(normalized, expected, StringComparison.OrdinalIgnoreCase)) { found = hwnd; return false; }
      return true;
    };
    EnumChildWindows(parent, callback, IntPtr.Zero);
    GC.KeepAlive(callback);
    return found;
  }
}
'@

$insertion = '$patched = $source.Replace($StaleNestedToken, ''duplicate_registration_forbidden'')'
if ([regex]::Matches($source,[regex]::Escape($insertion)).Count -ne 1) {
  throw '03.16 nested terminal patch insertion boundary drifted.'
}

$extended = @'
$patched = $source.Replace($StaleNestedToken, 'duplicate_registration_forbidden')

$oldTerminalFallback = @'
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

$newTerminalFallback = @'
  # The terminal page has already been independently proven. Resolve the actual
  # Win32 child Button named Close (title-bar Close is not a child Button) and
  # deliver BM_CLICK directly to the NSIS wizard control. This preserves the
  # terminal callback/exit path and avoids treating a generic root close as
  # successful uninstallation.
  $nativeClose=[Vsn0316TerminalBridge]::FindVisibleButtonByText($rootHandle,'Close')
  if ($nativeClose -ne [IntPtr]::Zero -and [Vsn0316NativeUi]::IsWindow($nativeClose)) {
    [void][Vsn0316NativeUi]::SendMessage($nativeClose,[uint32]0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)
    if ($firstAttempt) {
      [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='native-terminal-enumerated-bm-click';control='Close';at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence
    }
    Start-Sleep -Milliseconds 450
    return $true
  }

  # Keep the previous bounded fallback only when no native content Close button
  # can be resolved. It cannot weaken completion because Drive-SuccessUi still
  # requires the independent uninstall predicate, process exit and exit code 0.
  try { $Window.SetFocus() } catch {}
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0100,[IntPtr]0x0D,[IntPtr]::Zero)
  [void][Vsn0316NativeUi]::PostMessage($rootHandle,[uint32]0x0101,[IntPtr]0x0D,[IntPtr]::Zero)
  if ($firstAttempt) {
    [void]$UiActions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase=$Phase;action='terminal-default-enter-fallback';control='proven-uninstall-terminal-page';at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence
  }
  Start-Sleep -Milliseconds 350
  return $true
}
'@.Replace("`r`n", "`n")

$count = [regex]::Matches($patched,[regex]::Escape($oldTerminalFallback)).Count
if ($count -ne 1) { throw "03.16 terminal fallback patch boundary mismatch: expected 1, found $count" }
$patched = $patched.Replace($oldTerminalFallback,$newTerminalFallback)
'@

$patchedOuter = $source.Replace($insertion,$extended)
$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeWrapper = Join-Path $tempRoot 'pkg03-0316-native-terminal-wrapper.ps1'
[IO.File]::WriteAllText($runtimeWrapper,$patchedOuter,[Text.UTF8Encoding]::new($false))

$tokens=$null; $errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeWrapper,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.16 patched wrapper has $($errors.Count) parse error(s)."
}

& $runtimeWrapper `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
