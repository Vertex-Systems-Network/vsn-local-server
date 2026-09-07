[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$CandidateRoot,

    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-f]{40}$')]
    [string]$ExpectedSourceSha,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedVersion,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedProduct,

    [string]$EvidencePath = '',

    [string]$PublishedNotesPath = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = (Resolve-Path -LiteralPath $CandidateRoot).Path
$manifestPath = Join-Path $root 'candidate-manifest.json'
$sumsPath = Join-Path $root 'SHA256SUMS.txt'
$draftNotesPath = Join-Path $root 'RELEASE-NOTES-DRAFT.md'
$assetsPath = Join-Path $root 'release-assets'

foreach ($requiredPath in @($manifestPath, $sumsPath, $draftNotesPath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Unsigned beta candidate is missing required file: $requiredPath"
    }
}
if (-not (Test-Path -LiteralPath $assetsPath -PathType Container)) {
    throw "Unsigned beta candidate is missing release-assets directory: $assetsPath"
}

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ([int]$manifest.schema_version -ne 1) { throw 'Unexpected candidate manifest schema.' }
if ([string]$manifest.release_kind -ne 'unsigned-beta-pre-release-candidate') { throw 'Unexpected candidate release kind.' }
if ([string]$manifest.product -ne $ExpectedProduct) { throw "Candidate product mismatch: $($manifest.product)" }
if ([string]$manifest.version -ne $ExpectedVersion) { throw "Candidate version mismatch: $($manifest.version)" }
if ([string]$manifest.source_commit -ne $ExpectedSourceSha) { throw "Candidate source mismatch: $($manifest.source_commit)" }
if ([string]$manifest.source_branch -ne 'main') { throw "Candidate source branch mismatch: $($manifest.source_branch)" }
if ([string]$manifest.application_identifier -ne 'dev.vsn.platform') { throw "Candidate application identifier mismatch: $($manifest.application_identifier)" }
if ([string]$manifest.intended_future_signing_provider -ne 'SignPath Foundation') { throw 'Unexpected future signing provider.' }

foreach ($falseField in @('production_signed','production_accepted','pkg03_0322_accepted','stable_1_0','publish_authorized')) {
    if ([bool]$manifest.$falseField) { throw "Candidate fail-closed field unexpectedly true: $falseField" }
}

$expectedNames = @(
    "VSN-Dev-Platform-$ExpectedVersion-UNSIGNED.exe",
    "VSN-Dev-Platform-$ExpectedVersion-UNSIGNED.msi",
    "VSN-Dev-Platform-$ExpectedVersion-current-user-UNSIGNED.exe",
    "VSN-Dev-Platform-$ExpectedVersion-per-machine-UNSIGNED.exe"
) | Sort-Object

$actualFiles = @(Get-ChildItem -LiteralPath $assetsPath -File | Sort-Object Name)
$actualNames = @($actualFiles | ForEach-Object { $_.Name })
$nameDiff = @(Compare-Object $expectedNames $actualNames)
if ($actualNames.Count -ne $expectedNames.Count -or $nameDiff.Count -ne 0) {
    throw "Unsigned beta asset set mismatch. Expected=$($expectedNames -join ',') Actual=$($actualNames -join ',')"
}

$manifestRows = @($manifest.files)
if ($manifestRows.Count -ne 4) { throw "Expected four manifest file rows, got $($manifestRows.Count)." }
$manifestByName = @{}
foreach ($row in $manifestRows) {
    $name = [string]$row.file_name
    if ($manifestByName.ContainsKey($name)) { throw "Duplicate manifest file row: $name" }
    if ([string]$row.authenticode_status -ne 'NotSigned') { throw "$name manifest Authenticode status is not NotSigned." }
    $manifestByName[$name] = $row
}

$sums = @{}
foreach ($line in @(Get-Content -LiteralPath $sumsPath)) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    if ($line -notmatch '^([0-9a-fA-F]{64})\s{2}(.+)$') { throw "Malformed SHA256SUMS line: $line" }
    $hash = $Matches[1].ToLowerInvariant()
    $name = $Matches[2]
    if ($sums.ContainsKey($name)) { throw "Duplicate SHA256SUMS entry: $name" }
    $sums[$name] = $hash
}
if ($sums.Count -ne 4) { throw "Expected four SHA256SUMS entries, got $($sums.Count)." }

$verifiedRows = @()
foreach ($file in $actualFiles) {
    if (-not $manifestByName.ContainsKey($file.Name)) { throw "Manifest row missing for $($file.Name)." }
    if (-not $sums.ContainsKey($file.Name)) { throw "SHA256SUMS row missing for $($file.Name)." }

    $sig = Get-AuthenticodeSignature -LiteralPath $file.FullName
    if ([string]$sig.Status -ne 'NotSigned') {
        throw "$($file.Name) unexpectedly has Authenticode status $($sig.Status)."
    }

    $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $row = $manifestByName[$file.Name]
    if ($hash -ne ([string]$row.sha256).ToLowerInvariant()) { throw "$($file.Name) hash does not match candidate manifest." }
    if ($hash -ne $sums[$file.Name]) { throw "$($file.Name) hash does not match SHA256SUMS." }
    if ([long]$file.Length -ne [long]$row.size_bytes) { throw "$($file.Name) size does not match candidate manifest." }

    $verifiedRows += [ordered]@{
        file_name = $file.Name
        size_bytes = [long]$file.Length
        sha256 = $hash
        authenticode_status = 'NotSigned'
    }
}

$draftNotes = Get-Content -LiteralPath $draftNotesPath -Raw
foreach ($requiredNote in @('UNSIGNED BETA / PRE-RELEASE CANDIDATE','NOT production-signed','does NOT satisfy PKG-03 task 03.22')) {
    if (-not $draftNotes.Contains($requiredNote)) { throw "Candidate draft release note invariant missing: $requiredNote" }
}

$publishedNotesHashesMatch = $null
if (-not [string]::IsNullOrWhiteSpace($PublishedNotesPath)) {
    if (-not (Test-Path -LiteralPath $PublishedNotesPath -PathType Leaf)) {
        throw "Published release notes are missing: $PublishedNotesPath"
    }
    $publishedNotes = Get-Content -LiteralPath $PublishedNotesPath -Raw
    foreach ($name in $expectedNames) {
        $expectedChecksumLine = "$($sums[$name])  $name"
        if (-not $publishedNotes.Contains($expectedChecksumLine)) {
            throw "Published release notes do not bind the candidate checksum for $name."
        }
    }
    $publishedNotesHashesMatch = $true
}

$evidence = [ordered]@{
    schema_version = 1
    verification = 'signpath-unsigned-beta-publication-handoff'
    source_commit = $ExpectedSourceSha
    product = $ExpectedProduct
    version = $ExpectedVersion
    asset_count = 4
    all_assets_authenticode_not_signed = $true
    manifest_fail_closed = $true
    sha256sums_match = $true
    published_release_notes_hashes_match = $publishedNotesHashesMatch
    publication_authorized = $false
    files = $verifiedRows
}

if (-not [string]::IsNullOrWhiteSpace($EvidencePath)) {
    $parent = Split-Path -Parent $EvidencePath
    if (-not [string]::IsNullOrWhiteSpace($parent)) { New-Item -ItemType Directory -Force $parent | Out-Null }
    $evidence | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $EvidencePath -Encoding utf8NoBOM
}

Write-Host "SIGNPATH_UNSIGNED_BETA_VERIFY status=PASS source=$ExpectedSourceSha version=$ExpectedVersion assets=4 authenticode=NotSigned notes_hashes_bound=$publishedNotesHashesMatch publication_authorized=false"
