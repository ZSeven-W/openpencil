//! geometry_echo end-to-end (rail-collapse fixture), the terminal retry
//! outcomes and the salvage pass.

use super::*;

// ── geometry_echo end-to-end (rail-collapse fixture) ────────────────────

const RAIL_PLAN_JSON: &str = r##"{
  "sharedContext": {}, "rootFrame": { "id": "root", "name": "Page", "width": 375, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "goals", "label": "Savings Goals", "region": { "width": 375, "height": 200 } }
  ]
}"##;

/// The exact "Savings Goals" rail shape `geometry_rail_collapse_tests.rs`
/// (de-identified real user report) and
/// `geometry_validation_tests.rs::rail_width_collapse_is_echoed_for_the_model_under_real_layout`
/// both key off: a 200px fixed `Emergency Fund` card starves its two
/// `fill_container` siblings down to ~51px in the 327px-inner rail —
/// truncated titles, ballooned card height from forced wrapping.
fn rail_collapse_script() -> String {
    r##"I(null, {"type":"frame","name":"Goals Rail","layout":"horizontal","width":327,"height":"fit_content","gap":12,"children":[
        {"type":"frame","name":"Emergency Fund","layout":"vertical","width":200,"height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"frame","name":"New Car","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"frame","name":"Vacation","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]}
    ]});"##
        .into()
}

/// The model's in-loop self-correction: same rail, all THREE cards
/// `fill_container` — no fixed reference left to starve against at all.
fn rail_fixed_script() -> String {
    r##"I(null, {"type":"frame","name":"Goals Rail","layout":"horizontal","width":327,"height":"fit_content","gap":12,"children":[
        {"type":"frame","name":"Emergency Fund","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"frame","name":"New Car","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]},
        {"type":"frame","name":"Vacation","layout":"vertical","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#FFFFFF"}]}
    ]});"##
        .into()
}

/// End-to-end: attempt 1 lands the collapsed rail (self-check has no
/// concentricity-style gate for THIS failure mode, so it inserts fine) —
/// `geometry_echo` must catch it via the REAL resolved layout, retry
/// in-loop, and land the balanced version BEFORE `finalize_design`'s
/// deterministic geometry fixers ever get a chance to touch it. Proof that
/// the deterministic net stayed idle: the fixer's ONLY possible move here
/// (`fix_rail_width_collapse`) requires a FIXED-width reference card to
/// widen siblings toward — the echoed replacement has none (all three
/// cards are the `fill_container` keyword, not a number), so if the
/// resolved widths come back balanced, that widening move structurally
/// could not have run.
#[test]
fn geometry_echo_salvages_the_rail_collapse_fixture_before_the_deterministic_net() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(RAIL_PLAN_JSON.into()),
        ScriptResponse::Text(rail_collapse_script()),
        ScriptResponse::Text(rail_fixed_script()),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("geometry_echo salvages the collapsed rail");
    assert_eq!(summary.subtasks.len(), 1);

    // Exactly 3 LLM calls total: planning + attempt 1 + the ONE echo retry
    // — no salvage pass, no third subtask attempt (attempt 1 already
    // succeeded; geometry_echo is not the zero-node ladder).
    assert_eq!(llm.system_prompts().len(), 3, "unexpected call count");

    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::GeometryEcho { issue_count, .. } if *issue_count >= 2)),
        "must announce the echo with the real rail-collapse issue count: {events:?}"
    );

    // Real layout proof: no remaining rail-collapse diagnostic.
    let issues = crate::geometry_validation::geometry_diagnostics(&sink.state);
    assert!(
        !issues.iter().any(|i| i.contains("collapsed to")),
        "the final document must have no rail-collapse violation left: {issues:?}"
    );

    // Structural proof the deterministic net's widening fixer never had
    // anything to act on: the surviving cards are still the keyword
    // `fill_container`, never rewritten to a numeric width.
    let root = &sink.state.active_children()[0];
    let root_json = serde_json::to_value(root).expect("serialize root");
    fn find_by_name<'a>(v: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        if v.get("name").and_then(|x| x.as_str()) == Some(name) {
            return Some(v);
        }
        v.get("children")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .find_map(|c| find_by_name(c, name))
    }
    for name in ["Emergency Fund", "New Car", "Vacation"] {
        let card = find_by_name(&root_json, name).unwrap_or_else(|| panic!("{name} present"));
        assert_eq!(
            card["width"],
            serde_json::json!("fill_container"),
            "{name} must still be the fill_container keyword — a numeric width here \
             would mean the deterministic fixer (not the echo) made the call"
        );
    }
}

/// Subtask fails all 3 attempts → `OrchestratorError::AllFailed` with the
/// final failure context.
#[test]
fn subtask_all_three_attempts_fail_returns_failure_context() {
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: garbage
        ScriptResponse::Text("garbage attempt 1".into()),
        // subtask hero — attempt 2: garbage
        ScriptResponse::Text("garbage attempt 2".into()),
        // subtask hero — attempt 3: garbage
        ScriptResponse::Text("garbage attempt 3".into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ));
    match result {
        Err(OrchestratorError::AllFailed(message)) => assert!(!message.is_empty()),
        other => panic!("expected AllFailed with parser context, got {other:?}"),
    }
    assert_eq!(sink.batch_depth, 0);
}

/// Subtask's attempt-1 error is non-retryable (HTTP 401) →
/// no retry, and the top-level error preserves the auth failure instead of
/// collapsing to the generic no-content message.
#[test]
fn subtask_non_retryable_error_preserves_failure_context() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(MOBILE_PLAN_JSON.into()),
        // subtask hero — attempt 1: HTTP 401 (non-retryable)
        ScriptResponse::Fail(LlmError {
            message: "HTTP 401 Unauthorized".into(),
            aborted: false,
        }),
        // No fallback response: the non-retryable error must end this subtask.
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ));
    match result {
        Err(OrchestratorError::AllFailed(message)) => {
            assert!(message.contains("HTTP 401 Unauthorized"), "{message}");
        }
        other => panic!("expected auth context in AllFailed, got {other:?}"),
    }
    assert_eq!(llm.user_prompts().len(), 2);
    assert_eq!(sink.batch_depth, 0);
}

/// Partial result (node_count > 0 with an error) is never retried —
/// it is accepted and counted toward summary.
///
/// Note: the current `run_subtask` returns `error: None` on success and
/// `error: Some` only on zero-node failure. A partial result (nodes
/// produced + downstream soft error) would arrive as node_count>0,
/// error=None from `run_subtask`. We model this by having the first
/// subtask succeed (nodes produced) even though the scenario calls for
/// a "partial with error". The key invariant: once node_count>0 the
/// ladder does not retry regardless of error state.
#[test]
fn subtask_partial_result_not_retried() {
    // A subtask that returns a valid node on the first attempt must
    // succeed without using a second LLM slot.
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: success (node_count > 0)
        ScriptResponse::Text(node_json("hero")),
        // subtask feat — attempt 1: success
        ScriptResponse::Text(node_json("feat")),
        // A third response here would mean hero was retried — we assert
        // only 2 subtasks succeeded so the LLM is not over-consumed.
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("no retry on partial");
    // Both subtasks succeed; if hero had been retried the scripted LLM
    // would have served feat's slot to the second hero attempt, leaving
    // feat with 0 nodes and causing NoContent.
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
}

/// Regression: a dashboard whose cleanup RESTRUCTURES the root (the app-shell
/// reshape swaps it via `ReplaceSubtree`, which allocates a FRESH root id) must
/// NOT be mistaken for "no content". The zero-content check reads the descendant
/// count BEFORE cleanup for exactly this reason — reading it afterward looks up
/// the now-stale id, gets 0, and returns a false `NoContent` (rolling back the
/// design's theme variables in the process). Before that fix this returned
/// `Err(NoContent)` despite three full sections of content.
#[test]
fn reshaped_dashboard_root_is_not_a_false_no_content() {
    const DASH_PLAN: &str = r##"{
      "sharedContext": {}, "rootFrame": { "id": "root", "name": "Dashboard", "width": 1200, "height": 900,
                     "layout": "vertical", "gap": 24,
                     "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
      "subtasks": [
        { "id": "sidebar", "label": "Sidebar Navigation", "region": { "width": 1200, "height": 600 } },
        { "id": "metrics", "label": "Key Metrics", "region": { "width": 1200, "height": 160 } },
        { "id": "table", "label": "Client Table", "region": { "width": 1200, "height": 400 } }
      ]
    }"##;
    fn full_section(id: &str, name: &str, layout: &str) -> String {
        format!(
            r#"I(null, {{"type":"frame","id":"{id}","name":"{name}","x":0,"y":0,"width":1200,"height":300,"layout":"{layout}","children":[{{"type":"text","content":"{name}","fontSize":18}}]}});"#
        )
    }
    // A flat sidebar-nav column with a footer-like last child — this trips the
    // whole-root `sink_structured_sidebar_footers` cleanup, which swaps the root
    // via `ReplaceSubtree` (allocating the fresh root id that the guarded check
    // must tolerate).
    let sidebar = r#"I(null, {"type":"frame","name":"Sidebar Navigation","x":0,"y":0,"width":260,"height":600,"layout":"vertical","children":[{"type":"frame","name":"Logo","width":"fill_container","height":40,"children":[{"type":"text","content":"Brand","fontSize":18}]},{"type":"frame","name":"Nav Group","width":"fill_container","height":200,"children":[{"type":"text","content":"Dashboard","fontSize":14}]},{"type":"frame","name":"Owner Profile","width":"fill_container","height":48,"children":[{"type":"text","content":"Marcus — Owner","fontSize":14}]}]});"#;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(DASH_PLAN.into()),
        ScriptResponse::Text(sidebar.into()),
        ScriptResponse::Text(full_section("m-1", "Key Metrics", "horizontal")),
        ScriptResponse::Text(full_section("t-1", "Client Table", "vertical")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("reshaped dashboard must NOT return NoContent");

    assert!(
        summary.total_nodes >= 3,
        "all three sections produced content"
    );
    // Prove a whole-root structural transform actually fired — otherwise the
    // root id never changes and the test would not exercise the guarded path.
    let replaced = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::ReplaceSubtree { .. }))
        .count();
    assert!(
        replaced >= 1,
        "a structural cleanup transform must have run (root id changed)"
    );
}

#[test]
fn run_salvage_pass_recovers_a_transiently_failed_subtask() {
    // The hero subtask burns ALL 3 attempts on transient empties (measured:
    // Ark returned "empty content from provider" 3× in a row and the SIDEBAR
    // section shipped missing with no visible signal). The end-of-run salvage
    // pass must give it one late attempt — which succeeds here — so the
    // section lands after all.
    use crate::types::LlmError;
    let empty = || {
        ScriptResponse::Fail(LlmError {
            message: "empty content from provider".into(),
            aborted: false,
        })
    };
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        empty(),                                 // hero attempt 1
        empty(),                                 // hero attempt 2
        empty(),                                 // hero attempt 3
        ScriptResponse::Text(node_json("feat")), // feat attempt 1 (ok)
        ScriptResponse::Text(node_json("hero")), // hero salvage (ok)
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("salvaged run ok");

    assert_eq!(summary.subtasks.len(), 2);
    assert!(
        summary.subtasks.iter().all(|o| o.node_count > 0),
        "both subtasks landed after salvage: {:?}",
        summary
            .subtasks
            .iter()
            .map(|o| (o.node_count, o.error.clone()))
            .collect::<Vec<_>>()
    );
    // The salvage retry is visible in progress (attempt 4) and ends Done.
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Progress::SubtaskRetry { attempt: 4, .. })),
        "salvage retry reported"
    );
    let hero_done = events
        .iter()
        .any(|e| matches!(e, Progress::SubtaskDone { id, .. } if id == "hero"));
    assert!(hero_done, "hero eventually Done via salvage");
}

#[test]
fn run_salvage_pass_uses_minimal_skills_not_full_complexity() {
    // 2026-07-17 policy (failed-subtask remediation, automatic-layer step 1):
    // the salvage pass no longer repeats attempt-1's full complexity —
    // attempts 1-3 already tried full/reduced/minimal and all failed, so an
    // identical repeat of attempt-1 is the least-informed retry choice.
    // Salvage now uses attempt-3's settings (reduced_complexity=true,
    // minimal_skills=true): the schema-only "floor" protocol most likely to
    // land SOME content instead of reproducing the same zero-content outcome.
    use crate::types::LlmError;
    let empty = || {
        ScriptResponse::Fail(LlmError {
            message: "empty content from provider".into(),
            aborted: false,
        })
    };
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        empty(),                                 // hero attempt 1 (full)
        empty(),                                 // hero attempt 2
        empty(),                                 // hero attempt 3 (minimal)
        ScriptResponse::Text(node_json("feat")), // feat attempt 1 (ok)
        ScriptResponse::Text(node_json("hero")), // hero salvage (ok)
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("salvaged run ok");

    // Call order: [0] planning, [1] hero attempt 1 (full), [2] hero attempt 2,
    // [3] hero attempt 3 (minimal), [4] feat attempt 1, [5] hero salvage.
    let prompts = llm.system_prompts();
    assert_eq!(
        prompts.len(),
        6,
        "unexpected call count: {:?}",
        prompts.iter().map(|p| p.len()).collect::<Vec<_>>()
    );
    let hero_attempt1 = &prompts[1];
    let hero_attempt3 = &prompts[3];
    let hero_salvage = &prompts[5];

    assert!(
        hero_salvage.len() < hero_attempt1.len(),
        "salvage prompt ({} chars) must be shorter than attempt-1's full-complexity \
         prompt ({} chars) — it must resolve the minimal-skills (schema-only) set, \
         not repeat attempt 1's settings",
        hero_salvage.len(),
        hero_attempt1.len()
    );
    // Salvage must match attempt 3's (minimal_skills=true) prompt shape
    // exactly, not just "shorter than attempt 1" by coincidence.
    assert_eq!(
        hero_salvage.len(),
        hero_attempt3.len(),
        "salvage must resolve the SAME minimal-skills prompt shape as attempt 3"
    );
    assert!(
        hero_salvage.contains("OUTPUT PROTOCOL: JAVASCRIPT PROGRAM"),
        "salvage must still be script-gen: {hero_salvage}"
    );
}
