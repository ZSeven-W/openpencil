//! Tests for selected slices of `mcp::component_tools` — covers
//! the newly-added `set_selection_set` tool's tolerance for
//! unknown / duplicate ids.

use super::component_tools::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

use crate::document::{Document, Node, NodeId, NodeKind, Page};
use crate::{Color, Point2D, Rect};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D { x, y },
        size: Point2D { x: w, y: h },
    }
}

fn doc_with_two_nodes() -> Document {
    let mut doc = Document::empty();
    let a = Node::leaf(11, NodeKind::Rect, "a")
        .with_bounds(rect(0.0, 0.0, 10.0, 10.0))
        .with_fill(Color::RED);
    let b = Node::leaf(22, NodeKind::Rect, "b")
        .with_bounds(rect(20.0, 0.0, 10.0, 10.0))
        .with_fill(Color::BLUE);
    doc.pages[0] = Page::new(1, "P", vec![a, b]);
    doc
}

#[test]
fn set_selection_set_drops_unknown_ids_keeps_known() {
    let mut doc = doc_with_two_nodes();
    let cmd = McpCommand::SetSelectionSet {
        node_ids: vec![11, 999, 22, 7777],
    };
    assert!(doc.apply_mcp_command(&cmd));
    let ids: Vec<u64> = doc.selected_set.iter().map(|n| n.raw()).collect();
    assert_eq!(ids, vec![11, 22], "unknown 999/7777 must drop");
    assert_eq!(doc.selected.raw(), 22, "anchor follows last resolved id");
}

#[test]
fn set_selection_set_dedupes_repeated_ids() {
    let mut doc = doc_with_two_nodes();
    let cmd = McpCommand::SetSelectionSet {
        node_ids: vec![11, 22, 11, 22, 11],
    };
    assert!(doc.apply_mcp_command(&cmd));
    let ids: Vec<u64> = doc.selected_set.iter().map(|n| n.raw()).collect();
    assert_eq!(ids, vec![11, 22], "duplicates collapse to first occurrence");
}

#[test]
fn set_selection_set_empty_clears_selection() {
    let mut doc = doc_with_two_nodes();
    doc.selected_set = vec![NodeId::new(11), NodeId::new(22)];
    doc.selected = NodeId::new(22);
    let cmd = McpCommand::SetSelectionSet { node_ids: vec![] };
    assert!(doc.apply_mcp_command(&cmd));
    assert!(doc.selected_set.is_empty());
    assert_eq!(doc.selected.raw(), NodeId::NONE.raw());
}

#[test]
fn set_selection_set_all_unknown_clears_selection() {
    let mut doc = doc_with_two_nodes();
    doc.selected_set = vec![NodeId::new(11)];
    doc.selected = NodeId::new(11);
    let cmd = McpCommand::SetSelectionSet {
        node_ids: vec![9001, 9002],
    };
    assert!(doc.apply_mcp_command(&cmd));
    assert!(doc.selected_set.is_empty(), "no surviving ids ⇒ clear");
}

#[test]
fn set_selection_set_tool_rejects_negative_ids() {
    let tool = set_selection_set_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_ids".into(), "11, -3, 22".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
}

#[test]
fn toggle_node_selection_adds_then_removes() {
    let mut doc = doc_with_two_nodes();
    // First call: 11 not in set ⇒ add as anchor.
    let cmd = McpCommand::ToggleNodeSelection { node_id: 11 };
    assert!(doc.apply_mcp_command(&cmd));
    assert_eq!(doc.selected_set.len(), 1);
    assert_eq!(doc.selected_set[0].raw(), 11);
    assert_eq!(doc.selected.raw(), 11);
    // Second call: 22 not in set ⇒ add as new anchor.
    let cmd = McpCommand::ToggleNodeSelection { node_id: 22 };
    assert!(doc.apply_mcp_command(&cmd));
    assert_eq!(doc.selected_set.len(), 2);
    assert_eq!(doc.selected.raw(), 22);
    // Third call: 11 in set ⇒ remove; anchor falls back to last
    // surviving id (22).
    let cmd = McpCommand::ToggleNodeSelection { node_id: 11 };
    assert!(doc.apply_mcp_command(&cmd));
    assert_eq!(doc.selected_set.len(), 1);
    assert_eq!(doc.selected_set[0].raw(), 22);
    assert_eq!(doc.selected.raw(), 22);
}

#[test]
fn toggle_node_selection_rejects_unknown_id_at_apply_time() {
    let mut doc = doc_with_two_nodes();
    let cmd = McpCommand::ToggleNodeSelection { node_id: 9999 };
    assert!(!doc.apply_mcp_command(&cmd));
    assert!(doc.selected_set.is_empty());
}

#[test]
fn selection_apply_rejects_ids_on_non_active_pages() {
    // Codex stop-gate: selection state is scoped to the active
    // page; an id that exists only on a *different* page must be
    // rejected by every selection apply path. Otherwise the doc
    // ends up with `selected_set` pointing at nodes the canvas /
    // panels can't see, and every downstream read silently fails.
    let mut doc = doc_with_two_nodes();
    // Add a second page carrying its own node id=555.
    let off_page_node = Node::leaf(555, NodeKind::Rect, "off")
        .with_bounds(rect(0.0, 0.0, 10.0, 10.0))
        .with_fill(Color::GREEN);
    doc.pages.push(Page::new(2, "P2", vec![off_page_node]));
    // active_page_index stays 0 (the page that has 11, 22).
    let cmds = [
        McpCommand::SetSelection { node_id: 555 },
        McpCommand::ToggleNodeSelection { node_id: 555 },
    ];
    for cmd in cmds {
        assert!(
            !doc.apply_mcp_command(&cmd),
            "off-page id 555 must reject: {cmd:?}"
        );
        assert!(
            doc.selected_set.is_empty(),
            "selection must stay empty after rejected {cmd:?}"
        );
    }
    // set_selection_set: off-page id drops silently, on-page id
    // survives.
    let cmd = McpCommand::SetSelectionSet {
        node_ids: vec![11, 555, 22],
    };
    assert!(doc.apply_mcp_command(&cmd));
    let ids: Vec<u64> = doc.selected_set.iter().map(|n| n.raw()).collect();
    assert_eq!(ids, vec![11, 22], "off-page 555 must be filtered out");
}

#[test]
fn cycle_active_axis_value_rejects_unknown_axis_at_call_time() {
    // Codex stop-gate: previously the tool was zero-state and an
    // unknown axis fell through to the applier, which surfaced as
    // a generic ToolErrorCode::Internal "host rejected command".
    // The snapshot now carries the set of known axes-with-values
    // so the tool returns ToolFailed up front, matching
    // `set_active_axis_value`'s contract.
    let mut doc = doc_with_two_nodes();
    doc.var_table.themes.push(crate::document::ThemeAxis {
        name: "mode".into(),
        values: vec!["light".into(), "dark".into()],
    });
    let tool = cycle_active_axis_value_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "nope".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::ToolFailed);
            assert!(msg.contains("nope"));
        }
        other => panic!("expected ToolFailed for unknown axis, got {other:?}"),
    }
}

#[test]
fn cycle_active_axis_value_accepts_known_axis() {
    let mut doc = doc_with_two_nodes();
    doc.var_table.themes.push(crate::document::ThemeAxis {
        name: "mode".into(),
        values: vec!["light".into(), "dark".into()],
    });
    let tool = cycle_active_axis_value_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "mode".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::CycleActiveAxisValue { axis }) => {
            assert_eq!(axis, "mode");
        }
        other => panic!("expected OkWithCommand, got {other:?}"),
    }
}

#[test]
fn cycle_active_axis_value_excludes_empty_axes_from_snapshot() {
    let mut doc = doc_with_two_nodes();
    doc.var_table.themes.push(crate::document::ThemeAxis {
        name: "empty".into(),
        values: vec![],
    });
    let tool = cycle_active_axis_value_snapshot(&doc);
    let mut args = BTreeMap::new();
    args.insert("axis".into(), "empty".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::ToolFailed),
        other => panic!("empty-values axis must reject, got {other:?}"),
    }
}

#[test]
fn set_selection_set_tool_accepts_empty_arg() {
    let tool = set_selection_set_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_ids".into(), "".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::SetSelectionSet { node_ids }) => {
            assert!(node_ids.is_empty());
        }
        other => panic!("expected OkWithCommand with empty list, got {other:?}"),
    }
}
