param(
    [Parameter(Mandatory = $true)][string]$SetupPath,
    [Parameter(Mandatory = $true)][string]$SourceSha,
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
    [DllImport("user32.dll", SetLastError=true)]
    public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll", SetLastError=true)]
    public static extern IntPtr GetAncestor(IntPtr hWnd, uint gaFlags);
    [DllImport("user32.dll", SetLastError=true)]
    public static extern int GetDlgCtrlID(IntPtr hWnd);
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsWindow(IntPtr hWnd);
}

public static class VsnToken {
    const uint PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
    const uint TOKEN_QUERY = 0x0008;
    const int TokenElevation = 20;
    const int TokenIntegrityLevel = 25;

    [StructLayout(LayoutKind.Sequential)]
    struct TOKEN_ELEVATION { public int TokenIsElevated; }
    [StructLayout(LayoutKind.Sequential)]
    struct SID_AND_ATTRIBUTES { public IntPtr Sid; public uint Attributes; }
    [StructLayout(LayoutKind.Sequential)]
    struct TOKEN_MANDATORY_LABEL { public SID_AND_ATTRIBUTES Label; }

    [DllImport("kernel32.dll", SetLastError=true)]
    static extern IntPtr OpenProcess(uint access, bool inheritHandle, int processId);
    [DllImport("kernel32.dll", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    static extern bool CloseHandle(IntPtr handle);
    [DllImport("advapi32.dll", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    static extern bool OpenProcessToken(IntPtr processHandle, uint desiredAccess, out IntPtr tokenHandle);
    [DllImport("advapi32.dll", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    static extern bool GetTokenInformation(IntPtr tokenHandle, int tokenInfoClass, IntPtr tokenInfo, int tokenInfoLength, out int returnLength);
    [DllImport("advapi32.dll", SetLastError=true)]
    static extern IntPtr GetSidSubAuthorityCount(IntPtr sid);
    [DllImport("advapi32.dll", SetLastError=true)]
    static extern IntPtr GetSidSubAuthority(IntPtr sid, uint subAuthority);

    static IntPtr OpenToken(int pid, out IntPtr processHandle) {
        processHandle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        if (processHandle == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error(), "OpenProcess failed");
        IntPtr token;
        if (!OpenProcessToken(processHandle, TOKEN_QUERY, out token)) {
            int e = Marshal.GetLastWin32Error();
            CloseHandle(processHandle);
            processHandle = IntPtr.Zero;
            throw new Win32Exception(e, "OpenProcessToken failed");
        }
        return token;
    }

    public static bool IsElevated(int pid) {
        IntPtr process = IntPtr.Zero, token = IntPtr.Zero, buffer = IntPtr.Zero;
        try {
            token = OpenToken(pid, out process);
            int n = Marshal.SizeOf(typeof(TOKEN_ELEVATION)), returned;
            buffer = Marshal.AllocHGlobal(n);
            if (!GetTokenInformation(token, TokenElevation, buffer, n, out returned))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "GetTokenInformation(TokenElevation) failed");
            return ((TOKEN_ELEVATION)Marshal.PtrToStructure(buffer, typeof(TOKEN_ELEVATION))).TokenIsElevated != 0;
        } finally {
            if (buffer != IntPtr.Zero) Marshal.FreeHGlobal(buffer);
            if (token != IntPtr.Zero) CloseHandle(token);
            if (process != IntPtr.Zero) CloseHandle(process);
        }
    }

    public static int IntegrityRid(int pid) {
        IntPtr process = IntPtr.Zero, token = IntPtr.Zero, buffer = IntPtr.Zero;
        try {
            token = OpenToken(pid, out process);
            int needed;
            GetTokenInformation(token, TokenIntegrityLevel, IntPtr.Zero, 0, out needed);
            if (needed <= 0) throw new Win32Exception(Marshal.GetLastWin32Error(), "Unable to size TokenIntegrityLevel");
            buffer = Marshal.AllocHGlobal(needed);
            if (!GetTokenInformation(token, TokenIntegrityLevel, buffer, needed, out needed))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "GetTokenInformation(TokenIntegrityLevel) failed");
            var label = (TOKEN_MANDATORY_LABEL)Marshal.PtrToStructure(buffer, typeof(TOKEN_MANDATORY_LABEL));
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

function Assert-Condition([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Get-CanonicalPath([string]$Path) {
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\')
}

function Get-ProcessPrivilegeSnapshot([int]$ProcessId) {
    $lastError = ''
    for ($attempt = 1; $attempt -le 15; $attempt++) {
        try {
            $elevated = [VsnToken]::IsElevated($ProcessId)
            $rid = [VsnToken]::IntegrityRid($ProcessId)
            return [pscustomobject][ordered]@{
                pid = $ProcessId
                elevated = [bool]$elevated
                integrity_rid = [int]$rid
                high_integrity = [bool]($rid -ge 0x3000)
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
    $token = Get-ProcessPrivilegeSnapshot -ProcessId $PID
    $enableLua = [int](Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -Name EnableLUA -ErrorAction Stop).EnableLUA
    Assert-Condition ($enableLua -in @(0, 1)) "Unexpected EnableLUA value: $enableLua"
    return [pscustomobject][ordered]@{
        identity = $identity.Name
        administrator = [bool]$principal.IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)
        elevated = [bool]$token.elevated
        integrity_rid = [int]$token.integrity_rid
        high_integrity = [bool]$token.high_integrity
        enable_lua = $enableLua
        uac_disabled = [bool]($enableLua -eq 0)
    }
}

function Get-SafeName([System.Windows.Automation.AutomationElement]$Element) {
    try { return ([string]$Element.Current.Name).Trim() } catch { return '' }
}

function Get-RelevantWindows([int]$RootPid) {
    $snapshot = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    [void]$ids.Add($RootPid)
    do {
        $changed = $false
        foreach ($p in $snapshot) {
            $pidNow = [int]$p.ProcessId
            $ppid = [int]$p.ParentProcessId
            if ($ids.Contains($ppid) -and -not $ids.Contains($pidNow)) {
                [void]$ids.Add($pidNow)
                $changed = $true
            }
        }
    } while ($changed)

    $result = [System.Collections.Generic.List[System.Windows.Automation.AutomationElement]]::new()
    $root = [System.Windows.Automation.AutomationElement]::RootElement
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($element in $all) {
        try {
            $name = [string]$element.Current.Name
            $pidNow = [int]$element.Current.ProcessId
            $visible = -not [bool]$element.Current.IsOffscreen
            $handle = [int]$element.Current.NativeWindowHandle
            $titleFallback = $name -match '(?i)VSN Dev Platform.*(Setup|Install|Uninstall)|(Setup|Install|Uninstall).*VSN Dev Platform'
            if ($visible -and $handle -ne 0 -and ($ids.Contains($pidNow) -or $titleFallback)) {
                [void]$result.Add($element)
            }
        } catch {}
    }
    return $result
}

function Get-Controls([System.Windows.Automation.AutomationElement]$Window, [System.Windows.Automation.ControlType]$Type) {
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty, $Type
    )
    return @($Window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition))
}

function Record-Window([string]$Phase, [System.Windows.Automation.AutomationElement]$Window) {
    $buttons = @(Get-Controls $Window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
    [void]$Observations.Add([pscustomobject][ordered]@{
        phase = $Phase
        pid = [int]$Window.Current.ProcessId
        title = Get-SafeName $Window
        buttons = $buttons
        at_utc = [DateTime]::UtcNow.ToString('o')
    })
}

function Set-SafetyCheckboxes([string]$Phase, [System.Windows.Automation.AutomationElement]$Window) {
    foreach ($box in @(Get-Controls $Window ([System.Windows.Automation.ControlType]::CheckBox))) {
        $name = Get-SafeName $box
        $mustOff = (
            ($Phase -eq 'install' -and $name -match '(?i)run.*VSN Dev Platform|launch.*VSN Dev Platform') -or
            ($Phase -eq 'uninstall' -and $name -match '(?i)delete.*(app.*data|data)|remove.*(app.*data|user.*data)')
        )
        if (-not $mustOff) { continue }
        try {
            $toggle = [System.Windows.Automation.TogglePattern]$box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
            if ($toggle.Current.ToggleState -ne [System.Windows.Automation.ToggleState]::Off) {
                $toggle.Toggle()
                Start-Sleep -Milliseconds 250
            }
            [void]$Actions.Add([pscustomobject][ordered]@{
                phase=$Phase; action='ensure-checkbox-off'; control=$name; at_utc=[DateTime]::UtcNow.ToString('o')
            })
        } catch {}
    }
}

function Invoke-TerminalFallback(
    [string]$Phase,
    [System.Windows.Automation.AutomationElement]$Window,
    [System.Windows.Automation.AutomationElement]$Button,
    [string]$ButtonName,
    [bool]$CompletionReached
) {
    if (-not $CompletionReached) { return }
    try { $buttonHandle = [IntPtr][int]$Button.Current.NativeWindowHandle } catch { return }
    if ($buttonHandle -eq [IntPtr]::Zero -or -not [VsnNativeUi]::IsWindow($buttonHandle)) { return }
    $rootHandle = [VsnNativeUi]::GetAncestor($buttonHandle, [uint32]2)
    if ($rootHandle -eq [IntPtr]::Zero) {
        try { $rootHandle = [IntPtr][int]$Window.Current.NativeWindowHandle } catch { return }
    }
    if ($rootHandle -eq [IntPtr]::Zero) { return }
    $key = "${Phase}:$($rootHandle.ToInt64())"
    if (-not $TerminalFallbackRoots.Add($key)) { return }

    $controlId = [VsnNativeUi]::GetDlgCtrlID($buttonHandle)
    if ($controlId -gt 0) {
        [void][VsnNativeUi]::SendMessage($rootHandle, [uint32]0x0111, [IntPtr]$controlId, $buttonHandle)
        [void]$Actions.Add([pscustomobject][ordered]@{
            phase=$Phase; action='native-wm-command-fallback'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
        })
        Start-Sleep -Milliseconds 400
    }
    if ([VsnNativeUi]::IsWindow($rootHandle)) {
        [void][VsnNativeUi]::PostMessage($rootHandle, [uint32]0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
        [void]$Actions.Add([pscustomobject][ordered]@{
            phase=$Phase; action='native-wm-close-terminal-fallback'; control=$ButtonName; at_utc=[DateTime]::UtcNow.ToString('o')
        })
    }
}

function Invoke-PrimaryButton(
    [string]$Phase,
    [System.Windows.Automation.AutomationElement]$Window,
    [bool]$CompletionReached
) {
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
    $priority = if ($Phase -eq 'install') { @('^Install$', '^Next\b', '^Finish$', '^Close$') } else { @('^Uninstall$', '^Next\b', '^Finish$', '^Close$') }
    foreach ($pattern in $priority) {
        $selected = $candidates | Where-Object { $_.Normalized -match "(?i)$pattern" } | Select-Object -First 1
        if ($null -eq $selected) { continue }
        try {
            $invoke = [System.Windows.Automation.InvokePattern]$selected.Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
            $invoke.Invoke()
            [void]$Actions.Add([pscustomobject][ordered]@{
                phase=$Phase; action='invoke-button'; control=$selected.Name; at_utc=[DateTime]::UtcNow.ToString('o')
            })
            if ($selected.Normalized -match '(?i)^(Finish|Close)$') {
                Start-Sleep -Milliseconds 350
                Invoke-TerminalFallback $Phase $Window $selected.Element $selected.Name $CompletionReached
            }
            return $selected.Normalized
        } catch {
            continue
        }
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

function Write-Diagnostics(
    [string]$Phase,
    [System.Diagnostics.Process]$RootProcess,
    [object]$Privilege,
    [bool]$Visible,
    [bool]$Complete,
    [bool]$Closed
) {
    New-Item -ItemType Directory -Force -Path $EvidencePath | Out-Null
    $data = [ordered]@{
        schema_version=1; package_id='PKG-03'; task_id='03.07'; source_commit=$SourceSha
        phase=$Phase; visible_window_observed=$Visible; completion_reached=$Complete
        terminal_window_closed=$Closed; root_process_id=$RootProcess.Id; root_process_privilege=$Privilege
        state=[ordered]@{
            expected_root=$ExpectedRoot
            expected_root_exists=(Test-Path $ExpectedRoot)
            forbidden_user_root=$ForbiddenUserRoot
            forbidden_user_root_exists=(Test-Path $ForbiddenUserRoot)
            desktop_executable_exists=(Test-Path (Join-Path $ExpectedRoot 'VSN Dev Platform.exe'))
            uninstaller_exists=(Test-Path (Join-Path $ExpectedRoot 'uninstall.exe'))
            hkcu_uninstall_key_exists=(Test-Path $HkcuKey)
            hklm_uninstall_key_exists=(Test-Path $HklmKey)
        }
        actions=@($Actions | Where-Object { $_.phase -eq $Phase })
        observations=@($Observations | Where-Object { $_.phase -eq $Phase })
        captured_at_utc=[DateTime]::UtcNow.ToString('o')
    }
    $data | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $EvidencePath "phase-$Phase-diagnostics.json") -Encoding utf8NoBOM
    ConvertTo-Json -InputObject @($Observations | ForEach-Object { $_ }) -Depth 8 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-observations.json') -Encoding utf8NoBOM
    ConvertTo-Json -InputObject @($Actions | ForEach-Object { $_ }) -Depth 8 | Set-Content -LiteralPath (Join-Path $EvidencePath 'ui-actions.json') -Encoding utf8NoBOM
}

function Invoke-InteractivePhase(
    [System.Diagnostics.Process]$RootProcess,
    [object]$Privilege,
    [ValidateSet('install','uninstall')][string]$Phase,
    [scriptblock]$CompletionTest,
    [int]$TimeoutSeconds = 210
) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    $visible = $false
    $quiet = 0
    $lastFingerprint = ''
    while ([DateTime]::UtcNow -lt $deadline) {
        $completeNow = [bool](& $CompletionTest)
        $windows = @(Get-RelevantWindows $RootProcess.Id)
        if ($windows.Count -eq 0) {
            if ($completeNow) { $quiet++ } else { $quiet = 0 }
            if ($quiet -ge 3) { break }
            Start-Sleep -Milliseconds 500
            continue
        }
        $visible = $true
        $quiet = 0
        foreach ($window in $windows) {
            try { $window.SetFocus() } catch {}
            $names = @(Get-Controls $window ([System.Windows.Automation.ControlType]::Button) | ForEach-Object { Get-SafeName $_ } | Where-Object { $_ })
            $fingerprint = "$Phase|$($window.Current.ProcessId)|$(Get-SafeName $window)|$($names -join '|')"
            if ($fingerprint -ne $lastFingerprint) {
                Record-Window $Phase $window
                $lastFingerprint = $fingerprint
            }
            Set-SafetyCheckboxes $Phase $window
            $clicked = Invoke-PrimaryButton $Phase $window $completeNow
            if ($clicked) { Start-Sleep -Milliseconds 900; break }
        }
    }

    $complete = [bool](& $CompletionTest)
    $closed = $complete -and ($quiet -ge 3)
    if ($closed) { Wait-Process -Id $RootProcess.Id -Timeout 15 -ErrorAction SilentlyContinue }
    $exited = $false
    try { $exited = $RootProcess.HasExited } catch { $exited = $true }
    Write-Diagnostics $Phase $RootProcess $Privilege $visible $complete $closed

    Assert-Condition $visible "No visible NSIS $Phase window was observed."
    Assert-Condition $complete "$Phase lifecycle did not reach required state."
    Assert-Condition $closed "$Phase terminal GUI did not close."
    Assert-Condition $exited "$Phase root process did not exit."
    $phaseActions = @($Actions | Where-Object { $_.phase -eq $Phase -and $_.action -eq 'invoke-button' })
    Assert-Condition ($phaseActions.Count -ge 2) "Interactive $Phase did not visibly progress through at least two GUI controls."
    Assert-Condition (@($phaseActions | Where-Object { (($_.control -replace '&','').Trim()) -match '(?i)^(Finish|Close)$' }).Count -ge 1) "$Phase never invoked Finish/Close."
    if ($Phase -eq 'install') {
        Assert-Condition (@($phaseActions | Where-Object { (($_.control -replace '&','').Trim()) -match '(?i)^(Install|Next\b)' }).Count -ge 1) 'Install never invoked Install/Next.'
    } else {
        Assert-Condition (@($phaseActions | Where-Object { (($_.control -replace '&','').Trim()) -match '(?i)^Uninstall$' }).Count -ge 1) 'Uninstall never invoked Uninstall.'
    }
    return [pscustomobject]@{ VisibleObserved=$visible; CompletionReached=$complete; TerminalClosed=$closed }
}

$actualHead = (git rev-parse HEAD).Trim()
Assert-Condition ($actualHead -eq $SourceSha) "Source SHA mismatch: expected=$SourceSha actual=$actualHead"
$SetupPath = (Resolve-Path -LiteralPath $SetupPath).Path
Assert-Condition ((Get-Item -LiteralPath $SetupPath).Length -gt 0) "Setup missing/empty: $SetupPath"
Assert-Condition (-not (Test-Path $ExpectedRoot)) "Expected clean Program Files root already exists: $ExpectedRoot"
Assert-Condition (-not (Test-Path $ForbiddenUserRoot)) "Forbidden LocalAppData root already exists: $ForbiddenUserRoot"
Assert-Condition (-not (Test-Path $HkcuKey)) "Expected clean HKCU key already exists: $HkcuKey"
Assert-Condition (-not (Test-Path $HklmKey)) "Expected clean HKLM key already exists: $HklmKey"

New-Item -ItemType Directory -Force -Path $EvidencePath | Out-Null
$setupHash = (Get-FileHash -LiteralPath $SetupPath -Algorithm SHA256).Hash.ToLowerInvariant()
$runnerPrivilege = Get-RunnerPrivilegeSnapshot
Assert-Condition $runnerPrivilege.administrator 'Runner is not in Administrators.'
Assert-Condition $runnerPrivilege.elevated 'Runner token is not elevated.'
Assert-Condition $runnerPrivilege.high_integrity 'Runner token is not high integrity.'
Assert-Condition ($runnerPrivilege.enable_lua -in @(0,1)) "Unexpected EnableLUA: $($runnerPrivilege.enable_lua)"

# Exact interactive entry point: empty argument vector, no /S, /P, /UPDATE or RunAs.
$setupProcess = Start-Process -FilePath $SetupPath -PassThru
$installerPrivilege = Get-ProcessPrivilegeSnapshot -ProcessId $setupProcess.Id
Assert-Condition $installerPrivilege.elevated 'Per-machine installer process is not elevated.'
Assert-Condition $installerPrivilege.high_integrity 'Per-machine installer process is not high integrity.'
$installerResult = Invoke-InteractivePhase $setupProcess $installerPrivilege install { Test-InstalledState }

$expectedRootCanonical = Get-CanonicalPath $ExpectedRoot
$programFilesCanonical = Get-CanonicalPath $env:ProgramFiles
Assert-Condition $expectedRootCanonical.StartsWith($programFilesCanonical, [StringComparison]::OrdinalIgnoreCase) "Install root is outside Program Files: $expectedRootCanonical"
Assert-Condition (Test-InstalledState) 'Installed per-machine state is incomplete.'
Assert-Condition (-not (Test-Path $HkcuKey)) 'Per-machine install created forbidden HKCU registration.'
Assert-Condition (-not (Test-Path $ForbiddenUserRoot)) 'Per-machine install created forbidden LocalAppData root.'
Assert-Condition (-not (Test-Path (Join-Path $ExpectedRoot 'bin/vsn.exe'))) '03.07 packaged bin/vsn.exe before 03.10.'
Assert-Condition (-not (Test-Path (Join-Path $ExpectedRoot 'bin/vsn-agent.exe'))) '03.07 packaged bin/vsn-agent.exe before 03.10.'

$reg = Get-ItemProperty -LiteralPath $HklmKey
Assert-Condition ([string]$reg.DisplayName -eq $ProductName) "DisplayName mismatch: $($reg.DisplayName)"
Assert-Condition ([string]$reg.DisplayVersion -eq $ExpectedVersion) "DisplayVersion mismatch: $($reg.DisplayVersion)"
Assert-Condition ([string]$reg.Publisher -eq $ExpectedPublisher) "Publisher mismatch: $($reg.Publisher)"
$registeredLocation = ([string]$reg.InstallLocation).Trim().Trim('"')
Assert-Condition ((Get-CanonicalPath $registeredLocation) -eq $expectedRootCanonical) "InstallLocation mismatch: $registeredLocation"
Assert-Condition ([string]$reg.UninstallString -match '(?i)uninstall\.exe') "UninstallString does not target uninstall.exe: $($reg.UninstallString)"

$installedExe = Join-Path $ExpectedRoot 'VSN Dev Platform.exe'
$uninstaller = Join-Path $ExpectedRoot 'uninstall.exe'
Assert-Condition (Test-Path $installedExe -PathType Leaf) 'Installed Desktop executable missing.'
Assert-Condition (Test-Path $uninstaller -PathType Leaf) 'Installed uninstaller missing.'

$escapedApp = @(Get-Process -ErrorAction SilentlyContinue | Where-Object {
    try { $_.Path -and (Get-CanonicalPath $_.Path) -eq (Get-CanonicalPath $installedExe) } catch { $false }
})
Assert-Condition ($escapedApp.Count -eq 0) 'Installer finish page launched the application.'

# Exact interactive uninstall entry point: empty argument vector, no /S, /P, /UPDATE or RunAs.
$uninstallProcess = Start-Process -FilePath $uninstaller -PassThru
$uninstallerPrivilege = Get-ProcessPrivilegeSnapshot -ProcessId $uninstallProcess.Id
Assert-Condition $uninstallerPrivilege.elevated 'Per-machine uninstaller process is not elevated.'
Assert-Condition $uninstallerPrivilege.high_integrity 'Per-machine uninstaller process is not high integrity.'
$uninstallerResult = Invoke-InteractivePhase $uninstallProcess $uninstallerPrivilege uninstall { Test-UninstalledState } 180

Assert-Condition (-not (Test-Path $HklmKey)) 'HKLM registration remained after uninstall.'
Assert-Condition (-not (Test-Path $HkcuKey)) 'HKCU registration appeared during per-machine lifecycle.'
Assert-Condition (-not (Test-Path $installedExe)) 'Desktop executable remained after uninstall.'
Assert-Condition (-not (Test-Path $uninstaller)) 'uninstall.exe remained after uninstall.'
Assert-Condition (-not (Test-Path $ForbiddenUserRoot)) 'LocalAppData root appeared during per-machine lifecycle.'

$tracked = @(git status --porcelain=v1 --untracked-files=no)
if ($tracked.Count -ne 0) { $tracked | Write-Host; throw 'Tracked repository drift detected during 03.07.' }

$evidence = [ordered]@{
    schema_version=1
    package_id='PKG-03'
    task_id='03.07'
    source_commit=$SourceSha
    setup=[ordered]@{
        filename=[System.IO.Path]::GetFileName($SetupPath)
        size_bytes=(Get-Item $SetupPath).Length
        sha256=$setupHash
        arguments=@()
        elevation_verb=$null
    }
    hosted_runner_privilege=$runnerPrivilege
    uac_boundary=[ordered]@{
        enable_lua=[int]$runnerPrivilege.enable_lua
        uac_disabled_runner_environment=[bool]$runnerPrivilege.uac_disabled
        uac_policy_measured=$true
        uac_prompt_observed=$false
        uac_prompt_certified=$false
        explicit_runas_used=$false
    }
    per_machine_scope=[ordered]@{
        expected_install_root_token='%ProgramFiles%\VSN Dev Platform'
        actual_install_root=$expectedRootCanonical
        hklm_registration_observed=$true
        hkcu_registration_absent=$true
        current_user_install_root_absent=$true
        display_name=$ProductName
        display_version=$ExpectedVersion
        publisher=$ExpectedPublisher
        uninstall_string_targeted_uninstall_exe=$true
    }
    process_privilege=[ordered]@{ installer=$installerPrivilege; uninstaller=$uninstallerPrivilege }
    installed_payload=[ordered]@{
        desktop_executable_observed=$true
        uninstaller_observed=$true
        cli_absent_until_03_10=$true
        agent_absent_until_03_10=$true
    }
    interaction=[ordered]@{
        visible_installer_window_observed=[bool]$installerResult.VisibleObserved
        visible_uninstaller_window_observed=[bool]$uninstallerResult.VisibleObserved
        installer_terminal_window_closed=[bool]$installerResult.TerminalClosed
        uninstaller_terminal_window_closed=[bool]$uninstallerResult.TerminalClosed
        passive_switch_used=$false
        silent_switch_used=$false
        update_switch_used=$false
        actions=@($Actions)
        observations_file='ui-observations.json'
    }
    clean_uninstall=[ordered]@{
        hklm_registration_removed=$true
        hkcu_registration_absent=$true
        desktop_executable_removed=$true
        uninstaller_removed=$true
        current_user_install_root_absent=$true
        destructive_app_data_option_selected=$false
    }
    scope_nonclaims=[ordered]@{
        uac_prompt_certified=$false
        fixed_uac_policy_certified=$false
        standard_user_account_certified=$false
        msi_certified=$false
        shortcut_lifecycle_certified=$false
        cli_agent_placement_certified=$false
        service_lifecycle_certified=$false
        acl_lifecycle_certified=$false
        silent_deployment_certified=$false
        signing_performed=$false
        updater_mutation=$false
    }
    tracked_repository_drift_zero=$true
}

$evidenceFile = Join-Path $EvidencePath 'evidence.json'
$evidence | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $evidenceFile -Encoding utf8NoBOM
$evidenceHash = (Get-FileHash $evidenceFile -Algorithm SHA256).Hash.ToLowerInvariant()
"$evidenceHash  evidence.json" | Set-Content -LiteralPath (Join-Path $EvidencePath 'evidence.json.sha256') -Encoding utf8NoBOM
"$setupHash  $([System.IO.Path]::GetFileName($SetupPath))" | Set-Content -LiteralPath (Join-Path $EvidencePath 'setup.sha256') -Encoding utf8NoBOM
$evidence | ConvertTo-Json -Depth 12
