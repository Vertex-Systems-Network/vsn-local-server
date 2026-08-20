$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
Write-Host '== VSN 0.6 Windows integration smoke =='

cargo build --workspace
cargo test --workspace

$Agent = Join-Path $Root 'target\debug\vsn-agent.exe'
$Cli = Join-Path $Root 'target\debug\vsn.exe'
if (!(Test-Path $Agent) -or !(Test-Path $Cli)) { throw 'compiled Agent/CLI not found' }

& $Agent --once | Out-Host
$proc = Start-Process -FilePath $Agent -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 2
try {
  & $Cli status | Out-Host
  $Workspace = Join-Path $env:TEMP 'vsn-0.6-smoke'
  New-Item -ItemType Directory -Force $Workspace | Out-Null
  & $Cli workspace add $Workspace | Out-Host

  $textFile = Join-Path $Workspace 'roundtrip.txt'
  'hello from VSN 0.6' | & $Cli files write $textFile | Out-Host
  $read = & $Cli files read $textFile | Out-String
  if ($read -notmatch 'hello from VSN 0.6') { throw 'text file round-trip failed' }

  $binaryFile = Join-Path $Workspace 'binary.bin'
  $bytes = [byte[]](0..255)
  $b64 = [Convert]::ToBase64String($bytes)
  $transfer = 'smoke_transfer_001'
  $b64 | & $Cli files binary-write $binaryFile $transfer 0 true | Out-Host
  $chunkJson = (& $Cli files binary-read $binaryFile | Out-String) | ConvertFrom-Json
  $roundtrip = [Convert]::FromBase64String($chunkJson.data_b64)
  if ($roundtrip.Length -ne 256) { throw 'binary transfer length mismatch' }

  '{"intent":"inspect_machine"}' | & $Cli ai plan | Out-Host
  '{"name":"smoke-cloud","provider":"generic-ssh","region":"local","machine_type":"vm","os_image":"ubuntu","disk_gb":64,"runtime_requirements":{},"services":[],"labels":{}}' | & $Cli cloud workspace-plan | Out-Host

  & $Cli container backends | Out-Host
  & $Cli db clients | Out-Host
  Write-Host 'VSN 0.6 local integration smoke completed.'
}
finally {
  if ($proc -and !$proc.HasExited) { Stop-Process -Id $proc.Id -Force }
}
