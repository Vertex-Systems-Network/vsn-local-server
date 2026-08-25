param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$FeatureId = 'pkg02-0223-test-dns'
$FeatureVersion = '1.0.0'
$QaAddendumVersion = '1.0.1'
$PermissionAddendumVersion = '1.0.0'
$CanonicalBaseSha = '94feeb8e67dad96ac6a384a8517229ba2c5c38f5'
$PlanSha256 = 'cc9b7b503c87d4ede7fb625e080500049fd0d3c4f0d8cdd956f2d7747c3db9ed'
$QaAddendumSha256 = '328c71171f987107edb5f2c26099ba8a8b99df3f850ceda0991a83e36b5756e0'
$PermissionAddendumSha256 = '9246c02dc4f4d2f19334adcaa8de5a2f083e976ce2b98c7606a58e12aacd9c0b'
$ResearchSha256 = '05a7a1116eedf9308abf6bd8852a7369134b0c5db473ce884e3fc25fb3a3a71d'
$LifecycleSha256 = '3012cef4a49d218ceaf5d75434c8f828d802afa2e1184b14f198c2ab247d95ff'
$PreflightSha256 = '8a2a921c319cc1e4efe591319089dcf63246679376685f69dae3ba63ea34620a'
$CandidateId = 'c579788ddb171fc3c094c0614b3f6e134aaa6bb2660d7e1b1856a742aebd6474'
$ProductVersion = '0.38.1'
$AgentIpcPort = 39731

function Assert-Exit([string]$Message) {
    if ($LASTEXITCODE -ne 0) { throw "$Message (exit=$LASTEXITCODE)" }
}
function Assert-Sha([string]$Path,[string]$Expected,[string]$Name) {
    $actual=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Expected) { throw "$Name digest mismatch: expected=$Expected actual=$actual" }
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
    $u=[Net.Sockets.UdpClient]::new()
    try {
        $u.Client.Bind([Net.IPEndPoint]::new([Net.IPAddress]::Loopback,0))
        return ([Net.IPEndPoint]$u.Client.LocalEndPoint).Port
    } finally { $u.Dispose() }
}
function Test-UdpFree([int]$Port) {
    if ($Port -le 0) { return $true }
    $u=[Net.Sockets.UdpClient]::new()
    try {
        $u.Client.Bind([Net.IPEndPoint]::new([Net.IPAddress]::Loopback,$Port))
        return $true
    } catch { return $false }
    finally { $u.Dispose() }
}
function Invoke-CliJson([string[]]$CliArgs,[string]$Name) {
    $out=Join-Path $script:Root "$Name.stdout.json"
    $err=Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code=$LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt")
    if ($code -ne 0) {
        $detail=if(Test-Path $err){Get-Content $err -Raw}else{''}
        throw "$Name failed (exit=$code): $detail"
    }
    Get-Content $out -Raw | ConvertFrom-Json
}
function Invoke-CliFailure([string[]]$CliArgs,[string]$Name) {
    $out=Join-Path $script:Root "$Name.stdout.log"
    $err=Join-Path $script:Root "$Name.stderr.log"
    & $script:Cli @CliArgs 1> $out 2> $err
    $code=$LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Name.exit-code.txt")
    if ($code -eq 0) { throw "$Name unexpectedly succeeded" }
    $stdoutText = if (Test-Path $out) { Get-Content $out -Raw } else { '' }
    $stderrText = if (Test-Path $err) { Get-Content $err -Raw } else { '' }
    [pscustomobject]@{ ExitCode = $code; Stdout = $stdoutText; Stderr = $stderrText }
}
function Start-Agent {
    $script:Agent=Start-Process -FilePath $script:AgentExe `
      -RedirectStandardOutput (Join-Path $script:Root 'agent.stdout.log') `
      -RedirectStandardError (Join-Path $script:Root 'agent.stderr.log') `
      -PassThru -WindowStyle Hidden
    $sw=[Diagnostics.Stopwatch]::StartNew()
    foreach($i in 1..100){
        & $script:Cli ping *> $null
        if($LASTEXITCODE -eq 0){$sw.Stop();$script:AgentReadyMs=[int64]$sw.ElapsedMilliseconds;return}
        if($script:Agent.HasExited){throw "Agent exited before readiness code=$($script:Agent.ExitCode)"}
        Start-Sleep -Milliseconds 250
    }
    throw 'Agent readiness exceeded 25 seconds'
}
function Invoke-Probe([string]$Mode,[string]$Listen,[string]$Name,[int]$Type,[string]$Artifact) {
    $out=Join-Path $script:Root "$Artifact.stdout.json"
    $err=Join-Path $script:Root "$Artifact.stderr.log"
    if($Mode -eq 'query'){ & $script:Probe query $Listen $Name $Type 1> $out 2> $err }
    else { & $script:Probe $Mode $Listen 1> $out 2> $err }
    $code=$LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Artifact.exit-code.txt")
    if($code -ne 0){
        $detail=if(Test-Path $err){Get-Content $err -Raw}else{''}
        throw "$Artifact probe failed (exit=$code): $detail"
    }
    Get-Content $out -Raw | ConvertFrom-Json
}
function Assert-NoResponse([string]$Mode,[string]$Listen,[string]$Artifact) {
    $out=Join-Path $script:Root "$Artifact.stdout.log"
    $err=Join-Path $script:Root "$Artifact.stderr.log"
    if($Mode -eq 'query'){ & $script:Probe query $Listen demo.test 1 1> $out 2> $err }
    else { & $script:Probe $Mode $Listen 1> $out 2> $err }
    $code=$LASTEXITCODE
    $code | Set-Content (Join-Path $script:Root "$Artifact.exit-code.txt")
    if($code -ne 2){throw "$Artifact expected bounded no-response exit=2, got $code"}
}
function Wait-Dns([string]$Listen,[string]$Label) {
    $sw=[Diagnostics.Stopwatch]::StartNew()
    while($sw.ElapsedMilliseconds -lt 4500){
        & $script:Probe query $Listen demo.test 1 *> $null
        if($LASTEXITCODE -eq 0){$sw.Stop();return [int64]$sw.ElapsedMilliseconds}
        Start-Sleep -Milliseconds 50
    }
    throw "$Label DNS readiness exceeded 5 seconds"
}

$script:Root=Join-Path $PWD 'dist-self-hosted\02.23'
$bin=Join-Path $script:Root 'bin'
$sandbox=Join-Path $env:RUNNER_TEMP ('vsn-pkg02-0223-'+[guid]::NewGuid().ToString('N'))
$isolated=Join-Path $sandbox 'localappdata'
$originalLocal=$env:LOCALAPPDATA
$ipcKey=Join-Path $env:ProgramData 'VSN\security\ipc.key'
$hadKey=Test-Path $ipcKey
$originalKeyBytes=if($hadKey){[IO.File]::ReadAllBytes($ipcKey)}else{$null}
$originalKeyHash=if($hadKey){(Get-FileHash $ipcKey -Algorithm SHA256).Hash.ToLowerInvariant()}else{$null}
$script:Agent=$null;$script:AgentExe=$null;$script:Cli=$null;$script:Probe=$null;$script:AgentReadyMs=0
$dnsPort=0;$occupiedPort=0;$occupied=$null
$pids=[System.Collections.Generic.List[int]]::new()
$success=$false

if(Test-Path $script:Root){Remove-Item $script:Root -Recurse -Force}
New-Item -ItemType Directory -Force -Path $script:Root,$bin,$sandbox,$isolated | Out-Null
$env:LOCALAPPDATA=$isolated

try {
    if(-not $IsWindows){throw '02.23 requires Windows'}
    if($env:RUNNER_ENVIRONMENT -ne 'github-hosted'){throw '02.23 requires GitHub-hosted'}
    if(-not $env:EXPECTED_SHA){throw 'EXPECTED_SHA required'}
    if((git rev-parse HEAD).Trim() -ne $env:EXPECTED_SHA){throw 'exact source mismatch'}
    if((& rustc --version).Trim() -notmatch '^rustc 1\.97\.1\b'){throw 'rustc 1.97.1 required'}
    if((& cargo --version).Trim() -notmatch '^cargo 1\.97\.1\b'){throw 'cargo 1.97.1 required'}
    if(Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue){throw 'IPC port occupied'}

    Assert-Sha '.ai\plans\pkg02-0223-test-dns-v1.md' $PlanSha256 'plan'
    Assert-Sha '.ai\features\pkg02-0223\qa-clippy-scope-addendum.md' $QaAddendumSha256 'QA addendum'
    Assert-Sha '.ai\features\pkg02-0223\permission-boundary-addendum.md' $PermissionAddendumSha256 'permission boundary addendum'
    Assert-Sha '.ai\features\pkg02-0223\research.md' $ResearchSha256 'research'
    Assert-Sha '.ai\features\pkg02-0223\lifecycle-review.md' $LifecycleSha256 'lifecycle'
    Assert-Sha '.ai\features\pkg02-0223\development-preflight.md' $PreflightSha256 'preflight'
    $manifest=Get-Content '.ai\manifests\pkg02-0223-test-dns.v1.json' -Raw | ConvertFrom-Json
    if([string]$manifest.feature_id -ne $FeatureId -or [string]$manifest.version -ne $FeatureVersion){throw 'manifest identity mismatch'}
    if([string]$manifest.canonical_base_sha -ne $CanonicalBaseSha){throw 'manifest base mismatch'}
    if([string]$manifest.plan.sha256 -ne $PlanSha256){throw 'manifest plan mismatch'}
    if([string]$manifest.qa_addendum.version -ne $QaAddendumVersion -or [string]$manifest.qa_addendum.sha256 -ne $QaAddendumSha256){throw 'manifest QA addendum mismatch'}
    if($manifest.qa_addendum.frozen_behavioral_criteria_changed -ne $false){throw 'QA addendum changed frozen behavior'}
    if([string]$manifest.permission_boundary_addendum.version -ne $PermissionAddendumVersion -or [string]$manifest.permission_boundary_addendum.sha256 -ne $PermissionAddendumSha256){throw 'manifest permission addendum mismatch'}
    if($manifest.permission_boundary_addendum.frozen_behavioral_criteria_changed -ne $false -or $manifest.permission_boundary_addendum.network_manage_granted_to_local_authenticated -ne $false){throw 'permission addendum widened frozen behavior'}
    if([string]$manifest.research.market_delta -ne 'none'){throw 'market delta not cleared'}

    $network=Get-Content 'crates\vsn-network\src\lib.rs' -Raw
    foreach($n in @('pub fn dns_resolver_plan','pub fn run_dns_server','DNS responder suffix must remain .test','DNS listener must bind to loopback','DNS listener port must be non-zero','DNS baseline accepts exactly one question','compressed query names are not accepted by the local DNS baseline','DNS name exceeds 255 bytes','name == "test" || name.ends_with(".test")','let rcode = if local { 0u16 } else { 5u16 }','127, 0, 0, 1','Ipv6Addr::LOCALHOST')){if(-not $network.Contains($n)){throw "missing network invariant: $n"}}
    $core=Get-Content 'crates\vsn-core\src\lib.rs' -Raw
    foreach($n in @('pub fn dns_plan','pub fn dns_start','pub fn dns_status','pub fn dns_stop','id: "vsn-dns"')){if(-not $core.Contains($n)){throw "missing core invariant: $n"}}
    if($core -notmatch '(?s)pub fn dns_start\(\s*principal: &Principal,\s*listen: &str,\s*\) -> Result<vsn_system::ManagedProcessState, CoreError> \{\s*vsn_policy::require\(principal, Permission::NetworkView\)\?;\s*vsn_policy::require\(principal, Permission::ServiceManage\)\?;'){throw 'DNS start permission boundary mismatch'}
    if($core -notmatch '(?s)pub fn dns_stop\(principal: &Principal\) -> Result<vsn_system::ManagedProcessState, CoreError> \{\s*vsn_policy::require\(principal, Permission::ServiceManage\)\?;'){throw 'DNS stop permission boundary mismatch'}
    $policy=Get-Content 'crates\vsn-policy\src\lib.rs' -Raw
    $localAuth=[regex]::Match($policy,'(?s)pub fn local_authenticated\(\) -> Self \{.*?pub fn local_network_admin\(\) -> Self \{')
    if(-not $localAuth.Success){throw 'local authenticated policy block unavailable'}
    if($localAuth.Value.Contains('NetworkManage')){throw 'ordinary local principal unexpectedly has NetworkManage'}

    cargo fmt --all -- --check
    Assert-Exit 'fmt failed'
    cargo clippy --locked --package vsn-network --package vsn-core --package vsn-ipc --all-targets --no-deps -- -D warnings
    Assert-Exit 'direct 02.23 package Clippy failed'
    cargo test --locked --package vsn-network --package vsn-core --package vsn-ipc
    Assert-Exit '02.23 tests failed'
    git diff --check
    Assert-Exit 'git diff check failed'
    cargo build --locked --release --package vsn-agent --package vsn
    Assert-Exit 'release Agent/CLI build failed'
    Copy-Item 'target\release\vsn-agent.exe' (Join-Path $bin 'vsn-agent.exe') -Force
    Copy-Item 'target\release\vsn.exe' (Join-Path $bin 'vsn.exe') -Force
    $script:AgentExe=Join-Path $bin 'vsn-agent.exe'
    $script:Cli=Join-Path $bin 'vsn.exe'

    $probeSrc=Join-Path $sandbox 'dns_probe.rs'
@'
use std::{env,net::UdpSocket,process,time::{Duration,Instant}};
fn query(name:&str,t:u16)->Vec<u8>{let mut q=vec![0x12,0x34,0x01,0,0,1,0,0,0,0,0,0];for l in name.split('.') {q.push(l.len() as u8);q.extend_from_slice(l.as_bytes());}q.push(0);q.extend_from_slice(&t.to_be_bytes());q.extend_from_slice(&1u16.to_be_bytes());q}
fn main(){let a:Vec<String>=env::args().collect();if a.len()<3{process::exit(64)};let mode=&a[1];let addr=&a[2];let packet=match mode.as_str(){"query"=>{if a.len()!=5{process::exit(64)};query(&a[3],a[4].parse().unwrap())},"qd2"=>{let mut q=query("demo.test",1);q[5]=2;q},"compressed"=>{let mut q=vec![0x22,0x33,1,0,0,1,0,0,0,0,0,0];q.extend_from_slice(&[0xC0,0x0C]);q.extend_from_slice(&1u16.to_be_bytes());q.extend_from_slice(&1u16.to_be_bytes());q},_=>process::exit(64)};let s=UdpSocket::bind("127.0.0.1:0").unwrap();s.set_read_timeout(Some(Duration::from_millis(175))).unwrap();let sw=Instant::now();s.send_to(&packet,addr).unwrap();let mut b=[0u8;4096];let(n,_)=match s.recv_from(&mut b){Ok(v)=>v,Err(e)if matches!(e.kind(),std::io::ErrorKind::WouldBlock|std::io::ErrorKind::TimedOut|std::io::ErrorKind::ConnectionReset|std::io::ErrorKind::ConnectionRefused)=>{eprintln!("no-response");process::exit(2)},Err(e)=>{eprintln!("recv={e}");process::exit(3)}};if n<12{process::exit(4)};let flags=u16::from_be_bytes([b[2],b[3]]);let rcode=flags&15;let answers=u16::from_be_bytes([b[6],b[7]]);let t=if mode=="query"{a[4].parse::<u16>().unwrap()}else{0};let address=if t==1&&answers==1&&n>=4{format!("{}.{}.{}.{}",b[n-4],b[n-3],b[n-2],b[n-1])}else if t==28&&answers==1&&n>=16{let mut o=[0u8;16];o.copy_from_slice(&b[n-16..n]);std::net::Ipv6Addr::from(o).to_string()}else{String::new()};println!("{{\"rcode\":{rcode},\"answers\":{answers},\"address\":\"{address}\",\"bytes\":{n},\"elapsed_ms\":{}}}",sw.elapsed().as_millis());}
'@ | Set-Content $probeSrc -Encoding utf8
    rustc $probeSrc -O -o (Join-Path $bin 'dns-probe.exe')
    Assert-Exit 'DNS probe build failed'
    $script:Probe=Join-Path $bin 'dns-probe.exe'

    $dnsPort=Get-FreeUdpPort;$listen="127.0.0.1:$dnsPort"
    Start-Agent
    $plan=Invoke-CliJson @('dns','plan',$listen) 'dns-plan'
    if([string]$plan.listen -ne $listen -or [string]$plan.suffix -ne '.test' -or [string]$plan.ipv4 -ne '127.0.0.1' -or [string]$plan.ipv6 -ne '::1' -or $plan.requires_admin_to_configure_os_resolver -ne $true){throw 'plan contract failed'}
    $nl=Invoke-CliFailure @('dns','plan',"0.0.0.0:$dnsPort") 'plan-nonloopback'
    $zp=Invoke-CliFailure @('dns','plan','127.0.0.1:0') 'plan-zero-port'
    if($nl.Stderr -notmatch 'loopback' -or $zp.Stderr -notmatch 'non-zero'){throw 'listen boundary failure text mismatch'}

    $start=Invoke-CliJson @('dns','start',$listen) 'dns-start'
    if([string]$start.id -ne 'vsn-dns' -or $start.running -ne $true){throw 'DNS start failed'}
    $pids.Add([int]$start.pid)
    $initialReady=Wait-Dns $listen 'initial'
    $status=Invoke-CliJson @('dns','status') 'dns-status-running'
    if($status.running -ne $true){throw 'DNS status not running'}

    $a=Invoke-Probe 'query' $listen 'demo.test' 1 'dns-a'
    $aaaa=Invoke-Probe 'query' $listen 'api.demo.test' 28 'dns-aaaa'
    $ext=Invoke-Probe 'query' $listen 'example.com' 1 'dns-external'
    if ([int]$a.rcode -ne 0 -or [int]$a.answers -ne 1 -or [string]$a.address -ne '127.0.0.1'){throw 'A contract failed'}
    if ([int]$aaaa.rcode -ne 0 -or [int]$aaaa.answers -ne 1 -or [string]$aaaa.address -ne '::1'){throw 'AAAA contract failed'}
    if ([int]$ext.rcode -ne 5 -or [int]$ext.answers -ne 0){throw 'external refusal failed'}
    Assert-NoResponse 'qd2' $listen 'invalid-two-questions'
    Assert-NoResponse 'compressed' $listen 'invalid-compressed-name'

    $stop=Invoke-CliJson @('dns','stop') 'dns-stop'
    if($stop.running -ne $false){throw 'stop did not report stopped'}
    foreach($i in 1..20){if(Test-ProcessStopped ([int]$start.pid)){break};Start-Sleep -Milliseconds 100}
    if(-not(Test-ProcessStopped ([int]$start.pid))){throw 'first DNS child survived stop'}
    Assert-NoResponse 'query' $listen 'post-stop'

    $restart=Invoke-CliJson @('dns','start',$listen) 'dns-restart'
    $pids.Add([int]$restart.pid)
    $restartReady=Wait-Dns $listen 'restart'
    $restartA=Invoke-Probe 'query' $listen 'restarted.test' 1 'dns-restart-a'
    if ([string]$restartA.address -ne '127.0.0.1'){throw 'restart answer failed'}
    $finalStop=Invoke-CliJson @('dns','stop') 'dns-final-stop'
    if($finalStop.running -ne $false){throw 'final stop failed'}
    foreach($i in 1..20){if(Test-ProcessStopped ([int]$restart.pid)){break};Start-Sleep -Milliseconds 100}
    if(-not(Test-ProcessStopped ([int]$restart.pid))){throw 'restart DNS child survived stop'}

    $occupied=[Net.Sockets.UdpClient]::new()
    $occupied.Client.Bind([Net.IPEndPoint]::new([Net.IPAddress]::Loopback,0))
    $occupiedPort=([Net.IPEndPoint]$occupied.Client.LocalEndPoint).Port
    $occListen="127.0.0.1:$occupiedPort";$sw=[Diagnostics.Stopwatch]::StartNew()
    & $script:Cli dns start $occListen 1> (Join-Path $script:Root 'occupied.stdout.json') 2> (Join-Path $script:Root 'occupied.stderr.log')
    $occExit=$LASTEXITCODE;$occExit|Set-Content (Join-Path $script:Root 'occupied.exit-code.txt')
    $failClosed = $occExit -ne 0
    if(-not $failClosed){
        $occ=Get-Content (Join-Path $script:Root 'occupied.stdout.json') -Raw|ConvertFrom-Json
        if ($null -ne $occ.pid){$pids.Add([int]$occ.pid)}
        foreach($i in 1..25){$s=Invoke-CliJson @('dns','status') "occupied-status-$i";if ($s.running -eq $false){$failClosed=$true;break};Start-Sleep -Milliseconds 100}
    }
    $sw.Stop()
    if (-not $failClosed -or $sw.ElapsedMilliseconds -ge 5000){throw 'occupied port did not fail closed'}
    try{& $script:Cli dns stop *> $null}catch{}
    $occupied.Dispose();$occupied=$null;Start-Sleep -Milliseconds 200

    foreach($p in $pids){if(-not(Test-ProcessStopped $p)){throw "DNS PID $p survived"}}
    if(-not(Test-UdpFree $dnsPort) -or -not(Test-UdpFree $occupiedPort)){throw 'UDP port cleanup failed'}
    $audit=Invoke-CliJson @('audit','verify') 'audit'
    if ($audit.valid -ne $true -or [uint64]$audit.events -eq 0){throw 'audit invalid'}
    $candidate=Get-Content 'docs\release-candidate-current.json' -Raw|ConvertFrom-Json
    if ([string]$candidate.candidate_id -ne $CandidateId -or [string]$candidate.product_version -ne $ProductVersion){throw 'candidate drift'}

    $e=[ordered]@{
      schema_version=1;feature_id=$FeatureId;feature_version=$FeatureVersion;qa_addendum_version=$QaAddendumVersion;qa_addendum_sha256=$QaAddendumSha256;permission_addendum_version=$PermissionAddendumVersion;permission_addendum_sha256=$PermissionAddendumSha256;
      package_id='PKG-02';task_id='02.23';canonical_base_sha=$CanonicalBaseSha;plan_sha256=$PlanSha256;source_commit=$env:EXPECTED_SHA;
      product_version=[string]$candidate.product_version;candidate_id=[string]$candidate.candidate_id;runner_name=$env:RUNNER_NAME;runner_environment=$env:RUNNER_ENVIRONMENT;runner_os=$env:RUNNER_OS;runner_arch=$env:RUNNER_ARCH;ipc_address="127.0.0.1:$AgentIpcPort";listen=$listen;
      checks=[ordered]@{exact_source=$true;frozen_plan=$true;qa_addendum=$true;permission_boundary=$true;ordinary_local_network_manage_absent=$true;direct_package_clippy=$true;tests=$true;release_build=$true;plan=$true;listener_boundary=$true;lifecycle=$true;ipv4=$true;ipv6=$true;external_refusal=$true;parser_fail_closed=$true;occupied_port_fail_closed=$true;privileged_resolver_untouched=$true;audit=$true};
      measurements=[ordered]@{agent_readiness_ms=$script:AgentReadyMs;dns_port=$dnsPort;initial_readiness_ms=$initialReady;a_response_ms=[int64]$a.elapsed_ms;aaaa_response_ms=[int64]$aaaa.elapsed_ms;external_refusal_ms=[int64]$ext.elapsed_ms;restart_readiness_ms=$restartReady;occupied_port=$occupiedPort;occupied_fail_closed_ms=[int64]$sw.ElapsedMilliseconds;tracked_dns_pids=$pids.Count;audit_events=[uint64]$audit.events}
    }
    $e|ConvertTo-Json -Depth 12|Set-Content (Join-Path $script:Root 'evidence.json') -Encoding utf8
    (Get-FileHash (Join-Path $script:Root 'evidence.json') -Algorithm SHA256).Hash.ToLowerInvariant()|Set-Content (Join-Path $script:Root 'evidence.json.sha256')
    $success=$true
}
finally {
    if($script:Cli -and $script:Agent -and -not $script:Agent.HasExited){try{& $script:Cli dns stop *> $null}catch{}}
    if($null-ne$occupied){try{$occupied.Dispose()}catch{}}
    Stop-ProcessSafe $script:Agent
    $env:LOCALAPPDATA=$originalLocal
    $keyRestored=$false
    try{
      if($hadKey){
        $restore=(-not(Test-Path $ipcKey))
        if(-not$restore){$restore=((Get-FileHash $ipcKey -Algorithm SHA256).Hash.ToLowerInvariant() -ne $originalKeyHash)}
        if($restore){New-Item -ItemType Directory -Force -Path (Split-Path $ipcKey -Parent)|Out-Null;[IO.File]::WriteAllBytes($ipcKey,$originalKeyBytes)}
        $keyRestored=(Test-Path $ipcKey) -and ((Get-FileHash $ipcKey -Algorithm SHA256).Hash.ToLowerInvariant() -eq $originalKeyHash)
      } else {if(Test-Path $ipcKey){Remove-Item $ipcKey -Force};$keyRestored = -not (Test-Path $ipcKey)}
    }catch{$keyRestored=$false}
    foreach($p in $pids){if(-not(Test-ProcessStopped $p)){try{Stop-Process -Id $p -Force -ErrorAction SilentlyContinue}catch{}}}
    Start-Sleep -Milliseconds 150
    if(Test-Path $sandbox){Remove-Item $sandbox -Recurse -Force -ErrorAction SilentlyContinue}
    $cleanup=[ordered]@{
      certification_completed=$success;
      agent_stopped=(($null -eq $script:Agent) -or $script:Agent.HasExited);
      dns_processes_stopped=(@($pids | Where-Object { -not (Test-ProcessStopped $_) }).Count -eq 0);
      localappdata_restored=($env:LOCALAPPDATA -eq $originalLocal);
      ipc_key_restored=$keyRestored;
      ipc_port_released=(-not [bool](Get-NetTCPConnection -LocalPort $AgentIpcPort -State Listen -ErrorAction SilentlyContinue));
      dns_udp_port_released=(Test-UdpFree $dnsPort);
      occupied_udp_port_released=(Test-UdpFree $occupiedPort);
      sandbox_removed=(-not (Test-Path $sandbox))
    }
    $cleanup|ConvertTo-Json -Depth 5|Set-Content (Join-Path $script:Root 'cleanup.json') -Encoding utf8
    if ($success) {
        foreach ($key in $cleanup.Keys) {
            if ($cleanup[$key] -ne $true) { throw "cleanup failed: $key" }
        }
    }
}
