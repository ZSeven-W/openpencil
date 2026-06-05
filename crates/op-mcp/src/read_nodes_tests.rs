//! `read_nodes` parity tests.

use std::collections::BTreeMap;

use jian_ops_schema::variable::{VariableKind, VariableScalar};

use super::test_fixtures::{add_theme_axis, add_variable, sample};
use super::{read_nodes_snapshot, McpTool, ToolOutcome};

#[test]
fn read_nodes_depth_zero_truncates_children_and_includes_variables() {
    let mut state = sample();
    add_variable(
        &mut state,
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff0000".into()),
    );
    add_theme_axis(&mut state, "Mode", &["Light", "Dark"]);

    let tool = read_nodes_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("nodeIds".into(), r#"["n10"]"#.into());
    args.insert("depth".into(), "0".into());
    args.insert("includeVariables".into(), "true".into());

    match tool.call(&args) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("read_nodes json");
            // TS shape: { nodes, variables?, themes? }, native, no `count`.
            assert!(v.get("count").is_none(), "no Rust-only count key: {v}");
            assert_eq!(v["nodes"][0]["id"], "n10");
            assert_eq!(v["nodes"][0]["children"], "...");
            assert_eq!(v["variables"]["brand"]["value"], "#ff0000");
            assert_eq!(v["themes"]["Mode"][1], "Dark");
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn read_nodes_absent_themes_match_ts_empty_array_not_object() {
    // A doc with no theme axes: TS handleReadNodes returns themes = doc.themes
    // ?? [] (empty ARRAY) and variables = doc.variables ?? {} (empty OBJECT).
    let state = sample();
    let tool = read_nodes_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert("includeVariables".into(), "true".into());

    match tool.call(&args) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("read_nodes json");
            assert_eq!(
                v["themes"],
                serde_json::json!([]),
                "absent themes must serialize as [] (TS doc.themes ?? []), not {{}}: {v}"
            );
            assert!(
                v["variables"].is_object(),
                "absent variables must be an object (TS doc.variables ?? {{}}): {v}"
            );
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn read_nodes_without_ids_returns_top_level_children() {
    let state = sample();
    let tool = read_nodes_snapshot(&state);
    let args = BTreeMap::new();

    match tool.call(&args) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("read_nodes json");
            assert!(v.get("count").is_none(), "no Rust-only count key: {v}");
            assert_eq!(v["nodes"][0]["id"], "n10");
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}
