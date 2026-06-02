//! Tests for `mcp::write_tools::CopyNode`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Tool-layer validation + `EditorCommand` emission,
//! plus end-to-end `EditorState::apply` checks; the apply-path
//! correctness is covered by `op-editor-core`'s `command_tests.rs`.

use super::reparent_tools::*;
use super::test_fixtures::sample;
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::NodeId;
use std::collections::BTreeMap;

#[test]
fn copy_node_validates_args() {
    let tool = copy_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("target_parent_id"));
        }
        _ => panic!(),
    }
    // node_id == target_parent_id IS allowed.
    args.insert("node_id".into(), "n10".into());
    args.insert("target_parent_id".into(), "n10".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                page_id,
            },
        ) => {
            assert_eq!(node_id.as_str(), "n10");
            assert_eq!(target_parent.as_str(), "n10");
            assert!(page_id.is_none());
        }
        other => panic!("expected CopyNode, got {other:?}"),
    }
}

#[test]
fn copy_node_maps_zero_target_to_page_root() {
    let tool = copy_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n12".into());
    args.insert("target_parent_id".into(), "0".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::CopyNode { target_parent, .. }) => {
            assert!(!target_parent.is_real(), "0 maps to page-root NONE");
        }
        other => panic!("expected CopyNode, got {other:?}"),
    }
}

#[test]
fn copy_node_accepts_ts_source_parent_and_page_args() {
    let tool = copy_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("sourceId".into(), "n12".into());
    args.insert("parent".into(), "n10".into());
    args.insert("pageId".into(), "page-2".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                page_id,
            },
        ) => {
            assert_eq!(node_id.as_str(), "n12");
            assert_eq!(target_parent.as_str(), "n10");
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected CopyNode command with TS args, got {other:?}"),
    }
}

#[test]
fn copy_node_command_clones_to_page_root() {
    let mut s = sample();
    let pre_root_len = s.active_children().len();
    assert!(s.apply(EditorCommand::CopyNode {
        node_id: NodeId::new("n12"),
        target_parent: NodeId::NONE,
        page_id: None,
    }));
    assert_eq!(s.active_children().len(), pre_root_len + 1);
}

#[test]
fn copy_node_command_rejects_unknown_source_or_target() {
    let mut s = sample();
    assert!(!s.apply(EditorCommand::CopyNode {
        node_id: NodeId::new("n99999"),
        target_parent: NodeId::NONE,
        page_id: None,
    }));
    assert!(!s.apply(EditorCommand::CopyNode {
        node_id: NodeId::new("n11"),
        target_parent: NodeId::new("n99999"),
        page_id: None,
    }));
}
