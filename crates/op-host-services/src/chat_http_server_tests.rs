use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

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
    /// Optional gate that holds the create-session response until opened.
    session_create_gate: Option<Arc<AtomicBool>>,
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
        ("POST", "/session") => {
            if let Some(gate) = &scenario.session_create_gate {
                while !gate.load(Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
            write_json(&mut stream, 200, r#"{"id":"ses_mock"}"#);
        }
        ("POST", "/session/ses_mock/message") => write_json(&mut stream, 200, r#"{"info":{}}"#),
        ("POST", "/session/ses_mock/prompt_async") => write_json(&mut stream, 200, "{}"),
        ("POST", "/session/ses_mock/abort") => write_json(&mut stream, 200, "true"),
        ("DELETE", "/session/ses_mock") => write_json(&mut stream, 200, "true"),
        ("GET", "/session/ses_mock/message") => {
            write_json(&mut stream, 200, &scenario.messages_fallback)
        }
        ("GET", "/global/health") => {
            write_json(&mut stream, 200, r#"{"healthy":true,"version":"1.15.0"}"#)
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

fn probe_health_document(body: &'static str) -> bool {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind health probe");
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept health probe");
        let mut reader = BufReader::new(stream.try_clone().expect("clone health probe"));
        let mut line = String::new();
        while reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
            line.clear();
        }
        write_json(&mut stream, 200, body);
    });
    crate::chat_runtime::block_on_anywhere(async move {
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .build()
            .expect("health client");
        probe_server(&client, &base).await
    })
}

fn collect_deltas(server: &MockServer, request: ChatRequest) -> Vec<ChatDelta> {
    let provider = OpenCodeProvider::with_base_url(server.base.clone());
    provider.send(request).collect()
}

fn wait_for_request(server: &MockServer, method: &str, path: &str) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if server
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|(seen_method, seen_path, _)| seen_method == method && seen_path == path)
        {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    false
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
        session_create_gate: None,
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
    assert!(server
        .requests
        .lock()
        .unwrap()
        .iter()
        .any(|(method, path, _)| method == "DELETE" && path == "/session/ses_mock"));

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
    let prompt_json: serde_json::Value = serde_json::from_str(&prompt.2).unwrap();
    assert_eq!(prompt_json["tools"]["*"], false);
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
        session_create_gate: None,
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
        session_create_gate: None,
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
fn opencode_reconciles_a_missing_final_sse_suffix() {
    let scenario = Scenario {
        sse_events: vec![
            r#"{"type":"message.part.delta","properties":{"sessionID":"ses_mock","field":"text","delta":"partial"}}"#.into(),
            r#"{"type":"session.idle","properties":{"sessionID":"ses_mock"}}"#.into(),
        ],
        messages_fallback: r#"[
            {"info":{"role":"assistant"},"parts":[{"type":"text","text":"partial-final"}]}
        ]"#
        .into(),
        session_create_gate: None,
    };
    let server = start_mock(scenario);
    let deltas = collect_deltas(
        &server,
        ChatRequest {
            user_message: "hi".into(),
            ..Default::default()
        },
    );
    let text: String = deltas
        .iter()
        .filter_map(|delta| match delta {
            ChatDelta::TextDelta(text) => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "partial-final", "{deltas:?}");
}

#[test]
fn opencode_aborts_if_a_tool_escapes_the_text_only_contract() {
    let scenario = Scenario {
        sse_events: vec![
            r#"{"type":"message.part.updated","properties":{"part":{"id":"prt_1","sessionID":"ses_mock","messageID":"msg_1","type":"tool","callID":"call_1","tool":"write","state":{"status":"running","input":{},"time":{"start":1}}}}}"#.into(),
        ],
        messages_fallback: "[]".into(),
        session_create_gate: None,
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
        deltas.iter().any(|delta| matches!(
            delta,
            ChatDelta::Error(message) if message.contains("forbidden `write` tool")
        )),
        "{deltas:?}"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::Aborted
        })
    ));
    assert!(server
        .requests
        .lock()
        .unwrap()
        .iter()
        .any(|(method, path, _)| method == "POST" && path == "/session/ses_mock/abort"));
    assert!(server
        .requests
        .lock()
        .unwrap()
        .iter()
        .any(|(method, path, _)| method == "DELETE" && path == "/session/ses_mock"));
}

#[test]
fn dropping_receiver_aborts_and_deletes_session_on_reused_server() {
    let server = start_mock(Scenario {
        // No terminal event: dropping the iterator is the only reason this
        // turn ends, which reproduces Stop/New Chat against a reused server.
        sse_events: vec![r#"{"type":"server.connected","properties":{}}"#.into()],
        messages_fallback: "[]".into(),
        session_create_gate: None,
    });
    let provider = OpenCodeProvider::with_base_url(server.base.clone());
    let turn = provider.send(ChatRequest {
        user_message: "keep running".into(),
        ..Default::default()
    });
    assert!(
        wait_for_request(&server, "POST", "/session/ses_mock/prompt_async"),
        "prompt must be running before the receiver is dropped"
    );

    drop(turn);

    assert!(
        wait_for_request(&server, "POST", "/session/ses_mock/abort"),
        "receiver drop must abort work even when OpenPencil did not spawn the server"
    );
    assert!(
        wait_for_request(&server, "DELETE", "/session/ses_mock"),
        "the temporary integration session must be deleted after cancellation"
    );
}

#[test]
fn cancellable_send_aborts_and_deletes_a_silent_reused_server_session() {
    let server = start_mock(Scenario {
        sse_events: vec![r#"{"type":"server.connected","properties":{}}"#.into()],
        messages_fallback: "[]".into(),
        session_create_gate: None,
    });
    let provider = OpenCodeProvider::with_base_url(server.base.clone());
    let cancel = Arc::new(AtomicBool::new(false));
    let mut turn = provider.send_cancellable(
        ChatRequest {
            user_message: "keep running".into(),
            ..Default::default()
        },
        Arc::clone(&cancel),
    );
    assert!(
        wait_for_request(&server, "POST", "/session/ses_mock/prompt_async"),
        "prompt must be running before cancellation"
    );

    cancel.store(true, Ordering::Release);
    assert!(
        turn.next().is_none(),
        "cooperative cancellation must end the blocking iterator"
    );

    assert!(
        wait_for_request(&server, "POST", "/session/ses_mock/abort"),
        "cancellation must wake tx.closed and abort the reused server session"
    );
    assert!(
        wait_for_request(&server, "DELETE", "/session/ses_mock"),
        "cancellation must delete the integration-owned session"
    );
}

#[test]
fn receiver_drop_cancels_pending_session_create_and_cleans_reused_server() {
    let create_gate = Arc::new(AtomicBool::new(false));
    let server = start_mock(Scenario {
        sse_events: Vec::new(),
        messages_fallback: "[]".into(),
        session_create_gate: Some(Arc::clone(&create_gate)),
    });
    let (tx, rx) = mpsc::channel::<ChatDelta>(1);
    let base = server.base.clone();
    let worker = shared_runtime().spawn(async move {
        let mut spawned = None;
        run_opencode_turn(
            &tx,
            "opencode",
            Some(base),
            "",
            vec![serde_json::json!({ "type": "text", "text": "hi" })],
            None,
            &mut spawned,
        )
        .await;
        assert!(spawned.is_none());
    });
    assert!(
        wait_for_request(&server, "POST", "/session"),
        "session creation must be in flight before cancellation"
    );

    drop(rx);
    let stopped = crate::chat_runtime::block_on_anywhere(async {
        tokio::time::timeout(std::time::Duration::from_millis(500), worker).await
    });
    create_gate.store(true, Ordering::Release);
    assert!(
        matches!(stopped, Ok(Ok(()))),
        "receiver drop must end the turn while session creation is pending: {stopped:?}"
    );
    assert!(
        wait_for_request(&server, "POST", "/session/ses_mock/abort"),
        "an accepted create on the reused server must be aborted"
    );
    assert!(
        wait_for_request(&server, "DELETE", "/session/ses_mock"),
        "an accepted create on the reused server must be deleted"
    );
}

#[test]
fn opencode_idle_with_no_output_emits_empty_response_error() {
    let scenario = Scenario {
        sse_events: vec![r#"{"type":"session.idle","properties":{"sessionID":"ses_mock"}}"#.into()],
        messages_fallback: "[]".into(),
        session_create_gate: None,
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
fn spawned_server_keeps_a_kill_on_drop_guard() {
    let source = include_str!("chat_http_server_startup.rs");
    let spawn_body = source
        .split_once("async fn spawn_opencode_server")
        .expect("spawn function exists")
        .1;
    assert!(
        spawn_body.contains("cmd.kill_on_drop(true);"),
        "runtime/task abort must not detach the spawned OpenCode server"
    );
}

#[test]
fn parse_server_url_matches_ts_handshake() {
    assert_eq!(
        parse_server_url("opencode server listening on http://127.0.0.1:4096").as_deref(),
        Some("http://127.0.0.1:4096")
    );
    // Prefix is mandatory (TS startsWith check).
    assert!(parse_server_url("server listening on http://127.0.0.1:1").is_none());
    assert!(parse_server_url("Warning: OPENCODE_SERVER_PASSWORD is not set").is_none());
    assert!(parse_server_url("opencode server listening on https://127.0.0.1:4096").is_none());
    assert!(parse_server_url("opencode server listening on http://example.com:4096").is_none());
    assert!(parse_server_url("opencode server listening on http://127.0.0.1:4096/other").is_none());
}

#[test]
fn existing_server_probe_requires_documented_health_identity() {
    assert!(probe_health_document(
        r#"{"healthy":true,"version":"1.15.0"}"#
    ));
    assert!(!probe_health_document(r#"{"healthy":true}"#));
    assert!(!probe_health_document(
        r#"{"healthy":false,"version":"1.15.0"}"#
    ));
    assert!(!probe_health_document(r#"{"providers":[]}"#));
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
    assert!(p.supports_cancellable_send());
}
