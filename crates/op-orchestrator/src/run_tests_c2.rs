//! Task C2 (S3b-2) tests — `Orchestrator::run()` branch on `effective_concurrency`.
//!
//! Wired as `#[path = "run_tests_c2.rs"] mod tests_c2;` inside `run.rs`; stays
//! a child module of `run`, so `use super::*` resolves to `run`.
//!
//! Covers:
//! - Multi-screen plan + `concurrency >= 2` → concurrent path (N root frames,
//!   all subtasks' nodes land in the sink).
//! - Single-screen plan OR `concurrency == 1` → sequential path (all existing
//!   `run_*` tests remain green — checked by the absence of regressions).
//! - Concurrent all-fail → `Err(OrchestratorError::AllFailed(_))`.

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use crate::types::OrchestratorError;
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

/// A multi-screen plan JSON: two subtasks on different screens.
const MULTI_SCREEN_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "App", "width": 390, "height": 844,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "login", "label": "Login Screen",
      "region": { "width": 390, "height": 844 },
      "screen": "Login" },
    { "id": "home", "label": "Home Screen",
      "region": { "width": 390, "height": 844 },
      "screen": "Home" }
  ]
}"##;

/// A single-screen plan JSON: two subtasks, no `screen` field.
const SINGLE_SCREEN_PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } }
  ]
}"##;

fn node_json(prefix: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":390,"height":300,"children":[{{"type":"text","id":"{prefix}-title","content":"{prefix}","fontSize":18}}]}}]"#
    )
}

/// Default concurrent request: 2 screens, concurrency=2.
fn req_concurrent() -> DesignRequest {
    DesignRequest {
        prompt: "a mobile app".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 2,
        append_context: None,
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

/// Sequential request: concurrency=1 (forces the sequential path regardless
/// of screen count).
fn req_sequential() -> DesignRequest {
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

// ── Task C2 tests ─────────────────────────────────────────────────────────────

/// Multi-screen plan (2 screens) + `concurrency=2` → concurrent path.
/// - Two distinct root frames should be inserted (N roots).
/// - Both subtasks' nodes land in the sink (total_nodes >= 2).
/// - `RunSummary.subtasks.len() == 2`.
#[test]
fn run_multiscreen_concurrent_creates_n_roots_and_lands_all_nodes() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        // Two subtask responses — one per screen group (concurrent path uses
        // 2-attempt ladder; both succeed on attempt 1).
        ScriptResponse::Text(node_json("login")),
        ScriptResponse::Text(node_json("home")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_concurrent(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("concurrent run should succeed");

    // Both subtasks reported.
    assert_eq!(summary.subtasks.len(), 2, "expected 2 subtask outcomes");
    // Nodes landed.
    assert!(
        summary.total_nodes >= 2,
        "expected >= 2 nodes, got {}",
        summary.total_nodes
    );
    // At least 2 InsertSubtree commands for the N scaffold roots +
    // at least 2 InsertSubtree commands for the subtask nodes.
    let inserts: Vec<_> = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .collect();
    assert!(
        inserts.len() >= 4,
        "expected >= 4 InsertSubtree (2 scaffold + 2 subtask), got {}",
        inserts.len()
    );
    // Undo batch properly closed.
    assert_eq!(sink.batch_depth, 0);
    // Progress: Planning + ScaffoldDone + SubtaskStarted×2 + SubtaskDone×2
    // + CleanupDone + ValidationStarted + … + ValidationDone.
    assert!(matches!(events.first(), Some(Progress::Planning)));
    // CleanupDone must be present (validation runs after it).
    assert!(
        events.iter().any(|e| matches!(e, Progress::CleanupDone)),
        "expected CleanupDone in events"
    );
}

/// `concurrency=1` with a multi-screen plan → sequential path.
/// Behaves identically to the existing sequential tests (1 root, 3-attempt ladder).
#[test]
fn run_concurrency_one_multiscreen_takes_sequential_path() {
    // Force concurrency=1 even though plan has 2 screens → sequential path.
    let mut req = req_concurrent();
    req.concurrency = 1;

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        // Sequential path: one LLM call per subtask (no grouping).
        ScriptResponse::Text(node_json("login")),
        ScriptResponse::Text(node_json("home")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("sequential run should succeed");

    // Sequential path: one root frame → root_frame_id is non-empty.
    assert!(!summary.root_frame_id.is_empty());
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    assert_eq!(sink.batch_depth, 0);
}

/// Single-screen plan (no `screen` fields) → sequential path even with
/// `concurrency=3`, because `effective_concurrency` returns 1 when
/// `screen_group_count <= 1`.
#[test]
fn run_single_screen_plan_takes_sequential_path_regardless_of_concurrency() {
    let mut req = req_concurrent();
    req.concurrency = 3;

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(SINGLE_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req,
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("single-screen run should succeed");

    // Single root (sequential path).
    assert!(!summary.root_frame_id.is_empty());
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    assert_eq!(sink.batch_depth, 0);
}

/// Concurrent path, all subtasks fail (0 nodes) →
/// `Err(OrchestratorError::AllFailed(_))`.
#[test]
fn run_concurrent_all_fail_returns_all_failed() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        // Both workers produce garbage (2-attempt: each worker tries twice).
        ScriptResponse::Text("garbage".into()),
        ScriptResponse::Text("garbage".into()),
        ScriptResponse::Text("garbage".into()),
        ScriptResponse::Text("garbage".into()),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};

    let result = futures::executor::block_on(Orchestrator::new().run(
        req_concurrent(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ));

    assert!(
        matches!(result, Err(OrchestratorError::AllFailed(_))),
        "all-fail concurrent run should return AllFailed, got: {result:?}"
    );
    // Undo batch properly closed even on error path.
    assert_eq!(sink.batch_depth, 0);
}

/// Existing sequential happy-path still works (regression guard).
/// Exactly mirrors `run_happy_path_applies_scaffold_and_subtasks` in run.rs.
#[test]
fn run_sequential_happy_path_regression() {
    const PLAN_JSON: &str = r##"{
  "rootFrame": { "id": "root", "name": "Page", "width": 1200, "height": 800,
                 "layout": "vertical", "gap": 0,
                 "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
  "subtasks": [
    { "id": "hero", "label": "Hero", "region": { "width": 1200, "height": 400 } },
    { "id": "feat", "label": "Features", "region": { "width": 1200, "height": 400 } }
  ]
}"##;
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut on_progress = |_p: Progress| {};

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_sequential(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &stub_providers(),
    ))
    .expect("sequential happy path");

    assert!(!summary.root_frame_id.is_empty());
    assert_eq!(summary.subtasks.len(), 2);
    assert!(summary.total_nodes >= 2);
    assert_eq!(sink.batch_depth, 0);
}
