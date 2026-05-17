//! `EditorState::apply` tests for the per-node-attribute commands
//! (flip / ellipse-arc / node-effects) — split out of
//! `command_tests.rs` to keep both files under the 800-line cap.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{ellipse, rect, state_with};
use crate::walkers::find_node;
use jian_ops_schema::node::PenNode;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

// --- SetNodeFlip -----------------------------------------------------

#[test]
fn set_node_flip_writes_both_axes() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::SetNodeFlip {
        node_id: id("n1"),
        flip_x: Some(true),
        flip_y: Some(true),
    }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().flip_x, Some(true));
    assert_eq!(n.base().flip_y, Some(true));
}

#[test]
fn set_node_flip_leaves_omitted_axis_untouched() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::SetNodeFlip {
        node_id: id("n1"),
        flip_x: Some(true),
        flip_y: None,
    }));
    let n = find_node(s.active_children(), &id("n1")).unwrap();
    assert_eq!(n.base().flip_x, Some(true));
    assert_eq!(n.base().flip_y, None);
}

#[test]
fn set_node_flip_rejects_empty_and_missing() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    // Nothing supplied → no-op rejection.
    assert!(!s.apply(EditorCommand::SetNodeFlip {
        node_id: id("n1"),
        flip_x: None,
        flip_y: None,
    }));
    // Unknown node → rejection.
    assert!(!s.apply(EditorCommand::SetNodeFlip {
        node_id: id("ghost"),
        flip_x: Some(true),
        flip_y: None,
    }));
}

// --- SetEllipseArc ---------------------------------------------------

#[test]
fn set_ellipse_arc_writes_pie_geometry() {
    let mut s = state_with(vec![ellipse("e1", "e", 0.0, 0.0, 40.0, 40.0)]);
    assert!(s.apply(EditorCommand::SetEllipseArc {
        node_id: id("e1"),
        start_angle: Some(0.0),
        sweep_angle: Some(270.0),
        inner_radius: Some(0.5),
    }));
    let n = find_node(s.active_children(), &id("e1")).unwrap();
    match n {
        PenNode::Ellipse(e) => {
            assert_eq!(e.start_angle, Some(0.0));
            assert_eq!(e.sweep_angle, Some(270.0));
            assert_eq!(e.inner_radius, Some(0.5));
        }
        _ => panic!("expected ellipse"),
    }
}

#[test]
fn set_ellipse_arc_rejects_non_ellipse() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::SetEllipseArc {
        node_id: id("n1"),
        start_angle: Some(0.0),
        sweep_angle: Some(90.0),
        inner_radius: None,
    }));
}

#[test]
fn set_ellipse_arc_rejects_out_of_range_inner_radius() {
    let mut s = state_with(vec![ellipse("e1", "e", 0.0, 0.0, 40.0, 40.0)]);
    // `inner_radius` is a 0.0..=1.0 fraction — 1.5 is rejected and the
    // node is left byte-for-byte unchanged.
    assert!(!s.apply(EditorCommand::SetEllipseArc {
        node_id: id("e1"),
        start_angle: None,
        sweep_angle: None,
        inner_radius: Some(1.5),
    }));
    let n = find_node(s.active_children(), &id("e1")).unwrap();
    match n {
        PenNode::Ellipse(e) => assert_eq!(e.inner_radius, None),
        _ => panic!("expected ellipse"),
    }
}

// --- AddNodeEffect / RemoveNodeEffect --------------------------------

#[test]
fn add_node_effect_appends_to_the_node() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 40.0, 40.0)]);
    assert!(s.apply(EditorCommand::AddNodeEffect {
        node_id: id("n1"),
        kind: "shadow".into(),
    }));
    assert!(s.apply(EditorCommand::AddNodeEffect {
        node_id: id("n1"),
        kind: "blur".into(),
    }));
    match find_node(s.active_children(), &id("n1")).unwrap() {
        PenNode::Rectangle(r) => {
            assert_eq!(r.container.effects.as_ref().map(|e| e.len()), Some(2));
        }
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn add_node_effect_rejects_unknown_kind() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::AddNodeEffect {
        node_id: id("n1"),
        kind: "glow".into(),
    }));
}

#[test]
fn remove_node_effect_drops_and_clears_when_empty() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::AddNodeEffect {
        node_id: id("n1"),
        kind: "shadow".into(),
    }));
    assert!(s.apply(EditorCommand::RemoveNodeEffect {
        node_id: id("n1"),
        index: 0,
    }));
    match find_node(s.active_children(), &id("n1")).unwrap() {
        // The list is cleared back to `None` once the last effect goes.
        PenNode::Rectangle(r) => assert!(r.container.effects.is_none()),
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn remove_node_effect_rejects_out_of_range() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(!s.apply(EditorCommand::RemoveNodeEffect {
        node_id: id("n1"),
        index: 0,
    }));
}
