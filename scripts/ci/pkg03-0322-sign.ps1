param(
  [Parameter(Mandatory=$true)][string]$FilePath,
  [Parameter(Mandatory=$true)][string]$CertificateThumbprint,
  [Parameter(Mandatory=$true)][ValidateSet('test','production')][string]$Mode,
  [Parameter(Mandatory=$true)][string]$EventLog,
  [string]$TimestampUrl = '',
  [string]$SignToolPath = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

function Get-Sha256([string]$Path) {
  return (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
}

if (-not (Test-Path -LiteralPath $FilePath -PathType Leaf)) { throw "03.22 signing target missing: $FilePath" }
$FilePath=(Resolve-Path -LiteralPath $FilePath).Path
if ([string]::IsNullOrWhiteSpace($SignToolPath)) {
  $candidates=@(Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter signtool.exe -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending)
  if ($candidates.Count -lt 1) { throw '03.22 SignTool x64 could not be located.' }
  $SignToolPath=$candidates[0].FullName
}
if (-not (Test-Path -LiteralPath $SignToolPath -PathType Leaf)) { throw "03.22 SignTool missing: $SignToolPath" }

$thumb=($CertificateThumbprint -replace '\s','').ToUpperInvariant()
if ($thumb -notmatch '^[0-9A-F]{40}$') { throw '03.22 certificate thumbprint must be a SHA-1 thumbprint.' }
$cert=Get-Item -LiteralPath "Cert:\CurrentUser\My\$thumb" -ErrorAction Stop
if (-not $cert.HasPrivateKey) { throw '03.22 selected certificate has no accessible private key.' }

$before=Get-Sha256 $FilePath
$args=@('sign','/sha1',$thumb,'/s','My','/fd','SHA256','/v')
if ($Mode -eq 'production') {
  if ([string]::IsNullOrWhiteSpace($TimestampUrl)) { throw '03.22 production signing requires an RFC3161 timestamp URL.' }
  if ($TimestampUrl -notmatch '^https?://') { throw '03.22 timestamp URL must be HTTP(S).' }
  $args += @('/tr',$TimestampUrl,'/td','SHA256')
}
$args += @($FilePath)

$output=(& $SignToolPath @args 2>&1 | Out-String)
$exit=$LASTEXITCODE
if ($exit -ne 0) {
  Write-Host $output
  throw "03.22 SignTool failed for $([IO.Path]::GetFileName($FilePath)) with exit code $exit"
}
$after=Get-Sha256 $FilePath
if ($after -eq $before) { throw '03.22 signing did not change target bytes.' }

$sig=Get-AuthenticodeSignature -LiteralPath $FilePath
$event=[ordered]@{
  schema_version=1
  task_id='03.22'
  mode=$Mode
  file_name=[IO.Path]::GetFileName($FilePath)
  unsigned_sha256=$before
  signed_sha256=$after
  file_digest='SHA256'
  timestamp_protocol=$(if($Mode -eq 'production'){'RFC3161'}else{'none-test-wiring'})
  timestamp_digest=$(if($Mode -eq 'production'){'SHA256'}else{$null})
  certificate_thumbprint=$thumb
  signer_subject=[string]$cert.Subject
  signer_issuer=[string]$cert.Issuer
  signer_not_after=$cert.NotAfter.ToUniversalTime().ToString('o')
  windows_signature_status=[string]$sig.Status
  private_key_material_recorded=$false
  credential_value_recorded=$false
  at_utc=[DateTime]::UtcNow.ToString('o')
}
$parent=Split-Path -Parent $EventLog
if ($parent) { New-Item -ItemType Directory -Force $parent | Out-Null }
($event | ConvertTo-Json -Compress -Depth 8) | Add-Content -LiteralPath $EventLog -Encoding utf8NoBOM
$event | ConvertTo-Json -Depth 8
