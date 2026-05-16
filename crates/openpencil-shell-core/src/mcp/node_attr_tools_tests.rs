//! Tests for `mcp::node_attr_tools` — covers tool-call validation
//! AND apply-side semantics for every node attribute writer.

use super::node_attr_tools::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

use crate::document::{Document, Node, NodeKind, Page};
use crate::{Color, Point2D, Rect};

const TEXT_ID: u64 = 101;
const RECT_ID: u64 = 202;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D { x, y },
        size: Point2D { x: w, y: h },
    }
}

fn doc_with_text_node() -> Document {
    let mut doc = Document::empty();
    let mut node = Node::leaf(format!("n{TEXT_ID}"), NodeKind::Text, "label")
        .with_bounds(rect(0.0, 0.0, 100.0, 24.0))
        .with_fill(Color::BLACK)
        .with_text("Hello");
    // Set font_size + font_weight explicitly so the test can detect
    // mutations vs `Node::leaf` defaults (0 / 0).
    node.font_size = 16.0;
    node.font_weight = 400;
    // Document::empty() ships with a "Page 1" already; replace its
    // children rather than pushing a second page (otherwise the
    // fixture node lands at pages[1] and the assertions miss it).
    doc.pages[0] = Page::new("n1", "P", vec![node]);
    doc
}

fn doc_with_rect_node() -> Document {
    let mut doc = Document::empty();
    let node = Node::leaf(format!("n{RECT_ID}"), NodeKind::Rect, "box")
        .with_bounds(rect(0.0, 0.0, 50.0, 50.0))
        .with_fill(Color::RED);
    doc.pages[0] = Page::new("n1", "P", vec![node]);
    doc
}

#[test]
fn set_node_font_size_rejects_non_positive() {
    let tool = set_node_font_size_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), TEXT_ID.to_string());
    args.insert("font_size".into(), "-1".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn set_node_font_size_apply_rejects_non_text_kind() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeFontSize {
        node_id: RECT_ID,
        font_size: 24.0,
    };
    assert!(!doc.apply_mcp_command(&cmd));
    assert_eq!(
        doc.pages[0].children[0].font_size, 0.0,
        "rect's font_size (default 0) must be unchanged"
    );
}

#[test]
fn set_node_font_size_apply_writes_on_text_kind() {
    let mut doc = doc_with_text_node();
    let cmd = McpCommand::SetNodeFontSize {
        node_id: TEXT_ID,
        font_size: 32.0,
    };
    assert!(doc.apply_mcp_command(&cmd));
    assert_eq!(doc.pages[0].children[0].font_size, 32.0);
}

#[test]
fn set_node_font_weight_apply_rejects_out_of_range() {
    let mut doc = doc_with_text_node();
    for bad in [0u16, 1001, 5000] {
        let cmd = McpCommand::SetNodeFontWeight {
            node_id: TEXT_ID,
            font_weight: bad,
        };
        assert!(
            !doc.apply_mcp_command(&cmd),
            "weight {bad} should reject"
        );
    }
    assert_eq!(doc.pages[0].children[0].font_weight, 400);
}

#[test]
fn set_node_font_weight_apply_writes_on_text_kind() {
    let mut doc = doc_with_text_node();
    let cmd = McpCommand::SetNodeFontWeight {
        node_id: TEXT_ID,
        font_weight: 700,
    };
    assert!(doc.apply_mcp_command(&cmd));
    assert_eq!(doc.pages[0].children[0].font_weight, 700);
}

#[test]
fn set_node_font_weight_apply_rejects_non_text_kind() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeFontWeight {
        node_id: RECT_ID,
        font_weight: 700,
    };
    assert!(!doc.apply_mcp_command(&cmd));
}

#[test]
fn set_node_stroke_hex_creates_default_stroke_when_none() {
    let mut doc = doc_with_rect_node();
    assert!(doc.pages[0].children[0].stroke.is_none());
    let cmd = McpCommand::SetNodeStrokeHex {
        node_id: RECT_ID,
        hex: "#00ff00".into(),
    };
    assert!(doc.apply_mcp_command(&cmd));
    let stroke = doc.pages[0].children[0]
        .stroke
        .as_ref()
        .expect("stroke attached");
    assert_eq!(stroke.width, 1.0, "default width must be 1 doc-px");
    assert!((stroke.color.g - 1.0).abs() < 1e-3);
    assert!(stroke.color.r < 1e-3);
}

#[test]
fn set_node_stroke_hex_overwrites_existing_color_preserves_width() {
    let mut doc = doc_with_rect_node();
    doc.pages[0].children[0].stroke = Some(crate::document::Stroke {
        color: Color::RED,
        width: 3.5,
    });
    let cmd = McpCommand::SetNodeStrokeHex {
        node_id: RECT_ID,
        hex: "#0000ff".into(),
    };
    assert!(doc.apply_mcp_command(&cmd));
    let stroke = doc.pages[0].children[0].stroke.as_ref().unwrap();
    assert!((stroke.color.b - 1.0).abs() < 1e-3);
    assert_eq!(stroke.width, 3.5, "existing width must be preserved");
}

#[test]
fn set_node_stroke_width_zero_clears_stroke() {
    let mut doc = doc_with_rect_node();
    doc.pages[0].children[0].stroke = Some(crate::document::Stroke {
        color: Color::BLACK,
        width: 2.0,
    });
    let cmd = McpCommand::SetNodeStrokeWidth {
        node_id: RECT_ID,
        width: 0.0,
    };
    assert!(doc.apply_mcp_command(&cmd));
    assert!(doc.pages[0].children[0].stroke.is_none());
}

#[test]
fn set_node_stroke_width_attaches_default_black_when_none() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeStrokeWidth {
        node_id: RECT_ID,
        width: 4.0,
    };
    assert!(doc.apply_mcp_command(&cmd));
    let stroke = doc.pages[0].children[0].stroke.as_ref().unwrap();
    assert_eq!(stroke.width, 4.0);
    assert!(stroke.color.r < 1e-3);
    assert!(stroke.color.g < 1e-3);
    assert!(stroke.color.b < 1e-3);
}

#[test]
fn set_node_stroke_width_rejects_negative() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeStrokeWidth {
        node_id: RECT_ID,
        width: -1.0,
    };
    assert!(!doc.apply_mcp_command(&cmd));
}

#[test]
fn set_node_fill_hex_writes_to_node_fill() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeFillHex {
        node_id: RECT_ID,
        hex: "#0080ff".into(),
    };
    assert!(doc.apply_mcp_command(&cmd));
    let fill = doc.pages[0].children[0]
        .fill
        .as_ref()
        .expect("fill present");
    assert!(fill.r < 1e-3);
    assert!((fill.b - 1.0).abs() < 1e-3);
}

#[test]
fn set_node_fill_hex_rejects_bad_hex() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeFillHex {
        node_id: RECT_ID,
        hex: "not-a-hex".into(),
    };
    assert!(!doc.apply_mcp_command(&cmd));
    let fill = doc.pages[0].children[0].fill.as_ref().unwrap();
    assert!((fill.r - 1.0).abs() < 1e-3, "original red fill preserved");
}

#[test]
fn set_node_name_writes_trimmed_value() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeName {
        node_id: RECT_ID,
        name: "  new name  ".into(),
    };
    assert!(doc.apply_mcp_command(&cmd));
    assert_eq!(doc.pages[0].children[0].name, "new name");
}

#[test]
fn set_node_name_rejects_empty_after_trim() {
    let mut doc = doc_with_rect_node();
    let cmd = McpCommand::SetNodeName {
        node_id: RECT_ID,
        name: "    ".into(),
    };
    assert!(!doc.apply_mcp_command(&cmd));
    assert_eq!(doc.pages[0].children[0].name, "box");
}

#[test]
fn set_node_name_tool_rejects_empty_at_call_time() {
    let tool = set_node_name_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), RECT_ID.to_string());
    args.insert("name".into(), "   ".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn unknown_node_id_rejects_at_apply_time() {
    let mut doc = doc_with_rect_node();
    let cmds = [
        McpCommand::SetNodeFillHex {
            node_id: 9999,
            hex: "#ffffff".into(),
        },
        McpCommand::SetNodeName {
            node_id: 9999,
            name: "x".into(),
        },
        McpCommand::SetNodeStrokeHex {
            node_id: 9999,
            hex: "#000000".into(),
        },
        McpCommand::SetNodeStrokeWidth {
            node_id: 9999,
            width: 2.0,
        },
        McpCommand::SetNodeFontSize {
            node_id: 9999,
            font_size: 12.0,
        },
        McpCommand::SetNodeFontWeight {
            node_id: 9999,
            font_weight: 700,
        },
    ];
    for cmd in cmds {
        assert!(!doc.apply_mcp_command(&cmd), "unknown id must reject: {cmd:?}");
    }
}
