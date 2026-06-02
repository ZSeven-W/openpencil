//! Tests for `mcp::write_tools`.
//!
//! Ported off the old shell-core `Document` / `McpCommand` onto
//! `op_editor_core::EditorState` / `EditorCommand`. These cover the
//! tool layer — argument validation + `EditorCommand` emission, plus a
//! handful of end-to-end checks through `EditorState::apply`. The
//! apply-path correctness itself is exhaustively covered by
//! `op-editor-core`'s own `command_tests.rs`.

use super::test_fixtures::{add_theme_axis, add_variable, sample, state_with};
use super::write_tools::*;
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use jian_ops_schema::variable::{VariableKind, VariableScalar};
use op_editor_core::{EditorState, NodeId};
use std::collections::BTreeMap;

fn state_with_color_var(name: &str, hex: &str) -> EditorState {
    let mut s = state_with(vec![]);
    add_variable(
        &mut s,
        name,
        VariableKind::Color,
        VariableScalar::Str(hex.to_string()),
    );
    s
}

#[test]
fn set_variable_color_validates_args_and_returns_command() {
    let s = state_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    args.insert("hex".into(), "#00ff00".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, cmd) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            match cmd {
                EditorCommand::SetVariableColor { name, hex } => {
                    assert_eq!(name, "brand");
                    assert_eq!(hex, "#00ff00");
                }
                other => panic!("expected SetVariableColor, got {other:?}"),
            }
        }
        other => panic!("expected OkWithCommand, got {other:?}"),
    }
}

#[test]
fn set_variable_color_errors_on_missing_args() {
    let s = state_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!("expected MissingArgument"),
    }
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("hex"));
        }
        _ => panic!("expected MissingArgument"),
    }
}

#[test]
fn set_variable_color_errors_on_unknown_variable() {
    let s = state_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "no-such-var".into());
    args.insert("hex".into(), "#000000".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("no-such-var"));
        }
        _ => panic!("expected ToolFailed"),
    }
}

#[test]
fn set_variable_color_errors_on_invalid_hex() {
    let s = state_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    for bad in &["not-hex", "ff00ff", "#12", "#fffffg"] {
        args.insert("hex".into(), (*bad).into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "hex={bad}")
            }
            _ => panic!("expected InvalidArgument for {bad}"),
        }
    }
}

#[test]
fn set_variable_color_command_applies_through_editor_state() {
    let mut s = state_with_color_var("brand", "#ff8800");
    let cmd = EditorCommand::SetVariableColor {
        name: "brand".into(),
        hex: "#11ccaa".into(),
    };
    assert!(s.apply(cmd));
    match s.resolve_variable("brand") {
        Some(VariableScalar::Str(hex)) => assert_eq!(hex, "#11ccaa"),
        other => panic!("unexpected {other:?}"),
    }
}

fn state_with_theme_axis(name: &str, values: &[&str]) -> EditorState {
    let mut s = state_with(vec![]);
    add_theme_axis(&mut s, name, values);
    s
}

#[test]
fn set_active_axis_value_validates_args_and_returns_command() {
    let s = state_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "mode".into());
    args.insert("value".into(), "dark".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, cmd) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            match cmd {
                EditorCommand::SetActiveAxisValue { axis, value } => {
                    assert_eq!(axis, "mode");
                    assert_eq!(value, "dark");
                }
                other => panic!("expected SetActiveAxisValue, got {other:?}"),
            }
        }
        other => panic!("expected OkWithCommand, got {other:?}"),
    }
}

#[test]
fn set_active_axis_value_errors_on_missing_args() {
    let s = state_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&s);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "mode".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("value"));
        }
        _ => panic!(),
    }
}

#[test]
fn set_active_axis_value_errors_on_unknown_axis() {
    let s = state_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "density".into());
    args.insert("value".into(), "compact".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("density"));
        }
        _ => panic!(),
    }
}

#[test]
fn set_active_axis_value_errors_on_value_not_in_axis() {
    let s = state_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&s);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "mode".into());
    args.insert("value".into(), "sepia".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::InvalidArgument);
            assert!(msg.contains("light"));
            assert!(msg.contains("dark"));
        }
        _ => panic!(),
    }
}

#[test]
fn set_active_axis_value_command_applies_through_editor_state() {
    let mut s = state_with_theme_axis("mode", &["light", "dark"]);
    assert!(s.apply(EditorCommand::SetActiveAxisValue {
        axis: "mode".into(),
        value: "dark".into(),
    }));
    assert_eq!(
        s.ui.variables.active_theme.get("mode").map(String::as_str),
        Some("dark")
    );
    assert!(!s.apply(EditorCommand::SetActiveAxisValue {
        axis: "mode".into(),
        value: "sepia".into(),
    }));
}

#[test]
fn insert_node_validates_required_args() {
    let tool = insert_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("name".into(), "X".into());
    args.insert("x".into(), "0".into());
    args.insert("y".into(), "0".into());
    args.insert("width".into(), "10".into());
    args.insert("height".into(), "10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("kind"));
        }
        _ => panic!(),
    }
    args.insert("kind".into(), "frobnicate".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn insert_node_validates_numeric_args() {
    let tool = insert_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "X".into());
    args.insert("x".into(), "not-a-number".into());
    args.insert("y".into(), "0".into());
    args.insert("width".into(), "10".into());
    args.insert("height".into(), "10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
    args.insert("x".into(), "0".into());
    args.insert("width".into(), "-5".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::InvalidArgument);
            assert!(msg.contains("non-negative"));
        }
        _ => panic!(),
    }
}

#[test]
fn insert_node_validates_optional_fill_hex() {
    let tool = insert_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "X".into());
    args.insert("x".into(), "0".into());
    args.insert("y".into(), "0".into());
    args.insert("width".into(), "10".into());
    args.insert("height".into(), "10".into());
    args.insert("fill_hex".into(), "not-hex".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn insert_node_returns_command_with_parsed_args() {
    let tool = insert_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "My Rect".into());
    args.insert("x".into(), "10".into());
    args.insert("y".into(), "20".into());
    args.insert("width".into(), "100".into());
    args.insert("height".into(), "50".into());
    args.insert("fill_hex".into(), "#ff0000".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::InsertNode {
                kind,
                name,
                x,
                y,
                width,
                height,
                fill_hex,
                target_parent,
                page_id,
            },
        ) => {
            assert_eq!(kind, "rect");
            assert_eq!(name, "My Rect");
            assert_eq!(x, 10);
            assert_eq!(y, 20);
            assert_eq!(width, 100);
            assert_eq!(height, 50);
            assert_eq!(fill_hex.as_deref(), Some("#ff0000"));
            assert!(!target_parent.is_real());
            assert_eq!(page_id, None);
        }
        other => panic!("expected InsertNode command, got {other:?}"),
    }
}

#[test]
fn insert_node_accepts_ts_parent_and_page_args() {
    let tool = insert_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "Nested".into());
    args.insert("x".into(), "1".into());
    args.insert("y".into(), "2".into());
    args.insert("width".into(), "3".into());
    args.insert("height".into(), "4".into());
    args.insert("parent".into(), "n10".into());
    args.insert("pageId".into(), "page-2".into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::InsertNode {
                target_parent,
                page_id,
                ..
            },
        ) => {
            assert_eq!(target_parent.as_str(), "n10");
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected InsertNode command with parent/page, got {other:?}"),
    }
}

#[test]
fn insert_node_command_applies_through_editor_state() {
    let mut s = state_with(vec![]);
    let before = s.active_children().len();
    assert!(s.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Created".into(),
        x: 50,
        y: 60,
        width: 200,
        height: 150,
        fill_hex: Some("#00ff00".into()),
        target_parent: NodeId::NONE,
        page_id: None,
    }));
    assert_eq!(s.active_children().len(), before + 1);
}

#[test]
fn update_node_requires_node_id_and_one_field() {
    let tool = update_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("at least one"));
        }
        _ => panic!(),
    }
}

#[test]
fn update_node_rejects_empty_node_id() {
    // The canonical schema uses string ids — any non-empty string is
    // syntactically valid; only the empty string (NONE sentinel) is
    // rejected at the wire layer.
    let tool = update_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "".into());
    args.insert("name".into(), "ok".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn update_node_returns_command_with_partial_patch() {
    let tool = update_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n10".into());
    args.insert("x".into(), "50".into());
    args.insert("width".into(), "200".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::UpdateNode {
                node_id,
                x,
                y,
                width,
                height,
                name,
                fill_hex,
            },
        ) => {
            assert_eq!(node_id.as_str(), "n10");
            assert_eq!(x, Some(50));
            assert_eq!(y, None);
            assert_eq!(width, Some(200));
            assert_eq!(height, None);
            assert!(name.is_none());
            assert!(fill_hex.is_none());
        }
        other => panic!("expected UpdateNode command, got {other:?}"),
    }
}

#[test]
fn update_node_command_applies_through_editor_state() {
    let mut s = sample();
    assert!(s.apply(EditorCommand::UpdateNode {
        node_id: op_editor_core::NodeId::new("n11"),
        x: Some(100),
        y: None,
        width: None,
        height: Some(50),
        name: Some("Renamed".into()),
        fill_hex: None,
    }));
    // Unknown id rejects.
    assert!(!s.apply(EditorCommand::UpdateNode {
        node_id: op_editor_core::NodeId::new("n99999"),
        x: Some(0),
        y: None,
        width: None,
        height: None,
        name: None,
        fill_hex: None,
    }));
}

#[test]
fn delete_node_validates_arg() {
    let tool = delete_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn delete_node_command_applies_through_editor_state() {
    let mut s = sample();
    assert!(s.apply(EditorCommand::DeleteNode {
        node_id: op_editor_core::NodeId::new("n11"),
    }));
    assert!(!s.apply(EditorCommand::DeleteNode {
        node_id: op_editor_core::NodeId::new("n99999"),
    }));
}

#[test]
fn move_node_validates_args() {
    let tool = move_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("target_parent_id"));
        }
        _ => panic!(),
    }
    // node_id == target_parent_id rejects.
    args.insert("node_id".into(), "n10".into());
    args.insert("target_parent_id".into(), "n10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn move_node_returns_command_with_empty_target_for_page_root() {
    let tool = move_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n11".into());
    // Legacy "0" + the empty string both map to the page-root sentinel.
    args.insert("target_parent_id".into(), "0".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::MoveNode {
                node_id,
                target_parent,
            },
        ) => {
            assert_eq!(node_id.as_str(), "n11");
            assert!(!target_parent.is_real(), "0 maps to the page-root NONE");
        }
        other => panic!("expected MoveNode, got {other:?}"),
    }
}

#[test]
fn move_node_command_applies_through_editor_state() {
    let mut s = sample();
    // Move Title (n11) to the page root.
    assert!(s.apply(EditorCommand::MoveNode {
        node_id: op_editor_core::NodeId::new("n11"),
        target_parent: op_editor_core::NodeId::NONE,
    }));
    assert!(s
        .active_children()
        .iter()
        .any(|n| op_editor_core::pen_node_ext::PenNodeExt::id_str(n) == "n11"));
}

#[test]
fn import_svg_accepts_ts_parent_arg() {
    let tool = import_svg_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "svg".into(),
        r#"<svg><rect width="10" height="10"/></svg>"#.into(),
    );
    args.insert("parent".into(), "n10".into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::ImportSvg {
                svg, target_parent, ..
            },
        ) => {
            assert!(svg.contains("<svg>"));
            assert_eq!(target_parent.as_str(), "n10");
        }
        other => panic!("expected ImportSvg with parent, got {other:?}"),
    }
}

#[test]
fn import_svg_maps_root_parent_alias_to_page_root() {
    let tool = import_svg_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "svg".into(),
        r#"<svg><rect width="10" height="10"/></svg>"#.into(),
    );
    args.insert("parent".into(), "root".into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::ImportSvg { target_parent, .. }) => {
            assert!(!target_parent.is_real(), "root maps to page root")
        }
        other => panic!("expected ImportSvg with page-root parent, got {other:?}"),
    }
}

#[test]
fn import_svg_accepts_ts_page_arg() {
    let tool = import_svg_snapshot();
    let mut args = BTreeMap::new();
    args.insert(
        "svg".into(),
        r#"<svg><rect width="10" height="10"/></svg>"#.into(),
    );
    args.insert("pageId".into(), "page-2".into());

    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::ImportSvg { page_id, .. }) => {
            assert_eq!(page_id.as_deref(), Some("page-2"));
        }
        other => panic!("expected ImportSvg with page_id, got {other:?}"),
    }
}
