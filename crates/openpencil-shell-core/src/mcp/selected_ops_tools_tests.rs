//! Tests for `mcp::selected_ops_tools`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Tool-layer validation + `EditorCommand` emission;
//! the apply-path correctness is covered by `op-editor-core`'s
//! `command_tests.rs`.

use super::selected_ops_tools::*;
use super::test_fixtures::{rect, state_with};
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::{EditorState, NodeId, ReorderDirection};
use std::collections::BTreeMap;

fn state_with_three_rects() -> EditorState {
    let mut s = state_with(vec![
        rect("n101", "a", 0.0, 0.0, 50.0, 50.0),
        rect("n102", "b", 100.0, 30.0, 50.0, 50.0),
        rect("n103", "c", 200.0, 70.0, 50.0, 50.0),
    ]);
    s.selection.set = vec![
        NodeId::new("n101"),
        NodeId::new("n102"),
        NodeId::new("n103"),
    ];
    s.selection.anchor = NodeId::new("n103");
    s
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
            ToolOutcome::OkWithCommand(_, EditorCommand::AlignSelected { action: got }) => {
                assert_eq!(got, action);
            }
            other => panic!("action {action:?}: expected AlignSelected, got {other:?}"),
        }
    }
}

#[test]
fn align_selected_left_command_applies() {
    let mut s = state_with_three_rects();
    assert!(s.apply(EditorCommand::AlignSelected {
        action: "left".into(),
    }));
}

#[test]
fn align_selected_unknown_action_command_rejects_at_apply_time() {
    let mut s = state_with_three_rects();
    assert!(!s.apply(EditorCommand::AlignSelected {
        action: "bogus".into(),
    }));
}

#[test]
fn duplicate_selected_returns_command_with_default_offset() {
    let tool = duplicate_selected_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkWithCommand(_, EditorCommand::DuplicateSelected { offset_px }) => {
            assert_eq!(offset_px, 10);
        }
        other => panic!("expected DuplicateSelected, got {other:?}"),
    }
}

#[test]
fn nudge_selected_rejects_both_zero() {
    let tool = nudge_selected_snapshot();
    let mut args = BTreeMap::new();
    args.insert("dx".into(), "0".into());
    args.insert("dy".into(), "0".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn reorder_selected_maps_direction_string_to_enum() {
    let tool = reorder_selected_snapshot();
    let mut args = BTreeMap::new();
    args.insert("direction".into(), "up".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::ReorderSelected { direction }) => {
            assert_eq!(direction, ReorderDirection::Up);
        }
        other => panic!("expected ReorderSelected, got {other:?}"),
    }
    args.insert("direction".into(), "sideways".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn cut_selected_emits_command() {
    let tool = cut_selected_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkWithCommand(_, EditorCommand::CutSelected) => {}
        other => panic!("expected CutSelected, got {other:?}"),
    }
}

#[test]
fn cut_selected_command_rejects_empty_selection() {
    let mut s = state_with_three_rects();
    s.clear_selection();
    assert!(!s.apply(EditorCommand::CutSelected));
}
