#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::test_support::{rect, state_with};
use crate::walkers::{find_node, find_node_mut};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenStroke, SidedThickness, StrokeThickness};

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn set_node_stroke_width_preserves_sided_edges() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    let node = find_node_mut(s.active_children_mut(), &id("n1")).unwrap();
    let PenNode::Rectangle(r) = node else {
        panic!("rectangle");
    };
    r.container.stroke = Some(PenStroke {
        thickness: StrokeThickness::Sided(SidedThickness {
            bottom: Some(1.0),
            ..Default::default()
        }),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: None,
    });

    assert!(s.apply(EditorCommand::SetNodeStrokeWidth {
        node_id: id("n1"),
        width: 6.0,
    }));

    let PenNode::Rectangle(r) = find_node(s.active_children(), &id("n1")).unwrap() else {
        panic!("rectangle");
    };
    let stroke = r.container.stroke.as_ref().expect("stroke");
    assert!(matches!(
        &stroke.thickness,
        StrokeThickness::Sided(sides)
            if sides.top.is_none()
                && sides.right.is_none()
                && sides.bottom == Some(6.0)
                && sides.left.is_none()
    ));
}

#[test]
fn commit_property_edit_writes_one_stroke_edge() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    let node = find_node_mut(s.active_children_mut(), &id("n1")).unwrap();
    let PenNode::Rectangle(r) = node else {
        panic!("rectangle");
    };
    r.container.stroke = Some(PenStroke {
        thickness: StrokeThickness::Uniform(1.0),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: None,
    });

    s.set_single_selection(id("n1"));
    s.editor_ui.stroke_edit_mode = Some(crate::PaddingEditMode::Individual);
    assert!(s.commit_property_edit(crate::PropertyFocus::StrokeRightWidth, 3.0));

    let PenNode::Rectangle(r) = find_node(s.active_children(), &id("n1")).unwrap() else {
        panic!("rectangle");
    };
    let stroke = r.container.stroke.as_ref().expect("stroke");
    assert!(matches!(
        &stroke.thickness,
        StrokeThickness::Sided(sides)
            if sides.top == Some(1.0)
                && sides.right == Some(3.0)
                && sides.bottom == Some(1.0)
                && sides.left == Some(1.0)
    ));
}

#[test]
fn commit_property_edit_single_stroke_mode_writes_all_edges() {
    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    let node = find_node_mut(s.active_children_mut(), &id("n1")).unwrap();
    let PenNode::Rectangle(r) = node else {
        panic!("rectangle");
    };
    r.container.stroke = Some(PenStroke {
        thickness: StrokeThickness::Uniform(1.0),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: None,
    });

    s.set_single_selection(id("n1"));
    assert!(s.commit_property_edit(crate::PropertyFocus::StrokeTopWidth, 3.0));

    let PenNode::Rectangle(r) = find_node(s.active_children(), &id("n1")).unwrap() else {
        panic!("rectangle");
    };
    let stroke = r.container.stroke.as_ref().expect("stroke");
    assert!(matches!(
        &stroke.thickness,
        StrokeThickness::Sided(sides)
            if sides.top == Some(3.0)
                && sides.right == Some(3.0)
                && sides.bottom == Some(3.0)
                && sides.left == Some(3.0)
    ));
}
