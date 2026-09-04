Set-StrictMode -Version Latest

function Get-VsnPersistedKeyDescriptor {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [System.Security.Cryptography.X509Certificates.X509Certificate2]$Certificate
    )

    $key = $null
    try {
        $key = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($Certificate)
        if ($null -eq $key) {
            $key = [System.Security.Cryptography.X509Certificates.ECDsaCertificateExtensions]::GetECDsaPrivateKey($Certificate)
        }
        if ($null -eq $key) {
            throw 'Production certificate private-key provider could not be resolved.'
        }

        if ($key -is [System.Security.Cryptography.RSACng] -or $key -is [System.Security.Cryptography.ECDsaCng]) {
            $cngKey = $key.Key
            $name = [string]$cngKey.KeyName
            $provider = [string]$cngKey.Provider.Provider
            if ([string]::IsNullOrWhiteSpace($name) -or [string]::IsNullOrWhiteSpace($provider)) {
                throw 'Persisted CNG private key did not expose a stable key/provider identity.'
            }
            return [pscustomobject]@{
                Kind = 'CNG'
                ProviderName = $provider
                KeyName = $name
                ProviderType = $null
                ContainerName = $null
                KeyNumber = $null
            }
        }

        if ($key -is [System.Security.Cryptography.RSACryptoServiceProvider]) {
            $info = $key.CspKeyContainerInfo
            if ([string]::IsNullOrWhiteSpace([string]$info.ProviderName) -or [string]::IsNullOrWhiteSpace([string]$info.KeyContainerName)) {
                throw 'Persisted CAPI private key did not expose a stable provider/container identity.'
            }
            return [pscustomobject]@{
                Kind = 'CAPI'
                ProviderName = [string]$info.ProviderName
                KeyName = $null
                ProviderType = [int]$info.ProviderType
                ContainerName = [string]$info.KeyContainerName
                KeyNumber = [int]$info.KeyNumber
            }
        }

        throw "Unsupported production private-key provider type: $($key.GetType().FullName)"
    }
    finally {
        if ($null -ne $key) { $key.Dispose() }
    }
}

function New-VsnCapiParameters {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][object]$Descriptor
    )
    $parameters = [System.Security.Cryptography.CspParameters]::new(
        [int]$Descriptor.ProviderType,
        [string]$Descriptor.ProviderName,
        [string]$Descriptor.ContainerName
    )
    $parameters.Flags = [System.Security.Cryptography.CspProviderFlags]::UseExistingKey
    $parameters.KeyNumber = [int]$Descriptor.KeyNumber
    return $parameters
}

function Test-VsnPersistedKeyExists {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][object]$Descriptor
    )

    if ([string]$Descriptor.Kind -eq 'CNG') {
        $opened = $null
        try {
            $provider = [System.Security.Cryptography.CngProvider]::new([string]$Descriptor.ProviderName)
            $opened = [System.Security.Cryptography.CngKey]::Open(
                [string]$Descriptor.KeyName,
                $provider,
                [System.Security.Cryptography.CngKeyOpenOptions]::None
            )
            return $true
        }
        catch [System.Security.Cryptography.CryptographicException] {
            return $false
        }
        finally {
            if ($null -ne $opened) { $opened.Dispose() }
        }
    }

    if ([string]$Descriptor.Kind -eq 'CAPI') {
        $opened = $null
        try {
            $opened = [System.Security.Cryptography.RSACryptoServiceProvider]::new((New-VsnCapiParameters -Descriptor $Descriptor))
            [void]$opened.KeySize
            return $true
        }
        catch [System.Security.Cryptography.CryptographicException] {
            return $false
        }
        finally {
            if ($null -ne $opened) { $opened.Dispose() }
        }
    }

    throw "Unsupported persisted-key descriptor kind: $($Descriptor.Kind)"
}

function Remove-VsnPersistedKey {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)][object]$Descriptor
    )

    if ([string]$Descriptor.Kind -eq 'CNG') {
        $opened = $null
        try {
            $provider = [System.Security.Cryptography.CngProvider]::new([string]$Descriptor.ProviderName)
            $opened = [System.Security.Cryptography.CngKey]::Open(
                [string]$Descriptor.KeyName,
                $provider,
                [System.Security.Cryptography.CngKeyOpenOptions]::None
            )
            $opened.Delete()
        }
        catch [System.Security.Cryptography.CryptographicException] {
            if (Test-VsnPersistedKeyExists -Descriptor $Descriptor) { throw }
        }
        finally {
            if ($null -ne $opened) { $opened.Dispose() }
        }
    }
    elseif ([string]$Descriptor.Kind -eq 'CAPI') {
        $opened = $null
        try {
            $opened = [System.Security.Cryptography.RSACryptoServiceProvider]::new((New-VsnCapiParameters -Descriptor $Descriptor))
            $opened.PersistKeyInCsp = $false
            $opened.Clear()
        }
        catch [System.Security.Cryptography.CryptographicException] {
            if (Test-VsnPersistedKeyExists -Descriptor $Descriptor) { throw }
        }
        finally {
            if ($null -ne $opened) { $opened.Dispose() }
        }
    }
    else {
        throw "Unsupported persisted-key descriptor kind: $($Descriptor.Kind)"
    }

    if (Test-VsnPersistedKeyExists -Descriptor $Descriptor) {
        throw 'Persisted production private-key container remained accessible after explicit cleanup.'
    }
    return $true
}
