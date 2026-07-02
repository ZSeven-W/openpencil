//! Tests for `mcp::node_attr_tools`.
//!
//! Ported off the old shell-core `Document` onto `op_editor_core::
//! EditorState`. These cover the tool layer — argument validation +
//! `EditorCommand` emission — plus a handful of end-to-end checks
//! through `EditorState::apply`. The apply-path semantics themselves
//! are exhaustively covered by `op-editor-core`'s `command_tests.rs`.

use super::node_attr_tools::*;
use super::test_fixtures::{rect, state_with, text};
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};
use op_editor_core::{EditorState, NodeId};
use std::collections::BTreeMap;

const TEXT_ID: &str = "n101";
const RECT_ID: &str = "n202";

fn state_with_text_node() -> EditorState {
    state_with(vec![text(TEXT_ID, "label", 0.0, 0.0, 100.0, 24.0, "Hello")])
}

fn state_with_rect_node() -> EditorState {
    state_with(vec![rect(RECT_ID, "box", 0.0, 0.0, 50.0, 50.0)])
}

#[test]
fn set_node_font_size_rejects_non_positive() {
    let tool = set_node_font_size_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), TEXT_ID.into());
    args.insert("font_size".into(), "-1".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn set_node_font_size_returns_command() {
    let tool = set_node_font_size_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), TEXT_ID.into());
    args.insert("font_size".into(), "32".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::SetNodeFontSize { node_id, font_size }) => {
            assert_eq!(node_id.as_str(), TEXT_ID);
            assert_eq!(font_size, 32.0);
        }
        other => panic!("expected SetNodeFontSize, got {other:?}"),
    }
}

#[test]
fn set_node_font_size_command_applies_on_text_kind() {
    let mut s = state_with_text_node();
    assert!(s.apply(EditorCommand::SetNodeFontSize {
        node_id: NodeId::new(TEXT_ID),
        font_size: 32.0,
    }));
    // Non-text kind rejects.
    let mut r = state_with_rect_node();
    assert!(!r.apply(EditorCommand::SetNodeFontSize {
        node_id: NodeId::new(RECT_ID),
        font_size: 24.0,
    }));
}

#[test]
fn set_node_font_weight_tool_rejects_out_of_range() {
    let tool = set_node_font_weight_snapshot();
    for bad in ["0", "1001", "5000"] {
        let mut args = BTreeMap::new();
        args.insert("node_id".into(), TEXT_ID.into());
        args.insert("font_weight".into(), bad.into());
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "weight {bad}")
            }
            other => panic!("expected InvalidArgument for {bad}, got {other:?}"),
        }
    }
}

#[test]
fn set_node_font_weight_command_applies_on_text_kind() {
    let mut s = state_with_text_node();
    assert!(s.apply(EditorCommand::SetNodeFontWeight {
        node_id: NodeId::new(TEXT_ID),
        font_weight: 700,
    }));
    let mut r = state_with_rect_node();
    assert!(!r.apply(EditorCommand::SetNodeFontWeight {
        node_id: NodeId::new(RECT_ID),
        font_weight: 700,
    }));
}

#[test]
fn set_node_stroke_hex_command_applies() {
    let mut s = state_with_rect_node();
    assert!(s.apply(EditorCommand::SetNodeStrokeHex {
        node_id: NodeId::new(RECT_ID),
        hex: "#00ff00".into(),
    }));
}

#[test]
fn set_node_stroke_width_command_applies() {
    let mut s = state_with_rect_node();
    assert!(s.apply(EditorCommand::SetNodeStrokeWidth {
        node_id: NodeId::new(RECT_ID),
        width: 4.0,
    }));
}

#[test]
fn set_node_stroke_width_tool_rejects_negative() {
    let tool = set_node_stroke_width_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), RECT_ID.into());
    args.insert("width".into(), "-1".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn set_node_stroke_side_width_returns_command() {
    let tool = set_node_stroke_side_width_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), RECT_ID.into());
    args.insert("side".into(), "bottom".into());
    args.insert("width".into(), "3".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(
            _,
            EditorCommand::SetNodeStrokeSideWidth {
                node_id,
                side,
                width,
            },
        ) => {
            assert_eq!(node_id.as_str(), RECT_ID);
            assert_eq!(side, op_editor_core::StrokeSide::Bottom);
            assert_eq!(width, 3.0);
        }
        other => panic!("expected SetNodeStrokeSideWidth, got {other:?}"),
    }
}

#[test]
fn set_node_stroke_side_width_rejects_unknown_side() {
    let tool = set_node_stroke_side_width_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), RECT_ID.into());
    args.insert("side".into(), "diagonal".into());
    args.insert("width".into(), "2".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn set_node_stroke_side_width_rejects_negative() {
    let tool = set_node_stroke_side_width_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), RECT_ID.into());
    args.insert("side".into(), "top".into());
    args.insert("width".into(), "-1".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn set_node_fill_hex_command_applies() {
    let mut s = state_with_rect_node();
    assert!(s.apply(EditorCommand::SetNodeFillHex {
        node_id: NodeId::new(RECT_ID),
        hex: "#0080ff".into(),
    }));
}

#[test]
fn set_node_name_returns_command() {
    let tool = set_node_name_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), RECT_ID.into());
    args.insert("name".into(), "new name".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::SetNodeName { node_id, name }) => {
            assert_eq!(node_id.as_str(), RECT_ID);
            assert_eq!(name, "new name");
        }
        other => panic!("expected SetNodeName, got {other:?}"),
    }
}

#[test]
fn set_node_name_tool_rejects_empty_at_call_time() {
    let tool = set_node_name_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), RECT_ID.into());
    args.insert("name".into(), "   ".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn node_attr_tools_reject_empty_node_id() {
    // The canonical schema uses string ids; the empty string (NONE
    // sentinel) is rejected at the wire layer.
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "".into());
    args.insert("hex".into(), "#ffffff".into());
    match set_node_fill_hex_snapshot().call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn unknown_node_id_rejects_at_apply_time() {
    let mut s = state_with_rect_node();
    let unknown = NodeId::new("n9999");
    assert!(!s.apply(EditorCommand::SetNodeFillHex {
        node_id: unknown.clone(),
        hex: "#ffffff".into(),
    }));
    assert!(!s.apply(EditorCommand::SetNodeName {
        node_id: unknown.clone(),
        name: "x".into(),
    }));
    assert!(!s.apply(EditorCommand::SetNodeStrokeWidth {
        node_id: unknown,
        width: 2.0,
    }));
}
