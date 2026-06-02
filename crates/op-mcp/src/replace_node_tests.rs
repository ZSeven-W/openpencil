//! Tests for `mcp::write_tools::ReplaceNode`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Tool-layer validation + `EditorCommand` emission,
//! plus end-to-end `EditorState::apply` checks for the destructive-
//! swap guard. The apply-path correctness is covered by
//! `op-editor-core`'s `command_tests.rs`.

use super::test_fixtures::sample;
use super::write_tools::*;
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::{NodeId, PenNodeExt};
use std::collections::BTreeMap;

#[test]
fn replace_node_validates_required_args() {
    let tool = replace_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n11".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("kind"));
        }
        _ => panic!(),
    }
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "Replacement".into());
    args.insert("x".into(), "10".into());
    args.insert("y".into(), "20".into());
    args.insert("width".into(), "100".into());
    args.insert("height".into(), "30".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::ReplaceNode {
                node_id,
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                drop_children,
                page_id,
            },
        ) => {
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(kind, "rect");
            assert_eq!(name, "Replacement");
            assert_eq!(x, 10);
            assert_eq!(y, 20);
            assert_eq!(width, 100);
            assert_eq!(height, 30);
            assert!(fill_hex.is_none());
            assert!(
                !drop_children,
                "default opt-out keeps container subtrees safe"
            );
            assert!(page_id.is_none());
        }
        other => panic!("expected ReplaceNode, got {other:?}"),
    }
}

#[test]
fn replace_node_accepts_ts_node_id_data_and_page_args() {
    let tool = replace_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("nodeId".into(), "n11".into());
    args.insert(
        "data".into(),
        r##"{"type":"rectangle","name":"Replacement","x":10,"y":20,"width":100,"height":30,"fill":[{"type":"solid","color":"#112233"}]}"##.into(),
    );
    args.insert("pageId".into(), "page-2".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::ReplaceNode {
                node_id,
                kind,
                name,
                fill_hex,
                page_id,
                ..
            },
        ) => {
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(kind, "rect");
            assert_eq!(name, "Replacement");
            assert_eq!(fill_hex.as_deref(), Some("#112233"));
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected ReplaceNode command with TS args, got {other:?}"),
    }
}

#[test]
fn replace_node_accepts_ts_data_tree_as_subtree() {
    let tool = replace_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("nodeId".into(), "n11".into());
    args.insert(
        "data".into(),
        r##"{"type":"frame","name":"Card","width":200,"height":120,"children":[{"type":"text","name":"Title","content":"Hello","width":100,"height":24}]}"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::ReplaceSubtree {
                node_id,
                node,
                drop_children,
                page_id,
            },
        ) => {
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(node.base().name.as_deref(), Some("Card"));
            assert_eq!(node.children().expect("children").len(), 1);
            assert!(!drop_children);
            assert!(page_id.is_none());
        }
        other => panic!("expected ReplaceSubtree command from TS data tree, got {other:?}"),
    }
}

#[test]
fn replace_node_preserves_ts_leaf_data_with_extra_fields_as_subtree() {
    let mut args = BTreeMap::new();
    args.insert("nodeId".into(), "n11".into());
    args.insert("data".into(), r##"{"type":"text","name":"Hero","content":"Welcome","width":240,"height":48,"fontSize":24}"##.into());

    let ToolOutcome::OkWithCommand(_, EditorCommand::ReplaceSubtree { node, .. }) =
        replace_node_snapshot().call(&args)
    else {
        panic!("expected ReplaceSubtree preserving TS leaf data");
    };
    let jian_ops_schema::node::PenNode::Text(text) = node.as_ref() else {
        panic!("expected text node, got {node:?}");
    };
    assert_eq!(text.base.name.as_deref(), Some("Hero"));
    assert_eq!(text.font_size, Some(24.0));
}

#[test]
fn replace_node_rejects_bad_kind_and_geometry_and_fill() {
    let tool = replace_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n11".into());
    args.insert("kind".into(), "blob".into());
    args.insert("name".into(), "X".into());
    args.insert("x".into(), "0".into());
    args.insert("y".into(), "0".into());
    args.insert("width".into(), "10".into());
    args.insert("height".into(), "10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
    args.insert("kind".into(), "rect".into());
    args.insert("width".into(), "-5".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
    args.insert("width".into(), "5".into());
    args.insert("fill_hex".into(), "not-a-hex".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn replace_node_command_swaps_leaf_at_same_slot() {
    let mut s = sample();
    // Title (n11) is a leaf — drop_children=false still swaps it.
    assert!(s.apply(EditorCommand::ReplaceNode {
        node_id: NodeId::new("n11"),
        kind: "rect".into(),
        name: "Swapped".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: Some("#ff0000".into()),
        drop_children: false,
        page_id: None,
    }));
}

#[test]
fn replace_node_command_rejects_unknown_id() {
    let mut s = sample();
    assert!(!s.apply(EditorCommand::ReplaceNode {
        node_id: NodeId::new("n99999"),
        kind: "rect".into(),
        name: "Ghost".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        drop_children: false,
        page_id: None,
    }));
}

#[test]
fn replace_node_command_refuses_to_drop_container_children() {
    let mut s = sample();
    // Frame (n10) has children — refuse without explicit consent.
    assert!(!s.apply(EditorCommand::ReplaceNode {
        node_id: NodeId::new("n10"),
        kind: "rect".into(),
        name: "WouldNuke".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: None,
        drop_children: false,
        page_id: None,
    }));
    // With explicit consent the swap succeeds.
    assert!(s.apply(EditorCommand::ReplaceNode {
        node_id: NodeId::new("n10"),
        kind: "rect".into(),
        name: "ExplicitNuke".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: None,
        drop_children: true,
        page_id: None,
    }));
}

#[test]
fn replace_node_rejects_malformed_drop_children() {
    let tool = replace_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n11".into());
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "X".into());
    args.insert("x".into(), "0".into());
    args.insert("y".into(), "0".into());
    args.insert("width".into(), "10".into());
    args.insert("height".into(), "10".into());
    for bad in ["yes", "", "TRUE", "1", "0"] {
        args.insert("drop_children".into(), bad.into());
        match tool.call(&args) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "input {bad:?}");
                assert!(msg.contains("drop_children"), "input {bad:?} msg = {msg}");
            }
            _ => panic!("expected InvalidArgument on malformed drop_children {bad:?}"),
        }
    }
}

#[test]
fn parser_refuses_structured_drop_children_at_wire_layer() {
    use crate::parse_tool_call;
    for bad_json in [
        r#"{"id":1,"method":"replace_node","params":{"node_id":"n10","kind":"rect","name":"X","x":"0","y":"0","width":"5","height":"5","drop_children":{}}}"#,
        r#"{"id":1,"method":"replace_node","params":{"node_id":"n10","kind":"rect","name":"X","x":"0","y":"0","width":"5","height":"5","drop_children":[true]}}"#,
        r#"{"id":1,"method":"replace_node","params":{"node_id":"n10","kind":"rect","name":{},"x":"0","y":"0","width":"5","height":"5"}}"#,
    ] {
        assert!(
            parse_tool_call(bad_json).is_none(),
            "structured arg must refuse parse: {bad_json}"
        );
    }
}
