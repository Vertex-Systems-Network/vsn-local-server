$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'pkg03-0322-production-key-cleanup.ps1')

$out=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../dist-pkg03/03.22-key-cleanup-probe'))
New-Item -ItemType Directory -Force $out | Out-Null
$password=[Guid]::NewGuid().ToString('N')
$pfx=$null;$memRsa=$null;$memCert=$null;$cert=$null;$store=$null;$thumb=$null;$descriptor=$null

try{
  $memRsa=[Security.Cryptography.RSA]::Create(2048)
  $req=[Security.Cryptography.X509Certificates.CertificateRequest]::new('CN=VSN CI PKG03 Key Cleanup Probe',$memRsa,[Security.Cryptography.HashAlgorithmName]::SHA256,[Security.Cryptography.RSASignaturePadding]::Pkcs1)
  $memCert=$req.CreateSelfSigned([DateTimeOffset]::UtcNow.AddMinutes(-5),[DateTimeOffset]::UtcNow.AddDays(1))
  $pfx=$memCert.Export([Security.Cryptography.X509Certificates.X509ContentType]::Pfx,$password)
  $flags=[Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet -bor [Security.Cryptography.X509Certificates.X509KeyStorageFlags]::UserKeySet
  $cert=[Security.Cryptography.X509Certificates.X509Certificate2]::new($pfx,$password,$flags)
  if(-not $cert.HasPrivateKey){throw 'Imported test certificate has no private key.'}
  $descriptor=Get-VsnPersistedKeyDescriptor -Certificate $cert
  $thumb=([string]$cert.Thumbprint -replace '\s','').ToUpperInvariant()

  $store=[Security.Cryptography.X509Certificates.X509Store]::new([Security.Cryptography.X509Certificates.StoreName]::My,[Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser)
  $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
  $store.Add($cert)

  # Reproduce the pre-hardening cleanup shape first, then prove whether the key survived.
  $store.Dispose();$store=$null
  Remove-Item -LiteralPath "Cert:\CurrentUser\My\$thumb" -Force
  $cert.Dispose();$cert=$null
  [Array]::Clear($pfx,0,$pfx.Length)
  $persisted=Test-VsnPersistedKeyExists -Descriptor $descriptor

  # Exercise the exact helper that the trusted production signer will use.
  $cleanupOk=Remove-VsnPersistedKey -Descriptor $descriptor
  $left=Test-VsnPersistedKeyExists -Descriptor $descriptor
  if(-not $cleanupOk -or $left){throw 'Trusted persisted-key cleanup helper failed its generated-PFX regression.'}

  $e=[ordered]@{
    schema_version=2;package_id='PKG-03';task_id='03.22';probe='production-signing-key-container-cleanup';
    generated_test_certificate_only=$true;production_credentials_used=$false;import_flags=@('PersistKeySet','UserKeySet');
    provider_kind=[string]$descriptor.Kind;provider_name=[string]$descriptor.ProviderName;
    certificate_removed=$true;pfx_bytes_cleared=$true;
    persisted_key_accessible_after_pre_hardening_cleanup=[bool]$persisted;
    trusted_cleanup_helper_removed_provider_key=$true;
    persisted_key_accessible_after_trusted_cleanup=$false;
    runner=[ordered]@{os=$env:RUNNER_OS;arch=$env:RUNNER_ARCH;image_os=$env:ImageOS;image_version=$env:ImageVersion};
    conclusion='trusted-cleanup-helper-verified'
  }
  $ep=Join-Path $out 'evidence.json'
  $e|ConvertTo-Json -Depth 8|Set-Content $ep -Encoding utf8NoBOM
  $d=(Get-FileHash $ep -Algorithm SHA256).Hash.ToLowerInvariant()
  "$d  evidence.json"|Set-Content (Join-Path $out 'evidence.json.sha256') -Encoding utf8NoBOM
  Write-Host "provider=$($descriptor.Kind)/$($descriptor.ProviderName) survived_pre_hardening_cleanup=$persisted trusted_cleanup_ok=$cleanupOk"
}
finally{
  if($store){$store.Dispose()}
  if($thumb -and (Test-Path "Cert:\CurrentUser\My\$thumb")){Remove-Item "Cert:\CurrentUser\My\$thumb" -Force -ErrorAction SilentlyContinue}
  if($cert){$cert.Dispose()}
  if($descriptor){try{Remove-VsnPersistedKey -Descriptor $descriptor|Out-Null}catch{Write-Warning $_}}
  if($memCert){$memCert.Dispose()};if($memRsa){$memRsa.Dispose()};if($pfx){[Array]::Clear($pfx,0,$pfx.Length)}
}