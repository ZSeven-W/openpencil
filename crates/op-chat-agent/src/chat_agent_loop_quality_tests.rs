//! Quality-credential tests for the agent loop's shared finalize tail —
//! the positive counterpart to `chat_agent_loop_blockers_tests.rs`. Where
//! those guard "a run with problems never looks clean", these guard the
//! other three shapes: a clean run still earns a visible credential, a run
//! with leftovers reports them in the same line, and a turn whose executor
//! checked nothing gets no credential at all.

use super::tests::{
    anthropic_tool_use_turn, run_loop_collect, serve_sse_script, update_node_tool_def,
    ScriptedExecutor,
};
use super::*;

fn anthropic_text_turn() -> String {
    [
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Done."}}"#,
        "",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#,
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
    finalize_on_exit: bool,
) -> AgentLoopConfig {
    AgentLoopConfig {
        url: format!("{base}/v1/messages"),
        api_key: "sk-test".into(),
        model: "claude-test".into(),
        system_prompt: String::new(),
        history: Vec::new(),
        user_prompt: "build the app".into(),
        max_output_tokens: 512,
        tools: vec![update_node_tool_def()],
        executor,
        max_turns: 5,
        finalize_on_exit,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    }
}

fn credential_line(deltas: &[ChatDelta]) -> Option<String> {
    deltas.iter().find_map(|d| match d {
        ChatDelta::TextDelta(s) if s.contains("• Checked ") => Some(s.clone()),
        _ => None,
    })
}

#[test]
fn clean_run_still_earns_a_visible_credential() {
    let (base, _req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_quality_finalize(&["layout", "overflow", "hierarchy"], &[]);
    let (outcome, deltas) = run_loop_collect(base_cfg(&base, executor, true), true);
    assert_eq!(outcome, Ok(true));

    let line = credential_line(&deltas).expect("a checked run must show its credential");
    assert!(
        line.contains("Checked layout, overflow, hierarchy"),
        "only the checks that ran may be listed: {line}"
    );
    assert!(
        line.contains("nothing needed fixing") && line.contains("no issues left"),
        "a clean run reports zero repairs AND zero leftovers: {line}"
    );
}

#[test]
fn repairs_are_reported_with_their_real_count() {
    let (base, _req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#).with_quality_finalize(
        &["layout", "overflow", "structure"],
        &[("layout", 2), ("structure", 4)],
    );
    let (outcome, deltas) = run_loop_collect(base_cfg(&base, executor, true), true);
    assert_eq!(outcome, Ok(true));

    let line = credential_line(&deltas).expect("credential owed");
    assert!(
        line.contains("6 auto-repair(s) applied"),
        "the headline number is the sum of the scripted counts: {line}"
    );
    assert!(
        line.contains("▸ repairs: layout 2, structure 4"),
        "the per-check breakdown must match what was counted: {line}"
    );
}

#[test]
fn leftover_blockers_are_counted_into_the_credential() {
    let (base, _req_rx) = serve_sse_script(
        std::iter::once(anthropic_tool_use_turn())
            .chain(std::iter::repeat_with(anthropic_text_turn).take(4))
            .collect(),
    );
    // Blockers persist past the nudge budget, so the loop reaches its honest
    // report tier with two issues still open. The credential must agree with
    // the report lines above it instead of softening them.
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_blocker_check(&[("structure", "duplicate status bar")])
        .with_blocker_check(&[("structure", "duplicate status bar")])
        .with_blocker_check(&[
            ("structure", "duplicate status bar"),
            ("nav", "unbound tab: Home"),
        ])
        .with_quality_finalize(&["structure"], &[("structure", 1)]);
    let (outcome, deltas) = run_loop_collect(base_cfg(&base, executor, true), true);
    assert_eq!(outcome, Ok(true));

    let line = credential_line(&deltas).expect("credential owed");
    assert!(
        line.contains("2 issue(s) still open"),
        "unresolved blockers must survive into the credential: {line}"
    );
    assert!(
        !line.contains("no issues left"),
        "must never claim a clean finish while blockers stand: {line}"
    );
}

#[test]
fn credential_lands_after_the_problem_lines_it_summarizes() {
    let (base, _req_rx) = serve_sse_script(
        std::iter::once(anthropic_tool_use_turn())
            .chain(std::iter::repeat_with(anthropic_text_turn).take(4))
            .collect(),
    );
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_blocker_check(&[("nav", "unbound tab: Home")])
        .with_blocker_check(&[("nav", "unbound tab: Home")])
        .with_blocker_check(&[("nav", "unbound tab: Home")])
        .with_quality_finalize(&["structure"], &[]);
    let (_outcome, deltas) = run_loop_collect(base_cfg(&base, executor, true), true);

    let texts: Vec<&String> = deltas
        .iter()
        .filter_map(|d| match d {
            ChatDelta::TextDelta(s) => Some(s),
            _ => None,
        })
        .collect();
    let blocker_at = texts
        .iter()
        .position(|s| s.contains("unresolved blocker"))
        .expect("blocker report expected");
    let credential_at = texts
        .iter()
        .position(|s| s.contains("• Checked "))
        .expect("credential expected");
    assert!(
        blocker_at < credential_at,
        "the credential is the closing receipt over the problem lines, not a preamble"
    );
}

#[test]
fn executor_that_checked_nothing_gets_no_credential() {
    // Default scripted executor: `finalize` returns an empty quality summary,
    // exactly like a host that owns no live document.
    let (base, _req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let (outcome, deltas) = run_loop_collect(base_cfg(&base, executor, true), true);
    assert_eq!(outcome, Ok(true));

    assert!(
        credential_line(&deltas).is_none(),
        "nothing was checked, so nothing may be vouched for: {deltas:?}"
    );
}

#[test]
fn plain_chat_turn_never_emits_a_credential() {
    // `finalize_on_exit: false` — the loop serving ordinary chat must not
    // touch the document, and therefore has nothing to certify.
    let (base, _req_rx) = serve_sse_script(vec![anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_quality_finalize(&["layout"], &[("layout", 3)]);
    let (outcome, deltas) = run_loop_collect(base_cfg(&base, executor.clone(), false), true);
    assert_eq!(outcome, Ok(true));

    assert_eq!(executor.finalizes(), 0, "a plain chat turn never finalizes");
    assert!(
        credential_line(&deltas).is_none(),
        "a non-design turn must stay silent: {deltas:?}"
    );
}
