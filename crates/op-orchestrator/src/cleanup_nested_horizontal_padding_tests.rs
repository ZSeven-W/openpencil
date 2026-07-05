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
fn nested_double_horizontal_padding_collapsed() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "header",
                    "name": "Header & Search",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "padding": [0, 20, 0, 20],
                    "children": [
                        {
                            "type": "frame",
                            "id": "wrapper",
                            "name": "Content Wrapper",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "padding": [16, 20, 0, 20],
                            "children": [
                                {
                                    "type": "frame",
                                    "id": "search",
                                    "name": "Search",
                                    "width": "fill_container",
                                    "height": 44,
                                    "layout": "horizontal",
                                    "children": []
                                }
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );

    collapse_nested_horizontal_padding(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "header")["padding"],
        json!([0.0, 20.0, 0.0, 20.0])
    );
    assert_eq!(
        node_json(&sink, "wrapper")["padding"],
        json!([16.0, 0.0, 0.0, 0.0])
    );
}

#[test]
fn single_padding_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "section",
                    "name": "Deals of the Week",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "padding": [0, 20, 0, 20],
                    "children": [
                        {
                            "type": "frame",
                            "id": "content",
                            "name": "Content",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    );

    let before = node_json(&sink, "root");
    collapse_nested_horizontal_padding(&mut sink, "root");

    assert_eq!(node_json(&sink, "root"), before);
}

#[test]
fn child_with_fill_not_collapsed() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "section",
                    "name": "Recently Viewed",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "padding": [0, 20, 0, 20],
                    "children": [
                        {
                            "type": "frame",
                            "id": "card",
                            "name": "Real Card",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "padding": [16, 20, 16, 20],
                            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    );

    collapse_nested_horizontal_padding(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "card")["padding"],
        json!([16.0, 20.0, 16.0, 20.0])
    );
}

#[test]
fn vertical_padding_preserved() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "section",
                    "name": "Header & Search",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "padding": [0, 20, 0, 20],
                    "children": [
                        {
                            "type": "frame",
                            "id": "wrapper",
                            "name": "Content Wrapper",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "padding": [24, 20, 12, 20],
                            "children": [
                                {
                                    "type": "frame",
                                    "id": "search",
                                    "name": "Search",
                                    "width": "fill_container",
                                    "height": 44,
                                    "layout": "horizontal",
                                    "children": []
                                }
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );

    collapse_nested_horizontal_padding(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "wrapper")["padding"],
        json!([24.0, 0.0, 12.0, 0.0])
    );
}
