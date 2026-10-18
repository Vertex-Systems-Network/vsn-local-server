//! Memcached over its text protocol, limited to loopback servers (a space's own cache).

use crate::NativeDbError;
use serde_json::{json, Map, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(8);
const MAX_VALUE: usize = 1024 * 1024;
const SAMPLE_KEYS: usize = 200;

fn err<E: std::fmt::Display>(e: E) -> NativeDbError {
    NativeDbError::Memcached(e.to_string())
}

/// `host:port` that resolves only to loopback addresses.
fn loopback(address: &str) -> Result<SocketAddr, NativeDbError> {
    let addrs: Vec<SocketAddr> = address
        .to_socket_addrs()
        .map_err(|e| NativeDbError::Invalid(format!("Memcached address: {e}")))?
        .collect();
    match addrs.first() {
        Some(a) if addrs.iter().all(|a| a.ip().is_loopback()) => Ok(*a),
        _ => Err(NativeDbError::Invalid(
            "Memcached must run on this machine (127.0.0.1, ::1 or localhost)".into(),
        )),
    }
}

/// Memcached keys: 1–250 bytes, no spaces or control characters.
pub(crate) fn valid_key(key: &str) -> Result<(), NativeDbError> {
    if key.is_empty() || key.len() > 250 || key.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        Err(NativeDbError::Invalid(
            "Memcached keys are 1–250 bytes without spaces or control characters".into(),
        ))
    } else {
        Ok(())
    }
}

struct Conn {
    reader: BufReader<TcpStream>,
}
impl Conn {
    fn open(address: &str) -> Result<Self, NativeDbError> {
        let stream = TcpStream::connect_timeout(&loopback(address)?, TIMEOUT).map_err(err)?;
        stream.set_read_timeout(Some(TIMEOUT)).map_err(err)?;
        stream.set_write_timeout(Some(TIMEOUT)).map_err(err)?;
        Ok(Self {
            reader: BufReader::new(stream),
        })
    }
    fn send(&mut self, bytes: &[u8]) -> Result<(), NativeDbError> {
        self.reader.get_mut().write_all(bytes).map_err(err)
    }
    fn line(&mut self) -> Result<String, NativeDbError> {
        let mut line = String::new();
        let n = (&mut self.reader)
            .take(64 * 1024)
            .read_line(&mut line)
            .map_err(err)?;
        if n == 0 {
            return Err(err("connection closed"));
        }
        let line = line.trim_end_matches(['\r', '\n']).to_string();
        if line == "ERROR" || line.starts_with("CLIENT_ERROR") || line.starts_with("SERVER_ERROR") {
            return Err(err(line));
        }
        Ok(line)
    }
}

pub fn inspect(address: &str) -> Result<Value, NativeDbError> {
    let mut c = Conn::open(address)?;
    c.send(b"stats\r\n")?;
    let mut stats = Map::new();
    loop {
        let line = c.line()?;
        if line == "END" {
            break;
        }
        if let Some(rest) = line.strip_prefix("STAT ") {
            if let Some((k, v)) = rest.split_once(' ') {
                stats.insert(k.to_string(), Value::String(v.to_string()));
            }
        }
    }
    // Key listing needs the LRU crawler (memcached 1.4.31+); older servers just give no sample.
    let mut keys = Vec::new();
    c.send(b"lru_crawler metadump all\r\n")?;
    while let Ok(line) = c.line() {
        if line == "END" {
            break;
        }
        if keys.len() < SAMPLE_KEYS {
            if let Some(key) = line
                .split(' ')
                .find_map(|f| f.strip_prefix("key="))
                .map(percent_decode)
            {
                keys.push(key);
            }
        }
    }
    Ok(json!({
        "address": address,
        "version": stats.get("version").cloned().unwrap_or(Value::Null),
        "items": stats.get("curr_items").cloned().unwrap_or(Value::Null),
        "bytes": stats.get("bytes").cloned().unwrap_or(Value::Null),
        "hits": stats.get("get_hits").cloned().unwrap_or(Value::Null),
        "misses": stats.get("get_misses").cloned().unwrap_or(Value::Null),
        "sample_keys": keys,
        "stats": stats,
    }))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn get(address: &str, key: &str) -> Result<Value, NativeDbError> {
    valid_key(key)?;
    let mut c = Conn::open(address)?;
    c.send(format!("get {key}\r\n").as_bytes())?;
    let head = c.line()?;
    if head == "END" {
        return Ok(json!({"key":key,"found":false,"value":null}));
    }
    // VALUE <key> <flags> <bytes>
    let len: usize = head
        .split(' ')
        .nth(3)
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| err(format!("unexpected reply: {head}")))?;
    if len > MAX_VALUE {
        return Err(NativeDbError::Invalid(
            "Memcached value is larger than the 1 MiB view limit".into(),
        ));
    }
    let mut data = vec![0u8; len + 2];
    c.reader.read_exact(&mut data).map_err(err)?;
    data.truncate(len);
    let _ = c.line(); // END
    Ok(json!({"key":key,"found":true,"bytes":len,"value":String::from_utf8_lossy(&data)}))
}

pub fn set(
    address: &str,
    key: &str,
    value: &str,
    ttl_seconds: Option<u64>,
) -> Result<Value, NativeDbError> {
    valid_key(key)?;
    if value.len() > MAX_VALUE {
        return Err(NativeDbError::Invalid(
            "Memcached values are limited to 1 MiB".into(),
        ));
    }
    let ttl = ttl_seconds.unwrap_or(0);
    if ttl > 30 * 24 * 60 * 60 {
        return Err(NativeDbError::Invalid(
            "Memcached TTL must be at most 30 days".into(),
        ));
    }
    let mut c = Conn::open(address)?;
    c.send(format!("set {key} 0 {ttl} {}\r\n", value.len()).as_bytes())?;
    c.send(value.as_bytes())?;
    c.send(b"\r\n")?;
    let reply = c.line()?;
    Ok(json!({"key":key,"ok":reply=="STORED","ttl_seconds":ttl_seconds}))
}

pub fn delete(address: &str, key: &str) -> Result<Value, NativeDbError> {
    valid_key(key)?;
    let mut c = Conn::open(address)?;
    c.send(format!("delete {key}\r\n").as_bytes())?;
    let reply = c.line()?;
    Ok(json!({"key":key,"deleted":u64::from(reply=="DELETED")}))
}

pub fn flush(address: &str) -> Result<Value, NativeDbError> {
    let mut c = Conn::open(address)?;
    c.send(b"flush_all\r\n")?;
    let reply = c.line()?;
    Ok(json!({"flushed":reply=="OK"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn rejects_non_loopback_and_bad_keys() {
        assert!(loopback("10.0.0.5:11211").is_err());
        assert!(loopback("127.0.0.1:11211").is_ok());
        assert!(valid_key("ok:key").is_ok());
        assert!(valid_key("has space").is_err());
        assert!(valid_key(&"k".repeat(251)).is_err());
        assert_eq!(percent_decode("user%3A1"), "user:1");
    }

    #[test]
    fn speaks_the_text_protocol() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server = std::thread::spawn(move || {
            for reply in [
                &b"STORED\r\n"[..],
                b"VALUE k 0 5\r\nhello\r\nEND\r\n",
                b"DELETED\r\n",
            ] {
                let (mut s, _) = listener.accept().unwrap();
                let mut buf = [0u8; 256];
                let _ = s.read(&mut buf).unwrap();
                s.write_all(reply).unwrap();
            }
        });
        assert_eq!(set(&addr, "k", "hello", Some(60)).unwrap()["ok"], true);
        let got = get(&addr, "k").unwrap();
        assert_eq!(got["value"], "hello");
        assert_eq!(got["bytes"], 5);
        assert_eq!(delete(&addr, "k").unwrap()["deleted"], 1);
        server.join().unwrap();
    }
}
