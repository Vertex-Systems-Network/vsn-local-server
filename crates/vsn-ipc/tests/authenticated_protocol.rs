use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use vsn_ipc::{
    call, serve_until, IpcError, RequestEnvelope, ResponseEnvelope, IPC_ADDRESS, PROTOCOL_VERSION,
};
use vsn_security::IpcAuthenticator;

const MAX_CLOCK_SKEW_MS: u128 = 30_000;
const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy)]
enum FakeResponseMode {
    WrongNonce,
    TamperedMac,
    WrongVersion,
    Expired,
    Oversized,
}

#[test]
#[ignore = "requires an isolated local keyring and loopback IPC port"]
fn authenticated_protocol_enforcement() {
    let auth = IpcAuthenticator::load_or_create().expect("create shared IPC authenticator");
    let stop = Arc::new(AtomicBool::new(false));
    let handled = Arc::new(AtomicUsize::new(0));
    let server_stop = stop.clone();
    let server_handled = handled.clone();

    let server = thread::spawn(move || {
        serve_until(server_stop, move |request| {
            server_handled.fetch_add(1, Ordering::SeqCst);
            (
                true,
                json!({
                    "command": request.command,
                    "accepted": true,
                }),
            )
        })
    });
    wait_for_server();

    let response = call("ping", json!({"probe": "client-auth"})).expect("authenticated call");
    assert!(response.ok);
    assert_eq!(
        response.payload.get("command"),
        Some(&Value::String("ping".into()))
    );
    mark("authenticated-envelope");

    let seed = RequestEnvelope::new("ping", json!({"probe": "replay"}), &auth);
    assert_eq!(seed.nonce.len(), 48);
    assert!(seed.nonce.bytes().all(|byte| byte.is_ascii_hexdigit()));
    mark("generated-nonce-format");

    let before_seed = handled.load(Ordering::SeqCst);
    let first = raw_request(&seed);
    assert!(first.ok);
    assert_eq!(handled.load(Ordering::SeqCst), before_seed + 1);
    let replay = raw_request(&seed);
    assert!(!replay.ok);
    assert_eq!(
        replay.payload.get("error").and_then(Value::as_str),
        Some("replayed request")
    );
    assert_eq!(handled.load(Ordering::SeqCst), before_seed + 1);
    mark("request-replay-rejected");

    let mut tampered = RequestEnvelope::new("ping", json!({"safe": true}), &auth);
    tampered.params = json!({"safe": false});
    let tampered_response = raw_request(&tampered);
    assert!(!tampered_response.ok);
    assert_eq!(
        tampered_response
            .payload
            .get("error")
            .and_then(Value::as_str),
        Some("authentication failed")
    );
    mark("request-mac-tamper-rejected");

    let mut manual_valid = RequestEnvelope::new("ping", json!({"manual": true}), &auth);
    manual_valid.nonce = "a".repeat(48);
    resign_request(&mut manual_valid, &auth);
    assert!(raw_request(&manual_valid).ok);

    let mut invalid_nonce = RequestEnvelope::new("ping", json!({"manual": true}), &auth);
    invalid_nonce.nonce = "g".repeat(48);
    resign_request(&mut invalid_nonce, &auth);
    let invalid_nonce_response = raw_request(&invalid_nonce);
    assert!(!invalid_nonce_response.ok);
    assert_eq!(
        invalid_nonce_response
            .payload
            .get("error")
            .and_then(Value::as_str),
        Some("authentication failed")
    );
    mark("invalid-nonce-rejected");

    let mut wrong_version = RequestEnvelope::new("ping", json!({}), &auth);
    wrong_version.version = PROTOCOL_VERSION + 1;
    resign_request(&mut wrong_version, &auth);
    let version_response = raw_request(&wrong_version);
    assert!(!version_response.ok);
    assert_eq!(
        version_response
            .payload
            .get("error")
            .and_then(Value::as_str),
        Some("unsupported protocol version")
    );
    mark("request-version-rejected");

    let mut expired = RequestEnvelope::new("ping", json!({}), &auth);
    expired.timestamp_unix_ms = now_ms().saturating_sub(MAX_CLOCK_SKEW_MS + 1_000);
    resign_request(&mut expired, &auth);
    let expired_response = raw_request(&expired);
    assert!(!expired_response.ok);
    assert_eq!(
        expired_response
            .payload
            .get("error")
            .and_then(Value::as_str),
        Some("request expired or clock skew too large")
    );
    mark("expired-request-rejected");

    send_oversized_request();
    mark("oversized-request-rejected");

    stop.store(true, Ordering::SeqCst);
    server
        .join()
        .expect("join protocol server")
        .expect("protocol server exits cleanly");

    let modes = [
        FakeResponseMode::WrongNonce,
        FakeResponseMode::TamperedMac,
        FakeResponseMode::WrongVersion,
        FakeResponseMode::Expired,
        FakeResponseMode::Oversized,
    ];
    let fake_listener = bind_with_retry();
    let fake_auth = auth.clone();
    let fake_server = thread::spawn(move || serve_fake_responses(fake_listener, fake_auth, &modes));

    assert!(matches!(
        call("ping", json!({"probe": "wrong-nonce"})),
        Err(IpcError::ResponseMismatch)
    ));
    mark("response-nonce-binding");

    assert!(matches!(
        call("ping", json!({"probe": "tampered-response"})),
        Err(IpcError::Authentication)
    ));
    mark("response-mac-tamper-rejected");

    assert!(matches!(
        call("ping", json!({"probe": "wrong-response-version"})),
        Err(IpcError::ProtocolVersion)
    ));
    mark("response-version-rejected");

    assert!(matches!(
        call("ping", json!({"probe": "expired-response"})),
        Err(IpcError::Expired)
    ));
    mark("expired-response-rejected");

    assert!(matches!(
        call("ping", json!({"probe": "oversized-response"})),
        Err(IpcError::FrameTooLarge)
    ));
    mark("oversized-response-rejected");

    fake_server.join().expect("join fake response server");
}

fn raw_request(request: &RequestEnvelope) -> ResponseEnvelope {
    let mut stream = TcpStream::connect(IPC_ADDRESS).expect("connect to protocol server");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set read timeout");
    let mut encoded = serde_json::to_vec(request).expect("serialize request");
    encoded.push(b'\n');
    stream.write_all(&encoded).expect("write request");
    stream.flush().expect("flush request");

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .expect("read response");
    serde_json::from_str(&line).expect("parse response")
}

fn resign_request(request: &mut RequestEnvelope, auth: &IpcAuthenticator) {
    request.mac = auth.sign(&canonical_request_bytes(request));
}

fn canonical_request_bytes(request: &RequestEnvelope) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": request.version,
        "timestamp_unix_ms": request.timestamp_unix_ms,
        "nonce": request.nonce,
        "command": request.command,
        "params": request.params,
    }))
    .expect("serialize canonical request")
}

fn resign_response(response: &mut ResponseEnvelope, auth: &IpcAuthenticator) {
    response.mac = auth.sign(&canonical_response_bytes(response));
}

fn canonical_response_bytes(response: &ResponseEnvelope) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": response.version,
        "timestamp_unix_ms": response.timestamp_unix_ms,
        "request_nonce": response.request_nonce,
        "ok": response.ok,
        "payload": response.payload,
    }))
    .expect("serialize canonical response")
}

fn send_oversized_request() {
    let mut stream = TcpStream::connect(IPC_ADDRESS).expect("connect oversized request");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set oversized read timeout");
    stream
        .write_all(&vec![b'x'; MAX_FRAME_BYTES + 1])
        .expect("write oversized request");
    let _ = stream.shutdown(Shutdown::Write);

    let mut output = Vec::new();
    match stream.read_to_end(&mut output) {
        Ok(_) => assert!(output.is_empty()),
        Err(error) => assert!(matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::BrokenPipe
        )),
    }
}

fn wait_for_server() {
    for _ in 0..100 {
        if TcpStream::connect(IPC_ADDRESS).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("protocol server did not become ready");
}

fn bind_with_retry() -> TcpListener {
    for _ in 0..100 {
        match TcpListener::bind(IPC_ADDRESS) {
            Ok(listener) => return listener,
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("bind fake response server: {error}"),
        }
    }
    panic!("fake response server port did not become available");
}

fn serve_fake_responses(listener: TcpListener, auth: IpcAuthenticator, modes: &[FakeResponseMode]) {
    for mode in modes {
        let (mut stream, _) = listener.accept().expect("accept fake response client");
        let cloned = stream.try_clone().expect("clone fake stream");
        let mut line = String::new();
        BufReader::new(cloned)
            .read_line(&mut line)
            .expect("read fake request");
        let request: RequestEnvelope = serde_json::from_str(&line).expect("parse fake request");

        if matches!(mode, FakeResponseMode::Oversized) {
            stream
                .write_all(&vec![b'x'; MAX_FRAME_BYTES + 1])
                .expect("write oversized response");
            stream.flush().expect("flush oversized response");
            continue;
        }

        let mut response = ResponseEnvelope::new(
            request.nonce.clone(),
            true,
            json!({"accepted": true}),
            &auth,
        );
        match mode {
            FakeResponseMode::WrongNonce => {
                response.request_nonce = "0".repeat(48);
                resign_response(&mut response, &auth);
            }
            FakeResponseMode::TamperedMac => {
                response.payload = json!({"accepted": false});
            }
            FakeResponseMode::WrongVersion => {
                response.version = PROTOCOL_VERSION + 1;
                resign_response(&mut response, &auth);
            }
            FakeResponseMode::Expired => {
                response.timestamp_unix_ms = now_ms().saturating_sub(MAX_CLOCK_SKEW_MS + 1_000);
                resign_response(&mut response, &auth);
            }
            FakeResponseMode::Oversized => unreachable!(),
        }

        let mut encoded = serde_json::to_vec(&response).expect("serialize fake response");
        encoded.push(b'\n');
        stream.write_all(&encoded).expect("write fake response");
        stream.flush().expect("flush fake response");
    }
}

fn mark(name: &str) {
    println!("02.02-check={name}");
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_millis()
}
