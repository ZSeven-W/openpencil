//! `run.rs` inline tests — sequential + single-mode planning + 3-attempt ladder.
//!
//! Wired as `#[path = "run_tests.rs"] mod tests;` inside `run.rs`;
//! stays a child module of `run`, so `use super::*` resolves to `run`.

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

fn req() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

// Standard tier model id (used by the 3-attempt subtask ladder tests).
fn req_standard() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        // "gpt-4o" matches Standard tier in model_profile table
        model: Some("gpt-4o".into()),
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

const PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

const MOBILE_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Mobile Page", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFF8F0" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 390, "height": 300 } }
  ]
}"##;

fn node_json(prefix: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[{{"type":"text","id":"{prefix}-title","content":"{prefix}","fontSize":18}}]}}]"#
    )
}

fn existing_root_json(
    id: &str,
    name: &str,
    x: f64,
    y: f64,
    width: f64,
) -> jian_ops_schema::node::PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": id,
        "name": name,
        "x": x,
        "y": y,
        "width": width,
        "height": 844.0,
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": format!("{id}-content"), "name": "Content", "children": []}
        ]
    }))
    .expect("valid existing root")
}

// ── existing tests (must stay green) ─────────────────────────────────────

#[test]
fn run_happy_path_applies_scaffold_and_subtasks() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
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
    .expect("run ok");

    // root_frame_id 是 InsertSubtree 重映射后的真实 id —— 不是
    // plan 里的 "root" 字面值,只断言它非空。
    assert!(!summary.root_frame_id.is_empty());
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    // undo batch 配对。
    assert_eq!(sink.batch_depth, 0);
    // 至少有 scaffold + 两个 subtask 的 InsertSubtree。
    let inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert!(inserts >= 3, "expected >=3 InsertSubtree, got {inserts}");
    assert!(matches!(events.first(), Some(Progress::Planning)));
    // CleanupDone must be present (validation runs after it).
    assert!(
        events.iter().any(|e| matches!(e, Progress::CleanupDone)),
        "expected CleanupDone in events"
    );
}

#[test]
fn run_mobile_scaffold_reveals_status_bar() {
    let _guard = crate::agent_indicator_test_support::lock();
    let epoch = op_editor_core::agent_indicators::begin();
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MOBILE_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let mut request = req();
    request.prompt = "a mobile food app".into();
    request.validation_enabled = false;

    let summary = futures::executor::block_on(Orchestrator::new().with_indicator_epoch(epoch).run(
        request,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("mobile run ok");

    assert!(!summary.root_frame_id.is_empty());
    let root = sink.state.active_children().first().expect("root inserted");
    let status_id = root
        .children()
        .expect("mobile root should have children")
        .iter()
        .find(|node| {
            serde_json::to_value(node)
                .ok()
                .is_some_and(|v| v["role"] == "status-bar")
        })
        .map(|node| node.id_str().to_string())
        .expect("mobile scaffold should insert a status bar");
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(
        snapshot.frames.contains_key(root.id_str()),
        "sequential scaffold root should get an agent frame badge"
    );
    assert!(
        snapshot.reveals.contains_key(&status_id),
        "status bar should get a reveal animation, got {:?}",
        snapshot.reveals.keys().collect::<Vec<_>>()
    );
    op_editor_core::agent_indicators::end_if_epoch(epoch);
}

#[test]
fn run_follow_on_screen_scaffold_lands_beside_existing_root() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MOBILE_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("discover")),
    ]);
    let mut sink = VecDocSink::new();
    sink.state.active_children_mut().clear();
    sink.state
        .active_children_mut()
        .push(existing_root_json("home", "Home", 80.0, 40.0, 390.0));
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let mut request = req();
    request.prompt = "继续画出发现页".into();
    request.validation_enabled = false;

    let summary = futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("follow-on run ok");

    let existing_right = 80.0 + 390.0;
    let new_root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == summary.root_frame_id)
        .expect("new root inserted");
    assert!(
        new_root.base().x.unwrap_or(0.0) >= existing_right + 80.0,
        "new root should be laid out beside the existing screen"
    );
    assert_eq!(new_root.base().y, Some(40.0));
}

#[test]
fn run_zero_node_subtask_preserves_failure_context() {
    // 规划 OK,但第一个 subtask 吐垃圾(3 次全失败)→ 零节点 → AllFailed
    // with the parser failure context, not generic NoContent.
    // C3 引入 3-attempt 梯子:需要 3 条垃圾响应才能穷尽重试。
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text("the model refused".into()),
        ScriptResponse::Text("still refused".into()),
        ScriptResponse::Text("refused again".into()),
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
    // undo batch 仍配对。
    assert_eq!(sink.batch_depth, 0);
}

#[test]
fn run_planning_failure_uses_fallback_plan() {
    // 规划吐垃圾 → fallback plan;subtask 正常 → 成功。
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text("no json here".into()),
        ScriptResponse::Text(node_json("section-1")),
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
    .expect("fallback run ok");
    assert!(summary.total_nodes >= 1);
}

// ── single-mode planning tests ───────────────────────────────────────────

/// Planning parse failure → heuristic fallback plan used, run succeeds.
/// (Single-mode planning: one bad response falls straight through to the
/// fallback plan — no mode rotation. The fallback plan has one subtask.)
#[test]
fn planning_parse_failure_uses_fallback_plan() {
    let llm = ScriptedLlm::new(vec![
        // planning call → bad JSON
        ScriptResponse::Text("not valid json at all".into()),
        // fallback plan's single subtask
        ScriptResponse::Text(node_json("section-1")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("fallback after parse failure ok");
    assert!(summary.total_nodes >= 1);
}

/// Planning stream error → heuristic fallback plan used, run succeeds.
#[test]
fn planning_stream_error_uses_fallback_plan() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![
        // planning call → stream error (non-abort)
        ScriptResponse::Fail(LlmError {
            message: "HTTP 500 upstream".into(),
            aborted: false,
        }),
        // fallback plan's single subtask
        ScriptResponse::Text(node_json("section-1")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_standard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("fallback on stream error ok");
    assert!(summary.total_nodes >= 1);
}

/// Abort during planning stream → `OrchestratorError::Aborted`.
#[test]
fn planning_abort_during_stream_returns_aborted() {
    use crate::types::LlmError;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
        message: "user aborted".into(),
        aborted: true,
    })]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let abort = AbortFlag::new();
    let result = futures::executor::block_on(Orchestrator::new().run(
        req(),
        &mut sink,
        &llm,
        &mut on_progress,
        &abort,
        &stub_providers(),
    ));
    assert!(matches!(result, Err(OrchestratorError::Aborted)));
    // undo batch 在 abort 路径前返回,文档不应已进入批
    assert_eq!(sink.batch_depth, 0);
}

// ── Task C3: sub-agent 3-attempt tier-gated retry ladder ──────────────────

/// Subtask returns zero nodes on attempt 1 but succeeds on attempt 2 →
/// the subtask's nodes land (ladder retries once).
/// Uses Full tier (attempt 2: reduced_complexity=false, minimal_skills=false).
#[test]
fn subtask_retries_on_attempt1_zero_succeeds_on_attempt2() {
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: garbage (0 nodes, retryable)
        ScriptResponse::Text("the model gave garbage".into()),
        // subtask hero — attempt 2: success
        ScriptResponse::Text(node_json("hero")),
        // subtask feat — attempt 1: success
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};
    let summary = futures::executor::block_on(Orchestrator::new().run(
        req(), // Full tier → reduced_complexity=false on attempt 2
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("retry succeeded");
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    assert_eq!(sink.batch_depth, 0);
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
        ScriptResponse::Text(PLAN_JSON.into()),
        // subtask hero — attempt 1: HTTP 401 (non-retryable)
        ScriptResponse::Fail(LlmError {
            message: "HTTP 401 Unauthorized".into(),
            aborted: false,
        }),
        // This response should NOT be consumed — the first error is
        // non-retryable.
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
      "rootFrame": { "id": "root", "name": "Dashboard", "width": 1200, "height": 900,
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
            r#"[{{"type":"frame","id":"{id}","name":"{name}","x":0,"y":0,"width":1200,"height":300,"layout":"{layout}","children":[{{"type":"text","id":"{id}-t","content":"{name}","fontSize":18}}]}}]"#
        )
    }
    // A flat sidebar-nav column with a footer-like last child — this trips the
    // whole-root `sink_structured_sidebar_footers` cleanup, which swaps the root
    // via `ReplaceSubtree` (allocating the fresh root id that the guarded check
    // must tolerate).
    let sidebar = r#"[{"type":"frame","id":"nav","name":"Sidebar Navigation","x":0,"y":0,"width":260,"height":600,"layout":"vertical","children":[{"type":"frame","id":"logo","name":"Logo","width":"fill_container","height":40,"children":[{"type":"text","id":"logo-t","content":"Brand","fontSize":18}]},{"type":"frame","id":"navg","name":"Nav Group","width":"fill_container","height":200,"children":[{"type":"text","id":"navg-t","content":"Dashboard","fontSize":14}]},{"type":"frame","id":"owner","name":"Owner Profile","width":"fill_container","height":48,"children":[{"type":"text","id":"owner-t","content":"Marcus — Owner","fontSize":14}]}]}]"#;
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

const DASHBOARD_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Barbershop Dashboard", "width": 1200, "height": 0,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#0A0A0A" }] },
  "subtasks": [
    { "id": "sidebar", "label": "Sidebar Navigation", "region": { "width": 260, "height": 900 } },
    { "id": "kpi", "label": "KPI Stat Cards", "region": { "width": 940, "height": 200 } },
    { "id": "clients", "label": "Client Table", "region": { "width": 940, "height": 500 } }
  ]
}"##;

/// GOLDEN end-to-end (no LLM): dashboard plan → two-column scaffold →
/// scripted subtasks → finalize. The scaffold's Sidebar shell is authored
/// `height: fill_container` and MUST still be fill_container when the run
/// finishes — the user-visible symptom of losing it is the sidebar footer
/// floating mid-page (reported three times).
#[test]
fn run_dashboard_shell_keeps_sidebar_fill_height_end_to_end() {
    let sidebar_nodes = r#"[{"type":"frame","id":"sb-1","name":"Sidebar Navigation","layout":"vertical","width":"fill_container","height":"fill_container","justifyContent":"space_between","children":[{"type":"frame","id":"sb-top","name":"Top","layout":"vertical","children":[{"type":"text","id":"sb-logo","content":"MAISON","fontSize":20}]},{"type":"frame","id":"sb-bottom","name":"Owner Card","layout":"vertical","children":[{"type":"text","id":"sb-owner","content":"Marcus Reed","fontSize":14}]}]}]"#;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        ScriptResponse::Text(sidebar_nodes.into()),
        ScriptResponse::Text(node_json("kpi")),
        ScriptResponse::Text(node_json("clients")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let mut request = req();
    request.prompt = "barbershop client-management dashboard with a left sidebar".into();
    request.validation_enabled = false;

    futures::executor::block_on(Orchestrator::new().run(
        request,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("dashboard run ok");

    let root = sink.state.active_children().first().expect("root");
    let v = serde_json::to_value(root).unwrap();
    let kids = v["children"].as_array().expect("root children");
    let sidebar = kids
        .iter()
        .find(|k| {
            k["name"]
                .as_str()
                .map(|n| n.to_lowercase().contains("sidebar"))
                .unwrap_or(false)
        })
        .unwrap_or_else(|| {
            panic!(
                "sidebar shell present, got children: {:?}",
                kids.iter().map(|k| k["name"].clone()).collect::<Vec<_>>()
            )
        });
    assert_eq!(
        sidebar["height"],
        serde_json::json!("fill_container"),
        "sidebar shell keeps fill height; sidebar = {}",
        serde_json::to_string_pretty(&sidebar)
            .unwrap()
            .chars()
            .take(600)
            .collect::<String>()
    );
}
