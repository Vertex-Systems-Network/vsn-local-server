//! VSN Devify LAN sharing (DVF-06): open a local dev server on the Wi-Fi/LAN so a phone or
//! another computer can reach it. Dev servers usually listen on 127.0.0.1 only, so VSN
//! listens on all interfaces and forwards each connection to `127.0.0.1:<target>`.
//! Every share expires; nothing stays exposed after the app forgets about it.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEFAULT_LIFETIME: Duration = Duration::from_secs(2 * 60 * 60);
pub const MAX_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
const MAX_SHARES: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    #[error("invalid share request: {0}")]
    Invalid(String),
    #[error("share io error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShareInfo {
    pub id: u64,
    pub target_port: u16,
    pub listen_port: u16,
    /// http://<lan-ip>:<listen_port>/ when a LAN address is known.
    pub url: Option<String>,
    pub lan_ip: Option<String>,
    pub expires_at_unix: u64,
}

struct Share {
    info: ShareInfo,
    stop: Arc<AtomicBool>,
}

fn registry() -> &'static Mutex<BTreeMap<u64, Share>> {
    static REG: OnceLock<Mutex<BTreeMap<u64, Share>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(BTreeMap::new()))
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The machine's primary LAN address: the source address the OS would use to reach the
/// internet. `connect` on UDP sends no packets.
pub fn lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

/// Most simultaneous connections one share serves; more are refused (flood guard).
const MAX_CONNECTIONS: usize = 64;
/// A connection with no traffic for this long is closed (slowloris guard).
const IDLE_TIMEOUT: Duration = Duration::from_secs(120);

/// Only devices on the same private network may connect: RFC 1918, link-local,
/// loopback and IPv6 unique-local addresses. Anything routed from the internet
/// (e.g. through a forwarded router port) is dropped before a byte is read.
pub fn is_local_peer(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() || v4.is_link_local() || v4.is_loopback(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_local_peer(IpAddr::V4(v4));
            }
            let seg = v6.segments()[0];
            v6.is_loopback() || (seg & 0xfe00) == 0xfc00 || (seg & 0xffc0) == 0xfe80
        }
    }
}

fn pipe(mut from: TcpStream, mut to: TcpStream, live: Arc<std::sync::atomic::AtomicUsize>) {
    let _ = from.set_read_timeout(Some(IDLE_TIMEOUT));
    let _ = io::copy(&mut from, &mut to);
    let _ = to.shutdown(Shutdown::Write);
    live.fetch_sub(1, Ordering::SeqCst);
}

fn serve(listener: TcpListener, target_port: u16, stop: Arc<AtomicBool>, expires_at: u64) {
    let _ = listener.set_nonblocking(true);
    // Each accepted connection runs two pipe threads; this counts pipe threads.
    let live = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    while !stop.load(Ordering::SeqCst) && now_unix() < expires_at {
        match listener.accept() {
            Ok((client, peer)) => {
                if !is_local_peer(peer.ip()) || live.load(Ordering::SeqCst) >= MAX_CONNECTIONS * 2 {
                    let _ = client.shutdown(Shutdown::Both);
                    continue;
                }
                let _ = client.set_nonblocking(false);
                let target = SocketAddr::from(([127, 0, 0, 1], target_port));
                match TcpStream::connect_timeout(&target, Duration::from_secs(5)) {
                    Ok(upstream) => {
                        if let (Ok(c2), Ok(u2)) = (client.try_clone(), upstream.try_clone()) {
                            live.fetch_add(2, Ordering::SeqCst);
                            let (l1, l2) = (Arc::clone(&live), Arc::clone(&live));
                            std::thread::spawn(move || pipe(client, upstream, l1));
                            std::thread::spawn(move || pipe(u2, c2, l2));
                        }
                    }
                    Err(_) => {
                        let _ = client.shutdown(Shutdown::Both);
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100))
            }
            Err(_) => std::thread::sleep(Duration::from_millis(250)),
        }
    }
    if let Ok(mut reg) = registry().lock() {
        reg.retain(|_, s| !Arc::ptr_eq(&s.stop, &stop));
    }
}

/// Share `127.0.0.1:target_port` on the LAN. `listen_port` 0 picks a free port.
pub fn start(
    target_port: u16,
    listen_port: u16,
    lifetime: Duration,
) -> Result<ShareInfo, ShareError> {
    if target_port == 0 {
        return Err(ShareError::Invalid("target port must be non-zero".into()));
    }
    if lifetime.is_zero() || lifetime > MAX_LIFETIME {
        return Err(ShareError::Invalid(
            "lifetime must be between 1 second and 24 hours".into(),
        ));
    }
    if listen_port != 0 && listen_port < 1024 {
        return Err(ShareError::Invalid(
            "listen port must be 1024 or higher".into(),
        ));
    }
    {
        let reg = registry()
            .lock()
            .map_err(|_| ShareError::Invalid("share registry poisoned".into()))?;
        if reg.len() >= MAX_SHARES {
            return Err(ShareError::Invalid(format!(
                "at most {MAX_SHARES} shares at a time"
            )));
        }
    }
    let listener = TcpListener::bind(("0.0.0.0", listen_port))?;
    let listen_port = listener.local_addr()?.port();
    let ip = lan_ip();
    let expires_at_unix = now_unix() + lifetime.as_secs();
    let info = ShareInfo {
        id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
        target_port,
        listen_port,
        url: ip.map(|ip| format!("http://{ip}:{listen_port}/")),
        lan_ip: ip.map(|ip| ip.to_string()),
        expires_at_unix,
    };
    let stop = Arc::new(AtomicBool::new(false));
    registry()
        .lock()
        .map_err(|_| ShareError::Invalid("share registry poisoned".into()))?
        .insert(
            info.id,
            Share {
                info: info.clone(),
                stop: Arc::clone(&stop),
            },
        );
    std::thread::spawn(move || serve(listener, target_port, stop, expires_at_unix));
    Ok(info)
}

pub fn stop(id: u64) -> bool {
    let removed = registry().lock().ok().and_then(|mut reg| reg.remove(&id));
    match removed {
        Some(share) => {
            share.stop.store(true, Ordering::SeqCst);
            true
        }
        None => false,
    }
}

pub fn list() -> Vec<ShareInfo> {
    let now = now_unix();
    registry()
        .lock()
        .map(|reg| {
            reg.values()
                .filter(|s| s.info.expires_at_unix > now)
                .map(|s| s.info.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Pull the public URL out of `cloudflared` quick-tunnel output
/// (`https://<words>.trycloudflare.com`).
pub fn parse_quick_tunnel_url(log: &str) -> Option<String> {
    log.split(|c: char| c.is_whitespace() || c == '|' || c == '"')
        .find(|w| {
            w.starts_with("https://")
                && w.ends_with(".trycloudflare.com")
                && w["https://".len()..]
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_private_network_peers() {
        for ok in [
            "192.168.1.9",
            "10.0.0.2",
            "172.20.1.1",
            "169.254.3.3",
            "127.0.0.1",
            "fe80::1",
            "fd00::5",
            "::ffff:192.168.0.4",
        ] {
            assert!(is_local_peer(ok.parse().unwrap()), "{ok}");
        }
        for bad in [
            "8.8.8.8",
            "100.64.1.1",
            "203.0.113.5",
            "2001:db8::1",
            "::ffff:1.1.1.1",
        ] {
            assert!(!is_local_peer(bad.parse().unwrap()), "{bad}");
        }
    }
    use std::io::{Read, Write};

    #[test]
    fn forwards_lan_connections_to_local_port_and_stops() {
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        let target = upstream.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = upstream.accept() {
                let mut buf = [0u8; 5];
                s.read_exact(&mut buf).unwrap();
                s.write_all(b"pong:").unwrap();
                s.write_all(&buf).unwrap();
            }
        });
        let share = start(target, 0, Duration::from_secs(60)).unwrap();
        assert!(list().iter().any(|s| s.id == share.id));
        let mut client = TcpStream::connect(("127.0.0.1", share.listen_port)).unwrap();
        client.write_all(b"hello").unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        let mut reply = String::new();
        client.read_to_string(&mut reply).unwrap();
        assert_eq!(reply, "pong:hello");
        assert!(stop(share.id));
        assert!(!list().iter().any(|s| s.id == share.id));
        assert!(!stop(share.id));
    }

    #[test]
    fn finds_quick_tunnel_url_in_cloudflared_log() {
        let log = "2026-10-17T23:50:01Z INF |  Your quick Tunnel has been created! Visit it at (it may take some time to be reachable):  |\n2026-10-17T23:50:01Z INF |  https://brave-lion-tiny-sky.trycloudflare.com                                        |\n";
        assert_eq!(
            parse_quick_tunnel_url(log).as_deref(),
            Some("https://brave-lion-tiny-sky.trycloudflare.com")
        );
        assert_eq!(
            parse_quick_tunnel_url("https://api.trycloudflare.com/ok"),
            None
        );
        assert_eq!(parse_quick_tunnel_url("no url yet"), None);
    }

    #[test]
    fn rejects_bad_requests() {
        assert!(start(0, 0, Duration::from_secs(60)).is_err());
        assert!(start(8000, 80, Duration::from_secs(60)).is_err());
        assert!(start(8000, 0, Duration::from_secs(0)).is_err());
        assert!(start(8000, 0, MAX_LIFETIME + Duration::from_secs(1)).is_err());
    }
}
