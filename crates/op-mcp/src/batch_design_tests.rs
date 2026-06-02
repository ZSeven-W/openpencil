//! Tests for `mcp::batch_design::BatchDesign`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Tool-layer parsing / validation + a few end-to-end
//! `EditorState::apply` checks; the apply-path correctness is covered
//! by `op-editor-core`'s `command_tests.rs`.

use super::batch_design::*;
use super::test_fixtures::sample;
use super::{BatchInsertItem, EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::PenNodeExt;
use std::collections::BTreeMap;

#[test]
fn batch_design_requires_nodes_json() {
    let tool = batch_design_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("nodes_json"));
        }
        _ => panic!(),
    }
}

#[test]
fn batch_design_rejects_empty_array() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("nodes_json".into(), "[]".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn batch_design_parses_minimal_two_node_array() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r##"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":20},{"kind":"ellipse","name":"B","x":40,"y":50,"width":30,"height":30,"fill_hex":"#ff0000"}]"##
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(result, EditorCommand::BatchInsert { items }) => {
            assert_eq!(result.get("count"), Some(&"2".to_string()));
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].kind, "rect");
            assert_eq!(items[0].name, "A");
            assert_eq!(items[0].width, 10);
            assert_eq!(items[0].height, 20);
            assert!(items[0].fill_hex.is_none());
            assert_eq!(items[1].kind, "ellipse");
            assert_eq!(items[1].fill_hex.as_deref(), Some("#ff0000"));
        }
        other => panic!("expected BatchInsert, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_ts_insert_operations_tree() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Page","width":320,"height":240})
label=I(root, {"type":"text","name":"Greeting","content":"Hello","width":120,"height":24})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(result, EditorCommand::InsertSubtree { nodes, parent_id }) => {
            assert_eq!(result.get("count"), Some(&"2".to_string()));
            assert!(!parent_id.is_real());
            assert_eq!(nodes.len(), 1);
            let root = &nodes[0];
            assert!(root.is_container());
            assert_eq!(root.children().expect("children").len(), 1);
            assert_eq!(
                root.children().unwrap()[0].base().name.as_deref(),
                Some("Greeting")
            );
        }
        other => panic!("expected InsertSubtree command, got {other:?}"),
    }
}

#[test]
fn batch_design_insert_operations_apply_as_one_nested_subtree() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"card=I(null, {"type":"frame","name":"Card","width":200,"height":120})
title=I(card, {"type":"text","name":"Title","content":"Ready","width":100,"height":24})"##
            .into(),
    );
    let cmd = match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, cmd) => cmd,
        other => panic!("expected command, got {other:?}"),
    };

    let mut s = sample();
    let before = s.active_children().len();
    assert!(s.apply(cmd));
    assert_eq!(s.active_children().len(), before + 1);
    let inserted = s.active_children().last().expect("inserted root");
    assert_eq!(inserted.base().name.as_deref(), Some("Card"));
    assert_eq!(inserted.children().expect("nested children").len(), 1);
}

#[test]
fn batch_design_rejects_unknown_kind_in_any_item() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10},{"kind":"blob","name":"B","x":0,"y":0,"width":10,"height":10}]"#
            .into(),
    );
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!("a single bad entry must reject the whole batch"),
    }
}

#[test]
fn batch_design_rejects_negative_geometry() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "nodes_json".into(),
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":-1,"height":10}]"#.into(),
    );
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn batch_design_rejects_malformed_json() {
    let tool = batch_design_snapshot();
    for bad in [
        "not json",
        "{}",
        "[{}]",
        r#"[{"kind":"rect"}]"#,
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10"#,
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10},]"#,
    ] {
        let mut args = BTreeMap::new();
        args.insert("nodes_json".into(), bad.into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "{bad}")
            }
            _ => panic!("expected reject on {bad}"),
        }
    }
}

#[test]
fn batch_insert_command_adds_all_nodes() {
    let mut s = sample();
    let pre_root_len = s.active_children().len();
    assert!(s.apply(EditorCommand::BatchInsert {
        items: vec![
            BatchInsertItem {
                kind: "rect".into(),
                name: "A".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 20,
                fill_hex: None,
            },
            BatchInsertItem {
                kind: "ellipse".into(),
                name: "B".into(),
                x: 40,
                y: 50,
                width: 30,
                height: 30,
                fill_hex: Some("#00ff00".into()),
            },
        ],
    }));
    assert_eq!(s.active_children().len(), pre_root_len + 2);
}

#[test]
fn batch_insert_command_atomic_on_bad_descriptor() {
    let mut s = sample();
    let pre_root_len = s.active_children().len();
    assert!(!s.apply(EditorCommand::BatchInsert {
        items: vec![
            BatchInsertItem {
                kind: "rect".into(),
                name: "A".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
            },
            BatchInsertItem {
                kind: "blob".into(),
                name: "B".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
            },
        ],
    }));
    assert_eq!(
        s.active_children().len(),
        pre_root_len,
        "no partial insertion"
    );
}

#[test]
fn batch_insert_command_rejects_empty_items() {
    let mut s = sample();
    assert!(!s.apply(EditorCommand::BatchInsert { items: vec![] }));
}
