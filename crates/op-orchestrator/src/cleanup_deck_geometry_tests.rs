//! The geometric half of the deck judgement in cleanup (`root_is_deck_board`)
//! and the provenance gate in front of it.
//!
//! `CleanupPolicy::is_deck` comes from prompt keywords, so it is false on every
//! path that has no prompt — a board generated without anyone typing "PPT" got
//! none of the board treatment. The artboard itself can supply that verdict.
//!
//! But centring is intent-tier: an asymmetric board can be the composition the
//! author wanted, and "no explicit `justifyContent`" does not distinguish an
//! author who just placed their content from a model that top-stacked it. So
//! the geometric verdict only applies to roots the run itself produced
//! (`roots_are_run_output`), which the orchestrator's fresh and append paths
//! can prove and the whole-document loop finalize cannot.

use super::*;
use crate::test_support::VecDocSink;
use op_editor_core::{EditorCommand, NodeId};

/// The policy shape of a fresh/append orchestrator run: these roots are this
/// run's own output, so their geometry may drive intent-tier decisions.
fn run_output_policy() -> CleanupPolicy {
    CleanupPolicy {
        roots_are_run_output: true,
        ..Default::default()
    }
}

/// A top-stacked board with no authored distribution — the shape
/// `centre_deck_board_content` exists to repair.
fn board(width: f64, height: f64) -> jian_ops_schema::node::PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "board",
        "name": "Cover",
        "width": width,
        "height": height,
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": "s1", "name": "Title", "width": "fill_container", "height": "fit_content"},
            {"type": "frame", "id": "s2", "name": "Body", "width": "fill_container", "height": "fit_content"}
        ]
    }))
    .expect("fixture must deserialize as PenNode")
}

fn plan_for(root_id: &str, width: f64, height: f64) -> crate::plan::OrchestratorPlan {
    crate::plan::OrchestratorPlan {
        root_frame: crate::plan::RootFrameSpec {
            id: root_id.to_string(),
            name: "Root".into(),
            width,
            height,
            layout: Some("vertical".into()),
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: Vec::new(),
        style_guide_name: None,
        palette: None,
    }
}

fn run_with(width: f64, height: f64, policy: CleanupPolicy) -> serde_json::Value {
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![board(width, height)],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    let mut summary = crate::repair_summary::RepairSummary::default();
    crate::cleanup::run_cleanup_passes_with_summary_and_policy_for_tests(
        &mut sink,
        &plan_for(&root_id, width, height),
        &[&root_id],
        &mut summary,
        policy,
    );
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialize")
}

#[test]
fn a_board_this_run_produced_is_centred_even_when_no_prompt_said_deck() {
    // The hole the geometric half closes: a user who asks for a 1920x1080
    // 主视觉 without ever typing PPT still drew a board, and `is_deck` (prompt
    // keywords) is false for them.
    let root = run_with(1920.0, 1080.0, run_output_policy());
    assert_eq!(
        root["justifyContent"].as_str(),
        Some("center"),
        "the artboard alone must be enough on this run's own root: {root}"
    );
}

#[test]
fn a_board_this_run_did_not_produce_is_left_alone() {
    // The intent guard. A hand-arranged board reaching a whole-document
    // finalize (the loop path, which passes every top-level root because it
    // cannot tell which are its own) must not be re-composed: an asymmetric
    // board can be exactly what its author wanted, and an author who simply
    // placed their content never set a `justifyContent` for the other guard
    // to notice.
    let root = run_with(1920.0, 1080.0, CleanupPolicy::default());
    assert_eq!(
        root.get("justifyContent")
            .and_then(serde_json::Value::as_str),
        None,
        "pre-existing content must keep its authored composition: {root}"
    );
}

#[test]
fn the_prompt_half_is_not_gated_on_provenance() {
    // Saying "PPT" IS the statement of intent, so the prompt half applies to
    // whatever the run was pointed at — unchanged by the provenance gate.
    let root = run_with(
        1920.0,
        1080.0,
        CleanupPolicy {
            is_deck: true,
            ..Default::default()
        },
    );
    assert_eq!(root["justifyContent"].as_str(), Some("center"));
}

#[test]
fn a_page_is_not_centred_by_the_geometric_half() {
    // The union must be strictly additive. A 1200-wide scrolling page is not
    // a board at any height, and centring one would push its hero away from
    // the top of the viewport.
    let root = run_with(1200.0, 2400.0, run_output_policy());
    assert_eq!(
        root.get("justifyContent")
            .and_then(serde_json::Value::as_str),
        None,
        "a page must be untouched by the deck branch: {root}"
    );
}

#[test]
fn a_deck_wide_long_page_is_not_a_board() {
    // Same aspect gate the budget override now uses: deck WIDTH without deck
    // SHAPE is a long page. Reading it as a board would centre 2000px of
    // content in a 2000px frame — visible, and wrong.
    let root = run_with(1920.0, 2000.0, run_output_policy());
    assert_eq!(
        root.get("justifyContent")
            .and_then(serde_json::Value::as_str),
        None,
        "1920x2000 is a long page, not a projector board: {root}"
    );
}

#[test]
fn an_authored_distribution_still_wins_over_the_geometric_half() {
    // The existing "explicit distribution is a composition" guard is what
    // keeps this union off a board somebody arranged on purpose. It must hold
    // when the deck verdict comes from geometry, not just from the prompt.
    let mut sink = VecDocSink::new();
    let tree: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "board",
        "name": "Cover",
        "width": 1920,
        "height": 1080,
        "layout": "vertical",
        "justifyContent": "space_between",
        "children": [
            {"type": "frame", "id": "s1", "name": "Title", "width": "fill_container", "height": "fit_content"},
            {"type": "frame", "id": "s2", "name": "Meta", "width": "fill_container", "height": "fit_content"}
        ]
    }))
    .expect("fixture must deserialize as PenNode");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    let mut summary = crate::repair_summary::RepairSummary::default();
    crate::cleanup::run_cleanup_passes_with_summary_and_policy_for_tests(
        &mut sink,
        &plan_for(&root_id, 1920.0, 1080.0),
        &[&root_id],
        &mut summary,
        run_output_policy(),
    );
    let root = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize");
    assert_eq!(root["justifyContent"].as_str(), Some("space_between"));
}
