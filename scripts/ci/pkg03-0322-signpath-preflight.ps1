param()

$ErrorActionPreference = 'Stop'

$workflowPath = '.github/workflows/pkg03-0322-signpath-preflight.yml'
$policyPath = 'docs/CODE-SIGNING-POLICY.md'
$integrationPath = 'docs/SIGNPATH-FOUNDATION-INTEGRATION.md'

foreach ($path in @($workflowPath, $policyPath, $integrationPath)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required SignPath preparation file is missing: $path"
    }
}

$workflow = Get-Content -LiteralPath $workflowPath -Raw
$policy = Get-Content -LiteralPath $policyPath -Raw
$integration = Get-Content -LiteralPath $integrationPath -Raw

# Preparation must never consume signing credentials.
foreach ($name in @(
    'SIGNPATH_API_TOKEN',
    'SIGNPATH_ORGANIZATION_ID',
    'SIGNPATH_PROJECT_SLUG',
    'SIGNPATH_SIGNING_POLICY_SLUG',
    'SIGNPATH_ARTIFACT_CONFIGURATION_SLUG',
    'VSN_SIGNING_PFX_B64',
    'VSN_SIGNING_PFX_PASSWORD'
)) {
    if (Test-Path "Env:$name") {
        throw "$name must not be present in the mutable SignPath preparation workflow environment."
    }
}

# Remove comments before checking for executable provider submission wiring.
$activeLines = @(
    $workflow -split "`r?`n" |
        Where-Object { $_ -notmatch '^\s*#' }
)
$activeWorkflow = $activeLines -join "`n"

if ($activeWorkflow -match 'signpath/github-action-submit-signing-request') {
    throw 'SignPath submission action became executable before provider approval/trusted-main reconciliation.'
}
if ($activeWorkflow -match '\$\{\{\s*secrets\.SIGNPATH_API_TOKEN') {
    throw 'Mutable preparation workflow contains executable SignPath API-token access.'
}
if ($activeWorkflow -match '\$\{\{\s*vars\.SIGNPATH_') {
    throw 'Mutable preparation workflow contains executable SignPath production-variable access.'
}
if ($activeWorkflow -match '^\s*environment\s*:\s*production-signing\s*$') {
    throw 'Mutable preparation workflow must not bind the protected production-signing Environment.'
}

foreach ($required in @(
    "provider_approval='pending'",
    'production_submission_enabled=$false',
    "production_accepted=$false",
    'Refuse mutable-branch production submission'
)) {
    if (-not $workflow.Contains($required)) {
        throw "Fail-closed preparation marker missing: $required"
    }
}

if ($policy -notmatch '(?i)code signing') {
    throw 'Public code-signing policy does not identify its signing purpose.'
}
if ($integration -notmatch '(?i)SignPath') {
    throw 'Integration document does not identify SignPath.'
}
if ($integration -notmatch '(?i)03\.22') {
    throw 'Integration document is not bound to PKG-03 03.22.'
}

$tracked = @(git status --porcelain=v1 --untracked-files=no)
if ($LASTEXITCODE -ne 0) { throw 'git status failed.' }
if ($tracked.Count -ne 0) {
    throw "Tracked repository drift detected during SignPath preflight: $($tracked -join '; ')"
}

Write-Host 'PKG-03 03.22 SignPath preparation contract: PASS (fail-closed, no signing credentials, no production submission).'
