//! Zero-write completion-gate tests (the empty-canvas postmortem): a design
//! run in which the model never applies a write must never exit looking
//! like a clean success. Four shapes: the guard fires one corrective round
//! with the script-first nudge; a run that then writes proceeds to a normal
//! finalize with no zero-write report; a run that stays write-free ends
//! with the honest report (stop reason included); and a normally-writing
//! run is entirely unaffected. Plus the Anthropic-wire regression locks
//! from the same postmortem: the request body carries the low-reasoning
//! control, and a thinking-only (empty-content) assistant turn is replayed
//! as a minimal text block instead of the `content: []` Anthropic rejects.

use super::tests::{
    anthropic_tool_use_turn, run_loop_collect, serve_sse_script, update_node_tool_def,
    ScriptedExecutor,
};
use super::*;

fn anthropic_text_turn() -> String {
    [
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"All read, looks fine."}}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#,
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
        "",
    ]
    .join("\n")
}

/// A thinking-only turn: the model burned its budget on reasoning and
/// stopped without ANY replayable content block — the exact wire shape the
/// starved-reasoning failure produced.
fn anthropic_thinking_only_turn() -> String {
    [
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"thinking"}}"#,
        "",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"planning the layout..."}}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"}}"#,
        "",
        r#"data: {"type":"message_stop"}"#,
        "",
        "",
    ]
    .join("\n")
}

fn base_cfg(
    base: &str,
    executor: std::sync::Arc<ScriptedExecutor>,
    model: &str,
    disable_thinking: bool,
) -> AgentLoopConfig {
    AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: model.into(),
        system_prompt: String::new(),
        history: Vec::new(),
        user_prompt: "design the music app home".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor,
        max_turns: 5,
        finalize_on_exit: true,
        disable_thinking,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    }
}

#[test]
fn zero_write_stop_gets_one_corrective_round_with_the_script_nudge() {
    let (base, req_rx) = serve_sse_script(vec![anthropic_text_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, _deltas) =
        run_loop_collect(base_cfg(&base, executor, "claude-test", false), true);
    assert_eq!(outcome, Ok(true));

    let _first = req_rx.recv().expect("initial request");
    let second = req_rx.recv().expect("zero-write corrective round");
    assert!(
        second.contains("the canvas is EMPTY") && second.contains("batch_design"),
        "the corrective round must restate the script-first build contract, got: {second}"
    );
    assert!(
        second.contains("`script`"),
        "the nudge steers to script-mode generation: {second}"
    );
    assert!(
        req_rx.recv().is_err(),
        "one corrective round only — the guard never loops"
    );
}

#[test]
fn zero_write_round_that_then_writes_finalizes_without_the_report() {
    // Turn 1: read-only stop → guard round. Turn 2: the nudge works — the
    // model writes. Turn 3: normal stop → normal finalize, and the honest
    // zero-write report must NOT appear (the run did write in the end).
    let (base, _req_rx) = serve_sse_script(vec![
        anthropic_text_turn(),
        anthropic_tool_use_turn(),
        anthropic_text_turn(),
    ]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, deltas) = run_loop_collect(
        base_cfg(&base, executor.clone(), "claude-test", false),
        true,
    );
    assert_eq!(outcome, Ok(true));
    assert_eq!(executor.finalizes(), 1);
    assert_eq!(executor.calls().len(), 1, "the rescued write executed");
    assert!(
        !deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("without applying any design write"))),
        "a rescued run must not carry the zero-write report: {deltas:?}"
    );
}

#[test]
fn run_that_stays_write_free_ends_with_the_honest_report_and_stop_reason() {
    let (base, _req_rx) = serve_sse_script(vec![anthropic_text_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, deltas) = run_loop_collect(
        base_cfg(&base, executor.clone(), "claude-test", false),
        true,
    );
    assert_eq!(outcome, Ok(true));
    assert_eq!(executor.finalizes(), 1, "finalize still runs at the exit");

    let report = deltas
        .iter()
        .find_map(|d| match d {
            ChatDelta::TextDelta(s) if s.contains("without applying any design write") => {
                Some(s.clone())
            }
            _ => None,
        })
        .expect("a write-free design run must report honestly");
    assert!(
        report.contains("the canvas is unchanged"),
        "the report states the consequence: {report}"
    );
    assert!(
        report.contains("stop reason: end_turn"),
        "the stop reason rides the report so field failures stay diagnosable: {report}"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));
}

#[test]
fn normally_writing_run_is_unaffected_by_the_guard() {
    let (base, req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, deltas) = run_loop_collect(
        base_cfg(&base, executor.clone(), "claude-test", false),
        true,
    );
    assert_eq!(outcome, Ok(true));

    let _first = req_rx.recv().expect("initial request");
    let _second = req_rx.recv().expect("post-write request");
    assert!(
        req_rx.recv().is_err(),
        "a run that wrote gets no extra corrective round"
    );
    assert!(
        !deltas.iter().any(|d| matches!(
            d,
            ChatDelta::TextDelta(s) if s.contains("without applying any design write")
                || s.contains("the canvas is EMPTY")
        )),
        "no zero-write machinery may surface on a writing run: {deltas:?}"
    );
}

#[test]
fn anthropic_wire_carries_the_low_reasoning_control() {
    // The postmortem's root cause: the OpenAI-compat loop disabled hidden
    // reasoning and the Anthropic loop did not, so a reasoning model on its
    // Anthropic-compatible endpoint burned the whole turn budget thinking.
    let (base, req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, _deltas) =
        run_loop_collect(base_cfg(&base, executor, "deepseek-v4-flash", true), true);
    assert_eq!(outcome, Ok(true));

    let first = req_rx.recv().expect("initial request");
    assert!(
        first.contains(r#""thinking":{"type":"disabled"}"#),
        "the Anthropic body must carry the wire control for a thinking_disabled model: {first}"
    );
    // Every round re-applies it, corrective rounds included.
    let second = req_rx.recv().expect("follow-up request");
    assert!(second.contains(r#""thinking":{"type":"disabled"}"#));
}

#[test]
fn anthropic_wire_control_is_a_no_op_for_models_off_the_whitelist() {
    let (base, req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, _deltas) = run_loop_collect(base_cfg(&base, executor, "claude-test", true), true);
    assert_eq!(outcome, Ok(true));
    let first = req_rx.recv().expect("initial request");
    assert!(
        !first.contains(r#""thinking""#),
        "an unlisted model must not receive a guessed thinking field: {first}"
    );
}

#[test]
fn thinking_only_turn_replays_as_a_minimal_text_block_not_empty_content() {
    // Turn 1: thinking-only (no replayable blocks) → the guard's corrective
    // round must NOT push `"content": []` (Anthropic 400s on it), or the
    // rescue request would fail exactly when it is needed.
    let (base, req_rx) =
        serve_sse_script(vec![anthropic_thinking_only_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, _deltas) =
        run_loop_collect(base_cfg(&base, executor, "claude-test", false), true);
    assert_eq!(outcome, Ok(true));

    let _first = req_rx.recv().expect("initial request");
    let second = req_rx.recv().expect("corrective round request");
    assert!(
        !second.contains(r#""content":[]"#),
        "an empty assistant content array must never ride the wire: {second}"
    );
    assert!(
        second.contains("(no visible output this turn)"),
        "the thinking-only turn is replayed as a minimal text block: {second}"
    );
    // The honest report names the truncation-shaped stop reason.
    assert!(second.contains("the canvas is EMPTY"));
}
