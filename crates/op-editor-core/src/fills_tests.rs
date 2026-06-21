#![cfg(test)]

use crate::fills::node_stroke_width;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenStroke, SidedThickness, StrokeThickness};

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
