param(
  [string]$Subject = 'CN=VSN Dev Platform CI Test Signing'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Deterministic, non-production test certificate creation for PKG-03 03.22.
# Uses .NET cryptography/store APIs instead of the PKIClient
# New-SelfSignedCertificate/Import-Certificate path that stalled on the hosted
# Windows 2025 runner. No test private-key bytes are written to disk or evidence.

$rsa = [System.Security.Cryptography.RSA]::Create(2048)
$raw = $null
$persisted = $null
$publicOnly = $null
$myStore = $null
$rootStore = $null
$pfxBytes = $null
try {
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

  $publicOnly = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
    $persisted.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert)
  )
  $rootStore = [System.Security.Cryptography.X509Certificates.X509Store]::new(
    [System.Security.Cryptography.X509Certificates.StoreName]::Root,
    [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
  )
  $rootStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
  $rootStore.Add($publicOnly)

  $stored = Get-Item -LiteralPath "Cert:\CurrentUser\My\$($persisted.Thumbprint)" -ErrorAction Stop
  if (-not $stored.HasPrivateKey) { throw '03.22 CurrentUser My test certificate is not SignTool-usable.' }
  $rootStored = Get-Item -LiteralPath "Cert:\CurrentUser\Root\$($persisted.Thumbprint)" -ErrorAction Stop
  if ($null -eq $rootStored) { throw '03.22 CurrentUser Root trust certificate was not persisted.' }

  "VSN_0322_TEST_THUMB=$($persisted.Thumbprint)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "VSN_0322_TEST_SUBJECT=$($persisted.Subject)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  [pscustomobject][ordered]@{
    valid=$true
    mode='test-only'
    subject=$persisted.Subject
    thumbprint=$persisted.Thumbprint
    has_private_key=$stored.HasPrivateKey
    trusted_current_user_root=$true
    private_key_material_recorded=$false
    production_accepted=$false
  } | ConvertTo-Json -Depth 6
}
finally {
  if ($null -ne $rootStore) { $rootStore.Dispose() }
  if ($null -ne $myStore) { $myStore.Dispose() }
  if ($null -ne $publicOnly) { $publicOnly.Dispose() }
  if ($null -ne $persisted) { $persisted.Dispose() }
  if ($null -ne $raw) { $raw.Dispose() }
  if ($null -ne $rsa) { $rsa.Dispose() }
  if ($null -ne $pfxBytes) { [Array]::Clear($pfxBytes,0,$pfxBytes.Length) }
}
