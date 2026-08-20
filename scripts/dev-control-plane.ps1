param(
  [string]$AdminToken = "",
  [string]$Bind = "127.0.0.1:9070"
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($AdminToken)) {
  $bytes = New-Object byte[] 32
  $rng = [Security.Cryptography.RandomNumberGenerator]::Create()
  try { $rng.GetBytes($bytes) } finally { $rng.Dispose() }
  $AdminToken = -join ($bytes | ForEach-Object { $_.ToString("x2") })
  Write-Host "Development admin token: $AdminToken"
}
$env:VSN_CONTROL_ADMIN_TOKEN = $AdminToken
$env:VSN_CONTROL_BIND = $Bind
Push-Location "$root\cloud\dashboard"
try {
  if (-not (Test-Path node_modules)) { npm install }
  npm run build
} finally { Pop-Location }
Push-Location $root
try { cargo run -p vsn-control-plane } finally { Pop-Location }
