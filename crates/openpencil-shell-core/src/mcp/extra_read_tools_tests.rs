//! Tests for `mcp::extra_read_tools::GetNodeChildren`.

use super::extra_read_tools::*;
use super::{McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

use crate::document::{Document, Node, NodeKind, Page};
use crate::{Color, Point2D, Rect};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D { x, y },
        size: Point2D { x: w, y: h },
    }
}

fn doc_with_frame_and_two_children() -> Document {
    let mut doc = Document::empty();
    let child_a = Node::leaf("n20", NodeKind::Rect, "a")
        .with_bounds(rect(10.0, 10.0, 30.0, 30.0))
        .with_fill(Color::RED);
    let child_b = Node::leaf("n21", NodeKind::Text, "b")
        .with_bounds(rect(50.0, 10.0, 30.0, 20.0))
        .with_fill(Color::BLACK);
    let frame = Node::with_children("n10", NodeKind::Frame, "frame", vec![child_a, child_b])
        .with_bounds(rect(0.0, 0.0, 200.0, 100.0));
    doc.pages[0] = Page::new("n1", "P", vec![frame]);
    doc
}

#[test]
fn get_node_children_returns_count_and_ids_for_known_parent() {
    let doc = doc_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n10".into());
    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"2".to_string()));
            assert_eq!(out.get("ids"), Some(&"n20,n21".to_string()));
            assert_eq!(out.get("child_0_id"), Some(&"n20".to_string()));
            assert_eq!(out.get("child_0_kind"), Some(&"rect".to_string()));
            assert_eq!(out.get("child_1_kind"), Some(&"text".to_string()));
        }
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[test]
fn get_node_children_errors_on_unknown_id() {
    let doc = doc_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n9999".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("9999"));
        }
        other => panic!("expected ToolFailed, got {other:?}"),
    }
}

#[test]
fn get_node_children_returns_empty_for_leaf_id() {
    // Codex stop-gate: leaves are *known* ids, so they must
    // return `count=0` instead of an error. Only truly unknown
    // ids should produce ToolFailed.
    let doc = doc_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n20".into());
    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"0".to_string()));
            assert_eq!(out.get("ids"), Some(&"".to_string()));
        }
        other => panic!("expected Ok(count=0) for known leaf, got {other:?}"),
    }
}

#[test]
fn get_node_children_returns_empty_for_known_container_without_children() {
    let mut doc = Document::empty();
    // An empty Frame at the page root — known id, no children.
    let frame = Node::with_children("n42", NodeKind::Frame, "empty", vec![])
        .with_bounds(rect(0.0, 0.0, 100.0, 100.0));
    doc.pages[0] = Page::new("n1", "P", vec![frame]);
    let tool = get_node_children_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "n42".into());
    match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            assert_eq!(out.get("count"), Some(&"0".to_string()));
        }
        other => panic!("expected Ok(count=0) for empty container, got {other:?}"),
    }
}

#[test]
fn get_node_children_rejects_missing_arg() {
    let doc = doc_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&doc);
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        other => panic!("expected MissingArgument, got {other:?}"),
    }
}

#[test]
fn get_node_children_rejects_unknown_string_id() {
    // `node_id` is now the canonical `.op` schema string id —
    // any arbitrary string is a syntactically-valid id, so an
    // id that isn't in the document is a not-found ToolFailed
    // (no numeric-parse InvalidArgument path remains).
    let doc = doc_with_frame_and_two_children();
    let tool = get_node_children_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "not-a-known-id".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::ToolFailed),
        other => panic!("expected ToolFailed, got {other:?}"),
    }
}
