use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc};
use std::time::Duration;

use op_ai::chat_provider::{
    ChatAttachment, ChatProvider, ChatRequest, ChatToolExecutor, ChatToolResult,
};
use op_editor_core::{BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey};
use serde_json::Value;

use crate::chat_builtin_http::{supports_native_image_input, ConfiguredBuiltinProvider};

struct NoopExecutor;

impl ChatToolExecutor for NoopExecutor {
    fn execute(&self, _name: &str, _args_json: &str) -> ChatToolResult {
        panic!("the capture response must not request a tool")
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let n = stream.read(&mut chunk).expect("read request");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let Some(header_end) = buf.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        if buf.len() >= header_end + 4 + content_length {
            break;
        }
    }
    String::from_utf8(buf).expect("request is UTF-8")
}

fn capture_request(
    kind: BuiltinAgentKind,
    base_path: &str,
    response_body: &'static str,
    use_agent_loop: bool,
) -> (String, Value) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind capture server");
    let addr = listener.local_addr().expect("capture server address");
    let (request_tx, request_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        request_tx
            .send(read_request(&mut stream))
            .expect("send captured request");
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            response_body.len(),
            response_body,
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&BuiltinAgentConfig {
        id: "builtin-minimax".into(),
        preset: BuiltinAgentPresetKey::MiniMax,
        display_name: "MiniMax".into(),
        kind,
        api_key: "test-key".into(),
        model: "MiniMax-M3".into(),
        base_url: format!("http://{addr}{base_path}"),
        enabled: true,
    })
    .expect("ready provider");
    let provider = if use_agent_loop {
        provider.with_canvas_tools(Vec::new(), Arc::new(NoopExecutor))
    } else {
        provider
    };
    let _deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "Inspect this reference image.".into(),
            max_output_tokens: 64,
            attachments: vec![ChatAttachment {
                name: "reference.png".into(),
                media_type: "image/png".into(),
                data: b"png".to_vec(),
            }],
            ..Default::default()
        })
        .collect();
    server.join().expect("capture server exits");

    let request = request_rx.recv().expect("captured request");
    let (headers, body) = request
        .split_once("\r\n\r\n")
        .expect("request has headers and body");
    let request_line = headers.lines().next().expect("request line").to_string();
    let body = serde_json::from_str(body).expect("request body is JSON");
    (request_line, body)
}

#[test]
fn only_minimax_m3_enables_native_image_input() {
    assert!(supports_native_image_input("MiniMax-M3"));
    assert!(!supports_native_image_input("MiniMax-M2.7"));
}

#[test]
fn openai_compatible_request_appends_path_and_sends_image_data_url() {
    let (request_line, body) = capture_request(
        BuiltinAgentKind::OpenAiCompat,
        "/v1",
        "data: [DONE]\n\n",
        false,
    );
    assert_eq!(request_line, "POST /v1/chat/completions HTTP/1.1");
    assert_eq!(
        body.pointer("/messages/0/content/1/type")
            .and_then(Value::as_str),
        Some("image_url")
    );
    assert_eq!(
        body.pointer("/messages/0/content/1/image_url/url")
            .and_then(Value::as_str),
        Some("data:image/png;base64,cG5n")
    );
}

#[test]
fn anthropic_request_appends_path_and_sends_image_block() {
    let (request_line, body) = capture_request(
        BuiltinAgentKind::Anthropic,
        "/anthropic",
        "data: {\"type\":\"message_stop\"}\n\n",
        false,
    );
    assert_eq!(request_line, "POST /anthropic/v1/messages HTTP/1.1");
    assert_eq!(
        body.pointer("/messages/0/content/1/type")
            .and_then(Value::as_str),
        Some("image")
    );
    assert_eq!(
        body.pointer("/messages/0/content/1/source/data")
            .and_then(Value::as_str),
        Some("cG5n")
    );
}

#[test]
fn agent_loops_preserve_native_image_content_for_both_protocols() {
    let (openai_request_line, openai_body) = capture_request(
        BuiltinAgentKind::OpenAiCompat,
        "/v1",
        "data: [DONE]\n\n",
        true,
    );
    assert_eq!(openai_request_line, "POST /v1/chat/completions HTTP/1.1");
    assert_eq!(
        openai_body
            .pointer("/messages/0/content/1/type")
            .and_then(Value::as_str),
        Some("image_url")
    );

    let (anthropic_request_line, anthropic_body) = capture_request(
        BuiltinAgentKind::Anthropic,
        "/anthropic",
        "data: {\"type\":\"message_stop\"}\n\n",
        true,
    );
    assert_eq!(
        anthropic_request_line,
        "POST /anthropic/v1/messages HTTP/1.1"
    );
    assert_eq!(
        anthropic_body
            .pointer("/messages/0/content/1/type")
            .and_then(Value::as_str),
        Some("image")
    );
}
