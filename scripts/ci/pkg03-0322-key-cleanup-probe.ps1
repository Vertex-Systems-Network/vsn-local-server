$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$out=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../dist-pkg03/03.22-key-cleanup-probe'))
New-Item -ItemType Directory -Force $out | Out-Null

function Open-Cng([string]$Name,[string]$Provider){
  try{return [Security.Cryptography.CngKey]::Open($Name,[Security.Cryptography.CngProvider]::new($Provider),[Security.Cryptography.CngKeyOpenOptions]::None)}catch [Security.Cryptography.CryptographicException]{return $null}
}
function Open-Capi([int]$Type,[string]$Provider,[string]$Container,[int]$KeyNumber){
  try{
    $p=[Security.Cryptography.CspParameters]::new($Type,$Provider,$Container);$p.Flags=[Security.Cryptography.CspProviderFlags]::UseExistingKey;$p.KeyNumber=$KeyNumber
    return [Security.Cryptography.RSACryptoServiceProvider]::new($p)
  }catch [Security.Cryptography.CryptographicException]{return $null}
}

$password=[Guid]::NewGuid().ToString('N');$pfx=$null;$memRsa=$null;$memCert=$null;$cert=$null;$key=$null;$store=$null;$thumb=$null
$kind=$null;$provider=$null;$keyName=$null;$capiType=0;$capiContainer=$null;$capiKeyNumber=0
try{
  $memRsa=[Security.Cryptography.RSA]::Create(2048)
  $req=[Security.Cryptography.X509Certificates.CertificateRequest]::new('CN=VSN CI PKG03 Key Cleanup Probe',$memRsa,[Security.Cryptography.HashAlgorithmName]::SHA256,[Security.Cryptography.RSASignaturePadding]::Pkcs1)
  $memCert=$req.CreateSelfSigned([DateTimeOffset]::UtcNow.AddMinutes(-5),[DateTimeOffset]::UtcNow.AddDays(1))
  $pfx=$memCert.Export([Security.Cryptography.X509Certificates.X509ContentType]::Pfx,$password)
  $flags=[Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet -bor [Security.Cryptography.X509Certificates.X509KeyStorageFlags]::UserKeySet
  $cert=[Security.Cryptography.X509Certificates.X509Certificate2]::new($pfx,$password,$flags)
  if(-not $cert.HasPrivateKey){throw 'Imported test certificate has no private key.'}
  $thumb=([string]$cert.Thumbprint -replace '\s','').ToUpperInvariant()
  $store=[Security.Cryptography.X509Certificates.X509Store]::new([Security.Cryptography.X509Certificates.StoreName]::My,[Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser)
  $store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite);$store.Add($cert)
  $key=[Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($cert)
  if($key -is [Security.Cryptography.RSACng]){$kind='CNG';$keyName=[string]$key.Key.KeyName;$provider=[string]$key.Key.Provider.Provider}
  elseif($key -is [Security.Cryptography.RSACryptoServiceProvider]){$kind='CAPI';$i=$key.CspKeyContainerInfo;$capiType=[int]$i.ProviderType;$provider=[string]$i.ProviderName;$capiContainer=[string]$i.KeyContainerName;$capiKeyNumber=[int]$i.KeyNumber}
  else{throw "Unsupported RSA provider: $($key.GetType().FullName)"}

  # Reproduce current trusted-job cleanup shape.
  $store.Dispose();$store=$null
  Remove-Item -LiteralPath "Cert:\CurrentUser\My\$thumb" -Force
  $key.Dispose();$key=$null;$cert.Dispose();$cert=$null;[Array]::Clear($pfx,0,$pfx.Length)

  if($kind -eq 'CNG'){
    $probe=Open-Cng $keyName $provider;$persisted=($null -ne $probe)
    if($probe){$probe.Delete();$probe.Dispose()};$again=Open-Cng $keyName $provider;$left=($null -ne $again);if($again){$again.Dispose()}
  }else{
    $probe=Open-Capi $capiType $provider $capiContainer $capiKeyNumber;$persisted=($null -ne $probe)
    if($probe){$probe.PersistKeyInCsp=$false;$probe.Clear();$probe.Dispose()};$again=Open-Capi $capiType $provider $capiContainer $capiKeyNumber;$left=($null -ne $again);if($again){$again.Dispose()}
  }
  if($left){throw 'Explicit provider cleanup failed to remove generated test key.'}

  $e=[ordered]@{schema_version=1;package_id='PKG-03';task_id='03.22';probe='production-signing-key-container-cleanup';generated_test_certificate_only=$true;production_credentials_used=$false;import_flags=@('PersistKeySet','UserKeySet');provider_kind=$kind;provider_name=$provider;certificate_removed=$true;pfx_bytes_cleared=$true;persisted_key_accessible_after_current_cleanup=[bool]$persisted;explicit_provider_cleanup_succeeded=$true;runner=[ordered]@{os=$env:RUNNER_OS;arch=$env:RUNNER_ARCH;image_os=$env:ImageOS;image_version=$env:ImageVersion};conclusion=$(if($persisted){'hardening-required'}else{'current-cleanup-removed-provider-key-on-this-runner'})}
  $ep=Join-Path $out 'evidence.json';$e|ConvertTo-Json -Depth 8|Set-Content $ep -Encoding utf8NoBOM
  $d=(Get-FileHash $ep -Algorithm SHA256).Hash.ToLowerInvariant();"$d  evidence.json"|Set-Content (Join-Path $out 'evidence.json.sha256') -Encoding utf8NoBOM
  Write-Host "provider=$kind/$provider persisted_after_current_cleanup=$persisted explicit_cleanup_ok=$(-not $left)"
}finally{
  if($store){$store.Dispose()};if($thumb -and (Test-Path "Cert:\CurrentUser\My\$thumb")){Remove-Item "Cert:\CurrentUser\My\$thumb" -Force -ErrorAction SilentlyContinue}
  if($key){$key.Dispose()};if($cert){$cert.Dispose()};if($memCert){$memCert.Dispose()};if($memRsa){$memRsa.Dispose()};if($pfx){[Array]::Clear($pfx,0,$pfx.Length)}
}