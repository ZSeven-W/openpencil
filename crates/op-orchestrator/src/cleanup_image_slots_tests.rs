//! Tests for `cleanup_image_slots::materialize_empty_image_fill_slots`:
//! the weak-model "image slot" shape (childless frame/rect + one empty image
//! fill) becomes a real `PenNode::Image` in place; everything else stays
//! untouched.

use super::*;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::style::PenFill;

fn insert_tree(sink: &mut VecDocSink, json: &str) {
    let tree: PenNode = serde_json::from_str(json).expect("test tree json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

fn find_active_node<'a>(sink: &'a VecDocSink, id: &str) -> &'a PenNode {
    sink.state
        .active_children()
        .iter()
        .find_map(|node| find_node(node, id))
        .expect("node exists")
}

#[test]
fn childless_rect_with_empty_image_fill_becomes_image() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "rectangle",
                    "id": "photo",
                    "name": "Album Cover",
                    "width": 240,
                    "height": 160,
                    "cornerRadius": 12,
                    "fill": [{ "type": "image", "url": "" }]
                }
            ]
        }"##,
    );

    materialize_empty_image_fill_slots(&mut sink, "root");

    let PenNode::Image(image) = find_active_node(&sink, "photo") else {
        panic!("expected image node after materialization");
    };
    assert_eq!(image.base.id, "photo");
    assert_eq!(image.base.name.as_deref(), Some("Album Cover"));
    assert_eq!(image.width, Some(SizingBehavior::Number(240.0)));
    assert_eq!(image.height, Some(SizingBehavior::Number(160.0)));
    assert_eq!(
        image.corner_radius,
        Some(CornerRadius::Uniform(12.0)),
        "the slot's corner radius must survive the conversion"
    );
    assert_eq!(image.src, "", "src stays empty for the enrichment pipeline");
    assert_eq!(image.image_search_query, None);
    assert_eq!(image.image_prompt, None);
    assert!(
        sink.applied.iter().any(|cmd| matches!(
            cmd,
            EditorCommand::PatchNodeData { node_id, .. } if node_id.as_str() == "photo"
        )),
        "the conversion must go through the sink command path"
    );
}

#[test]
fn frame_with_children_and_empty_image_fill_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "bg",
                    "name": "Background",
                    "width": "fill_container",
                    "height": 200,
                    "fill": [{ "type": "image", "url": "" }],
                    "children": [
                        { "type": "text", "id": "t", "content": "Hero copy" }
                    ]
                }
            ]
        }"##,
    );

    materialize_empty_image_fill_slots(&mut sink, "root");

    let PenNode::Frame(frame) = find_active_node(&sink, "bg") else {
        panic!("a background-image container is a design, not a slot");
    };
    assert_eq!(frame.children.as_ref().map(Vec::len), Some(1));
    assert!(
        sink.applied.iter().all(
            |cmd| !matches!(cmd, EditorCommand::PatchNodeData { node_id, .. }
                if node_id.as_str() == "bg")
        ),
        "no command may touch the background-image frame"
    );
}

#[test]
fn non_empty_image_fill_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "rectangle",
                    "id": "photo",
                    "name": "Album Cover",
                    "width": 240,
                    "height": 160,
                    "fill": [{ "type": "image", "url": "https://example.com/photo.jpg" }]
                }
            ]
        }"##,
    );

    materialize_empty_image_fill_slots(&mut sink, "root");

    let PenNode::Rectangle(rect) = find_active_node(&sink, "photo") else {
        panic!("a landed image fill must stay a rectangle");
    };
    let Some([PenFill::Image(body)]) = rect.container.fill.as_deref() else {
        panic!("expected the single image fill");
    };
    assert_eq!(body.url, "https://example.com/photo.jpg");
}

#[test]
fn multi_fill_stack_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "rectangle",
                    "id": "photo",
                    "name": "Album Cover",
                    "width": 240,
                    "height": 160,
                    "fill": [
                        { "type": "image", "url": "" },
                        { "type": "solid", "color": "#E5E7EB" }
                    ]
                }
            ]
        }"##,
    );

    materialize_empty_image_fill_slots(&mut sink, "root");

    let PenNode::Rectangle(rect) = find_active_node(&sink, "photo") else {
        panic!("a multi-fill stack is not a slot");
    };
    assert_eq!(rect.container.fill.as_ref().map(Vec::len), Some(2));
}
