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
