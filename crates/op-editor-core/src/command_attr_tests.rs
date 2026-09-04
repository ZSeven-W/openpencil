//! `EditorState::apply` tests for the per-node-attribute commands
//! (flip / ellipse-arc / node-effects) — split out of
//! `command_tests.rs` to keep both files under the 800-line cap.

#![cfg(test)]

use crate::command::{EditorCommand, LayoutPropValue};
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{ellipse, rect, state_with, text};
use crate::walkers::find_node;
use jian_ops_schema::node::container::{AlignItems, JustifyContent, LayoutMode, Padding};
use jian_ops_schema::node::text::{TextAlign, TextAlignVertical, TextGrowth};
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

#[test]
fn set_effect_param_writes_shadow_offset() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::AddNodeEffect {
        node_id: id("n1"),
        kind: "shadow".into(),
    }));
    assert!(s.apply(EditorCommand::SetEffectParam {
        node_id: id("n1"),
        index: 0,
        field: crate::EffectField::OffsetX,
        value: 12.0,
    }));
    match find_node(s.active_children(), &id("n1")).unwrap() {
        PenNode::Rectangle(r) => match &r.container.effects.as_ref().unwrap()[0] {
            jian_ops_schema::style::PenEffect::Shadow(sh) => assert_eq!(sh.offset_x, 12.0),
            other => panic!("expected shadow, got {other:?}"),
        },
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn set_effect_param_clamps_blur_radius_to_non_negative() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::AddNodeEffect {
        node_id: id("n1"),
        kind: "blur".into(),
    }));
    assert!(s.apply(EditorCommand::SetEffectParam {
        node_id: id("n1"),
        index: 0,
        field: crate::EffectField::Radius,
        value: -9.0,
    }));
    match find_node(s.active_children(), &id("n1")).unwrap() {
        PenNode::Rectangle(r) => match &r.container.effects.as_ref().unwrap()[0] {
            jian_ops_schema::style::PenEffect::Blur(b) => assert_eq!(b.radius, 0.0),
            other => panic!("expected blur, got {other:?}"),
        },
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn set_effect_param_rejects_field_effect_mismatch() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.apply(EditorCommand::AddNodeEffect {
        node_id: id("n1"),
        kind: "blur".into(),
    }));
    // OffsetX is a Shadow-only field — rejected on a Blur effect.
    assert!(!s.apply(EditorCommand::SetEffectParam {
        node_id: id("n1"),
        index: 0,
        field: crate::EffectField::OffsetX,
        value: 5.0,
    }));
}

// --- SetNodeLayoutProp ----------------------------------------------

#[test]
fn set_node_layout_prop_writes_container_layout_fields() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    for (property, value) in [
        ("layout", LayoutPropValue::Keyword("horizontal".into())),
        (
            "justifyContent",
            LayoutPropValue::Keyword("space_between".into()),
        ),
        ("alignItems", LayoutPropValue::Keyword("center".into())),
        ("gap", LayoutPropValue::Number(12.0)),
        ("x", LayoutPropValue::Number(3.5)),
        ("y", LayoutPropValue::Number(7.5)),
        (
            "padding",
            LayoutPropValue::NumberArray(vec![1.0, 2.0, 3.0, 4.0]),
        ),
        ("clipContent", LayoutPropValue::Bool(true)),
    ] {
        assert!(s.apply(EditorCommand::SetNodeLayoutProp {
            node_id: id("n1"),
            property: property.into(),
            value,
        }));
    }
    match find_node(s.active_children(), &id("n1")).unwrap() {
        PenNode::Rectangle(r) => {
            assert_eq!(r.base.x, Some(3.5));
            assert_eq!(r.base.y, Some(7.5));
            assert_eq!(r.container.layout, Some(LayoutMode::Horizontal));
            assert_eq!(
                r.container.justify_content,
                Some(JustifyContent::SpaceBetween)
            );
            assert_eq!(r.container.align_items, Some(AlignItems::Center));
            assert_eq!(
                r.container.gap,
                Some(jian_ops_schema::node::base::NumberOrExpression::Number(
                    12.0
                ))
            );
            assert_eq!(
                r.container.padding,
                Some(Padding::LtrB([1.0, 2.0, 3.0, 4.0]))
            );
            assert_eq!(r.container.clip_content, Some(true));
        }
        other => panic!("expected rect, got {other:?}"),
    }
}

#[test]
fn undo_layout_mutation_restores_preserve_geometry() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.editor_ui.preserve_authored_geometry = true;
    s.commit_history();

    assert!(s.apply(EditorCommand::SetNodeLayoutProp {
        node_id: id("n1"),
        property: "gap".into(),
        value: LayoutPropValue::Number(12.0),
    }));
    assert!(!s.editor_ui.preserve_authored_geometry);

    assert!(s.undo());
    assert!(s.editor_ui.preserve_authored_geometry);
    assert!(s.redo());
    assert!(!s.editor_ui.preserve_authored_geometry);
}

#[test]
fn set_node_layout_prop_writes_text_specific_fields() {
    let mut s = state_with(vec![text("t1", "t", 0.0, 0.0, 100.0, 40.0, "hello")]);
    for (property, value) in [
        ("fontFamily", LayoutPropValue::Keyword("Inter".into())),
        ("textAlign", LayoutPropValue::Keyword("justify".into())),
        (
            "textAlignVertical",
            LayoutPropValue::Keyword("middle".into()),
        ),
        (
            "textGrowth",
            LayoutPropValue::Keyword("fixed-width-height".into()),
        ),
        ("lineHeight", LayoutPropValue::Number(1.4)),
        ("letterSpacing", LayoutPropValue::Number(2.0)),
    ] {
        assert!(s.apply(EditorCommand::SetNodeLayoutProp {
            node_id: id("t1"),
            property: property.into(),
            value,
        }));
    }
    match find_node(s.active_children(), &id("t1")).unwrap() {
        PenNode::Text(t) => {
            assert_eq!(t.font_family.as_deref(), Some("Inter"));
            assert_eq!(t.text_align, Some(TextAlign::Justify));
            assert_eq!(t.text_align_vertical, Some(TextAlignVertical::Middle));
            assert_eq!(t.text_growth, Some(TextGrowth::FixedWidthHeight));
            assert_eq!(t.line_height, Some(1.4));
            assert_eq!(t.letter_spacing, Some(2.0));
        }
        other => panic!("expected text, got {other:?}"),
    }
}
