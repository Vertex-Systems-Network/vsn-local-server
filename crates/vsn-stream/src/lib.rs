use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub const STREAM_PROTOCOL_VERSION: u32 = 1;
pub const MAX_STREAMS: usize = 128;
pub const MAX_FRAME_BYTES: usize = 256 * 1024;
pub const MAX_BUFFER_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_PULL_FRAMES: usize = 64;

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("stream request rejected: {0}")]
    Invalid(String),
    #[error("stream not found")]
    NotFound,
    #[error("stream closed")]
    Closed,
    #[error("stream backpressure limit exceeded")]
    Backpressure,
    #[error("stream registry unavailable")]
    Poisoned,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Terminal,
    FileUpload,
    FileDownload,
    Database,
    Preview,
    Logs,
    Custom,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamDirection {
    ClientToAgent,
    AgentToClient,
    Bidirectional,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamOpenRequest {
    pub kind: StreamKind,
    pub direction: StreamDirection,
    pub resource_id: String,
    pub metadata: HashMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamState {
    pub version: u32,
    pub stream_id: String,
    pub kind: StreamKind,
    pub direction: StreamDirection,
    pub resource_id: String,
    pub opened_at_unix_ms: u128,
    pub last_activity_unix_ms: u128,
    pub next_in_seq: u64,
    pub next_out_seq: u64,
    pub buffered_bytes: usize,
    pub buffered_input_bytes: usize,
    pub buffered_output_bytes: usize,
    pub closed: bool,
    pub close_reason: Option<String>,
    pub metadata: HashMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamFrame {
    pub version: u32,
    pub stream_id: String,
    pub seq: u64,
    pub eof: bool,
    pub payload_base64: String,
    pub timestamp_unix_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamPull {
    pub stream: StreamState,
    pub frames: Vec<StreamFrame>,
}
struct Entry {
    state: StreamState,
    inbound: VecDeque<StreamFrame>,
    outbound: VecDeque<StreamFrame>,
}
static REGISTRY: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
fn registry() -> &'static Mutex<HashMap<String, Entry>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn validate_open_request(request: &StreamOpenRequest) -> Result<(), StreamError> {
    validate_resource(&request.resource_id)?;
    if request.metadata.len() > 32 {
        return Err(StreamError::Invalid(
            "stream metadata exceeds 32 entries".into(),
        ));
    }
    for (k, v) in &request.metadata {
        if k.is_empty()
            || k.len() > 64
            || v.len() > 1024
            || k.chars().any(char::is_control)
            || v.chars().any(char::is_control)
        {
            return Err(StreamError::Invalid("invalid stream metadata".into()));
        }
    }
    Ok(())
}
pub fn open_stream(request: StreamOpenRequest) -> Result<StreamState, StreamError> {
    open_stream_at(request, 0, 0)
}
pub fn open_stream_at(
    request: StreamOpenRequest,
    next_in_seq: u64,
    next_out_seq: u64,
) -> Result<StreamState, StreamError> {
    validate_open_request(&request)?;
    let mut map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    map.retain(|_, e| {
        !e.state.closed || now_ms().saturating_sub(e.state.last_activity_unix_ms) < 60_000
    });
    if map.len() >= MAX_STREAMS {
        return Err(StreamError::Invalid("stream limit reached".into()));
    }
    let id = random_id();
    let now = now_ms();
    let state = StreamState {
        version: STREAM_PROTOCOL_VERSION,
        stream_id: id.clone(),
        kind: request.kind,
        direction: request.direction,
        resource_id: request.resource_id,
        opened_at_unix_ms: now,
        last_activity_unix_ms: now,
        next_in_seq,
        next_out_seq,
        buffered_bytes: 0,
        buffered_input_bytes: 0,
        buffered_output_bytes: 0,
        closed: false,
        close_reason: None,
        metadata: request.metadata,
    };
    map.insert(
        id,
        Entry {
            state: state.clone(),
            inbound: VecDeque::new(),
            outbound: VecDeque::new(),
        },
    );
    Ok(state)
}
pub fn accept_input_frame(
    stream_id: &str,
    seq: u64,
    payload_base64: &str,
    eof: bool,
) -> Result<StreamState, StreamError> {
    let bytes = B64
        .decode(payload_base64)
        .map_err(|_| StreamError::Invalid("payload_base64 is invalid".into()))?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(StreamError::Invalid("stream frame exceeds 256 KiB".into()));
    }
    let mut map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    let e = map.get_mut(stream_id).ok_or(StreamError::NotFound)?;
    if e.state.closed {
        return Err(StreamError::Closed);
    }
    if matches!(e.state.direction, StreamDirection::AgentToClient) {
        return Err(StreamError::Invalid(
            "stream direction rejects client input".into(),
        ));
    }
    if seq != e.state.next_in_seq {
        return Err(StreamError::Invalid(format!(
            "input sequence mismatch: expected {}, got {seq}",
            e.state.next_in_seq
        )));
    }
    if e.state.buffered_bytes.saturating_add(bytes.len()) > MAX_BUFFER_BYTES {
        return Err(StreamError::Backpressure);
    }
    let now = now_ms();
    let frame = StreamFrame {
        version: STREAM_PROTOCOL_VERSION,
        stream_id: stream_id.into(),
        seq,
        eof,
        payload_base64: payload_base64.into(),
        timestamp_unix_ms: now,
    };
    e.inbound.push_back(frame);
    e.state.next_in_seq = e.state.next_in_seq.saturating_add(1);
    e.state.last_activity_unix_ms = now;
    e.state.buffered_input_bytes = e.state.buffered_input_bytes.saturating_add(bytes.len());
    e.state.buffered_bytes = e
        .state
        .buffered_input_bytes
        .saturating_add(e.state.buffered_output_bytes);
    if eof {
        e.state.closed = true;
        e.state.close_reason = Some("peer_eof".into());
    }
    Ok(e.state.clone())
}
pub fn queue_output(
    stream_id: &str,
    payload: &[u8],
    eof: bool,
) -> Result<StreamFrame, StreamError> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(StreamError::Invalid("stream frame exceeds 256 KiB".into()));
    }
    let mut map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    let e = map.get_mut(stream_id).ok_or(StreamError::NotFound)?;
    if e.state.closed && !eof {
        return Err(StreamError::Closed);
    }
    if matches!(e.state.direction, StreamDirection::ClientToAgent) {
        return Err(StreamError::Invalid(
            "stream direction rejects agent output".into(),
        ));
    }
    if e.state.buffered_bytes.saturating_add(payload.len()) > MAX_BUFFER_BYTES {
        return Err(StreamError::Backpressure);
    }
    let now = now_ms();
    let frame = StreamFrame {
        version: STREAM_PROTOCOL_VERSION,
        stream_id: stream_id.into(),
        seq: e.state.next_out_seq,
        eof,
        payload_base64: B64.encode(payload),
        timestamp_unix_ms: now,
    };
    e.state.next_out_seq = e.state.next_out_seq.saturating_add(1);
    e.state.last_activity_unix_ms = now;
    e.state.buffered_output_bytes = e.state.buffered_output_bytes.saturating_add(payload.len());
    e.state.buffered_bytes = e
        .state
        .buffered_input_bytes
        .saturating_add(e.state.buffered_output_bytes);
    if eof {
        e.state.closed = true;
        e.state.close_reason = Some("agent_eof".into());
    }
    e.outbound.push_back(frame.clone());
    Ok(frame)
}
pub fn pull_output(stream_id: &str, max_frames: usize) -> Result<StreamPull, StreamError> {
    let max = max_frames.clamp(1, MAX_PULL_FRAMES);
    let mut map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    let e = map.get_mut(stream_id).ok_or(StreamError::NotFound)?;
    let mut frames = Vec::new();
    for _ in 0..max {
        let Some(frame) = e.outbound.pop_front() else {
            break;
        };
        let len = B64
            .decode(&frame.payload_base64)
            .map(|v| v.len())
            .unwrap_or(0);
        e.state.buffered_output_bytes = e.state.buffered_output_bytes.saturating_sub(len);
        e.state.buffered_bytes = e
            .state
            .buffered_input_bytes
            .saturating_add(e.state.buffered_output_bytes);
        frames.push(frame);
    }
    e.state.last_activity_unix_ms = now_ms();
    Ok(StreamPull {
        stream: e.state.clone(),
        frames,
    })
}

pub fn pull_input(stream_id: &str, max_frames: usize) -> Result<StreamPull, StreamError> {
    let max = max_frames.clamp(1, MAX_PULL_FRAMES);
    let mut map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    let e = map.get_mut(stream_id).ok_or(StreamError::NotFound)?;
    let mut frames = Vec::new();
    for _ in 0..max {
        let Some(frame) = e.inbound.pop_front() else {
            break;
        };
        let len = B64
            .decode(&frame.payload_base64)
            .map(|v| v.len())
            .unwrap_or(0);
        e.state.buffered_input_bytes = e.state.buffered_input_bytes.saturating_sub(len);
        e.state.buffered_bytes = e
            .state
            .buffered_input_bytes
            .saturating_add(e.state.buffered_output_bytes);
        frames.push(frame);
    }
    e.state.last_activity_unix_ms = now_ms();
    Ok(StreamPull {
        stream: e.state.clone(),
        frames,
    })
}
pub fn close_stream(stream_id: &str, reason: Option<&str>) -> Result<StreamState, StreamError> {
    let mut map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    let e = map.get_mut(stream_id).ok_or(StreamError::NotFound)?;
    e.state.closed = true;
    e.state.last_activity_unix_ms = now_ms();
    e.state.close_reason = Some(reason.unwrap_or("closed").chars().take(256).collect());
    Ok(e.state.clone())
}
pub fn stream_state(stream_id: &str) -> Result<StreamState, StreamError> {
    let map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    Ok(map
        .get(stream_id)
        .ok_or(StreamError::NotFound)?
        .state
        .clone())
}
pub fn list_streams() -> Result<Vec<StreamState>, StreamError> {
    let map = registry().lock().map_err(|_| StreamError::Poisoned)?;
    let mut out = map.values().map(|e| e.state.clone()).collect::<Vec<_>>();
    out.sort_by_key(|a| a.opened_at_unix_ms);
    Ok(out)
}
fn validate_resource(v: &str) -> Result<(), StreamError> {
    if v.is_empty() || v.len() > 512 || v.chars().any(|c| c.is_control() || c == '\0') {
        Err(StreamError::Invalid("invalid stream resource_id".into()))
    } else {
        Ok(())
    }
}
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
fn random_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    format!(
        "stream_{:x}_{:x}",
        now_ms(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stream_sequences_and_backpressure() {
        let s = open_stream(StreamOpenRequest {
            kind: StreamKind::Terminal,
            direction: StreamDirection::Bidirectional,
            resource_id: "pty-1".into(),
            metadata: HashMap::new(),
        })
        .unwrap();
        accept_input_frame(&s.stream_id, 0, &B64.encode(b"hi"), false).unwrap();
        assert!(accept_input_frame(&s.stream_id, 0, &B64.encode(b"bad"), false).is_err());
        let input = pull_input(&s.stream_id, 8).unwrap();
        assert_eq!(input.frames.len(), 1);
        queue_output(&s.stream_id, b"out", false).unwrap();
        let p = pull_output(&s.stream_id, 8).unwrap();
        assert_eq!(p.frames.len(), 1);
    }
}
