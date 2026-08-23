use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use vsn_security::{IpcAuthenticator, SecurityError};

pub const IPC_ADDRESS: &str = "127.0.0.1:39731";
pub const PROTOCOL_VERSION: u32 = 1;
const MAX_CLOCK_SKEW_MS: u128 = 30_000;
const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_CONNECTIONS: usize = 128;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const NONCE_HEX_LEN: usize = 48;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("security error: {0}")]
    Security(#[from] SecurityError),
    #[error("authentication failed")]
    Authentication,
    #[error("request expired or clock skew too large")]
    Expired,
    #[error("replayed request")]
    Replay,
    #[error("replay window capacity exceeded")]
    ReplayWindowSaturated,
    #[error("unsupported protocol version")]
    ProtocolVersion,
    #[error("frame exceeds maximum size")]
    FrameTooLarge,
    #[error("agent response did not match request")]
    ResponseMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub version: u32,
    pub timestamp_unix_ms: u128,
    pub nonce: String,
    pub command: String,
    #[serde(default)]
    pub params: Value,
    pub mac: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub version: u32,
    pub timestamp_unix_ms: u128,
    pub request_nonce: String,
    pub ok: bool,
    pub payload: Value,
    pub mac: String,
}

impl RequestEnvelope {
    pub fn new(command: impl Into<String>, params: Value, auth: &IpcAuthenticator) -> Self {
        let mut envelope = Self {
            version: PROTOCOL_VERSION,
            timestamp_unix_ms: now_ms(),
            nonce: random_nonce(),
            command: command.into(),
            params,
            mac: String::new(),
        };
        envelope.mac = auth.sign(&envelope.canonical_bytes());
        envelope
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut fields = BTreeMap::new();
        fields.insert("command", Value::String(self.command.clone()));
        fields.insert("nonce", Value::String(self.nonce.clone()));
        fields.insert("params", canonical_json_value(&self.params));
        fields.insert("timestamp_unix_ms", json!(self.timestamp_unix_ms));
        fields.insert("version", Value::from(self.version));
        serde_json::to_vec(&fields).expect("serializing request canonical form cannot fail")
    }
}

impl ResponseEnvelope {
    pub fn new(request_nonce: String, ok: bool, payload: Value, auth: &IpcAuthenticator) -> Self {
        let mut envelope = Self {
            version: PROTOCOL_VERSION,
            timestamp_unix_ms: now_ms(),
            request_nonce,
            ok,
            payload,
            mac: String::new(),
        };
        envelope.mac = auth.sign(&envelope.canonical_bytes());
        envelope
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut fields = BTreeMap::new();
        fields.insert("ok", Value::Bool(self.ok));
        fields.insert("payload", canonical_json_value(&self.payload));
        fields.insert("request_nonce", Value::String(self.request_nonce.clone()));
        fields.insert("timestamp_unix_ms", json!(self.timestamp_unix_ms));
        fields.insert("version", Value::from(self.version));
        serde_json::to_vec(&fields).expect("serializing response canonical form cannot fail")
    }
}

#[derive(Clone)]
pub struct RequestGuard {
    auth: IpcAuthenticator,
    nonces: Arc<Mutex<ReplayCache>>,
}

impl RequestGuard {
    pub fn new(auth: IpcAuthenticator) -> Self {
        Self {
            auth,
            nonces: Arc::new(Mutex::new(ReplayCache::new(2048))),
        }
    }

    pub fn verify(&self, request: &RequestEnvelope) -> Result<(), IpcError> {
        if request.version != PROTOCOL_VERSION {
            return Err(IpcError::ProtocolVersion);
        }
        if !valid_nonce(&request.nonce)
            || request.command.is_empty()
            || request.command.len() > 128
            || !request.command.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            })
        {
            return Err(IpcError::Authentication);
        }
        let now = now_ms();
        let skew = now.abs_diff(request.timestamp_unix_ms);
        if skew > MAX_CLOCK_SKEW_MS {
            return Err(IpcError::Expired);
        }
        if !self.auth.verify(&request.canonical_bytes(), &request.mac) {
            return Err(IpcError::Authentication);
        }
        let mut cache = self.nonces.lock().map_err(|_| IpcError::Authentication)?;
        match cache.insert(request.nonce.clone(), request.timestamp_unix_ms, now) {
            ReplayInsert::Inserted => Ok(()),
            ReplayInsert::Duplicate => Err(IpcError::Replay),
            ReplayInsert::Saturated => Err(IpcError::ReplayWindowSaturated),
        }
    }

    pub fn authenticator(&self) -> &IpcAuthenticator {
        &self.auth
    }
}

pub fn serve<F>(handler: F) -> Result<(), IpcError>
where
    F: Fn(RequestEnvelope) -> (bool, Value) + Send + Sync + 'static,
{
    serve_until(Arc::new(AtomicBool::new(false)), handler)
}

pub fn serve_until<F>(stop: Arc<AtomicBool>, handler: F) -> Result<(), IpcError>
where
    F: Fn(RequestEnvelope) -> (bool, Value) + Send + Sync + 'static,
{
    let auth = IpcAuthenticator::load_or_create()?;
    let listener = TcpListener::bind(IPC_ADDRESS)?;
    listener.set_nonblocking(true)?;
    let guard = RequestGuard::new(auth);
    let handler = Arc::new(handler);
    let active = Arc::new(AtomicUsize::new(0));

    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                if active.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                    active.fetch_sub(1, Ordering::SeqCst);
                    drop(stream);
                    continue;
                }
                let guard = guard.clone();
                let handler = handler.clone();
                let active = active.clone();
                thread::spawn(move || {
                    let _connection_slot = ConnectionSlot(active);
                    if let Err(error) = handle_connection(stream, &guard, handler.as_ref()) {
                        eprintln!("ipc_connection_error={error}");
                    }
                });
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(IpcError::Io(error)),
        }
    }
    Ok(())
}

pub fn call(command: &str, params: Value) -> Result<ResponseEnvelope, IpcError> {
    let mut stream = TcpStream::connect_timeout(
        &IPC_ADDRESS.parse().expect("static socket address"),
        Duration::from_secs(2),
    )?;
    let auth = IpcAuthenticator::load_or_create()?;
    let request = RequestEnvelope::new(command, params, &auth);
    let expected_nonce = request.nonce.clone();
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut encoded = serde_json::to_vec(&request)?;
    encoded.push(b'\n');
    stream.write_all(&encoded)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let line = read_bounded_line(&mut reader)?;
    let response: ResponseEnvelope = serde_json::from_str(&line)?;
    if response.request_nonce != expected_nonce {
        return Err(IpcError::ResponseMismatch);
    }
    if response.version != PROTOCOL_VERSION {
        return Err(IpcError::ProtocolVersion);
    }
    if now_ms().abs_diff(response.timestamp_unix_ms) > MAX_CLOCK_SKEW_MS {
        return Err(IpcError::Expired);
    }
    if !auth.verify(&response.canonical_bytes(), &response.mac) {
        return Err(IpcError::Authentication);
    }
    Ok(response)
}

struct ConnectionSlot(Arc<AtomicUsize>);
impl Drop for ConnectionSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn handle_connection<F>(
    mut stream: TcpStream,
    guard: &RequestGuard,
    handler: &F,
) -> Result<(), IpcError>
where
    F: Fn(RequestEnvelope) -> (bool, Value),
{
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(CONNECTION_TIMEOUT))?;
    stream.set_write_timeout(Some(CONNECTION_TIMEOUT))?;
    if !stream.peer_addr()?.ip().is_loopback() {
        return Err(IpcError::Authentication);
    }
    let cloned = stream.try_clone()?;
    let mut reader = BufReader::new(cloned);
    let line = read_bounded_line(&mut reader)?;
    if line.is_empty() {
        return Ok(());
    }
    let request: RequestEnvelope = serde_json::from_str(&line)?;
    let nonce = request.nonce.clone();
    let result = guard.verify(&request);
    let response = match result {
        Ok(()) => {
            let (ok, payload) = handler(request);
            ResponseEnvelope::new(nonce, ok, payload, guard.authenticator())
        }
        Err(error) => ResponseEnvelope::new(
            nonce,
            false,
            json!({ "error": error.to_string() }),
            guard.authenticator(),
        ),
    };
    let mut encoded = serde_json::to_vec(&response)?;
    encoded.push(b'\n');
    stream.write_all(&encoded)?;
    stream.flush()?;
    Ok(())
}

fn canonical_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_unstable();
            let mut canonical = serde_json::Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonical_json_value(&map[key]));
            }
            Value::Object(canonical)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json_value).collect()),
        _ => value.clone(),
    }
}

fn read_bounded_line<R: BufRead>(reader: &mut R) -> Result<String, IpcError> {
    let mut output = Vec::new();
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.map(|index| index + 1).unwrap_or(buffer.len());
        if output.len().saturating_add(take) > MAX_FRAME_BYTES {
            return Err(IpcError::FrameTooLarge);
        }
        output.extend_from_slice(&buffer[..take]);
        reader.consume(take);
        if newline.is_some() {
            break;
        }
    }
    String::from_utf8(output)
        .map_err(|error| IpcError::Io(io::Error::new(io::ErrorKind::InvalidData, error)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayInsert {
    Inserted,
    Duplicate,
    Saturated,
}

#[derive(Debug)]
struct ReplayCache {
    capacity: usize,
    entries: HashMap<String, u128>,
}

impl ReplayCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
        }
    }

    fn insert(&mut self, nonce: String, timestamp: u128, now: u128) -> ReplayInsert {
        self.entries.retain(|_, stored_timestamp| {
            stored_timestamp.saturating_add(MAX_CLOCK_SKEW_MS) >= now
        });
        if self.entries.contains_key(&nonce) {
            return ReplayInsert::Duplicate;
        }
        if self.entries.len() >= self.capacity {
            return ReplayInsert::Saturated;
        }
        self.entries.insert(nonce, timestamp);
        ReplayInsert::Inserted
    }
}

fn valid_nonce(nonce: &str) -> bool {
    nonce.len() == NONCE_HEX_LEN && nonce.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn random_nonce() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_envelope_bytes_are_feature_order_independent() {
        let mut nested = serde_json::Map::new();
        nested.insert("d".into(), json!(4));
        nested.insert("c".into(), json!(3));
        let mut params = serde_json::Map::new();
        params.insert("z".into(), json!(1));
        params.insert("a".into(), Value::Object(nested));

        let request = RequestEnvelope {
            version: 1,
            timestamp_unix_ms: 123,
            nonce: "00112233445566778899aabbccddeeff0011223344556677".into(),
            command: "status".into(),
            params: Value::Object(params),
            mac: String::new(),
        };
        assert_eq!(
            String::from_utf8(request.canonical_bytes()).unwrap(),
            "{\"command\":\"status\",\"nonce\":\"00112233445566778899aabbccddeeff0011223344556677\",\"params\":{\"a\":{\"c\":3,\"d\":4},\"z\":1},\"timestamp_unix_ms\":123,\"version\":1}"
        );

        let response = ResponseEnvelope {
            version: 1,
            timestamp_unix_ms: 456,
            request_nonce: request.nonce.clone(),
            ok: true,
            payload: json!({"z": 9, "a": {"d": 4, "c": 3}}),
            mac: String::new(),
        };
        assert_eq!(
            String::from_utf8(response.canonical_bytes()).unwrap(),
            "{\"ok\":true,\"payload\":{\"a\":{\"c\":3,\"d\":4},\"z\":9},\"request_nonce\":\"00112233445566778899aabbccddeeff0011223344556677\",\"timestamp_unix_ms\":456,\"version\":1}"
        );
    }

    #[test]
    fn nonce_format_requires_24_bytes_of_hex_entropy() {
        let nonce = random_nonce();
        assert!(valid_nonce(&nonce));
        assert_eq!(nonce.len(), NONCE_HEX_LEN);
        assert!(!valid_nonce(""));
        assert!(!valid_nonce("abcd"));
        assert!(!valid_nonce(&"g".repeat(NONCE_HEX_LEN)));
    }

    #[test]
    fn replay_cache_blocks_duplicate_nonce_inside_live_window() {
        let mut cache = ReplayCache::new(2);
        assert_eq!(cache.insert("a".into(), 100, 100), ReplayInsert::Inserted);
        assert_eq!(cache.insert("a".into(), 100, 101), ReplayInsert::Duplicate);
    }

    #[test]
    fn replay_window_fails_closed_when_saturated_and_releases_expired_entries() {
        let mut cache = ReplayCache::new(2);
        assert_eq!(cache.insert("a".into(), 1, 1), ReplayInsert::Inserted);
        assert_eq!(cache.insert("b".into(), 1, 1), ReplayInsert::Inserted);
        assert_eq!(cache.insert("c".into(), 1, 1), ReplayInsert::Saturated);

        let after_expiry = MAX_CLOCK_SKEW_MS + 2;
        assert_eq!(
            cache.insert("c".into(), after_expiry, after_expiry),
            ReplayInsert::Inserted
        );
    }

    #[test]
    fn bounded_reader_rejects_oversized_frame() {
        let oversized = vec![b'x'; MAX_FRAME_BYTES + 1];
        let mut reader = std::io::Cursor::new(oversized);
        assert!(matches!(
            read_bounded_line(&mut reader),
            Err(IpcError::FrameTooLarge)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn accepted_nonblocking_stream_waits_for_delayed_request() {
        let auth = IpcAuthenticator::load_or_create().expect("load IPC authenticator");
        let guard = RequestGuard::new(auth);
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
        listener
            .set_nonblocking(true)
            .expect("make test listener nonblocking");
        let address = listener.local_addr().expect("resolve test address");
        let mut client = TcpStream::connect(address).expect("connect test client");
        client
            .set_write_timeout(Some(Duration::from_secs(2)))
            .expect("set test client timeout");

        let accepted = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("accept test connection: {error}"),
            }
        };
        accepted
            .set_nonblocking(true)
            .expect("model inherited nonblocking accepted stream");

        let writer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            client.write_all(b"\n").expect("write delayed request");
            client.flush().expect("flush delayed request");
        });

        let started = std::time::Instant::now();
        handle_connection(accepted, &guard, &|_| {
            panic!("empty request must not reach handler")
        })
        .expect("accepted stream should wait instead of returning WouldBlock");
        assert!(
            started.elapsed() >= Duration::from_millis(75),
            "handler returned before delayed request became readable"
        );
        writer.join().expect("delayed request writer");
    }
}
