//! Agent-loop tests — scripted loopback SSE servers + a scripted
//! executor, no real network. Split out of `chat_agent_loop.rs` to
//! honor the 800-line cap.

#![cfg(test)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::chat_runtime::shared_runtime;
use op_ai::chat_provider::{ChatToolDef, ChatToolExecutor, ChatToolResult};

/// Executor double — records calls, replays a fixed result.
struct ScriptedExecutor {
    calls: Mutex<Vec<(String, String)>>,
    result: ChatToolResult,
}

impl ScriptedExecutor {
    fn ok(content: &str) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            result: ChatToolResult {
                content: content.to_string(),
                is_error: false,
            },
        })
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

impl ChatToolExecutor for ScriptedExecutor {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult {
        self.calls
            .lock()
            .unwrap()
            .push((name.to_string(), args_json.to_string()));
        self.result.clone()
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

    let first = req_rx.recv().expect("first request captured");
    assert!(first.contains(r#""tools""#));
    assert!(first.contains(r#""role":"system""#));
    assert!(first.contains(r#""role":"assistant""#), "history seeded");
    let second = req_rx.recv().expect("second request captured");
    assert!(second.contains(r#""role":"tool""#));
    assert!(second.contains("call_1"));
    assert!(second.contains("deleted"));
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
