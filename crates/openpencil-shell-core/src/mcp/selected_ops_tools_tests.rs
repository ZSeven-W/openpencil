//! Tests for `mcp::selected_ops_tools` — focus on the new
//! `align_selected` tool's validation + apply semantics.

use super::selected_ops_tools::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

use crate::document::{Document, Node, NodeId, NodeKind, Page};
use crate::{Color, Point2D, Rect};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D { x, y },
        size: Point2D { x: w, y: h },
    }
}

fn doc_with_three_rects() -> Document {
    let mut doc = Document::empty();
    let a = Node::leaf("n101", NodeKind::Rect, "a")
        .with_bounds(rect(0.0, 0.0, 50.0, 50.0))
        .with_fill(Color::RED);
    let b = Node::leaf("n102", NodeKind::Rect, "b")
        .with_bounds(rect(100.0, 30.0, 50.0, 50.0))
        .with_fill(Color::GREEN);
    let c = Node::leaf("n103", NodeKind::Rect, "c")
        .with_bounds(rect(200.0, 70.0, 50.0, 50.0))
        .with_fill(Color::BLUE);
    doc.pages[0] = Page::new("n1", "P", vec![a, b, c]);
    doc.selected_set = vec![NodeId::new("n101"), NodeId::new("n102"), NodeId::new("n103")];
    doc.selected = NodeId::new("n103");
    doc
}

#[test]
fn align_selected_rejects_unknown_action() {
    let tool = align_selected_snapshot();
    let mut args = BTreeMap::new();
    args.insert("action".into(), "rotate-45".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn align_selected_emits_command_for_each_valid_action() {
    let tool = align_selected_snapshot();
    for action in [
        "left",
        "center_h",
        "right",
        "top",
        "center_v",
        "bottom",
        "distribute_h",
        "distribute_v",
    ] {
        let mut args = BTreeMap::new();
        args.insert("action".into(), action.into());
        match tool.call(&args) {
            ToolOutcome::OkWithCommand(_, McpCommand::AlignSelected { action: got }) => {
                assert_eq!(got, action);
            }
            other => panic!("action {action:?}: expected AlignSelected, got {other:?}"),
        }
    }
}

#[test]
fn align_selected_left_snaps_to_min_x() {
    let mut doc = doc_with_three_rects();
    let cmd = McpCommand::AlignSelected {
        action: "left".into(),
    };
    assert!(doc.apply_mcp_command(&cmd));
    for child in &doc.pages[0].children {
        assert!(
            (child.bounds.origin.x - 0.0).abs() < 0.5,
            "node {} should snap to x=0, got {}",
            child.name,
            child.bounds.origin.x
        );
    }
}

#[test]
fn align_selected_distribute_under_three_no_ops() {
    let mut doc = doc_with_three_rects();
    doc.selected_set = vec![NodeId::new("n101"), NodeId::new("n102")];
    doc.selected = NodeId::new("n102");
    let pre_x = doc.pages[0].children[0].bounds.origin.x;
    let cmd = McpCommand::AlignSelected {
        action: "distribute_h".into(),
    };
    assert!(!doc.apply_mcp_command(&cmd), "distribute under 3 must no-op");
    assert_eq!(doc.pages[0].children[0].bounds.origin.x, pre_x);
}

#[test]
fn align_selected_unknown_action_in_command_rejects_at_apply_time() {
    let mut doc = doc_with_three_rects();
    let cmd = McpCommand::AlignSelected {
        action: "bogus".into(),
    };
    assert!(!doc.apply_mcp_command(&cmd));
}

#[test]
fn cut_selected_rolls_back_clipboard_on_failed_delete() {
    // Codex stop-gate: a cut against an all-locked selection
    // used to leave the clipboard mutated even though apply
    // returned false. Pre-stash + restore should make cut
    // atomic from the caller's perspective.
    let mut doc = doc_with_three_rects();
    // Lock every selected node so `delete_selected` rejects.
    for child in doc.pages[0].children.iter_mut() {
        child.locked = true;
    }
    // Seed clipboard with a sentinel so we can assert it stayed
    // intact through the failed cut.
    let sentinel = Node::leaf("n900", NodeKind::Rect, "sentinel")
        .with_bounds(rect(0.0, 0.0, 1.0, 1.0));
    doc.clipboard = vec![sentinel.clone()];

    let cmd = McpCommand::CutSelected;
    assert!(!doc.apply_mcp_command(&cmd), "cut must reject all-locked selection");
    assert_eq!(
        doc.clipboard.len(),
        1,
        "clipboard length must be restored to the pre-cut state"
    );
    assert_eq!(
        doc.clipboard[0].id, sentinel.id,
        "clipboard contents must be the sentinel, not the would-be-cut rects"
    );
    // Doc tree untouched.
    assert_eq!(doc.pages[0].children.len(), 3, "no nodes should be deleted");
}

#[test]
fn cut_selected_clears_clipboard_when_no_selection() {
    let mut doc = doc_with_three_rects();
    doc.selected_set.clear();
    doc.selected = crate::document::NodeId::NONE;
    // Pre-stash with a sentinel so we can assert it's preserved.
    let sentinel = Node::leaf("n800", NodeKind::Rect, "sentinel")
        .with_bounds(rect(0.0, 0.0, 1.0, 1.0));
    doc.clipboard = vec![sentinel.clone()];
    let cmd = McpCommand::CutSelected;
    assert!(!doc.apply_mcp_command(&cmd));
    assert_eq!(
        doc.clipboard.len(),
        1,
        "empty-selection cut must preserve existing clipboard"
    );
    assert_eq!(doc.clipboard[0].id, sentinel.id);
}
