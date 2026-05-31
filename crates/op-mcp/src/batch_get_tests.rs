//! TS-compatible `batch_get` parity tests.

use std::collections::BTreeMap;

use super::test_fixtures::sample;
use super::{batch_get_snapshot, McpTool, ToolOutcome};

#[test]
fn batch_get_without_filters_returns_top_level_children_at_default_depth() {
    let state = sample();
    let tool = batch_get_snapshot(&state);
    let args = BTreeMap::new();

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"1".to_string()));
            let nodes: serde_json::Value =
                serde_json::from_str(out.get("nodes").expect("nodes json")).expect("nodes json");
            assert_eq!(nodes[0]["id"], "n10");
            assert_eq!(nodes[0]["children"][0]["id"], "n11");
            assert_eq!(nodes[0]["children"][1]["id"], "n12");
            assert_eq!(nodes[0]["children"][1]["children"], "...");
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn batch_get_filters_by_name_pattern_and_parent_id() {
    let state = sample();
    let tool = batch_get_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("parentId".into(), "n12".into());
    args.insert("patterns".into(), r#"[{"name":"Click"}]"#.into());
    args.insert("readDepth".into(), "0".into());

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"1".to_string()));
            let nodes: serde_json::Value =
                serde_json::from_str(out.get("nodes").expect("nodes json")).expect("nodes json");
            assert_eq!(nodes[0]["id"], "n14");
            assert_eq!(nodes[0]["name"], "Click me");
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn batch_get_reads_specific_node_ids() {
    let state = sample();
    let tool = batch_get_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("nodeIds".into(), r#"["n11","n13"]"#.into());
    args.insert("readDepth".into(), "0".into());

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"2".to_string()));
            let nodes: serde_json::Value =
                serde_json::from_str(out.get("nodes").expect("nodes json")).expect("nodes json");
            assert_eq!(nodes[0]["id"], "n11");
            assert_eq!(nodes[1]["id"], "n13");
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}
