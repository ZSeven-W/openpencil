//! TS-compatible `batch_get` parity tests.

use std::collections::BTreeMap;

use super::batch_get::resolve_refs_in_node;
use super::test_fixtures::sample;
use super::{batch_get_snapshot, McpTool, ToolOutcome};
use jian_ops_schema::variable::VariableScalar;

/// Parse the OkJson payload of a batch_get call and assert the TS shape:
/// `{ nodes: [...] }` (nodes is a JSON array; no Rust-only `count`).
fn batch_get_payload(outcome: ToolOutcome) -> serde_json::Value {
    match outcome {
        ToolOutcome::OkJson(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).expect("nodes json");
            assert!(
                value["nodes"].is_array(),
                "nodes must be a JSON array: {value}"
            );
            assert!(
                value.get("count").is_none(),
                "TS batch_get has no `count`: {value}"
            );
            value
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn batch_get_without_filters_returns_top_level_children_at_default_depth() {
    let state = sample();
    let tool = batch_get_snapshot(&state);
    let value = batch_get_payload(tool.call(&BTreeMap::new()));
    let nodes = &value["nodes"];
    assert_eq!(nodes[0]["id"], "n10");
    assert_eq!(nodes[0]["children"][0]["id"], "n11");
    assert_eq!(nodes[0]["children"][1]["id"], "n12");
    assert_eq!(nodes[0]["children"][1]["children"], "...");
}

#[test]
fn batch_get_filters_by_name_pattern_and_parent_id() {
    let state = sample();
    let tool = batch_get_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("parentId".into(), "n12".into());
    args.insert("patterns".into(), r#"[{"name":"Click"}]"#.into());
    args.insert("readDepth".into(), "0".into());

    let value = batch_get_payload(tool.call(&args));
    assert_eq!(value["nodes"][0]["id"], "n14");
    assert_eq!(value["nodes"][0]["name"], "Click me");
}

#[test]
fn resolve_refs_resolves_fill_stroke_colors_and_numeric_slots() {
    // resolveRefs=true expands $variable refs like TS resolveNodeForCanvas:
    // fill/stroke colors (Str) + opacity/gap/padding (Num), recursively.
    let mut vars = std::collections::BTreeMap::new();
    vars.insert(
        "brand".to_string(),
        VariableScalar::Str("#ff0000".to_string()),
    );
    vars.insert("half".to_string(), VariableScalar::Num(0.5));
    let mut node = serde_json::json!({
        "id": "n1",
        "type": "rectangle",
        "fill": [
            { "type": "solid", "color": "$brand" },
            { "type": "linear_gradient", "stops": [{ "color": "$brand", "position": 0 }] }
        ],
        "stroke": { "fill": [{ "type": "solid", "color": "$brand" }], "thickness": 1 },
        "effects": [{ "type": "shadow", "color": "$brand", "blur": 4 }],
        "opacity": "$half",
        "gap": "$half",
        "cornerRadius": "$half",
        "children": [
            { "id": "n2", "type": "text", "fill": [{ "type": "solid", "color": "$brand" }] }
        ]
    });
    resolve_refs_in_node(&mut node, &vars);
    assert_eq!(node["fill"][0]["color"], "#ff0000");
    // Gradient stop colors resolve too.
    assert_eq!(node["fill"][1]["stops"][0]["color"], "#ff0000");
    assert_eq!(node["stroke"]["fill"][0]["color"], "#ff0000");
    // Shadow/effect colors resolve.
    assert_eq!(node["effects"][0]["color"], "#ff0000");
    assert_eq!(node["opacity"], 0.5);
    assert_eq!(node["gap"], 0.5);
    assert_eq!(node["cornerRadius"], 0.5);
    // Recurses into children.
    assert_eq!(node["children"][0]["fill"][0]["color"], "#ff0000");
    // A non-ref / unknown ref is left untouched.
    let mut literal = serde_json::json!({ "fill": [{ "type": "solid", "color": "#00ff00" }] });
    resolve_refs_in_node(&mut literal, &vars);
    assert_eq!(literal["fill"][0]["color"], "#00ff00");
}

#[test]
fn batch_get_reads_specific_node_ids() {
    let state = sample();
    let tool = batch_get_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("nodeIds".into(), r#"["n11","n13"]"#.into());
    args.insert("readDepth".into(), "0".into());

    let value = batch_get_payload(tool.call(&args));
    assert_eq!(value["nodes"][0]["id"], "n11");
    assert_eq!(value["nodes"][1]["id"], "n13");
}
