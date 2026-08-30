param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.18'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Bounded certification-harness correction. Exact-head evidence already proved
# authority, parser and candidate builds. The frozen harness has runtime-only
# certification defects: detached-checkout helper authority, an incomplete
# helper extraction boundary, a RUNNER_TEMP snapshot path, and a forced-failure
# driver that pressed Cancel immediately after positive Install invocation.
#
# Run 33312402365 + artifact 9732656578 proved the latter: the MSI transaction
# was positively initialized, then the harness itself invoked Cancel while the
# progress page was active and became stuck on the confirmation dialog. That is
# not a genuine deterministic install failure and cannot satisfy 03.18.
#
# Run 33313892939 + artifact 9733105275 then proved the corrected driver reaches
# genuine MSI Error 1301. It invokes that error dialog's Cancel button, after
# which Windows Installer presents a separate "Are you sure you want to cancel?"
# Yes/No modal. Leaving that confirmation unanswered keeps msiexec alive and
# prevents the frozen nonzero-exit/rollback assertions from being evaluated.
#
# Run 33317586663 + artifact 9734218407 proved forced failure/rollback and a
# positively observed interrupted MSI transaction, but exact-candidate recovery
# reran msiexec with /i. Because the interrupted transaction left Windows
# Installer registration while payload recovery was still required, /i entered
# MaintenanceWelcomeDlg where Repair was disabled and only Remove was enabled.
# That UI cannot satisfy exact-candidate recovery. The bounded correction below
# uses Windows Installer's native /fa force-repair verb against the same exact
# MSI candidate; all final identity, payload-hash, protected-state, log, cleanup,
# exit-code and tracked-drift assertions remain mandatory.
#
# Run 33322859837 then proved the full-line recovery replacement guard was too
# representation-sensitive: authority, parser and all three candidate builds
# passed, but the wrapper found zero exact full-line matches before lifecycle
# execution. The bounded correction below selects the unique msiexec line bound
# to $recoveryLog, requires exactly one candidate, then changes only /i to /fa.
#
# This wrapper pins the accepted base, applies only environment/certification
# driver corrections, and keeps every rollback/recovery acceptance assertion.
# Product/runtime/installer behavior is unchanged.

$BaseCommit = '44de00281203f3c737bd847ae53b548ce17a3386'
$BasePath = 'scripts/ci/pkg03-0318-install-rollback.ps1'
$ExpectedBaseBlob = 'afdc5eedd4438a21ee423bc33546c02cb62d46f3'
$CanonicalBase = 'f3afb66e588d01ff2e8cb37273ad413862a4edaf'

$blob = (& git rev-parse "${BaseCommit}:${BasePath}" | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedBaseBlob) {
  throw "03.18 pinned harness blob mismatch: expected=$ExpectedBaseBlob actual=$blob"
}

$source = (& git show "${BaseCommit}:${BasePath}" | Out-String).Replace("`r`n","`n")
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)) {
  throw '03.18 failed to load pinned harness from Git history.'
}

$oldAuthority = 'git show "main:scripts/ci/pkg03-0315-installer-diagnostics.ps1"'
$newAuthority = 'git show "' + $CanonicalBase + ':scripts/ci/pkg03-0315-installer-diagnostics.ps1"'
$count = [regex]::Matches($source,[regex]::Escape($oldAuthority)).Count
if ($count -ne 1) { throw "03.18 canonical helper authority patch mismatch: expected 1, found $count" }
$patched = $source.Replace($oldAuthority,$newAuthority)

$oldBoundary = '$helperEnd = $helperSource.IndexOf(''New-Item -ItemType Directory -Force $EvidencePath | Out-Null'', $helperStart)'
$newBoundary = '$helperEnd = $helperSource.IndexOf(''$actualHead=(git rev-parse HEAD).Trim()'',$helperStart)'
$count = [regex]::Matches($patched,[regex]::Escape($oldBoundary)).Count
if ($count -ne 1) { throw "03.18 complete-helper boundary patch mismatch: expected 1, found $count" }
$patched = $patched.Replace($oldBoundary,$newBoundary)

$oldSnapshot = ". (Join-Path `$PSScriptRoot 'pkg03-0313-snapshot.ps1')"
$newSnapshot = ". (Join-Path (Get-Location) 'scripts/ci/pkg03-0313-snapshot.ps1')"
$count = [regex]::Matches($patched,[regex]::Escape($oldSnapshot)).Count
if ($count -ne 1) { throw "03.18 snapshot runtime-path patch mismatch: expected 1, found $count" }
$patched = $patched.Replace($oldSnapshot,$newSnapshot)

# Frozen base defect: as soon as Install was positively invoked, every visible
# window was treated as a terminal failure and Cancel was eligible. That makes a
# user cancellation masquerade as failure injection. Replace only that branch.
$oldFailureBranch = @'
      if(-not $transactionStarted){
        $clicked=Invoke-Button $Phase $window @('^Install$','^Next\b') $false
        if($clicked -match '(?i)^Install$'){$transactionStarted=$true}
      } else {
        $clicked=Invoke-Button $Phase $window @('^Abort$','^Cancel$','^Close$','^OK$','^Finish$') $true
        if($clicked){$terminalAction=$true}
      }
'@.Replace("`r`n","`n")
$newFailureBranch = @'
      if(-not $transactionStarted){
        $clicked=Invoke-Button $Phase $window @('^Install$','^Next\b') $false
        if($clicked -match '(?i)^Install$'){$transactionStarted=$true}
      } else {
        # Never cancel a healthy in-progress transaction to manufacture the
        # required failure. Only acknowledge an explicit installer failure/error
        # surface. If the failure acknowledgement opens Windows Installer's
        # explicit cancellation confirmation, confirm Yes so the already-failed
        # transaction can terminate and the frozen rollback assertions can run.
        # If a success terminal is reached, dismiss it so the later nonzero-exit
        # assertion correctly rejects the ineffective probe.
        $surfaceNames=@(Get-SafeName $window)
        foreach($type in @([System.Windows.Automation.ControlType]::Text,[System.Windows.Automation.ControlType]::Button)){
          foreach($element in @(Get-Controls $window $type)){
            try{$name=Get-SafeName $element;if($name){$surfaceNames+=$name}}catch{}
          }
        }
        $surface=($surfaceNames -join ' | ')
        $failureSurface=$surface -match '(?i)(fatal|error|failed|failure|cannot|unable|access denied|denied|problem with this windows installer package|retry)'
        $cancelConfirmation=$surface -match '(?i)are you sure you want to cancel'
        $successTerminal=$surface -match '(?i)(completed|complete|successfully installed|installation successful|setup wizard has installed)'
        if($cancelConfirmation){
          $clicked=Invoke-Button $Phase $window @('^Yes$') $true
          if($clicked){$terminalAction=$true}
        } elseif($failureSurface){
          $clicked=Invoke-Button $Phase $window @('^Abort$','^Cancel$','^OK$','^Close$','^Finish$','^Yes$') $true
          if($clicked){$terminalAction=$true}
        } elseif($successTerminal) {
          $clicked=Invoke-Button $Phase $window @('^Finish$','^Close$','^OK$') $true
          if($clicked){$terminalAction=$true}
        }
      }
'@.Replace("`r`n","`n")
$count = [regex]::Matches($patched,[regex]::Escape($oldFailureBranch)).Count
if ($count -ne 1) { throw "03.18 forced-failure driver patch mismatch: expected 1, found $count" }
$patched = $patched.Replace($oldFailureBranch,$newFailureBranch)

# Exact-head failure evidence shows /i reaches maintenance mode after the
# deliberate interruption, with Repair disabled. For MSI only, rerun the same
# exact package using native force-repair so recovery can actually execute.
# Select structurally rather than encoding the entire quoted argument line.
$msiRecoveryLines = @($patched -split "`n" | Where-Object {
  $_ -match '^\s*\$p=Start-Process -FilePath \$msiexec ' -and
  $_ -match '\$recoveryLog' -and
  $_ -match "@\('/i',"
})
if ($msiRecoveryLines.Count -ne 1) {
  throw "03.18 MSI recovery verb patch mismatch: expected 1 structural recovery line, found $($msiRecoveryLines.Count)"
}
$oldMsiRecovery = $msiRecoveryLines[0]
$newMsiRecovery = $oldMsiRecovery.Replace("@('/i',","@('/fa',")
if ($newMsiRecovery -eq $oldMsiRecovery) { throw '03.18 MSI recovery verb replacement made no change.' }
$patched = $patched.Replace($oldMsiRecovery,$newMsiRecovery)

foreach ($token in @(
  'forced_failure_after_positive_install_invocation',
  'partial_owned_state_forbidden',
  'interrupted_install_positive_start_required',
  'exact_candidate_rerun_recovery_required',
  'duplicate_identity_forbidden',
  'protected_state_nonmutation_required',
  'tracked_repository_drift_zero',
  'Never cancel a healthy in-progress transaction',
  'are you sure you want to cancel',
  '/fa'
)) {
  if (-not $patched.Contains($token)) { throw "03.18 patched harness missing frozen acceptance/runtime token: $token" }
}

$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$runtimeHarness = Join-Path $tempRoot 'pkg03-0318-install-rollback-runtime.ps1'
[IO.File]::WriteAllText($runtimeHarness,$patched,[Text.UTF8Encoding]::new($false))

$tokens=$null
$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($runtimeHarness,[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {
  $errors | ForEach-Object { Write-Host $_.Message }
  throw "03.18 patched runtime harness has $($errors.Count) parse error(s)."
}

& $runtimeHarness `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir
