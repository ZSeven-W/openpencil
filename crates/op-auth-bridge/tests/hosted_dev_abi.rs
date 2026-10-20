//! End-to-end public bridge coverage against an explicitly opted-in local v4
//! archive. All HTTP is numeric loopback and all credentials are synthetic.
#![cfg(all(op_auth_development_prebuilt, op_auth_hosted_access_prebuilt))]

use op_auth_bridge::{
    hosted_access_provider, AuthInitConfig, AuthStatus, HostedAccess, HostedAccessError,
    HostedAccessPoll, HostedAccessRequestId, SESSION_HANDLE,
};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[test]
fn public_bridge_login_access_cache_cancel_and_logout_use_one_private_session() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || serve(listener, address));
    let storage = TestStorage::new();
    assert!(op_auth_bridge::init(&AuthInitConfig {
        base_url: format!("http://{address}"),
        storage_dir: storage.0.clone(),
        device_name: "Public bridge test".into(),
        app_version: "0.0.0".into(),
    }));
    assert!(!op_auth_bridge::restore());
    let provider = hosted_access_provider();
    assert!(provider.available());
    let initial_epoch = provider.session_epoch();
    assert_ne!(initial_epoch, 0);
    assert_eq!(provider.begin(), Err(HostedAccessError::NotSignedIn));

    let login = op_auth_bridge::login_begin();
    assert_ne!(login, 0);
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        match op_auth_bridge::poll(login) {
            AuthStatus::SignedIn { device_id, .. } => {
                assert_eq!(device_id, "synthetic-device");
                break;
            }
            AuthStatus::Error { .. } | AuthStatus::Canceled => panic!("mock login failed"),
            _ if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => panic!("mock login timed out"),
        }
    }
    let epoch = provider.session_epoch();
    assert!(epoch > initial_epoch);
    assert!(matches!(
        op_auth_bridge::poll(SESSION_HANDLE),
        AuthStatus::SignedIn { .. }
    ));
    assert!(!format!("{:?}", op_auth_bridge::poll(SESSION_HANDLE)).contains("device-secret"));

    let first = provider.begin().unwrap();
    let canceled = provider.begin().unwrap();
    assert_ne!(first, canceled);
    provider.cancel(canceled);
    assert!(matches!(
        provider.poll(canceled),
        Err(HostedAccessError::RequestNotFound)
    ));
    let access = wait_access(first);
    assert_eq!(access.subject(), "12345678-abcd-4000-8000-123456789012");
    assert_eq!(access.issuer(), "https://sso.example.test");
    assert_eq!(access.gateway_base_url(), "https://gateway.example.test/v1");
    assert!(access.expires_at_unix() > now());
    assert!(access.bearer().starts_with("opllm_"));
    assert!(!format!("{access:?}").contains(access.bearer()));
    assert!(matches!(
        provider.poll(first),
        Err(HostedAccessError::RequestNotFound)
    ));

    // The listener closes after one exchange; a second access must use cache.
    server.join().unwrap();
    let cached = wait_access(provider.begin().unwrap());
    assert_eq!(cached.bearer(), access.bearer());
    assert_eq!(provider.session_epoch(), epoch);
    for entry in std::fs::read_dir(&storage.0).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let bytes = std::fs::read(path).unwrap();
            assert!(!bytes.windows(6).any(|window| window == b"opllm_"));
        }
    }

    let stale = provider.begin().unwrap();
    op_auth_bridge::sign_out();
    assert!(provider.session_epoch() > epoch);
    assert_eq!(op_auth_bridge::poll(SESSION_HANDLE), AuthStatus::Idle);
    assert!(matches!(
        provider.poll(stale),
        Err(HostedAccessError::RequestNotFound)
    ));
    assert_eq!(provider.begin(), Err(HostedAccessError::NotSignedIn));
}

fn wait_access(id: HostedAccessRequestId) -> HostedAccess {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match hosted_access_provider().poll(id) {
            Ok(HostedAccessPoll::Ready(access)) => return access,
            Ok(HostedAccessPoll::Pending) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            state => panic!("mock access failed: {state:?}"),
        }
    }
}

fn serve(listener: TcpListener, address: SocketAddr) {
    let responses = [
        (
            "POST /api/v1/device/login/start ",
            201,
            json!({
                "pairing_id": "pair-1", "verification_uri": format!("http://{address}/verify"),
                "ws_url": "", "poll_interval_seconds": 1,
            }),
        ),
        (
            "GET /api/v1/device/login/poll?pairing_id=pair-1 ",
            200,
            json!({"status": "approved"}),
        ),
        (
            "POST /api/v1/device/login/exchange ",
            200,
            json!({
                "device_token": "synthetic-device-secret", "device_id": "synthetic-device",
                "user": {"display_name": "Fixture", "username": "fixture", "primary_email": null, "avatar_url": null},
            }),
        ),
    ];
    for (path, code, body) in responses {
        let mut stream = accept(&listener);
        let request = read_request(&mut stream);
        assert!(request.starts_with(path));
        respond(&mut stream, code, &body);
    }
    let mut stream = accept(&listener);
    let request = read_request(&mut stream);
    assert!(request.starts_with("POST /api/v1/device/llm-token "));
    let (headers, body) = request.split_once("\r\n\r\n").unwrap();
    let headers = headers.to_ascii_lowercase();
    assert!(headers.contains("authorization: bearer synthetic-device-secret\r\n"));
    assert_eq!(headers.matches("idempotency-key:").count(), 1);
    let body: Value = serde_json::from_str(body).unwrap();
    assert_eq!(body.as_object().unwrap().len(), 2);
    assert_eq!(body["expires_in_seconds"], 300);
    assert!(body["token"].as_str().unwrap().starts_with("opllm_"));
    let issued = now();
    respond(
        &mut stream,
        200,
        &json!({
            "version": 1, "token": body["token"],
            "token_id": "1234567890abcdef1234567890abcdef", "token_type": "Bearer",
            "issuer": "https://sso.example.test", "subject": "12345678-abcd-4000-8000-123456789012",
            "audience": "llm-gateway", "scopes": ["llm:models", "llm:invoke"],
            "issued_at_unix": issued, "expires_at_unix": issued + 300,
            "gateway_base_url": "https://gateway.example.test/v1",
        }),
    );
}

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        match listener.accept() {
            Ok((stream, _)) => return stream,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => panic!("mock HTTP accept failed or timed out"),
        }
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut request = Vec::new();
    loop {
        let mut buffer = [0_u8; 1024];
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0);
        request.extend_from_slice(&buffer[..count]);
        assert!(request.len() <= 8 * 1024);
        if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&request[..end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")?
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or(0);
            if request.len() >= end + 4 + length {
                return String::from_utf8(request).unwrap();
            }
        }
    }
}

fn respond(stream: &mut TcpStream, code: u16, body: &Value) {
    let body = body.to_string();
    write!(stream, "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

struct TestStorage(PathBuf);

impl TestStorage {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "op-auth-public-loopback-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestStorage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
