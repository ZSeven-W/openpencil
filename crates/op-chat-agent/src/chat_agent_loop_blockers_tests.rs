//! Unresolved-blocker completion-gate tests — mirrors the "承诺-交付"
//! fill-round tests in `chat_agent_loop_tests.rs` (search for that banner),
//! but for structural blockers (`check_blockers`) instead of unfilled
//! screens (`check_unfilled_screens`). Three tiers: no blocker → unaffected;
//! blocker present with round budget left → one corrective nudge, then
//! completes once fixed; blocker persists past `BLOCKER_NUDGE_MAX_ROUNDS` →
//! never silently succeeds, the honest report still lands.

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
    max_turns: usize,
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
        max_turns,
        finalize_on_exit: true,
        disable_thinking: false,
        dial_policy: crate::provider_dial::EndpointDialPolicy::Trusted,
    }
}

#[test]
fn no_blockers_completes_without_nudge_or_report() {
    // Nothing scripted for `check_blockers` — the default (empty) report —
    // must behave EXACTLY like today: straight to finalize + Done, no
    // corrective round, no "unresolved blocker" line anywhere.
    // The write turn keeps the zero-write guard out of this test's scope —
    // its subject is the blocker tier alone.
    let (base, req_rx) = serve_sse_script(vec![anthropic_tool_use_turn(), anthropic_text_turn()]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#);
    let cfg = base_cfg(&base, executor.clone(), 5);
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    // Exactly two requests went out (initial + post-write) — no corrective
    // round was injected.
    let _first = req_rx.recv().expect("first request captured");
    let _second = req_rx.recv().expect("post-write request captured");
    assert!(
        req_rx.recv().is_err(),
        "no follow-up request when there is nothing to nudge about"
    );
    assert_eq!(executor.finalizes(), 1);
    assert!(
        !deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("unresolved blocker"))),
        "a clean run must never print a blocker report: {deltas:?}"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));
}

#[test]
fn blocker_present_gets_one_corrective_round_then_completes_once_fixed() {
    // Turn 1: model stops with a structural blocker still on the canvas —
    // the loop injects one corrective round instead of finalizing
    // immediately. Turn 2: model stops again; this time the scan reports
    // nothing left — straight to finalize, which also finds nothing.
    let (base, req_rx) = serve_sse_script(vec![
        anthropic_tool_use_turn(),
        anthropic_text_turn(),
        anthropic_text_turn(),
    ]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_blocker_check(&[("nav", "tab-profile is not bound to events.onTap yet")])
        .with_blocker_check(&[]);
    let cfg = base_cfg(&base, executor.clone(), 5);
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    // 3 checks total: the nudge round's probe (finds it), the nudge round's
    // probe on the NEXT pass (confirms it's fixed), and the final tail
    // check inside `finalize_and_report` (also confirms nothing left).
    assert_eq!(executor.blocker_checks(), 3);
    assert_eq!(
        executor.finalizes(),
        1,
        "finalize still runs exactly once, at the real exit"
    );

    // The corrective round's contract line actually rode the follow-up
    // request as a real turn.
    let _first = req_rx.recv().expect("first request captured");
    let _second = req_rx.recv().expect("post-write request captured");
    let third = req_rx
        .recv()
        .expect("corrective-round follow-up request captured");
    assert!(
        third.contains("unresolved blocker")
            && third.contains("[nav] tab-profile is not bound to events.onTap yet"),
        "the corrective round must name the exact blocker, got: {third}"
    );

    // Nothing left unresolved after the corrective round → no tier-3 report.
    assert!(
        !deltas
            .iter()
            .any(|d| matches!(d, ChatDelta::TextDelta(s) if s.contains("unresolved blocker"))),
        "a blocker the corrective round fixed must not also be reported: {deltas:?}"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));
}

#[test]
fn blocker_persisting_past_round_budget_still_reports_needs_attention() {
    // The model voluntarily stops 3 times in a row and the SAME blocker is
    // still present every time. `BLOCKER_NUDGE_MAX_ROUNDS` (2) must stop the
    // corrective nudging after 2 rounds — the 3rd stop must NOT spend a
    // 3rd nudge round, but the run must still end with an honest report,
    // never a silent success.
    let (base, req_rx) = serve_sse_script(vec![
        anthropic_tool_use_turn(),
        anthropic_text_turn(),
        anthropic_text_turn(),
        anthropic_text_turn(),
    ]);
    let executor = ScriptedExecutor::ok(r#"{"success":true,"data":{}}"#)
        .with_blocker_check(&[("structure", "duplicate Explore root r1/r2")])
        .with_blocker_check(&[("structure", "duplicate Explore root r1/r2")])
        .with_blocker_check(&[("structure", "duplicate Explore root r1/r2")]);
    // Turn budget is generous — neither nudge round counts against `turn`,
    // so ONLY the round budget, not the ordinary turn cap, ends this.
    let cfg = base_cfg(&base, executor.clone(), 5);
    let (outcome, deltas) = run_loop_collect(cfg, true);
    assert_eq!(outcome, Ok(true));

    // 3 checks: round 1 (nudge), round 2 (nudge), round 3 (budget spent —
    // the gate short-circuits before probing again for the NUDGE decision,
    // but the final tail check inside `finalize_and_report` still runs
    // once more) — so exactly 3 scripted values get consumed.
    assert_eq!(executor.blocker_checks(), 3);
    assert_eq!(executor.finalizes(), 1);

    // Only 2 corrective requests went out (round 1 and round 2); the 3rd
    // model stop goes straight to finalize+Done — no 3rd nudge request.
    let _first = req_rx.recv().expect("initial request");
    let _second = req_rx.recv().expect("post-write request");
    let third = req_rx.recv().expect("round 1 corrective request");
    assert!(third.contains("unresolved blocker"));
    let fourth = req_rx.recv().expect("round 2 corrective request");
    assert!(fourth.contains("unresolved blocker"));
    assert!(
        req_rx.recv().is_err(),
        "the round budget must stop a 3rd corrective request from going out"
    );

    let report = deltas.iter().find_map(|d| match d {
        ChatDelta::TextDelta(s) if s.contains("unresolved blocker") => Some(s.clone()),
        _ => None,
    });
    assert!(
        report.is_some_and(|s| s.contains("duplicate Explore root r1/r2")),
        "a blocker still present after exhausting its round budget must be reported, not silently dropped: {deltas:?}"
    );
    assert!(matches!(
        deltas.last(),
        Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn
        })
    ));
}
