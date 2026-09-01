param(
  [Parameter(Mandatory=$true)][string]$CurrentUserNsisPath,
  [Parameter(Mandatory=$true)][string]$PerMachineNsisPath,
  [Parameter(Mandatory=$true)][string]$MsiPath,
  [Parameter(Mandatory=$true)][string]$SourceSha,
  [string]$EvidenceDir = 'dist-pkg03/03.18'
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

# Exact-head run 33335548668 / job 99321680359 / independently verified
# artifact 9739305919 isolated one certification defect in the current-user NSIS
# interrupted-install lane. NSIS exposes no literal Install control on this path:
# the driver invoked three real Next actions, package-created state reached the
# frozen PositiveStart predicate, and the UI reached Finish. The inherited driver
# evaluated PositiveStart only behind $installInvoked, so it ignored that stronger
# runtime witness and looped until timeout.
#
# Exact-head run 33449836653 / job 99677006909 then proved the first bounded
# correction targeted the intermediate wrapper source one layer too early: all
# authority/parser/build gates passed, but lifecycle stopped before execution at
# the wrapper's own interrupted positive-start gate self-check. The actual
# Invoke-InterruptedInstall gate materializes only when the pinned nested wrapper
# generates the final base harness.
#
# Exact-head run 33451474269 / job 99682109502 proved the final-layer target was
# correct, but the nested interpolated here-string used to transport that patch
# produced an invalid outer runtime wrapper before lifecycle execution. Keep the
# same final-harness correction and acceptance semantics, but transport the exact
# inner patch as deterministic UTF-8 Base64 so PowerShell performs no additional
# nested here-string interpolation/quoting at the outer wrapper layer.
#
# Exact-head run 33531247397 / job 99934662427 then proved the Base64 payload
# itself is valid, but the generated outer wrapper still joined the injected block
# directly to the following $tempRoot anchor without a guaranteed statement
# separator. Preserve the exact patch and force deterministic LF boundaries at
# both generated insertion points.
#
# MSI retains the literal Install requirement. Interruption remains possible only
# after exact-candidate owned payload/ARP transaction-start proof. Product/runtime/
# installer behavior remains untouched.

$PriorCommit='4cba4bbb8ec5217f7a8767f17bc85e86a272bba7'
$PriorPath='scripts/ci/pkg03-0318-install-rollback.ps1'
$ExpectedPriorBlob='6cb964a09390a2d0889e77bcf1cf88408c35b444'

$blob=(& git rev-parse "${PriorCommit}:${PriorPath}"|Out-String).Trim()
if($LASTEXITCODE -ne 0 -or $blob -ne $ExpectedPriorBlob){
  throw "03.18 prior harness blob mismatch: expected=$ExpectedPriorBlob actual=$blob"
}
$source=(& git show "${PriorCommit}:${PriorPath}"|Out-String).Replace("`r`n","`n")
if($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($source)){throw '03.18 failed to load pinned prior harness.'}
foreach($token in @(
  'forced_failure_after_positive_install_invocation','partial_owned_state_forbidden',
  'interrupted_install_positive_start_required','exact_candidate_rerun_recovery_required',
  'duplicate_identity_forbidden','protected_state_nonmutation_required',
  'tracked_repository_drift_zero','/fa',
  'runner-isolation-security-reset-after-successful-machine-lifecycle',
  'failed_attempt_residue=$false','positive-transaction-start-target-write-attempt'
)){
  if(-not $source.Contains($token)){throw "03.18 pinned prior harness missing token: $token"}
}

# The pinned 4cba wrapper first generates an intermediate 011b wrapper in
# $patched. That intermediate wrapper then generates the final 44de base harness,
# where Invoke-InterruptedInstall and its PositiveStart gate actually exist.
# Insert a deterministic decoded source payload into the intermediate wrapper
# immediately before it emits the final runtime harness. Avoid a nested expandable
# here-string: run 33451474269 proved that representation can corrupt the outer
# wrapper before the final harness is even parsed.
$outerAnchor='$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}'
if(([regex]::Matches($source,[regex]::Escape($outerAnchor))).Count -ne 1){
  throw '03.18 interrupted-start outer insertion boundary mismatch.'
}
$outerInsertion=@'
$innerAnchor='$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }'
if(([regex]::Matches($patched,[regex]::Escape($innerAnchor))).Count -ne 1){
  throw '03.18 interrupted-start final-harness insertion boundary mismatch.'
}
$innerPayloadB64='JG9sZEludGVycnVwdGVkR2F0ZT0nICAgIGlmKCRpbnN0YWxsSW52b2tlZCAtYW5kIFtib29sXSgmICRQb3NpdGl2ZVN0YXJ0KSl7JHBvc2l0aXZlPSR0cnVlO2JyZWFrfScKaWYoKFtyZWdleF06Ok1hdGNoZXMoJHBhdGNoZWQsW3JlZ2V4XTo6RXNjYXBlKCRvbGRJbnRlcnJ1cHRlZEdhdGUpKSkuQ291bnQgLW5lIDEpewogIHRocm93ICcwMy4xOCBpbnRlcnJ1cHRlZCBwb3NpdGl2ZS1zdGFydCBnYXRlIG1pc21hdGNoLicKfQokbmV3SW50ZXJydXB0ZWRHYXRlPUAoCicgICAgJHBvc2l0aXZlTm93PVtib29sXSgmICRQb3NpdGl2ZVN0YXJ0KScsCicgICAgaWYoJGluc3RhbGxJbnZva2VkIC1hbmQgJHBvc2l0aXZlTm93KXskcG9zaXRpdmU9JHRydWU7YnJlYWt9JywKJyAgICBpZigoLW5vdCAkaW5zdGFsbEludm9rZWQpIC1hbmQgKCRQaGFzZSAtbWF0Y2ggJycoP2kpXm5zaXMtJycpIC1hbmQgJHBvc2l0aXZlTm93KXsnLAonICAgICAgIyBOU0lTIG1heSBleGVjdXRlIHRoZSBvd25lZCB3cml0ZSB0cmFuc2FjdGlvbiBmcm9tIGl0cyBmaW5hbCBOZXh0IGFjdGlvbicsCicgICAgICAjIHdpdGhvdXQgZXZlciBleHBvc2luZyBhIGxpdGVyYWwgSW5zdGFsbCBidXR0b24uIFRoZSBmcm96ZW4gUG9zaXRpdmVTdGFydCcsCicgICAgICAjIHByZWRpY2F0ZSBvYnNlcnZlcyBwYWNrYWdlLWNyZWF0ZWQgb3duZWQgcGF5bG9hZC9BUlAgc3RhdGUuJywKJyAgICAgICRpbnN0YWxsSW52b2tlZD0kdHJ1ZScsCicgICAgICAkcG9zaXRpdmU9JHRydWUnLAonICAgICAgW3ZvaWRdJEFjdGlvbnMuQWRkKFtwc2N1c3RvbW9iamVjdF1bb3JkZXJlZF1AeycsCicgICAgICAgIHBoYXNlPSRQaGFzZScsCicgICAgICAgIGFjdGlvbj0nJ3Bvc2l0aXZlLXRyYW5zYWN0aW9uLXN0YXJ0LXdpdGhvdXQtbGl0ZXJhbC1pbnN0YWxsLWNvbnRyb2wnJycsCicgICAgICAgIHByb29mPScnZnJvemVuLXBvc2l0aXZlLXN0YXJ0LW93bmVkLXBheWxvYWQtb3ItYXJwJycnLAonICAgICAgICBhdF91dGM9W0RhdGVUaW1lXTo6VXRjTm93LlRvU3RyaW5nKCcnbycnKScsCicgICAgICB9KScsCicgICAgICBXcml0ZS1VaUV2aWRlbmNlJywKJyAgICAgIGJyZWFrJywKJyAgICB9JwopIC1qb2luIFtjaGFyXTEwCiRwYXRjaGVkPSRwYXRjaGVkLlJlcGxhY2UoJG9sZEludGVycnVwdGVkR2F0ZSwkbmV3SW50ZXJydXB0ZWRHYXRlKQpmb3JlYWNoKCRyZXF1aXJlZCBpbiBAKCdwb3NpdGl2ZS10cmFuc2FjdGlvbi1zdGFydC13aXRob3V0LWxpdGVyYWwtaW5zdGFsbC1jb250cm9sJywnZnJvemVuLXBvc2l0aXZlLXN0YXJ0LW93bmVkLXBheWxvYWQtb3ItYXJwJywnKD9pKV5uc2lzLScpKXsKICBpZigtbm90ICRwYXRjaGVkLkNvbnRhaW5zKCRyZXF1aXJlZCkpe3Rocm93ICIwMy4xOCBpbnRlcnJ1cHRlZC1zdGFydCBwYXRjaCBtaXNzaW5nIHRva2VuOiAkcmVxdWlyZWQifQp9Cg=='
$innerInsertion=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($innerPayloadB64))
if([string]::IsNullOrWhiteSpace($innerInsertion) -or -not $innerInsertion.Contains('positive-transaction-start-without-literal-install-control')){
  throw '03.18 interrupted-start decoded patch payload invalid.'
}
$patched=$patched.Replace($innerAnchor,$innerInsertion+"`n"+$innerAnchor)
'@.Replace("`r`n","`n")
$outerPatched=$source.Replace($outerAnchor,$outerInsertion+"`n"+$outerAnchor)

$tempRoot=if($env:RUNNER_TEMP){$env:RUNNER_TEMP}else{[IO.Path]::GetTempPath()}
$outerRuntime=Join-Path $tempRoot 'pkg03-0318-interrupted-positive-start-wrapper-runtime.ps1'
[IO.File]::WriteAllText($outerRuntime,$outerPatched,[Text.UTF8Encoding]::new($false))
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile($outerRuntime,[ref]$tokens,[ref]$errors)|Out-Null
if($errors.Count -ne 0){$errors|ForEach-Object{Write-Host $_.Message};throw "03.18 outer wrapper has $($errors.Count) parse error(s)."}

& $outerRuntime `
  -CurrentUserNsisPath $CurrentUserNsisPath `
  -PerMachineNsisPath $PerMachineNsisPath `
  -MsiPath $MsiPath `
  -SourceSha $SourceSha `
  -EvidenceDir $EvidenceDir