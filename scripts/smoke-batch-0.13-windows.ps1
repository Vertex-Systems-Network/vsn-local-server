$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
Write-Host '== VSN 0.13 Windows integration smoke =='

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace

$Agent = Join-Path $Root 'target\debug\vsn-agent.exe'
$Cli = Join-Path $Root 'target\debug\vsn.exe'
if (!(Test-Path $Agent) -or !(Test-Path $Cli)) { throw 'compiled Agent/CLI not found' }

& $Agent --once | Out-Host
$proc = Start-Process -FilePath $Agent -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
try {
  $version = (& $Cli --version | Out-String).Trim()
  if ($version -notmatch '0\.13\.0') { throw "unexpected CLI version: $version" }
  & $Cli status | Out-Host

  $Workspace = Join-Path $env:TEMP 'vsn-0.13-smoke'
  if (Test-Path $Workspace) { Remove-Item -Recurse -Force $Workspace }
  New-Item -ItemType Directory -Force $Workspace | Out-Null
  & $Cli workspace add $Workspace | Out-Host

  $file = Join-Path $Workspace 'binary.bin'
  $bytes = [byte[]](0..255)
  $b64 = [Convert]::ToBase64String($bytes)
  $transfer = 'smoke_transfer_013'
  $b64 | & $Cli files binary-write $file $transfer 0 false | Out-Host
  $status = ((& $Cli files binary-status $file $transfer) | Out-String) | ConvertFrom-Json
  if ($status.committed_bytes -ne 256) { throw 'binary upload offset mismatch' }
  '' | & $Cli files binary-write $file $transfer 256 true | Out-Host
  $digest = ((& $Cli files digest $file) | Out-String) | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace($digest.sha256)) { throw 'file digest missing' }

  $pty = ((& $Cli terminal pty-start $Workspace cmd.exe /Q) | Out-String) | ConvertFrom-Json
  "echo VSN_PTY_013_OK`r`nexit`r`n" | & $Cli terminal pty-write $pty.session_id | Out-Host
  $ptyOut = ((& $Cli terminal pty-read-wait $pty.session_id 3000) | Out-String) | ConvertFrom-Json
  if ($ptyOut.output -notmatch 'VSN_PTY_013_OK') { throw 'PTY output missing' }
  & $Cli terminal pty-remove $pty.session_id | Out-Host

  # Generic stream foundation smoke through authenticated Agent IPC.
  $stream = ('{"kind":"custom","direction":"bidirectional","resource_id":"smoke-013","metadata":{}}' | & $Cli stream open | Out-String) | ConvertFrom-Json
  $payload = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('hello-stream'))
  $payload | & $Cli stream input $stream.stream_id 0 false | Out-Host
  $inputPull = ((& $Cli stream input-pull $stream.stream_id) | Out-String) | ConvertFrom-Json
  if ($inputPull.frames.Count -ne 1) { throw 'stream input frame missing' }
  $payload | & $Cli stream output $stream.stream_id false | Out-Host
  $outputPull = ((& $Cli stream pull $stream.stream_id) | Out-String) | ConvertFrom-Json
  if ($outputPull.frames.Count -ne 1) { throw 'stream output frame missing' }
  & $Cli stream close $stream.stream_id | Out-Host

  & $Cli db clients | Out-Host
  & $Cli container backends | Out-Host
  & $Cli cloud cli-detect | Out-Host

  if (Get-Command python -ErrorAction SilentlyContinue) {
    python scripts/validate-batch-0.13.py
    python scripts/validate-schemas.py
  }

  # Snapshot/clone are intentionally not executed in smoke: they can create billable provider resources.
  # Their argument/confirmation safety is covered by vsn-cloud unit tests above.

  & $Cli files delete $file false | Out-Host
  Write-Host 'VSN 0.13 Windows integration smoke completed.'
}
finally {
  if ($proc -and !$proc.HasExited) { Stop-Process -Id $proc.Id -Force }
}
