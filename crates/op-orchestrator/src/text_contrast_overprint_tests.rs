use super::*;

fn fixture() -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.doc.variables = Some(palette());
    sink.state.doc.children = serde_json::from_value(json!([
        {"type":"frame","id":"board","width":800,"height":600,"fill":[{"type":"solid","color":"#F1EDE1"}],"children":[
            {"type":"frame","id":"effect","layout":"none","width":600,"height":180,"children":[
                {"type":"text","id":"ink","x":0,"y":0,"content":"咖啡小聚","fontSize":100,"fontFamily":"Noto Serif SC","fontWeight":700,"width":"fit_content","height":"fit_content","fill":[{"type":"solid","color":"#1E1A16"}]},
                {"type":"text","id":"red","x":3,"y":3,"content":"咖啡小聚","fontSize":100,"fontFamily":"Noto Serif SC","fontWeight":700,"width":"fit_content","height":"fit_content","fill":[{"type":"solid","color":"#B74A37"}]}
            ]}
        ]}
    ])).unwrap();
    sink
}

#[test]
fn readable_foreground_protects_a_decorative_offset_copy_from_recoloring() {
    let mut sink = fixture();
    let original = sink.state.doc.clone();
    assert_eq!(repair_text_contrast(&mut sink, "board"), 0);
    assert_eq!(sink.state.doc, original);
}

#[test]
fn unreadable_or_transparent_foreground_cannot_protect_its_decoration() {
    for fill in [
        json!([{"type":"solid","color":"#FFFFFF"}]),
        json!([{"type":"solid","color":"#1E1A1600"}]),
    ] {
        let mut sink = fixture();
        let node = op_editor_core::walkers::find_node_mut(
            sink.state.active_children_mut(),
            &NodeId::new("ink"),
        )
        .unwrap();
        let mut value = serde_json::to_value(&*node).unwrap();
        value["fill"] = fill;
        *node = serde_json::from_value(value).unwrap();
        assert!(repair_text_contrast(&mut sink, "board") > 0);
        let value = serde_json::to_value(
            op_editor_core::walkers::find_node(sink.state.active_children(), &NodeId::new("red"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(value["fill"][0]["color"], "$--foreground");
    }
}

#[test]
fn different_reading_copy_keeps_its_own_contrast_requirement() {
    let mut sink = fixture();
    let PenNode::Text(red) = op_editor_core::walkers::find_node_mut(
        sink.state.active_children_mut(),
        &NodeId::new("red"),
    )
    .unwrap() else {
        panic!()
    };
    red.content = jian_ops_schema::node::TextContent::Plain("另一段正文".into());
    assert_eq!(repair_text_contrast(&mut sink, "board"), 1);
}
