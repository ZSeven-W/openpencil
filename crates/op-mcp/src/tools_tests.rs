//! Tests for `mcp::tools`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`.

use super::test_fixtures::{add_theme_axis, add_variable, frame, state_with};
use super::tools::*;
use super::{McpTool, ToolOutcome};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::{EditorCommand, EditorState, NodeId};
use std::collections::BTreeMap;

fn state_with_variables() -> EditorState {
    let mut s = state_with(vec![]);
    add_variable(
        &mut s,
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#ff0000".into()),
    );
    add_variable(
        &mut s,
        "spacing",
        VariableKind::Number,
        VariableScalar::Num(16.0),
    );
    add_variable(
        &mut s,
        "compact",
        VariableKind::Boolean,
        VariableScalar::Bool(true),
    );
    s
}

#[test]
fn list_variables_reports_count_and_records() {
    let s = state_with_variables();
    let tool = list_variables_snapshot(&s);
    assert_eq!(tool.variables.len(), 3);
    // BTreeMap iteration is sorted by name: color-1, compact, spacing.
    let by_name = |n: &str| tool.variables.iter().find(|v| v.name == n).unwrap();
    assert_eq!(by_name("color-1").kind, "color");
    assert_eq!(by_name("color-1").value, "#ff0000");
    assert_eq!(by_name("spacing").kind, "number");
    assert_eq!(by_name("spacing").value, "16");
    assert_eq!(by_name("compact").kind, "boolean");
    assert_eq!(by_name("compact").value, "true");
}

#[test]
fn list_variables_emits_json_array() {
    let s = state_with_variables();
    let tool = list_variables_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("variables json");
            assert!(v.get("count").is_none(), "TS shape has no count: {v}");
            let vars = v["variables"].as_array().expect("variables array");
            assert_eq!(vars.len(), 3);
            let find = |n: &str| vars.iter().find(|x| x["name"] == n).expect("var");
            assert_eq!(find("color-1")["kind"], "color");
            assert_eq!(find("color-1")["value"], "#ff0000");
            assert_eq!(find("spacing")["value"], "16");
            assert_eq!(find("compact")["value"], "true");
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn get_active_theme_reports_active_axes_and_options() {
    let mut s = state_with(vec![]);
    add_theme_axis(&mut s, "mode", &["light", "dark", "sepia"]);
    add_theme_axis(&mut s, "density", &["compact", "comfortable"]);
    s.set_active_axis_value("mode", "dark");
    let tool = get_active_theme_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("theme json");
            assert_eq!(v["active"]["mode"], "dark");
            assert_eq!(
                v["themes"]["mode"],
                serde_json::json!(["light", "dark", "sepia"])
            );
            assert_eq!(
                v["themes"]["density"],
                serde_json::json!(["compact", "comfortable"])
            );
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn get_active_theme_empty_document_is_zero() {
    let s = state_with(vec![]);
    let tool = get_active_theme_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("theme json");
            assert!(v["active"].as_object().expect("active obj").is_empty());
            assert!(v["themes"].as_object().expect("themes obj").is_empty());
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn list_variables_empty_document_returns_zero_count() {
    let s = state_with(vec![]);
    let tool = list_variables_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            let v: serde_json::Value = serde_json::from_str(&json).expect("variables json");
            assert!(v["variables"]
                .as_array()
                .expect("variables array")
                .is_empty());
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}

#[test]
fn list_components_reports_registered_components() {
    let mut s = state_with(vec![frame(
        "n1",
        "Card Root",
        0.0,
        0.0,
        100.0,
        80.0,
        Vec::new(),
    )]);
    assert!(s.apply(EditorCommand::CreateComponent {
        node_id: NodeId::new("n1"),
        name: "Card".into(),
    }));
    let tool = list_components_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"1".to_string()));
            assert_eq!(out.get("components"), Some(&"Card|n1".to_string()));
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn get_component_reports_kind_and_subtree_size() {
    let mut s = state_with(vec![frame(
        "n1",
        "Card Root",
        0.0,
        0.0,
        100.0,
        80.0,
        Vec::new(),
    )]);
    assert!(s.apply(EditorCommand::CreateComponent {
        node_id: NodeId::new("n1"),
        name: "Card".into(),
    }));
    let tool = get_component_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("component_id".into(), "n1".into());
    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("name"), Some(&"Card".to_string()));
            assert_eq!(out.get("kind"), Some(&"frame".to_string()));
            assert_eq!(out.get("leaf_count"), Some(&"1".to_string()));
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn escape_record_field_round_trips_pipe_semicolon_backslash() {
    for raw in &[
        "plain",
        "a|b",
        "a;b",
        "a\\b",
        "a|b;c\\d",
        "\\",
        ";;|||",
        "",
        "color/primary",
        "label with space",
    ] {
        let escaped = super::tools::escape_record_field(raw);
        let back = unescape_record_field(&escaped);
        assert_eq!(&back, raw, "round-trip failed for {raw:?}");
    }
}

#[test]
fn list_variables_json_preserves_special_chars_in_value() {
    let mut s = state_with(vec![]);
    add_variable(
        &mut s,
        "msg",
        VariableKind::String,
        VariableScalar::Str("a|b;c\\d".into()),
    );
    let tool = list_variables_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::OkJson(json) => {
            // JSON escaping handles `|` / `;` / `\` natively — the value
            // round-trips exactly with no custom delimiter encoding.
            let v: serde_json::Value = serde_json::from_str(&json).expect("variables json");
            let var = &v["variables"][0];
            assert_eq!(var["name"], "msg");
            assert_eq!(var["kind"], "string");
            assert_eq!(var["value"], "a|b;c\\d");
        }
        other => panic!("expected OkJson, got {other:?}"),
    }
}
