param(
    [Parameter(Mandatory = $false)]
    [string]$PackagePath = ".fast-msix-product-out/vsn-fast-epoch-product-shell.msix",
    [Parameter(Mandatory = $false)]
    [string]$OutputDir = ".fast-msix-install-out",
    [Parameter(Mandatory = $false)]
    [string]$Subject = "CN=VSN Fast Epoch Fixture"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Push-Location $repoRoot
$rsa = $null
$raw = $null
$persisted = $null
$publicOnly = $null
$myStore = $null
$trustedPeopleStore = $null
$pfxBytes = $null
$thumbprint = $null
$installedPackageFullName = $null
try {
    $package = (Resolve-Path -LiteralPath $PackagePath -ErrorAction Stop).Path
    if (-not (Test-Path -LiteralPath $package -PathType Leaf)) { throw "MSIX package missing: $PackagePath" }
    $unsignedHash = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
    $out = Join-Path $repoRoot $OutputDir
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $out | Out-Null

    $signToolCandidates = @(Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter signtool.exe -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object FullName -Descending)
    if ($signToolCandidates.Count -lt 1) { throw 'SignTool x64 not found on Windows runner.' }
    $signTool = $signToolCandidates[0].FullName

    $existing = @(Get-AppxPackage -Name 'VSN.FastEpochFixture' -ErrorAction SilentlyContinue)
    foreach ($entry in $existing) {
        Remove-AppxPackage -Package $entry.PackageFullName -ErrorAction Stop
    }
    if (@(Get-AppxPackage -Name 'VSN.FastEpochFixture' -ErrorAction SilentlyContinue).Count -ne 0) {
        throw 'Fixture package identity remained installed before test.'
    }

    $rsa = [System.Security.Cryptography.RSA]::Create(2048)
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
    $raw = $request.CreateSelfSigned([DateTimeOffset]::UtcNow.AddMinutes(-5),[DateTimeOffset]::UtcNow.AddHours(4))
    if ($null -eq $raw -or -not $raw.HasPrivateKey) { throw 'Ephemeral MSIX test certificate has no private key.' }

    $password = [Guid]::NewGuid().ToString('N')
    $pfxBytes = $raw.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Pfx,$password)
    $flags = [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet -bor
             [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::UserKeySet
    $persisted = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($pfxBytes,$password,$flags)
    if (-not $persisted.HasPrivateKey) { throw 'Persisted MSIX test certificate lost its private key.' }
    $thumbprint = $persisted.Thumbprint

    $myStore = [System.Security.Cryptography.X509Certificates.X509Store]::new(
        [System.Security.Cryptography.X509Certificates.StoreName]::My,
        [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
    )
    $myStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    $myStore.Add($persisted)

    $publicBytes = $persisted.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Cert)
    $publicOnly = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new($publicBytes)
    if ($publicOnly.HasPrivateKey) { throw 'Public trust copy unexpectedly contains a private key.' }
    $trustedPeopleStore = [System.Security.Cryptography.X509Certificates.X509Store]::new(
        [System.Security.Cryptography.X509Certificates.StoreName]::TrustedPeople,
        [System.Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine
    )
    $trustedPeopleStore.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
    $trustedPeopleStore.Add($publicOnly)

    $stored = Get-Item -LiteralPath "Cert:\CurrentUser\My\$thumbprint" -ErrorAction Stop
    if (-not $stored.HasPrivateKey) { throw 'CurrentUser My certificate is not SignTool-usable.' }
    $trusted = Get-Item -LiteralPath "Cert:\LocalMachine\TrustedPeople\$thumbprint" -ErrorAction Stop
    if ($trusted.HasPrivateKey) { throw 'TrustedPeople public certificate unexpectedly exposes private key.' }

    & $signTool sign /fd SHA256 /sha1 $thumbprint $package
    if ($LASTEXITCODE -ne 0) { throw "SignTool MSIX test signing failed with exit code $LASTEXITCODE" }
    & $signTool verify /pa /v $package
    if ($LASTEXITCODE -ne 0) { throw "SignTool MSIX test verification failed with exit code $LASTEXITCODE" }
    $signedHash = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($signedHash -eq $unsignedHash) { throw 'MSIX digest did not change after test signing.' }

    Add-AppxPackage -Path $package -ErrorAction Stop
    $installed = @(Get-AppxPackage -Name 'VSN.FastEpochFixture' -ErrorAction Stop)
    if ($installed.Count -ne 1) { throw "Expected one registered fixture package, found $($installed.Count)." }
    $pkg = $installed[0]
    $installedPackageFullName = $pkg.PackageFullName
    if ([string]$pkg.Name -ne 'VSN.FastEpochFixture') { throw "Registered package identity mismatch: $($pkg.Name)" }
    if ([string]$pkg.Publisher -ne $Subject) { throw "Registered package publisher mismatch: $($pkg.Publisher)" }
    if ([string]$pkg.Version -ne '0.38.1.0') { throw "Registered fixture version mismatch: $($pkg.Version)" }

    Remove-AppxPackage -Package $installedPackageFullName -ErrorAction Stop
    $installedPackageFullName = $null
    if (@(Get-AppxPackage -Name 'VSN.FastEpochFixture' -ErrorAction SilentlyContinue).Count -ne 0) {
        throw 'Fixture package remained registered after Remove-AppxPackage.'
    }

    $evidence = [ordered]@{
        schema_version = 1
        lane = 'msix-store-preimplementation'
        mode = 'NON_ACCEPTANCE_PREIMPLEMENTATION'
        source_commit = (git rev-parse HEAD).Trim()
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        windows_image_os = $env:ImageOS
        windows_image_version = $env:ImageVersion
        fixture_identity = $true
        production_identity_present = $false
        test_certificate_used = $true
        test_certificate_subject = $Subject
        private_key_material_recorded = $false
        public_trust_store = 'LocalMachine\\TrustedPeople'
        package_unsigned_sha256 = $unsignedHash
        package_test_signed_sha256 = $signedHash
        test_signature_verified = $true
        package_registration_pass = $true
        registered_name = 'VSN.FastEpochFixture'
        registered_publisher = $Subject
        registered_version = '0.38.1.0'
        package_removal_pass = $true
        application_launch_tested = $false
        store_submission_performed = $false
        production_evidence_consumed = $false
        production_acceptance = $false
        canonical_state_changed = $false
        implementation_authority = $false
        standalone_full_feature_store_package_ready = $false
        required_runtime_boundary = 'external_or_preprovisioned_authenticated_agent'
        direct_installer_lane_must_remain = $true
    }
    $evidencePath = Join-Path $out 'evidence.json'
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $evidencePath -Encoding utf8NoBOM
    Write-Host ($evidence | ConvertTo-Json -Depth 8 -Compress)
    Write-Host 'PKG03_MSIX_TEST_INSTALL=PASS'
}
finally {
    if ($installedPackageFullName) {
        Remove-AppxPackage -Package $installedPackageFullName -ErrorAction SilentlyContinue
    }
    if ($null -ne $trustedPeopleStore) { $trustedPeopleStore.Dispose() }
    if ($null -ne $myStore) { $myStore.Dispose() }
    if ($thumbprint) {
        Remove-Item -LiteralPath "Cert:\CurrentUser\My\$thumbprint" -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath "Cert:\LocalMachine\TrustedPeople\$thumbprint" -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $publicOnly) { $publicOnly.Dispose() }
    if ($null -ne $persisted) { $persisted.Dispose() }
    if ($null -ne $raw) { $raw.Dispose() }
    if ($null -ne $rsa) { $rsa.Dispose() }
    if ($null -ne $pfxBytes) { [Array]::Clear($pfxBytes,0,$pfxBytes.Length) }
    Pop-Location
}

$global:LASTEXITCODE = 0
