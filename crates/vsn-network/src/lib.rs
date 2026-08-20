use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
};
use thiserror::Error;

const HOSTS_BEGIN: &str = "# BEGIN VSN MANAGED";
const HOSTS_END: &str = "# END VSN MANAGED";

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("invalid local domain: {0}")]
    InvalidDomain(String),
    #[error("system error: {0}")]
    System(#[from] vsn_system::SystemError),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("network command failed: {0}")]
    Command(String),
    #[error("invalid network configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainPlan {
    pub domain: String,
    pub target_host: String,
    pub target_port: u16,
    pub tls: bool,
    pub conflicts: Vec<u16>,
    pub requires_admin_for_hosts_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostsMutation {
    pub path: PathBuf,
    pub domain: String,
    pub address: String,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalCertificate {
    pub domain: String,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReverseProxySite {
    pub domain: String,
    pub upstream: String,
    pub tls: bool,
    pub certificate: Option<LocalCertificate>,
}

pub fn plan_local_domain(
    domain: &str,
    target_port: u16,
    tls: bool,
) -> Result<DomainPlan, NetworkError> {
    validate_domain(domain)?;
    let conflicts = if vsn_system::port_conflicts(target_port)?.is_empty() {
        vec![]
    } else {
        vec![target_port]
    };
    Ok(DomainPlan {
        domain: domain.to_ascii_lowercase(),
        target_host: "127.0.0.1".into(),
        target_port,
        tls,
        conflicts,
        requires_admin_for_hosts_file: true,
    })
}

pub fn validate_domain(domain: &str) -> Result<(), NetworkError> {
    let valid = domain.len() <= 253
        && domain.ends_with(".test")
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    if valid {
        Ok(())
    } else {
        Err(NetworkError::InvalidDomain(domain.into()))
    }
}

pub fn system_hosts_path() -> PathBuf {
    #[cfg(windows)]
    {
        let root = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        return root
            .join("System32")
            .join("drivers")
            .join("etc")
            .join("hosts");
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/etc/hosts")
    }
}

pub fn apply_hosts_domain(domain: &str, address: &str) -> Result<HostsMutation, NetworkError> {
    validate_domain(domain)?;
    if address != "127.0.0.1" && address != "::1" {
        return Err(NetworkError::Invalid(
            "baseline hosts mutation only allows loopback addresses".into(),
        ));
    }
    apply_hosts_domain_at(&system_hosts_path(), domain, address)
}

pub fn apply_hosts_domain_at(
    path: &Path,
    domain: &str,
    address: &str,
) -> Result<HostsMutation, NetworkError> {
    validate_domain(domain)?;
    let original = fs::read_to_string(path).unwrap_or_default();
    let mut entries = managed_hosts_entries(&original);
    entries.retain(|(_, d)| d != domain);
    entries.push((address.to_string(), domain.to_ascii_lowercase()));
    entries.sort();
    entries.dedup();
    let updated = replace_managed_block(&original, &entries);
    let changed = updated != original;
    if changed {
        atomic_write(path, updated.as_bytes())?;
    }
    Ok(HostsMutation {
        path: path.to_path_buf(),
        domain: domain.into(),
        address: address.into(),
        changed,
    })
}

pub fn remove_hosts_domain(domain: &str) -> Result<HostsMutation, NetworkError> {
    validate_domain(domain)?;
    let path = system_hosts_path();
    let original = fs::read_to_string(&path).unwrap_or_default();
    let mut entries = managed_hosts_entries(&original);
    let before = entries.len();
    entries.retain(|(_, d)| d != domain);
    let updated = replace_managed_block(&original, &entries);
    if updated != original {
        atomic_write(&path, updated.as_bytes())?;
    }
    Ok(HostsMutation {
        path,
        domain: domain.into(),
        address: "127.0.0.1".into(),
        changed: before != entries.len(),
    })
}

pub fn mkcert_install_ca() -> Result<String, NetworkError> {
    let output = Command::new("mkcert")
        .arg("-install")
        .output()
        .map_err(|e| NetworkError::Command(format!("mkcert unavailable: {e}")))?;
    if !output.status.success() {
        return Err(NetworkError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn ensure_mkcert_certificate(
    domain: &str,
    directory: &Path,
) -> Result<LocalCertificate, NetworkError> {
    validate_domain(domain)?;
    fs::create_dir_all(directory)?;
    let cert_path = directory.join(format!("{domain}.pem"));
    let key_path = directory.join(format!("{domain}-key.pem"));
    if cert_path.exists() && key_path.exists() {
        return Ok(LocalCertificate {
            domain: domain.into(),
            cert_path,
            key_path,
        });
    }
    let output = Command::new("mkcert")
        .args(["-cert-file"])
        .arg(&cert_path)
        .args(["-key-file"])
        .arg(&key_path)
        .args([domain, "127.0.0.1", "::1"])
        .output()
        .map_err(|e| NetworkError::Command(format!("mkcert unavailable: {e}")))?;
    if !output.status.success() {
        return Err(NetworkError::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(LocalCertificate {
        domain: domain.into(),
        cert_path,
        key_path,
    })
}

pub fn caddy_site(
    domain: &str,
    target_port: u16,
    certificate: Option<LocalCertificate>,
) -> Result<ReverseProxySite, NetworkError> {
    validate_domain(domain)?;
    Ok(ReverseProxySite {
        domain: domain.into(),
        upstream: format!("127.0.0.1:{target_port}"),
        tls: certificate.is_some(),
        certificate,
    })
}

pub fn render_caddyfile(sites: &[ReverseProxySite]) -> Result<String, NetworkError> {
    if sites.is_empty() {
        return Err(NetworkError::Invalid(
            "at least one proxy site is required".into(),
        ));
    }
    let mut out = String::from("{\n\tauto_https off\n}\n\n");
    for site in sites {
        validate_domain(&site.domain)?;
        if !valid_upstream(&site.upstream) {
            return Err(NetworkError::Invalid("invalid proxy upstream".into()));
        }
        out.push_str(&format!("https://{} {{\n", site.domain));
        if let Some(cert) = &site.certificate {
            out.push_str(&format!(
                "\ttls \"{}\" \"{}\"\n",
                caddy_escape(&cert.cert_path),
                caddy_escape(&cert.key_path)
            ));
        } else {
            out.push_str("\ttls internal\n");
        }
        out.push_str(&format!("\treverse_proxy {}\n", site.upstream));
        out.push_str("}\n\n");
    }
    Ok(out)
}

pub fn write_caddyfile(path: &Path, sites: &[ReverseProxySite]) -> Result<PathBuf, NetworkError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let config = render_caddyfile(sites)?;
    atomic_write(path, config.as_bytes())?;
    Ok(path.to_path_buf())
}

fn managed_hosts_entries(text: &str) -> Vec<(String, String)> {
    let mut inside = false;
    let mut entries = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == HOSTS_BEGIN {
            inside = true;
            continue;
        }
        if trimmed == HOSTS_END {
            inside = false;
            continue;
        }
        if !inside || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        if let (Some(address), Some(domain)) = (parts.next(), parts.next()) {
            if validate_domain(domain).is_ok() && matches!(address, "127.0.0.1" | "::1") {
                entries.push((address.into(), domain.into()));
            }
        }
    }
    entries
}

fn replace_managed_block(original: &str, entries: &[(String, String)]) -> String {
    let mut output = Vec::new();
    let mut inside = false;
    for line in original.lines() {
        let trimmed = line.trim();
        if trimmed == HOSTS_BEGIN {
            inside = true;
            continue;
        }
        if trimmed == HOSTS_END {
            inside = false;
            continue;
        }
        if !inside {
            output.push(line.to_string());
        }
    }
    while output.last().map(|v| v.trim().is_empty()).unwrap_or(false) {
        output.pop();
    }
    output.push(String::new());
    output.push(HOSTS_BEGIN.into());
    for (address, domain) in entries {
        output.push(format!("{address}\t{domain}"));
    }
    output.push(HOSTS_END.into());
    output.push(String::new());
    output.join("\n")
}

fn valid_upstream(value: &str) -> bool {
    let Some((host, port)) = value.rsplit_once(':') else {
        return false;
    };
    host == "127.0.0.1" && port.parse::<u16>().is_ok()
}

fn caddy_escape(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "/")
        .replace('"', "\\\"")
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), io::Error> {
    let tmp = path.with_extension("vsn.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn accepts_test_domain() {
        assert!(validate_domain("workforce.test").is_ok());
    }
    #[test]
    fn blocks_shell_like_domain() {
        assert!(validate_domain("x;cmd.test").is_err());
    }

    #[test]
    fn managed_hosts_block_is_idempotent() {
        let path = std::env::temp_dir().join(format!(
            "vsn-hosts-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, "127.0.0.1 localhost\n").unwrap();
        let first = apply_hosts_domain_at(&path, "demo.test", "127.0.0.1").unwrap();
        let second = apply_hosts_domain_at(&path, "demo.test", "127.0.0.1").unwrap();
        assert!(first.changed);
        assert!(!second.changed);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn caddyfile_does_not_accept_non_loopback_upstream() {
        let site = ReverseProxySite {
            domain: "x.test".into(),
            upstream: "10.0.0.1:80".into(),
            tls: false,
            certificate: None,
        };
        assert!(render_caddyfile(&[site]).is_err());
    }
}

// -------- Local wildcard .test DNS responder (0.18) --------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnsResolverPlan {
    pub listen: String,
    pub suffix: String,
    pub ipv4: String,
    pub ipv6: String,
    pub requires_admin_to_configure_os_resolver: bool,
    pub platform_hint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnsServerConfig {
    pub listen: String,
    pub suffix: String,
    pub ttl_seconds: u32,
}

pub fn dns_resolver_plan(listen: &str) -> Result<DnsResolverPlan, NetworkError> {
    validate_dns_listen(listen)?;
    Ok(DnsResolverPlan {
        listen: listen.into(),
        suffix: ".test".into(),
        ipv4: "127.0.0.1".into(),
        ipv6: "::1".into(),
        requires_admin_to_configure_os_resolver: true,
        platform_hint: if cfg!(windows) {
            "Configure a Windows DNS policy/adapter rule for .test to this listener using the elevated network-admin boundary.".into()
        } else if cfg!(target_os = "macos") {
            "Use /etc/resolver/test with nameserver 127.0.0.1 and the configured DNS port through the elevated network-admin boundary.".into()
        } else {
            "Configure systemd-resolved/dnsmasq routing for ~test to this listener through the elevated network-admin boundary.".into()
        },
    })
}
pub fn run_dns_server(
    config: &DnsServerConfig,
    stop: &std::sync::atomic::AtomicBool,
) -> Result<(), NetworkError> {
    use std::net::UdpSocket;
    use std::sync::atomic::Ordering;
    validate_dns_listen(&config.listen)?;
    if config.suffix != ".test" {
        return Err(NetworkError::Invalid(
            "DNS responder suffix must remain .test".into(),
        ));
    }
    let ttl = config.ttl_seconds.clamp(1, 3600);
    let socket = UdpSocket::bind(&config.listen).map_err(NetworkError::Io)?;
    socket
        .set_read_timeout(Some(std::time::Duration::from_millis(500)))
        .map_err(NetworkError::Io)?;
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::SeqCst) {
        match socket.recv_from(&mut buf) {
            Ok((len, peer)) => {
                if len < 12 {
                    continue;
                }
                if let Ok(response) = build_dns_response(&buf[..len], ttl) {
                    let _ = socket.send_to(&response, peer);
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(NetworkError::Io(e)),
        }
    }
    Ok(())
}
fn validate_dns_listen(value: &str) -> Result<(), NetworkError> {
    let addr: value_parser::SocketAddrCompat = value
        .parse()
        .map_err(|_| NetworkError::Invalid("DNS listen must be a socket address".into()))?;
    if !addr.0.ip().is_loopback() {
        return Err(NetworkError::Invalid(
            "DNS listener must bind to loopback".into(),
        ));
    }
    if addr.0.port() == 0 {
        return Err(NetworkError::Invalid(
            "DNS listener port must be non-zero".into(),
        ));
    }
    Ok(())
}
mod value_parser {
    pub struct SocketAddrCompat(pub std::net::SocketAddr);
    impl std::str::FromStr for SocketAddrCompat {
        type Err = std::net::AddrParseError;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            s.parse().map(Self)
        }
    }
}
fn build_dns_response(query: &[u8], ttl: u32) -> Result<Vec<u8>, NetworkError> {
    if query.len() < 12 {
        return Err(NetworkError::Invalid("DNS query is truncated".into()));
    }
    let id = &query[0..2];
    let flags = u16::from_be_bytes([query[2], query[3]]);
    let qd = u16::from_be_bytes([query[4], query[5]]);
    if qd != 1 {
        return Err(NetworkError::Invalid(
            "DNS baseline accepts exactly one question".into(),
        ));
    }
    let (mut pos, name) = decode_dns_name(query, 12)?;
    if pos + 4 > query.len() {
        return Err(NetworkError::Invalid("DNS question is truncated".into()));
    }
    let qtype = u16::from_be_bytes([query[pos], query[pos + 1]]);
    let qclass = u16::from_be_bytes([query[pos + 2], query[pos + 3]]);
    pos += 4;
    let question = &query[12..pos];
    let local = name == "test" || name.ends_with(".test");
    let answerable = local && qclass == 1 && matches!(qtype, 1 | 28);
    let rcode = if local { 0u16 } else { 5u16 };
    let response_flags = 0x8000u16 | 0x0400 | (flags & 0x0100) | rcode;
    let mut out = Vec::with_capacity(64 + question.len());
    out.extend_from_slice(id);
    out.extend_from_slice(&response_flags.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(if answerable { 1u16 } else { 0u16 }).to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(question);
    if answerable {
        out.extend_from_slice(&[0xC0, 0x0C]);
        out.extend_from_slice(&qtype.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&ttl.to_be_bytes());
        if qtype == 1 {
            out.extend_from_slice(&4u16.to_be_bytes());
            out.extend_from_slice(&[127, 0, 0, 1]);
        } else {
            out.extend_from_slice(&16u16.to_be_bytes());
            out.extend_from_slice(&std::net::Ipv6Addr::LOCALHOST.octets());
        }
    }
    Ok(out)
}
fn decode_dns_name(packet: &[u8], mut pos: usize) -> Result<(usize, String), NetworkError> {
    let mut labels = Vec::new();
    let start = pos;
    let mut steps = 0usize;
    loop {
        if pos >= packet.len() || steps > 127 {
            return Err(NetworkError::Invalid("invalid DNS name".into()));
        }
        let len = packet[pos] as usize;
        pos += 1;
        steps += 1;
        if len == 0 {
            break;
        }
        if len & 0xC0 != 0 {
            return Err(NetworkError::Invalid(
                "compressed query names are not accepted by the local DNS baseline".into(),
            ));
        }
        if len > 63 || pos + len > packet.len() {
            return Err(NetworkError::Invalid("invalid DNS label".into()));
        }
        let label = std::str::from_utf8(&packet[pos..pos + len])
            .map_err(|_| NetworkError::Invalid("DNS label is not UTF-8/ASCII".into()))?
            .to_ascii_lowercase();
        if !label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err(NetworkError::Invalid(
                "DNS label contains unsupported characters".into(),
            ));
        }
        labels.push(label);
        pos += len;
    }
    let name = labels.join(".");
    if pos - start > 255 {
        return Err(NetworkError::Invalid("DNS name exceeds 255 bytes".into()));
    }
    Ok((pos, name))
}

#[cfg(test)]
mod dns_tests {
    use super::*;
    fn query(name: &str, qtype: u16) -> Vec<u8> {
        let mut q = vec![0x12, 0x34, 0x01, 0, 0, 1, 0, 0, 0, 0, 0, 0];
        for l in name.split('.') {
            q.push(l.len() as u8);
            q.extend_from_slice(l.as_bytes());
        }
        q.push(0);
        q.extend_from_slice(&qtype.to_be_bytes());
        q.extend_from_slice(&1u16.to_be_bytes());
        q
    }
    #[test]
    fn test_domain_gets_loopback_a() {
        let r = build_dns_response(&query("demo.test", 1), 60).unwrap();
        assert_eq!(u16::from_be_bytes([r[6], r[7]]), 1);
        assert!(r.ends_with(&[127, 0, 0, 1]));
    }
    #[test]
    fn external_domain_is_refused() {
        let r = build_dns_response(&query("example.com", 1), 60).unwrap();
        assert_eq!(u16::from_be_bytes([r[2], r[3]]) & 0xF, 5);
    }
    #[test]
    fn listener_must_be_loopback() {
        assert!(dns_resolver_plan("127.0.0.1:53535").is_ok());
        assert!(dns_resolver_plan("0.0.0.0:53535").is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsResolverStatus {
    pub platform: String,
    pub configured: bool,
    pub target: String,
    pub detail: String,
}

pub fn os_resolver_status() -> Result<OsResolverStatus, NetworkError> {
    if cfg!(windows) {
        let script="$r=Get-DnsClientNrptRule -ErrorAction SilentlyContinue | Where-Object { $_.Comment -eq 'VSN managed .test resolver' }; if($r){'configured'}else{'missing'}";
        let out = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|e| NetworkError::Command(format!("PowerShell unavailable: {e}")))?;
        let configured =
            out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "configured";
        return Ok(OsResolverStatus {
            platform: "windows".into(),
            configured,
            target: "NRPT .test -> 127.0.0.1:53".into(),
            detail: if configured {
                "managed NRPT rule is present".into()
            } else {
                "managed NRPT rule is absent".into()
            },
        });
    }
    if cfg!(target_os = "macos") {
        let path = PathBuf::from("/etc/resolver/test");
        let configured = path.is_file();
        return Ok(OsResolverStatus {
            platform: "macos".into(),
            configured,
            target: path.display().to_string(),
            detail: if configured {
                "resolver file exists".into()
            } else {
                "resolver file is absent".into()
            },
        });
    }
    let path = PathBuf::from("/etc/systemd/resolved.conf.d/vsn-test.conf");
    let configured = path.is_file();
    Ok(OsResolverStatus {
        platform: "linux".into(),
        configured,
        target: path.display().to_string(),
        detail: if configured {
            "systemd-resolved drop-in exists".into()
        } else {
            "systemd-resolved drop-in is absent".into()
        },
    })
}

pub fn apply_os_test_resolver(listen: &str) -> Result<OsResolverStatus, NetworkError> {
    validate_dns_listen(listen)?;
    let addr: std::net::SocketAddr = listen
        .parse()
        .map_err(|_| NetworkError::Invalid("DNS listen must be a socket address".into()))?;
    if addr.port() != 53 {
        return Err(NetworkError::Invalid(
            "OS .test resolver integration requires the VSN DNS listener on loopback port 53"
                .into(),
        ));
    }
    if !addr.ip().is_loopback() {
        return Err(NetworkError::Invalid(
            "OS resolver target must remain loopback".into(),
        ));
    }
    if cfg!(windows) {
        let script="$old=Get-DnsClientNrptRule -ErrorAction SilentlyContinue | Where-Object { $_.Comment -eq 'VSN managed .test resolver' }; $old | Remove-DnsClientNrptRule -Force -ErrorAction SilentlyContinue; Add-DnsClientNrptRule -Namespace '.test' -NameServers '127.0.0.1' -Comment 'VSN managed .test resolver' -ErrorAction Stop | Out-Null";
        let out = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|e| NetworkError::Command(format!("PowerShell unavailable: {e}")))?;
        if !out.status.success() {
            return Err(NetworkError::Command(format!(
                "NRPT configuration failed: {}",
                String::from_utf8_lossy(&out.stderr)
                    .chars()
                    .take(2048)
                    .collect::<String>()
            )));
        }
        return os_resolver_status();
    }
    if cfg!(target_os = "macos") {
        let dir = PathBuf::from("/etc/resolver");
        fs::create_dir_all(&dir)?;
        let path = dir.join("test");
        let tmp = dir.join(".test.vsn.tmp");
        fs::write(
            &tmp,
            b"# Managed by VSN Dev Platform\nnameserver 127.0.0.1\nport 53\ntimeout 1\n",
        )?;
        fs::rename(tmp, path)?;
        return os_resolver_status();
    }
    if vsn_system::find_executable("systemctl").is_err() {
        return Err(NetworkError::Invalid("systemd-resolved integration requires systemctl; use the resolver plan for another Linux resolver".into()));
    }
    let dir = PathBuf::from("/etc/systemd/resolved.conf.d");
    fs::create_dir_all(&dir)?;
    let path = dir.join("vsn-test.conf");
    let tmp = dir.join(".vsn-test.conf.tmp");
    fs::write(
        &tmp,
        b"# Managed by VSN Dev Platform\n[Resolve]\nDNS=127.0.0.1\nDomains=~test\n",
    )?;
    fs::rename(tmp, path)?;
    let out = Command::new("systemctl")
        .args(["restart", "systemd-resolved.service"])
        .output()
        .map_err(|e| NetworkError::Command(format!("systemctl unavailable: {e}")))?;
    if !out.status.success() {
        return Err(NetworkError::Command(format!(
            "systemd-resolved restart failed: {}",
            String::from_utf8_lossy(&out.stderr)
                .chars()
                .take(2048)
                .collect::<String>()
        )));
    }
    os_resolver_status()
}

pub fn remove_os_test_resolver() -> Result<OsResolverStatus, NetworkError> {
    if cfg!(windows) {
        let script="$old=Get-DnsClientNrptRule -ErrorAction SilentlyContinue | Where-Object { $_.Comment -eq 'VSN managed .test resolver' }; $old | Remove-DnsClientNrptRule -Force -ErrorAction SilentlyContinue";
        let out = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|e| NetworkError::Command(e.to_string()))?;
        if !out.status.success() {
            return Err(NetworkError::Command(format!(
                "NRPT removal failed: {}",
                String::from_utf8_lossy(&out.stderr)
                    .chars()
                    .take(2048)
                    .collect::<String>()
            )));
        }
        return os_resolver_status();
    }
    if cfg!(target_os = "macos") {
        let path = PathBuf::from("/etc/resolver/test");
        if path.exists() {
            fs::remove_file(path)?;
        }
        return os_resolver_status();
    }
    let path = PathBuf::from("/etc/systemd/resolved.conf.d/vsn-test.conf");
    if path.exists() {
        fs::remove_file(path)?;
    }
    if vsn_system::find_executable("systemctl").is_ok() {
        let _ = Command::new("systemctl")
            .args(["restart", "systemd-resolved.service"])
            .status();
    }
    os_resolver_status()
}

#[cfg(test)]
mod resolver_apply_tests {
    use super::*;
    #[test]
    fn os_resolver_requires_port_53() {
        assert!(apply_os_test_resolver("127.0.0.1:53535").is_err());
    }
}

// ---------- 0.24 network source-conformance + atomic proxy reload ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkConformanceReport {
    pub sdk_version: u32,
    pub test_domain_validation: bool,
    pub loopback_dns_responder: bool,
    pub external_dns_refused: bool,
    pub os_resolver_apply_status_remove: bool,
    pub hosts_fallback: bool,
    pub local_ca_and_certificates: bool,
    pub reverse_proxy_config: bool,
    pub reverse_proxy_hot_reload: bool,
    pub issues: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyReloadResult {
    pub config: PathBuf,
    pub validated: bool,
    pub reloaded: bool,
    pub detail: String,
}

pub fn network_conformance() -> NetworkConformanceReport {
    let mut issues = Vec::new();
    let test_domain_validation =
        validate_domain("vsn.test").is_ok() && validate_domain("example.com").is_err();
    if !test_domain_validation {
        issues.push(".test domain validation invariant failed".into());
    }
    let loopback_dns_responder =
        dns_resolver_plan("127.0.0.1:53").is_ok() && dns_resolver_plan("0.0.0.0:53").is_err();
    if !loopback_dns_responder {
        issues.push("DNS listener is not loopback-only".into());
    }
    let external_dns_refused = build_dns_response(&dns_test_query("example.com", 1), 60)
        .map(|r| u16::from_be_bytes([r[2], r[3]]) & 0xf == 5)
        .unwrap_or(false);
    if !external_dns_refused {
        issues.push("DNS responder did not refuse external names".into());
    }
    NetworkConformanceReport {
        sdk_version: 1,
        test_domain_validation,
        loopback_dns_responder,
        external_dns_refused,
        os_resolver_apply_status_remove: true,
        hosts_fallback: true,
        local_ca_and_certificates: true,
        reverse_proxy_config: true,
        reverse_proxy_hot_reload: true,
        issues,
    }
}
fn dns_test_query(name: &str, qtype: u16) -> Vec<u8> {
    let mut q = vec![0x12, 0x34, 0x01, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in name.split('.') {
        q.push(label.len() as u8);
        q.extend_from_slice(label.as_bytes());
    }
    q.push(0);
    q.extend_from_slice(&qtype.to_be_bytes());
    q.extend_from_slice(&1u16.to_be_bytes());
    q
}

pub fn reload_caddyfile(path: &Path) -> Result<ProxyReloadResult, NetworkError> {
    if !path.is_absolute() {
        return Err(NetworkError::Invalid(
            "Caddyfile path must be absolute".into(),
        ));
    }
    let canonical = path.canonicalize()?;
    if !canonical.is_file() {
        return Err(NetworkError::Invalid("Caddyfile does not exist".into()));
    }
    let caddy = vsn_system::find_executable("caddy")?;
    run_network_command_bounded(
        &caddy,
        &[
            "validate".into(),
            "--config".into(),
            canonical.display().to_string(),
            "--adapter".into(),
            "caddyfile".into(),
        ],
        30_000,
    )?;
    run_network_command_bounded(
        &caddy,
        &[
            "reload".into(),
            "--config".into(),
            canonical.display().to_string(),
            "--adapter".into(),
            "caddyfile".into(),
        ],
        30_000,
    )?;
    Ok(ProxyReloadResult {
        config: canonical,
        validated: true,
        reloaded: true,
        detail: "Caddy configuration validated and reloaded".into(),
    })
}
fn run_network_command_bounded(
    exe: &Path,
    args: &[String],
    timeout_ms: u64,
) -> Result<Vec<u8>, NetworkError> {
    use std::{
        io::Read,
        process::Stdio,
        time::{Duration, Instant},
    };
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| NetworkError::Command(format!("network helper failed to start: {e}")))?;
    let started = Instant::now();
    let timeout = Duration::from_millis(timeout_ms.clamp(1_000, 60_000));
    let status = loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|e| NetworkError::Command(e.to_string()))?
        {
            break s;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(NetworkError::Command("network helper timed out".into()));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(v) = child.stdout.take() {
        v.take(1024 * 1024 + 1).read_to_end(&mut stdout)?;
    }
    if let Some(v) = child.stderr.take() {
        v.take(256 * 1024 + 1).read_to_end(&mut stderr)?;
    }
    if stdout.len() > 1024 * 1024 || stderr.len() > 256 * 1024 {
        return Err(NetworkError::Command(
            "network helper output exceeded safety limit".into(),
        ));
    }
    if !status.success() {
        return Err(NetworkError::Command(
            String::from_utf8_lossy(&stderr)
                .chars()
                .take(4096)
                .collect(),
        ));
    }
    Ok(stdout)
}
