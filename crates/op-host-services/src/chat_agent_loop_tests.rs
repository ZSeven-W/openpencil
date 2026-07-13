//! Agent-loop tests — scripted loopback SSE servers + a scripted
//! executor, no real network. Split out of `chat_agent_loop.rs` to
//! honor the 800-line cap.

#![cfg(test)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::chat_runtime::shared_runtime;
use base64::Engine as _;
use op_ai::chat_provider::{ChatToolDef, ChatToolExecutor, ChatToolResult};

/// Executor double — records calls + loop-finalize invocations, replays a
/// fixed result.
struct ScriptedExecutor {
    calls: Mutex<Vec<(String, String)>>,
    finalize_calls: std::sync::atomic::AtomicUsize,
    results: Mutex<VecDeque<ChatToolResult>>,
}

impl ScriptedExecutor {
    fn ok(content: &str) -> Arc<Self> {
        Self::sequence(&[content])
    }

    fn sequence(contents: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            finalize_calls: std::sync::atomic::AtomicUsize::new(0),
            results: Mutex::new(
                contents
                    .iter()
                    .map(|content| ChatToolResult {
                        content: (*content).to_string(),
                        is_error: false,
                    })
                    .collect(),
            ),
        })
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }

    /// How many times the loop ran the Step-4 structural backstop.
    fn finalizes(&self) -> usize {
        self.finalize_calls
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl ChatToolExecutor for ScriptedExecutor {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult {
        self.calls
            .lock()
            .unwrap()
            .push((name.to_string(), args_json.to_string()));
        let mut results = self.results.lock().unwrap();
        if results.len() > 1 {
            results.pop_front().expect("scripted result")
        } else {
            results.front().expect("scripted result").clone()
        }
    }

    fn finalize(&self) {
        self.finalize_calls
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

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

/// Serve `bodies.len()` sequential connections, each answered with the
/// corresponding SSE body. Captured request payloads ride `req_rx`.
fn serve_sse_script(bodies: Vec<String>) -> (String, std_mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock SSE server");
    let addr = listener.local_addr().expect("local addr");
    let (req_tx, req_rx) = std_mpsc::channel::<String>();
    std::thread::spawn(move || {
        for body in bodies {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");
            let request = read_http_request(&mut stream);
            let _ = req_tx.send(request);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write SSE response");
        }
    });
    (format!("http://{addr}"), req_rx)
}

fn update_node_tool_def() -> ChatToolDef {
    ChatToolDef {
        name: "update_node".into(),
        description: "Update properties of an existing node by ID".into(),
        level: "modify".into(),
        input_schema_json: r#"{"type":"object","properties":{"nodeId":{"type":"string"}}}"#.into(),
    }
}

fn delete_node_tool_def() -> ChatToolDef {
    ChatToolDef {
        name: "delete_node".into(),
        description: "Delete a node".into(),
        level: "delete".into(),
        input_schema_json: r#"{"type":"object","properties":{"nodeId":{"type":"string"}}}"#.into(),
    }
}

fn get_screenshot_tool_def() -> ChatToolDef {
    ChatToolDef {
        name: "get_screenshot".into(),
        description: "Render a node to a base64 PNG".into(),
        level: "read".into(),
        input_schema_json: r#"{"type":"object","properties":{"nodeId":{"type":"string"}}}"#.into(),
    }
}

/// A 1×1 transparent PNG, base64-encoded — stands in for a rendered design
/// so the tests can decode it back and prove it round-trips through the wire.
const TINY_PNG_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";
const SECOND_TINY_PNG_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9Zl9sAAAAASUVORK5CYII=";

fn anthropic_tool_use_turn() -> String {
    [
        r#"data: {"type":"message_start"}"#,
        "",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"update_node"}}"#,
        "",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"nodeId\":\"n1\","}}"#,
        "",
        r##"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"\"fill_hex\":\"#ff0000\"}"}}"##,
        "",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"}}"#,
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
        "",
    ]
    .join("\n")
}

fn anthropic_get_screenshot_turn() -> String {
    [
        r#"data: {"type":"message_start"}"#,
        "",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_shot","name":"get_screenshot"}}"#,
        "",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"nodeId\":\"root\"}"}}"#,
        "",
        r#"data: {"type":"content_block_stop","index":0}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"tool_use"}}"#,
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
        "",
    ]
    .join("\n")
}

fn anthropic_text_turn() -> String {
    [
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Recolored the title."}}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#,
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
        "",
    ]
    .join("\n")
}

fn run_loop_collect(
    cfg: AgentLoopConfig,
    anthropic: bool,
) -> (Result<bool, String>, Vec<ChatDelta>) {
    let (tx, mut rx) = mpsc::channel::<ChatDelta>(64);
    let outcome = shared_runtime().block_on(async {
        if anthropic {
            run_anthropic_agent_loop(cfg, &tx).await
        } else {
            run_openai_agent_loop(cfg, &tx).await
        }
    });
    drop(tx);
    let mut deltas = Vec::new();
    while let Ok(d) = rx.try_recv() {
        deltas.push(d);
    }
    (outcome, deltas)
}

#[test]
fn anthropic_loop_executes_tool_and_continues_with_tool_result() {
    let (base, req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{"wrote":"true"}}"#);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: "You are a design editor.".into(),
        history: vec![(ChatHistoryRole::User, "earlier turn".into())],
        user_prompt: "make the title red".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor: executor.clone(),
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
    };
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    // Executor ran exactly once with the accumulated input_json.
    assert_eq!(
        executor.calls(),
        vec![(
            "update_node".to_string(),
            r##"{"nodeId":"n1","fill_hex":"#ff0000"}"##.to_string()
        )]
    );

    // Delta order: ToolUse card (running envelope, level from the
    // def), then the follow-up text, then a terminal Done.
    let tool_use = deltas
        .iter()
        .find_map(|d| match d {
            ChatDelta::ToolUse { name, args } => Some((name.clone(), args.clone())),
            _ => None,
        })
        .expect("loop must surface the tool call to the transcript");
    assert_eq!(tool_use.0, "update_node");
    assert!(tool_use.1.contains("\"status\":\"running\""));
    assert!(tool_use.1.contains("\"level\":\"modify\""));
    assert!(deltas
        .iter()
        .any(|d| matches!(d, ChatDelta::TextDelta(s) if s == "Recolored the title.")));
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));

    // Step-4 backstop ran ONCE at the normal model-stop exit.
    assert_eq!(
        executor.finalizes(),
        1,
        "loop must run the structural backstop once at the normal model-stop exit"
    );

    // First request: tools advertised + history + system prompt.
    let first = req_rx.recv().expect("first request captured");
    assert!(first.contains(r#""tools""#));
    assert!(first.contains("update_node"));
    assert!(first.contains("earlier turn"));
    assert!(first.contains("You are a design editor."));
    // Second request: the tool_result rides back, tied to the call id.
    let second = req_rx.recv().expect("second request captured");
    assert!(second.contains(r#""tool_result""#));
    assert!(second.contains("toolu_1"));
    assert!(second.contains(r#"\"wrote\""#) || second.contains("wrote"));
}

#[test]
fn loop_skips_finalize_when_disabled_for_plain_chat() {
    // The agent loop is the shared execution path for BOTH the gated
    // design loop AND ordinary builtin chat. `finalize_on_exit: false`
    // (the regular-chat default) MUST keep the loop from running the
    // document-mutating Step-4 backstop — otherwise every plain chat
    // turn would silently re-finalize the user's existing design.
    let (base, _req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: String::new(),
        history: Vec::new(),
        user_prompt: "just chatting".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor: executor.clone(),
        max_turns: 5,
        finalize_on_exit: false,
        disable_thinking: false,
    };
    let (outcome, _deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));
    assert_eq!(
        executor.finalizes(),
        0,
        "a regular chat turn (finalize_on_exit=false) must NOT run the document-mutating backstop"
    );
}

#[test]
fn anthropic_loop_stops_at_turn_cap_with_max_tokens_reason() {
    // The mock model calls a tool on EVERY turn; the loop must stop at
    // the cap instead of looping forever (TS maxTurns semantics).
    let (base, _req_rx) =
        serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_tool_use_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: String::new(),
        history: Vec::new(),
        user_prompt: "loop forever".into(),
        max_output_tokens: 128,
        tools: vec![update_node_tool_def()],
        executor: executor.clone(),
        max_turns: 2,
        finalize_on_exit: true,
        disable_thinking: false,
    };
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));
    assert_eq!(executor.calls().len(), 2, "one execution per capped turn");
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::MaxTokens
        })
    ));
    // Step-4 backstop ran ONCE at the turn-cap truncation exit too.
    assert_eq!(
        executor.finalizes(),
        1,
        "loop must run the structural backstop once at the turn-cap truncation exit"
    );
}

#[test]
fn openai_loop_executes_tool_and_continues_with_role_tool_message() {
    let tool_turn = [
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"delete_node","arguments":"{\"node"}}]}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"Id\":\"n9\"}"}}]}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    let text_turn = [
        r#"data: {"choices":[{"delta":{"content":"Deleted."}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    let (base, req_rx) = serve_sse_script(vec![tool_turn, text_turn]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{"deleted":true}}"#);
    let cfg = AgentLoopConfig {
        url: format!("{base}/chat/completions"),
        api_key: "sk-test".into(),
        model: "gpt-test".into(),
        system_prompt: "You are a design editor.".into(),
        history: vec![
            (ChatHistoryRole::User, "hi".into()),
            (ChatHistoryRole::Assistant, "hello".into()),
        ],
        user_prompt: "delete the badge".into(),
        max_output_tokens: 512,
        tools: vec![delete_node_tool_def()],
        executor: executor.clone(),
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
    };
    let (outcome, deltas) = run_loop_collect(cfg, false);
    assert_eq!(outcome, Ok(true));

    assert_eq!(
        executor.calls(),
        vec![("delete_node".to_string(), r#"{"nodeId":"n9"}"#.to_string())]
    );
    assert!(deltas.iter().any(
        |d| matches!(d, ChatDelta::ToolUse { name, args } if name == "delete_node" && args.contains("\"level\":\"delete\""))
    ));
    assert!(deltas
        .iter()
        .any(|d| matches!(d, ChatDelta::TextDelta(s) if s == "Deleted.")));
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));
    // OpenAI-wire loop also runs the Step-4 backstop once at loop end.
    assert_eq!(
        executor.finalizes(),
        1,
        "openai-wire loop must run the structural backstop once at loop end"
    );

    let first = req_rx.recv().expect("first request captured");
    assert!(first.contains(r#""tools""#));
    assert!(first.contains(r#""role":"system""#));
    assert!(first.contains(r#""role":"assistant""#), "history seeded");
    let second = req_rx.recv().expect("second request captured");
    assert!(second.contains(r#""role":"tool""#));
    assert!(second.contains("call_1"));
    assert!(second.contains("deleted"));
}

// ── Step 1: get_screenshot tool_result becomes a real image content block ──

#[test]
fn screenshot_image_base64_extracts_from_png_result_only() {
    // Successful screenshot result → Some(b64).
    let ok = ChatToolResult {
        content: serde_json::json!({ "image_base64": "AAAB", "format": "png" }).to_string(),
        is_error: false,
    };
    assert_eq!(
        super::screenshot_image_base64("get_screenshot", &ok),
        Some("AAAB".to_string())
    );

    // Wrong tool name → None (a non-screenshot result must stay text).
    assert_eq!(
        super::screenshot_image_base64("get_editor_state", &ok),
        None
    );

    // Error envelope → None (never replay an error as an image).
    let err = ChatToolResult {
        content: ok.content.clone(),
        is_error: true,
    };
    assert_eq!(super::screenshot_image_base64("get_screenshot", &err), None);

    // Missing/empty base64 or wrong format → None.
    let empty = ChatToolResult {
        content: serde_json::json!({ "image_base64": "", "format": "png" }).to_string(),
        is_error: false,
    };
    assert_eq!(
        super::screenshot_image_base64("get_screenshot", &empty),
        None
    );
    let not_png = ChatToolResult {
        content: serde_json::json!({ "image_base64": "AAAB", "format": "jpeg" }).to_string(),
        is_error: false,
    };
    assert_eq!(
        super::screenshot_image_base64("get_screenshot", &not_png),
        None
    );
    // Non-JSON content (e.g. a plain CRUD string) → None.
    let plain = ChatToolResult {
        content: "wrote ok".to_string(),
        is_error: false,
    };
    assert_eq!(
        super::screenshot_image_base64("get_screenshot", &plain),
        None
    );
}

#[test]
fn anthropic_loop_replays_screenshot_result_as_image_content_block() {
    let (base, req_rx) =
        serve_sse_script(vec![anthropic_get_screenshot_turn(), anthropic_text_turn()]);
    // The executor returns exactly what the real get_screenshot MCP tool returns.
    // Match the real MCP dispatch envelope, not the old bare-payload test
    // double that hid the production extraction bug.
    let screenshot_result = serde_json::json!({
        "success": true,
        "data": { "image_base64": TINY_PNG_B64, "format": "png" }
    })
    .to_string();
    let executor = ScriptedExecutor::ok(&screenshot_result);
    let cfg = AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "render and check the design".into(),
        max_output_tokens: 512,
        tools: vec![get_screenshot_tool_def()],
        executor: executor.clone(),
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
    };
    let (outcome, _deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));
    assert_eq!(executor.calls().len(), 1, "screenshot executed once");

    // First request advertises get_screenshot.
    let _first = req_rx.recv().expect("first request captured");

    // Second request carries the tool_result. PROVE it is a real image
    // content block, NOT an opaque base64 text string.
    let second = req_rx.recv().expect("second request captured");
    let body_start = second
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .expect("request has a body");
    let body: Value = serde_json::from_str(&second[body_start..]).expect("request body is JSON");
    let messages = body["messages"].as_array().expect("messages array");
    // The tool_result lives in the last user message's content array.
    let last_user = messages
        .iter()
        .rev()
        .find(|m| m["role"] == "user")
        .expect("a user message carries the tool_result");
    let tr = last_user["content"]
        .as_array()
        .expect("tool_result content array")
        .iter()
        .find(|c| c["type"] == "tool_result")
        .expect("a tool_result block");
    assert_eq!(tr["tool_use_id"], "toolu_shot");
    let inner = tr["content"]
        .as_array()
        .expect("tool_result content must be a multimodal array, not a string");
    let image = inner
        .iter()
        .find(|c| c["type"] == "image")
        .expect("a real image content block");
    assert_eq!(image["source"]["type"], "base64");
    assert_eq!(image["source"]["media_type"], "image/png");
    // The base64 round-trips intact, and decodes to a valid PNG.
    let data = image["source"]["data"].as_str().expect("base64 data");
    assert_eq!(data, TINY_PNG_B64);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("decodable base64");
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
        "model receives a decodable PNG"
    );

    // And the raw base64 must NOT appear as a bare JSON string value
    // (i.e. the old text-string path is gone).
    assert!(
        !second.contains(&format!(r#""content":"{TINY_PNG_B64}"#)),
        "screenshot base64 must not ride as an opaque text string"
    );
}

#[test]
fn openai_loop_replays_screenshot_result_as_image_url_part() {
    let shot_turn = [
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_shot","type":"function","function":{"name":"get_screenshot","arguments":"{\"nodeId\":\"root\"}"}}]}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    let text_turn = [
        r#"data: {"choices":[{"delta":{"content":"Looks good."}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    let (base, req_rx) = serve_sse_script(vec![shot_turn, text_turn]);
    let screenshot_result = serde_json::json!({
        "success": true,
        "data": { "image_base64": TINY_PNG_B64, "format": "png" }
    })
    .to_string();
    let executor = ScriptedExecutor::ok(&screenshot_result);
    let cfg = AgentLoopConfig {
        url: format!("{base}/chat/completions"),
        api_key: "sk-test".into(),
        model: "gpt-test".into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "render and check".into(),
        max_output_tokens: 512,
        tools: vec![get_screenshot_tool_def()],
        executor: executor.clone(),
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
    };
    let (outcome, _deltas) = run_loop_collect(cfg, false);
    assert_eq!(outcome, Ok(true));
    assert_eq!(executor.calls().len(), 1);

    let _first = req_rx.recv().expect("first request captured");
    let second = req_rx.recv().expect("second request captured");
    let body_start = second.find("\r\n\r\n").map(|i| i + 4).expect("body");
    let body: Value = serde_json::from_str(&second[body_start..]).expect("body JSON");
    let messages = body["messages"].as_array().expect("messages");

    // The role:"tool" message holds only a short text ack (string).
    let tool_msg = messages
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message");
    assert!(
        tool_msg["content"].is_string(),
        "openai tool content must stay a string"
    );

    // A follow-up role:"user" message carries the image as an image_url
    // data URL — the only OpenAI-wire way to make the model see the render.
    let img_user = messages
        .iter()
        .filter(|m| m["role"] == "user")
        .find_map(|m| m["content"].as_array())
        .expect("a user message with multimodal content parts");
    let part = img_user
        .iter()
        .find(|p| p["type"] == "image_url")
        .expect("an image_url part");
    let url = part["image_url"]["url"].as_str().expect("data url");
    let expected_prefix = "data:image/png;base64,";
    assert!(url.starts_with(expected_prefix), "got {url}");
    assert_eq!(&url[expected_prefix.len()..], TINY_PNG_B64);
}

#[test]
fn openai_loop_keeps_only_latest_screenshot_without_dropping_user_intent() {
    let shot_turn = |id: &str| {
        [
            format!(
                r#"data: {{"choices":[{{"delta":{{"tool_calls":[{{"index":0,"id":"{id}","type":"function","function":{{"name":"get_screenshot","arguments":"{{\"nodeId\":\"root\"}}"}}}}]}}}}]}}"#
            ),
            String::new(),
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#.into(),
            String::new(),
            "data: [DONE]".into(),
            String::new(),
            String::new(),
        ]
        .join("\n")
    };
    let text_turn = [
        r#"data: {"choices":[{"delta":{"content":"Visual check complete."}}]}"#,
        "",
        r#"data: {"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "",
        "data: [DONE]",
        "",
        "",
    ]
    .join("\n");
    let (base, req_rx) = serve_sse_script(vec![
        shot_turn("call_shot_1"),
        shot_turn("call_shot_2"),
        text_turn,
    ]);
    let first_screenshot_result = serde_json::json!({
        "success": true,
        "data": { "image_base64": TINY_PNG_B64, "format": "png" }
    })
    .to_string();
    let second_screenshot_result = serde_json::json!({
        "success": true,
        "data": { "image_base64": SECOND_TINY_PNG_B64, "format": "png" }
    })
    .to_string();
    let executor = ScriptedExecutor::sequence(&[
        first_screenshot_result.as_str(),
        second_screenshot_result.as_str(),
    ]);
    let cfg = AgentLoopConfig {
        url: format!("{base}/chat/completions"),
        api_key: "sk-test".into(),
        model: "MiniMax-M3".into(),
        system_prompt: "You are a design editor.".into(),
        history: Vec::new(),
        user_prompt: "Build the exact coffee landing page I requested".into(),
        max_output_tokens: 6_144,
        tools: vec![get_screenshot_tool_def()],
        executor,
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
    };
    let (outcome, _deltas) = run_loop_collect(cfg, false);
    assert_eq!(outcome, Ok(true));

    let _first = req_rx.recv().expect("initial request");
    let _second = req_rx.recv().expect("first screenshot request");
    let third = req_rx.recv().expect("second screenshot request");
    let body_start = third.find("\r\n\r\n").map(|i| i + 4).expect("body");
    let body: Value = serde_json::from_str(&third[body_start..]).expect("body JSON");
    let wire = body["messages"].to_string();

    assert_eq!(
        wire.matches("data:image/png;base64,").count(),
        1,
        "only the newest visual observation may ride the next request"
    );
    assert!(
        !wire.contains(TINY_PNG_B64),
        "the superseded first screenshot payload must be absent"
    );
    assert_eq!(wire.matches(SECOND_TINY_PNG_B64).count(), 1);
    assert!(wire.contains(crate::chat_agent_context::ELIDED_SCREENSHOT_TEXT));
    assert!(
        wire.contains("Build the exact coffee landing page I requested"),
        "context compaction must preserve the user's design intent"
    );
    assert!(wire.contains("call_shot_1"), "tool identity is preserved");
    assert!(
        wire.contains("call_shot_2"),
        "latest tool call is preserved"
    );
}

#[test]
fn tool_card_envelope_wraps_args_with_level_and_running_status() {
    let envelope = tool_card_envelope("modify", r#"{"nodeId":"n1"}"#);
    let v: Value = serde_json::from_str(&envelope).unwrap();
    assert_eq!(v["level"], "modify");
    assert_eq!(v["status"], "running");
    assert_eq!(v["args"]["nodeId"], "n1");
    // Unparseable args degrade to a string payload instead of panicking.
    let envelope = tool_card_envelope("read", "not-json");
    let v: Value = serde_json::from_str(&envelope).unwrap();
    assert_eq!(v["args"], "not-json");
}

#[path = "chat_agent_loop_text_only_tests.rs"]
mod text_only_tests;
