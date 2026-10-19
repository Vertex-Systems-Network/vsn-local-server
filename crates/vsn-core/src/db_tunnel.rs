//! SSH tunnels for saved remote database connections (DVF-04).
//!
//! The system `ssh` client forwards a free 127.0.0.1 port to the database host as seen
//! from the jump host. Authentication is key/agent only (`BatchMode=yes`) and the jump
//! host's key must already be known (`StrictHostKeyChecking=yes`); nothing is prompted.
//! Tunnels are reused per saved connection while the ssh process is alive.

use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::CoreError;

/// A validated jump host: `[user@]host[:port]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTarget {
    pub user: Option<String>,
    pub host: String,
    pub port: u16,
}

fn safe_part(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && s.len() <= 253
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

pub fn parse_ssh_target(spec: &str) -> Result<SshTarget, CoreError> {
    let bad = || CoreError::Rejected("SSH tunnel must look like user@host or user@host:22".into());
    let spec = spec.trim();
    let (user, rest) = match spec.split_once('@') {
        Some((u, r)) => (Some(u), r),
        None => (None, spec),
    };
    let (host, port) = match rest.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| bad())?),
        None => (rest, 22),
    };
    if !safe_part(host) || port == 0 || user.is_some_and(|u| !safe_part(u)) {
        return Err(bad());
    }
    Ok(SshTarget {
        user: user.map(str::to_string),
        host: host.to_string(),
        port,
    })
}

fn default_port(engine: &str) -> u16 {
    match engine {
        "postgres" => 5432,
        "mysql" => 3306,
        "mongo" => 27017,
        _ => 6379,
    }
}

/// The database host and port a connection string points at.
pub fn connection_endpoint(engine: &str, conn: &str) -> Result<(String, u16), CoreError> {
    let bad = || CoreError::Rejected("cannot find the database host in the connection".into());
    if let Some((_, rest)) = conn.split_once("://") {
        let authority = rest.split(['/', '?']).next().unwrap_or(rest);
        let hostport = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        if hostport.contains(',') {
            return Err(CoreError::Rejected(
                "SSH tunnels support a single database host".into(),
            ));
        }
        let (host, port) = match hostport.rsplit_once(':') {
            Some((h, p)) if !h.ends_with(']') || h.starts_with('[') => {
                (h, p.parse::<u16>().map_err(|_| bad())?)
            }
            _ => (hostport, default_port(engine)),
        };
        let host = host.trim_start_matches('[').trim_end_matches(']');
        if host.is_empty() {
            return Err(bad());
        }
        return Ok((host.to_string(), port));
    }
    let mut host = None;
    let mut port = default_port(engine);
    for kv in conn.split_whitespace() {
        if let Some(v) = kv.strip_prefix("host=") {
            host = Some(v.to_string());
        } else if let Some(v) = kv.strip_prefix("port=") {
            port = v.parse().map_err(|_| bad())?;
        }
    }
    Ok((host.ok_or_else(bad)?, port))
}

/// The same connection string pointed at `127.0.0.1:local_port`.
pub fn rewrite_endpoint(conn: &str, local_port: u16) -> String {
    let local = format!("127.0.0.1:{local_port}");
    if let Some((scheme, rest)) = conn.split_once("://") {
        let end = rest.find(['/', '?']).unwrap_or(rest.len());
        let (authority, tail) = rest.split_at(end);
        let creds = authority.rsplit_once('@').map(|(c, _)| format!("{c}@"));
        return format!("{scheme}://{}{local}{tail}", creds.unwrap_or_default());
    }
    let mut parts: Vec<String> = conn
        .split_whitespace()
        .filter(|kv| {
            !kv.starts_with("host=") && !kv.starts_with("port=") && !kv.starts_with("hostaddr=")
        })
        .map(str::to_string)
        .collect();
    parts.insert(0, format!("port={local_port}"));
    parts.insert(0, "host=127.0.0.1".into());
    parts.join(" ")
}

struct Tunnel {
    child: Child,
    local_port: u16,
    key: String,
}

fn tunnels() -> &'static Mutex<HashMap<String, Tunnel>> {
    static SET: OnceLock<Mutex<HashMap<String, Tunnel>>> = OnceLock::new();
    SET.get_or_init(Default::default)
}

fn free_port() -> Result<u16, CoreError> {
    let l = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| CoreError::Rejected(format!("no free local port: {e}")))?;
    l.local_addr()
        .map(|a| a.port())
        .map_err(|e| CoreError::Rejected(e.to_string()))
}

/// Open (or reuse) a tunnel for saved connection `id` and return the local port.
pub fn ensure_tunnel(
    id: &str,
    ssh: &SshTarget,
    db_host: &str,
    db_port: u16,
) -> Result<u16, CoreError> {
    if !safe_part(db_host) {
        return Err(CoreError::Rejected(
            "database host is not valid for a tunnel".into(),
        ));
    }
    let key = format!("{ssh:?}|{db_host}:{db_port}");
    let mut map = tunnels()
        .lock()
        .map_err(|_| CoreError::Rejected("tunnel table unavailable".into()))?;
    if let Some(t) = map.get_mut(id) {
        if t.key == key && matches!(t.child.try_wait(), Ok(None)) {
            return Ok(t.local_port);
        }
        let _ = t.child.kill();
        let _ = t.child.wait();
        map.remove(id);
    }
    let program = vsn_system::find_executable("ssh")
        .map_err(|_| CoreError::Rejected("install OpenSSH (ssh) to use SSH tunnels".into()))?;
    let local_port = free_port()?;
    let destination = match &ssh.user {
        Some(u) => format!("{u}@{}", ssh.host),
        None => ssh.host.clone(),
    };
    let mut cmd = Command::new(program);
    cmd.args([
        "-N",
        "-o",
        "BatchMode=yes",
        "-o",
        "StrictHostKeyChecking=yes",
        "-o",
        "ExitOnForwardFailure=yes",
        "-o",
        "ServerAliveInterval=30",
        "-p",
        &ssh.port.to_string(),
        "-L",
        &format!("127.0.0.1:{local_port}:{db_host}:{db_port}"),
        "--",
        &destination,
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| CoreError::Rejected(format!("cannot start ssh: {e}")))?;
    let addr = SocketAddr::from(([127, 0, 0, 1], local_port));
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            let mut err = String::new();
            if let Some(mut e) = child.stderr.take() {
                use std::io::Read;
                let _ = e.read_to_string(&mut err);
            }
            let err = err.trim();
            return Err(CoreError::Rejected(format!(
                "SSH tunnel failed ({status}){}{}",
                if err.is_empty() { "" } else { ": " },
                err.chars().take(400).collect::<String>()
            )));
        }
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CoreError::Rejected(
                "SSH tunnel did not open within 15 seconds".into(),
            ));
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    // Drop the stderr pipe so a chatty ssh can never block on a full buffer.
    drop(child.stderr.take());
    map.insert(
        id.to_string(),
        Tunnel {
            child,
            local_port,
            key,
        },
    );
    Ok(local_port)
}

/// Close the tunnel for `id`, if one is open.
pub fn close_tunnel(id: &str) {
    if let Ok(mut map) = tunnels().lock() {
        if let Some(mut t) = map.remove(id) {
            let _ = t.child.kill();
            let _ = t.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jump_hosts() {
        assert_eq!(
            parse_ssh_target("deploy@bastion.example.com:2222").unwrap(),
            SshTarget {
                user: Some("deploy".into()),
                host: "bastion.example.com".into(),
                port: 2222
            }
        );
        assert_eq!(parse_ssh_target("bastion").unwrap().port, 22);
        for bad in [
            "-oProxyCommand=x",
            "a@-b",
            "a b@c",
            "u@h:0",
            "u@h:x",
            "",
            "u@",
        ] {
            assert!(parse_ssh_target(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn finds_and_rewrites_endpoints() {
        let url = "mysql://app:p%40ss@db.internal:3307/app?ssl=1";
        assert_eq!(
            connection_endpoint("mysql", url).unwrap(),
            ("db.internal".into(), 3307)
        );
        assert_eq!(
            rewrite_endpoint(url, 40001),
            "mysql://app:p%40ss@127.0.0.1:40001/app?ssl=1"
        );

        assert_eq!(
            connection_endpoint("redis", "redis://:pw@cache").unwrap(),
            ("cache".into(), 6379)
        );
        assert_eq!(rewrite_endpoint("redis://cache", 9), "redis://127.0.0.1:9");

        let pq = "host=db.internal user=pg password=s dbname=app";
        assert_eq!(
            connection_endpoint("postgres", pq).unwrap(),
            ("db.internal".into(), 5432)
        );
        assert_eq!(
            rewrite_endpoint(pq, 5),
            "host=127.0.0.1 port=5 user=pg password=s dbname=app"
        );

        assert!(connection_endpoint("mongo", "mongodb://a:1,b:2/x").is_err());
    }
}
