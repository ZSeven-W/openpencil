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
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"1".to_string()));
            let nodes: serde_json::Value =
                serde_json::from_str(out.get("nodes").expect("nodes json")).expect("nodes json");
            assert_eq!(nodes[0]["id"], "n10");
            assert_eq!(nodes[0]["children"], "...");
            let variables: serde_json::Value =
                serde_json::from_str(out.get("variables").expect("variables json"))
                    .expect("variables json");
            assert_eq!(variables["brand"]["value"], "#ff0000");
            let themes: serde_json::Value =
                serde_json::from_str(out.get("themes").expect("themes json")).expect("themes json");
            assert_eq!(themes["Mode"][1], "Dark");
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn read_nodes_without_ids_returns_top_level_children() {
    let state = sample();
    let tool = read_nodes_snapshot(&state);
    let args = BTreeMap::new();

    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"1".to_string()));
            let nodes: serde_json::Value =
                serde_json::from_str(out.get("nodes").expect("nodes json")).expect("nodes json");
            assert_eq!(nodes[0]["id"], "n10");
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}
