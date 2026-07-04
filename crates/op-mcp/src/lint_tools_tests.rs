use std::collections::BTreeMap;

use crate::{lint_document_snapshot, McpTool, ToolErrorCode, ToolOutcome};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::page::PenPage;
use serde_json::json;

fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("fixture must deserialize as PenNode")
}

fn issue_node_ids(value: &serde_json::Value) -> Vec<String> {
    value["issues"]
        .as_array()
        .expect("issues must be an array")
        .iter()
        .map(|issue| issue["nodeId"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn lint_document_returns_issue_array() {
    let root = node(json!({
        "type": "frame",
        "id": "root",
        "children": [
            {"type": "path", "id": "empty"}
        ]
    }));
    let state = crate::test_fixtures::state_with(vec![root]);
    let tool = lint_document_snapshot(&state);

    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["count"], 1);
            assert_eq!(issue_node_ids(&value), vec!["empty"]);
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn lint_document_filters_to_node_subtree() {
    let root = node(json!({
        "type": "frame",
        "id": "root",
        "children": [
            {
                "type": "frame",
                "id": "left",
                "children": [{"type": "path", "id": "empty-left"}]
            },
            {
                "type": "frame",
                "id": "right",
                "children": [{"type": "path", "id": "empty-right"}]
            }
        ]
    }));
    let state = crate::test_fixtures::state_with(vec![root]);
    let tool = lint_document_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("nodeId".into(), "left".into());

    match tool.call(&args) {
        ToolOutcome::OkJson(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["count"], 1);
            assert_eq!(issue_node_ids(&value), vec!["empty-left"]);
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn lint_document_filters_to_component_master_on_non_active_page() {
    let master = node(json!({
        "type": "frame",
        "id": "button-master",
        "children": [
            {"type": "path", "id": "empty-path"}
        ]
    }));
    let mut state = crate::test_fixtures::state_with(vec![]);
    state.doc.pages = Some(vec![
        PenPage {
            id: "p1".into(),
            name: "Page 1".into(),
            children: Vec::new(),
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "components".into(),
            name: "Components".into(),
            children: vec![master],
            state: None,
            lifecycle: None,
        },
    ]);
    state.ui.active_page_index = 0;
    let tool = lint_document_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("nodeId".into(), "button-master".into());

    match tool.call(&args) {
        ToolOutcome::OkJson(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["count"], 1);
            assert_eq!(issue_node_ids(&value), vec!["empty-path"]);
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn lint_document_rejects_unknown_node_filter() {
    let state = crate::test_fixtures::state_with(vec![]);
    let tool = lint_document_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("nodeId".into(), "missing".into());

    match tool.call(&args) {
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, msg) => {
            assert!(msg.contains("nodeId not found"));
        }
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}
