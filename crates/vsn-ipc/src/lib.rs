mod base {
    include!("lib_base.rs");
}

pub use base::*;

use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::{self, BufRead, BufReader, Write},
    net::TcpStream,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use vsn_security::IpcAuthenticator;

// Keep this explicit in the public facade because long-running callers must still obey the
// authenticated IPC framing contract retained in lib_base.rs.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
const CALL_MAX_CLOCK_SKEW_MS: u128 = 30_000;
const DEFAULT_CALL_READ_TIMEOUT: Duration = Duration::from_secs(5);
const TERMINAL_EXEC_CALL_READ_TIMEOUT: Duration = Duration::from_secs(35);
const MAX_CALL_READ_TIMEOUT: Duration = Duration::from_secs(65);
const MIN_CALL_READ_TIMEOUT: Duration = Duration::from_millis(100);
const CALL_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const CALL_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

fn bounded_call_read_timeout(requested: Duration) -> Duration {
    requested.clamp(MIN_CALL_READ_TIMEOUT, MAX_CALL_READ_TIMEOUT)
}

fn call_read_timeout_for_command(command: &str) -> Duration {
    if command == "terminal.exec" {
        TERMINAL_EXEC_CALL_READ_TIMEOUT
    } else {
        DEFAULT_CALL_READ_TIMEOUT
    }
}

pub fn call(command: &str, params: Value) -> Result<ResponseEnvelope, IpcError> {
    call_with_timeout(command, params, call_read_timeout_for_command(command))
}

pub fn call_with_timeout(
    command: &str,
    params: Value,
    read_timeout: Duration,
) -> Result<ResponseEnvelope, IpcError> {
    let mut stream = TcpStream::connect_timeout(
        &IPC_ADDRESS.parse().expect("static socket address"),
        CALL_CONNECT_TIMEOUT,
    )?;
    let auth = IpcAuthenticator::load_or_create()?;
    let request = RequestEnvelope::new(command, params, &auth);
    let expected_nonce = request.nonce.clone();
    stream.set_read_timeout(Some(bounded_call_read_timeout(read_timeout)))?;
    stream.set_write_timeout(Some(CALL_WRITE_TIMEOUT))?;

    let mut encoded = serde_json::to_vec(&request)?;
    encoded.push(b'\n');
    if encoded.len() > MAX_FRAME_BYTES {
        return Err(IpcError::FrameTooLarge);
    }
    stream.write_all(&encoded)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let line = read_facade_bounded_line(&mut reader)?;
    let response: ResponseEnvelope = serde_json::from_str(&line)?;
    if response.request_nonce != expected_nonce {
        return Err(IpcError::ResponseMismatch);
    }
    if response.version != PROTOCOL_VERSION {
        return Err(IpcError::ProtocolVersion);
    }
    if facade_now_ms().abs_diff(response.timestamp_unix_ms) > CALL_MAX_CLOCK_SKEW_MS {
        return Err(IpcError::Expired);
    }
    if !auth.verify(&canonical_response_bytes(&response), &response.mac) {
        return Err(IpcError::Authentication);
    }
    Ok(response)
}

fn canonical_response_bytes(response: &ResponseEnvelope) -> Vec<u8> {
    let mut fields = BTreeMap::new();
    fields.insert("ok", Value::Bool(response.ok));
    fields.insert("payload", canonical_facade_json_value(&response.payload));
    fields.insert(
        "request_nonce",
        Value::String(response.request_nonce.clone()),
    );
    fields.insert(
        "timestamp_unix_ms",
        serde_json::json!(response.timestamp_unix_ms),
    );
    fields.insert("version", Value::from(response.version));
    serde_json::to_vec(&fields).expect("serializing response canonical form cannot fail")
}

fn canonical_facade_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_unstable();
            let mut canonical = serde_json::Map::new();
            for key in keys {
                canonical.insert(key.clone(), canonical_facade_json_value(&map[key]));
            }
            Value::Object(canonical)
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(canonical_facade_json_value).collect())
        }
        _ => value.clone(),
    }
}

fn read_facade_bounded_line<R: BufRead>(reader: &mut R) -> Result<String, IpcError> {
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

fn facade_now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod call_timeout_facade_tests {
    use super::*;

    #[test]
    fn call_read_timeout_is_bounded() {
        assert_eq!(
            bounded_call_read_timeout(Duration::ZERO),
            MIN_CALL_READ_TIMEOUT
        );
        assert_eq!(
            bounded_call_read_timeout(Duration::from_secs(35)),
            Duration::from_secs(35)
        );
        assert_eq!(
            bounded_call_read_timeout(Duration::from_secs(600)),
            MAX_CALL_READ_TIMEOUT
        );
    }

    #[test]
    fn terminal_exec_gets_longer_but_bounded_transport_window() {
        assert_eq!(
            call_read_timeout_for_command("terminal.exec"),
            TERMINAL_EXEC_CALL_READ_TIMEOUT
        );
        assert_eq!(
            call_read_timeout_for_command("status"),
            DEFAULT_CALL_READ_TIMEOUT
        );
    }

    #[test]
    fn facade_frame_reader_preserves_one_mib_ceiling() {
        let oversized = vec![b'x'; MAX_FRAME_BYTES + 1];
        let mut reader = std::io::Cursor::new(oversized);
        assert!(matches!(
            read_facade_bounded_line(&mut reader),
            Err(IpcError::FrameTooLarge)
        ));
    }
}
