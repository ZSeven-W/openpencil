#![cfg(test)]

use crate::fills::node_stroke_width;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenStroke, SidedThickness, StrokeThickness};

/// Removing a fill must also drop the node's `fill_refs` variable binding.
/// Otherwise the scene resolver's `fill_for` (a registered fill ref wins
/// over `container.fill`) keeps painting the variable colour after the
/// fill row is gone — the "deleted the fill but the colour stays" bug on
/// token-based (old .op) designs.
#[test]
fn remove_selected_fill_clears_the_variable_ref() {
    use crate::node_id::NodeId;
    use crate::test_support::{rect, state_with};
    use crate::walkers::{find_node, find_node_mut};

    let mut s = state_with(vec![rect("n1", "r", 0.0, 0.0, 10.0, 10.0)]);
    s.set_single_selection(NodeId::new("n1"));
    // Mirror a `$ref` fill: bind writes `$name` into fill[0] + fill_refs.
    let node = find_node_mut(s.active_children_mut(), &NodeId::new("n1")).unwrap();
    crate::fills::set_primary_fill_hex(node, "$color-info-bg");
    s.ui
        .variables
        .fill_refs
        .insert(NodeId::new("n1"), "color-info-bg".to_string());

    assert!(s.remove_selected_fill(0), "remove must report success");

    let node = find_node(s.active_children(), &NodeId::new("n1")).unwrap();
    assert!(
        crate::fills::node_fills(node)
            .map(|f| f.is_empty())
            .unwrap_or(true),
        "container.fill must be cleared"
    );
    assert!(
        !s.ui.variables.fill_refs.contains_key(&NodeId::new("n1")),
        "fill_ref must clear too, else fill_for keeps painting the variable colour"
    );
}

#[test]
fn node_stroke_width_reads_max_sided_edge() {
    let mut node = rect_node();
    let PenNode::Rectangle(r) = &mut node else {
        panic!("rectangle");
    };
    r.container.stroke = Some(PenStroke {
        thickness: StrokeThickness::Sided(SidedThickness {
            bottom: Some(3.0),
            ..Default::default()
        }),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: None,
    });

    assert_eq!(node_stroke_width(&node), Some(3.0));
}

fn rect_node() -> PenNode {
    let src = r#"{"version":"0.8.0","children":[
        {"type":"rectangle","id":"r1","name":"R",
         "x":0,"y":0,"width":10,"height":10}
    ]}"#;
    jian_ops_schema::load_str(src)
        .expect("fixture parses")
        .value
        .children
        .into_iter()
        .next()
        .expect("one node")
}
