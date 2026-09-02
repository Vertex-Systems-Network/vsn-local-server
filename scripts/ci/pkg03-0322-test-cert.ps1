param(
  [string]$Subject = 'CN=VSN Dev Platform CI Test Signing'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Deterministic, non-production test certificate creation for PKG-03 03.22.
# The private key remains PersistKeySet-backed in CurrentUser\My; no private-key
# bytes are written to repository/evidence. Historical exact-head runs proved
# CurrentUser\Root trust installation can surface or stall on Windows trust UI,
# including certutil -user -f -addstore Root. For the secret-free CI wiring lane,
# trust only the public end-entity certificate explicitly in
# CurrentUser\TrustedPeople. This avoids root-CA trust semantics while still
# allowing Windows Authenticode chain validation for the ephemeral test signer.
# The TrustedPeople entry is test-only and is removed by workflow cleanup; it can
# never satisfy production signing acceptance.

$rsa = [System.Security.Cryptography.RSA]::Create(2048)
$raw = $null
$persisted = $null
$publicOnly = $null
$myStore = $null
$trustedPeopleStore = $null
$pfxBytes = $null
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

  Write-Host '03.22 test-cert phase=trust-public-cert-current-user-trusted-people'
  $publicBytes = $persisted.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert)
  $publicOnly = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($publicBytes)
  if ($publicOnly.HasPrivateKey) { throw '03.22 public-only trust certificate unexpectedly contains a private key.' }

  $trustedPeopleStore = [System.Security.Cryptography.X509Certificates.X509Store]::new(
    [System.Security.Cryptography.X509Certificates.StoreName]::TrustedPeople,
    [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
  )
  $trustedPeopleStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
  $trustedPeopleStore.Add($publicOnly)

  $trusted = Get-Item -LiteralPath "Cert:\CurrentUser\TrustedPeople\$($persisted.Thumbprint)" -ErrorAction Stop
  if ($null -eq $trusted) { throw '03.22 CurrentUser TrustedPeople test certificate was not persisted.' }
  if ($trusted.HasPrivateKey) { throw '03.22 TrustedPeople entry unexpectedly exposes private key material.' }

  Write-Host '03.22 test-cert phase=bind-test-only-environment'
  "VSN_0322_TEST_THUMB=$($persisted.Thumbprint)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "VSN_0322_TEST_SUBJECT=$($persisted.Subject)" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  "VSN_0322_TEST_TRUST_STORE=TrustedPeople" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
  [pscustomobject][ordered]@{
    valid=$true
    mode='test-only'
    subject=$persisted.Subject
    thumbprint=$persisted.Thumbprint
    has_private_key=$stored.HasPrivateKey
    trusted_current_user_trusted_people=$true
    trust_install_method='x509store-current-user-trusted-people-public-only'
    private_key_material_recorded=$false
    production_accepted=$false
  } | ConvertTo-Json -Depth 6
}
finally {
  if ($null -ne $trustedPeopleStore) { $trustedPeopleStore.Dispose() }
  if ($null -ne $myStore) { $myStore.Dispose() }
  if ($null -ne $publicOnly) { $publicOnly.Dispose() }
  if ($null -ne $persisted) { $persisted.Dispose() }
  if ($null -ne $raw) { $raw.Dispose() }
  if ($null -ne $rsa) { $rsa.Dispose() }
  if ($null -ne $pfxBytes) { [Array]::Clear($pfxBytes,0,$pfxBytes.Length) }
}
