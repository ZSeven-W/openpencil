//! Tests for `mcp::write_tools`. Sibling file so write-tool tests
//! stay separate from the read-tool tests + every file under cap.

use super::write_tools::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

fn doc_with_color_var(name: &str, hex: &str) -> crate::document::Document {
    use crate::document::{Document, Variable, VariableKind, VariableScalar, VariableValue};
    let mut doc = Document::empty();
    doc.var_table.variables.push(Variable {
        name: name.into(),
        kind: VariableKind::Color,
        value: VariableValue::Scalar(VariableScalar::Str(hex.into())),
    });
    doc
}

#[test]
fn set_variable_color_validates_args_and_returns_command() {
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    args.insert("hex".into(), "#00ff00".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, cmd) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            match cmd {
                crate::mcp::McpCommand::SetVariableColor { name, hex } => {
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
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
        }
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
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
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
    let doc = doc_with_color_var("brand", "#ff8800");
    let tool = set_variable_color_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("name".into(), "brand".into());
    for bad in &["not-hex", "ff00ff", "#12", "#fffffg"] {
        args.insert("hex".into(), (*bad).into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "hex={bad}");
            }
            _ => panic!("expected InvalidArgument for {bad}"),
        }
    }
}

#[test]
fn apply_mcp_command_routes_set_variable_color_to_var_table() {
    // End-to-end: tool validates → registry returns
    // OkWithCommand → host's `var_table.apply_mcp_command`
    // writes through to the storage. resolve_color reads the
    // new value back.
    use crate::mcp::McpCommand;
    let mut doc = doc_with_color_var("brand", "#ff8800");
    let cmd = McpCommand::SetVariableColor {
        name: "brand".into(),
        hex: "#11ccaa".into(),
    };
    assert!(doc.var_table.apply_mcp_command(&cmd));
    let c = doc.var_table.resolve_color("brand").unwrap();
    assert!((c.r - 0x11 as f32 / 255.0).abs() < 0.01);
    assert!((c.g - 0xcc as f32 / 255.0).abs() < 0.01);
    assert!((c.b - 0xaa as f32 / 255.0).abs() < 0.01);
}

fn doc_with_theme_axis(name: &str, values: &[&str]) -> crate::document::Document {
    use crate::document::{Document, ThemeAxis};
    let mut doc = Document::empty();
    doc.var_table.themes.push(ThemeAxis {
        name: name.into(),
        values: values.iter().map(|s| s.to_string()).collect(),
    });
    doc
}

#[test]
fn set_active_axis_value_validates_args_and_returns_command() {
    let doc = doc_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "mode".into());
    args.insert("value".into(), "dark".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(out, cmd) => {
            assert_eq!(out.get("wrote"), Some(&"true".to_string()));
            match cmd {
                McpCommand::SetActiveAxisValue { axis, value } => {
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
    let doc = doc_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&doc);
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
    let doc = doc_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&doc);
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
    let doc = doc_with_theme_axis("mode", &["light", "dark"]);
    let tool = set_active_axis_value_snapshot(&doc);
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
fn apply_mcp_command_routes_set_active_axis_value() {
    let mut doc = doc_with_theme_axis("mode", &["light", "dark"]);
    let cmd = McpCommand::SetActiveAxisValue {
        axis: "mode".into(),
        value: "dark".into(),
    };
    assert!(doc.var_table.apply_mcp_command(&cmd));
    assert_eq!(doc.var_table.active_theme.get("mode"), Some(&"dark".to_string()));

    // Invalid value rejected at apply time too (host re-validates).
    let bad = McpCommand::SetActiveAxisValue {
        axis: "mode".into(),
        value: "sepia".into(),
    };
    assert!(!doc.var_table.apply_mcp_command(&bad));
}

#[test]
fn insert_node_validates_required_args() {
    let tool = insert_node_snapshot();
    // Missing kind.
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
    // Invalid kind.
    args.insert("kind".into(), "frobnicate".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => {
            assert_eq!(code, ToolErrorCode::InvalidArgument);
        }
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
        ToolOutcome::OkWithCommand(_, McpCommand::InsertNode { kind, name, x, y, width, height, fill_hex }) => {
            assert_eq!(kind, "rect");
            assert_eq!(name, "My Rect");
            assert_eq!(x, 10);
            assert_eq!(y, 20);
            assert_eq!(width, 100);
            assert_eq!(height, 50);
            assert_eq!(fill_hex.as_deref(), Some("#ff0000"));
        }
        other => panic!("expected InsertNode command, got {other:?}"),
    }
}

#[test]
fn apply_mcp_command_routes_insert_node() {
    use crate::document::Document;
    let mut doc = Document::empty();
    let initial_root_count = doc.pages[0].children.len();
    let cmd = McpCommand::InsertNode {
        kind: "rect".into(),
        name: "Created".into(),
        x: 50,
        y: 60,
        width: 200,
        height: 150,
        fill_hex: Some("#00ff00".into()),
    };
    assert!(doc.apply_mcp_command(&cmd));
    // New node lives on the active page; bounds + fill flow through.
    assert_eq!(
        doc.pages[0].children.len(),
        initial_root_count + 1,
        "node must be added"
    );
    let last = doc.pages[0].children.last().unwrap();
    assert_eq!(last.name, "Created");
    assert_eq!(last.bounds.size.x, 200.0);
    assert_eq!(last.bounds.size.y, 150.0);
    let fill = last.fill.expect("fill set");
    assert!((fill.g - 1.0).abs() < 0.01);
}

#[test]
fn apply_mcp_command_insert_rejects_when_id_space_exhausted() {
    // Codex stop-gate: when an existing node sits at u64::MAX
    // the previous saturating_add(1) wrapped back to u64::MAX and
    // silently overwrote the live node. Now `next_node_id_seed`
    // returns None on overflow + apply_mcp_command fails cleanly.
    use crate::document::{Document, Node, NodeKind};
    let mut doc = Document::empty();
    // Plant a node at the maximum id directly in active page.
    let mut max_node = Node::leaf(u64::MAX, NodeKind::Rect, "max-id-node");
    max_node.bounds = crate::Rect::xywh(0.0, 0.0, 10.0, 10.0);
    doc.pages[0].children.push(max_node);
    let before_len = doc.pages[0].children.len();

    let cmd = McpCommand::InsertNode {
        kind: "rect".into(),
        name: "should-fail".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
    };
    assert!(!doc.apply_mcp_command(&cmd), "id-space-exhausted must reject");
    assert_eq!(
        doc.pages[0].children.len(),
        before_len,
        "no node should have been appended"
    );
    // Live u64::MAX node still has its original name.
    assert_eq!(
        doc.pages[0].children.last().unwrap().name,
        "max-id-node"
    );
}

#[test]
fn apply_mcp_command_rejects_invalid_node_kind() {
    use crate::document::Document;
    let mut doc = Document::empty();
    let cmd = McpCommand::InsertNode {
        kind: "frobnicate".into(),
        name: "X".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
    };
    assert!(!doc.apply_mcp_command(&cmd));
}

#[test]
fn update_node_requires_node_id_and_one_field() {
    let tool = update_node_snapshot();
    // No node_id.
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    // node_id alone (no patch) → MissingArgument.
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("at least one"));
        }
        _ => panic!(),
    }
}

#[test]
fn update_node_validates_node_id_format() {
    let tool = update_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "0".into());
    args.insert("name".into(), "ok".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
    args.insert("node_id".into(), "abc".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn update_node_returns_command_with_partial_patch() {
    let tool = update_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "10".into());
    args.insert("x".into(), "50".into());
    args.insert("width".into(), "200".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::UpdateNode { node_id, x, y, width, height, name, fill_hex }) => {
            assert_eq!(node_id, 10);
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
fn apply_mcp_command_routes_update_node() {
    use crate::document::Document;
    let doc_base = Document::sample();
    let mut doc = doc_base;
    // Sample document's NodeId(11) is "Title" text at (60, 60, 240, 28).
    let cmd = McpCommand::UpdateNode {
        node_id: 11,
        x: Some(100),
        y: None,
        width: None,
        height: Some(50),
        name: Some("Renamed".into()),
        fill_hex: None,
    };
    assert!(doc.apply_mcp_command(&cmd));
    // Walk the frame to find Title node.
    let frame = &doc.pages[0].children[0]; // Frame is first child
    let title = frame
        .children
        .iter()
        .find(|n| n.id.raw() == 11)
        .expect("Title node");
    assert_eq!(title.bounds.origin.x, 100.0);
    assert_eq!(title.bounds.origin.y, 60.0); // unchanged
    assert_eq!(title.bounds.size.y, 50.0);
    assert_eq!(title.name, "Renamed");
}

#[test]
fn apply_mcp_command_update_node_rejects_unknown_id() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let cmd = McpCommand::UpdateNode {
        node_id: 99999,
        x: Some(0),
        y: None,
        width: None,
        height: None,
        name: None,
        fill_hex: None,
    };
    assert!(!doc.apply_mcp_command(&cmd));
}

#[test]
fn delete_node_validates_arg() {
    let tool = delete_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "0".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn apply_mcp_command_routes_delete_node() {
    use crate::document::Document;
    let mut doc = Document::sample();
    // Delete the Title (id 11). The Frame at id 10 has 2 children;
    // after delete it has 1.
    let before_frame_children = doc.pages[0].children[0].children.len();
    let cmd = McpCommand::DeleteNode { node_id: 11 };
    assert!(doc.apply_mcp_command(&cmd));
    let after_frame_children = doc.pages[0].children[0].children.len();
    assert_eq!(after_frame_children, before_frame_children - 1);
    assert!(doc.pages[0].children[0]
        .children
        .iter()
        .all(|n| n.id.raw() != 11));
}

#[test]
fn apply_mcp_command_delete_node_rejects_unknown_id() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let cmd = McpCommand::DeleteNode { node_id: 99999 };
    assert!(!doc.apply_mcp_command(&cmd));
}

#[test]
fn apply_mcp_command_update_node_is_atomic_on_invalid_geometry() {
    // Codex stop-gate: update_node was applying x/y BEFORE
    // checking width/height for negative values, so a request
    // like `{x: 100, width: -1}` would move the node AND reject
    // the resize — leaving the node in a half-updated state.
    // Now every validation runs before any write, so an
    // invalid-geometry request fails cleanly with the node
    // untouched.
    use crate::document::Document;
    let mut doc = Document::sample();
    // Snapshot original bounds of NodeId(11) — sample's Title.
    let before = {
        let frame = &doc.pages[0].children[0];
        let title = frame.children.iter().find(|n| n.id.raw() == 11).unwrap();
        title.bounds
    };
    let cmd = McpCommand::UpdateNode {
        node_id: 11,
        x: Some(999),     // valid
        y: Some(999),     // valid
        width: Some(-1),  // INVALID
        height: None,
        name: Some("should-not-apply".into()),
        fill_hex: None,
    };
    assert!(!doc.apply_mcp_command(&cmd), "negative width must reject");
    // Title is exactly as it was.
    let frame = &doc.pages[0].children[0];
    let title = frame.children.iter().find(|n| n.id.raw() == 11).unwrap();
    assert_eq!(title.bounds.origin.x, before.origin.x, "x was modified");
    assert_eq!(title.bounds.origin.y, before.origin.y, "y was modified");
    assert_eq!(title.bounds.size.x, before.size.x, "w was modified");
    assert_eq!(title.bounds.size.y, before.size.y, "h was modified");
    assert_eq!(title.name, "Title", "name was modified");
}

#[test]
fn apply_mcp_command_update_node_is_atomic_on_invalid_hex() {
    // Parallel atomicity test for the fill_hex path: bad hex must
    // not apply x/y/width/height first.
    use crate::document::Document;
    let mut doc = Document::sample();
    let before = {
        let frame = &doc.pages[0].children[0];
        let title = frame.children.iter().find(|n| n.id.raw() == 11).unwrap();
        title.bounds
    };
    let cmd = McpCommand::UpdateNode {
        node_id: 11,
        x: Some(999),
        y: Some(999),
        width: None,
        height: None,
        name: Some("should-not-apply".into()),
        fill_hex: Some("not-a-hex".into()),
    };
    assert!(!doc.apply_mcp_command(&cmd));
    let frame = &doc.pages[0].children[0];
    let title = frame.children.iter().find(|n| n.id.raw() == 11).unwrap();
    assert_eq!(title.bounds.origin.x, before.origin.x);
    assert_eq!(title.bounds.origin.y, before.origin.y);
    assert_eq!(title.name, "Title");
}
