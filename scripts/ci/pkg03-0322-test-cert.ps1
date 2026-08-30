param(
  [string]$Subject = 'CN=VSN Dev Platform CI Test Signing'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Deterministic, non-production test certificate creation for PKG-03 03.22.
# The private key remains process-memory/PersistKeySet-backed in CurrentUser\My;
# no private-key bytes are written to repository/evidence. Exact run
# 33313616443 attempt 2 remained in this helper for ~46 minutes after unsigned
# provenance succeeded. The previous helper added the self-signed public cert to
# CurrentUser\Root through X509Store.Add(), a trust-install path that can surface
# interactive Windows trust UI. Use the Windows-native certutil -user -f -Silent
# addstore path instead, with a bounded child-process timeout. Only a public DER
# certificate is written under RUNNER_TEMP and is deleted in finally. This lane
# remains test-only and can never satisfy production signing acceptance.

$rsa = [System.Security.Cryptography.RSA]::Create(2048)
$raw = $null
$persisted = $null
$myStore = $null
$pfxBytes = $null
$publicCertPath = $null
$certutilProcess = $null
try {
  Write-Host '03.22 test-cert phase=create-key-and-request'
  $dn = [System.Security.Cryptography.X509Certificates.X500DistinguishedName]::new($Subject)
  $request = [System.Security.Cryptography.X509Certificates.CertificateRequest]::new(
    $dn,
    $rsa,
    [System.Security.Cryptography.HashAlgorithmName]::SHA256,
    [System.Security.Cryptography.RSASignaturePadding]::Pkcs1
  )
  $eku = [System.Security.Cryptography.OidCollection]::new()
  [void]$eku.Add([System.Security.Cryptography.Oid]::new('1.3.6.1.5.5.7.3.3','Code Signing'))
  $request.CertificateExtensions.Add(
    [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new($eku,$false)
  )
  $request.CertificateExtensions.Add(
    [System.Security.Cryptography.X509Certificates.X509KeyUsageExtension]::new(
      [System.Security.Cryptography.X509Certificates.X509KeyUsageFlags]::DigitalSignature,
      $true
    )
  )
  $request.CertificateExtensions.Add(
    [System.Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]::new($false,$false,0,$true)
  )

  $raw = $request.CreateSelfSigned([DateTimeOffset]::UtcNow.AddMinutes(-5),[DateTimeOffset]::UtcNow.AddDays(1))
  if ($null -eq $raw -or -not $raw.HasPrivateKey) { throw '03.22 .NET test certificate has no private key.' }

  Write-Host '03.22 test-cert phase=persist-private-key-current-user-my'
  # Re-import an in-memory PFX with PersistKeySet so SignTool in later workflow
  # steps can resolve the private key by CurrentUser\My thumbprint. The random
  # password and PFX bytes remain process-memory only and are cleared below.
  $password = [Guid]::NewGuid().ToString('N')
  $pfxBytes = $raw.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Pfx,$password)
  $flags = [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet -bor
           [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::UserKeySet
  $persisted = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($pfxBytes,$password,$flags)
  if (-not $persisted.HasPrivateKey) { throw '03.22 persisted test certificate lost its private key.' }

  $myStore = [System.Security.Cryptography.X509Certificates.X509Store]::new(
    [System.Security.Cryptography.X509Certificates.StoreName]::My,
    [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
  )
  $myStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
  $myStore.Add($persisted)

  $stored = Get-Item -LiteralPath "Cert:\CurrentUser\My\$($persisted.Thumbprint)" -ErrorAction Stop
  if (-not $stored.HasPrivateKey) { throw '03.22 CurrentUser My test certificate is not SignTool-usable.' }

  Write-Host '03.22 test-cert phase=install-public-root-cert-noninteractive'
  $tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
  $publicCertPath = Join-Path $tempRoot ("pkg03-0322-test-root-{0}.cer" -f $persisted.Thumbprint)
  [IO.File]::WriteAllBytes(
    $publicCertPath,
    $persisted.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert)
  )
  $certutil = Join-Path $env:SystemRoot 'System32\certutil.exe'
  if (-not (Test-Path -LiteralPath $certutil -PathType Leaf)) { throw '03.22 certutil.exe missing.' }
  $certutilArgs = @('-user','-f','-Silent','-addstore','Root',('"{0}"' -f $publicCertPath))
  $certutilProcess = Start-Process -FilePath $certutil -ArgumentList $certutilArgs -PassThru -NoNewWindow
  if (-not $certutilProcess.WaitForExit(30000)) {
    try { Stop-Process -Id $certutilProcess.Id -Force -ErrorAction SilentlyContinue } catch {}
    throw '03.22 certutil CurrentUser Root add exceeded bounded 30-second timeout.'
  }
  if ([int]$certutilProcess.ExitCode -ne 0) {
    throw "03.22 certutil CurrentUser Root add failed with exit code $($certutilProcess.ExitCode)."
  }

  $rootStored = Get-Item -LiteralPath "Cert:\CurrentUser\Root\$($persisted.Thumbprint)" -ErrorAction Stop
  if ($null -eq $rootStored) { throw '03.22 CurrentUser Root trust certificate was not persisted.' }

  Write-Host '03.22 test-cert phase=bind-test-only-environment'
  "VSN_0322_TEST_THUMB=$($persisted.Thumbprint)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "VSN_0322_TEST_SUBJECT=$($persisted.Subject)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  [pscustomobject][ordered]@{
    valid=$true
    mode='test-only'
    subject=$persisted.Subject
    thumbprint=$persisted.Thumbprint
    has_private_key=$stored.HasPrivateKey
    trusted_current_user_root=$true
    trust_install_method='certutil-user-force-silent-addstore'
    trust_install_timeout_seconds=30
    private_key_material_recorded=$false
    production_accepted=$false
  } | ConvertTo-Json -Depth 6
}
finally {
  if ($null -ne $myStore) { $myStore.Dispose() }
  if ($null -ne $persisted) { $persisted.Dispose() }
  if ($null -ne $raw) { $raw.Dispose() }
  if ($null -ne $rsa) { $rsa.Dispose() }
  if ($null -ne $pfxBytes) { [Array]::Clear($pfxBytes,0,$pfxBytes.Length) }
  if ($publicCertPath -and (Test-Path -LiteralPath $publicCertPath)) {
    Remove-Item -LiteralPath $publicCertPath -Force -ErrorAction SilentlyContinue
  }
}
