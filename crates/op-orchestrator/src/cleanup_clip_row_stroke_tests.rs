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
fn clipping_horizontal_row_with_stroked_chip_preserves_trailing_edge() {
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
        json!([1.0, 0.0, 1.0, 1.0])
    );
}

#[test]
fn clipping_horizontal_row_preserves_zero_trailing_padding() {
    let mut sink = VecDocSink::new();
    insert_date_scroller(
        &mut sink,
        "horizontal",
        true,
        "[0, 0, 0, 24]",
        r##""stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]} ,"##,
    );

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "date-row")["padding"],
        json!([1.0, 0.0, 1.0, 24.0])
    );

    sink.applied.clear();
    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");
    assert!(
        sink.applied.is_empty(),
        "the intentionally flush trailing edge must not make the pass emit a no-op command forever"
    );
}

#[test]
fn sibling_equalization_does_not_undo_clip_stroke_safety() {
    let mut sink = VecDocSink::new();
    let rows: Vec<Value> = (1..=6)
        .map(|index| {
            let stroke = if index <= 2 {
                json!({
                    "thickness": 1,
                    "fill": [{"type": "solid", "color": "#E5E7EB"}]
                })
            } else {
                Value::Null
            };
            let mut child = json!({
                "type": "frame",
                "id": format!("chip-{index}"),
                "name": format!("Chip {index:02}"),
                "width": 48,
                "height": 60,
                "children": []
            });
            if !stroke.is_null() {
                child["stroke"] = stroke;
            }
            json!({
                "type": "frame",
                "id": format!("row-{index}"),
                "name": format!("Date Scroller {index:02}"),
                "width": "fill_container",
                "height": "fit_content",
                "layout": "horizontal",
                "clipContent": true,
                "padding": [0, 20],
                "children": [child]
            })
        })
        .collect();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame",
            "id": "root",
            "name": "Mobile Root",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "children": rows
        })
        .to_string(),
    );

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");
    let top_padding: Vec<f64> = (1..=6)
        .map(|index| {
            node_json(&sink, &format!("row-{index}"))["padding"][0]
                .as_f64()
                .expect("top padding is numeric")
        })
        .collect();
    assert_eq!(top_padding, vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
    let zero_votes = top_padding
        .iter()
        .filter(|padding| **padding == 0.0)
        .count();
    assert_eq!(
        zero_votes, 4,
        "the fixture must have four zero-padding siblings"
    );
    assert!(
        zero_votes * 3 >= top_padding.len() * 2,
        "four of six siblings must satisfy the equalizer's 2/3 majority threshold"
    );
    assert_ne!(
        top_padding[0], 0.0,
        "without the clip-stroke floor, the zero majority would target this protected edge"
    );
    assert_eq!(
        equalize_sibling_items(&mut sink, "root"),
        0,
        "the unstroked 4/6 majority must not vote protected edges back to zero"
    );
    assert_eq!(
        node_json(&sink, "row-1")["padding"],
        json!([1.0, 20.0, 1.0, 20.0])
    );

    sink.applied.clear();
    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");
    assert_eq!(equalize_sibling_items(&mut sink, "root"), 0);
    assert!(
        sink.applied.is_empty(),
        "the ordered finalize slice must emit zero commands on its second run"
    );

    // Exercise the public native finalizer too: the DSH adapter calls this
    // surface, and used to observe fresh repair records/version bumps even
    // though the document ended each run byte-identical.
    crate::loop_finalize::apply_loop_finalize_counted(&mut sink.state);
    let before_second_finalize =
        serde_json::to_value(sink.state.active_children()).expect("serialize finalized tree");
    let second = crate::loop_finalize::apply_loop_finalize_counted(&mut sink.state);
    let after_second_finalize =
        serde_json::to_value(sink.state.active_children()).expect("serialize finalized tree");
    assert_eq!(
        second.total_repairs(),
        0,
        "a second native finalize must report zero repairs: {:?}",
        second.records()
    );
    assert_eq!(
        after_second_finalize, before_second_finalize,
        "a second native finalize must leave the document unchanged"
    );
}

#[test]
fn fit_content_clipping_row_preserves_all_sides_of_child_strokes() {
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
            "children": [{
                "type": "frame",
                "id": "actions",
                "name": "Actions Group",
                "width": "fit_content",
                "height": "fit_content",
                "layout": "horizontal",
                "clipContent": true,
                "children": [{
                    "type": "frame",
                    "id": "avatar",
                    "name": "User Avatar",
                    "width": 36,
                    "height": 36,
                    "cornerRadius": 18,
                    "stroke": {
                        "thickness": 2,
                        "fill": [{"type": "solid", "color": "#FF6B6B"}]
                    },
                    "children": []
                }]
            }]
        }"##,
    );

    pad_clipping_horizontal_row_for_stroke(&mut sink, "root");

    assert_eq!(
        node_json(&sink, "actions")["padding"],
        json!([2.0, 2.0, 2.0, 2.0]),
        "a hugging wrapper has no intentional trailing crop, so all four stroke edges need room"
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
