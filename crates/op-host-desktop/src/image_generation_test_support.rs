//! Local provider capture shared by automatic-job and manual-panel tests.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider, ImageTestStatus};

pub(crate) fn capture_provider() -> (ImageGenProfile, Receiver<serde_json::Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "provider was never called");
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("provider accept: {error}"),
            }
        };
        // Windows sockets inherit the listener's non-blocking mode on accept.
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut request = Vec::new();
        let body = loop {
            let mut bytes = [0; 4096];
            let size = stream.read(&mut bytes).unwrap();
            assert!(size > 0, "request ended before its JSON body");
            request.extend_from_slice(&bytes[..size]);
            if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:").map(str::trim))
                    .unwrap()
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + length {
                    break serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap();
                }
            }
            assert!(
                request.len() < 64 * 1024,
                "unexpectedly large image request"
            );
        };
        let response = r#"{"data":[{"b64_json":"AA=="}]}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}", response.len()).unwrap();
        tx.send(body).unwrap();
    });
    (
        ImageGenProfile {
            id: "capture".into(),
            name: "Local test".into(),
            provider: ImageGenProvider::Custom,
            api_key: "test-key".into(),
            model: "test-model".into(),
            base_url: Some(format!("http://{address}")),
            test_status: ImageTestStatus::Idle,
        },
        rx,
    )
}
