param(
    [Parameter(Mandatory = $false)]
    [string]$OutputDir = ".fast-msix-product-out",
    [Parameter(Mandatory = $false)]
    [string]$ProductExe = "target/release/VSN Dev Platform.exe"
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Push-Location $repoRoot
try {
    $product = (Resolve-Path -LiteralPath $ProductExe -ErrorAction Stop).Path
    if (-not (Test-Path -LiteralPath $product -PathType Leaf)) { throw "Product executable missing: $ProductExe" }
    $productInfo = Get-Item -LiteralPath $product
    if ($productInfo.Length -lt 100000) { throw "Product executable is unexpectedly small: $($productInfo.Length) bytes" }
    $productSig = Get-AuthenticodeSignature -LiteralPath $product
    if ([string]$productSig.Status -ne 'NotSigned') { throw "Preimplementation product executable must be unsigned; status=$($productSig.Status)" }
    $productHash = (Get-FileHash -LiteralPath $product -Algorithm SHA256).Hash.ToLowerInvariant()

    $out = Join-Path $repoRoot $OutputDir
    $render = Join-Path $out "render"
    $stage = Join-Path $out "stage"
    $unpacked = Join-Path $out "unpacked"
    $package = Join-Path $out "vsn-fast-epoch-product-shell.msix"
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $render,$stage,(Join-Path $stage "Assets") | Out-Null

    & python scripts/ci/pkg03-msix-manifest-preimplementation.py --repo-root . --fixture --output-dir $render
    if ($LASTEXITCODE -ne 0) { throw "MSIX manifest fixture renderer failed." }
    Copy-Item -LiteralPath (Join-Path $render "AppxManifest.xml") -Destination (Join-Path $stage "AppxManifest.xml")
    Copy-Item -LiteralPath $product -Destination (Join-Path $stage "VSN Dev Platform.exe")

    $assetScript = @'
import pathlib, struct, sys, zlib
root=pathlib.Path(sys.argv[1])
root.mkdir(parents=True, exist_ok=True)
def chunk(kind,data):
    return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
def write_png(name,w,h):
    pixel=bytes((0,96,57,255))
    raw=b''.join(b'\x00'+pixel*w for _ in range(h))
    png=(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(raw,9))+chunk(b'IEND',b''))
    (root/name).write_bytes(png)
write_png('StoreLogo.png',50,50)
write_png('Square44x44Logo.png',44,44)
write_png('Square150x150Logo.png',150,150)
write_png('Wide310x150Logo.png',310,150)
'@
    $assetScript | python - (Join-Path $stage "Assets")
    if ($LASTEXITCODE -ne 0) { throw "Synthetic Store asset generation failed." }

    $makeAppxCandidates = @(Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Recurse -Filter MakeAppx.exe -File -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\MakeAppx\.exe$' } |
        Sort-Object FullName -Descending)
    if ($makeAppxCandidates.Count -lt 1) { throw "MakeAppx x64 not found on runner." }
    $makeAppx = $makeAppxCandidates[0].FullName

    & $makeAppx pack /d $stage /p $package /o
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx product-shell pack failed with exit code $LASTEXITCODE" }
    if (-not (Test-Path -LiteralPath $package -PathType Leaf)) { throw "Product-shell MSIX was not created." }

    & $makeAppx unpack /p $package /d $unpacked /o
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx product-shell unpack failed with exit code $LASTEXITCODE" }

    $unpackedExe = Join-Path $unpacked "VSN Dev Platform.exe"
    if (-not (Test-Path -LiteralPath $unpackedExe -PathType Leaf)) { throw "Unpacked product executable missing." }
    $unpackedHash = (Get-FileHash -LiteralPath $unpackedExe -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($unpackedHash -ne $productHash) { throw "Product executable hash changed during MSIX round trip." }
    if ((Get-Item -LiteralPath $unpackedExe).Length -ne $productInfo.Length) { throw "Product executable size changed during MSIX round trip." }

    if (Test-Path -LiteralPath (Join-Path $unpacked 'AppxSignature.p7x')) { throw "Product-shell fixture unexpectedly contains AppxSignature.p7x." }
    $manifestText = Get-Content -LiteralPath (Join-Path $unpacked 'AppxManifest.xml') -Raw
    if (-not $manifestText.Contains('Name="VSN.FastEpochFixture"')) { throw "Fixture identity missing after product-shell package round trip." }
    if (-not $manifestText.Contains('Publisher="CN=VSN Fast Epoch Fixture"')) { throw "Fixture publisher missing after product-shell package round trip." }
    if ($manifestText.Contains('PARTNER_CENTER') -or $manifestText.Contains('${')) { throw "Production placeholder leaked into product-shell fixture manifest." }

    $packageInfo = Get-Item -LiteralPath $package
    $evidence = [ordered]@{
        schema_version = 1
        lane = 'msix-store-preimplementation'
        mode = 'NON_ACCEPTANCE_PREIMPLEMENTATION'
        source_commit = (git rev-parse HEAD).Trim()
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        windows_image_os = $env:ImageOS
        windows_image_version = $env:ImageVersion
        makeappx_path = $makeAppx
        makeappx_file_version = (Get-Item -LiteralPath $makeAppx).VersionInfo.FileVersion
        fixture_identity = $true
        production_identity_present = $false
        product_binary_used = $true
        product_binary_build_mode = 'tauri-release-no-bundle'
        product_binary_source = $ProductExe
        product_binary_size_bytes = [long]$productInfo.Length
        product_binary_sha256 = $productHash
        product_binary_authenticode_status = [string]$productSig.Status
        unpacked_product_binary_sha256 = $unpackedHash
        product_binary_roundtrip_hash_match = $true
        package_file = $packageInfo.Name
        package_size_bytes = [long]$packageInfo.Length
        package_sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
        package_signature_present = $false
        package_signed = $false
        store_submission_performed = $false
        production_evidence_consumed = $false
        canonical_state_changed = $false
        implementation_authority = $false
        standalone_full_feature_store_package_ready = $false
        required_runtime_boundary = 'external_or_preprovisioned_authenticated_agent'
        direct_installer_lane_must_remain = $true
        package_roundtrip_unpack_pass = $true
    }
    $evidencePath = Join-Path $out 'evidence.json'
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $evidencePath -Encoding utf8NoBOM
    Write-Host ($evidence | ConvertTo-Json -Depth 8 -Compress)
    Write-Host 'PKG03_MSIX_PRODUCT_SHELL_PACKAGE=PASS'
}
finally {
    Pop-Location
}
