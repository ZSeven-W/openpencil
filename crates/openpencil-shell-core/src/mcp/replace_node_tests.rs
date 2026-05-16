//! Tests for `mcp::write_tools::ReplaceNode` + the matching
//! `Document::apply_mcp_command(ReplaceNode)` branch. Sibling
//! file (same rationale as `copy_node_tests.rs`) — keeps
//! `write_tools_tests.rs` under the 800-line cap as the write
//! surface grows.

use super::write_tools::*;
use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};
use std::collections::BTreeMap;

#[test]
fn replace_node_validates_required_args() {
    let tool = replace_node_snapshot();
    match tool.call(&BTreeMap::new()) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
        _ => panic!(),
    }
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "11".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, msg) => {
            assert_eq!(code, ToolErrorCode::MissingArgument);
            assert!(msg.contains("kind"));
        }
        _ => panic!(),
    }
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "Replacement".into());
    args.insert("x".into(), "10".into());
    args.insert("y".into(), "20".into());
    args.insert("width".into(), "100".into());
    args.insert("height".into(), "30".into());
    match tool.call(&args) {
        ToolOutcome::OkWithCommand(_, McpCommand::ReplaceNode { node_id, kind, name, x, y, width, height, fill_hex, drop_children }) => {
            assert_eq!(node_id, 11);
            assert_eq!(kind, "rect");
            assert_eq!(name, "Replacement");
            assert_eq!(x, 10);
            assert_eq!(y, 20);
            assert_eq!(width, 100);
            assert_eq!(height, 30);
            assert!(fill_hex.is_none());
            assert!(!drop_children, "default opt-out keeps container subtrees safe");
        }
        other => panic!("expected ReplaceNode, got {other:?}"),
    }
}

#[test]
fn replace_node_rejects_bad_kind_and_geometry_and_fill() {
    let tool = replace_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "11".into());
    args.insert("kind".into(), "blob".into()); // not in ALLOWED_KINDS
    args.insert("name".into(), "X".into());
    args.insert("x".into(), "0".into());
    args.insert("y".into(), "0".into());
    args.insert("width".into(), "10".into());
    args.insert("height".into(), "10".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
    args.insert("kind".into(), "rect".into());
    args.insert("width".into(), "-5".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
    args.insert("width".into(), "5".into());
    args.insert("fill_hex".into(), "not-a-hex".into());
    match tool.call(&args) {
        ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::InvalidArgument),
        _ => panic!(),
    }
}

#[test]
fn apply_mcp_command_replace_node_swaps_at_same_slot() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_max = doc.max_node_id();
    // Title (id 11) is at frame.children[0]. Replace it with a Rect.
    let cmd = McpCommand::ReplaceNode {
        node_id: 11,
        kind: "rect".into(),
        name: "Swapped".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: Some("#ff0000".into()),
        // Title is a leaf — drop_children doesn't matter, but
        // exercise the safe default to prove leaves still swap.
        drop_children: false,
    };
    assert!(doc.apply_mcp_command(&cmd));
    let frame = doc.pages[0].children.iter().find(|n| n.id.raw() == "n10").unwrap();
    // Slot 0 now holds the replacement; old id is gone.
    assert!(frame.children.iter().all(|n| n.id.raw() != "n11"));
    let swapped = &frame.children[0];
    assert_eq!(swapped.name, "Swapped");
    let swapped_n = swapped.id.raw()[1..].parse::<u64>().unwrap();
    assert!(swapped_n > pre_max, "fresh id past pre_max");
    // Sibling (Button group, id 12) still at slot 1.
    assert_eq!(frame.children[1].id.raw(), "n12");
}

#[test]
fn apply_mcp_command_replace_node_rejects_unknown_id() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_root_len = doc.pages[0].children.len();
    let cmd = McpCommand::ReplaceNode {
        node_id: 99999,
        kind: "rect".into(),
        name: "Ghost".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: None,
        drop_children: false,
    };
    assert!(!doc.apply_mcp_command(&cmd));
    // Doc unchanged.
    assert_eq!(doc.pages[0].children.len(), pre_root_len);
}

#[test]
fn apply_mcp_command_replace_node_atomic_on_invalid_fill_hex() {
    // The applier pre-validates fill_hex BEFORE allocating the
    // fresh id or mutating pages (same atomicity pattern as
    // update_node + move_node).
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_max = doc.max_node_id();
    let cmd = McpCommand::ReplaceNode {
        node_id: 11,
        kind: "rect".into(),
        name: "WouldFail".into(),
        x: 0,
        y: 0,
        width: 10,
        height: 10,
        fill_hex: Some("not-hex".into()),
        drop_children: false,
    };
    assert!(!doc.apply_mcp_command(&cmd));
    // Title still in place under Frame.
    let frame = doc.pages[0].children.iter().find(|n| n.id.raw() == "n10").unwrap();
    assert!(frame.children.iter().any(|n| n.id.raw() == "n11"));
    // No id was allocated.
    assert_eq!(doc.max_node_id(), pre_max);
}

#[test]
fn apply_mcp_command_replace_node_refuses_to_drop_container_children() {
    // Frame (id 10) carries Title + Button under it. Replacing
    // it without explicit `drop_children=true` MUST refuse — the
    // tool would otherwise silently delete the subtree.
    use crate::document::Document;
    let mut doc = Document::sample();
    let pre_max = doc.max_node_id();
    let pre_frame_children = doc.pages[0].children[0].children.len();
    let cmd = McpCommand::ReplaceNode {
        node_id: 10, // Frame container
        kind: "rect".into(),
        name: "WouldNuke".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: None,
        drop_children: false,
    };
    assert!(!doc.apply_mcp_command(&cmd), "container swap must refuse without consent");
    // Frame intact with both children.
    let frame = doc.pages[0].children.iter().find(|n| n.id.raw() == "n10").unwrap();
    assert_eq!(frame.children.len(), pre_frame_children);
    // No fresh id allocated.
    assert_eq!(doc.max_node_id(), pre_max);
}

#[test]
fn apply_mcp_command_replace_node_drops_container_children_when_opted_in() {
    use crate::document::Document;
    let mut doc = Document::sample();
    let cmd = McpCommand::ReplaceNode {
        node_id: 10, // Frame container — has children
        kind: "rect".into(),
        name: "ExplicitNuke".into(),
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill_hex: None,
        drop_children: true,
    };
    assert!(doc.apply_mcp_command(&cmd), "container swap with consent must succeed");
    // Frame id is gone; a fresh leaf sits at page root with no children.
    let root = &doc.pages[0].children;
    assert!(root.iter().all(|n| n.id.raw() != "n10"), "old frame replaced");
    assert_eq!(root.len(), 1, "replacement landed at the same slot");
    assert_eq!(root[0].name, "ExplicitNuke");
    assert!(root[0].children.is_empty(), "replacement is a leaf");
}

#[test]
fn replace_node_rejects_malformed_drop_children() {
    let tool = replace_node_snapshot();
    let mut args = BTreeMap::new();
    args.insert("node_id".into(), "11".into());
    args.insert("kind".into(), "rect".into());
    args.insert("name".into(), "X".into());
    args.insert("x".into(), "0".into());
    args.insert("y".into(), "0".into());
    args.insert("width".into(), "10".into());
    args.insert("height".into(), "10".into());
    for bad in ["yes", "", "TRUE", "1", "0"] {
        args.insert("drop_children".into(), bad.into());
        match tool.call(&args) {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument, "input {bad:?}");
                assert!(msg.contains("drop_children"), "input {bad:?} msg = {msg}");
            }
            _ => panic!("expected InvalidArgument on malformed drop_children {bad:?}"),
        }
    }
}

#[test]
fn parser_refuses_structured_drop_children_at_wire_layer() {
    // End-to-end coverage for codex's parser-layer concern:
    // a structured `drop_children` (or any other arg) MUST NOT
    // reach the tool layer at all. The parser refuses to build
    // a ToolCall so neither ReplaceNode::call nor the applier
    // gets a chance to misinterpret it. Without the wire-level
    // reject, a sentinel approach would collide with legitimate
    // scalars (e.g. a variable literally named "{...}").
    use crate::mcp::parse_tool_call;
    for bad_json in [
        r#"{"id":1,"method":"replace_node","params":{"node_id":"10","kind":"rect","name":"X","x":"0","y":"0","width":"5","height":"5","drop_children":{}}}"#,
        r#"{"id":1,"method":"replace_node","params":{"node_id":"10","kind":"rect","name":"X","x":"0","y":"0","width":"5","height":"5","drop_children":[true]}}"#,
        r#"{"id":1,"method":"replace_node","params":{"node_id":"10","kind":"rect","name":{},"x":"0","y":"0","width":"5","height":"5"}}"#,
    ] {
        assert!(
            parse_tool_call(bad_json).is_none(),
            "structured arg must refuse parse: {bad_json}"
        );
    }
}
