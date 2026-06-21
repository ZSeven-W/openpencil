//! Task B2 (S3b-4) tests — append-to-document mode wiring in `Orchestrator::run()`.
//!
//! Wired as `#[path = "run_tests_b4.rs"] mod tests_b4;` inside `run.rs`;
//! stays a child module of `run`, so `use super::*` resolves to `run`.
//!
//! Covers:
//! - Append mode: no scaffold `InsertSubtree` for the root, every subtask's
//!   `parent_frame_id` is the ctx target, effective concurrency forced to 1.
//! - Append mode: sub-agent content lands as new children of `ctx.target_parent_id`.
//! - Non-append mode: all prior paths unchanged (covered by green existing tests).

use super::*;
use crate::test_support::{
    ScriptResponse, ScriptedLlm, SkippedPreValidator, SkippedScreenshotProvider,
    SkippedVisionLlmClient, VecDocSink,
};
use crate::types::{AppendContext, DesignRequest};
use op_editor_core::{EditorCommand, PenNodeExt};

fn stub_providers() -> ValidationProviders<'static> {
    ValidationProviders {
        pre_validator: &SkippedPreValidator,
        screenshot: &SkippedScreenshotProvider,
        vision: &SkippedVisionLlmClient,
        system_prompt: String::new(),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn node_json(prefix: &str) -> String {
    format!(
        r#"[{{"type":"frame","id":"{prefix}-1","name":"Sec","x":0,"y":0,"width":1200,"height":300,"children":[{{"type":"text","id":"{prefix}-title","content":"{prefix}","fontSize":18}}]}}]"#
    )
}

/// Insert a target frame into `sink` and return its LIVE id
/// (the remapped `n{N}` id assigned by `cmd_insert_subtree`).
/// The target frame is an empty vertical frame of 1200 wide.
fn insert_target_frame(sink: &mut VecDocSink, hint_id: &str) -> String {
    let node: jian_ops_schema::node::PenNode = serde_json::from_str(&format!(
        r#"{{"type":"frame","id":"{hint_id}","name":"Existing","x":0,"y":0,
             "width":1200,"height":800,"layout":"vertical","gap":0,"children":[]}}"#
    ))
    .expect("parse target node");
    let before_count = sink.state().active_children().len();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    // The newly inserted node is appended at the end.
    sink.state()
        .active_children()
        .get(before_count)
        .map(|n| n.id_str().to_string())
        .unwrap_or_else(|| hint_id.to_string())
}

/// Insert a target frame for mobile (390 wide) and return its live id.
fn insert_target_frame_mobile(sink: &mut VecDocSink, hint_id: &str) -> String {
    let node: jian_ops_schema::node::PenNode = serde_json::from_str(&format!(
        r#"{{"type":"frame","id":"{hint_id}","name":"Existing","x":0,"y":0,
             "width":390,"height":844,"layout":"vertical","gap":0,"children":[]}}"#
    ))
    .expect("parse target node");
    let before_count = sink.state().active_children().len();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    sink.state()
        .active_children()
        .get(before_count)
        .map(|n| n.id_str().to_string())
        .unwrap_or_else(|| hint_id.to_string())
}

fn req_append(live_target_id: &str) -> DesignRequest {
    DesignRequest {
        prompt: "add a pricing section".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: Some(AppendContext {
            target_parent_id: live_target_id.into(),
            target_width: 1200.0,
            existing_section_labels: vec!["Hero".into(), "Features".into()],
            is_mobile: false,
        }),
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

fn req_append_concurrent(live_target_id: &str) -> DesignRequest {
    DesignRequest {
        prompt: "add more screens".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 4,
        append_context: Some(AppendContext {
            target_parent_id: live_target_id.into(),
            target_width: 390.0,
            existing_section_labels: vec![],
            is_mobile: false,
        }),
        validation_enabled: true,

        visual_ref_enabled: false,
    }
}

// ── Task B2 tests ─────────────────────────────────────────────────────────────

/// (a) Append mode: run does NOT emit a scaffold-root InsertSubtree.
///
/// We pre-insert the target frame, capture its live remapped id, then run
/// with append mode.  The run should emit only 2 sub-agent InsertSubtrees
/// (one per subtask), NOT a scaffold-root InsertSubtree.
#[test]
fn append_mode_does_not_emit_scaffold_root_insert() {
    // Minimal plan whose rootFrame.id doesn't matter — apply_append_context_to_plan
    // replaces it with ctx.target_parent_id (the live id) before any scaffold call.
    const PLAN_JSON: &str = r##"{
      "rootFrame": { "id": "frame-placeholder", "name": "Page", "width": 1200, "height": 800,
                     "layout": "vertical", "gap": 0,
                     "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
      "subtasks": [
        { "id": "pricing", "label": "Pricing Section", "region": { "width": 1200, "height": 400 } },
        { "id": "cta", "label": "Call to Action", "region": { "width": 1200, "height": 300 } }
      ]
    }"##;

    let mut sink = VecDocSink::new();
    let live_id = insert_target_frame(&mut sink, "hint-target");
    // After pre-setup, sink has 1 InsertSubtree (the pre-insert).
    let pre_insert_count = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert_eq!(pre_insert_count, 1, "sanity: pre-insert count");

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("pricing")),
        ScriptResponse::Text(node_json("cta")),
    ]);
    let abort = AbortFlag::new();

    let result = futures::executor::block_on(Orchestrator::new().run(
        req_append(&live_id),
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ));

    assert!(result.is_ok(), "append run should succeed: {result:?}");

    // Count InsertSubtrees AFTER pre-setup: 2 sub-agent inserts expected,
    // NOT 3 (which would indicate a scaffold-root insert).
    let total_inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    // 1 (pre-setup target frame) + 2 (sub-agents) = 3 total.
    // If scaffold were inserted, it would be 4.
    assert_eq!(
        total_inserts, 3,
        "expected exactly 3 InsertSubtrees (1 pre-setup + 2 sub-agents, no scaffold root): got {total_inserts}"
    );
}

/// (b) Append mode: summary.root_frame_id equals ctx.target_parent_id.
#[test]
fn append_mode_summary_root_equals_target_parent_id() {
    const PLAN_JSON: &str = r##"{
      "rootFrame": { "id": "frame-placeholder", "name": "Page", "width": 1200, "height": 800,
                     "layout": "vertical", "gap": 0,
                     "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
      "subtasks": [
        { "id": "pricing", "label": "Pricing Section", "region": { "width": 1200, "height": 400 } },
        { "id": "cta", "label": "Call to Action", "region": { "width": 1200, "height": 300 } }
      ]
    }"##;

    let mut sink = VecDocSink::new();
    let live_id = insert_target_frame(&mut sink, "hint-target");

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("pricing")),
        ScriptResponse::Text(node_json("cta")),
    ]);
    let abort = AbortFlag::new();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_append(&live_id),
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ))
    .expect("append run ok");

    assert_eq!(
        summary.root_frame_id, live_id,
        "root_frame_id must match ctx.target_parent_id (the live id)"
    );
    assert_eq!(summary.subtasks.len(), 2, "both subtasks should appear");
}

/// (c) Append mode: effective concurrency is 1 even with concurrency=4 + 2 screens.
///
/// With `concurrency=4` and a plan with 2 distinct screens, the normal path
/// would trigger concurrent multi-screen mode.  Append mode forces sequential.
#[test]
fn append_mode_forces_effective_concurrency_to_one() {
    const MULTISCREEN_PLAN_JSON: &str = r##"{
      "rootFrame": { "id": "frame-placeholder", "name": "App", "width": 390, "height": 844,
                     "layout": "vertical", "gap": 0,
                     "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
      "subtasks": [
        { "id": "login", "label": "Login Screen", "region": { "width": 390, "height": 844 },
          "screen": "Login" },
        { "id": "home", "label": "Home Screen", "region": { "width": 390, "height": 844 },
          "screen": "Home" }
      ]
    }"##;

    let mut sink = VecDocSink::new();
    let live_id = insert_target_frame_mobile(&mut sink, "hint-screens");

    // 2 sub-agent calls = sequential path.  If concurrent, the scripted LLM
    // would receive 2 parallel calls which our ScriptedLlm handles fine, but
    // the summary would have 2 separate root frames.  We verify single root.
    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(MULTISCREEN_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("login")),
        ScriptResponse::Text(node_json("home")),
    ]);
    let abort = AbortFlag::new();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_append_concurrent(&live_id),
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ))
    .expect("append concurrent run ok");

    // Sequential path: exactly 1 root (the target frame).
    assert_eq!(
        summary.root_frame_id, live_id,
        "root_frame_id must be the target in sequential mode"
    );
    assert_eq!(summary.subtasks.len(), 2);

    // 1 pre-setup + 2 sub-agent InsertSubtrees = 3 total (no N scaffold roots).
    let inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert_eq!(
        inserts, 3,
        "expected 3 InsertSubtrees (1 pre-setup + 2 sub-agents): got {inserts}"
    );
}

/// (d) Append mode: sub-agent content lands as new children of ctx.target_parent_id.
///
/// The target frame starts empty; after the run it must have more descendants.
#[test]
fn append_mode_content_lands_in_target_frame() {
    const PLAN_JSON: &str = r##"{
      "rootFrame": { "id": "frame-placeholder", "name": "Page", "width": 1200, "height": 800,
                     "layout": "vertical", "gap": 0,
                     "fill": [{ "type": "solid", "color": "#FFFFFF" }] },
      "subtasks": [
        { "id": "pricing", "label": "Pricing Section", "region": { "width": 1200, "height": 400 } },
        { "id": "cta", "label": "Call to Action", "region": { "width": 1200, "height": 300 } }
      ]
    }"##;

    let mut sink = VecDocSink::new();
    let live_id = insert_target_frame(&mut sink, "hint-target");
    let before_count = descendant_count(sink.state(), &live_id);

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("pricing")),
        ScriptResponse::Text(node_json("cta")),
    ]);
    let abort = AbortFlag::new();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req_append(&live_id),
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ))
    .expect("append run ok");

    let after_count = descendant_count(sink.state(), &live_id);

    assert!(
        after_count > before_count,
        "target frame must gain descendants: before={before_count} after={after_count}"
    );
    assert!(
        summary.total_nodes >= 2,
        "total_nodes must reflect sub-agent output"
    );
}

/// Non-append mode: existing sequential path is unchanged.
///
/// A request without `append_context` takes the normal scaffold+sequential
/// path: it inserts a scaffold root frame AND the sub-agent nodes.
#[test]
fn non_append_mode_takes_normal_sequential_path() {
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
    let abort = AbortFlag::new();

    let req = DesignRequest {
        prompt: "a landing page".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None, // no append context
        validation_enabled: true,

        visual_ref_enabled: false,
    };

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req,
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ))
    .expect("non-append run ok");

    // Normal path: scaffold root + 2 subtask inserts = at least 3 InsertSubtree.
    let inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert!(
        inserts >= 3,
        "expected >=3 InsertSubtrees (scaffold + subtasks): got {inserts}"
    );
    assert!(!summary.root_frame_id.is_empty());
    assert_eq!(summary.subtasks.len(), 2);
}

#[test]
fn non_append_mode_resolves_scaffold_root_when_empty_frame_is_replaced() {
    const PLAN_JSON: &str = r##"{
      "rootFrame": { "id": "page", "name": "Food App Home", "width": 375, "height": 812,
                     "layout": "vertical", "gap": 16,
                     "fill": [{ "type": "solid", "color": "#FFF8F0" }] },
      "subtasks": [
        { "id": "hero", "label": "Hero", "region": { "width": 375, "height": 240 } }
      ]
    }"##;

    let mut sink = VecDocSink::new();
    insert_target_frame(&mut sink, "starter-frame");
    assert_eq!(sink.state().active_children().len(), 1);

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(PLAN_JSON.into()),
        ScriptResponse::Text(node_json("hero")),
    ]);
    let abort = AbortFlag::new();
    let req = DesignRequest {
        prompt: "a mobile food app".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
    };

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req,
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ))
    .expect("normal run should replace the empty starter frame");

    assert_eq!(sink.state().active_children().len(), 1);
    assert_eq!(
        sink.state().active_children()[0].id_str(),
        summary.root_frame_id
    );
    assert_ne!(summary.root_frame_id, "page");
}

// ── Dashboard / append mutex (spec §2) ────────────────────────────────────────

/// Append + dashboard request → append fast-path wins (dashboard does NOT fire).
///
/// The request carries `Some(append_context)` AND a prompt + plan that would
/// normally trigger `should_use_dashboard_columns`:
/// - prompt "an analytics admin dashboard" → dashboard keyword
/// - rootFrame.width 1440 (> 480)
/// - sidebar subtask + metrics subtask
///
/// Without the mutex guard the dashboard path would dispatch first, inserting
/// a multi-frame dashboard scaffold (sidebar + main + row frames).  With the
/// guard, the append fast-path takes priority — no scaffold-root InsertSubtree
/// beyond the 1 pre-setup target, content lands in `ctx.target_parent_id`.
#[test]
fn append_mode_wins_over_dashboard_branch() {
    // Dashboard-like plan: 1440 wide, sidebar + metrics subtasks, dashboard
    // keyword in subtask labels.
    const DASHBOARD_PLAN_JSON: &str = r##"{
      "rootFrame": { "id": "frame-placeholder", "name": "Analytics Dashboard",
                     "width": 1440, "height": 900,
                     "layout": "vertical", "gap": 20,
                     "fill": [{ "type": "solid", "color": "#0F172A" }] },
      "subtasks": [
        { "id": "sidebar-nav", "label": "Sidebar Navigation",
          "region": { "width": 260, "height": 760 } },
        { "id": "metrics-panel", "label": "Metrics Chart analytics dashboard",
          "region": { "width": 900, "height": 320 } }
      ]
    }"##;

    // Pre-insert the target frame (width 1440 to match the would-be dashboard).
    let mut sink = VecDocSink::new();
    let live_id = {
        let node: jian_ops_schema::node::PenNode = serde_json::from_str(
            r#"{"type":"frame","id":"hint-dash","name":"Existing","x":0,"y":0,
                 "width":1440,"height":900,"layout":"vertical","gap":0,"children":[]}"#,
        )
        .expect("parse target node");
        let before_count = sink.state().active_children().len();
        sink.apply(EditorCommand::InsertSubtree {
            nodes: vec![node],
            parent_id: op_editor_core::NodeId::NONE,
            page_id: None,
        });
        sink.state()
            .active_children()
            .get(before_count)
            .map(|n| n.id_str().to_string())
            .unwrap_or_else(|| "hint-dash".to_string())
    };
    let before_count = descendant_count(sink.state(), &live_id);

    // Prompt carries the dashboard keyword so without the mutex guard
    // `should_use_dashboard_columns` would return true.
    let req = DesignRequest {
        prompt: "an analytics admin dashboard".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        append_context: Some(AppendContext {
            target_parent_id: live_id.clone(),
            target_width: 1440.0,
            existing_section_labels: vec!["Hero".into()],
            is_mobile: false,
        }),
        validation_enabled: true,

        visual_ref_enabled: false,
    };

    let llm = ScriptedLlm::new(vec![
        ScriptResponse::Text(DASHBOARD_PLAN_JSON.into()),
        ScriptResponse::Text(node_json("sidebar")),
        ScriptResponse::Text(node_json("metrics")),
    ]);
    let abort = AbortFlag::new();

    let summary = futures::executor::block_on(Orchestrator::new().run(
        req,
        &mut sink,
        &llm,
        &mut |_| {},
        &abort,
        &stub_providers(),
    ))
    .expect("append+dashboard run ok");

    // Append fast-path won: root_frame_id matches the live target.
    assert_eq!(
        summary.root_frame_id, live_id,
        "append fast-path must win — root_frame_id should be the target id"
    );

    // No dashboard scaffold: total InsertSubtree count is exactly
    // 1 (pre-setup) + 2 (sub-agents) = 3.  Dashboard scaffold would push
    // 1 root + 1 sidebar + 1 main + N row frames → >>3.
    let inserts = sink
        .applied
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert_eq!(
        inserts, 3,
        "expected 3 InsertSubtrees (1 pre-setup + 2 sub-agents, no dashboard scaffold): got {inserts}"
    );

    // Content landed in target frame.
    let after_count = descendant_count(sink.state(), &live_id);
    assert!(
        after_count > before_count,
        "target frame must gain descendants: before={before_count} after={after_count}"
    );
}
