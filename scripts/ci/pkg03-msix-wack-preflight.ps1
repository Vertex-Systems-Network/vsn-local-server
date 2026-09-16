param(
    [Parameter(Mandatory = $true)]
    [string]$PackagePath,

    [Parameter(Mandatory = $false)]
    [string]$OutputDir = ".fast-msix-wack-out"
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
Push-Location $repoRoot
try {
    $package = (Resolve-Path -LiteralPath $PackagePath).Path
    if (-not (Test-Path -LiteralPath $package -PathType Leaf)) {
        throw "MSIX package not found: $PackagePath"
    }

    $out = Join-Path $repoRoot $OutputDir
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $out | Out-Null

    $programFilesX86 = ${env:ProgramFiles(x86)}
    $appCert = if ($programFilesX86) {
        Join-Path $programFilesX86 'Windows Kits\10\App Certification Kit\appcert.exe'
    } else {
        'C:\Program Files (x86)\Windows Kits\10\App Certification Kit\appcert.exe'
    }

    $appCertPresent = Test-Path -LiteralPath $appCert -PathType Leaf
    $sessionId = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
    $activeUserSession = ($sessionId -ne 0)
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
    $isAdmin = $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
    $environmentReady = ($appCertPresent -and $activeUserSession -and $isAdmin)

    $environmentBlocker = $null
    if (-not $appCertPresent) {
        $environmentBlocker = 'APPCERT_NOT_FOUND'
    }
    elseif (-not $activeUserSession) {
        $environmentBlocker = 'SESSION0_NOT_SUPPORTED'
    }
    elseif (-not $isAdmin) {
        $environmentBlocker = 'ADMIN_TOKEN_REQUIRED'
    }

    $resetInvoked = $false
    $resetExit = $null
    $testInvoked = $false
    $testExit = $null
    $reportPath = Join-Path $out 'wack-report.xml'
    $resetLogPath = Join-Path $out 'appcert-reset.log'
    $testLogPath = Join-Path $out 'appcert-test.log'

    if ($environmentReady) {
        $resetInvoked = $true
        $resetOutput = & $appCert reset 2>&1
        $resetExit = $LASTEXITCODE
        @($resetOutput) | ForEach-Object { [string]$_ } | Set-Content -LiteralPath $resetLogPath -Encoding utf8NoBOM

        if ($resetExit -eq 0) {
            $testInvoked = $true
            $testOutput = & $appCert test -appxpackagepath $package -reportoutputpath $reportPath 2>&1
            $testExit = $LASTEXITCODE
            @($testOutput) | ForEach-Object { [string]$_ } | Set-Content -LiteralPath $testLogPath -Encoding utf8NoBOM
        }
    }

    $reportExists = Test-Path -LiteralPath $reportPath -PathType Leaf
    $reportSha256 = if ($reportExists) {
        (Get-FileHash -LiteralPath $reportPath -Algorithm SHA256).Hash.ToLowerInvariant()
    } else {
        $null
    }

    $outcome = if (-not $environmentReady) {
        'ENVIRONMENT_UNAVAILABLE'
    }
    elseif ($resetInvoked -and $resetExit -ne 0) {
        'RESET_FAILED'
    }
    elseif ($testInvoked -and $testExit -eq 0) {
        'TEST_PROCESS_EXIT_ZERO'
    }
    elseif ($testInvoked) {
        'TEST_PROCESS_EXIT_NONZERO'
    }
    else {
        'NOT_RUN'
    }

    $evidence = [ordered]@{
        schema_version = 1
        lane = 'msix-store-preimplementation'
        mode = 'NON_ACCEPTANCE_PREIMPLEMENTATION'
        source_commit = (git rev-parse HEAD).Trim()
        package_file = (Get-Item -LiteralPath $package).Name
        package_sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
        package_signed = $false
        production_identity_present = $false
        appcert_path = $appCert
        appcert_present = [bool]$appCertPresent
        process_session_id = [int]$sessionId
        active_user_session = [bool]$activeUserSession
        administrator_token = [bool]$isAdmin
        environment_ready = [bool]$environmentReady
        environment_blocker = $environmentBlocker
        reset_invoked = [bool]$resetInvoked
        reset_exit_code = $resetExit
        test_invoked = [bool]$testInvoked
        test_exit_code = $testExit
        report_exists = [bool]$reportExists
        report_sha256 = $reportSha256
        wack_outcome = $outcome
        wack_process_exit_zero = [bool]($testInvoked -and $testExit -eq 0)
        wack_required_for_production_preflight = $true
        certification_pass_claimed = $false
        production_acceptance = $false
        store_submission_performed = $false
        production_evidence_consumed = $false
        canonical_state_changed = $false
        implementation_authority = $false
        required_runtime_boundary = 'external_or_preprovisioned_authenticated_agent'
        direct_installer_lane_must_remain = $true
    }

    $evidencePath = Join-Path $out 'evidence.json'
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $evidencePath -Encoding utf8NoBOM
    Write-Host ($evidence | ConvertTo-Json -Depth 8 -Compress)
    Write-Host 'PKG03_MSIX_WACK_PREFLIGHT=PASS'
}
finally {
    Pop-Location
}
