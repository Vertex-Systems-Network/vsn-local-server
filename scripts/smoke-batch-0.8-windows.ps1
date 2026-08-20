$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
Write-Host '== VSN 0.8 Windows integration smoke =='

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
  & $Cli status | Out-Host
  $Workspace = Join-Path $env:TEMP 'vsn-0.8-smoke'
  if (Test-Path $Workspace) { Remove-Item -Recurse -Force $Workspace }
  New-Item -ItemType Directory -Force $Workspace | Out-Null
  & $Cli workspace add $Workspace | Out-Host

  # Files: directory create -> atomic text write -> move -> delete.
  $DirA = Join-Path $Workspace 'dir-a'
  $DirB = Join-Path $Workspace 'dir-b'
  & $Cli files mkdir $DirA | Out-Host
  $textFile = Join-Path $DirA 'roundtrip.txt'
  'hello from VSN 0.8' | & $Cli files write $textFile | Out-Host
  & $Cli files move $DirA $DirB | Out-Host
  $moved = Join-Path $DirB 'roundtrip.txt'
  $read = & $Cli files read $moved | Out-String
  if ($read -notmatch 'hello from VSN 0.8') { throw 'file move/text round-trip failed' }

  # Resumable binary transfer baseline retained from 0.6.
  $binaryFile = Join-Path $Workspace 'binary.bin'
  $bytes = [byte[]](0..255)
  $b64 = [Convert]::ToBase64String($bytes)
  $transfer = 'smoke_transfer_008'
  $b64 | & $Cli files binary-write $binaryFile $transfer 0 true | Out-Host
  $chunkJson = (& $Cli files binary-read $binaryFile | Out-String) | ConvertFrom-Json
  $roundtrip = [Convert]::FromBase64String($chunkJson.data_b64)
  if ($roundtrip.Length -ne 256) { throw 'binary transfer length mismatch' }

  # Persistent terminal pipe session retained as the bounded non-PTY path.
  $session = ((& $Cli terminal start $Workspace cmd.exe /Q) | Out-String) | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace($session.session_id)) { throw 'terminal session id missing' }
  "echo VSN_TERMINAL_OK`r`nexit`r`n" | & $Cli terminal write $session.session_id | Out-Host
  Start-Sleep -Milliseconds 500
  $output = ((& $Cli terminal read $session.session_id) | Out-String) | ConvertFrom-Json
  if (($output.stdout + $output.stderr) -notmatch 'VSN_TERMINAL_OK') { throw 'persistent terminal session output missing' }
  & $Cli terminal remove $session.session_id | Out-Host

  # True Windows ConPTY path through portable-pty.
  $pty = ((& $Cli terminal pty-start $Workspace cmd.exe /Q) | Out-String) | ConvertFrom-Json
  if ([string]::IsNullOrWhiteSpace($pty.session_id)) { throw 'PTY session id missing' }
  "echo VSN_PTY_OK`r`nexit`r`n" | & $Cli terminal pty-write $pty.session_id | Out-Host
  Start-Sleep -Milliseconds 700
  $ptyOut = ((& $Cli terminal pty-read $pty.session_id) | Out-String) | ConvertFrom-Json
  if ($ptyOut.output -notmatch 'VSN_PTY_OK') { throw 'PTY/ConPTY output missing' }
  & $Cli terminal pty-remove $pty.session_id | Out-Host

  # Static/local feature surfaces.
  & $Cli db clients | Out-Host
  & $Cli container backends | Out-Host
  '{"intent":"inspect_machine"}' | & $Cli ai plan | Out-Host
  '{"name":"smoke-cloud","provider":"generic-ssh","region":"local","machine_type":"vm","os_image":"ubuntu","disk_gb":64,"runtime_requirements":{},"services":[],"labels":{}}' | & $Cli cloud workspace-plan | Out-Host

  & $Cli files delete $DirB true | Out-Host
  & $Cli files delete $binaryFile false | Out-Host
  Write-Host 'VSN 0.8 Windows integration smoke completed.'
}
finally {
  if ($proc -and !$proc.HasExited) { Stop-Process -Id $proc.Id -Force }
}
