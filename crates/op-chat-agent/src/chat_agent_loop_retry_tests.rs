//! Regression coverage for the design-loop corrective write retry.

#![cfg(test)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatToolDef, ChatToolExecutor, ChatToolResult, StopReason,
};
use serde_json::{json, Value};

use super::tests::{run_loop_collect, serve_sse_script};
use super::AgentLoopConfig;

struct RetryExecutor {
    calls: Mutex<Vec<(String, String)>>,
    results: Mutex<VecDeque<ChatToolResult>>,
}

impl RetryExecutor {
    fn new(results: Vec<ChatToolResult>) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            results: Mutex::new(results.into()),
        })
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().expect("calls lock").clone()
    }
}

impl ChatToolExecutor for RetryExecutor {
    fn execute(&self, name: &str, args_json: &str) -> ChatToolResult {
        self.calls
            .lock()
            .expect("calls lock")
            .push((name.to_string(), args_json.to_string()));
        let mut results = self.results.lock().expect("results lock");
        if results.len() > 1 {
            results.pop_front().expect("scripted tool result")
        } else {
            results.front().expect("scripted tool result").clone()
        }
    }
}

fn result(content: &str, is_error: bool) -> ChatToolResult {
    ChatToolResult {
        content: content.to_string(),
        is_error,
    }
}

fn tool_def(name: &str, level: &str) -> ChatToolDef {
    ChatToolDef {
        name: name.to_string(),
        description: format!("Test tool {name}"),
        level: level.to_string(),
        input_schema_json: r#"{"type":"object"}"#.to_string(),
    }
}

fn cfg(
    base: &str,
    anthropic: bool,
    tool: ChatToolDef,
    executor: Arc<dyn ChatToolExecutor>,
    finalize_on_exit: bool,
) -> AgentLoopConfig {
    AgentLoopConfig {
        url: if anthropic {
            format!("{base}/v1/messages")
        } else {
            format!("{base}/chat/completions")
        },
        api_key: "sk-test".into(),
        model: if anthropic {
            "claude-test".into()
        } else {
            "qwen-test".into()
        },
        system_prompt: "You edit the live design.".into(),
        history: Vec::<(ChatHistoryRole, String)>::new(),
        user_prompt: "apply the requested design change".into(),
        max_output_tokens: 256,
        tools: vec![tool],
        executor,
        // The failed first write consumes the only ordinary turn. A retry
        // succeeds only if it owns the dedicated bounded correction budget.
        max_turns: 1,
        finalize_on_exit,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    }
}

fn anthropic_tool_turn(id: &str, name: &str, args: Value) -> String {
    let events = [
        json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": { "type": "tool_use", "id": id, "name": name }
        }),
        json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "input_json_delta",
                "partial_json": serde_json::to_string(&args).expect("serialize tool args")
            }
        }),
        json!({ "type": "content_block_stop", "index": 0 }),
        json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" } }),
        json!({ "type": "message_stop" }),
    ];
    events
        .into_iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect()
}

fn openai_tool_turn(id: &str, name: &str, args: Value) -> String {
    openai_tool_turns(&[(id, name, args)])
}

fn openai_tool_turns(calls: &[(&str, &str, Value)]) -> String {
    let calls: Vec<Value> = calls
        .iter()
        .enumerate()
        .map(|(index, (id, name, args))| {
            json!({
                "index": index,
                "id": id,
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": serde_json::to_string(args).expect("serialize tool args")
                }
            })
        })
        .collect();
    let event = json!({
        "choices": [{
            "delta": { "tool_calls": calls }
        }]
    });
    let stop = json!({ "choices": [{ "delta": {}, "finish_reason": "tool_calls" }] });
    format!("data: {event}\n\ndata: {stop}\n\ndata: [DONE]\n\n")
}

fn text_count(deltas: &[ChatDelta], needle: &str) -> usize {
    deltas
        .iter()
        .filter(|delta| matches!(delta, ChatDelta::TextDelta(text) if text.contains(needle)))
        .count()
}

fn serve_http_status(code: u16, reason: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind status server");
    let addr = listener.local_addr().expect("status server address");
    let reason = reason.to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("status request");
        let mut request = [0_u8; 8192];
        let _ = stream.read(&mut request);
        let response =
            format!("HTTP/1.1 {code} {reason}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        stream
            .write_all(response.as_bytes())
            .expect("write status response");
    });
    format!("http://{addr}")
}

#[test]
fn anthropic_failed_write_gets_one_model_authored_correction_past_turn_cap() {
    let first_args = json!({ "nodeId": "n1", "height": "fit_content" });
    let corrected_args = json!({ "nodeId": "n1", "height": 420 });
    let (base, req_rx) = serve_sse_script(vec![
        anthropic_tool_turn("toolu_bad", "update_node", first_args.clone()),
        anthropic_tool_turn("toolu_fixed", "update_node", corrected_args.clone()),
    ]);
    let executor = RetryExecutor::new(vec![
        result(
            r#"{"success":false,"error":"height must be an integer"}"#,
            true,
        ),
        result(r#"{"success":true,"data":{"wrote":true}}"#, false),
    ]);

    let (outcome, deltas) = run_loop_collect(
        cfg(
            &base,
            true,
            tool_def("update_node", "modify"),
            executor.clone(),
            true,
        ),
        true,
    );

    assert_eq!(outcome, Ok(true));
    assert_eq!(
        executor.calls(),
        vec![
            ("update_node".into(), first_args.to_string()),
            ("update_node".into(), corrected_args.to_string()),
        ],
        "the host must execute only calls authored by the model, with corrected arguments"
    );
    assert_eq!(text_count(&deltas, "Retrying failed design write"), 1);
    assert_eq!(text_count(&deltas, "still failed after corrective"), 0);
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::MaxTokens
        })
    ));

    let _first_request = req_rx.recv().expect("first provider request");
    let correction_request = req_rx.recv().expect("correction provider request");
    assert!(correction_request.contains("one corrective retry"));
    assert!(correction_request.contains("Do not repeat the same tool arguments unchanged"));
    assert!(correction_request.contains("height must be an integer"));
    assert!(correction_request.contains(r#""is_error":true"#));
}

#[test]
fn openai_second_write_failure_stops_after_the_single_correction() {
    let first_args = json!({ "operations": "bad syntax" });
    let corrected_args = json!({ "script": "still invalid" });
    let second_failed_args = json!({ "operations": "also invalid" });
    let (base, req_rx) = serve_sse_script(vec![
        openai_tool_turn("call_bad", "batch_design", first_args.clone()),
        openai_tool_turns(&[
            ("call_retry_ok", "batch_design", corrected_args.clone()),
            ("call_retry_bad", "batch_design", second_failed_args.clone()),
        ]),
    ]);
    let executor = RetryExecutor::new(vec![
        result(
            r#"{"success":false,"error":"Transaction rolled back","data":{"applied":false,"errors":[{"line":"bad syntax","error":"parse failed"}]}}"#,
            true,
        ),
        result(r#"{"success":true,"data":{"wrote":true}}"#, false),
        result(r#"{"success":false,"error":"script failed"}"#, true),
    ]);

    let (outcome, deltas) = run_loop_collect(
        cfg(
            &base,
            false,
            tool_def("batch_design", "create"),
            executor.clone(),
            true,
        ),
        false,
    );

    assert_eq!(outcome, Ok(true));
    assert_eq!(
        executor.calls(),
        vec![
            ("batch_design".into(), first_args.to_string()),
            ("batch_design".into(), corrected_args.to_string()),
            ("batch_design".into(), second_failed_args.to_string()),
        ],
        "one successful write must not hide a sibling failure, and no third round may run"
    );
    assert_eq!(text_count(&deltas, "Retrying failed design write"), 1);
    assert_eq!(text_count(&deltas, "still failed after corrective"), 1);
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::MaxTokens
        })
    ));

    let _first_request = req_rx.recv().expect("first provider request");
    let correction_request = req_rx.recv().expect("correction provider request");
    assert!(correction_request.contains("one corrective retry"));
    assert!(correction_request.contains(r#""role":"tool""#));
    assert!(correction_request.contains(r#"\"success\":false"#));
    assert!(correction_request.contains("errors"));
    assert!(
        correction_request
            .find(r#""role":"tool""#)
            .expect("tool result position")
            < correction_request
                .find("one corrective retry")
                .expect("nudge position"),
        "the role:tool failure must precede the user correction nudge"
    );
}

#[test]
fn openai_read_failure_does_not_trigger_design_write_correction() {
    let args = json!({ "nodeId": "root" });
    let (base, _req_rx) = serve_sse_script(vec![openai_tool_turn(
        "call_read",
        "get_screenshot",
        args.clone(),
    )]);
    let executor = RetryExecutor::new(vec![result(
        r#"{"success":false,"error":"renderer unavailable"}"#,
        true,
    )]);

    let (outcome, deltas) = run_loop_collect(
        cfg(
            &base,
            false,
            tool_def("get_screenshot", "read"),
            executor.clone(),
            true,
        ),
        false,
    );

    assert_eq!(outcome, Ok(true));
    assert_eq!(
        executor.calls(),
        vec![("get_screenshot".into(), args.to_string())]
    );
    assert_eq!(text_count(&deltas, "Retrying failed design write"), 0);
}

#[test]
fn plain_chat_write_failure_does_not_use_the_design_retry_budget() {
    let args = json!({ "nodeId": "n1" });
    let (base, _req_rx) = serve_sse_script(vec![openai_tool_turn(
        "call_modify",
        "update_node",
        args.clone(),
    )]);
    let executor = RetryExecutor::new(vec![result(
        r#"{"success":false,"error":"not found"}"#,
        true,
    )]);

    let (outcome, deltas) = run_loop_collect(
        cfg(
            &base,
            false,
            tool_def("update_node", "modify"),
            executor.clone(),
            false,
        ),
        false,
    );

    assert_eq!(outcome, Ok(true));
    assert_eq!(
        executor.calls(),
        vec![("update_node".into(), args.to_string())]
    );
    assert_eq!(text_count(&deltas, "Retrying failed design write"), 0);
}

#[test]
fn provider_auth_failures_never_enter_semantic_correction() {
    for (code, reason) in [(401, "Unauthorized"), (403, "Forbidden")] {
        let base = serve_http_status(code, reason);
        let executor = RetryExecutor::new(vec![result(r#"{"success":true,"data":{}}"#, false)]);

        let (outcome, deltas) = run_loop_collect(
            cfg(
                &base,
                false,
                tool_def("batch_design", "create"),
                executor.clone(),
                true,
            ),
            false,
        );

        let error = outcome
            .expect_err("provider auth status must abort before tools")
            .to_string();
        assert!(error.contains(&format!("http {code}")));
        assert!(executor.calls().is_empty());
        assert_eq!(text_count(&deltas, "Retrying failed design write"), 0);
    }
}
