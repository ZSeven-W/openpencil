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
fn two_fill_width_fit_height_cards_in_row_become_fill_height() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Page",
            "width": 390,
            "height": "fit_content",
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "deals-row",
                    "name": "Deals of the Week",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "horizontal",
                    "children": [
                        {
                            "type": "frame",
                            "id": "card-a",
                            "name": "Deal Card A",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                {"type": "text", "id": "card-a-title", "content": "Weekend Bento"}
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "card-b",
                            "name": "Deal Card B",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                {"type": "text", "id": "card-b-title", "content": "Family feast with extra dessert"}
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );

    equalize_horizontal_card_heights(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "card-a")["height"],
        json!("fill_container")
    );
    assert_eq!(
        node_json(&sink, "card-b")["height"],
        json!("fill_container")
    );
}

#[test]
fn single_card_row_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Page",
            "width": 390,
            "height": "fit_content",
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "deals-row",
                    "name": "Deals of the Week",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "horizontal",
                    "children": [
                        {
                            "type": "frame",
                            "id": "card-a",
                            "name": "Deal Card A",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                {"type": "text", "id": "card-a-title", "content": "Weekend Bento"}
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "root");

    equalize_horizontal_card_heights(&mut sink, "root");

    assert_eq!(node_json(&sink, "root"), before);
}

#[test]
fn mixed_width_row_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Page",
            "width": 390,
            "height": "fit_content",
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "deals-row",
                    "name": "Deals of the Week",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "horizontal",
                    "children": [
                        {
                            "type": "frame",
                            "id": "card-a",
                            "name": "Deal Card A",
                            "width": "fill_container",
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                {"type": "text", "id": "card-a-title", "content": "Weekend Bento"}
                            ]
                        },
                        {
                            "type": "frame",
                            "id": "card-b",
                            "name": "Deal Card B",
                            "width": 180,
                            "height": "fit_content",
                            "layout": "vertical",
                            "children": [
                                {"type": "text", "id": "card-b-title", "content": "Family feast"}
                            ]
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "root");

    equalize_horizontal_card_heights(&mut sink, "root");

    assert_eq!(node_json(&sink, "root"), before);
}

#[test]
fn vertical_container_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Page",
            "width": 390,
            "height": "fit_content",
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "card-a",
                    "name": "Deal Card A",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "children": [
                        {"type": "text", "id": "card-a-title", "content": "Weekend Bento"}
                    ]
                },
                {
                    "type": "frame",
                    "id": "card-b",
                    "name": "Deal Card B",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "children": [
                        {"type": "text", "id": "card-b-title", "content": "Family feast"}
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "root");

    equalize_horizontal_card_heights(&mut sink, "root");

    assert_eq!(node_json(&sink, "root"), before);
}

#[test]
fn empty_frame_siblings_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Page",
            "width": 390,
            "height": "fit_content",
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "deals-row",
                    "name": "Deals of the Week",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "horizontal",
                    "children": [
                        {
                            "type": "frame",
                            "id": "spacer-a",
                            "name": "Spacer A",
                            "width": "fill_container",
                            "height": "fit_content",
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "spacer-b",
                            "name": "Spacer B",
                            "width": "fill_container",
                            "height": "fit_content",
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    );
    let before = node_json(&sink, "root");

    equalize_horizontal_card_heights(&mut sink, "root");

    assert_eq!(node_json(&sink, "root"), before);
}
