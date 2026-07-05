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

fn insert_date_scroller(
    sink: &mut VecDocSink,
    row_layout: &str,
    clip_content: bool,
    row_padding: &str,
    child_stroke: &str,
) {
    insert_tree(
        sink,
        &format!(
            r##"{{
                "type": "frame",
                "id": "root",
                "name": "Mobile Root",
                "width": 390,
                "height": 844,
                "layout": "vertical",
                "children": [
                    {{
                        "type": "frame",
                        "id": "section",
                        "name": "Upcoming",
                        "width": "fill_container",
                        "height": "fit_content",
                        "layout": "vertical",
                        "children": [
                            {{
                                "type": "frame",
                                "id": "date-row",
                                "name": "Date Scroller",
                                "width": "fill_container",
                                "height": "fit_content",
                                "layout": "{row_layout}",
                                "clipContent": {clip_content},
                                "padding": {row_padding},
                                "children": [
                                    {{
                                        "type": "frame",
                                        "id": "date-chip",
                                        "name": "Tue 14",
                                        "width": 48,
                                        "height": 60,
                                        "layout": "vertical",
                                        {child_stroke}
                                        "children": []
                                    }}
                                ]
                            }}
                        ]
                    }}
                ]
            }}"##
        ),
    );
}

#[test]
fn clipping_horizontal_row_with_stroked_chip_gets_stroke_padding_on_all_sides() {
    let mut sink = VecDocSink::new();
    insert_date_scroller(
        &mut sink,
        "horizontal",
        true,
        "[0, 0, 0, 0]",
        r##""stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]},"##,
    );

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "date-row")["padding"],
        json!([1.0, 1.0, 1.0, 1.0])
    );
}

#[test]
fn non_clipping_row_untouched() {
    let mut sink = VecDocSink::new();
    insert_date_scroller(
        &mut sink,
        "horizontal",
        false,
        "[0, 12]",
        r##""stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]},"##,
    );
    let before = node_json(&sink, "date-row");

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");

    assert_eq!(node_json(&sink, "date-row"), before);
}

#[test]
fn row_without_stroked_children_untouched() {
    let mut sink = VecDocSink::new();
    insert_date_scroller(&mut sink, "horizontal", true, "[0, 12]", "");
    let before = node_json(&sink, "date-row");

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");

    assert_eq!(node_json(&sink, "date-row"), before);
}

#[test]
fn row_with_sufficient_vertical_padding_untouched() {
    let mut sink = VecDocSink::new();
    insert_date_scroller(
        &mut sink,
        "horizontal",
        true,
        "[2, 12, 2, 12]",
        r##""stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]},"##,
    );
    let before = node_json(&sink, "date-row");

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");

    assert_eq!(node_json(&sink, "date-row"), before);
}

#[test]
fn vertical_container_untouched() {
    let mut sink = VecDocSink::new();
    insert_date_scroller(
        &mut sink,
        "vertical",
        true,
        "[0, 12]",
        r##""stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]},"##,
    );
    let before = node_json(&sink, "date-row");

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");

    assert_eq!(node_json(&sink, "date-row"), before);
}
