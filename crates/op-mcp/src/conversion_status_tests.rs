use std::collections::BTreeMap;

use crate::{conversion_status_snapshot, McpTool, ToolOutcome};
use jian_ops_schema::conversion::{ConversionEntry, ConversionKind};
use op_editor_core::conversion::upsert_conversion_entry;

#[test]
fn conversion_status_reports_orphaned() {
    let mut state = crate::test_fixtures::state_with(vec![]);
    upsert_conversion_entry(
        &mut state.doc,
        ConversionEntry {
            kind: ConversionKind::Screen,
            key: "route:/".into(),
            source_path: None,
            source_hash: None,
            node_id: Some("n999".into()),
            node_ids: None,
        },
    );
    let tool = conversion_status_snapshot(&state);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["total"], 1);
            assert_eq!(value["entries"][0]["status"], "orphaned");
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn conversion_status_kind_filter() {
    let mut state = crate::test_fixtures::state_with(vec![]);
    upsert_conversion_entry(
        &mut state.doc,
        ConversionEntry {
            kind: ConversionKind::Token,
            key: "tokens:theme.css".into(),
            source_path: Some("src/theme.css".into()),
            source_hash: Some("h1".into()),
            node_id: None,
            node_ids: None,
        },
    );
    upsert_conversion_entry(
        &mut state.doc,
        ConversionEntry {
            kind: ConversionKind::Screen,
            key: "route:/".into(),
            source_path: None,
            source_hash: None,
            node_id: Some("n999".into()),
            node_ids: None,
        },
    );
    let tool = conversion_status_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("kind".into(), "token".into());
    match tool.call(&args) {
        ToolOutcome::OkJson(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["total"], 1);
            assert_eq!(value["entries"][0]["kind"], "token");
            assert_eq!(value["entries"][0]["status"], "ok");
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn conversion_status_empty_ledger() {
    let state = crate::test_fixtures::state_with(vec![]);
    let tool = conversion_status_snapshot(&state);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(value["total"], 0);
            assert_eq!(value["entries"].as_array().unwrap().len(), 0);
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}
