//! Task C3 (S3b-3) tests — dashboard column wiring in `Orchestrator::run()`.
//!
//! Wired as `#[path = "run_tests_c3.rs"] mod tests_c3;` inside `run.rs`;
//! stays a child module of `run`, so `use super::*` resolves to `run`.
//!
//! Covers:
//! - Dashboard-like plan → dashboard scaffold is used (sidebar + main frame),
//!   subtasks routed to their live slot/sidebar ids, reorder + cleanup runs.
//! - Non-dashboard plan → existing sequential vertical path, unchanged.

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use op_editor_core::EditorCommand;

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// A dashboard-like plan JSON.
///
/// - Width 1440 (> 480, triggers dashboard).
/// - "analytics admin dashboard" in subtask labels → `is_dashboard_like_prompt`.
/// - One sidebar subtask, one metrics subtask → `should_use_dashboard_columns`.
const DASHBOARD_PLAN_JSON: &str = r##"{
  "rootFrame": {
    "id": "dash-root",
    "name": "Analytics Dashboard",
    "width": 1440,
    "height": 900,
    "layout": "vertical",
    "gap": 20,
    "fill": [{ "type": "solid", "color": "#0F172A" }]
  },
  "subtasks": [
    {
      "id": "sidebar-nav",
      "label": "Sidebar Navigation",
      "region": { "width": 260, "height": 760 }
    },
    {
      "id": "metrics-panel",
      "label": "Metrics Chart analytics dashboard",
      "region": { "width": 900, "height": 320 }
    }
  ]
}"##;

fn node_json(prefix: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Content","x":0,"y":0,"width":200,"height":200,"children":[{{"type":"text","id":"{prefix}-title","content":"{prefix}","fontSize":18}}]}}]"#
    )
}

fn req_dashboard() -> DesignRequest {
    DesignRequest {
        prompt: "an analytics admin dashboard".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

fn req_non_dashboard() -> DesignRequest {
    DesignRequest {
        prompt: "a simple landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

/// Standard non-dashboard plan JSON (same as run_tests.rs PLAN_JSON).
const STANDARD_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

// ── Task C3 tests ─────────────────────────────────────────────────────────────

/// Dashboard plan → run succeeds, both subtask nodes land, dashboard scaffold is used.
///
/// Verifies:
/// - Dashboard scaffold is used: the live document has a root node whose
///   children include a "Sidebar" frame and a "Main Content" frame.
/// - Both subtasks' nodes land (total_nodes >= 2).
/// - Undo batch balanced.
/// - Progress sequence: Planning → ScaffoldDone → ... → CleanupDone.
#[test]
fn dashboard_run_succeeds_and_lands_all_subtask_nodes() {
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        // sidebar-nav subtask
        ScriptResponse::Text(node_json("sidebar")),
        // metrics-panel subtask
        ScriptResponse::Text(node_json("metrics")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_dashboard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("dashboard run ok");

    assert!(
        !summary.root_frame_id.is_empty(),
        "root_frame_id must be non-empty"
    );
    assert_eq!(
        summary.subtasks.len(),
        2,
        "both subtasks should be in summary"
    );
    assert!(summary.total_nodes >= 2, "both subtask nodes should land");
    assert_eq!(sink.batch_depth, 0, "undo batch must be balanced");

    // Progress events: Planning → ScaffoldDone → 2×(SubtaskStarted,SubtaskDone)
    // → CleanupDone → ValidationStarted → … → ValidationDone.
    assert!(matches!(events.first(), Some(Progress::Planning)));
    // CleanupDone must be present (validation runs after it).
    assert!(
        events.iter().any(|e| matches!(e, Progress::CleanupDone)),
        "expected CleanupDone in events"
    );

    // Dashboard scaffold emits 1 InsertSubtree for the root (with sidebar+main embedded)
    // + 2 InsertSubtrees for the subtask contents.
    let inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert!(
        inserts >= 3,
        "expected >=3 InsertSubtree commands, got {inserts}"
    );

    // Verify the dashboard scaffold was used: the root frame's children must include
    // a "Sidebar" frame and a "Main Content" frame (only present when using
    // `build_scaffold_dashboard`, not the regular vertical scaffold).
    use op_editor_core::PenNodeExt;
    let root_id = &summary.root_frame_id;
    let root_in_doc = sink
        .state()
        .active_children()
        .iter()
        .find(|n| n.id_str() == root_id);
    let root_children = root_in_doc
        .and_then(|n| n.children())
        .expect("root must have children (sidebar + main)");
    let child_names: Vec<String> = root_children
        .iter()
        .filter_map(|n| n.base().name.clone())
        .collect();
    assert!(
        child_names.iter().any(|n| n == "Sidebar"),
        "dashboard root must have a 'Sidebar' child; got: {child_names:?}"
    );
    assert!(
        child_names.iter().any(|n| n == "Main Content"),
        "dashboard root must have a 'Main Content' child; got: {child_names:?}"
    );
}

/// Non-dashboard plan → sequential vertical path is unchanged.
///
/// The existing `run_happy_path_applies_scaffold_and_subtasks` test is the
/// primary check; this test adds an explicit regression guard by using the
/// same non-dashboard prompt/plan.
#[test]
fn non_dashboard_run_takes_sequential_vertical_path() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(STANDARD_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_non_dashboard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("non-dashboard run ok");

    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    assert_eq!(sink.batch_depth, 0);
}

/// Dashboard run — zero-node subtask failure path is still handled correctly.
/// The 3-attempt retry ladder runs; after exhaustion → NoContent.
#[test]
fn dashboard_zero_node_subtask_returns_no_content() {
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        // sidebar attempt 1
        ScriptResponse::Text("garbage 1".into()),
        // sidebar attempt 2
        ScriptResponse::Text("garbage 2".into()),
        // sidebar attempt 3
        ScriptResponse::Text("garbage 3".into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};

    let result = futures::executor::block_on(Orchestrator::new().run(
        req_dashboard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ));

    assert!(
        matches!(result, Err(OrchestratorError::NoContent)),
        "expected NoContent, got: {result:?}"
    );
    assert_eq!(sink.batch_depth, 0);
}

/// Dashboard run — abort via AbortFlag mid-run returns Aborted.
///
/// We set the abort flag before the first subtask runs, which causes the
/// subtask loop to detect `abort.is_set()` and break immediately.
/// With no subtask content produced and `aborted_mid = true`, the path
/// returns `OrchestratorError::Aborted`.
#[test]
fn dashboard_abort_mid_run_returns_aborted() {
    let abort = AbortFlag::new();
    // Clone the flag so we can set it inside the on_progress callback.
    let abort_to_set = abort.clone();

    // Scripted LLM: planning succeeds, then subtask returns an error
    // (simulating an aborted stream); the abort flag is set via callback.
    let llm = ScriptedLlm::new(vec![
        // planning
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        // first subtask — will be retried until exhaustion OR abort.is_set() stops it
        ScriptResponse::Text("garbage 1".into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |p: Progress| {
        // Set the abort flag right after scaffold is done so the subtask loop
        // detects it and breaks — simulating a user abort during generation.
        if matches!(p, Progress::ScaffoldDone) {
            abort_to_set.set();
        }
    };

    let result = futures::executor::block_on(Orchestrator::new().run(
        req_dashboard(),
        &mut sink,
        &llm,
        &mut on_progress,
        &abort,
        &stub_providers(),
    ));

    assert!(
        matches!(result, Err(OrchestratorError::Aborted)),
        "expected Aborted, got: {result:?}"
    );
    assert_eq!(sink.batch_depth, 0);
}
