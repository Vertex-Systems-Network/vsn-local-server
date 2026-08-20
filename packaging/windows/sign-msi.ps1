param([Parameter(Mandatory=$true)][string]$Msi,[Parameter(Mandatory=$true)][string]$CertificateThumbprint,[string]$TimestampUrl="http://timestamp.digicert.com")
$ErrorActionPreference="Stop"
if(!(Test-Path $Msi)){throw "MSI not found: $Msi"}
$signtool=(Get-Command signtool.exe -ErrorAction SilentlyContinue).Source
if(-not $signtool){
  $root="${env:ProgramFiles(x86)}\Windows Kits\10\bin"
  if(Test-Path $root){$signtool=(Get-ChildItem -Path $root -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue | Where-Object {$_.FullName -match '\\x64\\signtool\.exe$'} | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName)}
}
if(-not $signtool){throw "signtool.exe from the Windows SDK is required"}
& $signtool sign /sha1 $CertificateThumbprint /fd SHA256 /tr $TimestampUrl /td SHA256 $Msi
if($LASTEXITCODE -ne 0){throw "SignTool sign failed: $LASTEXITCODE"}
& $signtool verify /pa /all /v $Msi
if($LASTEXITCODE -ne 0){throw "SignTool verify failed: $LASTEXITCODE"}
