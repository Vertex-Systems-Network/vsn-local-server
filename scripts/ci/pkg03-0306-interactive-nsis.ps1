param(
    [Parameter(Mandatory = $true)]
    [string]$SetupPath,

    [Parameter(Mandatory = $true)]
    [string]$SourceSha,

    [string]$EvidenceDir = 'dist-pkg03/03.06'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$ProductName = 'VSN Dev Platform'
$ExpectedVersion = '0.38.1'
$ExpectedPublisher = 'Vertex Systems Network'
$ExpectedRoot = Join-Path $env:LOCALAPPDATA $ProductName
$HkcuKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$HklmKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()

function Assert-Condition {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Get-CanonicalPath {
    param([string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
}

function Get-ProcessTreeIds {
    param([int]$RootPid)

    $snapshot = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    [void]$ids.Add($RootPid)
    do {
        $changed = $false
        foreach ($proc in $snapshot) {
            $processId = [int]$proc.ProcessId
            $parentId = [int]$proc.ParentProcessId
            if ($ids.Contains($parentId) -and -not $ids.Contains($processId)) {
                [void]$ids.Add($processId)
                $changed = $true
            }
        }
    } while ($changed)
    return @($ids)
}

function Get-RelevantWindows {
    param([int]$RootPid, [ValidateSet('install','uninstall')][string]$Phase)

    $ids = @(Get-ProcessTreeIds -RootPid $RootPid)
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    $matches = [System.Collections.Generic.List[object]]::new()
    foreach ($element in $all) {
        try {
            $name = [string]$element.Current.Name
            $processId = [int]$element.Current.ProcessId
            $visible = -not [bool]$element.Current.IsOffscreen
            $handle = [int]$element.Current.NativeWindowHandle
            $titleFallback = $name -match '(?i)VSN Dev Platform.*(Setup|Install|Uninstall)|(Setup|Install|Uninstall).*VSN Dev Platform'
            if ($visible -and $handle -ne 0 -and (($ids -contains $processId) -or $titleFallback)) {
                $matches.Add($element)
            }
        } catch {
            # A window can disappear while UI Automation is enumerating it.
        }
    }
    return @($matches)
}

function Get-ControlElements {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [System.Windows.Automation.ControlType]$ControlType
    )
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        $ControlType
    )
    return @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition))
}

function Get-SafeName {
    param([System.Windows.Automation.AutomationElement]$Element)
    try { return ([string]$Element.Current.Name).Trim() } catch { return '' }
}

function Set-CheckboxOffIfMatched {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [ValidateSet('install','uninstall')][string]$Phase
    )

    foreach ($box in @(Get-ControlElements -Window $Window -ControlType ([System.Windows.Automation.ControlType]::CheckBox))) {
        $name = Get-SafeName -Element $box
        $mustBeOff = $false
        if ($Phase -eq 'install' -and $name -match '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform') {
            $mustBeOff = $true
        }
        if ($Phase -eq 'uninstall' -and $name -match '(?i)delete.*(app.*data|data)|remove.*(app.*data|user.*data)') {
            $mustBeOff = $true
        }
        if (-not $mustBeOff) { continue }

        try {
            $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern(
                [System.Windows.Automation.TogglePattern]::Pattern
            )
            if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) {
                $toggle.Toggle()
                Start-Sleep -Milliseconds 250
            }
            $Actions.Add([ordered]@{
                phase = $Phase
                action = 'ensure-checkbox-off'
                control = $name
                at_utc = [DateTime]::UtcNow.ToString('o')
            })
        } catch {
            throw "Unable to force safety checkbox off during ${Phase}: '$name' :: $($_.Exception.Message)"
        }
    }
}

function Invoke-PrimaryButton {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [ValidateSet('install','uninstall')][string]$Phase
    )

    $buttons = @(Get-ControlElements -Window $Window -ControlType ([System.Windows.Automation.ControlType]::Button))
    $candidates = [System.Collections.Generic.List[object]]::new()
    foreach ($button in $buttons) {
        try {
            if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
            $name = Get-SafeName -Element $button
            if (-not $name) { continue }
            $normalized = ($name -replace '&', '').Trim()
            $candidates.Add([pscustomobject]@{ Element = $button; Name = $name; Normalized = $normalized })
        } catch {
        }
    }

    $priority = if ($Phase -eq 'install') {
        @('^Install$', '^Next\b', '^Finish$', '^Close$')
    } else {
        @('^Uninstall$', '^Next\b', '^Finish$', '^Close$')
    }

    foreach ($pattern in $priority) {
        $selected = $candidates | Where-Object { $_.Normalized -match "(?i)$pattern" } | Select-Object -First 1
        if ($null -eq $selected) { continue }
        try {
            $invoke = [System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern(
                [System.Windows.Automation.InvokePattern]::Pattern
            )
            $invoke.Invoke()
            $Actions.Add([ordered]@{
                phase = $Phase
                action = 'invoke-button'
                control = $selected.Name
                at_utc = [DateTime]::UtcNow.ToString('o')
            })
            return $selected.Normalized
        } catch {
            throw "Failed to invoke $Phase button '$($selected.Name)': $($_.Exception.Message)"
        }
    }
    return $null
}

function Test-InstalledState {
    return (
        (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe')) -and
        (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'uninstall.exe')) -and
        (Test-Path -LiteralPath $HkcuKey)
    )
}

function Test-UninstalledState {
    return (
        -not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe')) -and
        -not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'uninstall.exe')) -and
        -not (Test-Path -LiteralPath $HkcuKey)
    )
}

function Invoke-InteractivePhase {
    param(
        [System.Diagnostics.Process]$RootProcess,
        [ValidateSet('install','uninstall')][string]$Phase,
        [scriptblock]$CompletionTest,
        [int]$TimeoutSeconds = 210
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $visibleObserved = $false
    $quietCompletePolls = 0
    $lastFingerprint = ''

    while ([DateTime]::UtcNow -lt $deadline) {
        $windows = @(Get-RelevantWindows -RootPid $RootProcess.Id -Phase $Phase)
        if ($windows.Count -gt 0) {
            $visibleObserved = $true
            $quietCompletePolls = 0
            foreach ($window in $windows) {
                try { $window.SetFocus() } catch {}
                $title = Get-SafeName -Element $window
                $buttonNames = @(
                    Get-ControlElements -Window $window -ControlType ([System.Windows.Automation.ControlType]::Button) |
                        ForEach-Object { Get-SafeName -Element $_ } |
                        Where-Object { $_ }
                )
                $fingerprint = "$Phase|$($window.Current.ProcessId)|$title|$($buttonNames -join '|')"
                if ($fingerprint -ne $lastFingerprint) {
                    $Observations.Add([ordered]@{
                        phase = $Phase
                        pid = [int]$window.Current.ProcessId
                        title = $title
                        buttons = $buttonNames
                        at_utc = [DateTime]::UtcNow.ToString('o')
                    })
                    $lastFingerprint = $fingerprint
                }

                Set-CheckboxOffIfMatched -Window $window -Phase $Phase
                $clicked = Invoke-PrimaryButton -Window $window -Phase $Phase
                if ($clicked) {
                    Start-Sleep -Milliseconds 900
                    break
                }
            }
        } else {
            $complete = [bool](& $CompletionTest)
            if ($complete) {
                $quietCompletePolls++
                if ($quietCompletePolls -ge 3) { break }
            } else {
                $quietCompletePolls = 0
            }
            Start-Sleep -Milliseconds 500
        }
    }

    Assert-Condition $visibleObserved "No visible NSIS $Phase window was observed; interactive evidence is invalid."
    Assert-Condition ([bool](& $CompletionTest)) "$Phase lifecycle did not reach its required state before timeout."

    $phaseActions = @($Actions | Where-Object { $_.phase -eq $Phase -and $_.action -eq 'invoke-button' })
    Assert-Condition ($phaseActions.Count -ge 1) "No GUI button was invoked during $Phase."
    if ($Phase -eq 'install') {
        $installClicks = @($phaseActions | Where-Object { (($_.control -replace '&', '').Trim()) -match '(?i)^Install$' })
        Assert-Condition ($installClicks.Count -ge 1) 'Interactive install never invoked the Install button.'
    } else {
        $uninstallClicks = @($phaseActions | Where-Object { (($_.control -replace '&', '').Trim()) -match '(?i)^Uninstall$' })
        Assert-Condition ($uninstallClicks.Count -ge 1) 'Interactive uninstall never invoked the Uninstall button.'
    }

    return $visibleObserved
}

$actualHead = (git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"

$SetupPath = (Resolve-Path -LiteralPath $SetupPath).Path
Assert-Condition (Test-Path -LiteralPath $SetupPath -PathType Leaf) "Setup executable missing: $SetupPath"
Assert-Condition ((Get-Item -LiteralPath $SetupPath).Length -gt 0) 'Setup executable is empty.'
Assert-Condition (-not (Test-Path -LiteralPath $ExpectedRoot)) "Expected clean current-user install root already exists: $ExpectedRoot"
Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) "Expected clean HKCU uninstall key already exists: $HkcuKey"
Assert-Condition (-not (Test-Path -LiteralPath $HklmKey)) "Expected clean HKLM uninstall key already exists: $HklmKey"

New-Item -ItemType Directory -Force -Path $EvidencePath | Out-Null
$setupHash = (Get-FileHash -LiteralPath $SetupPath -Algorithm SHA256).Hash.ToLowerInvariant()

# Exact interactive entry point: empty argument vector, no /S, /P, /UPDATE, /R, /ARGS or RunAs.
$setupProcess = Start-Process -FilePath $SetupPath -PassThru
$installerVisible = Invoke-InteractivePhase -RootProcess $setupProcess -Phase install -CompletionTest { Test-InstalledState }

$expectedRootCanonical = Get-CanonicalPath $ExpectedRoot
$programFilesRoots = @($env:ProgramFiles, ${env:ProgramFiles(x86)}) | Where-Object { $_ } | ForEach-Object { Get-CanonicalPath $_ }
foreach ($programRoot in $programFilesRoots) {
    Assert-Condition (-not $expectedRootCanonical.StartsWith($programRoot, [StringComparison]::OrdinalIgnoreCase)) "Current-user install unexpectedly resolved under Program Files: $expectedRootCanonical"
}

Assert-Condition (Test-InstalledState) 'Installed state is incomplete after interactive setup.'
Assert-Condition (-not (Test-Path -LiteralPath $HklmKey)) 'Current-user install created forbidden HKLM package registration.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin/vsn.exe'))) '03.06 illegally packaged bin/vsn.exe before 03.10.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin/vsn-agent.exe'))) '03.06 illegally packaged bin/vsn-agent.exe before 03.10.'

$reg = Get-ItemProperty -LiteralPath $HkcuKey
Assert-Condition ([string]$reg.DisplayName -eq $ProductName) "DisplayName mismatch: $($reg.DisplayName)"
Assert-Condition ([string]$reg.DisplayVersion -eq $ExpectedVersion) "DisplayVersion mismatch: $($reg.DisplayVersion)"
Assert-Condition ([string]$reg.Publisher -eq $ExpectedPublisher) "Publisher mismatch: $($reg.Publisher)"
$registeredLocation = ([string]$reg.InstallLocation).Trim().Trim('"')
Assert-Condition ((Get-CanonicalPath $registeredLocation) -eq $expectedRootCanonical) "InstallLocation mismatch: '$registeredLocation' vs '$ExpectedRoot'"
$uninstallString = [string]$reg.UninstallString
Assert-Condition ($uninstallString -match '(?i)uninstall\.exe') "UninstallString does not target uninstall.exe: $uninstallString"

$installedExe = Join-Path $ExpectedRoot 'VSN Dev Platform.exe'
$uninstaller = Join-Path $ExpectedRoot 'uninstall.exe'
Assert-Condition (Test-Path -LiteralPath $installedExe -PathType Leaf) 'Installed Desktop executable missing.'
Assert-Condition (Test-Path -LiteralPath $uninstaller -PathType Leaf) 'Installed uninstaller missing.'

# Finish-page safety contract: no VSN app process should have been launched by the installer.
$escapedApp = @(Get-Process -ErrorAction SilentlyContinue | Where-Object {
    try { $_.Path -and (Get-CanonicalPath $_.Path) -eq (Get-CanonicalPath $installedExe) } catch { $false }
})
Assert-Condition ($escapedApp.Count -eq 0) 'Installer finish page launched the application; harness failed to keep the Run checkbox off.'

# Exact interactive uninstall entry point: empty argument vector, no /S, /P, /UPDATE or RunAs.
$uninstallProcess = Start-Process -FilePath $uninstaller -PassThru
$uninstallerVisible = Invoke-InteractivePhase -RootProcess $uninstallProcess -Phase uninstall -CompletionTest { Test-UninstalledState }

Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) 'HKCU uninstall registration remained after clean interactive uninstall.'
Assert-Condition (-not (Test-Path -LiteralPath $HklmKey)) 'HKLM uninstall registration appeared during current-user lifecycle.'
Assert-Condition (-not (Test-Path -LiteralPath $installedExe)) 'Desktop executable remained after clean interactive uninstall.'
Assert-Condition (-not (Test-Path -LiteralPath $uninstaller)) 'uninstall.exe remained after clean interactive uninstall.'

$tracked = @(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) {
    $tracked | Write-Host
    throw 'Tracked repository drift detected during 03.06 interactive lifecycle.'
}

$observationsPath = Join-Path $EvidencePath 'ui-observations.json'
ConvertTo-Json -InputObject @($Observations) -Depth 8 -Compress:$false | Set-Content -LiteralPath $observationsPath -Encoding utf8NoBOM

$evidence = [ordered]@{
    schema_version = 1
    package_id = 'PKG-03'
    task_id = '03.06'
    source_commit = $SourceSha
    setup = [ordered]@{
        filename = [System.IO.Path]::GetFileName($SetupPath)
        size_bytes = (Get-Item -LiteralPath $SetupPath).Length
        sha256 = $setupHash
        arguments = @()
        elevation_verb = $null
    }
    current_user_scope = [ordered]@{
        expected_install_root_token = '%LOCALAPPDATA%\VSN Dev Platform'
        actual_install_root = $expectedRootCanonical
        hkcu_registration_observed = $true
        hklm_registration_absent = $true
        display_name = $ProductName
        display_version = $ExpectedVersion
        publisher = $ExpectedPublisher
        uninstall_string_targeted_uninstall_exe = $true
    }
    installed_payload = [ordered]@{
        desktop_executable_observed = $true
        uninstaller_observed = $true
        cli_absent_until_03_10 = $true
        agent_absent_until_03_10 = $true
    }
    interaction = [ordered]@{
        visible_installer_window_observed = [bool]$installerVisible
        visible_uninstaller_window_observed = [bool]$uninstallerVisible
        passive_switch_used = $false
        silent_switch_used = $false
        update_switch_used = $false
        explicit_elevation_used = $false
        actions = @($Actions)
        observations_file = 'ui-observations.json'
    }
    clean_uninstall = [ordered]@{
        hkcu_registration_removed = $true
        hklm_registration_absent = $true
        desktop_executable_removed = $true
        uninstaller_removed = $true
        destructive_app_data_option_selected = $false
    }
    scope_nonclaims = [ordered]@{
        per_machine_certified = $false
        msi_certified = $false
        cli_agent_placement_certified = $false
        service_lifecycle_certified = $false
        acl_lifecycle_certified = $false
        silent_deployment_certified = $false
        signing_performed = $false
        updater_mutation = $false
    }
    tracked_repository_drift_zero = $true
}

$evidenceFile = Join-Path $EvidencePath 'evidence.json'
$evidence | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $evidenceFile -Encoding utf8NoBOM
$evidenceHash = (Get-FileHash -LiteralPath $evidenceFile -Algorithm SHA256).Hash.ToLowerInvariant()
"$evidenceHash  evidence.json" | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM
"$setupHash  $([System.IO.Path]::GetFileName($SetupPath))" | Set-Content -LiteralPath (Join-Path $EvidencePath 'setup.sha256') -Encoding utf8NoBOM

$evidence | ConvertTo-Json -Depth 10
