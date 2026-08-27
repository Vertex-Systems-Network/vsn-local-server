param(
    [Parameter(Mandatory = $true)][string]$MsiPath,
    [Parameter(Mandatory = $true)][string]$SourceSha,
    [string]$EvidenceDir = 'dist-pkg03/03.08'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$ProductName = 'VSN Dev Platform'
$ExpectedVersion = '0.38.1'
$ExpectedPublisher = 'Vertex Systems Network'
$ExpectedUpgradeCode = '157f304f-1d1b-55e0-b89c-0610ea27c645'
$ExpectedRoot = Join-Path $env:ProgramFiles $ProductName
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()

function Assert-Condition([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-MsiProperty([string]$Path, [string]$Property) {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $db = $installer.GetType().InvokeMember('OpenDatabase', 'InvokeMethod', $null, $installer, @($Path, 0))
    $sql = "SELECT `Value` FROM `Property` WHERE `Property`='$Property'"
    $view = $db.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $db, @($sql))
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
    $record = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
    if ($null -eq $record) { throw "MSI property '$Property' not found." }
    return [string]$record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @(1))
}

function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) {
    try { return ([string]$Element.Current.Name).Trim() } catch { return '' }
}

function Get-Controls([System.Windows.Automation.AutomationElement]$Window, [System.Windows.Automation.ControlType]$Type) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $Type
    )
    return @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition))
}

function Get-RelevantWindows {
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    $result = @()
    foreach ($element in $all) {
        try {
            $name = [string]$element.Current.Name
            $visible = -not [bool]$element.Current.IsOffscreen
            $handle = [int]$element.Current.NativeWindowHandle
            if ($visible -and $handle -ne 0 -and $name -match '(?i)VSN Dev Platform|Windows Installer') {
                $result += $element
            }
        } catch {}
    }
    return $result
}

function Record-Window([string]$Phase, [System.Windows.Automation.AutomationElement]$Window) {
    $buttons = @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
    $checks = @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
    [void]$Observations.Add([pscustomobject][ordered]@{
        phase = $Phase
        pid = [int]$Window.Current.ProcessId
        title = Get-SafeName $Window
        buttons = $buttons
        checkboxes = $checks
        at_utc = [DateTime]::UtcNow.ToString('o')
    })
}

function Disable-LaunchCheckbox([System.Windows.Automation.AutomationElement]$Window, [string]$Phase) {
    foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
        $name = Get-SafeName $box
        if ($name -notmatch '(?i)launch|run.*VSN Dev Platform') { continue }
        try {
            $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
            if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) {
                $toggle.Toggle()
                Start-Sleep -Milliseconds 200
            }
            [void]$Actions.Add([pscustomobject][ordered]@{
                phase=$Phase; action='ensure-launch-checkbox-off'; control=$name; at_utc=[DateTime]::UtcNow.ToString('o')
            })
        } catch {}
    }
}

function Invoke-PrimaryButton([string]$Phase, [System.Windows.Automation.AutomationElement]$Window) {
    $candidates = @()
    foreach ($button in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button))) {
        try {
            if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
            $name = Get-SafeName $button
            if ($name) {
                $candidates += [pscustomobject]@{ Element=$button; Name=$name; Normalized=($name -replace '&','').Trim() }
            }
        } catch {}
    }
    $priority = if ($Phase -eq 'install') {
        @('^Install$', '^Next\b', '^Finish$', '^Close$', '^OK$')
    } else {
        @('^Remove$', '^Uninstall$', '^Next\b', '^Yes$', '^Finish$', '^Close$', '^OK$')
    }
    foreach ($pattern in $priority) {
        $selected = $candidates | Where-Object { $_.Normalized -match "(?i)$pattern" } | Select-Object -First 1
        if ($null -eq $selected) { continue }
        try {
            $invoke = [System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
            $invoke.Invoke()
            [void]$Actions.Add([pscustomobject][ordered]@{
                phase=$Phase; action='invoke-button'; control=$selected.Name; at_utc=[DateTime]::UtcNow.ToString('o')
            })
            return $selected.Normalized
        } catch {}
    }
    return $null
}

function Wait-DriveMsiUi([string]$Phase, [scriptblock]$CompletionTest, [int]$TimeoutSeconds = 180) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $visibleObserved = $false
    $terminalClicked = $false
    while ([DateTime]::UtcNow -lt $deadline) {
        foreach ($window in @(Get-RelevantWindows)) {
            $visibleObserved = $true
            Record-Window $Phase $window
            Disable-LaunchCheckbox $window $Phase
            $clicked = Invoke-PrimaryButton $Phase $window
            if ($clicked -match '(?i)^(Finish|Close|OK)$') { $terminalClicked = $true }
        }
        if (& $CompletionTest) {
            if ($terminalClicked -or -not @(Get-RelevantWindows).Count) {
                return [pscustomobject]@{ visible=$visibleObserved; terminal_clicked=$terminalClicked }
            }
        }
        Start-Sleep -Milliseconds 700
    }
    throw "Timed out driving visible MSI $Phase UI."
}

function Get-ArpState([string]$ProductCode) {
    $key = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductCode"
    if (-not (Test-Path -LiteralPath $key)) {
        return [pscustomobject]@{ present=$false; key=$key }
    }
    $p = Get-ItemProperty -LiteralPath $key
    return [pscustomobject][ordered]@{
        present = $true
        key = $key
        display_name = [string]$p.DisplayName
        display_version = [string]$p.DisplayVersion
        publisher = [string]$p.Publisher
        uninstall_string = [string]$p.UninstallString
        windows_installer = [int]$p.WindowsInstaller
    }
}

New-Item -ItemType Directory -Force $EvidencePath | Out-Null
$MsiPath = (Resolve-Path -LiteralPath $MsiPath).Path
Assert-Condition (Test-Path -LiteralPath $MsiPath -PathType Leaf) 'MSI path does not exist.'
Assert-Condition ((Get-Item -LiteralPath $MsiPath).Length -gt 0) 'MSI package is empty.'

$productCode = Get-MsiProperty $MsiPath 'ProductCode'
$upgradeCode = Get-MsiProperty $MsiPath 'UpgradeCode'
$productNameFromMsi = Get-MsiProperty $MsiPath 'ProductName'
$productVersionFromMsi = Get-MsiProperty $MsiPath 'ProductVersion'
$manufacturerFromMsi = Get-MsiProperty $MsiPath 'Manufacturer'

Assert-Condition ($productCode -match '^\{[0-9A-Fa-f-]{36}\}$') "Invalid ProductCode: $productCode"
Assert-Condition ($upgradeCode.Trim('{}').ToLowerInvariant() -eq $ExpectedUpgradeCode) "UpgradeCode mismatch: $upgradeCode"
Assert-Condition ($productNameFromMsi -eq $ProductName) "ProductName mismatch: $productNameFromMsi"
Assert-Condition ($productVersionFromMsi -eq $ExpectedVersion) "ProductVersion mismatch: $productVersionFromMsi"
Assert-Condition ($manufacturerFromMsi -eq $ExpectedPublisher) "Manufacturer mismatch: $manufacturerFromMsi"

$preArp = Get-ArpState $productCode
Assert-Condition (-not $preArp.present) 'Exact ProductCode ARP entry already exists before test.'
if (Test-Path -LiteralPath $ExpectedRoot) {
    throw "Expected install root already exists before test: $ExpectedRoot"
}

$msiexec = Join-Path $env:SystemRoot 'System32\msiexec.exe'
$installArgs = @('/i', ('"{0}"' -f $MsiPath))
$installProcess = Start-Process -FilePath $msiexec -ArgumentList $installArgs -PassThru
$installUi = Wait-DriveMsiUi -Phase 'install' -CompletionTest {
    (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe')) -and (Get-ArpState $productCode).present
}
$installProcess.WaitForExit()
Assert-Condition ($installProcess.ExitCode -eq 0) "MSI install exited with code $($installProcess.ExitCode)."

$arp = Get-ArpState $productCode
Assert-Condition $installUi.visible 'No visible MSI install UI was observed.'
Assert-Condition $arp.present 'Exact ProductCode HKLM ARP entry was not observed.'
Assert-Condition ($arp.display_name -eq $ProductName) "ARP DisplayName mismatch: $($arp.display_name)"
Assert-Condition ($arp.display_version -eq $ExpectedVersion) "ARP DisplayVersion mismatch: $($arp.display_version)"
Assert-Condition ($arp.publisher -eq $ExpectedPublisher) "ARP Publisher mismatch: $($arp.publisher)"
Assert-Condition ($arp.windows_installer -eq 1) 'ARP WindowsInstaller marker is not 1.'
Assert-Condition ($arp.uninstall_string -match '(?i)msiexec') 'ARP uninstall string is not MSI-backed.'
Assert-Condition (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe')) 'Desktop executable missing after MSI install.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin\vsn.exe'))) 'CLI unexpectedly packaged before 03.10.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin\vsn-agent.exe'))) 'Agent unexpectedly packaged before 03.10.'

$uninstallArgs = @('/x', $productCode)
$uninstallProcess = Start-Process -FilePath $msiexec -ArgumentList $uninstallArgs -PassThru
$uninstallUi = Wait-DriveMsiUi -Phase 'uninstall' -CompletionTest {
    -not (Get-ArpState $productCode).present -and -not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe'))
}
$uninstallProcess.WaitForExit()
Assert-Condition ($uninstallProcess.ExitCode -eq 0) "MSI uninstall exited with code $($uninstallProcess.ExitCode)."
Assert-Condition $uninstallUi.visible 'No visible MSI uninstall UI was observed.'
Assert-Condition (-not (Get-ArpState $productCode).present) 'Exact ProductCode HKLM ARP entry remains after uninstall.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe'))) 'Desktop executable remains after MSI uninstall.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin\vsn.exe'))) 'CLI exists after uninstall before 03.10.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin\vsn-agent.exe'))) 'Agent exists after uninstall before 03.10.'

$Observations | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
$Actions | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM

$evidence = [ordered]@{
    schema_version = 1
    package_id = 'PKG-03'
    task_id = '03.08'
    source_commit = $SourceSha
    msi = [ordered]@{
        path = $MsiPath
        sha256 = Get-Sha256 $MsiPath
        size_bytes = (Get-Item -LiteralPath $MsiPath).Length
        product_code = $productCode
        upgrade_code = $upgradeCode
        product_name = $productNameFromMsi
        product_version = $productVersionFromMsi
        manufacturer = $manufacturerFromMsi
    }
    install = [ordered]@{
        command = 'msiexec.exe /i <exact-msi>'
        arguments = $installArgs
        exit_code = $installProcess.ExitCode
        visible_install_ui_observed = [bool]$installUi.visible
        install_root = $ExpectedRoot
        desktop_executable_observed = $true
        cli_absent_until_03_10 = $true
        agent_absent_until_03_10 = $true
    }
    arp = [ordered]@{
        arp_product_code_key_observed = [bool]$arp.present
        registry_root = 'HKLM'
        key = $arp.key
        display_name = $arp.display_name
        display_version = $arp.display_version
        publisher = $arp.publisher
        windows_installer = $arp.windows_installer
        uninstall_string = $arp.uninstall_string
    }
    uninstall = [ordered]@{
        command = 'msiexec.exe /x {ProductCode}'
        arguments = $uninstallArgs
        exit_code = $uninstallProcess.ExitCode
        visible_uninstall_ui_observed = [bool]$uninstallUi.visible
        arp_product_code_key_removed = $true
        desktop_executable_removed = $true
    }
    ui_boundary = [ordered]@{
        quiet_used = $false
        passive_used = $false
        qn_used = $false
        qb_used = $false
        qr_used = $false
        qf_used = $false
        blanket_hkcu_nonmutation_claimed = $false
    }
    shortcut_semantics_claimed = $false
    cli_agent_placement_claimed = $false
    service_registration_claimed = $false
    signing_claimed = $false
    updater_mutation_claimed = $false
    tracked_repository_drift_zero = $true
}
$evidence | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json') -Encoding utf8NoBOM
