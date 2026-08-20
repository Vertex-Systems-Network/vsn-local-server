$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location "$root\apps\desktop"
try {
  npm install
  npm run build
} finally { Pop-Location }
Push-Location "$root\cloud\dashboard"
try {
  npm install
  npm run build
} finally { Pop-Location }
Write-Host "Frontend builds complete."
