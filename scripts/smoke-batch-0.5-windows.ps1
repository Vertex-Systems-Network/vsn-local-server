$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host '== VSN 0.5 Windows smoke =='
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'cargo is required' }

cargo build --workspace
cargo test --workspace

$Agent = Join-Path $Root 'target\debug\vsn-agent.exe'
$Cli = Join-Path $Root 'target\debug\vsn.exe'
if (-not (Test-Path $Agent)) { throw 'vsn-agent.exe missing after build' }
if (-not (Test-Path $Cli)) { throw 'vsn.exe missing after build' }

$Workspace = Join-Path $env:TEMP 'vsn-0.5-smoke'
New-Item -ItemType Directory -Force -Path $Workspace | Out-Null
$File = Join-Path $Workspace 'hello.txt'

$agentJob = Start-Job -ScriptBlock { param($exe) & $exe } -ArgumentList $Agent
Start-Sleep -Seconds 2
try {
  & $Cli status | Out-Host
  & $Cli workspace add $Workspace | Out-Host
  'hello from VSN 0.5' | & $Cli files write $File | Out-Host
  $read = & $Cli files read $File
  if ($read -notmatch 'hello from VSN 0.5') { throw 'file round-trip failed' }
  & $Cli terminal exec $Workspace cmd.exe /c ver | Out-Host
  & $Cli db clients | Out-Host
  try { & $Cli preview fetch 9 / | Out-Host } catch { Write-Host "Expected closed-port preview outcome: $($_.Exception.Message)" }
} finally {
  Stop-Job $agentJob -ErrorAction SilentlyContinue | Out-Null
  Remove-Job $agentJob -Force -ErrorAction SilentlyContinue | Out-Null
}

Write-Host 'VSN 0.5 smoke completed.'
