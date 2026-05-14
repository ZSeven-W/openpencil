//! Tests for `mcp::batch_design::BatchDesign` + the
//! `Document::apply_mcp_command(BatchInsert)` branch. Sibling
//! file mirroring `copy_node_tests.rs` / `replace_node_tests.rs`.

use super::batch_design::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
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
        ToolOutcome::OkWithCommand(result, McpCommand::BatchInsert { items }) => {
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
        r#"[{"kind":"rect"}]"#, // missing required keys
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10"#, // unterminated
        r#"[{"kind":"rect","name":"A","x":0,"y":0,"width":10,"height":10},]"#, // trailing comma
    ] {
        let mut args = BTreeMap::new();
        args.insert("nodes_json".into(), bad.into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument, "{bad}"),
            _ => panic!("expected reject on {bad}"),
        }
    }
}

#[test]
fn apply_mcp_command_batch_insert_adds_all_nodes_with_fresh_ids() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_max = doc.max_node_id();
    let pre_root_len = doc.pages[0].children.len();
    let cmd = McpCommand::BatchInsert {
        items: vec![
            crate::mcp::BatchInsertItem {
                kind: "rect".into(),
                name: "A".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 20,
                fill_hex: None,
            },
            crate::mcp::BatchInsertItem {
                kind: "ellipse".into(),
                name: "B".into(),
                x: 40,
                y: 50,
                width: 30,
                height: 30,
                fill_hex: Some("#00ff00".into()),
            },
        ],
    };
    assert!(doc.apply_mcp_command(&cmd));
    assert_eq!(doc.pages[0].children.len(), pre_root_len + 2);
    // Every new node has a fresh id past pre_max, no duplicates.
    let new_ids: Vec<u64> = doc.pages[0].children.iter()
        .skip(pre_root_len)
        .map(|n| n.id.raw())
        .collect();
    assert_eq!(new_ids.len(), 2);
    assert!(new_ids[0] > pre_max);
    assert!(new_ids[1] > pre_max);
    assert_ne!(new_ids[0], new_ids[1]);
}

#[test]
fn apply_mcp_command_batch_insert_atomic_on_bad_descriptor() {
    // Bad descriptor (unknown kind) must reject the whole batch.
    // The command would only land here if the tool somehow let it
    // through — defense in depth at the applier.
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_root_len = doc.pages[0].children.len();
    let cmd = McpCommand::BatchInsert {
        items: vec![
            crate::mcp::BatchInsertItem {
                kind: "rect".into(),
                name: "A".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
            },
            crate::mcp::BatchInsertItem {
                kind: "blob".into(),
                name: "B".into(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
            },
        ],
    };
    assert!(!doc.apply_mcp_command(&cmd));
    assert_eq!(
        doc.pages[0].children.len(),
        pre_root_len,
        "no partial insertion on bad descriptor"
    );
}

#[test]
fn apply_mcp_command_batch_insert_rejects_empty_items() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let cmd = McpCommand::BatchInsert { items: vec![] };
    assert!(!doc.apply_mcp_command(&cmd));
}
