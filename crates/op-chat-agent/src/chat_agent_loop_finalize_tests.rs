//! Finalize-lifecycle invariant regression tests (0718-1-k3-1 postmortem).
//!
//! Root cause, confirmed by reading `chat_agent_loop.rs`: the mid-stream
//! `error` SSE event sets `collector.error`, but `pump_sse` itself still
//! returns `Ok(())` (it drained the stream fine) — the CALLER then reads
//! `collector.error` and does `return Err(err)`, one line past the loop's
//! own `run_loop_finalize` call. These tests reproduce that exact shape
//! (an `error` event mid-stream, not a hard connection failure) and assert
//! the new outer-wrapper backstop (`run_anthropic_agent_loop` /
//! `run_openai_agent_loop`) closes the gap.

use super::tests::{run_loop_collect, serve_sse_script, update_node_tool_def, ScriptedExecutor};
use super::*;

/// OpenAI-compatible mid-stream error event — the exact shape the
/// 0718-1-k3-1 transcript's "openai-compatible http 400" line matched
/// (`OpenAiCollector::handle` reads `/error/message` off ANY event, no
/// `type` wrapper needed).
fn openai_error_turn() -> String {
    [
        r#"data: {"choices":[{"delta":{"content":"partial"}}]}"#,
        "",
        r#"data: {"error":{"message":"simulated openai-compatible http 400"}}"#,
        "",
        "",
    ]
    .join("\n")
}

/// Anthropic mid-stream `error` SSE event shape (`AnthropicCollector::handle`
/// matches `"type":"error"` and reads `/error/message`).
fn anthropic_error_turn() -> String {
    [
        r#"data: {"type":"message_start"}"#,
        "",
        r#"data: {"type":"error","error":{"message":"simulated anthropic stream error"}}"#,
        "",
        "",
    ]
    .join("\n")
}

fn base_cfg(
    base: &str,
    executor: std::sync::Arc<ScriptedExecutor>,
    anthropic: bool,
) -> AgentLoopConfig {
    AgentLoopConfig {
        url: if anthropic {
            format!("{base}/v1/messages")
        } else {
            format!("{base}/chat/completions")
        },
        api_key: "sk-test".into(),
        model: "test-model".into(),
        system_prompt: String::new(),
        history: Vec::new(),
        user_prompt: "trigger a mid-stream error".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor,
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    }
}

#[test]
fn openai_err_exit_still_runs_finalize_backstop_and_emits_diagnostic() {
    let (base, _req_rx) = serve_sse_script(vec![openai_error_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let cfg = base_cfg(&base, executor.clone(), false);
    let (outcome, deltas) = run_loop_collect(cfg, false);

    assert!(
        outcome.is_err(),
        "a mid-stream error must still surface as Err to the caller"
    );
    assert_eq!(
        executor.finalizes(),
        1,
        "the outer wrapper's backstop must run the Step-4 structural finalize \
         exactly once on the Err exit the inner loop's own paths never reached"
    );
    assert!(
        deltas.iter().any(|d| matches!(
            d,
            ChatDelta::TextDelta(text) if text.contains("finalize ran") && text.contains("loop-exit")
        )),
        "the loop-exit diagnostic signal must land in the transcript: {deltas:?}"
    );
}

#[test]
fn anthropic_err_exit_still_runs_finalize_backstop_and_emits_diagnostic() {
    let (base, _req_rx) = serve_sse_script(vec![anthropic_error_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let cfg = base_cfg(&base, executor.clone(), true);
    let (outcome, deltas) = run_loop_collect(cfg, true);

    assert!(
        outcome.is_err(),
        "a mid-stream error must still surface as Err to the caller"
    );
    assert_eq!(
        executor.finalizes(),
        1,
        "the outer wrapper's backstop must run the Step-4 structural finalize \
         exactly once on the Err exit the inner loop's own paths never reached"
    );
    assert!(
        deltas.iter().any(|d| matches!(
            d,
            ChatDelta::TextDelta(text) if text.contains("finalize ran") && text.contains("loop-exit")
        )),
        "the loop-exit diagnostic signal must land in the transcript: {deltas:?}"
    );
}

/// A plain (non-design) chat turn hitting the SAME mid-stream error must
/// NOT run the document-mutating backstop or emit the diagnostic — mirrors
/// `chat_agent_loop_tests.rs::loop_skips_finalize_when_disabled_for_plain_chat`'s
/// own invariant, extended to the Err exit path.
#[test]
fn err_exit_with_finalize_disabled_runs_neither_backstop_nor_diagnostic() {
    let (base, _req_rx) = serve_sse_script(vec![openai_error_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let mut cfg = base_cfg(&base, executor.clone(), false);
    cfg.finalize_on_exit = false;
    let (outcome, deltas) = run_loop_collect(cfg, false);

    assert!(outcome.is_err());
    assert_eq!(
        executor.finalizes(),
        0,
        "a regular chat turn (finalize_on_exit=false) must NOT run the \
         document-mutating backstop even on an Err exit"
    );
    assert!(
        !deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(text) if text.contains("finalize ran"))),
        "no diagnostic signal for a plain chat turn: {deltas:?}"
    );
}
