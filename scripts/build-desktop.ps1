$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location "$root\apps\desktop"
try {
  if (-not (Test-Path node_modules)) { npm install }
  npm run build
  npm run tauri build
} finally { Pop-Location }
