//! Tests for the headless agentic tool-loop (`OPENPENCIL_SMOKE_LOOP=1`).
//!
//! Two layers of proof:
//!
//! 1. [`headless_executor_batch_design_mutates_shared_state`] — the
//!    [`HeadlessExecutor`] really applies a `batch_design` call to the
//!    shared `EditorState` (the load-bearing tool) and the result is the
//!    truthful success envelope.
//! 2. [`loop_mode_against_mock_provider_produces_real_op`] — a full
//!    `run_loop` against a scripted loopback OpenAI-compat SSE server that
//!    emits one `batch_design` tool call then stops. Proves the loop
//!    transport wires end-to-end and the mutated `EditorState` carries the
//!    inserted node — i.e. a real `.op` would be non-empty.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use op_ai::chat_provider::{ChatToolExecutor, EffortLevel, ThinkingMode};
use op_editor_core::EditorState;

use crate::loop_mode::{run_loop, HeadlessExecutor};

/// A `batch_design` call that inserts one 200×120 frame at the root.
const BATCH_DESIGN_ARGS: &str =
    r#"{"operations":"root=I(null,{type:'frame',width:200,height:120,name:'Card'})"}"#;

#[test]
fn headless_executor_batch_design_mutates_shared_state() {
    let state = Arc::new(Mutex::new(EditorState::new()));
    let exec = HeadlessExecutor::new(state.clone(), false);

    let result = exec.execute("batch_design", BATCH_DESIGN_ARGS);
    assert!(
        !result.is_error,
        "batch_design must succeed, got: {}",
        result.content
    );
    let v: serde_json::Value = serde_json::from_str(&result.content).expect("valid JSON envelope");
    assert_eq!(v["success"], serde_json::Value::Bool(true));

    // The shared state really mutated — a frame now exists at the root.
    let guard = state.lock().unwrap();
    assert!(
        !guard.active_children().is_empty(),
        "shared EditorState must carry the inserted frame after batch_design"
    );
}

#[test]
fn headless_executor_spawn_agents_is_honestly_deferred() {
    // spawn_agents can't run headlessly (desktop-only sub-loop launch). The
    // executor must return an HONEST structured error — never a faked success.
    let state = Arc::new(Mutex::new(EditorState::new()));
    let exec = HeadlessExecutor::new(state, false);

    let result = exec.execute("spawn_agents", r#"{"agents":[]}"#);
    assert!(result.is_error, "spawn_agents must be reported as an error");
    let v: serde_json::Value = serde_json::from_str(&result.content).expect("valid JSON envelope");
    assert_eq!(v["success"], serde_json::Value::Bool(false));
    assert!(
        v["error"].as_str().unwrap_or("").contains("unavailable"),
        "error must explain spawn_agents is unavailable headlessly, got: {}",
        result.content
    );
}

#[test]
fn headless_executor_finalize_runs_loop_finalize() {
    // finalize() must not panic and must operate on the shared state. Insert a
    // frame first so apply_loop_finalize has a real tree to walk.
    let state = Arc::new(Mutex::new(EditorState::new()));
    let exec = HeadlessExecutor::new(state.clone(), false);
    let _ = exec.execute("batch_design", BATCH_DESIGN_ARGS);
    exec.finalize();
    // The frame survives finalize (the structural backstop never empties a doc).
    assert!(
        !state.lock().unwrap().active_children().is_empty(),
        "design must survive finalize"
    );
}

/// Read a full HTTP request (headers + content-length body) from `stream`.
fn read_http_request(stream: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let n = stream.read(&mut chunk).expect("read request");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let Some(header_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_len = headers
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        if buf.len() >= header_end + 4 + content_len {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

/// Write an OpenAI-compatible streaming SSE response with `body` as the
/// `data:`-framed payload.
fn write_sse(stream: &mut TcpStream, body: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).expect("write SSE");
}

#[test]
fn loop_mode_against_mock_provider_produces_real_op() {
    // Scripted loopback OpenAI-compat server:
    //   request #1  → stream a `tool_calls` delta calling `batch_design`,
    //                 finish_reason="tool_calls" (the model wants the tool run).
    //   request #2  → after the host runs the tool + sends `tool_result`,
    //                 stream a short text + finish_reason="stop" (loop ends).
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local SSE server");
    let addr = listener.local_addr().expect("local addr");

    let server = std::thread::spawn(move || {
        // ── request #1: emit the batch_design tool call ──
        let (mut stream, _) = listener.accept().expect("accept request 1");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");
        let _ = read_http_request(&mut stream);
        // OpenAI tool_calls streamed in one delta (id + function name + full args),
        // then a finish chunk. Args JSON is escaped for embedding in the SSE.
        let args_escaped = serde_json::to_string(BATCH_DESIGN_ARGS).expect("escape args");
        let call_chunk = format!(
            r#"{{"choices":[{{"index":0,"delta":{{"tool_calls":[{{"index":0,"id":"call_1","type":"function","function":{{"name":"batch_design","arguments":{args_escaped}}}}}]}}}}]}}"#
        );
        let finish_chunk = r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#;
        let body1 = format!("data: {call_chunk}\n\ndata: {finish_chunk}\n\ndata: [DONE]\n\n");
        write_sse(&mut stream, &body1);
        drop(stream);

        // ── request #2: the follow-up after tool_result — end the turn ──
        let (mut stream, _) = listener.accept().expect("accept request 2");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");
        let req2 = read_http_request(&mut stream);
        let text_chunk =
            r#"{"choices":[{"index":0,"delta":{"content":"Done — created the card."}}]}"#;
        let finish_chunk = r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;
        let body2 = format!("data: {text_chunk}\n\ndata: {finish_chunk}\n\ndata: [DONE]\n\n");
        write_sse(&mut stream, &body2);
        drop(stream);
        req2
    });

    let state = run_loop(
        format!("http://{addr}/v1"),
        "sk-test".into(),
        "gpt-test".into(),
        "You are a design agent.".into(),
        "design a card".into(),
        ThinkingMode::Disabled,
        EffortLevel::Low,
        2048,
        false,
        None,
    )
    .expect("loop runs to completion");

    let req2 = server.join().expect("server thread exits");

    // The follow-up request must carry the tool result (proves the loop
    // executed the tool and fed the result back).
    assert!(
        req2.contains("\"role\":\"tool\"") || req2.contains("\"role\": \"tool\""),
        "follow-up request must include the tool_result message, got: {req2}"
    );

    // The shared EditorState really carries the inserted frame — a dumped
    // `.op` would be non-empty with the expected node.
    let guard = state.lock().unwrap();
    let children = guard.active_children();
    assert!(
        !children.is_empty(),
        "loop must have inserted a frame into the live EditorState"
    );

    // Serialize via the SAME path main.rs uses for the `.op` dump and assert
    // the produced document is non-empty and parses back.
    let json = serde_json::to_string_pretty(&guard.doc).expect("serialize doc");
    let reparsed: serde_json::Value = serde_json::from_str(&json).expect("dump re-parses");
    assert!(
        reparsed.get("children").is_some() || reparsed.get("pages").is_some(),
        "dumped .op must carry a node tree"
    );
    // The dump must actually mention the frame we created (non-empty design).
    assert!(
        json.contains("frame"),
        "dumped .op must contain the inserted frame node"
    );
}
