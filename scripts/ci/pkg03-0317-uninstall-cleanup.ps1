param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.17'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33332101163 / artifact 9738377120 independently proved the
# current-user native content-Close path and preserved protected state exactly;
# the elevated per-machine NSIS terminal page was then proven but native child
# enumeration returned no Close HWND and the inherited default-Enter fallback did
# not finalize uninstall state. This task-local shim pins that exact head and adds
# one bounded bridge before the inherited UIA/native/default fallbacks: resolve
# the visible content Close through UIAutomation, require class Button + numeric
# AutomationId, then dispatch WM_COMMAND to the already-proven terminal root with
# that exact control id. Title-bar controls are excluded by class/id constraints.
# Cleanup ownership, dirty-data/workspace preservation, reparse containment,
# context, protected firewall/hosts/resolver/trust equality, service/ARP removal,
# exit-code and zero-drift assertions remain in the pinned harness unchanged.
# Product/runtime/installer/shared accepted helpers are untouched.

$PriorCommit='a4f817f33a8d7566b811b6cb66dad3420c4d07b3'
$PriorPath='scripts/ci/pkg03-0317-uninstall-cleanup.ps1'
$ExpectedPriorBlob='27c5c50c11e25d86c3259f6b721f9992b95ecec9'

$blob=(& git rev-parse "${PriorCommit}:${PriorPath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedPriorBlob){
  throw "03.17 prior harness blob mismatch: expected=$ExpectedPriorBlob actual=$blob"
}
$source=(& git show "${PriorCommit}:${PriorPath}"|Out-String).Replace("`r`n","`n")
if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)){throw '03.17 failed to load pinned prior harness.'}
foreach($token in @(
  'Test-Pkg0317UninstallTerminalPage','Close-Pkg0317TerminalWindow',
  'Assert-RecordPreserved','Assert-Pkg0313SnapshotEqual','context-current-user',
  'local-service','tracked_repository_drift_zero','native-enumerated-terminal-bm-click',
  'terminal-default-enter'
)){
  if(-not $source.Contains($token)){throw "03.17 pinned prior harness missing frozen/evidence token: $token"}
}

$old=@'
  }

  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
'@.Replace("`r`n","`n")
$new=@'
  }

  # Elevated NSIS can expose the real terminal content button through UIA while
  # withholding a child HWND. The numeric AutomationId is the dialog command id;
  # dispatch that exact id to the already-proven terminal root. This does not
  # guess IDOK and cannot select title-bar Close because those are not Button
  # class controls with a numeric child AutomationId.
  if ($rootHandle -ne [IntPtr]::Zero -and [Vsn0313NativeUi]::IsWindow($rootHandle)) {
    foreach ($contentButton in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
      try {
        if (-not [bool]$contentButton.Current.IsEnabled -or [bool]$contentButton.Current.IsOffscreen) { continue }
        $contentName=Get-SafeName $contentButton
        if ((($contentName -replace '&','').Trim()) -ne 'Close') { continue }
        $className=[string]$contentButton.Current.ClassName
        $automationId=[string]$contentButton.Current.AutomationId
        if ($className -ne 'Button' -or $automationId -notmatch '^\d+$') { continue }
        $commandId=[int]$automationId
        [void][Vsn0313NativeUi]::SendMessage($rootHandle,[uint32]0x0111,[IntPtr]$commandId,[IntPtr]::Zero)
        [void]$Actions.Add([pscustomobject][ordered]@{lifecycle=$Lifecycle;phase='uninstall';action='native-wm-command-uia-id-terminal-close';control=$contentName;automation_id=$automationId;at_utc=[DateTime]::UtcNow.ToString('o')})
        Write-UiArtifacts
        Start-Sleep -Milliseconds 500
        return $true
      } catch {}
    }
  }

  foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
'@.Replace("`r`n","`n")

# Scope the replacement to the generated terminal-helper insertion only. The
# prior wrapper contains exactly one native child block followed by this inherited
# UIA loop, while its anchor declaration is a single-line string and cannot match.
$count=[regex]::Matches($source,[regex]::Escape($old)).Count
if($count -ne 1){throw "03.17 UIA-control-id bridge boundary mismatch: expected 1, found $count"}
$patched=$source.Replace($old,$new)
foreach($token in @('native-wm-command-uia-id-terminal-close',"automationId -notmatch '^\d+$'",'native-enumerated-terminal-bm-click','terminal-default-enter')){
  if(-not $patched.Contains($token)){throw "03.17 terminal bridge missing token: $token"}
}

$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$runtime=Join-Path $tempRoot 'pkg03-0317-uia-command-wrapper-runtime.ps1'
[IO.File]::WriteAllText($runtime,$patched,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtime,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){$errors|ForEach-Object{Write-Host $_.Message};throw "03.17 terminal bridge wrapper has $($errors.Count) parse error(s)."}

& $runtime `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
