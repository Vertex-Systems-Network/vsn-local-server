param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$FeatureId = 'pkg02-0223-test-dns'
$FeatureVersion = '1.0.0'
$CanonicalBaseSha = '94feeb8e67dad96ac6a384a8517229ba2c5c38f5'
$PlanSha256 = 'cc9b7b503c87d4ede7fb625e080500049fd0d3c4f0d8cdd956f2d7747c3db9ed'
$ResearchSha256 = '05a7a1116eedf9308abf6bd8852a7369134b0c5db473ce884e3fc25fb3a3a71d'
$LifecycleSha256 = '3012cef4a49d218ceaf5d75434c8f828d802afa2e1184b14f198c2ab247d95ff'
$PreflightSha256 = '8a2a921c319cc1e4efe591319089dcf63246679376685f69dae3ba63ea34620a'
$CandidateId = 'c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474'
$ProductVersion = '0.38.1'
$AgentIpcPort = 39731

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

function Stop-ProcessSafe($Process) {
    if ($null -ne $Process) {
        try {
            if (-not $Process.HasExited) {
                Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
                Wait-Process -Id $Process.Id -Timeout 10 -ErrorAction SilentlyContinue
            }
        } catch {}
    }
}

function Test-ProcessStopped([int]$ProcessId) {
    if ($ProcessId -le 0) { return $true }
    return $null -eq (Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)
}

function Get-FreeUdpPort {
    $socket = [Net.Sockets.UdpClient]::new()
    try {
        $socket.Client.Bind([Net.IPEndPoint]::new([Net.IPAddress]::Loopback, 0))
        return ([Net.IPEndPoint]$socket.Client.LocalEndPoint).Port
    }
    finally {
        $socket.Dispose()
    }
}

function Test-UdpPortFree([int]$Port) {
    if ($Port -le 0) { return $true }
    try {
        return -not [bool](Get-NetUDPEndpoint -LocalPort $Port -ErrorAction SilentlyContinue)
    } catch {
        $probeSocket = [Net.Sockets.UdpClient]::new()
        try {
            $probeSocket.Client.Bind([Net.IPEndPoint]::new([Net.IPAddress]::Loopback, $Port))
            return $true
        } catch {
            return $false
        } finally {
            $probeSocket.Dispose()
        }
    }
}

function Assert-FileSha([string]$Path, [string]$Expected, [string]$Name) {
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected) { throw "$Name digest mismatch: expected=$Expected actual=$actual" }
}

function Invoke-CliJson([string[]]$Arguments, [string]$Name) {
    $stdout = Join-Path $script:Root "$Name.stdout.json"
    $stderr = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @Arguments 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $script:Root "$Name.exit-code.txt")
    if ($code -ne 0) {
        $detail = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
        throw "CLI $Name failed (exit=$code): $detail"
    }
    return (Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json)
}

function Invoke-CliFailure([string[]]$Arguments, [string]$Name) {
    $stdout = Join-Path $script:Root "$Name.stdout.log"
    $stderr = Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @Arguments 1> $stdout 2> $stderr
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $script:Root "$Name.exit-code.txt")
    if ($code -eq 0) { throw "CLI $Name unexpectedly succeeded" }
    return [pscustomobject]@{
        ExitCode = $code
        Stdout = if (Test-Path -LiteralPath $stdout) { Get-Content -LiteralPath $stdout -Raw } else { '' }
        Stderr = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
    }
}

function Start-Agent {
    $script:Agent = Start-Process -FilePath $script:AgentExe `
        -RedirectStandardOutput (Join-Path $script:Root 'agent.stdout.log') `
        -RedirectStandardError (Join-Path $script:Root 'agent.stderr.log') `
        -PassThru -WindowStyle Hidden
    $watch = [Diagnostics.Stopwatch]::StartNew()
    foreach ($i in 1..100) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) {
            $watch.Stop()
            $script:AgentReadinessMs = [int64]$watch.ElapsedMilliseconds
            return
        }
        if ($script:Agent.HasExited) { throw "Agent exited before readiness with code $($script:Agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent did not become ready within 25 seconds'
}

function Invoke-DnsProbe([string]$Mode, [string]$Listen, [string]$Name, [int]$QType, [string]$ArtifactName) {
    $stdout = Join-Path $script:Root "$ArtifactName.stdout.json"
    $stderr = Join-Path $script:Root "$ArtifactName.stderr.log"
    if ($Mode -eq 'query') {
        & $script:DnsProbe query $Listen $Name $QType 1> $stdout 2> $stderr
    } else {
        & $script:DnsProbe $Mode $Listen 1> $stdout 2> $stderr
    }
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $script:Root "$ArtifactName.exit-code.txt")
    if ($code -ne 0) {
        $detail = if (Test-Path -LiteralPath $stderr) { Get-Content -LiteralPath $stderr -Raw } else { '' }
        throw "DNS probe $ArtifactName failed (exit=$code): $detail"
    }
    return (Get-Content -LiteralPath $stdout -Raw | ConvertFrom-Json)
}

function Test-DnsNoResponse([string]$Mode, [string]$Listen, [string]$ArtifactName) {
    $stdout = Join-Path $script:Root "$ArtifactName.stdout.log"
    $stderr = Join-Path $script:Root "$ArtifactName.stderr.log"
    if ($Mode -eq 'query') {
        & $script:DnsProbe query $Listen demo.test 1 1> $stdout 2> $stderr
    } else {
        & $script:DnsProbe $Mode $Listen 1> $stdout 2> $stderr
    }
    $code = $LASTEXITCODE
    $code | Set-Content -LiteralPath (Join-Path $script:Root "$ArtifactName.exit-code.txt")
    if ($code -eq 0) { throw "DNS probe $ArtifactName unexpectedly received a response" }
    if ($code -ne 2) { throw "DNS probe $ArtifactName failed for an unexpected reason (exit=$code)" }
    return $true
}

function Wait-DnsReady([string]$Listen, [string]$Name) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $attempt = 0
    while ($watch.ElapsedMilliseconds -lt 4500) {
        $attempt++
        $stdout = Join-Path $script:Root "$Name-ready-$attempt.stdout.json"
        $stderr = Join-Path $script:Root "$Name-ready-$attempt.stderr.log"
        & $script:DnsProbe query $Listen demo.test 1 1> $stdout 2> $stderr
        if ($LASTEXITCODE -eq 0) {
            $watch.Stop()
            if ($watch.ElapsedMilliseconds -ge 5000) { throw "$Name DNS readiness exceeded 5 seconds" }
            return [int64]$watch.ElapsedMilliseconds
        }
        Start-Sleep -Milliseconds 50
    }
    $watch.Stop()
    throw "$Name DNS responder did not become ready within 5 seconds"
}

function Stop-DnsBestEffort {
    if ($script:Cli -and $script:Agent -and -not $script:Agent.HasExited) {
        try { & $script:Cli dns stop *> $null } catch {}
    }
}

$script:Root = Join-Path $PWD 'dist-self-hosted\02.23'
$bin = Join-Path $script:Root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0223-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$originalIpcBytes = if ($hadIpcKey) { [IO.File]::ReadAllBytes($ipcKey) } else { $null }
$originalIpcHash = if ($hadIpcKey) { (Get-FileHash -LiteralPath $ipcKey -Algorithm SHA256).Hash.ToLowerInvariant() } else { $null }
$script:Agent = $null
$script:AgentExe = $null
$script:Cli = $null
$script:DnsProbe = $null
$script:AgentReadinessMs = 0
$dnsPort = 0
$occupiedPort = 0
$occupiedSocket = $null
$trackedDnsPids = [System.Collections.Generic.List[int]]::new()
$success = $false

if (Test-Path -LiteralPath $script:Root) { Remove-Item -LiteralPath $script:Root -Recurse -Force }
New-Item -ItemType Directory -Force -Path $script:Root,$bin,$sandbox,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

try {
    if (-not $IsWindows) { throw '02.23 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.23 certification requires a GitHub-hosted runner' }
    if (-not $env:EXPECTED_SHA) { throw 'EXPECTED_SHA is required for exact-source evidence binding' }
    $actualHead = (git rev-parse HEAD).Trim()
    if ($actualHead -ne $env:EXPECTED_SHA) { throw "02.23 checkout does not match EXPECTED_SHA: expected=$env:EXPECTED_SHA actual=$actualHead" }
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }
    if (Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue) {
        throw "VSN Agent IPC port $AgentIpcPort is occupied; refusing to disturb another Agent"
    }

    Assert-FileSha '.ai\plans\pkg02-0223-test-dns-v1.md' $PlanSha256 'frozen 02.23 plan'
    Assert-FileSha '.ai\features\pkg02-0223\research.md' $ResearchSha256 '02.23 research'
    Assert-FileSha '.ai\features\pkg02-0223\lifecycle-review.md' $LifecycleSha256 '02.23 lifecycle review'
    Assert-FileSha '.ai\features\pkg02-0223\development-preflight.md' $PreflightSha256 '02.23 development preflight'
    $manifest = Get-Content '.ai\manifests\pkg02-0223-test-dns.v1.json' -Raw | ConvertFrom-Json
    if ([string]$manifest.feature_id -ne $FeatureId -or [string]$manifest.version -ne $FeatureVersion) { throw '02.23 feature manifest identity mismatch' }
    if ([string]$manifest.canonical_base_sha -ne $CanonicalBaseSha) { throw '02.23 feature manifest canonical base mismatch' }
    if ([string]$manifest.plan.sha256 -ne $PlanSha256) { throw '02.23 feature manifest plan digest mismatch' }
    if ([string]$manifest.research.market_delta -ne 'none') { throw '02.23 market/protocol delta is not cleared' }
    foreach ($stage in @('research','plan','architecture','data_flow','security','design','qa','performance')) {
        $state = [string]$manifest.stages.$stage.status
        if ($state -notin @('complete','approved')) { throw "02.23 lifecycle stage not complete: $stage=$state" }
    }
    if ([string]$manifest.stages.development.status -ne 'ready') { throw '02.23 development preflight is not ready' }

    $networkSource = Get-Content 'crates\vsn-network\src\lib.rs' -Raw
    foreach ($needle in @(
        'pub fn dns_resolver_plan',
        'pub fn run_dns_server',
        'DNS responder suffix must remain .test',
        'DNS listener must bind to loopback',
        'DNS listener port must be non-zero',
        'DNS baseline accepts exactly one question',
        'compressed query names are not accepted by the local DNS baseline',
        'DNS name exceeds 255 bytes',
        'name == "test" || name.ends_with(".test")',
        'let rcode = if local { 0u16 } else { 5u16 }',
        '127, 0, 0, 1',
        'Ipv6Addr::LOCALHOST'
    )) {
        if (-not $networkSource.Contains($needle)) { throw "missing 02.23 DNS invariant: $needle" }
    }
    $coreSource = Get-Content 'crates\vsn-core\src\lib.rs' -Raw
    foreach ($needle in @('pub fn dns_plan','pub fn dns_start','pub fn dns_status','pub fn dns_stop','id: "vsn-dns"','Permission::NetworkManage')) {
        if (-not $coreSource.Contains($needle)) { throw "missing Core 02.23 invariant: $needle" }
    }
    $agentSource = Get-Content 'apps\agent\src\main.rs' -Raw
    foreach ($needle in @('Some("dns-server")','"network.dns-plan"','"network.dns-start"','"network.dns-status"','"network.dns-stop"')) {
        if (-not $agentSource.Contains($needle)) { throw "missing Agent 02.23 invariant: $needle" }
    }
    $cliSource = Get-Content 'apps\cli\src\main.rs' -Raw
    foreach ($needle in @('cmd == "dns" && sub == "plan"','cmd == "dns" && sub == "start"','cmd == "dns" && sub == "status"','cmd == "dns" && sub == "stop"')) {
        if (-not $cliSource.Contains($needle)) { throw "missing CLI 02.23 invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-network --package vsn-core --package vsn-ipc --all-targets -- -D warnings
    Assert-LastExit '02.23 network/core/ipc clippy failed'
    cargo test --locked --package vsn-network --package vsn-core --package vsn-ipc
    Assert-LastExit '02.23 network/core/ipc tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit '02.23 Agent/CLI release build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'
    $script:Cli = Join-Path $bin 'vsn.exe'

    $probeSource = Join-Path $sandbox 'dns_probe.rs'
    @'
use std::{env, net::UdpSocket, process, time::{Duration, Instant}};

fn query(name: &str, qtype: u16) -> Vec<u8> {
    let mut q = vec![0x12,0x34,0x01,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x00];
    for label in name.split('.') {
        q.push(label.len() as u8);
        q.extend_from_slice(label.as_bytes());
    }
    q.push(0);
    q.extend_from_slice(&qtype.to_be_bytes());
    q.extend_from_slice(&1u16.to_be_bytes());
    q
}

fn qd2() -> Vec<u8> {
    let mut q = query("demo.test", 1);
    q[4] = 0;
    q[5] = 2;
    q
}

fn compressed() -> Vec<u8> {
    let mut q = vec![0x22,0x33,0x01,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x00];
    q.extend_from_slice(&[0xC0, 0x0C]);
    q.extend_from_slice(&1u16.to_be_bytes());
    q.extend_from_slice(&1u16.to_be_bytes());
    q
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 { eprintln!("usage"); process::exit(64); }
    let mode = &args[1];
    let addr = &args[2];
    let packet = match mode.as_str() {
        "query" => {
            if args.len() != 5 { eprintln!("query usage"); process::exit(64); }
            let qtype: u16 = args[4].parse().unwrap_or_else(|_| { eprintln!("qtype"); process::exit(64) });
            query(&args[3], qtype)
        }
        "qd2" => qd2(),
        "compressed" => compressed(),
        _ => { eprintln!("unknown mode"); process::exit(64); }
    };
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_millis(150))).unwrap();
    let started = Instant::now();
    socket.send_to(&packet, addr).unwrap();
    let mut buf = [0u8;4096];
    let (n, _) = match socket.recv_from(&mut buf) {
        Ok(value) => value,
        Err(error) if matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionRefused) => {
            eprintln!("no-response");
            process::exit(2);
        }
        Err(error) => { eprintln!("recv={error}"); process::exit(3); }
    };
    if n < 12 { eprintln!("short response"); process::exit(4); }
    let flags = u16::from_be_bytes([buf[2],buf[3]]);
    let rcode = flags & 0x0f;
    let answers = u16::from_be_bytes([buf[6],buf[7]]);
    let qtype = if mode == "query" { args[4].parse::<u16>().unwrap() } else { 0 };
    let address = if qtype == 1 && answers == 1 && n >= 4 {
        format!("{}.{}.{}.{}",buf[n-4],buf[n-3],buf[n-2],buf[n-1])
    } else if qtype == 28 && answers == 1 && n >= 16 {
        let mut octets=[0u8;16];
        octets.copy_from_slice(&buf[n-16..n]);
        std::net::Ipv6Addr::from(octets).to_string()
    } else { String::new() };
    println!("{{\"rcode\":{rcode},\"answers\":{answers},\"address\":\"{address}\",\"bytes\":{n},\"elapsed_ms\":{}}}", started.elapsed().as_millis());
}
'@ | Set-Content -LiteralPath $probeSource -Encoding utf8
    rustc $probeSource -O -o (Join-Path $bin 'dns-probe.exe')
    Assert-LastExit '02.23 DNS probe build failed'
    $script:DnsProbe = Join-Path $bin 'dns-probe.exe'

    $dnsPort = Get-FreeUdpPort
    $listen = "127.0.0.1:$dnsPort"
    Start-Agent
    if ($script:AgentReadinessMs -ge 25000) { throw "Agent readiness exceeded budget: $($script:AgentReadinessMs)ms" }

    $plan = Invoke-CliJson @('dns','plan',$listen) 'dns-plan'
    if ([string]$plan.listen -ne $listen -or [string]$plan.suffix -ne '.test' -or [string]$plan.ipv4 -ne '127.0.0.1' -or [string]$plan.ipv6 -ne '::1') {
        throw '02.23 DNS resolver plan contract failed'
    }
    if ($plan.requires_admin_to_configure_os_resolver -ne $true) { throw '02.23 DNS plan lost privileged OS-resolver boundary' }

    $nonLoopback = Invoke-CliFailure @('dns','plan',"0.0.0.0:$dnsPort") 'dns-plan-nonloopback'
    $zeroPort = Invoke-CliFailure @('dns','plan','127.0.0.1:0') 'dns-plan-zero-port'
    if ($nonLoopback.Stderr -notmatch 'loopback') { throw 'non-loopback failure did not identify loopback boundary' }
    if ($zeroPort.Stderr -notmatch 'non-zero') { throw 'zero-port failure did not identify non-zero boundary' }

    $started = Invoke-CliJson @('dns','start',$listen) 'dns-start'
    if ([string]$started.id -ne 'vsn-dns' -or $started.running -ne $true) { throw '02.23 DNS managed process did not start' }
    $trackedDnsPids.Add([int]$started.pid)
    $initialReadinessMs = Wait-DnsReady $listen 'initial'

    $statusRunning = Invoke-CliJson @('dns','status') 'dns-status-running'
    if ([string]$statusRunning.id -ne 'vsn-dns' -or $statusRunning.running -ne $true) { throw '02.23 DNS status did not report running' }

    $a = Invoke-DnsProbe 'query' $listen 'demo.test' 1 'dns-a'
    if ([int]$a.rcode -ne 0 -or [int]$a.answers -ne 1 -or [string]$a.address -ne '127.0.0.1') { throw '02.23 A query did not return IPv4 loopback' }
    if ([int64]$a.elapsed_ms -ge 1000) { throw "02.23 A response latency exceeded 1000ms: $($a.elapsed_ms)ms" }

    $aaaa = Invoke-DnsProbe 'query' $listen 'api.demo.test' 28 'dns-aaaa'
    if ([int]$aaaa.rcode -ne 0 -or [int]$aaaa.answers -ne 1 -or [string]$aaaa.address -ne '::1') { throw '02.23 AAAA query did not return IPv6 loopback' }
    if ([int64]$aaaa.elapsed_ms -ge 1000) { throw "02.23 AAAA response latency exceeded 1000ms: $($aaaa.elapsed_ms)ms" }

    $external = Invoke-DnsProbe 'query' $listen 'example.com' 1 'dns-external'
    if ([int]$external.rcode -ne 5 -or [int]$external.answers -ne 0) { throw '02.23 non-.test name was not REFUSED' }
    if ([int64]$external.elapsed_ms -ge 1000) { throw "02.23 external refusal latency exceeded 1000ms: $($external.elapsed_ms)ms" }

    Test-DnsNoResponse 'qd2' $listen 'dns-invalid-two-questions' | Out-Null
    Test-DnsNoResponse 'compressed' $listen 'dns-invalid-compressed-name' | Out-Null

    $stopped = Invoke-CliJson @('dns','stop') 'dns-stop'
    if ($stopped.running -ne $false) { throw '02.23 DNS stop did not transition responder to stopped' }
    foreach ($i in 1..20) {
        if (Test-ProcessStopped ([int]$started.pid)) { break }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Test-ProcessStopped ([int]$started.pid))) { throw '02.23 initial DNS child remained alive after stop' }
    $statusStopped = Invoke-CliJson @('dns','status') 'dns-status-stopped'
    if ($statusStopped.running -ne $false) { throw '02.23 DNS status remained running after stop' }
    Test-DnsNoResponse 'query' $listen 'dns-post-stop' | Out-Null

    $restarted = Invoke-CliJson @('dns','start',$listen) 'dns-restart'
    if ([string]$restarted.id -ne 'vsn-dns' -or $restarted.running -ne $true) { throw '02.23 DNS responder did not restart' }
    $trackedDnsPids.Add([int]$restarted.pid)
    $restartReadinessMs = Wait-DnsReady $listen 'restart'
    $restartProbe = Invoke-DnsProbe 'query' $listen 'restarted.test' 1 'dns-restart-a'
    if ([int]$restartProbe.rcode -ne 0 -or [string]$restartProbe.address -ne '127.0.0.1') { throw '02.23 restarted DNS responder returned wrong answer' }
    $finalStop = Invoke-CliJson @('dns','stop') 'dns-final-stop'
    if ($finalStop.running -ne $false) { throw '02.23 final DNS stop failed' }
    foreach ($i in 1..20) {
        if (Test-ProcessStopped ([int]$restarted.pid)) { break }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Test-ProcessStopped ([int]$restarted.pid))) { throw '02.23 restarted DNS child remained alive after final stop' }

    $occupiedSocket = [Net.Sockets.UdpClient]::new()
    $occupiedSocket.Client.Bind([Net.IPEndPoint]::new([Net.IPAddress]::Loopback, 0))
    $occupiedPort = ([Net.IPEndPoint]$occupiedSocket.Client.LocalEndPoint).Port
    $occupiedListen = "127.0.0.1:$occupiedPort"
    $occupiedStdout = Join-Path $script:Root 'dns-occupied-start.stdout.json'
    $occupiedStderr = Join-Path $script:Root 'dns-occupied-start.stderr.log'
    $occupiedWatch = [Diagnostics.Stopwatch]::StartNew()
    & $script:Cli dns start $occupiedListen 1> $occupiedStdout 2> $occupiedStderr
    $occupiedExit = $LASTEXITCODE
    $occupiedExit | Set-Content -LiteralPath (Join-Path $script:Root 'dns-occupied-start.exit-code.txt')
    $occupiedFailClosed = $false
    if ($occupiedExit -ne 0) {
        $occupiedFailClosed = $true
    } else {
        $occupiedStart = Get-Content -LiteralPath $occupiedStdout -Raw | ConvertFrom-Json
        if ($null -ne $occupiedStart.pid) { $trackedDnsPids.Add([int]$occupiedStart.pid) }
        foreach ($i in 1..25) {
            $state = Invoke-CliJson @('dns','status') "dns-occupied-status-$i"
            if ($state.running -eq $false) {
                $occupiedFailClosed = $true
                break
            }
            Start-Sleep -Milliseconds 100
        }
    }
    $occupiedWatch.Stop()
    if (-not $occupiedFailClosed) { throw '02.23 occupied UDP port was represented as a healthy responder beyond the bounded window' }
    if ($occupiedWatch.ElapsedMilliseconds -ge 5000) { throw "02.23 occupied-port fail-closed detection exceeded 5 seconds: $($occupiedWatch.ElapsedMilliseconds)ms" }
    try { & $script:Cli dns stop *> $null } catch {}
    $occupiedSocket.Dispose()
    $occupiedSocket = $null
    Start-Sleep -Milliseconds 200
    foreach ($dnsProcessId in $trackedDnsPids) {
        if (-not (Test-ProcessStopped $dnsProcessId)) { throw "02.23 DNS child PID $dnsProcessId remained alive after lifecycle/occupied-port checks" }
    }
    if (-not (Test-UdpPortFree $occupiedPort)) { throw '02.23 occupied-port test left a UDP listener after holder release' }
    if (-not (Test-UdpPortFree $dnsPort)) { throw '02.23 responder UDP port remained occupied after final stop' }

    $chain = Invoke-CliJson @('audit','verify') 'audit'
    if ($chain.valid -ne $true) { throw '02.23 audit chain invalid' }
    $auditEvents = [uint64]$chain.events
    if ($auditEvents -eq 0) { throw '02.23 audit evidence reported zero events' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    if ([string]$candidate.candidate_id -ne $CandidateId -or [string]$candidate.product_version -ne $ProductVersion) { throw '02.23 release candidate/product drift detected' }

    $evidence = [ordered]@{
        schema_version = 1
        feature_id = $FeatureId
        feature_version = $FeatureVersion
        package_id = 'PKG-02'
        task_id = '02.23'
        artifact = 'test-dns-responder-lifecycle-windows-github-hosted'
        canonical_base_sha = $CanonicalBaseSha
        plan_path = '.ai/plans/pkg02-0223-test-dns-v1.md'
        plan_sha256 = $PlanSha256
        source_commit = $env:EXPECTED_SHA
        product_version = [string]$candidate.product_version
        candidate_id = [string]$candidate.candidate_id
        runner_name = $env:RUNNER_NAME
        runner_environment = $env:RUNNER_ENVIRONMENT
        runner_os = $env:RUNNER_OS
        runner_arch = $env:RUNNER_ARCH
        ipc_address = "127.0.0.1:$AgentIpcPort"
        listen = $listen
        checks = [ordered]@{
            exact_source = $true
            github_hosted_windows = $true
            frozen_plan_digest = $true
            lifecycle_artifact_digests = $true
            lifecycle_manifest = $true
            required_tests = $true
            plan_contract = $true
            loopback_listener_boundary = $true
            lifecycle_start_status_stop_restart = $true
            ipv4_loopback_answer = $true
            ipv6_loopback_answer = $true
            external_name_refused = $true
            parser_fail_closed = $true
            occupied_port_fail_closed = $true
            privileged_resolver_untouched = $true
            audit_chain_valid = $true
        }
        measurements = [ordered]@{
            agent_readiness_ms = $script:AgentReadinessMs
            dns_port = $dnsPort
            initial_readiness_ms = $initialReadinessMs
            a_response_ms = [int64]$a.elapsed_ms
            a_response_bytes = [int64]$a.bytes
            aaaa_response_ms = [int64]$aaaa.elapsed_ms
            aaaa_response_bytes = [int64]$aaaa.bytes
            external_refusal_ms = [int64]$external.elapsed_ms
            external_refusal_bytes = [int64]$external.bytes
            restart_readiness_ms = $restartReadinessMs
            occupied_port = $occupiedPort
            occupied_fail_closed_ms = [int64]$occupiedWatch.ElapsedMilliseconds
            tracked_dns_pids = $trackedDnsPids.Count
            audit_events = $auditEvents
        }
    }
    $evidence | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $script:Root 'evidence.json') -Encoding utf8
    (Get-FileHash -LiteralPath (Join-Path $script:Root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() |
        Set-Content -LiteralPath (Join-Path $script:Root 'evidence.json.sha256')
    $success = $true
}
finally {
    Stop-DnsBestEffort
    if ($null -ne $occupiedSocket) {
        try { $occupiedSocket.Dispose() } catch {}
        $occupiedSocket = $null
    }
    Stop-ProcessSafe $script:Agent
    $env:LOCALAPPDATA = $originalLocalAppData

    $ipcRestored = $false
    if ($hadIpcKey) {
        try {
            $needsRestore = -not (Test-Path -LiteralPath $ipcKey)
            if (-not $needsRestore) {
                $currentIpcHash = (Get-FileHash -LiteralPath $ipcKey -Algorithm SHA256).Hash.ToLowerInvariant()
                $needsRestore = $currentIpcHash -ne $originalIpcHash
            }
            if ($needsRestore -and $null -ne $originalIpcBytes) {
                $parent = Split-Path -Parent $ipcKey
                New-Item -ItemType Directory -Force -Path $parent | Out-Null
                [IO.File]::WriteAllBytes($ipcKey, $originalIpcBytes)
            }
            $ipcRestored = (Test-Path -LiteralPath $ipcKey) -and ((Get-FileHash -LiteralPath $ipcKey -Algorithm SHA256).Hash.ToLowerInvariant() -eq $originalIpcHash)
        } catch { $ipcRestored = $false }
    } else {
        try {
            if (Test-Path -LiteralPath $ipcKey) { Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue }
            $ipcRestored = -not (Test-Path -LiteralPath $ipcKey)
        } catch { $ipcRestored = $false }
    }

    foreach ($dnsProcessId in $trackedDnsPids) {
        if (-not (Test-ProcessStopped $dnsProcessId)) {
            try { Stop-Process -Id $dnsProcessId -Force -ErrorAction SilentlyContinue } catch {}
        }
    }
    Start-Sleep -Milliseconds 150

    if (Test-Path -LiteralPath $sandbox) {
        Remove-Item -LiteralPath $sandbox -Recurse -Force -ErrorAction SilentlyContinue
    }

    $agentStopped = ($null -eq $script:Agent) -or $script:Agent.HasExited
    $dnsProcessesStopped = $true
    foreach ($dnsProcessId in $trackedDnsPids) {
        if (-not (Test-ProcessStopped $dnsProcessId)) { $dnsProcessesStopped = $false }
    }
    $localAppDataRestored = ($env:LOCALAPPDATA -eq $originalLocalAppData)
    $sandboxRemoved = -not (Test-Path -LiteralPath $sandbox)
    $ipcPortReleased = -not [bool](Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue)
    $dnsPortReleased = Test-UdpPortFree $dnsPort
    $occupiedPortReleased = Test-UdpPortFree $occupiedPort

    $cleanup = [ordered]@{
        certification_completed = $success
        agent_stopped = $agentStopped
        dns_processes_stopped = $dnsProcessesStopped
        localappdata_restored = $localAppDataRestored
        ipc_key_restored = $ipcRestored
        ipc_port_released = $ipcPortReleased
        dns_udp_port_released = $dnsPortReleased
        occupied_udp_port_released = $occupiedPortReleased
        sandbox_removed = $sandboxRemoved
    }
    $cleanup | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $script:Root 'cleanup.json') -Encoding utf8

    if ($success -and (-not $agentStopped -or -not $dnsProcessesStopped -or -not $localAppDataRestored -or -not $ipcRestored -or -not $ipcPortReleased -or -not $dnsPortReleased -or -not $occupiedPortReleased -or -not $sandboxRemoved)) {
        throw '02.23 cleanup verification failed'
    }
}
