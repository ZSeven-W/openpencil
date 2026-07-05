use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

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

fn node_json(sink: &VecDocSink, id: &str) -> Value {
    serde_json::to_value(find_active_node(sink, id)).expect("serialize node")
}

#[test]
fn absolute_fit_content_wrapper_expands_to_image_height() {
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
                    "id": "image-wrapper",
                    "name": "Image Wrapper",
                    "width": 210,
                    "height": "fit_content",
                    "layout": "none",
                    "children": [
                        {
                            "type": "image",
                            "id": "product-image",
                            "name": "Product Image",
                            "src": "",
                            "width": 210,
                            "height": 240
                        }
                    ]
                }
            ]
        }"##,
    );

    expand_absolute_container_to_children(&mut sink, "root");

    assert_eq!(node_json(&sink, "image-wrapper")["height"], json!(240.0));
}

#[test]
fn absolute_wrapper_with_y_offset_uses_y_plus_height() {
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
                    "id": "image-wrapper",
                    "name": "Image Wrapper",
                    "width": 210,
                    "height": "fit_content",
                    "layout": "none",
                    "children": [
                        {
                            "type": "image",
                            "id": "product-image",
                            "name": "Product Image",
                            "src": "",
                            "y": 10,
                            "width": 210,
                            "height": 240
                        }
                    ]
                }
            ]
        }"##,
    );

    expand_absolute_container_to_children(&mut sink, "root");

    assert_eq!(node_json(&sink, "image-wrapper")["height"], json!(250.0));
}

#[test]
fn layout_none_container_without_image_child_untouched() {
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
                    "id": "guest-counter",
                    "name": "Guest Counter",
                    "width": 112,
                    "height": "fit_content",
                    "layout": "none",
                    "children": [
                        {
                            "type": "icon_font",
                            "id": "minus-icon",
                            "name": "Minus Icon",
                            "iconFontName": "minus",
                            "y": 200,
                            "width": 20,
                            "height": 20
                        },
                        {
                            "type": "text",
                            "id": "guest-count",
                            "name": "Guest Count",
                            "content": "2",
                            "y": 212,
                            "width": 16,
                            "height": 20
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "guest-counter");

    expand_absolute_container_to_children(&mut sink, "root");

    assert_eq!(node_json(&sink, "guest-counter"), before);
}

#[test]
fn flex_fit_content_untouched() {
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
                    "id": "image-wrapper",
                    "name": "Image Wrapper",
                    "width": 210,
                    "height": "fit_content",
                    "layout": "vertical",
                    "children": [
                        {
                            "type": "image",
                            "id": "product-image",
                            "name": "Product Image",
                            "src": "",
                            "width": 210,
                            "height": 240
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "image-wrapper");

    expand_absolute_container_to_children(&mut sink, "root");

    assert_eq!(node_json(&sink, "image-wrapper"), before);
}

#[test]
fn absolute_wrapper_with_numeric_height_untouched() {
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
                    "id": "status-bar",
                    "name": "Status Bar",
                    "width": "fill_container",
                    "height": 62,
                    "layout": "none",
                    "children": [
                        {
                            "type": "frame",
                            "id": "status-content",
                            "name": "Status Content",
                            "width": 210,
                            "height": 240,
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "status-bar");

    expand_absolute_container_to_children(&mut sink, "root");

    assert_eq!(node_json(&sink, "status-bar"), before);
}

#[test]
fn absolute_wrapper_only_fill_children_untouched() {
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
                    "id": "image-wrapper",
                    "name": "Image Wrapper",
                    "width": 210,
                    "height": "fit_content",
                    "layout": "none",
                    "children": [
                        {
                            "type": "frame",
                            "id": "fill-child",
                            "name": "Fill Child",
                            "width": "fill_container",
                            "height": "fill_container",
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "image-wrapper");

    expand_absolute_container_to_children(&mut sink, "root");

    assert_eq!(node_json(&sink, "image-wrapper"), before);
}
