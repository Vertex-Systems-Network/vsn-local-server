use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SystemError {
    #[error("command failed: {0}")]
    Command(String),
    #[error("unsupported operation on this OS: {0}")]
    Unsupported(&'static str),
    #[error("invalid input: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortInfo {
    pub protocol: String,
    pub local_address: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceState {
    pub name: String,
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceProviderDescriptor {
    pub id: String,
    pub platform: String,
    pub actions: Vec<String>,
    pub source_scope_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceProviderConformanceReport {
    pub descriptor: ServiceProviderDescriptor,
    pub valid: bool,
    pub issues: Vec<String>,
}

pub trait ServiceProvider {
    fn descriptor(&self) -> ServiceProviderDescriptor;
    fn state(&self, name: &str) -> Result<ServiceState, SystemError>;
    fn action(&self, name: &str, action: &str) -> Result<ServiceState, SystemError>;
}

pub struct NativeServiceProvider;
impl ServiceProvider for NativeServiceProvider {
    fn descriptor(&self) -> ServiceProviderDescriptor {
        native_service_provider_descriptor()
    }
    fn state(&self, name: &str) -> Result<ServiceState, SystemError> {
        service_state(name)
    }
    fn action(&self, name: &str, action: &str) -> Result<ServiceState, SystemError> {
        service_action(name, action)
    }
}

pub fn native_service_provider_descriptor() -> ServiceProviderDescriptor {
    ServiceProviderDescriptor {
        id: "native-os-service".into(),
        platform: std::env::consts::OS.into(),
        actions: vec![
            "state".into(),
            "start".into(),
            "stop".into(),
            "restart".into(),
        ],
        source_scope_complete: cfg!(any(windows, target_os = "linux", target_os = "macos")),
    }
}

pub fn service_provider_conformance() -> ServiceProviderConformanceReport {
    let descriptor = native_service_provider_descriptor();
    let mut issues = Vec::new();
    if descriptor.id.is_empty() {
        issues.push("provider id is empty".into());
    }
    for required in ["state", "start", "stop", "restart"] {
        if !descriptor.actions.iter().any(|a| a == required) {
            issues.push(format!("missing action: {required}"));
        }
    }
    if !descriptor.source_scope_complete {
        issues.push(format!(
            "native service provider is unsupported on {}",
            descriptor.platform
        ));
    }
    ServiceProviderConformanceReport {
        valid: issues.is_empty(),
        descriptor,
        issues,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthCheck {
    pub kind: String,
    pub target: String,
    pub healthy: bool,
    pub detail: String,
}

const MAX_PROCESS_SNAPSHOT_ITEMS: usize = 4096;
const MAX_PORT_SNAPSHOT_ITEMS: usize = 8192;
const MAX_TCP_HEALTH_TIMEOUT_MS: u64 = 5_000;
const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;
const MAX_LOG_LINES: usize = 5_000;

pub fn list_processes() -> Result<Vec<ProcessInfo>, SystemError> {
    #[cfg(windows)]
    let mut items = windows_processes()?;
    #[cfg(not(windows))]
    let mut items = unix_processes()?;
    items.sort_by_key(|item| item.pid);
    items.truncate(MAX_PROCESS_SNAPSHOT_ITEMS);
    Ok(items)
}

#[cfg(windows)]
fn windows_processes() -> Result<Vec<ProcessInfo>, SystemError> {
    let output = Command::new("tasklist.exe")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(SystemError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut items = Vec::new();
    for line in text.lines() {
        let cols = parse_csv_line(line);
        if cols.len() >= 2 {
            if let Ok(pid) = cols[1].parse::<u32>() {
                items.push(ProcessInfo {
                    pid,
                    name: cols[0].clone(),
                    command: None,
                });
            }
        }
    }
    Ok(items)
}

#[cfg(not(windows))]
fn unix_processes() -> Result<Vec<ProcessInfo>, SystemError> {
    let output = Command::new("ps")
        .args(["-eo", "pid=,comm=,args="])
        .output()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(SystemError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let mut items = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim();
        let cols: Vec<&str> = trimmed.split_whitespace().collect();
        if cols.len() >= 2 {
            if let Ok(pid) = cols[0].parse::<u32>() {
                let command = if cols.len() > 2 {
                    Some(cols[2..].join(" "))
                } else {
                    None
                };
                items.push(ProcessInfo {
                    pid,
                    name: cols[1].into(),
                    command,
                });
            }
        }
    }
    Ok(items)
}

pub fn list_ports() -> Result<Vec<PortInfo>, SystemError> {
    #[cfg(windows)]
    let mut items = windows_ports()?;
    #[cfg(target_os = "linux")]
    let mut items = linux_ports()?;
    #[cfg(target_os = "macos")]
    let mut items = macos_ports()?;
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    return Err(SystemError::Unsupported("port discovery"));
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    {
        items.sort_by(|a, b| {
            (a.port, &a.local_address, a.pid).cmp(&(b.port, &b.local_address, b.pid))
        });
        items.truncate(MAX_PORT_SNAPSHOT_ITEMS);
        Ok(items)
    }
}

#[cfg(windows)]
fn windows_ports() -> Result<Vec<PortInfo>, SystemError> {
    let output = Command::new("netstat.exe")
        .args(["-ano", "-p", "tcp"])
        .output()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(SystemError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(parse_windows_netstat(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

#[cfg(target_os = "linux")]
fn linux_ports() -> Result<Vec<PortInfo>, SystemError> {
    let output = Command::new("ss")
        .args(["-lntpH"])
        .output()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(SystemError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let local = cols[3];
        if let Some((addr, port)) = split_host_port(local) {
            let pid = line
                .split("pid=")
                .nth(1)
                .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|s| s.parse().ok());
            out.push(PortInfo {
                protocol: "tcp".into(),
                local_address: addr,
                port,
                pid,
                state: Some("LISTEN".into()),
            });
        }
    }
    Ok(out)
}

#[cfg(target_os = "macos")]
fn macos_ports() -> Result<Vec<PortInfo>, SystemError> {
    let output = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN"])
        .output()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(SystemError::Command(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let mut out = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 9 {
            continue;
        }
        if let Some((addr, port)) = split_host_port(cols[8].trim_end_matches(" (LISTEN)")) {
            out.push(PortInfo {
                protocol: "tcp".into(),
                local_address: addr,
                port,
                pid: cols.get(1).and_then(|v| v.parse().ok()),
                state: Some("LISTEN".into()),
            });
        }
    }
    Ok(out)
}

#[cfg(windows)]
fn parse_windows_netstat(text: &str) -> Vec<PortInfo> {
    let mut out = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 || !cols[0].eq_ignore_ascii_case("TCP") {
            continue;
        }
        let state = cols[3];
        if !state.eq_ignore_ascii_case("LISTENING") {
            continue;
        }
        if let Some((addr, port)) = split_host_port(cols[1]) {
            out.push(PortInfo {
                protocol: "tcp".into(),
                local_address: addr,
                port,
                pid: cols[4].parse().ok(),
                state: Some("LISTEN".into()),
            });
        }
    }
    out
}

fn split_host_port(value: &str) -> Option<(String, u16)> {
    let value = value.trim_matches(|c| c == '[' || c == ']');
    let index = value.rfind(':')?;
    let host = value[..index]
        .trim_matches(|c| c == '[' || c == ']')
        .to_string();
    let port = value[index + 1..].parse().ok()?;
    Some((host, port))
}

#[cfg(windows)]
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    out.push(current);
    out
}

pub fn port_conflicts(port: u16) -> Result<Vec<PortInfo>, SystemError> {
    Ok(list_ports()?
        .into_iter()
        .filter(|p| p.port == port)
        .collect())
}

pub fn unique_listening_ports() -> Result<Vec<u16>, SystemError> {
    let set: BTreeSet<u16> = list_ports()?.into_iter().map(|p| p.port).collect();
    Ok(set.into_iter().collect())
}

#[cfg(windows)]
fn windows_native_service_name(name: &str) -> &str {
    if name == "VSN-Agent" {
        "VSNAgent"
    } else {
        name
    }
}

#[cfg(windows)]
fn windows_service_command(name: &str, action: &str) -> Result<std::process::Output, SystemError> {
    let native_name = windows_native_service_name(name);
    Command::new("sc.exe")
        .args([action, native_name])
        .output()
        .map_err(|e| {
            SystemError::Command(format!(
                "sc.exe {action} {name} (native={native_name}) failed to launch: {e}"
            ))
        })
}

#[cfg(windows)]
fn windows_service_output_detail(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = format!("{stdout}\n{stderr}").trim().to_string();
    if detail.is_empty() {
        format!("sc.exe exited with {}", output.status)
    } else {
        detail
    }
}

#[cfg(windows)]
fn wait_for_windows_service_state(
    name: &str,
    expected: &str,
    timeout: Duration,
) -> Result<ServiceState, SystemError> {
    let started = Instant::now();
    loop {
        let state = service_state(name)?;
        if state.state == expected {
            return Ok(state);
        }
        if started.elapsed() >= timeout {
            return Err(SystemError::Command(format!(
                "service {name} did not reach {expected} within {} ms; last state={}",
                timeout.as_millis(),
                state.state
            )));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(windows)]
fn windows_start_service(name: &str, timeout: Duration) -> Result<ServiceState, SystemError> {
    let current = service_state(name)?;
    match current.state.as_str() {
        "running" => return Ok(current),
        "start_pending" => return wait_for_windows_service_state(name, "running", timeout),
        "stop_pending" => {
            let _ = wait_for_windows_service_state(name, "stopped", timeout)?;
        }
        "stopped" => {}
        other => {
            return Err(SystemError::Command(format!(
                "service {name} cannot be started from unsupported state {other}"
            )))
        }
    }
    let output = windows_service_command(name, "start")?;
    if !output.status.success() {
        return Err(SystemError::Command(windows_service_output_detail(&output)));
    }
    wait_for_windows_service_state(name, "running", timeout)
}

#[cfg(windows)]
fn windows_stop_service(name: &str, timeout: Duration) -> Result<ServiceState, SystemError> {
    let mut current = service_state(name)?;
    if current.state == "stopped" {
        return Ok(current);
    }
    if current.state == "start_pending" {
        current = wait_for_windows_service_state(name, "running", timeout)?;
    }
    if current.state == "stop_pending" {
        return wait_for_windows_service_state(name, "stopped", timeout);
    }
    if !matches!(current.state.as_str(), "running" | "paused") {
        return Err(SystemError::Command(format!(
            "service {name} cannot be stopped from unsupported state {}",
            current.state
        )));
    }
    let output = windows_service_command(name, "stop")?;
    if !output.status.success() {
        return Err(SystemError::Command(windows_service_output_detail(&output)));
    }
    wait_for_windows_service_state(name, "stopped", timeout)
}

pub fn service_state(name: &str) -> Result<ServiceState, SystemError> {
    validate_managed_service_name(name)?;
    #[cfg(windows)]
    {
        let output = windows_service_command(name, "query")?;
        let text = windows_service_output_detail(&output);
        if !output.status.success() {
            return Err(SystemError::Command(text));
        }
        let state = text
            .lines()
            .find(|line| line.contains("STATE"))
            .and_then(|line| line.split_whitespace().nth(3))
            .ok_or_else(|| SystemError::Command(format!("unable to parse service state for {name}: {text}")))?;
        return Ok(ServiceState {
            name: name.into(),
            state: state.to_ascii_lowercase(),
            detail: text,
        });
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args(["is-active", name])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        let state = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(ServiceState {
            name: name.into(),
            state,
            detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("launchctl")
            .args(["print", &format!("system/{name}")])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        return Ok(ServiceState {
            name: name.into(),
            state: if output.status.success() {
                "loaded".into()
            } else {
                "unknown".into()
            },
            detail: String::from_utf8_lossy(&output.stdout).into_owned(),
        });
    }
    #[allow(unreachable_code)]
    Err(SystemError::Unsupported("service state"))
}

pub fn service_action(name: &str, action: &str) -> Result<ServiceState, SystemError> {
    validate_managed_service_name(name)?;
    if !matches!(action, "start" | "stop" | "restart") {
        return Err(SystemError::Invalid(
            "action must be start, stop, or restart".into(),
        ));
    }
    #[cfg(windows)]
    {
        const SERVICE_TRANSITION_TIMEOUT: Duration = Duration::from_secs(20);
        return match action {
            "start" => windows_start_service(name, SERVICE_TRANSITION_TIMEOUT),
            "stop" => windows_stop_service(name, SERVICE_TRANSITION_TIMEOUT),
            "restart" => {
                let _ = windows_stop_service(name, SERVICE_TRANSITION_TIMEOUT)?;
                windows_start_service(name, SERVICE_TRANSITION_TIMEOUT)
            }
            _ => unreachable!("service action validated above"),
        };
    }
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .args([action, name])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        if !output.status.success() {
            return Err(SystemError::Command(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        return service_state(name);
    }
    #[cfg(target_os = "macos")]
    {
        let target = format!("system/{name}");
        let args: Vec<String> = match action {
            "start" => vec!["kickstart".into(), target.clone()],
            "stop" => vec!["kill".into(), "SIGTERM".into(), target.clone()],
            "restart" => vec!["kickstart".into(), "-k".into(), target.clone()],
            _ => {
                return Err(SystemError::Invalid(
                    "action must be start, stop, or restart".into(),
                ))
            }
        };
        let output = Command::new("launchctl")
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        if !output.status.success() {
            return Err(SystemError::Command(format!(
                "launchctl {} failed: {}",
                action,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        return service_state(name);
    }
    #[allow(unreachable_code)]
    Err(SystemError::Unsupported("service action"))
}

fn local_health_addresses(host: &str, port: u16) -> Result<Vec<SocketAddr>, SystemError> {
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port),
        ]);
    }
    let ip = host
        .parse::<IpAddr>()
        .map_err(|_| SystemError::Invalid("TCP health host must be localhost or a loopback IP".into()))?;
    if !ip.is_loopback() {
        return Err(SystemError::Invalid(
            "TCP health is restricted to loopback targets".into(),
        ));
    }
    Ok(vec![SocketAddr::new(ip, port)])
}

pub fn tcp_health(host: &str, port: u16, timeout_ms: u64) -> Result<HealthCheck, SystemError> {
    let addresses = local_health_addresses(host, port)?;
    let timeout = Duration::from_millis(timeout_ms.clamp(1, MAX_TCP_HEALTH_TIMEOUT_MS));
    let started = Instant::now();
    let mut connected = false;
    for address in addresses {
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        if TcpStream::connect_timeout(&address, remaining).is_ok() {
            connected = true;
            break;
        }
    }
    let target = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    Ok(HealthCheck {
        kind: "tcp".into(),
        target,
        healthy: connected,
        detail: if connected {
            "connection succeeded".into()
        } else {
            "connection failed".into()
        },
    })
}

pub fn tail_log(path: &Path, max_lines: usize) -> Result<Vec<String>, SystemError> {
    let metadata = fs::symlink_metadata(path).map_err(|e| SystemError::Command(e.to_string()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(SystemError::Invalid(
            "log path must be a regular non-symlink file".into(),
        ));
    }
    let file = fs::File::open(path).map_err(|e| SystemError::Command(e.to_string()))?;
    let mut reader = std::io::BufReader::new(file);
    let start = metadata.len().saturating_sub(MAX_LOG_BYTES);
    use std::io::{BufRead as _, Seek as _, SeekFrom};
    if start > 0 {
        reader
            .seek(SeekFrom::Start(start))
            .map_err(|e| SystemError::Command(e.to_string()))?;
        let mut discard = String::new();
        reader
            .read_line(&mut discard)
            .map_err(|e| SystemError::Command(e.to_string()))?;
    }
    let lines = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    let take = max_lines.clamp(1, MAX_LOG_LINES);
    Ok(lines
        .into_iter()
        .rev()
        .take(take)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect())
}

fn validate_service_name(value: &str) -> Result<(), SystemError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@'))
    {
        return Err(SystemError::Invalid("unsafe service name".into()));
    }
    Ok(())
}

fn validate_managed_service_name(value: &str) -> Result<(), SystemError> {
    validate_service_name(value)?;
    if !value.starts_with("VSN-") || value.len() <= 4 {
        return Err(SystemError::Invalid(
            "managed OS service name must start with VSN-".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessMetrics {
    pub pid: u32,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedProcessSpec {
    pub id: String,
    pub program: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: PathBuf,
    #[serde(default)]
    pub env: Vec<(String, String)>,
    pub log_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedProcessState {
    pub id: String,
    pub pid: u32,
    pub program: PathBuf,
    pub started_at_unix: u64,
    pub running: bool,
    pub log_path: PathBuf,
}

pub fn find_executable(name: &str) -> Result<PathBuf, SystemError> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(SystemError::Invalid("unsafe executable name".into()));
    }
    #[cfg(windows)]
    let output = Command::new("where.exe")
        .arg(name)
        .output()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    #[cfg(not(windows))]
    let output = Command::new("which")
        .arg(name)
        .output()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    if !output.status.success() {
        return Err(SystemError::Command(format!(
            "executable not found: {name}"
        )));
    }
    let first = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if first.is_empty() {
        return Err(SystemError::Command(format!(
            "executable not found: {name}"
        )));
    }
    Ok(PathBuf::from(first))
}

pub fn process_metrics(pid: u32) -> Result<ProcessMetrics, SystemError> {
    if pid == 0 {
        return Err(SystemError::Invalid("pid must be non-zero".into()));
    }
    #[cfg(windows)]
    {
        let script = format!(
            "$p1=Get-Process -Id {pid} -ErrorAction Stop; $cpu1=$p1.TotalProcessorTime.TotalMilliseconds; $sw=[Diagnostics.Stopwatch]::StartNew(); Start-Sleep -Milliseconds 125; $p2=Get-Process -Id {pid} -ErrorAction Stop; $sw.Stop(); $elapsed=[Math]::Max($sw.Elapsed.TotalMilliseconds,1); $cpu2=$p2.TotalProcessorTime.TotalMilliseconds; $pct=[Math]::Max(0,[Math]::Min(100,(($cpu2-$cpu1)/($elapsed*[Environment]::ProcessorCount))*100)); [pscustomobject]@{{WorkingSet64=[uint64]$p2.WorkingSet64;CpuPercent=[double]$pct}} | ConvertTo-Json -Compress"
        );
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        if !output.status.success() {
            return Err(SystemError::Command(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        let value: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| SystemError::Command(e.to_string()))?;
        let cpu_percent = value
            .get("CpuPercent")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32);
        let memory_bytes = value.get("WorkingSet64").and_then(|v| v.as_u64());
        if cpu_percent.is_none() || memory_bytes.is_none() {
            return Err(SystemError::Command(
                "Windows process metrics response is incomplete".into(),
            ));
        }
        return Ok(ProcessMetrics {
            pid,
            cpu_percent,
            memory_bytes,
        });
    }
    #[cfg(not(windows))]
    {
        let output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "%cpu=,rss="])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        if !output.status.success() {
            return Err(SystemError::Command(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
        let line = String::from_utf8_lossy(&output.stdout);
        let cols: Vec<&str> = line.split_whitespace().collect();
        let cpu = cols.first().and_then(|v| v.parse::<f32>().ok());
        let memory_bytes = cols
            .get(1)
            .and_then(|v| v.parse::<u64>().ok())
            .map(|kb| kb * 1024);
        if cpu.is_none() || memory_bytes.is_none() {
            return Err(SystemError::Command(format!(
                "process metrics unavailable for pid {pid}"
            )));
        }
        Ok(ProcessMetrics {
            pid,
            cpu_percent: cpu,
            memory_bytes,
        })
    }
}

pub fn spawn_managed(
    spec: &ManagedProcessSpec,
    state_dir: &Path,
) -> Result<ManagedProcessState, SystemError> {
    validate_managed_process_spec(spec)?;
    fs::create_dir_all(state_dir).map_err(|e| SystemError::Command(e.to_string()))?;
    let existing_path = state_dir.join(format!("{}.json", spec.id));
    if existing_path.exists() {
        if let Ok(existing) = managed_process_state(&spec.id, state_dir) {
            if existing.running {
                return Err(SystemError::Invalid(format!(
                    "managed process already running: {} (pid {})",
                    spec.id, existing.pid
                )));
            }
        }
    }
    if let Some(parent) = spec.log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| SystemError::Command(e.to_string()))?;
    }
    let stdout = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&spec.log_path)
        .map_err(|e| SystemError::Command(e.to_string()))?;
    let stderr = stdout
        .try_clone()
        .map_err(|e| SystemError::Command(e.to_string()))?;
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .current_dir(&spec.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    for (key, value) in &spec.env {
        validate_env_key(key)?;
        command.env(key, value);
    }
    let child = command.spawn().map_err(|e| {
        SystemError::Command(format!("failed to spawn {}: {e}", spec.program.display()))
    })?;
    let state = ManagedProcessState {
        id: spec.id.clone(),
        pid: child.id(),
        program: spec.program.clone(),
        started_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        running: true,
        log_path: spec.log_path.clone(),
    };
    write_managed_state(state_dir, &state)?;
    Ok(state)
}

pub fn managed_process_state(
    id: &str,
    state_dir: &Path,
) -> Result<ManagedProcessState, SystemError> {
    validate_managed_id(id)?;
    let path = state_dir.join(format!("{id}.json"));
    let mut state: ManagedProcessState =
        serde_json::from_slice(&fs::read(&path).map_err(|e| SystemError::Command(e.to_string()))?)
            .map_err(|e| SystemError::Command(e.to_string()))?;
    state.running = pid_running(state.pid);
    if !state.running {
        let _ = write_managed_state(state_dir, &state);
    }
    Ok(state)
}

pub fn stop_managed(id: &str, state_dir: &Path) -> Result<ManagedProcessState, SystemError> {
    let mut state = managed_process_state(id, state_dir)?;
    if !state.running {
        return Ok(state);
    }
    #[cfg(windows)]
    {
        let first = Command::new("taskkill.exe")
            .args(["/PID", &state.pid.to_string(), "/T"])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        if !first.status.success() && !pid_running(state.pid) {
            state.running = false;
            write_managed_state(state_dir, &state)?;
            return Ok(state);
        }
    }
    #[cfg(not(windows))]
    {
        let output = Command::new("kill")
            .args(["-TERM", &state.pid.to_string()])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        if !output.status.success() && pid_running(state.pid) {
            return Err(SystemError::Command(
                String::from_utf8_lossy(&output.stderr).trim().into(),
            ));
        }
    }
    for _ in 0..50 {
        if !pid_running(state.pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if pid_running(state.pid) {
        #[cfg(windows)]
        let force = Command::new("taskkill.exe")
            .args(["/PID", &state.pid.to_string(), "/T", "/F"])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        #[cfg(not(windows))]
        let force = Command::new("kill")
            .args(["-KILL", &state.pid.to_string()])
            .output()
            .map_err(|e| SystemError::Command(e.to_string()))?;
        if !force.status.success() {
            return Err(SystemError::Command(
                String::from_utf8_lossy(&force.stderr).trim().into(),
            ));
        }
    }
    state.running = false;
    write_managed_state(state_dir, &state)?;
    Ok(state)
}
pub fn list_managed(state_dir: &Path) -> Result<Vec<ManagedProcessState>, SystemError> {
    if !state_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(state_dir)
        .map_err(|e| SystemError::Command(e.to_string()))?
        .take(1024)
    {
        let path = entry
            .map_err(|e| SystemError::Command(e.to_string()))?
            .path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|v| v.to_str()) else {
            continue;
        };
        if let Ok(state) = managed_process_state(id, state_dir) {
            out.push(state);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}
pub fn remove_managed(id: &str, state_dir: &Path, force: bool) -> Result<bool, SystemError> {
    validate_managed_id(id)?;
    let path = state_dir.join(format!("{id}.json"));
    if !path.exists() {
        return Ok(false);
    }
    let state = managed_process_state(id, state_dir)?;
    if state.running {
        if !force {
            return Err(SystemError::Invalid(
                "managed process is still running; stop it or set force=true".into(),
            ));
        }
        let _ = stop_managed(id, state_dir)?;
    }
    fs::remove_file(path).map_err(|e| SystemError::Command(e.to_string()))?;
    Ok(true)
}

fn write_managed_state(state_dir: &Path, state: &ManagedProcessState) -> Result<(), SystemError> {
    fs::create_dir_all(state_dir).map_err(|e| SystemError::Command(e.to_string()))?;
    let path = state_dir.join(format!("{}.json", state.id));
    let tmp = path.with_extension("tmp");
    let mut bytes =
        serde_json::to_vec_pretty(state).map_err(|e| SystemError::Command(e.to_string()))?;
    bytes.push(b'\n');
    fs::write(&tmp, bytes).map_err(|e| SystemError::Command(e.to_string()))?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| SystemError::Command(e.to_string()))?;
    }
    fs::rename(tmp, path).map_err(|e| SystemError::Command(e.to_string()))
}

fn pid_running(pid: u32) -> bool {
    #[cfg(windows)]
    {
        Command::new("tasklist.exe")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

fn validate_managed_process_spec(spec: &ManagedProcessSpec) -> Result<(), SystemError> {
    validate_managed_id(&spec.id)?;
    if !spec.program.is_absolute() {
        return Err(SystemError::Invalid(
            "managed program must be an absolute path".into(),
        ));
    }
    if !spec.program.exists() {
        return Err(SystemError::Invalid(format!(
            "managed program not found: {}",
            spec.program.display()
        )));
    }
    if !spec.cwd.is_dir() {
        return Err(SystemError::Invalid(
            "managed process cwd must be a directory".into(),
        ));
    }
    if spec.args.len() > 256 || spec.env.len() > 128 {
        return Err(SystemError::Invalid(
            "managed process argument/environment limit exceeded".into(),
        ));
    }
    Ok(())
}
fn validate_managed_id(value: &str) -> Result<(), SystemError> {
    if value.len() < 5
        || value.len() > 96
        || !value.starts_with("vsn-")
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(SystemError::Invalid(
            "managed process id must start with vsn- and contain safe characters".into(),
        ));
    }
    Ok(())
}
fn validate_env_key(value: &str) -> Result<(), SystemError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(SystemError::Invalid(
            "unsafe environment variable name".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_port_parser_handles_ipv4() {
        assert_eq!(
            split_host_port("127.0.0.1:8080"),
            Some(("127.0.0.1".into(), 8080))
        );
    }
    #[test]
    fn tcp_health_rejects_non_loopback_targets() {
        assert!(tcp_health("192.0.2.1", 443, 10).is_err());
        assert!(tcp_health("example.com", 443, 10).is_err());
    }
    #[test]
    fn tcp_health_timeout_is_bounded() {
        let started = Instant::now();
        let result = tcp_health("127.0.0.1", 9, u64::MAX).unwrap();
        assert!(!result.healthy);
        assert!(started.elapsed() < Duration::from_secs(6));
    }
    #[test]
    fn service_name_validation_blocks_shell_chars() {
        assert!(validate_service_name("mysql;whoami").is_err());
    }
    #[test]
    fn managed_service_name_requires_vsn_namespace() {
        assert!(validate_managed_service_name("VSN-Agent").is_ok());
        assert!(validate_managed_service_name("Spooler").is_err());
        assert!(validate_managed_service_name("vsn-agent").is_err());
        assert!(validate_managed_service_name("VSN-").is_err());
    }
    #[test]
    fn service_api_rejects_outside_namespace_before_os_dispatch() {
        assert!(service_state("Spooler").is_err());
        assert!(service_action("Spooler", "start").is_err());
    }
    #[cfg(windows)]
    #[test]
    fn windows_netstat_only_reports_listeners() {
        let parsed = parse_windows_netstat(
            "  TCP    127.0.0.1:41000    0.0.0.0:0    LISTENING    1234\r\n  TCP    127.0.0.1:53000    93.184.216.34:443    ESTABLISHED    1234\r\n",
        );
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].port, 41000);
        assert_eq!(parsed[0].state.as_deref(), Some("LISTEN"));
    }
    #[cfg(windows)]
    #[test]
    fn windows_agent_public_alias_preserves_legacy_scm_name() {
        assert_eq!(windows_native_service_name("VSN-Agent"), "VSNAgent");
        assert_eq!(
            windows_native_service_name("VSN-PKG02-0213"),
            "VSN-PKG02-0213"
        );
    }
}
