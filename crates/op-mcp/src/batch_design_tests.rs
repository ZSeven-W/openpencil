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
        ToolOutcome::OkWithCommand(result, EditorCommand::BatchInsert { items, page_id }) => {
            assert_eq!(result.get("count"), Some(&"2".to_string()));
            assert!(page_id.is_none());
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
fn batch_design_nodes_json_accepts_outer_page_id() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert(
        "nodes_json".into(),
        r##"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":20}]"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::BatchInsert { items, page_id }) => {
            assert_eq!(items.len(), 1);
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected BatchInsert with page id, got {other:?}"),
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
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"2".to_string()));
            assert!(!parent_id.is_real());
            assert!(page_id.is_none());
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
fn batch_design_insert_operations_accept_outer_page_id() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert(
        "operations".into(),
        r##"root=I(null, {"type":"frame","name":"Page","width":320,"height":240})"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(nodes.len(), 1);
            assert!(!parent_id.is_real());
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected InsertSubtree with page id, got {other:?}"),
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
fn batch_design_direct_operation_accepts_outer_page_id() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert("operations".into(), r##"U("n11", {"x":80})"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::UpdateNode {
                node_id, page_id, ..
            },
        ) => {
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected UpdateNode with page id, got {other:?}"),
    }
}

#[test]
fn batch_design_direct_update_preserves_rich_ts_patch_fields() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("pageId".into(), "page-2".into());
    args.insert(
        "operations".into(),
        r##"U("n11", {"content":"Updated","fontSize":24})"##.into(),
    );

    let ToolOutcome::OkWithCommand(
        _,
        EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id,
        },
    ) = tool.call(&args)
    else {
        panic!("expected PatchNodeData command from rich U() patch");
    };
    let patch: serde_json::Value = serde_json::from_str(&patch_json).expect("patch json");
    assert_eq!(node_id.as_str(), "n11");
    assert_eq!(patch["content"], "Updated");
    assert_eq!(patch["fontSize"], 24);
    assert_eq!(page_id.as_deref(), Some("page-2"));
}

#[test]
fn batch_design_accepts_single_update_operation() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"U("n11", {"x":80,"y":90,"width":260,"height":32,"name":"Updated title","fill_hex":"#112233"})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::UpdateNode {
                node_id,
                x,
                y,
                width,
                height,
                name,
                fill_hex,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n11");
            assert_eq!(x, Some(80));
            assert_eq!(y, Some(90));
            assert_eq!(width, Some(260));
            assert_eq!(height, Some(32));
            assert_eq!(name.as_deref(), Some("Updated title"));
            assert_eq!(fill_hex.as_deref(), Some("#112233"));
            assert_eq!(page_id, None);
        }
        other => panic!("expected UpdateNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_delete_operation() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"D("n14")"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(result, EditorCommand::DeleteNode { node_id, page_id }) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n14");
            assert_eq!(page_id, None);
        }
        other => panic!("expected DeleteNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_move_operation_without_index() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"M("n14", null)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                page_id,
                index,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n14");
            assert!(!target_parent.is_real());
            assert!(page_id.is_none());
            assert!(index.is_none());
        }
        other => panic!("expected MoveNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_copy_operation_with_overrides() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"C("n12", "n10", {"name":"Copied","x":24,"id":"ignored"})"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                overrides_json,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert_eq!(target_parent.as_str(), "n10");
            assert!(page_id.is_none());

            let overrides: serde_json::Value =
                serde_json::from_str(overrides_json.as_deref().expect("overrides")).unwrap();
            assert_eq!(
                overrides.get("name").and_then(|v| v.as_str()),
                Some("Copied")
            );
            assert_eq!(overrides.get("x").and_then(|v| v.as_i64()), Some(24));
            assert_eq!(
                overrides.get("id").and_then(|v| v.as_str()),
                Some("ignored")
            );
        }
        other => panic!("expected CopyNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_copy_operation() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"copied=C("n12", null)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::CopyNode {
                node_id,
                target_parent,
                overrides_json,
                ..
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert!(!target_parent.is_real());
            assert!(overrides_json.is_none());
        }
        other => panic!("expected bound CopyNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_replace_operation() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"R("n12", {"type":"rectangle","name":"Replacement","x":5,"y":6,"width":70,"height":80,"fill":"#abcdef"})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
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
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert_eq!(kind, "rect");
            assert_eq!(name, "Replacement");
            assert_eq!(x, 5);
            assert_eq!(y, 6);
            assert_eq!(width, 70);
            assert_eq!(height, 80);
            assert_eq!(fill_hex.as_deref(), Some("#abcdef"));
            assert!(!drop_children);
            assert!(page_id.is_none());
        }
        other => panic!("expected ReplaceNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_replace_operation() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"replacement=R("n12", {"type":"text","content":"Renamed","width":120,"height":24})"##
            .into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::ReplaceNode {
                node_id,
                kind,
                name,
                width,
                height,
                ..
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n12");
            assert_eq!(kind, "text");
            assert_eq!(name, "Renamed");
            assert_eq!(width, 120);
            assert_eq!(height, 24);
        }
        other => panic!("expected bound ReplaceNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_single_image_operation_without_fetcher() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"G("n10", "search", "hero product photo")"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(parent_id.as_str(), "n10");
            assert!(page_id.is_none());
            assert_eq!(nodes.len(), 1);
            assert!(matches!(nodes[0], jian_ops_schema::node::PenNode::Image(_)));
            assert_eq!(nodes[0].base().name.as_deref(), Some("hero product photo"));
        }
        other => panic!("expected image InsertSubtree command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_image_operation_without_fetcher() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"hero=G(null, "generate", "dashboard background")"##.into(),
    );

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::InsertSubtree {
                nodes,
                parent_id,
                page_id,
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert!(!parent_id.is_real());
            assert!(page_id.is_none());
            assert_eq!(nodes.len(), 1);
            assert!(matches!(nodes[0], jian_ops_schema::node::PenNode::Image(_)));
            assert_eq!(
                nodes[0].base().name.as_deref(),
                Some("dashboard background")
            );
        }
        other => panic!("expected bound image InsertSubtree command, got {other:?}"),
    }
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
fn batch_design_accepts_single_move_operation_with_index() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"M("n14", "n10", 2)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::MoveNode {
                target_parent,
                index,
                ..
            },
        ) => {
            assert_eq!(target_parent.as_str(), "n10");
            assert_eq!(index, Some(2));
        }
        other => panic!("expected indexed MoveNode command, got {other:?}"),
    }
}

#[test]
fn batch_design_accepts_bound_single_move_operation() {
    let tool = batch_design_snapshot();
    let mut args = BTreeMap::new();
    args.insert("operations".into(), r##"moved=M("n14", "n10", 1)"##.into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            result,
            EditorCommand::MoveNode {
                node_id,
                target_parent,
                index,
                ..
            },
        ) => {
            assert_eq!(result.get("count"), Some(&"1".to_string()));
            assert_eq!(node_id.as_str(), "n14");
            assert_eq!(target_parent.as_str(), "n10");
            assert_eq!(index, Some(1));
        }
        other => panic!("expected bound MoveNode command, got {other:?}"),
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
        page_id: None,
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
        page_id: None,
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
    assert!(!s.apply(EditorCommand::BatchInsert {
        items: vec![],
        page_id: None,
    }));
}
