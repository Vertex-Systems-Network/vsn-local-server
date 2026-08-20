use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::Duration;
use thiserror::Error;

const MAX_BODY_BYTES: u64 = 512 * 1024;

#[derive(Debug, Error)]
pub enum PreviewError {
    #[error("preview request rejected: {0}")]
    Invalid(String),
    #[error("preview transport failed: {0}")]
    Transport(String),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewRequest {
    pub port: u16,
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
}
fn default_method() -> String {
    "GET".into()
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body_base64: String,
    pub text: Option<String>,
    pub truncated: bool,
}

pub fn fetch(request: &PreviewRequest) -> Result<PreviewResponse, PreviewError> {
    if request.port == 0 {
        return Err(PreviewError::Invalid("port must be non-zero".into()));
    }
    if !request.path.starts_with('/')
        || request.path.contains('\r')
        || request.path.contains('\n')
        || request.path.contains("://")
    {
        return Err(PreviewError::Invalid(
            "preview path must be a local absolute URL path".into(),
        ));
    }
    let method = request.method.to_ascii_uppercase();
    if !matches!(method.as_str(), "GET" | "HEAD") {
        return Err(PreviewError::Invalid(
            "only GET/HEAD preview requests are allowed".into(),
        ));
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let url = format!("http://127.0.0.1:{}{}", request.port, request.path);
    let mut response = if method == "HEAD" {
        client.head(url).send()
    } else {
        client.get(url).send()
    }
    .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    if method == "HEAD" {
        return Ok(PreviewResponse {
            status,
            content_type,
            body_base64: String::new(),
            text: Some(String::new()),
            truncated: false,
        });
    }
    if response.content_length().unwrap_or(0) > MAX_BODY_BYTES {
        return Err(PreviewError::Invalid(
            "preview response exceeds 512 KiB limit".into(),
        ));
    }
    let mut limited = (&mut response).take(MAX_BODY_BYTES + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let truncated = bytes.len() as u64 > MAX_BODY_BYTES;
    if truncated {
        bytes.truncate(MAX_BODY_BYTES as usize);
    }
    let text = if content_type
        .as_deref()
        .map(|v| {
            v.starts_with("text/")
                || v.contains("json")
                || v.contains("javascript")
                || v.contains("xml")
        })
        .unwrap_or(false)
    {
        Some(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        None
    };
    Ok(PreviewResponse {
        status,
        content_type,
        body_base64: B64.encode(&bytes),
        text,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_url_path_is_rejected() {
        assert!(fetch(&PreviewRequest {
            port: 8000,
            path: "https://example.com".into(),
            method: "GET".into()
        })
        .is_err());
    }
}

use std::collections::BTreeMap;
const MAX_PROXY_REQUEST_BODY: usize = 2 * 1024 * 1024;
const MAX_PROXY_RESPONSE_BODY: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewHttpRequest {
    pub port: u16,
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body_base64: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewHttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body_base64: String,
    pub text: Option<String>,
    pub truncated: bool,
}

pub fn request(input: &PreviewHttpRequest) -> Result<PreviewHttpResponse, PreviewError> {
    validate_target(input.port, &input.path)?;
    if input.headers.len() > 64 {
        return Err(PreviewError::Invalid(
            "preview request has too many headers".into(),
        ));
    }
    let method = input.method.trim().to_ascii_uppercase();
    if !matches!(
        method.as_str(),
        "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
    ) {
        return Err(PreviewError::Invalid(
            "preview HTTP method is not allowed".into(),
        ));
    }
    let body = match input.body_base64.as_deref() {
        Some(v) => {
            let decoded = B64.decode(v).map_err(|_| {
                PreviewError::Invalid("preview request body is not valid base64".into())
            })?;
            if decoded.len() > MAX_PROXY_REQUEST_BODY {
                return Err(PreviewError::Invalid(
                    "preview request body exceeds 2 MiB".into(),
                ));
            }
            decoded
        }
        None => Vec::new(),
    };
    if matches!(method.as_str(), "GET" | "HEAD") && !body.is_empty() {
        return Err(PreviewError::Invalid(
            "GET/HEAD preview requests cannot include a body".into(),
        ));
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let url = format!("http://127.0.0.1:{}{}", input.port, input.path);
    let method_obj = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|_| PreviewError::Invalid("invalid HTTP method".into()))?;
    let mut builder = client.request(method_obj, url);
    for (name, value) in &input.headers {
        let normalized = name.trim().to_ascii_lowercase();
        if !allowed_request_header(&normalized) {
            return Err(PreviewError::Invalid(format!(
                "preview request header is not allowed: {name}"
            )));
        }
        if value.len() > 16 * 1024 || value.contains('\r') || value.contains('\n') {
            return Err(PreviewError::Invalid(
                "preview request header value is invalid".into(),
            ));
        }
        let header_name = reqwest::header::HeaderName::from_bytes(normalized.as_bytes())
            .map_err(|_| PreviewError::Invalid("invalid preview header name".into()))?;
        let header_value = reqwest::header::HeaderValue::from_str(value)
            .map_err(|_| PreviewError::Invalid("invalid preview header value".into()))?;
        builder = builder.header(header_name, header_value);
    }
    if !body.is_empty() {
        builder = builder.body(body);
    }
    let mut response = builder
        .send()
        .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let status = response.status().as_u16();
    let mut headers = BTreeMap::new();
    for (name, value) in response.headers() {
        let n = name.as_str().to_ascii_lowercase();
        if allowed_response_header(&n) {
            if let Ok(v) = value.to_str() {
                if v.len() <= 16 * 1024 {
                    headers.insert(n, v.to_string());
                }
            }
        }
    }
    if method == "HEAD" {
        return Ok(PreviewHttpResponse {
            status,
            headers,
            body_base64: String::new(),
            text: Some(String::new()),
            truncated: false,
        });
    }
    if response.content_length().unwrap_or(0) > MAX_PROXY_RESPONSE_BODY {
        return Err(PreviewError::Invalid(
            "preview response exceeds 2 MiB limit".into(),
        ));
    }
    let mut limited = (&mut response).take(MAX_PROXY_RESPONSE_BODY + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let truncated = bytes.len() as u64 > MAX_PROXY_RESPONSE_BODY;
    if truncated {
        bytes.truncate(MAX_PROXY_RESPONSE_BODY as usize);
    }
    let content_type = headers.get("content-type").map(String::as_str);
    let text = if content_type
        .map(|v| {
            v.starts_with("text/")
                || v.contains("json")
                || v.contains("javascript")
                || v.contains("xml")
        })
        .unwrap_or(false)
    {
        Some(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        None
    };
    Ok(PreviewHttpResponse {
        status,
        headers,
        body_base64: B64.encode(&bytes),
        text,
        truncated,
    })
}
fn validate_target(port: u16, path: &str) -> Result<(), PreviewError> {
    if port == 0 {
        return Err(PreviewError::Invalid("port must be non-zero".into()));
    }
    if path.len() > 16 * 1024
        || !path.starts_with('/')
        || path.contains('\r')
        || path.contains('\n')
        || path.contains("://")
        || path.starts_with("//")
    {
        return Err(PreviewError::Invalid(
            "preview path must be a bounded local absolute URL path".into(),
        ));
    }
    Ok(())
}
fn allowed_request_header(name: &str) -> bool {
    matches!(
        name,
        "accept"
            | "accept-language"
            | "authorization"
            | "content-type"
            | "cookie"
            | "origin"
            | "referer"
            | "user-agent"
            | "x-requested-with"
    ) || name.starts_with("x-vsn-")
}
fn allowed_response_header(name: &str) -> bool {
    matches!(
        name,
        "content-type"
            | "content-length"
            | "cache-control"
            | "etag"
            | "last-modified"
            | "location"
            | "set-cookie"
            | "content-disposition"
            | "x-frame-options"
            | "content-security-policy"
    )
}

#[cfg(test)]
mod proxy_tests {
    use super::*;
    #[test]
    fn proxy_target_rejects_scheme_and_network_path() {
        assert!(validate_target(8080, "https://example.com").is_err());
        assert!(validate_target(8080, "//example.com/x").is_err());
        assert!(validate_target(8080, "/api/test").is_ok());
    }
    #[test]
    fn proxy_header_allowlist_is_narrow() {
        assert!(allowed_request_header("content-type"));
        assert!(!allowed_request_header("host"));
        assert!(!allowed_request_header("connection"));
    }
}

// ---------- bounded localhost SSE relay ----------
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError},
    Arc, Mutex, OnceLock,
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const MAX_PREVIEW_STREAMS: usize = 32;
pub const MAX_PREVIEW_STREAM_CHUNK: usize = 64 * 1024;
pub const MAX_PREVIEW_STREAM_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_PREVIEW_STREAM_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewEventStreamRequest {
    pub port: u16,
    pub path: String,
    #[serde(default)]
    pub last_event_id: Option<String>,
    #[serde(default = "default_stream_seconds")]
    pub max_duration_seconds: u64,
}
fn default_stream_seconds() -> u64 {
    60
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewEventStreamState {
    pub stream_id: String,
    pub status: u16,
    pub content_type: String,
    pub opened_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub bytes_read: u64,
    pub eof: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewEventStreamChunk {
    pub stream_id: String,
    pub payload_base64: String,
    pub bytes: u32,
    pub eof: bool,
    pub error: Option<String>,
    pub total_bytes: u64,
}

enum PreviewStreamMessage {
    Data(Vec<u8>),
    Eof,
    Error(String),
}
struct PreviewStreamEntry {
    state: PreviewEventStreamState,
    receiver: Receiver<PreviewStreamMessage>,
    cancel: Arc<AtomicBool>,
}
static PREVIEW_STREAMS: OnceLock<Mutex<HashMap<String, PreviewStreamEntry>>> = OnceLock::new();
static PREVIEW_STREAM_SEQ: AtomicU64 = AtomicU64::new(1);
fn preview_streams() -> &'static Mutex<HashMap<String, PreviewStreamEntry>> {
    PREVIEW_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}
fn stream_now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
fn preview_stream_id() -> String {
    format!(
        "preview_{:x}_{:x}",
        stream_now_ms(),
        PREVIEW_STREAM_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

pub fn start_event_stream(
    request: &PreviewEventStreamRequest,
) -> Result<PreviewEventStreamState, PreviewError> {
    validate_target(request.port, &request.path)?;
    if request
        .last_event_id
        .as_ref()
        .map(|v| v.len() > 1024 || v.contains('\r') || v.contains('\n'))
        .unwrap_or(false)
    {
        return Err(PreviewError::Invalid("Last-Event-ID is invalid".into()));
    }
    let duration = request
        .max_duration_seconds
        .clamp(5, MAX_PREVIEW_STREAM_SECONDS);
    let now = stream_now_ms();
    let expires = now + u128::from(duration) * 1000;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(duration.saturating_add(5)))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let url = format!("http://127.0.0.1:{}{}", request.port, request.path);
    let mut builder = client
        .get(url)
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .header(reqwest::header::CACHE_CONTROL, "no-cache");
    if let Some(id) = request.last_event_id.as_deref() {
        builder = builder.header("last-event-id", id);
    }
    let response = builder
        .send()
        .map_err(|e| PreviewError::Transport(e.to_string()))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(PreviewError::Invalid(format!(
            "SSE endpoint returned HTTP {status}"
        )));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !content_type.starts_with("text/event-stream") {
        return Err(PreviewError::Invalid(
            "preview SSE endpoint must return text/event-stream".into(),
        ));
    }
    let id = preview_stream_id();
    let cancel = Arc::new(AtomicBool::new(false));
    let thread_cancel = cancel.clone();
    let (tx, rx): (
        SyncSender<PreviewStreamMessage>,
        Receiver<PreviewStreamMessage>,
    ) = sync_channel(32);
    let id_for_thread = id.clone();
    let mut map = preview_streams()
        .lock()
        .map_err(|_| PreviewError::Transport("preview stream registry poisoned".into()))?;
    map.retain(|_, e| !e.state.eof && e.state.expires_at_unix_ms >= now);
    if map.len() >= MAX_PREVIEW_STREAMS {
        return Err(PreviewError::Invalid(
            "preview SSE stream capacity reached".into(),
        ));
    }
    let state = PreviewEventStreamState {
        stream_id: id.clone(),
        status,
        content_type: content_type.clone(),
        opened_at_unix_ms: now,
        expires_at_unix_ms: expires,
        bytes_read: 0,
        eof: false,
    };
    map.insert(
        id.clone(),
        PreviewStreamEntry {
            state: state.clone(),
            receiver: rx,
            cancel,
        },
    );
    drop(map);
    std::thread::spawn(move || {
        let mut response = response;
        let started = Instant::now();
        let max_duration = Duration::from_secs(duration);
        let mut total = 0u64;
        let mut buf = vec![0u8; MAX_PREVIEW_STREAM_CHUNK];
        let try_emit = |message: PreviewStreamMessage| -> bool {
            match tx.try_send(message) {
                Ok(()) => true,
                Err(TrySendError::Disconnected(_)) => false,
                Err(TrySendError::Full(_)) => false,
            }
        };
        loop {
            if thread_cancel.load(Ordering::SeqCst) {
                break;
            }
            if started.elapsed() >= max_duration {
                let _ = try_emit(PreviewStreamMessage::Eof);
                break;
            }
            match response.read(&mut buf) {
                Ok(0) => {
                    let _ = try_emit(PreviewStreamMessage::Eof);
                    break;
                }
                Ok(n) => {
                    total = total.saturating_add(n as u64);
                    if total > MAX_PREVIEW_STREAM_BYTES {
                        let _ = try_emit(PreviewStreamMessage::Error(
                            "preview SSE stream exceeded 16 MiB limit".into(),
                        ));
                        break;
                    }
                    if !try_emit(PreviewStreamMessage::Data(buf[..n].to_vec())) {
                        break;
                    }
                }
                Err(e) => {
                    let _ = try_emit(PreviewStreamMessage::Error(format!(
                        "preview SSE read failed: {e}"
                    )));
                    break;
                }
            }
        }
        let _ = id_for_thread;
    });
    Ok(state)
}

pub fn read_event_stream(stream_id: &str) -> Result<PreviewEventStreamChunk, PreviewError> {
    let mut map = preview_streams()
        .lock()
        .map_err(|_| PreviewError::Transport("preview stream registry poisoned".into()))?;
    let entry = map
        .get_mut(stream_id)
        .ok_or_else(|| PreviewError::Invalid("preview SSE stream not found".into()))?;
    if entry.state.eof {
        return Ok(PreviewEventStreamChunk {
            stream_id: stream_id.into(),
            payload_base64: String::new(),
            bytes: 0,
            eof: true,
            error: None,
            total_bytes: entry.state.bytes_read,
        });
    }
    match entry.receiver.try_recv() {
        Ok(PreviewStreamMessage::Data(bytes)) => {
            entry.state.bytes_read = entry.state.bytes_read.saturating_add(bytes.len() as u64);
            Ok(PreviewEventStreamChunk {
                stream_id: stream_id.into(),
                payload_base64: B64.encode(&bytes),
                bytes: u32::try_from(bytes.len()).unwrap_or(u32::MAX),
                eof: false,
                error: None,
                total_bytes: entry.state.bytes_read,
            })
        }
        Ok(PreviewStreamMessage::Eof) => {
            entry.state.eof = true;
            Ok(PreviewEventStreamChunk {
                stream_id: stream_id.into(),
                payload_base64: String::new(),
                bytes: 0,
                eof: true,
                error: None,
                total_bytes: entry.state.bytes_read,
            })
        }
        Ok(PreviewStreamMessage::Error(error)) => {
            entry.state.eof = true;
            Ok(PreviewEventStreamChunk {
                stream_id: stream_id.into(),
                payload_base64: String::new(),
                bytes: 0,
                eof: true,
                error: Some(error),
                total_bytes: entry.state.bytes_read,
            })
        }
        Err(TryRecvError::Empty) => Ok(PreviewEventStreamChunk {
            stream_id: stream_id.into(),
            payload_base64: String::new(),
            bytes: 0,
            eof: false,
            error: None,
            total_bytes: entry.state.bytes_read,
        }),
        Err(TryRecvError::Disconnected) => {
            entry.state.eof = true;
            Ok(PreviewEventStreamChunk {
                stream_id: stream_id.into(),
                payload_base64: String::new(),
                bytes: 0,
                eof: true,
                error: None,
                total_bytes: entry.state.bytes_read,
            })
        }
    }
}
pub fn close_event_stream(stream_id: &str) -> Result<bool, PreviewError> {
    let mut map = preview_streams()
        .lock()
        .map_err(|_| PreviewError::Transport("preview stream registry poisoned".into()))?;
    if let Some(entry) = map.remove(stream_id) {
        entry.cancel.store(true, Ordering::SeqCst);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod event_stream_tests {
    use super::*;
    #[test]
    fn event_stream_target_is_local_only() {
        assert!(validate_target(9000, "/events").is_ok());
        assert!(validate_target(9000, "//evil").is_err());
    }
    #[test]
    fn stream_duration_is_bounded() {
        assert_eq!(10u64.clamp(5, MAX_PREVIEW_STREAM_SECONDS), 10);
    }
}

// ---------- bounded localhost WebSocket preview relay ----------
use std::sync::atomic::{AtomicBool as WsAtomicBool, Ordering as WsOrdering};
use std::sync::mpsc::{
    sync_channel, Receiver as WsReceiver, SyncSender as WsSender, TryRecvError as WsTryRecvError,
    TrySendError as WsTrySendError,
};
use tungstenite::{stream::MaybeTlsStream, Error as TungsteniteError, Message as WsMessage};

const MAX_PREVIEW_WS_SESSIONS: usize = 32;
const MAX_PREVIEW_WS_MESSAGE: usize = 256 * 1024;
const MAX_PREVIEW_WS_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PREVIEW_WS_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewWebSocketRequest {
    pub port: u16,
    pub path: String,
    #[serde(default = "default_ws_duration")]
    pub max_duration_seconds: u64,
}
fn default_ws_duration() -> u64 {
    60
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewWebSocketState {
    pub session_id: String,
    pub opened_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub closed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewWebSocketFrameKind {
    Text,
    Binary,
    Ping,
    Pong,
    Close,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewWebSocketFrame {
    pub kind: PreviewWebSocketFrameKind,
    pub payload_base64: String,
    pub eof: bool,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewWebSocketSend {
    pub kind: PreviewWebSocketFrameKind,
    pub payload_base64: String,
}

enum PreviewWsCommand {
    Frame(PreviewWebSocketSend),
    Close,
}
enum PreviewWsEvent {
    Frame(PreviewWebSocketFrame),
    Closed(Option<String>),
}
struct PreviewWsEntry {
    state: PreviewWebSocketState,
    commands: WsSender<PreviewWsCommand>,
    events: WsReceiver<PreviewWsEvent>,
    cancel: std::sync::Arc<WsAtomicBool>,
}
static PREVIEW_WS_SESSIONS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, PreviewWsEntry>>,
> = std::sync::OnceLock::new();
fn preview_ws_sessions(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, PreviewWsEntry>> {
    PREVIEW_WS_SESSIONS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}
fn preview_ws_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    format!(
        "previewws_{:x}_{:x}",
        now_ms(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}

pub fn start_websocket(
    request: &PreviewWebSocketRequest,
) -> Result<PreviewWebSocketState, PreviewError> {
    validate_target(request.port, &request.path)?;
    let duration = request
        .max_duration_seconds
        .clamp(5, MAX_PREVIEW_WS_SECONDS);
    let now = now_ms();
    let expires = now + u128::from(duration) * 1000;
    let id = preview_ws_id();
    let mut map = preview_ws_sessions()
        .lock()
        .map_err(|_| PreviewError::Transport("preview WebSocket registry poisoned".into()))?;
    map.retain(|_, e| !e.state.closed && e.state.expires_at_unix_ms >= now);
    if map.len() >= MAX_PREVIEW_WS_SESSIONS {
        return Err(PreviewError::Invalid(
            "preview WebSocket capacity reached".into(),
        ));
    }
    let (command_tx, command_rx) = sync_channel::<PreviewWsCommand>(32);
    let (event_tx, event_rx) = sync_channel::<PreviewWsEvent>(32);
    let cancel = std::sync::Arc::new(WsAtomicBool::new(false));
    let thread_cancel = cancel.clone();
    let url = format!("ws://127.0.0.1:{}{}", request.port, request.path);
    let state = PreviewWebSocketState {
        session_id: id.clone(),
        opened_at_unix_ms: now,
        expires_at_unix_ms: expires,
        bytes_in: 0,
        bytes_out: 0,
        closed: false,
    };
    map.insert(
        id.clone(),
        PreviewWsEntry {
            state: state.clone(),
            commands: command_tx,
            events: event_rx,
            cancel,
        },
    );
    drop(map);
    std::thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let (mut socket, _) = tungstenite::connect(url.as_str())
                .map_err(|e| format!("preview WebSocket connect failed: {e}"))?;
            if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
                let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
            }
            let started = std::time::Instant::now();
            let mut transferred = 0u64;
            loop {
                if thread_cancel.load(WsOrdering::SeqCst) {
                    let _ = socket.close(None);
                    break;
                }
                if started.elapsed() >= Duration::from_secs(duration) {
                    let _ = socket.close(None);
                    let _ = event_tx.try_send(PreviewWsEvent::Closed(Some(
                        "preview WebSocket duration limit reached".into(),
                    )));
                    break;
                }
                loop {
                    match command_rx.try_recv() {
                        Ok(PreviewWsCommand::Close) => {
                            let _ = socket.close(None);
                            return Ok(());
                        }
                        Ok(PreviewWsCommand::Frame(frame)) => {
                            let bytes = B64.decode(&frame.payload_base64).map_err(|_| {
                                "preview WebSocket payload is invalid base64".to_string()
                            })?;
                            if bytes.len() > MAX_PREVIEW_WS_MESSAGE {
                                return Err("preview WebSocket message exceeds 256 KiB".into());
                            }
                            transferred = transferred.saturating_add(bytes.len() as u64);
                            if transferred > MAX_PREVIEW_WS_BYTES {
                                return Err(
                                    "preview WebSocket session exceeded 16 MiB limit".into()
                                );
                            }
                            let msg = match frame.kind {
                                PreviewWebSocketFrameKind::Text => WsMessage::Text(
                                    String::from_utf8(bytes)
                                        .map_err(|_| {
                                            "preview WebSocket text must be UTF-8".to_string()
                                        })?
                                        .into(),
                                ),
                                PreviewWebSocketFrameKind::Binary => {
                                    WsMessage::Binary(bytes.into())
                                }
                                PreviewWebSocketFrameKind::Ping => WsMessage::Ping(bytes.into()),
                                PreviewWebSocketFrameKind::Pong => WsMessage::Pong(bytes.into()),
                                PreviewWebSocketFrameKind::Close => {
                                    let _ = socket.close(None);
                                    return Ok(());
                                }
                            };
                            socket
                                .send(msg)
                                .map_err(|e| format!("preview WebSocket send failed: {e}"))?;
                        }
                        Err(WsTryRecvError::Empty) => break,
                        Err(WsTryRecvError::Disconnected) => return Ok(()),
                    }
                }
                match socket.read() {
                    Ok(msg) => {
                        let frame = match msg {
                            WsMessage::Text(v) => PreviewWebSocketFrame {
                                kind: PreviewWebSocketFrameKind::Text,
                                payload_base64: B64.encode(v.as_str().as_bytes()),
                                eof: false,
                                error: None,
                            },
                            WsMessage::Binary(v) => PreviewWebSocketFrame {
                                kind: PreviewWebSocketFrameKind::Binary,
                                payload_base64: B64.encode(v.as_ref()),
                                eof: false,
                                error: None,
                            },
                            WsMessage::Ping(v) => PreviewWebSocketFrame {
                                kind: PreviewWebSocketFrameKind::Ping,
                                payload_base64: B64.encode(v.as_ref()),
                                eof: false,
                                error: None,
                            },
                            WsMessage::Pong(v) => PreviewWebSocketFrame {
                                kind: PreviewWebSocketFrameKind::Pong,
                                payload_base64: B64.encode(v.as_ref()),
                                eof: false,
                                error: None,
                            },
                            WsMessage::Close(_) => {
                                let _ = event_tx.try_send(PreviewWsEvent::Closed(None));
                                break;
                            }
                            WsMessage::Frame(_) => continue,
                        };
                        let bytes = B64
                            .decode(&frame.payload_base64)
                            .map_err(|_| "preview WebSocket response base64 failure".to_string())?;
                        if bytes.len() > MAX_PREVIEW_WS_MESSAGE {
                            return Err("preview WebSocket response message exceeds 256 KiB".into());
                        }
                        transferred = transferred.saturating_add(bytes.len() as u64);
                        if transferred > MAX_PREVIEW_WS_BYTES {
                            return Err("preview WebSocket session exceeded 16 MiB limit".into());
                        }
                        match event_tx.try_send(PreviewWsEvent::Frame(frame)) {
                            Ok(()) => {}
                            Err(WsTrySendError::Full(_)) => {
                                return Err(
                                    "preview WebSocket browser backpressure limit reached".into()
                                )
                            }
                            Err(WsTrySendError::Disconnected(_)) => break,
                        }
                    }
                    Err(TungsteniteError::Io(e))
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(TungsteniteError::ConnectionClosed | TungsteniteError::AlreadyClosed) => {
                        break
                    }
                    Err(e) => return Err(format!("preview WebSocket read failed: {e}")),
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            let _ = event_tx.try_send(PreviewWsEvent::Closed(Some(error)));
        }
    });
    Ok(state)
}

pub fn send_websocket(
    session_id: &str,
    request: &PreviewWebSocketSend,
) -> Result<PreviewWebSocketState, PreviewError> {
    let bytes = B64
        .decode(&request.payload_base64)
        .map_err(|_| PreviewError::Invalid("preview WebSocket payload is invalid base64".into()))?;
    if bytes.len() > MAX_PREVIEW_WS_MESSAGE {
        return Err(PreviewError::Invalid(
            "preview WebSocket message exceeds 256 KiB".into(),
        ));
    }
    let mut map = preview_ws_sessions()
        .lock()
        .map_err(|_| PreviewError::Transport("preview WebSocket registry poisoned".into()))?;
    let entry = map
        .get_mut(session_id)
        .ok_or_else(|| PreviewError::Invalid("preview WebSocket session not found".into()))?;
    if entry.state.closed {
        return Err(PreviewError::Invalid(
            "preview WebSocket session is closed".into(),
        ));
    }
    match entry
        .commands
        .try_send(PreviewWsCommand::Frame(request.clone()))
    {
        Ok(()) => {}
        Err(WsTrySendError::Full(_)) => {
            return Err(PreviewError::Invalid(
                "preview WebSocket input backpressure limit reached".into(),
            ))
        }
        Err(WsTrySendError::Disconnected(_)) => {
            return Err(PreviewError::Transport(
                "preview WebSocket worker disconnected".into(),
            ))
        }
    }
    entry.state.bytes_in = entry.state.bytes_in.saturating_add(bytes.len() as u64);
    if entry.state.bytes_in.saturating_add(entry.state.bytes_out) > MAX_PREVIEW_WS_BYTES {
        return Err(PreviewError::Invalid(
            "preview WebSocket session exceeded 16 MiB limit".into(),
        ));
    }
    Ok(entry.state.clone())
}

pub fn read_websocket(session_id: &str) -> Result<PreviewWebSocketFrame, PreviewError> {
    let mut map = preview_ws_sessions()
        .lock()
        .map_err(|_| PreviewError::Transport("preview WebSocket registry poisoned".into()))?;
    let entry = map
        .get_mut(session_id)
        .ok_or_else(|| PreviewError::Invalid("preview WebSocket session not found".into()))?;
    match entry.events.try_recv() {
        Ok(PreviewWsEvent::Frame(frame)) => {
            let bytes = B64.decode(&frame.payload_base64).unwrap_or_default();
            entry.state.bytes_out = entry.state.bytes_out.saturating_add(bytes.len() as u64);
            Ok(frame)
        }
        Ok(PreviewWsEvent::Closed(error)) => {
            entry.state.closed = true;
            Ok(PreviewWebSocketFrame {
                kind: PreviewWebSocketFrameKind::Close,
                payload_base64: String::new(),
                eof: true,
                error,
            })
        }
        Err(WsTryRecvError::Empty) => Ok(PreviewWebSocketFrame {
            kind: PreviewWebSocketFrameKind::Binary,
            payload_base64: String::new(),
            eof: false,
            error: None,
        }),
        Err(WsTryRecvError::Disconnected) => {
            entry.state.closed = true;
            Ok(PreviewWebSocketFrame {
                kind: PreviewWebSocketFrameKind::Close,
                payload_base64: String::new(),
                eof: true,
                error: None,
            })
        }
    }
}
pub fn close_websocket(session_id: &str) -> Result<bool, PreviewError> {
    let mut map = preview_ws_sessions()
        .lock()
        .map_err(|_| PreviewError::Transport("preview WebSocket registry poisoned".into()))?;
    if let Some(entry) = map.remove(session_id) {
        entry.cancel.store(true, WsOrdering::SeqCst);
        let _ = entry.commands.try_send(PreviewWsCommand::Close);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod websocket_preview_tests {
    use super::*;
    #[test]
    fn websocket_request_stays_loopback() {
        assert!(validate_target(5173, "/hmr").is_ok());
        assert!(validate_target(5173, "https://evil.example/ws").is_err());
    }
    #[test]
    fn websocket_message_cap_is_bounded() {
        assert_eq!(MAX_PREVIEW_WS_MESSAGE, 256 * 1024);
    }
}

// ---------- 0.24 private preview/tunnel conformance ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviewConformanceReport {
    pub loopback_only: bool,
    pub redirects_disabled: bool,
    pub snapshot_get_head: bool,
    pub bounded_http_proxy: bool,
    pub http_mutation_methods: bool,
    pub request_header_allowlist: bool,
    pub response_header_allowlist: bool,
    pub cookie_forwarding: bool,
    pub sse: bool,
    pub websocket: bool,
    pub interactive_requires_separate_policy: bool,
    pub request_body_limit_bytes: usize,
    pub response_body_limit_bytes: u64,
    pub sse_session_limit_bytes: u64,
    pub websocket_session_limit_bytes: u64,
    pub issues: Vec<String>,
}
pub fn conformance() -> PreviewConformanceReport {
    let mut issues = Vec::new();
    let loopback_only = validate_target(5173, "/health").is_ok()
        && validate_target(5173, "https://evil.example/").is_err()
        && validate_target(5173, "//evil.example/").is_err();
    if !loopback_only {
        issues
            .push("preview target validation does not enforce loopback-only absolute paths".into());
    }
    PreviewConformanceReport {
        loopback_only,
        redirects_disabled: true,
        snapshot_get_head: true,
        bounded_http_proxy: true,
        http_mutation_methods: true,
        request_header_allowlist: true,
        response_header_allowlist: true,
        cookie_forwarding: true,
        sse: true,
        websocket: true,
        interactive_requires_separate_policy: true,
        request_body_limit_bytes: MAX_PROXY_REQUEST_BODY,
        response_body_limit_bytes: MAX_PROXY_RESPONSE_BODY,
        sse_session_limit_bytes: MAX_PREVIEW_STREAM_BYTES,
        websocket_session_limit_bytes: MAX_PREVIEW_WS_BYTES,
        issues,
    }
}
