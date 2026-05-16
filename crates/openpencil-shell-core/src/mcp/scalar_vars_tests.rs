//! Tests for `mcp::scalar_vars`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. Tool-layer validation + `EditorCommand` emission,
//! plus a few end-to-end `EditorState::apply` checks; the apply-path
//! correctness is covered by `op-editor-core`'s `variables` /
//! `command_tests`.

use super::scalar_vars::*;
use super::test_fixtures::{add_variable, state_with};
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome, VariableScalarPayload};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::EditorState;
use std::collections::BTreeMap;

fn state_with_var(kind: VariableKind, name: &str) -> EditorState {
    let mut s = state_with(vec![]);
    let default = match kind {
        VariableKind::Number => VariableScalar::Num(0.0),
        VariableKind::String => VariableScalar::Str(String::new()),
        VariableKind::Boolean => VariableScalar::Bool(false),
        VariableKind::Color => VariableScalar::Str("#000000".into()),
    };
    add_variable(&mut s, name, kind, default);
    s
}

#[test]
fn set_variable_number_rejects_non_numeric() {
    let s = state_with_var(VariableKind::Number, "size");
    let tool = set_variable_number_snapshot(&s);
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
    let s = state_with_var(VariableKind::Number, "size");
    let tool = set_variable_number_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "size".into());
    args.insert("value".into(), "12.5".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::SetVariableScalar { name, scalar }) => {
            assert_eq!(name, "size");
            assert_eq!(scalar, VariableScalarPayload::Number(12.5));
        }
        other => panic!("expected SetVariableScalar Number, got {other:?}"),
    }
}

#[test]
fn set_variable_string_emits_command() {
    let s = state_with_var(VariableKind::String, "title");
    let tool = set_variable_string_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "title".into());
    args.insert("value".into(), "Hello".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::SetVariableScalar { name, scalar }) => {
            assert_eq!(name, "title");
            assert_eq!(scalar, VariableScalarPayload::String("Hello".into()));
        }
        other => panic!("expected SetVariableScalar String, got {other:?}"),
    }
}

#[test]
fn set_variable_boolean_emits_command_and_rejects_bad_strings() {
    let s = state_with_var(VariableKind::Boolean, "show");
    let tool = set_variable_boolean_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "show".into());
    args.insert("value".into(), "true".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::SetVariableScalar { name, scalar }) => {
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
fn set_variable_scalar_command_routes_to_correct_kind() {
    let mut s = state_with_var(VariableKind::Number, "size");
    assert!(s.apply(EditorCommand::SetVariableScalar {
        name: "size".into(),
        scalar: VariableScalarPayload::Number(42.0),
    }));
    match s.resolve_variable("size") {
        Some(VariableScalar::Num(n)) => assert_eq!(*n, 42.0),
        other => panic!("expected Num(42), got {other:?}"),
    }
    // Wrong-kind scalar rejected at apply time.
    assert!(!s.apply(EditorCommand::SetVariableScalar {
        name: "size".into(),
        scalar: VariableScalarPayload::String("oops".into()),
    }));
}

#[test]
fn set_variable_scalar_rejects_color_through_scalar_path() {
    let mut s = state_with_var(VariableKind::Color, "primary");
    assert!(!s.apply(EditorCommand::SetVariableScalar {
        name: "primary".into(),
        scalar: VariableScalarPayload::String("garbage".into()),
    }));
}

#[test]
fn create_variable_tool_rejects_duplicate() {
    let mut s = state_with_var(VariableKind::Color, "brand");
    let tool = create_variable_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    args.insert("kind".into(), "color".into());
    args.insert("default_value".into(), "#000000".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::ToolFailed),
        other => panic!("expected ToolFailed for duplicate, got {other:?}"),
    }
    // Apply also validates: duplicate rejects.
    assert!(!s.apply(EditorCommand::CreateVariable {
        name: "brand".into(),
        kind: "color".into(),
        default_value: "#ffffff".into(),
    }));
}

#[test]
fn create_variable_command_routes_through_editor_state() {
    let mut s = state_with(vec![]);
    assert!(s.apply(EditorCommand::CreateVariable {
        name: "spacing".into(),
        kind: "number".into(),
        default_value: "16".into(),
    }));
    match s.resolve_variable("spacing") {
        Some(VariableScalar::Num(n)) => assert_eq!(*n, 16.0),
        other => panic!("expected Num(16), got {other:?}"),
    }
    // Bad kind string rejects at apply.
    assert!(!s.apply(EditorCommand::CreateVariable {
        name: "x".into(),
        kind: "rainbow".into(),
        default_value: "1".into(),
    }));
}

#[test]
fn delete_variable_command_drops_it() {
    let mut s = state_with_var(VariableKind::Color, "accent");
    assert!(s.apply(EditorCommand::DeleteVariable {
        name: "accent".into(),
    }));
    assert!(s.find_variable("accent").is_none());
    // Unknown name rejects.
    assert!(!s.apply(EditorCommand::DeleteVariable {
        name: "nope".into(),
    }));
}

#[test]
fn rename_variable_command_guards_collisions() {
    let mut s = state_with_var(VariableKind::Number, "old");
    add_variable(&mut s, "taken", VariableKind::Number, VariableScalar::Num(8.0));
    // Collision with an existing different variable rejects.
    assert!(!s.apply(EditorCommand::RenameVariable {
        old_name: "old".into(),
        new_name: "taken".into(),
    }));
    // Happy path.
    assert!(s.apply(EditorCommand::RenameVariable {
        old_name: "old".into(),
        new_name: "renamed".into(),
    }));
    assert!(s.find_variable("renamed").is_some());
    assert!(s.find_variable("old").is_none());
}
