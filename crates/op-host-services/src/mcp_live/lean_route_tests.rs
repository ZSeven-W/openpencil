//! The live server's `/mcp/lean` endpoint, driven through the real
//! `serve_connection` router with a stub UI thread.

use super::super::*;
use super::*;

const PORT: u16 = 51235;

struct MockStream {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
}

impl std::io::Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        std::io::Read::read(&mut self.input, buf)
    }
}

impl std::io::Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// POST `body` to `path` with a stub UI thread that answers snapshot
/// requests with an empty document. Returns the raw HTTP response and the
/// UI requests the router made, by kind.
fn post(path: &str, body: &str) -> (String, Vec<&'static str>) {
    let (req_tx, req_rx) = mpsc::channel::<UiRequest>();
    let ui = thread::spawn(move || {
        let mut seen = Vec::new();
        while let Ok(request) = req_rx.recv_timeout(Duration::from_secs(5)) {
            match request {
                UiRequest::Snapshot { ack } => {
                    seen.push("snapshot");
                    let _ = ack.send(EditorState::new());
                }
                UiRequest::ListPages { .. } => seen.push("list_pages"),
                UiRequest::Apply { .. } => seen.push("apply"),
                _ => seen.push("other"),
            }
        }
        seen
    });
    let admission = LiveAdmission::new("tok".to_string(), PORT);
    let stateful_lock = Mutex::new(());
    let quit_flag = AtomicBool::new(false);
    let wake_ui: UiWake = Arc::new(|| {});
    let client_identity = Mutex::new(None);
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.into_bytes()),
        output: Vec::new(),
    };
    serve_connection(
        &mut stream,
        &req_tx,
        &admission,
        &stateful_lock,
        &quit_flag,
        &wake_ui,
        &client_identity,
    )
    .expect("served");
    drop(req_tx);
    let seen = ui.join().expect("ui thread");
    (String::from_utf8_lossy(&stream.output).into_owned(), seen)
}

fn tool_names(response: &str) -> Vec<String> {
    let body = response.split_once("\r\n\r\n").expect("http body").1;
    let value: serde_json::Value = serde_json::from_str(body).expect("json-rpc body");
    value["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| tool["name"].as_str().expect("name").to_string())
        .collect()
}

const TOOLS_LIST: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;

#[test]
fn the_lean_path_lists_six_tools_and_the_default_path_the_full_catalog() {
    let (lean, _) = post("/mcp/lean", TOOLS_LIST);
    assert_eq!(
        tool_names(&lean),
        crate::mcp_serve::tool_catalog::LEAN_TOOLS,
        "{lean}"
    );
    let (full, _) = post("/mcp", TOOLS_LIST);
    assert!(tool_names(&full).len() > 100, "{full}");
}

#[test]
fn the_lean_path_refuses_fast_path_tools_instead_of_serving_them() {
    let call = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_pages","arguments":{}}}"#;
    let (response, seen) = post("/mcp/lean", call);
    assert!(response.contains("tool-not-in-profile"), "{response}");
    assert!(
        !seen.contains(&"list_pages") && !seen.contains(&"apply"),
        "a refused call must not reach the UI thread's tool paths: {seen:?}"
    );
}

#[test]
fn the_lean_path_serves_its_own_tools() {
    let call = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_editor_state","arguments":{}}}"#;
    let (response, seen) = post("/mcp/lean", call);
    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(!response.contains(r#""isError":true"#), "{response}");
    assert!(response.contains("variables"), "{response}");
    assert_eq!(seen, ["snapshot"]);
}

#[test]
fn unknown_mcp_subpaths_are_not_found() {
    let (response, seen) = post("/mcp/tiny", TOOLS_LIST);
    assert!(response.starts_with("HTTP/1.1 404"), "{response}");
    assert!(seen.is_empty());
}
