use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use super::*;

// ---------------------------------------------------------------
// Scripted mock OpenCode server — speaks just enough HTTP/1.1 for
// the provider's control plane + SSE event stream, recording every
// request so tests can assert the exact wire bodies.
// ---------------------------------------------------------------

#[derive(Clone)]
struct Scenario {
    /// JSON payloads emitted as `data:` lines on `GET /event`.
    sse_events: Vec<String>,
    /// Body served for `GET /session/ses_mock/message` (fallback).
    messages_fallback: String,
}

type RequestLog = Arc<Mutex<Vec<(String, String, String)>>>;

struct MockServer {
    base: String,
    requests: RequestLog,
}

fn start_mock(scenario: Scenario) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    let base = format!("http://{}", listener.local_addr().unwrap());
    let requests: RequestLog = Arc::default();
    let log = Arc::clone(&requests);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            let scenario = scenario.clone();
            let log = Arc::clone(&log);
            std::thread::spawn(move || handle_connection(stream, scenario, log));
        }
    });
    MockServer { base, requests }
}

fn handle_connection(mut stream: TcpStream, scenario: Scenario, log: RequestLog) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    // Headers — we only care about content-length.
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed
            .to_ascii_lowercase()
            .strip_prefix("content-length:")
            .map(str::trim)
            .map(str::to_string)
        {
            content_length = rest.parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 && reader.read_exact(&mut body).is_err() {
        return;
    }
    let body = String::from_utf8_lossy(&body).to_string();
    log.lock()
        .unwrap()
        .push((method.clone(), path.clone(), body));

    match (method.as_str(), path.as_str()) {
        ("GET", "/event") => {
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            );
            for ev in &scenario.sse_events {
                let _ = stream.write_all(format!("data: {ev}\n\n").as_bytes());
            }
            let _ = stream.flush();
            // Keep the stream open briefly past the script so the
            // provider's loop (not the connection close) decides when
            // the turn ends.
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        ("POST", "/session") => write_json(&mut stream, 200, r#"{"id":"ses_mock"}"#),
        ("POST", "/session/ses_mock/message") => write_json(&mut stream, 200, r#"{"info":{}}"#),
        ("POST", "/session/ses_mock/prompt_async") => write_json(&mut stream, 200, "{}"),
        ("GET", "/session/ses_mock/message") => {
            write_json(&mut stream, 200, &scenario.messages_fallback)
        }
        ("GET", "/config/providers") => write_json(&mut stream, 200, r#"{"providers":[]}"#),
        _ => write_json(&mut stream, 404, r#"{"message":"not found"}"#),
    }
}

fn write_json(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = if status == 200 { "OK" } else { "Not Found" };
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    );
    let _ = stream.flush();
}

fn collect_deltas(server: &MockServer, request: ChatRequest) -> Vec<ChatDelta> {
    let provider = OpenCodeProvider::with_base_url(server.base.clone());
    provider.send(request).collect()
}

// ---------------------------------------------------------------
// End-to-end turns against the scripted server
// ---------------------------------------------------------------

#[test]
fn opencode_turn_streams_text_thinking_and_done() {
    let scenario = Scenario {
        sse_events: vec![
            r#"{"type":"server.connected","properties":{}}"#.into(),
            // Foreign-session deltas must be filtered out.
            r#"{"type":"message.part.delta","properties":{"sessionID":"ses_other","field":"text","delta":"WRONG"}}"#.into(),
            r#"{"type":"message.part.delta","properties":{"sessionID":"ses_mock","field":"text","delta":"Hel"}}"#.into(),
            r#"{"type":"message.part.delta","properties":{"sessionID":"ses_mock","field":"reasoning","delta":"mull"}}"#.into(),
            r#"{"type":"message.part.delta","properties":{"sessionID":"ses_mock","field":"text","delta":"lo"}}"#.into(),
            r#"{"type":"session.idle","properties":{"sessionID":"ses_other"}}"#.into(),
            r#"{"type":"session.idle","properties":{"sessionID":"ses_mock"}}"#.into(),
        ],
        messages_fallback: "[]".into(),
    };
    let server = start_mock(scenario);
    let deltas = collect_deltas(
        &server,
        ChatRequest {
            system_prompt: "Be helpful.".into(),
            user_message: "hi".into(),
            model: Some("anthropic/claude-test".into()),
            ..Default::default()
        },
    );

    let text: String = deltas
        .iter()
        .filter_map(|d| match d {
            ChatDelta::TextDelta(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hello", "session-scoped text deltas only: {deltas:?}");
    assert!(
        deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::Thinking(s) if s == "mull")),
        "reasoning deltas forward as thinking: {deltas:?}"
    );
    assert!(
        !deltas.iter().any(|d| matches!(d, ChatDelta::Error(_))),
        "clean turn must not error: {deltas:?}"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));

    // Wire assertions: session create → noReply system injection →
    // prompt_async with parsed model + text part.
    let requests = server.requests.lock().unwrap().clone();
    let session_create = requests
        .iter()
        .find(|(m, p, _)| m == "POST" && p == "/session")
        .expect("session created");
    assert!(session_create.2.contains("OpenPencil Chat"));
    let sys_inject = requests
        .iter()
        .find(|(m, p, _)| m == "POST" && p == "/session/ses_mock/message")
        .expect("system prompt injected via noReply message");
    assert!(sys_inject.2.contains(r#""noReply":true"#));
    assert!(sys_inject.2.contains("Be helpful."));
    let prompt = requests
        .iter()
        .find(|(m, p, _)| m == "POST" && p == "/session/ses_mock/prompt_async")
        .expect("prompt_async sent");
    assert!(prompt.2.contains(r#""providerID":"anthropic""#));
    assert!(prompt.2.contains(r#""modelID":"claude-test""#));
    assert!(prompt.2.contains(r#""text":"hi""#));
}

#[test]
fn opencode_session_error_maps_to_label_with_nested_json() {
    // The exact structured-error shape captured from a live opencode
    // 1.15.0 server (expired API key).
    let scenario = Scenario {
        sse_events: vec![
            r#"{"type":"session.error","properties":{"sessionID":"ses_mock","error":{"name":"APIError","data":{"message":"Unauthorized: {\"error\":{\"code\":\"invalid_api_key\",\"message\":\"invalid access token\"}}","statusCode":401}}}}"#.into(),
        ],
        messages_fallback: "[]".into(),
    };
    let server = start_mock(scenario);
    let deltas = collect_deltas(
        &server,
        ChatRequest {
            user_message: "hi".into(),
            ..Default::default()
        },
    );
    assert!(
        deltas.iter().any(|d| matches!(
            d,
            ChatDelta::Error(msg) if msg == "API error — Unauthorized: invalid access token"
        )),
        "structured error must map through the TS label + nested-JSON path: {deltas:?}"
    );
    assert!(deltas.iter().any(|d| matches!(d, ChatDelta::Done { .. })));
}

#[test]
fn opencode_empty_stream_falls_back_to_session_messages() {
    let scenario = Scenario {
        sse_events: vec![
            r#"{"type":"session.idle","properties":{"sessionID":"ses_mock"}}"#.into(),
        ],
        messages_fallback: r#"[
            {"info":{"role":"user"},"parts":[{"type":"text","text":"hi"}]},
            {"info":{"role":"assistant"},"parts":[{"type":"step-start"},{"type":"text","text":"from-fallback"}]}
        ]"#
        .into(),
    };
    let server = start_mock(scenario);
    let deltas = collect_deltas(
        &server,
        ChatRequest {
            user_message: "hi".into(),
            ..Default::default()
        },
    );
    assert!(
        deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s == "from-fallback")),
        "messages fallback must surface the assistant text: {deltas:?}"
    );
    assert!(
        !deltas.iter().any(|d| matches!(d, ChatDelta::Error(_))),
        "fallback success must suppress the empty-response error: {deltas:?}"
    );
}

#[test]
fn opencode_idle_with_no_output_emits_empty_response_error() {
    let scenario = Scenario {
        sse_events: vec![r#"{"type":"session.idle","properties":{"sessionID":"ses_mock"}}"#.into()],
        messages_fallback: "[]".into(),
    };
    let server = start_mock(scenario);
    let deltas = collect_deltas(
        &server,
        ChatRequest {
            user_message: "hi".into(),
            ..Default::default()
        },
    );
    assert!(
        deltas.iter().any(|d| matches!(
            d,
            ChatDelta::Error(msg) if msg.starts_with("OpenCode returned an empty response.")
        )),
        "TS empty-response error expected: {deltas:?}"
    );
}

// ---------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------

#[test]
fn parse_server_url_matches_ts_handshake() {
    assert_eq!(
        parse_server_url("opencode server listening on http://127.0.0.1:4096").as_deref(),
        Some("http://127.0.0.1:4096")
    );
    // Prefix is mandatory (TS startsWith check).
    assert!(parse_server_url("server listening on http://127.0.0.1:1").is_none());
    assert!(parse_server_url("Warning: OPENCODE_SERVER_PASSWORD is not set").is_none());
}

#[test]
fn parse_opencode_model_splits_on_first_slash() {
    assert_eq!(
        parse_opencode_model("anthropic/claude-sonnet-4-5"),
        Some(("anthropic".into(), "claude-sonnet-4-5".into()))
    );
    // Model ids can themselves contain slashes — only the first
    // separates the provider (TS indexOf semantics).
    assert_eq!(
        parse_opencode_model("openrouter/meta/llama-3"),
        Some(("openrouter".into(), "meta/llama-3".into()))
    );
    assert_eq!(parse_opencode_model("no-slash"), None);
}

#[test]
fn parse_sse_data_line_extracts_json() {
    let val = parse_sse_data_line(r#"data: {"type":"session.idle","properties":{}}"#).unwrap();
    assert_eq!(val["type"], "session.idle");
    // Compact form without the space is also valid SSE.
    assert!(parse_sse_data_line(r#"data:{"a":1}"#).is_some());
    assert!(parse_sse_data_line("event: ping").is_none());
    assert!(parse_sse_data_line("").is_none());
}

#[test]
fn format_opencode_error_table_matches_ts() {
    // Structured error with nested JSON in the message.
    let structured: serde_json::Value = serde_json::json!({
        "name": "APIError",
        "data": { "message": "Unauthorized: {\"error\":{\"message\":\"invalid access token\"}}" }
    });
    assert_eq!(
        format_opencode_error(Some(&structured)),
        "API error — Unauthorized: invalid access token"
    );
    // Unknown names pass through as the label.
    let unknown_name: serde_json::Value = serde_json::json!({
        "name": "WeirdError",
        "data": { "message": "boom" }
    });
    assert_eq!(
        format_opencode_error(Some(&unknown_name)),
        "WeirdError — boom"
    );
    // Plain { message }.
    let plain: serde_json::Value = serde_json::json!({ "message": "plain failure" });
    assert_eq!(format_opencode_error(Some(&plain)), "plain failure");
    // String error.
    let s: serde_json::Value = serde_json::Value::String("just text".into());
    assert_eq!(format_opencode_error(Some(&s)), "just text");
    // Null / missing.
    assert_eq!(format_opencode_error(None), "Unknown error");
    assert_eq!(
        format_opencode_error(Some(&serde_json::Value::Null)),
        "Unknown error"
    );
    // Fallback: truncated JSON over 200 chars.
    let big: serde_json::Value = serde_json::json!({ "blob": "x".repeat(300) });
    let formatted = format_opencode_error(Some(&big));
    assert!(formatted.ends_with('…'));
    assert_eq!(formatted.chars().count(), 201);
}

#[test]
fn format_opencode_error_known_labels() {
    for (name, label) in [
        ("ProviderAuthError", "Authentication failed"),
        ("MessageOutputLengthError", "Response too long"),
        ("MessageAbortedError", "Request aborted"),
        ("StructuredOutputError", "Output format error"),
        ("ContextOverflowError", "Context too long"),
        ("UnknownError", "Unknown error"),
    ] {
        let err = serde_json::json!({ "name": name, "data": { "message": "m" } });
        assert_eq!(format_opencode_error(Some(&err)), format!("{label} — m"));
    }
}

#[test]
fn provider_constructs_as_chat_provider_trait_object() {
    let p: Arc<dyn ChatProvider> = Arc::new(OpenCodeProvider::new());
    assert_eq!(p.provider_label(), "OpenCode");
}
