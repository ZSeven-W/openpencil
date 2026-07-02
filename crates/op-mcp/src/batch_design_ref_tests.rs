//! Tests for `batch_design` emitting component instances (`ref` nodes).
//!
//! A `batch_design` operations DSL item with `type:"ref"` must deserialize
//! to a `PenNode::Ref` (the serde tag seam), survive the insert command
//! unchanged (`target` + `descendants` are not id-remapped), and expand to
//! the master's subtree via `resolve_refs_for_canvas` at render time.

use super::test_fixtures::{frame, state_with, text};
use super::{EditorCommand, McpTool, ToolOutcome};
use crate::batch_design_snapshot;
use jian_ops_schema::node::PenNode;
use op_editor_core::ref_resolve::resolve_refs_for_canvas;
use std::collections::BTreeMap;

/// A reusable master Frame `comp-card` (`reusable: true`) holding a Title
/// text and a Subtitle text, as the doc the ref will target.
fn doc_with_master() -> op_editor_core::EditorState {
    let title = text("c-title", "Title", 0.0, 0.0, 180.0, 24.0, "Master Title");
    let subtitle = text(
        "c-sub",
        "Subtitle",
        0.0,
        28.0,
        180.0,
        16.0,
        "Master Subtitle",
    );
    let mut master = frame(
        "comp-card",
        "Card",
        0.0,
        0.0,
        200.0,
        100.0,
        vec![title, subtitle],
    );
    if let PenNode::Frame(f) = &mut master {
        f.reusable = Some(true);
    }
    state_with(vec![master])
}

#[test]
fn batch_design_ref_operation_deserializes_to_pen_node_ref() {
    let tool = batch_design_snapshot(&doc_with_master());
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"inst=I(null, {"type":"ref","name":"Card Instance","ref":"comp-card","x":300,"y":50})"##
            .into(),
    );

    let (json, cmd) = match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(json, cmd) => (json, cmd),
        other => panic!("expected OkJsonWithCommand, got {other:?}"),
    };
    let (nodes, parent_id) = match cmd {
        EditorCommand::InsertAuthoredSubtree {
            nodes, parent_id, ..
        } => (nodes, parent_id),
        other => panic!("expected InsertAuthoredSubtree, got {other:?}"),
    };
    assert!(!parent_id.is_real(), "ref inserts at the page root");
    assert_eq!(nodes.len(), 1);
    let PenNode::Ref(reference) = &nodes[0] else {
        panic!(
            "type:\"ref\" must deserialize to PenNode::Ref, got {:?}",
            nodes[0]
        );
    };
    // The ref's `target` survives id remapping (only `base.id` is remapped).
    assert_eq!(reference.target, "comp-card");
    assert_eq!(reference.base.x, Some(300.0));
    assert_eq!(reference.base.y, Some(50.0));
    // The tool reports the instance under its binding (the host-assigned id).
    let v: serde_json::Value = serde_json::from_str(&json).expect("json result");
    assert_eq!(v["results"][0]["binding"], "inst");
}

#[test]
fn batch_design_ref_expands_to_master_subtree_on_canvas() {
    let mut state = doc_with_master();
    let tool = batch_design_snapshot(&state);
    let mut args = BTreeMap::new();
    args.insert(
        "operations".into(),
        r##"inst=I(null, {"type":"ref","name":"Card Instance","ref":"comp-card","x":300,"y":50,"descendants":{"c-title":{"content":"Instance Title"}}})"##
            .into(),
    );

    let cmd = match tool.call(&args) {
        ToolOutcome::OkJsonWithCommand(_, cmd) => cmd,
        other => panic!("expected command, got {other:?}"),
    };
    // Apply against the same doc the snapshot was taken from — the master
    // (`comp-card`) is still present so the ref resolves.
    assert!(state.apply(cmd), "ref subtree inserts");

    // The inserted ref is the last root; the master is the first.
    let inserted = state.doc.children.last().expect("inserted ref root");
    assert!(
        matches!(inserted, PenNode::Ref(_)),
        "the stored node is an unresolved PenNode::Ref, got {inserted:?}"
    );

    // resolve_refs_for_canvas expands the ref into the master's subtree.
    let resolved = resolve_refs_for_canvas(&state.doc);
    let expanded = resolved.children.last().expect("expanded instance root");
    let PenNode::Frame(frame) = expanded else {
        panic!("ref expands to the master's Frame type, got {expanded:?}");
    };
    // It carries the instance position, the master's size, and a non-empty
    // remapped subtree.
    assert_eq!(frame.base.x, Some(300.0));
    assert_eq!(frame.base.y, Some(50.0));
    assert_eq!(frame.reusable, None, "expanded instance is not a master");
    let children = frame.children.as_ref().expect("master subtree expands");
    assert_eq!(children.len(), 2, "both master children are present");
    // descendants override landed on the matching (remapped) child.
    let PenNode::Text(title) = &children[0] else {
        panic!("first child is the title text, got {:?}", children[0]);
    };
    assert_eq!(
        title.content,
        jian_ops_schema::node::TextContent::Plain("Instance Title".into()),
        "descendants override applied to the expanded child"
    );
    assert!(
        title.base.id.ends_with("__c-title"),
        "descendant id is remapped into the instance virtual space, got {}",
        title.base.id
    );
}
