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
        ToolOutcome::OkJson(json) => {
            let v: Value = serde_json::from_str(&json).expect("selection json");
            // TS get-selection returns EXACTLY { selectedIds, activePageId,
            // nodes } with native values — no stringification, no Rust-only keys.
            assert_eq!(v["activePageId"], "0");
            assert_eq!(v["selectedIds"], serde_json::json!(["n10", "n11"]));
            assert!(
                v.get("selected_id").is_none(),
                "no Rust-only selected_id key: {v}"
            );
            assert!(v.get("kind").is_none(), "no Rust-only kind key: {v}");
            assert!(v.get("x").is_none(), "no Rust-only x key: {v}");

            let nodes = v["nodes"].as_array().expect("nodes array");
            assert_eq!(nodes.len(), 2);
            assert_eq!(nodes[0]["id"], "n10");
            assert_eq!(nodes[0]["children"], "...");
            assert_eq!(nodes[1]["id"], "n11");
        }
        other => panic!("expected OkJson selection payload, got {other:?}"),
    }
}
