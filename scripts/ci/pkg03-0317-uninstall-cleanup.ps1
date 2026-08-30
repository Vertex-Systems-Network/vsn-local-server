param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.17'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness correction over exact head ce8b1d6. Run
# 33314444710 proved authority/parser/build stages and again reached the
# per-machine NSIS uninstall terminal boundary, but its UIA/BM_CLICK/default-
# Enter helper could not finalize the elevated terminal page. Preserve the exact
# ce8b1d6 cleanup/preservation harness and inject only a native Win32 child
# Button lookup before its existing terminal fallbacks. The terminal caller still
# has to prove the destructive action is complete before this helper is entered.
# Product/runtime/installer behavior and all preservation assertions are unchanged.

$BaseCommit = 'ce8b1d6da408bf7364bbdaacb95def3b45cea27c'
$BasePath = 'scripts/ci/pkg03-0317-uninstall-cleanup.ps1'
$ExpectedBaseBlob = 'c49eecd66404e6e91dfbb9c456835e9c41f0e73a'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.17 pinned terminal wrapper blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}
$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n", "`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.17 failed to load pinned terminal wrapper.'
}
foreach ($token in @('Test-Pkg0317UninstallTerminalPage','Close-Pkg0317TerminalWindow','Assert-RecordPreserved','Assert-Pkg0313SnapshotEqual','context-current-user','local-service','tracked_repository_drift_zero')) {
  if (-not $source.Contains($token)) { throw "03.17 pinned wrapper missing frozen token: $token" }
}

Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Vsn0317TerminalBridge {
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

$anchor = '  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {'
$count = [regex]::Matches($source,[regex]::Escape($anchor)).Count
if ($count -ne 1) { throw "03.17 native terminal insertion boundary mismatch: expected 1, found $count" }

$replacement = @'
  $rootHandle = [IntPtr]::Zero
  try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch {}
  if ($rootHandle -ne [IntPtr]::Zero -and [Vsn0313NativeUi]::IsWindow($rootHandle)) {
    $nativeClose = [Vsn0317TerminalBridge]::FindVisibleButtonByText($rootHandle,'Close')
    if ($nativeClose -ne [IntPtr]::Zero -and [Vsn0313NativeUi]::IsWindow($nativeClose)) {
      [void][Vsn0313NativeUi]::SendMessage($nativeClose,[uint32]0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)
      [void]$Actions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase='uninstall';action='native-enumerated-terminal-bm-click';control='Close';at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiArtifacts
      Start-Sleep -Milliseconds 450
      return $true
    }
  }

  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
'@.Replace("`r`n", "`n").TrimEnd("`n")

$patched = $source.Replace($anchor,$replacement)
$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeWrapper = Join-Path $tempRoot 'pkg03-0317-native-terminal-wrapper.ps1'
[IO.File]::WriteAllText($runtimeWrapper,$patched,[Text.UTF8Encoding]::new($false))

$tokens=$null; $errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeWrapper,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.17 patched wrapper has $($errors.Count) parse error(s)."
}

& $runtimeWrapper `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
