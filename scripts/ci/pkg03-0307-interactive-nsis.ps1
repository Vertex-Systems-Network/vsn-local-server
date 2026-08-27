param(
    [Parameter(Mandatory = $true)]
    [string]$SetupPath,

    [Parameter(Mandatory = $true)]
    [string]$SourceSha,

    [string]$EvidenceDir = 'dist-pkg03/03.07'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;

public static class VsnNativeUi {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr GetAncestor(IntPtr hWnd, uint gaFlags);

    [DllImport("user32.dll", SetLastError = true)]
    public static extern int GetDlgCtrlID(IntPtr hWnd);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindow(IntPtr hWnd);
}

public static class VsnToken {
    private const uint PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
    private const uint TOKEN_QUERY = 0x0008;
    private const int TokenElevation = 20;
    private const int TokenIntegrityLevel = 25;

    [StructLayout(LayoutKind.Sequential)]
    private struct TOKEN_ELEVATION {
        public int TokenIsElevated;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct SID_AND_ATTRIBUTES {
        public IntPtr Sid;
        public uint Attributes;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct TOKEN_MANDATORY_LABEL {
        public SID_AND_ATTRIBUTES Label;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr OpenProcess(uint access, bool inheritHandle, int processId);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(IntPtr handle);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool OpenProcessToken(IntPtr processHandle, uint desiredAccess, out IntPtr tokenHandle);

    [DllImport("advapi32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool GetTokenInformation(IntPtr tokenHandle, int tokenInfoClass, IntPtr tokenInfo, int tokenInfoLength, out int returnLength);

    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern IntPtr GetSidSubAuthorityCount(IntPtr sid);

    [DllImport("advapi32.dll", SetLastError = true)]
    private static extern IntPtr GetSidSubAuthority(IntPtr sid, uint subAuthority);

    private static IntPtr OpenToken(int pid, out IntPtr processHandle) {
        processHandle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if (processHandle == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error(), "OpenProcess failed");
        IntPtr tokenHandle;
        if (!OpenProcessToken(processHandle, TOKEN_QUERY, out tokenHandle)) {
            int error = Marshal.GetLastWin32Error();
            CloseHandle(processHandle);
            processHandle = IntPtr.Zero;
            throw new Win32Exception(error, "OpenProcessToken failed");
        }
        return tokenHandle;
    }

    public static bool IsElevated(int pid) {
        IntPtr process = IntPtr.Zero;
        IntPtr token = IntPtr.Zero;
        IntPtr buffer = IntPtr.Zero;
        try {
            token = OpenToken(pid, out process);
            int length = Marshal.SizeOf(typeof(TOKEN_ELEVATION));
            buffer = Marshal.AllocHGlobal(length);
            int returned;
            if (!GetTokenInformation(token, TokenElevation, buffer, length, out returned))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "GetTokenInformation(TokenElevation) failed");
            TOKEN_ELEVATION elevation = (TOKEN_ELEVATION)Marshal.PtrToStructure(buffer, typeof(TOKEN_ELEVATION));
            return elevation.TokenIsElevated != 0;
        } finally {
            if (buffer != IntPtr.Zero) Marshal.FreeHGlobal(buffer);
            if (token != IntPtr.Zero) CloseHandle(token);
            if (process != IntPtr.Zero) CloseHandle(process);
        }
    }

    public static int IntegrityRid(int pid) {
        IntPtr process = IntPtr.Zero;
        IntPtr token = IntPtr.Zero;
        IntPtr buffer = IntPtr.Zero;
        try {
            token = OpenToken(pid, out process);
            int needed;
            GetTokenInformation(token, TokenIntegrityLevel, IntPtr.Zero, 0, out needed);
            if (needed <= 0) throw new Win32Exception(Marshal.GetLastWin32Error(), "Unable to size TokenIntegrityLevel");
            buffer = Marshal.AllocHGlobal(needed);
            if (!GetTokenInformation(token, TokenIntegrityLevel, buffer, needed, out needed))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "GetTokenInformation(TokenIntegrityLevel) failed");
            TOKEN_MANDATORY_LABEL label = (TOKEN_MANDATORY_LABEL)Marshal.PtrToStructure(buffer, typeof(TOKEN_MANDATORY_LABEL));
            IntPtr countPtr = GetSidSubAuthorityCount(label.Label.Sid);
            if (countPtr == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error(), "GetSidSubAuthorityCount failed");
            byte count = Marshal.ReadByte(countPtr);
            if (count == 0) throw new InvalidOperationException("Integrity SID has no sub-authorities");
            IntPtr ridPtr = GetSidSubAuthority(label.Label.Sid, (uint)(count - 1));
            if (ridPtr == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error(), "GetSidSubAuthority failed");
            return Marshal.ReadInt32(ridPtr);
        } finally {
            if (buffer != IntPtr.Zero) Marshal.FreeHGlobal(buffer);
            if (token != IntPtr.Zero) CloseHandle(token);
            if (process != IntPtr.Zero) CloseHandle(process);
        }
    }
}
'@

$ProductName = 'VSN Dev Platform'
$ExpectedVersion = '0.38.1'
$ExpectedPublisher = 'Vertex Systems Network'
$ExpectedRoot = Join-Path $env:ProgramFiles $ProductName
$ForbiddenUserRoot = Join-Path $env:LOCALAPPDATA $ProductName
$HkcuKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$HklmKey = "HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$ProductName"
$EvidencePath = Join-Path (Get-Location) $EvidenceDir
$Observations = [System.Collections.Generic.List[object]]::new()
$Actions = [System.Collections.Generic.List[object]]::new()
$TerminalFallbackRoots = [System.Collections.Generic.HashSet[string]]::new()

function Assert-Condition {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Get-CanonicalPath {
    param([string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
}

function Get-ProcessPrivilegeSnapshot {
    param([int]$ProcessId)

    $lastError = $null
    for ($attempt = 1; $attempt -le 10; $attempt++) {
        try {
            $elevated = [VsnToken]::IsElevated($ProcessId)
            $integrityRid = [VsnToken]::IntegrityRid($ProcessId)
            return [pscustomobject][ordered]@{
                pid = $ProcessId
                elevated = [bool]$elevated
                integrity_rid = [int]$integrityRid
                high_integrity = [bool]($integrityRid -ge 0x3000)
            }
        } catch {
            $lastError = $_.Exception.Message
            Start-Sleep -Milliseconds 100
        }
    }
    throw "Unable to read privilege token for process ${ProcessId}: $lastError"
}

function Get-RunnerPrivilegeSnapshot {
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [System.Security.Principal.WindowsPrincipal]::new($identity)
    $isAdmin = $principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
    $token = Get-ProcessPrivilegeSnapshot -ProcessId $PID
    $enableLua = $null
    try {
        $enableLua = [int](Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name EnableLUA).EnableLUA
    } catch {
        throw "Unable to read hosted-runner UAC policy: $($_.Exception.Message)"
    }
    return [pscustomobject][ordered]@{
        identity = $identity.Name
        administrator = [bool]$isAdmin
        elevated = [bool]$token.elevated
        integrity_rid = [int]$token.integrity_rid
        high_integrity = [bool]$token.high_integrity
        enable_lua = $enableLua
        uac_disabled = [bool]($enableLua -eq 0)
    }
}

function Get-SafeName {
    param([System.Windows.Automation.AutomationElement]$Element)
    try { return ([string]$Element.Current.Name).Trim() } catch { return '' }
}

function Add-RelevantWindows {
    param(
        [int]$RootPid,
        [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]$Result
    )

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

    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll(
        [System.Windows.Automation.TreeScope]::Children,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    foreach ($element in $all) {
        if ($element -isnot [System.Windows.Automation.AutomationElement]) { continue }
        try {
            $name = [string]$element.Current.Name
            $processId = [int]$element.Current.ProcessId
            $visible = -not [bool]$element.Current.IsOffscreen
            $handle = [int]$element.Current.NativeWindowHandle
            $titleFallback = $name -match '(?i)VSN Dev Platform.*(Setup|Install|Uninstall)|(Setup|Install|Uninstall).*VSN Dev Platform'
            if ($visible -and $handle -ne 0 -and ($ids.Contains($processId) -or $titleFallback)) {
                [void]$Result.Add($element)
            }
        } catch {}
    }
}

function Add-ControlElements {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [System.Windows.Automation.ControlType]$ControlType,
        [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]$Result
    )

    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        $ControlType
    )
    $found = $Window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($element in $found) {
        if ($element -is [System.Windows.Automation.AutomationElement]) {
            [void]$Result.Add($element)
        }
    }
}

function Add-ControlSnapshot {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [System.Collections.Generic.List[object]]$Result
    )

    $found = $Window.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition
    )
    foreach ($element in $found) {
        if ($element -isnot [System.Windows.Automation.AutomationElement]) { continue }
        try {
            $patterns = @()
            try { $patterns = @($element.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }) } catch {}
            [void]$Result.Add([pscustomobject][ordered]@{
                control_type = [string]$element.Current.ControlType.ProgrammaticName
                name = ([string]$element.Current.Name).Trim()
                automation_id = [string]$element.Current.AutomationId
                class_name = [string]$element.Current.ClassName
                framework_id = [string]$element.Current.FrameworkId
                enabled = [bool]$element.Current.IsEnabled
                offscreen = [bool]$element.Current.IsOffscreen
                native_window_handle = [int]$element.Current.NativeWindowHandle
                patterns = $patterns
            })
        } catch {}
    }
}

function Set-CheckboxOffIfMatched {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [ValidateSet('install','uninstall')][string]$Phase
    )

    $boxes = [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]::new()
    Add-ControlElements -Window $Window -ControlType ([System.Windows.Automation.ControlType]::CheckBox) -Result $boxes
    foreach ($box in $boxes) {
        $name = Get-SafeName -Element $box
        $mustBeOff = (
            ($Phase -eq 'install' -and $name -match '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform') -or
            ($Phase -eq 'uninstall' -and $name -match '(?i)delete.*(app.*data|data)|remove.*(app.*data|user.*data)')
        )
        if (-not $mustBeOff) { continue }

        $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern(
            [System.Windows.Automation.TogglePattern]::Pattern
        )
        if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) {
            $toggle.Toggle()
            Start-Sleep -Milliseconds 250
        }
        [void]$Actions.Add([pscustomobject][ordered]@{
            phase = $Phase
            action = 'ensure-checkbox-off'
            control = $name
            at_utc = [DateTime]::UtcNow.ToString('o')
        })
    }
}

function Invoke-TerminalFallback {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [System.Windows.Automation.AutomationElement]$Button,
        [string]$ButtonName,
        [ValidateSet('install','uninstall')][string]$Phase,
        [bool]$CompletionReached
    )

    if (-not $CompletionReached) { return }
    try { $buttonHandle = [IntPtr][int]$Button.Current.NativeWindowHandle } catch { return }
    if ($buttonHandle -eq [IntPtr]::Zero -or -not [VsnNativeUi]::IsWindow($buttonHandle)) { return }

    $GA_ROOT = [uint32]2
    $rootHandle = [VsnNativeUi]::GetAncestor($buttonHandle, $GA_ROOT)
    if ($rootHandle -eq [IntPtr]::Zero) {
        try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return }
    }
    if ($rootHandle -eq [IntPtr]::Zero -or -not [VsnNativeUi]::IsWindow($rootHandle)) { return }

    $rootKey = "${Phase}:$($rootHandle.ToInt64())"
    if (-not $TerminalFallbackRoots.Add($rootKey)) { return }

    $controlId = [VsnNativeUi]::GetDlgCtrlID($buttonHandle)
    if ($controlId -gt 0) {
        $WM_COMMAND = [uint32]0x0111
        [void][VsnNativeUi]::SendMessage($rootHandle, $WM_COMMAND, [IntPtr]$controlId, $buttonHandle)
        [void]$Actions.Add([pscustomobject][ordered]@{
            phase = $Phase
            action = 'native-wm-command-fallback'
            control = $ButtonName
            at_utc = [DateTime]::UtcNow.ToString('o')
        })
        Start-Sleep -Milliseconds 500
    }

    if ([VsnNativeUi]::IsWindow($rootHandle)) {
        $WM_CLOSE = [uint32]0x0010
        [void][VsnNativeUi]::PostMessage($rootHandle, $WM_CLOSE, [IntPtr]::Zero, [IntPtr]::Zero)
        [void]$Actions.Add([pscustomobject][ordered]@{
            phase = $Phase
            action = 'native-wm-close-terminal-fallback'
            control = $ButtonName
            at_utc = [DateTime]::UtcNow.ToString('o')
        })
        Start-Sleep -Milliseconds 500
    }
}

function Invoke-PrimaryButton {
    param(
        [System.Windows.Automation.AutomationElement]$Window,
        [ValidateSet('install','uninstall')][string]$Phase,
        [bool]$CompletionReached
    )

    $buttons = [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]::new()
    Add-ControlElements -Window $Window -ControlType ([System.Windows.Automation.ControlType]::Button) -Result $buttons
    $candidates = @(
        foreach ($button in $buttons) {
            try {
                if (-not [bool]$button.Current.IsEnabled -or [bool]$button.Current.IsOffscreen) { continue }
                $name = Get-SafeName -Element $button
                if (-not $name) { continue }
                [pscustomobject]@{ Element = $button; Name = $name; Normalized = ($name -replace '&', '').Trim() }
            } catch {}
        }
    )

    $priority = if ($Phase -eq 'install') {
        @('^Install$', '^Next\b', '^Finish$', '^Close$')
    } else {
        @('^Uninstall$', '^Next\b', '^Finish$', '^Close$')
    }

    foreach ($pattern in $priority) {
        $selected = $candidates | Where-Object { $_.Normalized -match "(?i)$pattern" } | Select-Object -First 1
        if ($null -eq $selected) { continue }

        $invoke = [System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern(
            [System.Windows.Automation.InvokePattern]::Pattern
        )
        $invoke.Invoke()
        [void]$Actions.Add([pscustomobject][ordered]@{
            phase = $Phase
            action = 'invoke-button'
            control = $selected.Name
            at_utc = [DateTime]::UtcNow.ToString('o')
        })

        if ($selected.Normalized -match '(?i)^(Finish|Close)$') {
            Start-Sleep -Milliseconds 350
            Invoke-TerminalFallback -Window $Window -Button $selected.Element -ButtonName $selected.Name -Phase $Phase -CompletionReached $CompletionReached
        }
        return $selected.Normalized
    }
    return $null
}

function Test-InstalledState {
    return (
        (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe')) -and
        (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'uninstall.exe')) -and
        (Test-Path -LiteralPath $HklmKey)
    )
}

function Test-UninstalledState {
    return (
        -not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe')) -and
        -not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'uninstall.exe')) -and
        -not (Test-Path -LiteralPath $HklmKey) -and
        -not (Test-Path -LiteralPath $ForbiddenUserRoot)
    )
}

function Write-PhaseDiagnostics {
    param(
        [ValidateSet('install','uninstall')][string]$Phase,
        [System.Diagnostics.Process]$RootProcess,
        [object]$Privilege,
        [bool]$VisibleObserved,
        [bool]$CompletionReached,
        [bool]$TerminalClosed
    )

    New-Item -ItemType Directory -Force -Path $EvidencePath | Out-Null
    $rootItems = @()
    if (Test-Path -LiteralPath $ExpectedRoot) {
        $rootItems = @(Get-ChildItem -LiteralPath $ExpectedRoot -Force -ErrorAction SilentlyContinue |
            Select-Object Name, FullName, Length, PSIsContainer)
    }
    $processAlive = $false
    try { $processAlive = -not $RootProcess.HasExited } catch {}

    $diagnostics = [ordered]@{
        schema_version = 1
        package_id = 'PKG-03'
        task_id = '03.07'
        source_commit = $SourceSha
        phase = $Phase
        visible_window_observed = $VisibleObserved
        completion_reached = $CompletionReached
        terminal_window_closed = $TerminalClosed
        root_process_id = $RootProcess.Id
        root_process_alive = $processAlive
        root_process_privilege = $Privilege
        state = [ordered]@{
            expected_root = $ExpectedRoot
            expected_root_exists = (Test-Path -LiteralPath $ExpectedRoot)
            forbidden_user_root = $ForbiddenUserRoot
            forbidden_user_root_exists = (Test-Path -LiteralPath $ForbiddenUserRoot)
            desktop_executable_exists = (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'VSN Dev Platform.exe'))
            uninstaller_exists = (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'uninstall.exe'))
            hkcu_uninstall_key_exists = (Test-Path -LiteralPath $HkcuKey)
            hklm_uninstall_key_exists = (Test-Path -LiteralPath $HklmKey)
            root_items = $rootItems
        }
        actions = @($Actions | Where-Object { $_.phase -eq $Phase })
        observations = @($Observations | Where-Object { $_.phase -eq $Phase })
        captured_at_utc = [DateTime]::UtcNow.ToString('o')
    }

    $json = $diagnostics | ConvertTo-Json -Depth 14
    $json | Set-Content -LiteralPath (Join-Path $EvidencePath "phase-$Phase-diagnostics.json") -Encoding utf8NoBOM
    ConvertTo-Json -InputObject @($Observations | ForEach-Object { $_ }) -Depth 14 |
        Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
    ConvertTo-Json -InputObject @($Actions | ForEach-Object { $_ }) -Depth 8 |
        Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
    Write-Host "03.07 $Phase diagnostics:"
    Write-Host $json
}

function Invoke-InteractivePhase {
    param(
        [System.Diagnostics.Process]$RootProcess,
        [object]$Privilege,
        [ValidateSet('install','uninstall')][string]$Phase,
        [scriptblock]$CompletionTest,
        [int]$TimeoutSeconds = 210
    )

    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $visibleObserved = $false
    $quietCompletePolls = 0
    $lastFingerprint = ''

    while ([DateTime]::UtcNow -lt $deadline) {
        $completionNow = [bool](& $CompletionTest)
        $windows = [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]::new()
        Add-RelevantWindows -RootPid $RootProcess.Id -Result $windows

        if ($windows.Count -eq 0) {
            if ($completionNow) {
                $quietCompletePolls++
                if ($quietCompletePolls -ge 3) { break }
            } else {
                $quietCompletePolls = 0
            }
            Start-Sleep -Milliseconds 500
            continue
        }

        $visibleObserved = $true
        $quietCompletePolls = 0
        foreach ($window in $windows) {
            try { $window.SetFocus() } catch {}
            $title = Get-SafeName -Element $window
            $buttonElements = [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]::new()
            Add-ControlElements -Window $window -ControlType ([System.Windows.Automation.ControlType]::Button) -Result $buttonElements
            $buttonNames = @($buttonElements | ForEach-Object { Get-SafeName -Element $_ } | Where-Object { $_ })
            $fingerprint = "$Phase|$($window.Current.ProcessId)|$title|$($buttonNames -join '|')"

            if ($fingerprint -ne $lastFingerprint) {
                $controls = [System.Collections.Generic.List[object]]::new()
                Add-ControlSnapshot -Window $window -Result $controls
                [void]$Observations.Add([pscustomobject][ordered]@{
                    phase = $Phase
                    pid = [int]$window.Current.ProcessId
                    title = $title
                    buttons = $buttonNames
                    controls = @($controls)
                    at_utc = [DateTime]::UtcNow.ToString('o')
                })
                $lastFingerprint = $fingerprint
            }

            Set-CheckboxOffIfMatched -Window $window -Phase $Phase
            $clicked = Invoke-PrimaryButton -Window $window -Phase $Phase -CompletionReached $completionNow
            if ($clicked) {
                Start-Sleep -Milliseconds 900
                break
            }
        }
    }

    $completionReached = [bool](& $CompletionTest)
    $closed = $completionReached -and ($quietCompletePolls -ge 3)
    if ($closed) {
        Wait-Process -Id $RootProcess.Id -Timeout 15 -ErrorAction SilentlyContinue
    }
    $processExited = $false
    try { $processExited = $RootProcess.HasExited } catch { $processExited = $true }

    Write-PhaseDiagnostics -Phase $Phase -RootProcess $RootProcess -Privilege $Privilege -VisibleObserved $visibleObserved -CompletionReached $completionReached -TerminalClosed $closed

    Assert-Condition $visibleObserved "No visible NSIS $Phase window was observed; interactive evidence is invalid."
    Assert-Condition $completionReached "$Phase lifecycle did not reach its required state before timeout."
    Assert-Condition $closed "$Phase reached its required state but its terminal GUI did not close."
    Assert-Condition $processExited "$Phase terminal GUI closed but the root process did not exit."

    $phaseActions = @($Actions | Where-Object { $_.phase -eq $Phase -and $_.action -eq 'invoke-button' })
    Assert-Condition ($phaseActions.Count -ge 2) "Interactive $Phase did not visibly progress through the NSIS GUI."
    $terminalClicks = @($phaseActions | Where-Object { (($_.control -replace '&', '').Trim()) -match '(?i)^(Finish|Close)$' })
    Assert-Condition ($terminalClicks.Count -ge 1) "Interactive $Phase never invoked a terminal Finish/Close control."

    if ($Phase -eq 'install') {
        $progressClicks = @($phaseActions | Where-Object { (($_.control -replace '&', '').Trim()) -match '(?i)^(Install|Next\b)' })
        Assert-Condition ($progressClicks.Count -ge 1) 'Interactive install never invoked a primary NSIS progression control.'
    } else {
        $uninstallClicks = @($phaseActions | Where-Object { (($_.control -replace '&', '').Trim()) -match '(?i)^Uninstall$' })
        Assert-Condition ($uninstallClicks.Count -ge 1) 'Interactive uninstall never invoked the Uninstall button.'
    }

    return [pscustomobject]@{
        VisibleObserved = $visibleObserved
        CompletionReached = $completionReached
        TerminalClosed = $closed
    }
}

$actualHead = (git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"

$SetupPath = (Resolve-Path -LiteralPath $SetupPath).Path
Assert-Condition (Test-Path -LiteralPath $SetupPath -PathType Leaf) "Setup executable missing: $SetupPath"
Assert-Condition ((Get-Item -LiteralPath $SetupPath).Length -gt 0) 'Setup executable is empty.'
Assert-Condition (-not (Test-Path -LiteralPath $ExpectedRoot)) "Expected clean per-machine install root already exists: $ExpectedRoot"
Assert-Condition (-not (Test-Path -LiteralPath $ForbiddenUserRoot)) "Forbidden current-user install root already exists: $ForbiddenUserRoot"
Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) "Expected clean HKCU uninstall key already exists: $HkcuKey"
Assert-Condition (-not (Test-Path -LiteralPath $HklmKey)) "Expected clean HKLM uninstall key already exists: $HklmKey"

New-Item -ItemType Directory -Force -Path $EvidencePath | Out-Null
$setupHash = (Get-FileHash -LiteralPath $SetupPath -Algorithm SHA256).Hash.ToLowerInvariant()
$runnerPrivilege = Get-RunnerPrivilegeSnapshot
Assert-Condition $runnerPrivilege.administrator 'GitHub-hosted Windows runner is not in Administrators.'
Assert-Condition $runnerPrivilege.elevated 'GitHub-hosted Windows runner token is not elevated.'
Assert-Condition $runnerPrivilege.high_integrity 'GitHub-hosted Windows runner token is not high integrity.'
Assert-Condition $runnerPrivilege.uac_disabled 'Expected GitHub-hosted UAC-disabled environment was not observed.'

# Exact interactive entry point: empty argument vector, no /S, /P, /UPDATE or RunAs.
$setupProcess = Start-Process -FilePath $SetupPath -PassThru
$installerPrivilege = Get-ProcessPrivilegeSnapshot -ProcessId $setupProcess.Id
Assert-Condition $installerPrivilege.elevated 'Per-machine installer process is not elevated.'
Assert-Condition $installerPrivilege.high_integrity 'Per-machine installer process is not high integrity.'
$installerResult = Invoke-InteractivePhase -RootProcess $setupProcess -Privilege $installerPrivilege -Phase install -CompletionTest { Test-InstalledState }

$expectedRootCanonical = Get-CanonicalPath $ExpectedRoot
$programFilesCanonical = Get-CanonicalPath $env:ProgramFiles
Assert-Condition $expectedRootCanonical.StartsWith($programFilesCanonical, [StringComparison]::OrdinalIgnoreCase) "Per-machine install root is outside Program Files: $expectedRootCanonical"
Assert-Condition (Test-InstalledState) 'Installed per-machine state is incomplete after interactive setup.'
Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) 'Per-machine install created forbidden HKCU package registration.'
Assert-Condition (-not (Test-Path -LiteralPath $ForbiddenUserRoot)) 'Per-machine install created forbidden LocalAppData install root.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin/vsn.exe'))) '03.07 illegally packaged bin/vsn.exe before 03.10.'
Assert-Condition (-not (Test-Path -LiteralPath (Join-Path $ExpectedRoot 'bin/vsn-agent.exe'))) '03.07 illegally packaged bin/vsn-agent.exe before 03.10.'

$reg = Get-ItemProperty -LiteralPath $HklmKey
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

$escapedApp = @(Get-Process -ErrorAction SilentlyContinue | Where-Object {
    try { $_.Path -and (Get-CanonicalPath $_.Path) -eq (Get-CanonicalPath $installedExe) } catch { $false }
})
Assert-Condition ($escapedApp.Count -eq 0) 'Installer finish page launched the application; harness failed to keep the Run checkbox off.'

# Exact interactive uninstall entry point: empty argument vector, no /S, /P, /UPDATE or RunAs.
$uninstallProcess = Start-Process -FilePath $uninstaller -PassThru
$uninstallerPrivilege = Get-ProcessPrivilegeSnapshot -ProcessId $uninstallProcess.Id
Assert-Condition $uninstallerPrivilege.elevated 'Per-machine uninstaller process is not elevated.'
Assert-Condition $uninstallerPrivilege.high_integrity 'Per-machine uninstaller process is not high integrity.'
$uninstallerResult = Invoke-InteractivePhase -RootProcess $uninstallProcess -Privilege $uninstallerPrivilege -Phase uninstall -CompletionTest { Test-UninstalledState } -TimeoutSeconds 180

Assert-Condition (-not (Test-Path -LiteralPath $HklmKey)) 'HKLM uninstall registration remained after clean interactive uninstall.'
Assert-Condition (-not (Test-Path -LiteralPath $HkcuKey)) 'HKCU uninstall registration appeared during per-machine lifecycle.'
Assert-Condition (-not (Test-Path -LiteralPath $installedExe)) 'Desktop executable remained after clean interactive uninstall.'
Assert-Condition (-not (Test-Path -LiteralPath $uninstaller)) 'uninstall.exe remained after clean interactive uninstall.'
Assert-Condition (-not (Test-Path -LiteralPath $ForbiddenUserRoot)) 'Forbidden current-user install root appeared during per-machine lifecycle.'

$tracked = @(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) {
    $tracked | Write-Host
    throw 'Tracked repository drift detected during 03.07 interactive lifecycle.'
}

$evidence = [ordered]@{
    schema_version = 1
    package_id = 'PKG-03'
    task_id = '03.07'
    source_commit = $SourceSha
    setup = [ordered]@{
        filename = [System.IO.Path]::GetFileName($SetupPath)
        size_bytes = (Get-Item -LiteralPath $SetupPath).Length
        sha256 = $setupHash
        arguments = @()
        elevation_verb = $null
    }
    hosted_runner_privilege = $runnerPrivilege
    uac_boundary = [ordered]@{
        uac_disabled_runner_environment = $true
        uac_prompt_observed = $false
        uac_prompt_certified = $false
        explicit_runas_used = $false
    }
    per_machine_scope = [ordered]@{
        expected_install_root_token = '%ProgramFiles%\VSN Dev Platform'
        actual_install_root = $expectedRootCanonical
        hklm_registration_observed = $true
        hkcu_registration_absent = $true
        current_user_install_root_absent = $true
        display_name = $ProductName
        display_version = $ExpectedVersion
        publisher = $ExpectedPublisher
        uninstall_string_targeted_uninstall_exe = $true
    }
    process_privilege = [ordered]@{
        installer = $installerPrivilege
        uninstaller = $uninstallerPrivilege
    }
    installed_payload = [ordered]@{
        desktop_executable_observed = $true
        uninstaller_observed = $true
        cli_absent_until_03_10 = $true
        agent_absent_until_03_10 = $true
    }
    interaction = [ordered]@{
        visible_installer_window_observed = [bool]$installerResult.VisibleObserved
        visible_uninstaller_window_observed = [bool]$uninstallerResult.VisibleObserved
        installer_terminal_window_closed = [bool]$installerResult.TerminalClosed
        uninstaller_terminal_window_closed = [bool]$uninstallerResult.TerminalClosed
        passive_switch_used = $false
        silent_switch_used = $false
        update_switch_used = $false
        actions = @($Actions)
        observations_file = 'ui-observations.json'
    }
    clean_uninstall = [ordered]@{
        hklm_registration_removed = $true
        hkcu_registration_absent = $true
        desktop_executable_removed = $true
        uninstaller_removed = $true
        current_user_install_root_absent = $true
        destructive_app_data_option_selected = $false
    }
    scope_nonclaims = [ordered]@{
        uac_prompt_certified = $false
        standard_user_account_certified = $false
        msi_certified = $false
        shortcut_lifecycle_certified = $false
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
$evidence | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $evidenceFile -Encoding utf8NoBOM
$evidenceHash = (Get-FileHash -LiteralPath $evidenceFile -Algorithm SHA256).Hash.ToLowerInvariant()
"$evidenceHash  evidence.json" | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM
"$setupHash  $([System.IO.Path]::GetFileName($SetupPath))" | Set-Content -LiteralPath (Join-Path $EvidencePath 'setup.sha256') -Encoding utf8NoBOM

$evidence | ConvertTo-Json -Depth 12
