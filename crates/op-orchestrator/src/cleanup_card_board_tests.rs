//! Tests for `cleanup_root_patches::centre_card_board_content` (DS P2-b B):
//! a card board whose real layout proves a trailing void of >= 20% of the
//! board height gets the same one-line `justifyContent:"center"` patch the
//! deck centre uses; everything else — small void, explicit distribution,
//! deck boards, hug heights, a second run — is left alone.

use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::RepairSummary;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::EditorCommand;
use serde_json::json;

fn insert_tree(sink: &mut VecDocSink, tree: &serde_json::Value) {
    let tree: PenNode = serde_json::from_value(tree.clone()).expect("test tree json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn root_json(sink: &VecDocSink) -> serde_json::Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialize")
}

/// A 1080x1440 card board with one bounded section `height` tall: content
/// stacks from the top, leaving `1440 - height` px of trailing void.
fn sparse_card(section_height: f64) -> serde_json::Value {
    json!({
        "type": "frame",
        "id": "card",
        "name": "知识卡片",
        "width": 1080,
        "height": 1440,
        "layout": "vertical",
        "children": [
            { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
              "width": 900, "height": section_height,
              "children": [{ "type": "text", "id": "body-text", "content": "正文", "fontSize": 28 }] }
        ]
    })
}

#[test]
fn a_card_board_with_a_proven_trailing_void_is_centred() {
    // 600px of content on a 1440px board: 58% trailing void — the centre
    // patch splits it in half instead of leaving the lower board empty.
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &sparse_card(600.0));

    centre_card_board_content(&mut sink, "card");
    assert_eq!(
        root_json(&sink)["justifyContent"].as_str(),
        Some("center"),
        "the proven void must trigger the centre patch"
    );
}

#[test]
fn a_card_board_with_a_small_void_is_left_alone() {
    // 1300px of content: 9.7% void. A board that full is not top-stacked —
    // the narrow predicate must stand down.
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &sparse_card(1300.0));

    centre_card_board_content(&mut sink, "card");
    assert_eq!(
        root_json(&sink).get("justifyContent"),
        None,
        "a <20% void proves nothing"
    );
}

#[test]
fn a_card_board_with_explicit_distribution_is_left_alone() {
    // space_between is a composition, not the default top-stack — the same
    // exemption the deck centre carries.
    let mut tree = sparse_card(600.0);
    tree["justifyContent"] = json!("space_between");
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &tree);

    centre_card_board_content(&mut sink, "card");
    assert_eq!(
        root_json(&sink)["justifyContent"].as_str(),
        Some("space_between"),
        "an explicit distribution must survive"
    );
}

#[test]
fn a_deck_board_does_not_fall_into_the_card_centre_gate() {
    // A 16:9 deck with a 72% void stays with the deck centre mounted
    // earlier in the driver; the card gate must not claim it.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "deck", "name": "Cover",
            "width": 1920, "height": 1080, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "body", "name": "Body", "layout": "vertical",
                  "width": 1600, "height": 300,
                  "children": [{ "type": "text", "id": "t", "content": "x", "fontSize": 28 }] }
            ]
        }),
    );

    centre_card_board_content(&mut sink, "deck");
    assert!(
        root_json(&sink).get("justifyContent").is_none(),
        "deck boards are the deck gate's business"
    );
}

#[test]
fn a_hug_height_card_shaped_root_is_left_alone() {
    // A non-numeric height never classifies as a fixed card board, so the
    // form gate already rejects it — and the explicit numeric-height gate
    // inside the pass is the same answer for anything that ever reaches it.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "card", "name": "知识卡片",
            "width": 1080, "height": "fit_content", "layout": "vertical",
            "children": [
                { "type": "text", "id": "t", "content": "正文", "fontSize": 28 }
            ]
        }),
    );

    centre_card_board_content(&mut sink, "card");
    assert!(
        root_json(&sink).get("justifyContent").is_none(),
        "a hug board resolves to its content — no fixed surface to centre"
    );
}

#[test]
fn a_card_board_centring_is_idempotent() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &sparse_card(600.0));

    centre_card_board_content(&mut sink, "card");
    let applied_after_first = sink.applied.len();
    centre_card_board_content(&mut sink, "card");
    assert_eq!(
        sink.applied.len(),
        applied_after_first,
        "the explicit centre the first run wrote must gate the second run off"
    );
}

#[test]
fn the_driver_attributes_card_centring_to_the_card_board_centre_checkpoint() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &sparse_card(600.0));
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "card".to_string(),
            name: "知识卡片".into(),
            width: 1080.0,
            height: 1440.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();
    crate::cleanup::run_cleanup_passes_with_summary(&mut sink, &plan, &["card"], &mut summary);

    assert_eq!(
        root_json(&sink)["justifyContent"].as_str(),
        Some("center"),
        "the driver must centre the proven-void card"
    );
    assert!(
        summary
            .records()
            .iter()
            .any(|record| record.pass == "card-board-centre"),
        "the centre patch must be checkpointed as card-board-centre: {:?}",
        summary.records()
    );
}
