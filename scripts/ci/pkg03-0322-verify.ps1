param(
  [Parameter(Mandatory=$true)][string]$UnsignedDir,
  [Parameter(Mandatory=$true)][string]$SignedDir,
  [Parameter(Mandatory=$true)][ValidateSet('test','production')][string]$Mode,
  [Parameter(Mandatory=$true)][string]$ExpectedSubject,
  [Parameter(Mandatory=$true)][string]$EventLog,
  [Parameter(Mandatory=$true)][string]$OutputPath,
  [string]$SignToolPath = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'

$Names=@('nsis-current-user.exe','nsis-per-machine.exe','vsn-platform.msi','VSN Dev Platform.exe')
$ForbiddenSuffixes=@('.pfx','.p12','.key','.pem')
$ForbiddenMarkers=@(
  '-----BEGIN PRIVATE KEY-----',
  '-----BEGIN RSA PRIVATE KEY-----',
  '-----BEGIN EC PRIVATE KEY-----',
  '-----BEGIN ENCRYPTED PRIVATE KEY-----'
)

function Assert-Condition([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Get-Sha256([string]$Path){return (Get-FileHash -LiteralPath $Path -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()}
function Get-MsiProperty([string]$Path,[string]$Property){
  $installer=New-Object -ComObject WindowsInstaller.Installer
  $db=$installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@($Path,0))
  $view=$db.GetType().InvokeMember('OpenView','InvokeMethod',$null,$db,@("SELECT `Value` FROM `Property` WHERE `Property`='$Property'"))
  $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,$null)|Out-Null
  $record=$view.GetType().InvokeMember('Fetch','InvokeMethod',$null,$view,$null)
  if($null -eq $record){throw "MSI property '$Property' not found."}
  return [string]$record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1))
}
function Get-ExeIdentity([string]$Path){
  $v=[Diagnostics.FileVersionInfo]::GetVersionInfo($Path)
  return [pscustomobject][ordered]@{product_name=[string]$v.ProductName;product_version=[string]$v.ProductVersion;company_name=[string]$v.CompanyName;file_description=[string]$v.FileDescription}
}
function Assert-ObjectJsonEqual([object]$A,[object]$B,[string]$Label){
  $ja=$A|ConvertTo-Json -Compress -Depth 8;$jb=$B|ConvertTo-Json -Compress -Depth 8
  Assert-Condition ($ja -eq $jb) "$Label changed after signing."
}

$UnsignedDir=(Resolve-Path -LiteralPath $UnsignedDir).Path
$SignedDir=(Resolve-Path -LiteralPath $SignedDir).Path
if ([string]::IsNullOrWhiteSpace($SignToolPath)) {
  $candidates=@(Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter signtool.exe -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending)
  if($candidates.Count -lt 1){throw '03.22 SignTool x64 could not be located.'}
  $SignToolPath=$candidates[0].FullName
}
Assert-Condition (Test-Path -LiteralPath $EventLog -PathType Leaf) '03.22 signing event log is missing.'
$events=@()
foreach($line in @(Get-Content -LiteralPath $EventLog -ErrorAction Stop)){
  if([string]::IsNullOrWhiteSpace($line)){continue}
  $events += ($line | ConvertFrom-Json -ErrorAction Stop)
}
Assert-Condition ($events.Count -eq $Names.Count) "03.22 expected $($Names.Count) signing events, found $($events.Count)."

$rows=@()
foreach($name in $Names){
  $unsigned=Join-Path $UnsignedDir $name;$signed=Join-Path $SignedDir $name
  Assert-Condition (Test-Path -LiteralPath $unsigned -PathType Leaf) "Unsigned candidate missing: $name"
  Assert-Condition (Test-Path -LiteralPath $signed -PathType Leaf) "Signed candidate missing: $name"
  $unsignedHash=Get-Sha256 $unsigned;$signedHash=Get-Sha256 $signed
  Assert-Condition ($unsignedHash -ne $signedHash) "$name signed bytes equal unsigned bytes."

  $event=@($events|Where-Object { $_.file_name -eq $name -and $_.mode -eq $Mode })
  Assert-Condition ($event.Count -eq 1) "$name signing event count mismatch."
  $event=$event[0]
  Assert-Condition ($event.unsigned_sha256 -eq $unsignedHash) "$name unsigned provenance mismatch."
  Assert-Condition ($event.signed_sha256 -eq $signedHash) "$name signed provenance mismatch."
  Assert-Condition ($event.file_digest -eq 'SHA256') "$name Authenticode digest is not SHA256."
  Assert-Condition ($event.private_key_material_recorded -eq $false -and $event.credential_value_recorded -eq $false) "$name event widened credential evidence."
  if($Mode -eq 'production'){
    Assert-Condition ($event.timestamp_protocol -eq 'RFC3161' -and $event.timestamp_digest -eq 'SHA256') "$name production timestamp contract mismatch."
  }else{
    Assert-Condition ($event.timestamp_protocol -eq 'none-test-wiring') "$name test lane incorrectly claims production timestamping."
  }

  $sig=Get-AuthenticodeSignature -LiteralPath $signed
  Assert-Condition ([string]$sig.Status -eq 'Valid') "$name Windows Authenticode status is $($sig.Status), expected Valid."
  Assert-Condition ($null -ne $sig.SignerCertificate) "$name signer certificate missing."
  Assert-Condition ([string]$sig.SignerCertificate.Subject -eq $ExpectedSubject) "$name signer subject mismatch: $($sig.SignerCertificate.Subject)"
  if($Mode -eq 'production'){
    Assert-Condition ($null -ne $sig.TimeStamperCertificate) "$name production RFC3161 timestamp certificate missing."
  }

  $verifyOutput=(& $SignToolPath verify /pa /all /v $signed 2>&1 | Out-String)
  $verifyExit=$LASTEXITCODE
  if($verifyExit -ne 0){Write-Host $verifyOutput}
  Assert-Condition ($verifyExit -eq 0) "$name SignTool native verification failed with exit code $verifyExit."

  $identityBefore=$null;$identityAfter=$null
  if([IO.Path]::GetExtension($name).ToLowerInvariant() -eq '.msi'){
    $identityBefore=[pscustomobject][ordered]@{product_code=Get-MsiProperty $unsigned 'ProductCode';upgrade_code=Get-MsiProperty $unsigned 'UpgradeCode';product_name=Get-MsiProperty $unsigned 'ProductName';product_version=Get-MsiProperty $unsigned 'ProductVersion'}
    $identityAfter=[pscustomobject][ordered]@{product_code=Get-MsiProperty $signed 'ProductCode';upgrade_code=Get-MsiProperty $signed 'UpgradeCode';product_name=Get-MsiProperty $signed 'ProductName';product_version=Get-MsiProperty $signed 'ProductVersion'}
  }else{
    $identityBefore=Get-ExeIdentity $unsigned;$identityAfter=Get-ExeIdentity $signed
  }
  Assert-ObjectJsonEqual $identityBefore $identityAfter "$name package identity metadata"

  $rows += [pscustomobject][ordered]@{
    file_name=$name
    unsigned_sha256=$unsignedHash
    signed_sha256=$signedHash
    windows_status=[string]$sig.Status
    signer_subject=[string]$sig.SignerCertificate.Subject
    signer_thumbprint=[string]$sig.SignerCertificate.Thumbprint
    timestamp_present=($null -ne $sig.TimeStamperCertificate)
    timestamp_subject=$(if($null -ne $sig.TimeStamperCertificate){[string]$sig.TimeStamperCertificate.Subject}else{$null})
    signtool_verify_exit=$verifyExit
    identity_before=$identityBefore
    identity_after=$identityAfter
    identity_equal=$true
  }
}

# Deterministic negative probe: mutate signed PE body bytes, not the certificate
# table itself, and require both Windows verification surfaces to reject it.
$tamperSource=Join-Path $SignedDir 'VSN Dev Platform.exe'
$tamperDir=Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Force $tamperDir|Out-Null
$tampered=Join-Path $tamperDir "tampered-$Mode-VSN-Dev-Platform.exe"
Copy-Item -LiteralPath $tamperSource -Destination $tampered -Force
$stream=[IO.File]::Open($tampered,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try{
  Assert-Condition ($stream.Length -gt 16384) '03.22 tamper source unexpectedly small.'
  $offset=[long][Math]::Min(8192,[Math]::Floor($stream.Length/3))
  [void]$stream.Seek($offset,[IO.SeekOrigin]::Begin)
  $original=$stream.ReadByte();Assert-Condition ($original -ge 0) '03.22 tamper byte read failed.'
  [void]$stream.Seek($offset,[IO.SeekOrigin]::Begin)
  $stream.WriteByte([byte]($original -bxor 0x01));$stream.Flush()
}finally{$stream.Dispose()}
$tamperedSig=Get-AuthenticodeSignature -LiteralPath $tampered
$tamperOutput=(& $SignToolPath verify /pa /all /v $tampered 2>&1 | Out-String);$tamperExit=$LASTEXITCODE
Assert-Condition ([string]$tamperedSig.Status -ne 'Valid') '03.22 tampered copy unexpectedly remained Authenticode-valid.'
Assert-Condition ($tamperExit -ne 0) '03.22 SignTool unexpectedly accepted tampered copy.'

# Evidence-local secret leak scan. Runtime PFX material is required to live only
# in RUNNER_TEMP and is never copied below EvidenceDir.
$evidenceRoot=Split-Path -Parent $OutputPath
foreach($file in @(Get-ChildItem -LiteralPath $evidenceRoot -Recurse -File -Force -ErrorAction SilentlyContinue)){
  Assert-Condition ($ForbiddenSuffixes -notcontains $file.Extension.ToLowerInvariant()) "Forbidden secret-bearing evidence file: $($file.FullName)"
  if($file.Length -gt 2MB){continue}
  try{$text=[IO.File]::ReadAllText($file.FullName)}catch{continue}
  foreach($marker in $ForbiddenMarkers){Assert-Condition (-not $text.Contains($marker)) "Private-key marker found in evidence file: $($file.FullName)"}
}

$result=[ordered]@{
  schema_version=1
  package_id='PKG-03'
  task_id='03.22'
  mode=$Mode
  expected_subject=$ExpectedSubject
  candidates=$rows
  authenticode_digest='SHA256'
  rfc3161_timestamp_required=($Mode -eq 'production')
  timestamp_digest=$(if($Mode -eq 'production'){'SHA256'}else{$null})
  windows_native_verification=$true
  tamper_negative=[ordered]@{file_name=[IO.Path]::GetFileName($tampered);authenticode_status=[string]$tamperedSig.Status;signtool_verify_exit=$tamperExit;rejected=$true}
  package_identity_metadata_equal=$true
  secret_leak_scan_passed=$true
  private_key_material_recorded=$false
  production_accepted=($Mode -eq 'production')
  test_certificate_cannot_satisfy_production=($Mode -eq 'test')
}
$result|ConvertTo-Json -Depth 14|Set-Content -LiteralPath $OutputPath -Encoding utf8NoBOM
$result|ConvertTo-Json -Depth 14
