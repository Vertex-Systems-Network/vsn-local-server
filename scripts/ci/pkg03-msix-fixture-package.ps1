param(
    [Parameter(Mandatory = $false)]
    [string]$OutputDir = ".fast-msix-out"
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Push-Location $repoRoot
try {
    $out = Join-Path $repoRoot $OutputDir
    $render = Join-Path $out "render"
    $stage = Join-Path $out "stage"
    $unpacked = Join-Path $out "unpacked"
    $package = Join-Path $out "vsn-fast-epoch-fixture.msix"
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $render,$stage,(Join-Path $stage "Assets") | Out-Null

    & python scripts/ci/pkg03-msix-manifest-preimplementation.py --repo-root . --fixture --output-dir $render
    if ($LASTEXITCODE -ne 0) { throw "MSIX manifest fixture renderer failed." }
    Copy-Item -LiteralPath (Join-Path $render "Package.appxmanifest") -Destination (Join-Path $stage "Package.appxmanifest")

    $fixtureExe = Join-Path $env:SystemRoot "System32\where.exe"
    if (-not (Test-Path -LiteralPath $fixtureExe -PathType Leaf)) { throw "Windows fixture PE not found: $fixtureExe" }
    Copy-Item -LiteralPath $fixtureExe -Destination (Join-Path $stage "VSN Dev Platform.exe")

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
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx pack failed with exit code $LASTEXITCODE" }
    if (-not (Test-Path -LiteralPath $package -PathType Leaf)) { throw "Fixture MSIX was not created." }

    & $makeAppx unpack /p $package /d $unpacked /o
    if ($LASTEXITCODE -ne 0) { throw "MakeAppx unpack failed with exit code $LASTEXITCODE" }

    $expected = @(
        'Package.appxmanifest',
        'VSN Dev Platform.exe',
        'Assets\StoreLogo.png',
        'Assets\Square44x44Logo.png',
        'Assets\Square150x150Logo.png',
        'Assets\Wide310x150Logo.png'
    )
    foreach ($relative in $expected) {
        if (-not (Test-Path -LiteralPath (Join-Path $unpacked $relative) -PathType Leaf)) {
            throw "Unpacked fixture MSIX missing expected file: $relative"
        }
    }
    if (Test-Path -LiteralPath (Join-Path $unpacked 'AppxSignature.p7x')) {
        throw "Fixture package unexpectedly contains AppxSignature.p7x."
    }

    $manifestText = Get-Content -LiteralPath (Join-Path $unpacked 'Package.appxmanifest') -Raw
    if (-not $manifestText.Contains('Name="VSN.FastEpochFixture"')) { throw "Fixture identity missing after package round trip." }
    if (-not $manifestText.Contains('Publisher="CN=VSN Fast Epoch Fixture"')) { throw "Fixture publisher missing after package round trip." }
    if ($manifestText.Contains('PARTNER_CENTER') -or $manifestText.Contains('${')) { throw "Production placeholder leaked into packaged fixture manifest." }

    $packageInfo = Get-Item -LiteralPath $package
    $fixtureExeInfo = Get-Item -LiteralPath $fixtureExe
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
        product_binary_used = $false
        fixture_pe_source = $fixtureExe
        fixture_pe_sha256 = (Get-FileHash -LiteralPath $fixtureExe -Algorithm SHA256).Hash.ToLowerInvariant()
        package_file = $packageInfo.Name
        package_size_bytes = [long]$packageInfo.Length
        package_sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
        package_signature_present = $false
        package_signed = $false
        store_submission_performed = $false
        production_evidence_consumed = $false
        canonical_state_changed = $false
        implementation_authority = $false
        required_runtime_boundary = 'external_or_preprovisioned_authenticated_agent'
        direct_installer_lane_must_remain = $true
        package_roundtrip_unpack_pass = $true
    }
    $evidencePath = Join-Path $out 'evidence.json'
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $evidencePath -Encoding utf8NoBOM
    Write-Host ($evidence | ConvertTo-Json -Depth 8 -Compress)
    Write-Host 'PKG03_MSIX_WINDOWS_FIXTURE_PACKAGE=PASS'
}
finally {
    Pop-Location
}
