$ErrorActionPreference = "Stop"

Write-Host "VSN Windows build"
& "$PSScriptRoot\\bootstrap-windows.ps1"

cargo test --workspace
cargo build --workspace --release

Write-Host "Built:"
Write-Host "  target\\release\\vsn.exe"
Write-Host "  target\\release\\vsn-agent.exe"
Write-Host ""
Write-Host "Install Agent service from an elevated terminal:"
Write-Host "  .\\target\\release\\vsn-agent.exe service install"
Write-Host "  .\\target\\release\\vsn-agent.exe service start"
