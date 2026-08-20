$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
Write-Host '== VSN 0.18 Windows integration smoke =='

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace

$Agent = Join-Path $Root 'target\debug\vsn-agent.exe'
$Cli = Join-Path $Root 'target\debug\vsn.exe'
$Updater = Join-Path $Root 'target\debug\vsn-updater-helper.exe'
if (!(Test-Path $Agent) -or !(Test-Path $Cli) -or !(Test-Path $Updater)) { throw 'compiled Agent/CLI/updater helper not found' }

& $Agent --once | Out-Host
$proc = Start-Process -FilePath $Agent -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
try {
  $version = (& $Cli --version | Out-String).Trim()
  if ($version -notmatch '0\.17\.0') { throw "unexpected CLI version: $version" }
  & $Cli status | Out-Host

  $Workspace = Join-Path $env:TEMP 'vsn-0.18-smoke'
  if (Test-Path $Workspace) { Remove-Item -Recurse -Force $Workspace }
  New-Item -ItemType Directory -Force $Workspace | Out-Null
  & $Cli workspace add $Workspace | Out-Host

  $file = Join-Path $Workspace 'binary.bin'
  $bytes = [byte[]](0..255)
  $b64 = [Convert]::ToBase64String($bytes)
  $transfer = 'smoke_transfer_017'
  $b64 | & $Cli files binary-write $file $transfer 0 false | Out-Host
  $status = ((& $Cli files binary-status $file $transfer) | Out-String) | ConvertFrom-Json
  if ($status.committed_bytes -ne 256) { throw 'binary upload offset mismatch' }
  '' | & $Cli files binary-write $file $transfer 256 true | Out-Host
  $digest = ((& $Cli files digest $file) | Out-String) | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace($digest.sha256)) { throw 'file digest missing' }

  $pty = ((& $Cli terminal pty-start $Workspace cmd.exe /Q) | Out-String) | ConvertFrom-Json
  "echo VSN_PTY_017_OK`r`nexit`r`n" | & $Cli terminal pty-write $pty.session_id | Out-Host
  $ptyOut = ((& $Cli terminal pty-read-wait $pty.session_id 3000) | Out-String) | ConvertFrom-Json
  if ($ptyOut.output -notmatch 'VSN_PTY_017_OK') { throw 'PTY output missing' }
  & $Cli terminal pty-remove $pty.session_id | Out-Host
  $scroll = ((& $Cli terminal pty-scrollback-read $pty.session_id 0 65536) | Out-String) | ConvertFrom-Json
  $scrollText = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($scroll.payload_base64))
  if ($scrollText -notmatch 'VSN_PTY_017_OK') { throw 'durable PTY scrollback missing' }

  # Generic stream foundation smoke through authenticated Agent IPC.
  $stream = ('{"kind":"terminal","direction":"bidirectional","resource_id":"smoke-017","metadata":{}}' | & $Cli stream open | Out-String) | ConvertFrom-Json
  $payload = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes('hello-stream'))
  $payload | & $Cli stream input $stream.stream_id 0 false | Out-Host
  $inputPull = ((& $Cli stream input-pull $stream.stream_id) | Out-String) | ConvertFrom-Json
  if ($inputPull.frames.Count -ne 1) { throw 'stream input frame missing' }
  $payload | & $Cli stream output $stream.stream_id false | Out-Host
  $outputPull = ((& $Cli stream pull $stream.stream_id) | Out-String) | ConvertFrom-Json
  if ($outputPull.frames.Count -ne 1) { throw 'stream output frame missing' }
  & $Cli stream close $stream.stream_id | Out-Host

  # Out-of-process updater transaction against disposable temp files.
  $install = Join-Path $env:TEMP 'vsn-0.18-update-smoke'
  if (Test-Path $install) { Remove-Item -Recurse -Force $install }
  New-Item -ItemType Directory -Force (Join-Path $install 'bin') | Out-Null
  $target = Join-Path $install 'bin\demo.bin'
  $staged = Join-Path $env:TEMP 'vsn-0.18-staged.bin'
  [IO.File]::WriteAllText($target,'OLD')
  [IO.File]::WriteAllText($staged,'NEW')
  $sha = (Get-FileHash -Algorithm SHA256 $staged).Hash.ToLowerInvariant()
  $apply = @{ install_root=$install; target_relative='bin/demo.bin'; staged_artifact=$staged; expected_sha256=$sha; release='0.18.0-smoke'; confirm_apply=$true } | ConvertTo-Json -Compress
  $apply | & $Cli update apply-file | Out-Host
  if ((Get-Content -Raw $target) -ne 'NEW') { throw 'update apply did not replace target' }
  & $Cli update rollback-file $install | Out-Host
  if ((Get-Content -Raw $target) -ne 'OLD') { throw 'update rollback did not restore previous target' }
  (@{operation='status';install_root=$install} | ConvertTo-Json -Compress) | & $Updater | Out-Host

  & $Cli db clients | Out-Host
  & $Cli db job-list | Out-Host
  & $Cli db pg-native-job-list | Out-Host
  & $Cli container backends | Out-Host
  & $Cli cloud cli-detect | Out-Host

  if (Get-Command python -ErrorAction SilentlyContinue) {
    python scripts/validate-batch-0.18.py
    python scripts/validate-schemas.py
  }

  if (Get-Command wix -ErrorAction SilentlyContinue) {
    cargo build -p vsn-agent -p vsn -p vsn-updater-helper --release
    & (Join-Path $Root 'packaging\windows\build-msi.ps1') -Version 0.18.0 -SourceDir (Join-Path $Root 'target\release') -OutputDir (Join-Path $env:TEMP 'vsn-0.18-msi-smoke')
  }

  # Snapshot/clone are intentionally not executed in smoke: they can create billable provider resources.
  # Their argument/confirmation safety is covered by vsn-cloud unit tests above.

  & $Cli files delete $file false | Out-Host
  Write-Host 'VSN 0.18 Windows integration smoke completed.'
}
finally {
  if ($proc -and !$proc.HasExited) { Stop-Process -Id $proc.Id -Force }
}
