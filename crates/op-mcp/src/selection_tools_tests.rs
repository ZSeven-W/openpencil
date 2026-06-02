use std::collections::BTreeMap;

use serde_json::Value;

use crate::test_fixtures::sample;
use crate::{selection_snapshot, McpTool, ToolOutcome};

#[test]
fn get_selection_returns_ts_selected_ids_active_page_and_nodes() {
    let mut state = sample();
    state.clear_selection();
    state.toggle_selection(op_editor_core::NodeId::new("n10"));
    state.toggle_selection(op_editor_core::NodeId::new("n11"));

    let tool = selection_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("readDepth".to_string(), "0".to_string());

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("selected_id"), Some(&"n11".to_string()));
            assert_eq!(out.get("activePageId"), Some(&"0".to_string()));
            assert_eq!(
                out.get("selectedIds"),
                Some(&r#"["n10","n11"]"#.to_string())
            );

            let nodes: Value =
                serde_json::from_str(out.get("nodes").expect("nodes json")).expect("nodes");
            let nodes = nodes.as_array().expect("nodes array");
            assert_eq!(nodes.len(), 2);
            assert_eq!(nodes[0]["id"], "n10");
            assert_eq!(nodes[0]["children"], "...");
            assert_eq!(nodes[1]["id"], "n11");
        }
        other => panic!("expected Ok selection payload, got {other:?}"),
    }
}
