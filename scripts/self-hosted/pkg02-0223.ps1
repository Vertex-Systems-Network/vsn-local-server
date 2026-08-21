param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}

$root = Join-Path $PWD 'dist-self-hosted\02.23'
$bin = Join-Path $root 'bin'
$sandbox = Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0223-' + [guid]::NewGuid().ToString('N'))
$isolatedLocalAppData = Join-Path $sandbox 'localappdata'
$originalLocalAppData = $env:LOCALAPPDATA
$ipcKey = Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadIpcKey = Test-Path -LiteralPath $ipcKey
$agent = $null

New-Item -ItemType Directory -Force -Path $root,$bin,$isolatedLocalAppData | Out-Null
$env:LOCALAPPDATA = $isolatedLocalAppData

function Start-Agent {
    $script:agent = Start-Process -FilePath $script:AgentExe -RedirectStandardOutput (Join-Path $root 'agent.stdout.log') -RedirectStandardError (Join-Path $root 'agent.stderr.log') -PassThru -WindowStyle Hidden
    foreach ($i in 1..80) {
        & $script:Cli ping *> $null
        if ($LASTEXITCODE -eq 0) { return }
        if ($script:agent.HasExited) { throw "Agent exited before readiness with code $($script:agent.ExitCode)" }
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent did not become ready'
}

function Stop-ProcessSafe($Process) {
    if ($Process -and -not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue
        Wait-Process -Id $Process.Id -Timeout 10 -ErrorAction SilentlyContinue
    }
}

try {
    if (-not $IsWindows) { throw '02.23 certification requires Windows' }
    if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '02.23 certification requires a GitHub-hosted runner' }
    Write-Host "runner=$env:RUNNER_NAME environment=$env:RUNNER_ENVIRONMENT os=$env:RUNNER_OS arch=$env:RUNNER_ARCH"
    if ((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b') { throw 'rustc 1.97.1 required' }
    if ((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b') { throw 'cargo 1.97.1 required' }

    $network = Get-Content 'crates/vsn-network/src/lib.rs' -Raw
    foreach ($needle in @('pub fn dns_resolver_plan','pub fn run_dns_server','DNS responder suffix must remain .test','DNS listener must bind to loopback','build_dns_response','127, 0, 0, 1','Ipv6Addr::LOCALHOST','external_domain_is_refused')) {
        if (-not $network.Contains($needle)) { throw "missing DNS invariant: $needle" }
    }
    $core = Get-Content 'crates/vsn-core/src/lib.rs' -Raw
    foreach ($needle in @('pub fn dns_start','pub fn dns_status','pub fn dns_stop','id: "vsn-dns"')) {
        if (-not $core.Contains($needle)) { throw "missing Core DNS lifecycle invariant: $needle" }
    }

    cargo fmt --all -- --check
    Assert-LastExit 'cargo fmt failed'
    cargo clippy --locked --package vsn-network --package vsn-core --all-targets -- -D warnings
    Assert-LastExit 'DNS clippy failed'
    cargo test --locked --package vsn-network --package vsn-core
    Assert-LastExit 'DNS tests failed'
    git diff --check
    Assert-LastExit 'git diff --check failed'

    cargo build --locked --release --package vsn-agent --package vsn
    Assert-LastExit 'Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:Cli = Join-Path $bin 'vsn.exe'
    $script:AgentExe = Join-Path $bin 'vsn-agent.exe'

    $probeSource = Join-Path $sandbox 'dns_probe.rs'
    @'
use std::{env, net::UdpSocket, time::Duration};
fn query(name: &str, qtype: u16) -> Vec<u8> {
    let mut q = vec![0x12,0x34,0x01,0x00,0x00,0x01,0x00,0x00,0x00,0x00,0x00,0x00];
    for label in name.split('.') { q.push(label.len() as u8); q.extend_from_slice(label.as_bytes()); }
    q.push(0); q.extend_from_slice(&qtype.to_be_bytes()); q.extend_from_slice(&1u16.to_be_bytes()); q
}
fn main() {
    let addr = env::args().nth(1).expect("addr");
    let name = env::args().nth(2).expect("name");
    let qtype: u16 = env::args().nth(3).expect("qtype").parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket.set_read_timeout(Some(Duration::from_millis(1200))).unwrap();
    socket.send_to(&query(&name, qtype), &addr).unwrap();
    let mut buf=[0u8;4096];
    let (n,_) = socket.recv_from(&mut buf).expect("dns response timeout");
    if n < 12 { panic!("short response"); }
    let flags=u16::from_be_bytes([buf[2],buf[3]]); let rcode=flags&0x0f;
    let answers=u16::from_be_bytes([buf[6],buf[7]]);
    let tail = if qtype==1 && answers==1 && n>=4 { format!("{}.{}.{}.{}",buf[n-4],buf[n-3],buf[n-2],buf[n-1]) }
        else if qtype==28 && answers==1 && n>=16 {
            let mut oct=[0u8;16]; oct.copy_from_slice(&buf[n-16..n]); std::net::Ipv6Addr::from(oct).to_string()
        } else { String::new() };
    println!("{{\"rcode\":{rcode},\"answers\":{answers},\"address\":\"{tail}\"}}");
}
'@ | Set-Content -LiteralPath $probeSource -Encoding utf8
    rustc $probeSource -O -o (Join-Path $bin 'dns-probe.exe')
    Assert-LastExit 'DNS probe build failed'
    $probe = Join-Path $bin 'dns-probe.exe'

    $udp = [Net.Sockets.UdpClient]::new(0)
    $port = ([Net.IPEndPoint]$udp.Client.LocalEndPoint).Port
    $udp.Dispose()
    $listen = "127.0.0.1:$port"

    if (Get-NetTCPConnection -LocalPort 49731 -State Listen -ErrorAction SilentlyContinue) { throw 'TCP 49731 is already in use; refusing to disturb another Agent' }
    Start-Agent

    $plan = & $script:Cli dns plan $listen | Out-String | ConvertFrom-Json
    $plan | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'dns-plan.json') -Encoding utf8
    if ([string]$plan.listen -ne $listen -or [string]$plan.suffix -ne '.test' -or [string]$plan.ipv4 -ne '127.0.0.1' -or [string]$plan.ipv6 -ne '::1') { throw 'DNS resolver plan contract failed' }
    if ($plan.requires_admin_to_configure_os_resolver -ne $true) { throw 'DNS plan did not preserve privileged OS-resolver boundary' }

    $started = & $script:Cli dns start $listen | Out-String | ConvertFrom-Json
    $started | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'dns-start.json') -Encoding utf8
    if ([string]$started.id -ne 'vsn-dns' -or $started.running -ne $true) { throw 'DNS managed process did not start' }

    $ready = $false
    foreach ($i in 1..30) {
        & $probe $listen demo.test 1 1> (Join-Path $root 'probe-ready.json') 2> $null
        if ($LASTEXITCODE -eq 0) { $ready=$true; break }
        Start-Sleep -Milliseconds 150
    }
    if (-not $ready) { throw 'DNS responder did not become ready' }

    $a = & $probe $listen demo.test 1 | Out-String | ConvertFrom-Json
    $a | ConvertTo-Json -Compress | Set-Content (Join-Path $root 'dns-a.json')
    if ([int]$a.rcode -ne 0 -or [int]$a.answers -ne 1 -or [string]$a.address -ne '127.0.0.1') { throw 'A query did not return IPv4 loopback' }

    $aaaa = & $probe $listen api.demo.test 28 | Out-String | ConvertFrom-Json
    $aaaa | ConvertTo-Json -Compress | Set-Content (Join-Path $root 'dns-aaaa.json')
    if ([int]$aaaa.rcode -ne 0 -or [int]$aaaa.answers -ne 1 -or [string]$aaaa.address -ne '::1') { throw 'AAAA query did not return IPv6 loopback' }

    $external = & $probe $listen example.com 1 | Out-String | ConvertFrom-Json
    $external | ConvertTo-Json -Compress | Set-Content (Join-Path $root 'dns-external.json')
    if ([int]$external.rcode -ne 5 -or [int]$external.answers -ne 0) { throw 'non-.test name was not REFUSED' }

    $status = & $script:Cli dns status | Out-String | ConvertFrom-Json
    $status | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'dns-status-running.json')
    if ([string]$status.id -ne 'vsn-dns' -or $status.running -ne $true) { throw 'DNS status did not report running responder' }

    $stopped = & $script:Cli dns stop | Out-String | ConvertFrom-Json
    $stopped | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'dns-stop.json')
    if ($stopped.running -ne $false) { throw 'DNS stop did not transition responder to stopped' }
    Start-Sleep -Milliseconds 300
    & $probe $listen demo.test 1 1> (Join-Path $root 'post-stop.stdout') 2> (Join-Path $root 'post-stop.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'DNS responder still answered after stop' }

    $restarted = & $script:Cli dns start $listen | Out-String | ConvertFrom-Json
    if ($restarted.running -ne $true) { throw 'DNS responder could not restart cleanly' }
    foreach ($i in 1..30) {
        & $probe $listen restarted.test 1 *> $null
        if ($LASTEXITCODE -eq 0) { break }
        Start-Sleep -Milliseconds 150
    }
    if ($LASTEXITCODE -ne 0) { throw 'restarted DNS responder did not answer' }
    & $script:Cli dns stop *> $null
    Assert-LastExit 'final DNS stop failed'

    & $script:Cli dns plan '0.0.0.0:53535' 1> (Join-Path $root 'nonloopback.stdout') 2> (Join-Path $root 'nonloopback.stderr')
    if ($LASTEXITCODE -eq 0) { throw 'non-loopback DNS listener unexpectedly accepted' }

    $chain = & $script:Cli audit verify | Out-String | ConvertFrom-Json
    $chain | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $root 'audit.json') -Encoding utf8
    if ($chain.valid -ne $true) { throw 'audit chain invalid' }

    $candidate = Get-Content 'docs\release-candidate-current.json' -Raw | ConvertFrom-Json
    [ordered]@{
        schema_version=1; package_id='PKG-02'; task_id='02.23';
        artifact='test-dns-responder-lifecycle-windows-github-hosted';
        product_version=$candidate.product_version; candidate_id=$candidate.candidate_id;
        source_commit=$env:GITHUB_SHA; runner_name=$env:RUNNER_NAME; runner_environment=$env:RUNNER_ENVIRONMENT;
        plan_verified=$true; lifecycle_verified=$true; ipv4_loopback_answer_verified=$true; ipv6_loopback_answer_verified=$true;
        external_name_refusal_verified=$true; loopback_listener_boundary_verified=$true; restart_verified=$true; audit_chain_valid=$true
    } | ConvertTo-Json -Depth 8 | Set-Content (Join-Path $root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant() | Set-Content (Join-Path $root 'evidence.json.sha256')
}
finally {
    try { & $script:Cli dns stop *> $null } catch {}
    Stop-ProcessSafe $agent
    $env:LOCALAPPDATA = $originalLocalAppData
    if (-not $hadIpcKey -and (Test-Path -LiteralPath $ipcKey)) {
        Remove-Item -LiteralPath $ipcKey -Force -ErrorAction SilentlyContinue
    }
}
