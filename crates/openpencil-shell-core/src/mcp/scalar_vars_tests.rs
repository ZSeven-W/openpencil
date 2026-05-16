//! Tests for `mcp::scalar_vars::{SetVariableNumber, ...}` + the
//! `VariableTable::set_scalar` apply branch.

use super::scalar_vars::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome, VariableScalarPayload};
use std::collections::BTreeMap;

fn doc_with(kind: crate::document::VariableKind, name: &str) -> crate::document::Document {
    use crate::document::{Document, Variable, VariableScalar, VariableValue};
    let mut doc = Document::empty();
    let default_scalar = match kind {
        crate::document::VariableKind::Number => VariableScalar::Num(0.0),
        crate::document::VariableKind::String => VariableScalar::Str(String::new()),
        crate::document::VariableKind::Boolean => VariableScalar::Bool(false),
        crate::document::VariableKind::Color => VariableScalar::Str("#000000".into()),
    };
    doc.var_table.variables.push(Variable {
        name: name.into(),
        kind,
        value: VariableValue::Scalar(default_scalar),
    });
    doc
}

#[test]
fn set_variable_number_rejects_non_numeric() {
    let doc = doc_with(crate::document::VariableKind::Number, "size");
    let tool = set_variable_number_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "size".into());
    args.insert("value".into(), "abc".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn set_variable_number_emits_command() {
    let doc = doc_with(crate::document::VariableKind::Number, "size");
    let tool = set_variable_number_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "size".into());
    args.insert("value".into(), "12.5".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::SetVariableScalar { name, scalar }) => {
            assert_eq!(name, "size");
            assert_eq!(scalar, VariableScalarPayload::Number(12.5));
        }
        other => panic!("expected SetVariableScalar Number, got {other:?}"),
    }
}

#[test]
fn set_variable_string_emits_command() {
    let doc = doc_with(crate::document::VariableKind::String, "title");
    let tool = set_variable_string_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "title".into());
    args.insert("value".into(), "Hello".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::SetVariableScalar { name, scalar }) => {
            assert_eq!(name, "title");
            assert_eq!(scalar, VariableScalarPayload::String("Hello".into()));
        }
        other => panic!("expected SetVariableScalar String, got {other:?}"),
    }
}

#[test]
fn set_variable_boolean_emits_command_and_rejects_bad_strings() {
    let doc = doc_with(crate::document::VariableKind::Boolean, "show");
    let tool = set_variable_boolean_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "show".into());
    args.insert("value".into(), "true".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::SetVariableScalar { name, scalar }) => {
            assert_eq!(name, "show");
            assert_eq!(scalar, VariableScalarPayload::Boolean(true));
        }
        _ => panic!(),
    }
    args.insert("value".into(), "yes".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn apply_mcp_command_rejects_color_through_scalar_path() {
    // Codex stop-gate: set_variable_string snapshot wouldn't
    // include a Color variable name, but a misrouted call (or a
    // forged McpCommand) MUST be rejected at apply time too.
    // The Color variable must keep its original value.
    use crate::document::{VariableKind, VariableScalar, VariableValue};
    let mut doc = doc_with(VariableKind::Color, "primary");
    // Stamp a known-good hex so we can detect mutation.
    if let Some(v) = doc.var_table.variables.iter_mut().find(|v| v.name == "primary") {
        v.value = VariableValue::Scalar(VariableScalar::Str("#abcdef".into()));
    }
    let cmd = McpCommand::SetVariableScalar {
        name: "primary".into(),
        scalar: VariableScalarPayload::String("garbage".into()),
    };
    assert!(!doc.var_table.apply_mcp_command(&cmd), "Color must reject scalar path");
    let v = doc.var_table.variables.iter().find(|v| v.name == "primary").unwrap();
    match &v.value {
        VariableValue::Scalar(VariableScalar::Str(s)) => {
            assert_eq!(s, "#abcdef", "Color value must be unchanged");
        }
        other => panic!("expected unchanged hex, got {other:?}"),
    }
}

#[test]
fn apply_mcp_command_routes_to_set_scalar() {
    use crate::document::{Document, VariableKind, VariableScalar, VariableValue};
    let mut doc = doc_with(VariableKind::Number, "size");
    let cmd = McpCommand::SetVariableScalar {
        name: "size".into(),
        scalar: VariableScalarPayload::Number(42.0),
    };
    assert!(doc.var_table.apply_mcp_command(&cmd));
    let v = doc.var_table.variables.iter().find(|v| v.name == "size").unwrap();
    match &v.value {
        VariableValue::Scalar(VariableScalar::Num(n)) => assert_eq!(*n, 42.0),
        other => panic!("expected Num scalar, got {other:?}"),
    }
    // Wrong-kind scalar rejected.
    let bad = McpCommand::SetVariableScalar {
        name: "size".into(),
        scalar: VariableScalarPayload::String("oops".into()),
    };
    assert!(!doc.var_table.apply_mcp_command(&bad));
    let _ = Document::sample(); // suppress unused-import warning
}

#[test]
fn create_variable_appends_and_rejects_duplicate() {
    use crate::document::{Document, VariableKind, VariableScalar};
    let mut doc = Document::empty();
    assert!(doc.var_table.create_variable(
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#ff8800".into()),
    ));
    assert_eq!(doc.var_table.variables.len(), 1);
    // Duplicate name rejected.
    assert!(!doc.var_table.create_variable(
        "brand",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    // Bad hex rejected.
    assert!(!doc.var_table.create_variable(
        "bad",
        VariableKind::Color,
        VariableScalar::Str("not-hex".into()),
    ));
    // Kind/scalar mismatch rejected.
    assert!(!doc.var_table.create_variable(
        "mismatch",
        VariableKind::Number,
        VariableScalar::Str("12".into()),
    ));
    assert_eq!(doc.var_table.variables.len(), 1, "only the valid create stuck");
}

#[test]
fn delete_variable_drops_dangling_refs() {
    use crate::document::{Document, NodeId, VariableKind, VariableScalar};
    let mut doc = Document::empty();
    doc.var_table.create_variable(
        "accent",
        VariableKind::Color,
        VariableScalar::Str("#112233".into()),
    );
    doc.var_table.fill_refs.insert(NodeId::new("n7"), "accent".into());
    doc.var_table.stroke_refs.insert(NodeId::new("n9"), "accent".into());
    assert!(doc.var_table.delete_variable("accent"));
    assert!(doc.var_table.variables.is_empty());
    assert!(doc.var_table.fill_refs.is_empty(), "dangling fill_ref dropped");
    assert!(doc.var_table.stroke_refs.is_empty(), "dangling stroke_ref dropped");
    // Unknown name rejected.
    assert!(!doc.var_table.delete_variable("nope"));
}

#[test]
fn rename_variable_rewrites_refs_and_guards_collisions() {
    use crate::document::{Document, NodeId, VariableKind, VariableScalar};
    let mut doc = Document::empty();
    doc.var_table.create_variable(
        "old",
        VariableKind::Number,
        VariableScalar::Num(4.0),
    );
    doc.var_table.create_variable(
        "taken",
        VariableKind::Number,
        VariableScalar::Num(8.0),
    );
    doc.var_table.fill_refs.insert(NodeId::new("n3"), "old".into());
    // Collision with an existing different variable rejected.
    assert!(!doc.var_table.rename_variable("old", "taken"));
    // Empty new name rejected.
    assert!(!doc.var_table.rename_variable("old", "   "));
    // Unknown old name rejected.
    assert!(!doc.var_table.rename_variable("ghost", "fresh"));
    // Happy path: rename + ref rewrite.
    assert!(doc.var_table.rename_variable("old", "renamed"));
    assert!(doc.var_table.variables.iter().any(|v| v.name == "renamed"));
    assert!(!doc.var_table.variables.iter().any(|v| v.name == "old"));
    assert_eq!(
        doc.var_table.fill_refs.get(&NodeId::new("n3")),
        Some(&"renamed".to_string()),
        "fill_ref follows the rename"
    );
}

#[test]
fn create_variable_mcp_command_routes_through_var_table() {
    use crate::document::{Document, VariableKind, VariableValue, VariableScalar};
    let mut doc = Document::empty();
    let cmd = McpCommand::CreateVariable {
        name: "spacing".into(),
        kind: "number".into(),
        default_value: "16".into(),
    };
    assert!(doc.apply_mcp_command(&cmd));
    let v = doc
        .var_table
        .variables
        .iter()
        .find(|v| v.name == "spacing")
        .expect("variable created");
    assert!(matches!(v.kind, VariableKind::Number));
    match &v.value {
        VariableValue::Scalar(VariableScalar::Num(n)) => assert_eq!(*n, 16.0),
        other => panic!("expected Num(16), got {other:?}"),
    }
    // Bad kind string rejected at apply.
    let bad = McpCommand::CreateVariable {
        name: "x".into(),
        kind: "rainbow".into(),
        default_value: "1".into(),
    };
    assert!(!doc.apply_mcp_command(&bad));
}
