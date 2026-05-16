//! Tests for `mcp::extra_read_tools::GetNodeChildren`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`.

use super::extra_read_tools::*;
use super::test_fixtures::{frame, rect, state_with, text};
use super::{McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::EditorState;
use std::collections::BTreeMap;

fn state_with_frame_and_two_children() -> EditorState {
    let child_a = rect("n20", "a", 10.0, 10.0, 30.0, 30.0);
    let child_b = text("n21", "b", 50.0, 10.0, 30.0, 20.0, "b");
    let f = frame(
        "n10",
        "frame",
        0.0,
        0.0,
        200.0,
        100.0,
        vec![child_a, child_b],
    );
    state_with(vec![f])
}

#[test]
fn get_node_children_returns_count_and_ids_for_known_parent() {
    let s = state_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n10".into());
    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"2".to_string()));
            assert_eq!(out.get("ids"), Some(&"n20,n21".to_string()));
            assert_eq!(out.get("child_0_id"), Some(&"n20".to_string()));
            assert_eq!(out.get("child_0_kind"), Some(&"rect".to_string()));
            assert_eq!(out.get("child_1_kind"), Some(&"text".to_string()));
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn get_node_children_errors_on_unknown_id() {
    let s = state_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n9999".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("9999"));
        }
        other => panic!("expected ToolFailed, got {other:?}"),
    }
}

#[test]
fn get_node_children_returns_empty_for_leaf_id() {
    let s = state_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n20".into());
    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"0".to_string()));
            assert_eq!(out.get("ids"), Some(&"".to_string()));
        }
        other => panic!("expected Ok(count=0) for known leaf, got {other:?}"),
    }
}

#[test]
fn get_node_children_returns_empty_for_known_container_without_children() {
    // An empty Frame at the page root — known id, no children.
    let f = frame("n42", "empty", 0.0, 0.0, 100.0, 100.0, vec![]);
    let s = state_with(vec![f]);
    let tool = get_node_children_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n42".into());
    match tool.call(&args) {
        ToolOutcome::Ok(out) => assert_eq!(out.get("count"), Some(&"0".to_string())),
        other => panic!("expected Ok(count=0) for empty container, got {other:?}"),
    }
}

#[test]
fn get_node_children_rejects_missing_arg() {
    let s = state_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        other => panic!("expected MissingArgument, got {other:?}"),
    }
}

#[test]
fn get_node_children_rejects_unknown_string_id() {
    let s = state_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "not-a-known-id".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::ToolFailed),
        other => panic!("expected ToolFailed, got {other:?}"),
    }
}
