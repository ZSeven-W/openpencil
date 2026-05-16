//! Tests for `mcp::tools`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`.

use super::test_fixtures::{add_theme_axis, add_variable, state_with};
use super::tools::*;
use super::{McpTool, ToolOutcome};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::EditorState;
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
fn list_variables_encodes_for_wire() {
    let s = state_with_variables();
    let tool = list_variables_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"3".to_string()));
            let encoded = out.get("variables").expect("variables field");
            assert!(encoded.contains("color-1|color|#ff0000"));
            assert!(encoded.contains("spacing|number|16"));
            assert!(encoded.contains("compact|boolean|true"));
            assert_eq!(encoded.matches(';').count(), 2);
        }
        other => panic!("expected Ok, got {other:?}"),
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
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("axis_count"), Some(&"2".to_string()));
            assert_eq!(out.get("axes"), Some(&"mode|dark".to_string()));
            let opts = out.get("options").expect("options field");
            assert!(opts.contains("density|compact,comfortable"));
            assert!(opts.contains("mode|light,dark,sepia"));
            assert_eq!(opts.matches(';').count(), 1);
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn get_active_theme_empty_document_is_zero() {
    let s = state_with(vec![]);
    let tool = get_active_theme_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("axis_count"), Some(&"0".to_string()));
            assert_eq!(out.get("axes"), Some(&String::new()));
            assert_eq!(out.get("options"), Some(&String::new()));
        }
        _ => panic!("expected Ok"),
    }
}

#[test]
fn list_variables_empty_document_returns_zero_count() {
    let s = state_with(vec![]);
    let tool = list_variables_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"0".to_string()));
            assert_eq!(out.get("variables"), Some(&String::new()));
        }
        _ => panic!("expected Ok"),
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
fn list_variables_encodes_string_value_with_special_chars() {
    let mut s = state_with(vec![]);
    add_variable(
        &mut s,
        "msg",
        VariableKind::String,
        VariableScalar::Str("a|b;c\\d".into()),
    );
    let tool = list_variables_snapshot(&s);
    let out = match tool.call(&BTreeMap::new()) {
        ToolOutcome::Ok(o) => o,
        other => panic!("expected Ok, got {other:?}"),
    };
    assert_eq!(out.get("count"), Some(&"1".to_string()));
    let encoded = out.get("variables").expect("variables field");
    // Pipe inside the value is escaped — splitting on unescaped `|`
    // still yields exactly 3 fields (name | kind | value).
    let mut fields: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut chars = encoded.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&n) = chars.peek() {
                if matches!(n, '\\' | ';' | '|') {
                    cur.push(chars.next().unwrap());
                    continue;
                }
            }
            cur.push(c);
        } else if c == '|' {
            fields.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    fields.push(cur);
    assert_eq!(fields.len(), 3, "expected name|kind|value, got {fields:?}");
    assert_eq!(fields[0], "msg");
    assert_eq!(fields[1], "string");
    assert_eq!(fields[2], "a|b;c\\d");
}
