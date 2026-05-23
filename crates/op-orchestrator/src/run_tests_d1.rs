//! Task D1 tests — wire `run_post_generation_validation` into all 3 paths.
//!
//! Wired as `#[path = "run_tests_d1.rs"] mod tests_d1;` inside `run.rs`;
//! stays a child module of `run`, so `use super::*` resolves to `run`.
//!
//! Covers:
//! - `validation_enabled = true` + stub providers → all 3 paths emit
//!   `ValidationDone` in Progress after `CleanupDone`.
//! - `validation_enabled = false` → no `ValidationStarted` / `ValidationDone`
//!   events emitted.
//! - Abort fires before validation → no validation events (path returns
//!   `OrchestratorError::Aborted`).

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{OrchestratorError, Progress};

// ── Helpers ───────────────────────────────────────────────────────────────────

const PLAN_JSON: &str = r##"{
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
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[]}}]"#
    )
}

/// Multi-screen plan — triggers the concurrent path when concurrency=2.
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

fn node_json_mobile(prefix: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":390,"height":300,"children":[]}}]"#
    )
}

/// Dashboard plan — triggers the dashboard path.
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

fn req_validation_enabled() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: true,
    }
}

fn req_validation_disabled() -> DesignRequest {
    DesignRequest {
        prompt: "a landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
    }
}

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: "validation system prompt".to_string(),
    }
}

// ── Sequential path ───────────────────────────────────────────────────────────

/// Sequential path: `validation_enabled = true` + stub providers →
/// `ValidationDone` emitted after `CleanupDone`.
#[test]
fn sequential_validation_enabled_emits_validation_done() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let providers = stub_providers();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_validation_enabled(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &providers,
    ))
    .expect("sequential validation run ok");

    assert!(!summary.root_frame_id.is_empty());
    assert_eq!(summary.subtasks.len(), 2);

    // Verify CleanupDone comes BEFORE ValidationStarted.
    let cleanup_pos = events
        .iter()
        .position(|e| matches!(e, Progress::CleanupDone))
        .expect("CleanupDone expected");
    let validation_started_pos = events
        .iter()
        .position(|e| matches!(e, Progress::ValidationStarted))
        .expect("ValidationStarted expected");
    let validation_done_pos = events
        .iter()
        .position(|e| matches!(e, Progress::ValidationDone { .. }))
        .expect("ValidationDone expected");

    assert!(
        cleanup_pos < validation_started_pos,
        "CleanupDone should precede ValidationStarted"
    );
    assert!(
        validation_started_pos < validation_done_pos,
        "ValidationStarted should precede ValidationDone"
    );
}

/// Sequential path: `validation_enabled = false` → no validation events.
#[test]
fn sequential_validation_disabled_emits_no_validation_events() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let providers = stub_providers();

    let _summary = futures::executor::block_on(Orchestrator::new().run(
        req_validation_disabled(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &providers,
    ))
    .expect("disabled validation run ok");

    let has_validation_started = events
        .iter()
        .any(|e| matches!(e, Progress::ValidationStarted));
    let has_validation_done = events
        .iter()
        .any(|e| matches!(e, Progress::ValidationDone { .. }));

    assert!(
        !has_validation_started,
        "ValidationStarted should NOT be emitted when validation_enabled=false"
    );
    assert!(
        !has_validation_done,
        "ValidationDone should NOT be emitted when validation_enabled=false"
    );
}

/// Sequential path: abort fires during subtask → `Aborted` error,
/// no validation events.
#[test]
fn sequential_abort_before_validation_no_validation_events() {
    use crate::types::LlmError;
    // Abort during planning stream — simplest abort that prevents content.
    let llm = ScriptedLlm::new(vec![ScriptResponse::Fail(LlmError {
        message: "user aborted".into(),
        aborted: true,
    })]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let providers = stub_providers();

    let result = futures::executor::block_on(Orchestrator::new().run(
        req_validation_enabled(),
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &providers,
    ));

    assert!(
        matches!(result, Err(OrchestratorError::Aborted)),
        "expected Aborted, got: {result:?}"
    );
    let has_validation_started = events
        .iter()
        .any(|e| matches!(e, Progress::ValidationStarted));
    assert!(
        !has_validation_started,
        "ValidationStarted should NOT fire after abort"
    );
}

/// Sequential path: abort flag set externally just before the validation
/// hook fires (all subtasks succeed, but abort fires between cleanup and
/// validation). Since abort is checked `!abort.is_set()` at the hook site,
/// no validation runs.
#[test]
fn sequential_abort_set_externally_skips_validation() {
    use std::sync::{Arc, Mutex};

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
        ScriptResponse::Text(node_json("feat")),
    ]);
    let mut sink = VecDocSink::new();
    // Use Arc<Mutex<Vec>> so the closure can capture it while we still read it
    // after the run completes (avoids the move-then-borrow error).
    let events: Arc<Mutex<Vec<Progress>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&events);
    let abort = AbortFlag::new();
    // The meaningful scenario here is: all generation succeeds, abort is
    // set at the hook site AFTER cleanup. We simulate this by using a
    // progress callback that sets the abort flag when CleanupDone fires.
    let abort_for_closure = abort.clone();
    let mut on_progress = move |p: Progress| {
        events_clone.lock().unwrap().push(p.clone());
        if matches!(p, Progress::CleanupDone) {
            abort_for_closure.set();
        }
    };
    let providers = stub_providers();

    let _result = futures::executor::block_on(Orchestrator::new().run(
        req_validation_enabled(),
        &mut sink,
        &llm,
        &mut on_progress,
        &abort,
        &providers,
    ));

    // After abort, the run either succeeds (content was already produced) or
    // errors with Aborted/NoContent. In either case, no validation should run.
    let captured = events.lock().unwrap();
    let has_validation_started = captured
        .iter()
        .any(|e| matches!(e, Progress::ValidationStarted));
    assert!(
        !has_validation_started,
        "ValidationStarted should NOT fire when abort is set before validation hook"
    );
}

// ── Dashboard path ────────────────────────────────────────────────────────────

/// Dashboard path: `validation_enabled = true` → `ValidationDone` emitted.
#[test]
fn dashboard_validation_enabled_emits_validation_done() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("sidebar")),
        ScriptResponse::Text(node_json("metrics")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let providers = stub_providers();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        DesignRequest {
            prompt: "an analytics admin dashboard".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: true,
        },
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &providers,
    ))
    .expect("dashboard validation run ok");

    assert!(!summary.root_frame_id.is_empty());

    let has_cleanup = events.iter().any(|e| matches!(e, Progress::CleanupDone));
    let has_validation_done = events
        .iter()
        .any(|e| matches!(e, Progress::ValidationDone { .. }));

    assert!(has_cleanup, "CleanupDone expected in dashboard path");
    assert!(
        has_validation_done,
        "ValidationDone expected in dashboard path"
    );

    // CleanupDone precedes ValidationDone.
    let cleanup_pos = events
        .iter()
        .position(|e| matches!(e, Progress::CleanupDone))
        .unwrap();
    let done_pos = events
        .iter()
        .position(|e| matches!(e, Progress::ValidationDone { .. }))
        .unwrap();
    assert!(
        cleanup_pos < done_pos,
        "CleanupDone should precede ValidationDone in dashboard path"
    );
}

/// Dashboard path: `validation_enabled = false` → no validation events.
#[test]
fn dashboard_validation_disabled_no_validation_events() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("sidebar")),
        ScriptResponse::Text(node_json("metrics")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let providers = stub_providers();

    let _summary = futures::executor::block_on(Orchestrator::new().run(
        DesignRequest {
            prompt: "an analytics admin dashboard".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 1,
            append_context: None,
            validation_enabled: false,
        },
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &providers,
    ))
    .expect("dashboard disabled validation run ok");

    let has_validation_started = events
        .iter()
        .any(|e| matches!(e, Progress::ValidationStarted));
    assert!(
        !has_validation_started,
        "ValidationStarted should NOT fire in dashboard path when disabled"
    );
}

// ── Concurrent path ───────────────────────────────────────────────────────────

/// Concurrent path: `validation_enabled = true` → `ValidationDone` emitted.
#[test]
fn concurrent_validation_enabled_emits_validation_done() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json_mobile("login")),
        ScriptResponse::Text(node_json_mobile("home")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let providers = stub_providers();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        DesignRequest {
            prompt: "a mobile app".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 2,
            append_context: None,
            validation_enabled: true,
        },
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &providers,
    ))
    .expect("concurrent validation run ok");

    assert_eq!(summary.subtasks.len(), 2);

    let has_cleanup = events.iter().any(|e| matches!(e, Progress::CleanupDone));
    let has_validation_done = events
        .iter()
        .any(|e| matches!(e, Progress::ValidationDone { .. }));

    assert!(has_cleanup, "CleanupDone expected in concurrent path");
    assert!(
        has_validation_done,
        "ValidationDone expected in concurrent path"
    );

    // CleanupDone precedes ValidationDone.
    let cleanup_pos = events
        .iter()
        .position(|e| matches!(e, Progress::CleanupDone))
        .unwrap();
    let done_pos = events
        .iter()
        .position(|e| matches!(e, Progress::ValidationDone { .. }))
        .unwrap();
    assert!(
        cleanup_pos < done_pos,
        "CleanupDone should precede ValidationDone in concurrent path"
    );
}

/// Concurrent path: `validation_enabled = false` → no validation events.
#[test]
fn concurrent_validation_disabled_no_validation_events() {
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTI_SCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json_mobile("login")),
        ScriptResponse::Text(node_json_mobile("home")),
    ]);
    let mut sink = VecDocSink::new();
    let mut events: Vec<Progress> = Vec::new();
    let mut on_progress = |p: Progress| events.push(p);
    let providers = stub_providers();

    let _summary = futures::executor::block_on(Orchestrator::new().run(
        DesignRequest {
            prompt: "a mobile app".into(),
            model: None,
            provider: None,
            design_md: None,
            concurrency: 2,
            append_context: None,
            validation_enabled: false,
        },
        &mut sink,
        &llm,
        &mut on_progress,
        &AbortFlag::new(),
        &providers,
    ))
    .expect("concurrent disabled validation run ok");

    let has_validation_started = events
        .iter()
        .any(|e| matches!(e, Progress::ValidationStarted));
    assert!(
        !has_validation_started,
        "ValidationStarted should NOT fire in concurrent path when disabled"
    );
}
