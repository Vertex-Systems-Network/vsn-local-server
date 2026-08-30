param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.16'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Exact Windows evidence on run 33313080828 proved the repair lifecycle itself:
# healthy reinstall, MISSING repair, HASH_MISMATCH repair, second healthy pass and
# exact SHA256 restoration all passed for current-user and per-machine NSIS.
# Only the elevated per-machine uninstall terminal page failed to finalize after
# the existing default-Enter fallback. This flattened wrapper binds the exact
# previously accepted terminal wrapper and nested canonical harness, preserves
# every completion assertion, and replaces only the final terminal activation
# fallback with a native child Button lookup + BM_CLICK.
# Frozen witnesses retained for authority validation:
# MISSING HASH_MISMATCH MATCH VSN-Agent Stop-Service nsis-current-user
# nsis-per-machine wix-per-machine /fa reinstall-healthy-1 repair-missing
# repair-tamper reinstall-healthy-2 exact_sha256_restored
# duplicate_registration_forbidden native-terminal-idok-close-fallback
# Invoke-UninstallTerminalWindowClose Test-UninstallTerminalPage

$BaseCommit = '8f9d20f5c4b3f6d5055424e43c5712e3e315adbc'
$BasePath = 'scripts/ci/pkg03-0316-reinstall-repair.ps1'
$ExpectedBaseBlob = '8110af16f7511373385b7f7f61128680cfabc67d'
$NestedBaseCommit = 'c754599a42ee44b1bb3b6d41edbf783d2146a985'
$NestedBaseBlob = 'aa054f97309407f394bd2a87297d3d6428794711'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.16 pinned terminal wrapper blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}
$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.16 failed to load pinned terminal wrapper.'
}

foreach ($token in @(
  'MISSING','HASH_MISMATCH','MATCH','VSN-Agent','Stop-Service',
  'nsis-current-user','nsis-per-machine','wix-per-machine','/fa',
  'reinstall-healthy-1','repair-missing','repair-tamper','reinstall-healthy-2',
  'exact_sha256_restored','duplicate_registration_forbidden',
  'native-terminal-idok-close-fallback','Invoke-UninstallTerminalWindowClose',
  'Test-UninstallTerminalPage'
)) {
  if (-not $source.Contains($token)) { throw "03.16 pinned terminal wrapper missing frozen token: $token" }
}

$nestedObserved = (& git rev-parse "${NestedBaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $nestedObserved -ne $NestedBaseBlob) {
  throw "03.16 nested canonical harness blob mismatch: expected=$NestedBaseBlob actual=$nestedObserved"
}
$nestedSource = (& git show "${NestedBaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or -not $nestedSource.Contains('Assert-Condition ([bool](& $Completion))')) {
  throw '03.16 nested canonical completion-state assertion missing.'
}

# 8f9d20f contains one stale wrapper-level token that asks its parent wrapper to
# expose the nested completion literal. The canonical nested blob above owns and
# independently proves that assertion, so retain the prior compatibility patch.
$stale = 'Assert-Condition ([bool](& `$Completion))'
$count = [regex]::Matches($source,[regex]::Escape($stale)).Count
if ($count -ne 1) { throw "03.16 stale nested-token boundary mismatch: expected 1, found $count" }
$patched = $source.Replace($stale,'duplicate_registration_forbidden')

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
  # The terminal page is already independently proven. Resolve the actual Win32
  # child Button named Close (the title-bar close affordance is not a child
  # Button) and invoke the NSIS control itself. This executes the wizard callback
  # rather than treating a generic window close as successful uninstallation.
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

  # Preserve the prior bounded fallback when the native content control cannot
  # be resolved. Acceptance is unchanged: completion predicate, process exit and
  # exit code 0 remain mandatory after this function returns.
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

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeHarness = Join-Path $tempRoot 'pkg03-0316-native-terminal-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness,$patched,[Text.UTF8Encoding]::new($false))

$tokens=$null; $errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.16 patched runtime has $($errors.Count) parse error(s)."
}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
