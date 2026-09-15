param(
  [Parameter(Mandatory=$true)][string]$UnsignedDir,
  [Parameter(Mandatory=$true)][string]$SignedDir,
  [Parameter(Mandatory=$true)][string]$UnsignedProvenance,
  [Parameter(Mandatory=$true)][string]$ExpectedSubject,
  [Parameter(Mandatory=$true)][string]$OutputPath,
  [string]$ExpectedSourceSha = '',
  [string]$ExpectedTrustedRequestSha = '',
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
  return [pscustomobject][ordered]@{
    product_name=[string]$v.ProductName
    product_version=[string]$v.ProductVersion
    company_name=[string]$v.CompanyName
    file_description=[string]$v.FileDescription
  }
}
function Assert-ObjectJsonEqual([object]$A,[object]$B,[string]$Label){
  $ja=$A|ConvertTo-Json -Compress -Depth 8
  $jb=$B|ConvertTo-Json -Compress -Depth 8
  Assert-Condition ($ja -eq $jb) "$Label changed after signing."
}
function Resolve-OneByName([string]$Root,[string]$Name){
  $matches=@(Get-ChildItem -LiteralPath $Root -Recurse -File -ErrorAction Stop | Where-Object { $_.Name -ceq $Name })
  Assert-Condition ($matches.Count -eq 1) "Expected exactly one file named '$Name' below '$Root', found $($matches.Count)."
  return $matches[0].FullName
}

$UnsignedDir=(Resolve-Path -LiteralPath $UnsignedDir).Path
$SignedDir=(Resolve-Path -LiteralPath $SignedDir).Path
$UnsignedProvenance=(Resolve-Path -LiteralPath $UnsignedProvenance).Path
Assert-Condition (-not [string]::IsNullOrWhiteSpace($ExpectedSubject)) 'Approved SignPath publisher subject is required.'
Assert-Condition (Test-Path -LiteralPath $UnsignedProvenance -PathType Leaf) 'Unsigned provenance file is missing.'

if([string]::IsNullOrWhiteSpace($SignToolPath)){
  $candidates=@(Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter signtool.exe -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending)
  if($candidates.Count -lt 1){throw '03.22 SignTool x64 could not be located.'}
  $SignToolPath=$candidates[0].FullName
}
Assert-Condition (Test-Path -LiteralPath $SignToolPath -PathType Leaf) '03.22 SignTool path is invalid.'

$prov=Get-Content -LiteralPath $UnsignedProvenance -Raw -ErrorAction Stop | ConvertFrom-Json -ErrorAction Stop
Assert-Condition ([int]$prov.schema_version -eq 1) 'Unsigned provenance schema mismatch.'
Assert-Condition ([string]$prov.task_id -eq '03.22') 'Unsigned provenance task mismatch.'
Assert-Condition ($prov.production_secrets_available_in_build_job -eq $false) 'Unsigned build did not prove a secret-free build context.'
if(-not [string]::IsNullOrWhiteSpace($ExpectedSourceSha)){
  Assert-Condition ([string]$prov.source_commit -eq $ExpectedSourceSha) "Unsigned provenance source mismatch: $($prov.source_commit)"
}
if(-not [string]::IsNullOrWhiteSpace($ExpectedTrustedRequestSha)){
  Assert-Condition ([string]$prov.trusted_request_commit -eq $ExpectedTrustedRequestSha) "Unsigned provenance trusted-request mismatch: $($prov.trusted_request_commit)"
}

$provRows=@($prov.candidates)
Assert-Condition ($provRows.Count -eq $Names.Count) "Unsigned provenance expected $($Names.Count) candidates, found $($provRows.Count)."
$provNames=@($provRows | ForEach-Object { [string]$_.file_name } | Sort-Object)
Assert-Condition ((Compare-Object ($Names|Sort-Object) $provNames).Count -eq 0) 'Unsigned provenance candidate set mismatch.'

$rows=@()
foreach($name in $Names){
  $unsigned=Resolve-OneByName $UnsignedDir $name
  $signed=Resolve-OneByName $SignedDir $name
  $unsignedHash=Get-Sha256 $unsigned
  $signedHash=Get-Sha256 $signed
  Assert-Condition ($unsignedHash -ne $signedHash) "$name signed bytes equal unsigned bytes."

  $unsignedSig=Get-AuthenticodeSignature -LiteralPath $unsigned
  Assert-Condition ([string]$unsignedSig.Status -eq 'NotSigned') "$name unsigned input status is $($unsignedSig.Status), expected NotSigned."

  $provRow=@($provRows | Where-Object { [string]$_.file_name -ceq $name })
  Assert-Condition ($provRow.Count -eq 1) "$name unsigned provenance row count mismatch."
  Assert-Condition ([string]$provRow[0].sha256 -eq $unsignedHash) "$name unsigned SHA-256 does not match trusted provenance."
  Assert-Condition ([string]$provRow[0].authenticode_status -eq 'NotSigned') "$name provenance does not record NotSigned input."

  $sig=Get-AuthenticodeSignature -LiteralPath $signed
  Assert-Condition ([string]$sig.Status -eq 'Valid') "$name Windows Authenticode status is $($sig.Status), expected Valid."
  Assert-Condition ($null -ne $sig.SignerCertificate) "$name signer certificate is missing."
  Assert-Condition ([string]$sig.SignerCertificate.Subject -ceq $ExpectedSubject) "$name signer subject mismatch: $($sig.SignerCertificate.Subject)"
  Assert-Condition ($null -ne $sig.TimeStamperCertificate) "$name timestamp certificate is missing."

  $verifyOutput=(& $SignToolPath verify /pa /all /v $signed 2>&1 | Out-String)
  $verifyExit=$LASTEXITCODE
  if($verifyExit -ne 0){Write-Host $verifyOutput}
  Assert-Condition ($verifyExit -eq 0) "$name SignTool native verification failed with exit code $verifyExit."

  if([IO.Path]::GetExtension($name).ToLowerInvariant() -eq '.msi'){
    $identityBefore=[pscustomobject][ordered]@{
      product_code=Get-MsiProperty $unsigned 'ProductCode'
      upgrade_code=Get-MsiProperty $unsigned 'UpgradeCode'
      product_name=Get-MsiProperty $unsigned 'ProductName'
      product_version=Get-MsiProperty $unsigned 'ProductVersion'
    }
    $identityAfter=[pscustomobject][ordered]@{
      product_code=Get-MsiProperty $signed 'ProductCode'
      upgrade_code=Get-MsiProperty $signed 'UpgradeCode'
      product_name=Get-MsiProperty $signed 'ProductName'
      product_version=Get-MsiProperty $signed 'ProductVersion'
    }
  }else{
    $identityBefore=Get-ExeIdentity $unsigned
    $identityAfter=Get-ExeIdentity $signed
  }
  Assert-ObjectJsonEqual $identityBefore $identityAfter "$name package identity metadata"

  $rows += [pscustomobject][ordered]@{
    file_name=$name
    unsigned_sha256=$unsignedHash
    signed_sha256=$signedHash
    windows_status=[string]$sig.Status
    signer_subject=[string]$sig.SignerCertificate.Subject
    signer_thumbprint=[string]$sig.SignerCertificate.Thumbprint
    timestamp_present=$true
    timestamp_subject=[string]$sig.TimeStamperCertificate.Subject
    signtool_verify_exit=$verifyExit
    identity_before=$identityBefore
    identity_after=$identityAfter
    identity_equal=$true
  }
}

$evidenceRoot=Split-Path -Parent $OutputPath
if([string]::IsNullOrWhiteSpace($evidenceRoot)){$evidenceRoot='.'}
New-Item -ItemType Directory -Force $evidenceRoot | Out-Null

$tamperSource=Resolve-OneByName $SignedDir 'VSN Dev Platform.exe'
$tampered=Join-Path $evidenceRoot 'tampered-signpath-VSN-Dev-Platform.exe'
Copy-Item -LiteralPath $tamperSource -Destination $tampered -Force
$stream=[IO.File]::Open($tampered,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
try{
  Assert-Condition ($stream.Length -gt 16384) '03.22 tamper source unexpectedly small.'
  $offset=[long][Math]::Min(8192,[Math]::Floor($stream.Length/3))
  [void]$stream.Seek($offset,[IO.SeekOrigin]::Begin)
  $original=$stream.ReadByte()
  Assert-Condition ($original -ge 0) '03.22 tamper byte read failed.'
  [void]$stream.Seek($offset,[IO.SeekOrigin]::Begin)
  $stream.WriteByte([byte]($original -bxor 0x01))
  $stream.Flush()
}finally{$stream.Dispose()}

$tamperedSig=Get-AuthenticodeSignature -LiteralPath $tampered
$tamperOutput=(& $SignToolPath verify /pa /all /v $tampered 2>&1 | Out-String)
$tamperExit=$LASTEXITCODE
Assert-Condition ([string]$tamperedSig.Status -ne 'Valid') '03.22 tampered SignPath copy unexpectedly remained Authenticode-valid.'
Assert-Condition ($tamperExit -ne 0) '03.22 SignTool unexpectedly accepted tampered SignPath copy.'

foreach($file in @(Get-ChildItem -LiteralPath $evidenceRoot -Recurse -File -Force -ErrorAction SilentlyContinue)){
  Assert-Condition ($ForbiddenSuffixes -notcontains $file.Extension.ToLowerInvariant()) "Forbidden secret-bearing evidence file: $($file.FullName)"
  if($file.Length -gt 2MB){continue}
  try{$text=[IO.File]::ReadAllText($file.FullName)}catch{continue}
  foreach($marker in $ForbiddenMarkers){
    Assert-Condition (-not $text.Contains($marker)) "Private-key marker found in evidence file: $($file.FullName)"
  }
}

$result=[ordered]@{
  schema_version=1
  package_id='PKG-03'
  task_id='03.22'
  provider='signpath-foundation'
  production_accepted=$true
  source_commit=[string]$prov.source_commit
  source_base=[string]$prov.source_base
  source_ref=[string]$prov.source_ref
  trusted_request_commit=[string]$prov.trusted_request_commit
  expected_subject=$ExpectedSubject
  candidates=$rows
  authenticode_digest='SHA256'
  timestamp_required=$true
  windows_native_verification=$true
  expected_publisher_binding=$true
  package_identity_metadata_equal=$true
  tamper_negative=[ordered]@{
    file_name=[IO.Path]::GetFileName($tampered)
    authenticode_status=[string]$tamperedSig.Status
    signtool_verify_exit=$tamperExit
    rejected=$true
  }
  secret_leak_scan_passed=$true
  private_key_material_recorded=$false
}
$result | ConvertTo-Json -Depth 14 | Set-Content -LiteralPath $OutputPath -Encoding utf8NoBOM
$result | ConvertTo-Json -Depth 14

# The mandatory tamper-negative SignTool call is expected to fail. Clear only
# that intentional native exit code after both rejection assertions pass.
$global:LASTEXITCODE=0
