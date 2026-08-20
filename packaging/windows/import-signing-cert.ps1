param([Parameter(Mandatory=$true)][string]$PfxBase64,[Parameter(Mandatory=$true)][string]$PfxPassword)
$ErrorActionPreference='Stop'
$temp=Join-Path ([IO.Path]::GetTempPath()) ("vsn-signing-"+[guid]::NewGuid().ToString('N')+'.pfx')
try {
  [IO.File]::WriteAllBytes($temp,[Convert]::FromBase64String($PfxBase64))
  $secure=ConvertTo-SecureString $PfxPassword -AsPlainText -Force
  $cert=Import-PfxCertificate -FilePath $temp -CertStoreLocation 'Cert:\CurrentUser\My' -Password $secure -Exportable:$false
  if(-not $cert -or -not $cert.HasPrivateKey){ throw 'Imported signing certificate has no private key' }
  Write-Output $cert.Thumbprint
} finally { Remove-Item -Force -ErrorAction SilentlyContinue $temp }
