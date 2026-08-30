param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.19'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Bounded exact-head runtime shim for the frozen 03.19 certification harness.
# The underlying harness is tracked verbatim as a sibling .base.ps1 file and
# pinned by Git blob SHA. Only execution-environment/certification-UI defects
# are corrected:
# 1) bind the reused 03.15 helper to immutable canonical activation authority;
# 2) extract that helper at a syntactically complete boundary;
# 3) resolve the accepted 03.13 snapshot helper from repository root;
# 4) replace the frozen harness terminal override with native NSIS button
#    activation after run 33312134976 proved repeated UIA Finish invocation did
#    not exit the current-user installer; and
# 5) rename PowerShell $Pid references because $PID is read-only.
# No product/installer behavior, no harness pre-kill, and no 03.19 acceptance
# assertion is weakened.

$CanonicalBase='f3afb66e588d01ff2e8cb37273ad413862a4edaf'
$BasePath='scripts/ci/pkg03-0319-running-processes.base.ps1'
$ExpectedBaseBlob='dfd6407494d86756a9d97f1e7e605081b0299c47'

$blob=(& git rev-parse "HEAD:${BasePath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob){
  throw "03.19 pinned base harness blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}
$source=(& git show "HEAD:${BasePath}"|Out-String).Replace("`r`n","`n")
if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)){throw '03.19 failed to load pinned base harness.'}
foreach($token in @(
  'harness_pre_kill=$false',
  'installer_coordination_or_safe_block_required=$true',
  'silent_force_kill_forbidden=$true',
  'indefinite_hang_forbidden=$true',
  'msi_restart_manager_evidence_required=$true',
  'operator-cleanup-after-proven-block',
  'Restart Manager'
)){
  if(-not $source.Contains($token)){throw "03.19 pinned base harness missing frozen token: $token"}
}

$movingHelper="main:scripts/ci/pkg03-0315-installer-diagnostics.ps1"
$fixedHelper="${CanonicalBase}:scripts/ci/pkg03-0315-installer-diagnostics.ps1"
if(([regex]::Matches($source,[regex]::Escape($movingHelper))).Count -ne 1){throw '03.19 helper authority patch boundary mismatch.'}
$source=$source.Replace($movingHelper,$fixedHelper)

# The frozen base used the first New-Item($EvidencePath) occurrence as helper
# end. Accepted 03.15 contains that exact statement inside Write-UiEvidence, so
# the resulting substring ended with an open function block. The accepted
# helper section actually ends immediately before the exact-source execution
# block beginning with $actualHead. Pin to that unique execution boundary.
$oldBoundary='$helperEnd=$helperSource.IndexOf(''New-Item -ItemType Directory -Force $EvidencePath | Out-Null'',$helperStart)'
$newBoundary='$helperEnd=$helperSource.IndexOf(''$actualHead=(git rev-parse HEAD).Trim()'',$helperStart)'
if(([regex]::Matches($source,[regex]::Escape($oldBoundary))).Count -ne 1){throw '03.19 helper extraction boundary patch mismatch.'}
$source=$source.Replace($oldBoundary,$newBoundary)

$oldSnapshot=". (Join-Path `$PSScriptRoot 'pkg03-0313-snapshot.ps1')"
$newSnapshot=". (Join-Path (Get-Location) 'scripts/ci/pkg03-0313-snapshot.ps1')"
if(([regex]::Matches($source,[regex]::Escape($oldSnapshot))).Count -ne 1){throw '03.19 snapshot helper path patch boundary mismatch.'}
$source=$source.Replace($oldSnapshot,$newSnapshot)

# Exact-head failure evidence from run 33312134976 showed the current-user NSIS
# install reached a positively identified terminal page with a real enabled
# native Finish button (AutomationId 1, native HWND present). UIA InvokePattern
# was recorded repeatedly but did not execute the NSIS terminal callback, so the
# installer remained alive. Activate the actual native button/dialog command;
# never use process kill or WM_CLOSE to manufacture completion.
$oldTerminal=@'
function Invoke-NativeTerminal([string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.AutomationElement]$Button,[string]$Name){
  try{
    $invoke=[System.Windows.Automation.InvokePattern]$Button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $invoke.Invoke()
    [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='invoke-real-terminal-control';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence;Start-Sleep -Milliseconds 350;return
  }catch{}
  $root=[IntPtr]::Zero;try{$root=[IntPtr][int]$Window.Current.NativeWindowHandle}catch{return}
  if($root -ne [IntPtr]::Zero -and [Vsn0315NativeUi]::IsWindow($root)){
    [void][Vsn0315NativeUi]::PostMessage($root,[uint32]0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='native-terminal-close-fallback';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')});Write-UiEvidence
  }
}
'@.Replace("`r`n","`n")
$newTerminal=@'
function Invoke-NativeTerminal([string]$Phase,[System.Windows.Automation.AutomationElement]$Window,[System.Windows.Automation.AutomationElement]$Button,[string]$Name){
  $buttonHandle=[IntPtr]::Zero
  try{$buttonHandle=[IntPtr][int]$Button.Current.NativeWindowHandle}catch{}
  $root=[IntPtr]::Zero
  if($buttonHandle -ne [IntPtr]::Zero -and [Vsn0315NativeUi]::IsWindow($buttonHandle)){
    $root=[Vsn0315NativeUi]::GetAncestor($buttonHandle,[uint32]2)
    [void][Vsn0315NativeUi]::SendMessage($buttonHandle,[uint32]0x00F5,[IntPtr]::Zero,[IntPtr]::Zero)
    [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='native-terminal-bm-click';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')})
    Write-UiEvidence;Start-Sleep -Milliseconds 350
    if($root -ne [IntPtr]::Zero -and -not [Vsn0315NativeUi]::IsWindow($root)){return}
  }
  if($root -eq [IntPtr]::Zero){try{$root=[IntPtr][int]$Window.Current.NativeWindowHandle}catch{return}}
  if($root -eq [IntPtr]::Zero -or -not [Vsn0315NativeUi]::IsWindow($root)){return}
  if($buttonHandle -ne [IntPtr]::Zero -and [Vsn0315NativeUi]::IsWindow($buttonHandle)){
    $controlId=[Vsn0315NativeUi]::GetDlgCtrlID($buttonHandle)
    if($controlId -gt 0){
      [void][Vsn0315NativeUi]::SendMessage($root,[uint32]0x0111,[IntPtr]$controlId,$buttonHandle)
      [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='native-terminal-wm-command';control=$Name;control_id=$controlId;at_utc=[DateTime]::UtcNow.ToString('o')})
      Write-UiEvidence;Start-Sleep -Milliseconds 350
      if(-not [Vsn0315NativeUi]::IsWindow($root)){return}
    }
  }
  try{$Window.SetFocus()}catch{}
  [void][Vsn0315NativeUi]::PostMessage($root,[uint32]0x0100,[IntPtr]0x0D,[IntPtr]::Zero)
  [void][Vsn0315NativeUi]::PostMessage($root,[uint32]0x0101,[IntPtr]0x0D,[IntPtr]::Zero)
  [void]$Actions.Add([pscustomobject][ordered]@{phase=$Phase;action='terminal-default-enter';control=$Name;at_utc=[DateTime]::UtcNow.ToString('o')})
  Write-UiEvidence;Start-Sleep -Milliseconds 350
}
'@.Replace("`r`n","`n")
if(([regex]::Matches($source,[regex]::Escape($oldTerminal))).Count -ne 1){throw '03.19 terminal helper patch boundary mismatch.'}
$source=$source.Replace($oldTerminal,$newTerminal)

$pidMatches=[regex]::Matches($source,'(?i)\$pid\b').Count
if($pidMatches -lt 4){throw "03.19 expected multiple `$Pid references, found $pidMatches"}
$source=[regex]::Replace($source,'(?i)\$pid\b','$ProcessId')
if([regex]::IsMatch($source,'(?i)\$pid\b')){throw '03.19 runtime harness still contains a reserved $PID variable reference.'}

$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$runtimeHarness=Join-Path $tempRoot 'pkg03-0319-running-processes-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness,$source,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){$errors|ForEach-Object{Write-Host $_.Message};throw "03.19 runtime harness has $($errors.Count) parse error(s)."}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
