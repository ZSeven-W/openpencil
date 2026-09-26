//! Tests for the state-member exemption in `equalize_sibling_items`: the
//! selected chip / active tab — a member whose OWN fill or stroke differs from
//! the family majority — keeps its label's authored weight and size and does
//! not vote. No text is measured here, so nothing depends on installed faces.

use super::*;
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

fn chip(id: &str, fill: Value, stroke: Value, weight: u32, color: &str) -> Value {
    json!({
        "type": "frame", "id": id, "name": format!("range-chip-{id}"),
        "width": 44, "height": 28,
        "layout": "horizontal", "justifyContent": "center", "alignItems": "center",
        "fill": fill, "stroke": stroke,
        "children": [{
            "type": "text", "id": format!("{id}-text"), "name": format!("{id}-text"),
            "content": id, "fontSize": 12, "fontWeight": weight,
            "fill": [{"type": "solid", "color": color}]
        }]
    })
}

fn plain(id: &str, weight: u32) -> Value {
    chip(
        id,
        json!([]),
        json!({"thickness": 0, "fill": []}),
        weight,
        "#0F172A",
    )
}

fn selected(id: &str, weight: u32) -> Value {
    chip(
        id,
        json!([{"type": "solid", "color": "#FFFFFF"}]),
        json!({"thickness": 1, "fill": [{"type": "solid", "color": "#E2E8F0"}]}),
        weight,
        "#0F172A",
    )
}

fn sink_with_row(chips: Vec<Value>) -> (VecDocSink, String) {
    let root = json!({
        "type": "frame", "id": "root", "name": "Screen",
        "width": 375, "height": 812, "layout": "vertical",
        "children": [{
            "type": "frame", "id": "switcher", "name": "range-switcher",
            "layout": "horizontal", "gap": 4, "padding": 3,
            "children": chips
        }]
    });
    let root: PenNode = serde_json::from_value(root).expect("fixture");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    let root_id = sink.state.active_children()[0].id_str().to_string();
    (sink, root_id)
}

fn weight_of(sink: &VecDocSink, id: &str) -> Value {
    fn find(v: &Value, id: &str) -> Option<Value> {
        if v.get("id").and_then(Value::as_str) == Some(id) {
            return Some(v.clone());
        }
        v.get("children")?
            .as_array()?
            .iter()
            .find_map(|c| find(c, id))
    }
    let root = serde_json::to_value(&sink.state.active_children()[0]).expect("root");
    find(&root, id).expect("node")["fontWeight"].clone()
}

#[test]
fn the_selected_chip_keeps_its_bold_label() {
    let (mut sink, root_id) = sink_with_row(vec![
        plain("1w", 400),
        selected("1m", 600),
        plain("3m", 400),
    ]);

    assert_eq!(equalize_sibling_items(&mut sink, &root_id), 0);
    assert_eq!(weight_of(&sink, "1m-text"), json!(600));
    assert_eq!(weight_of(&sink, "1w-text"), json!(400));
    assert_eq!(weight_of(&sink, "3m-text"), json!(400));
}

/// The exempt member does not vote: without it the three plain chips hold a
/// 400 majority (2/3) that aligns the 500 straggler; with it counted (2/4)
/// nothing would clear the bar.
#[test]
fn the_selected_chip_does_not_drag_the_vote() {
    let (mut sink, root_id) = sink_with_row(vec![
        selected("a", 600),
        plain("b", 400),
        plain("c", 400),
        plain("d", 500),
    ]);

    equalize_sibling_items(&mut sink, &root_id);
    assert_eq!(weight_of(&sink, "a-text"), json!(600));
    assert_eq!(weight_of(&sink, "d-text"), json!(400));
}

#[test]
fn an_accidentally_bold_label_in_a_uniform_family_is_still_aligned() {
    let same = |id: &str, weight: u32| {
        chip(
            id,
            json!([{"type": "solid", "color": "#F1F5F9"}]),
            json!({"thickness": 0, "fill": []}),
            weight,
            "#0F172A",
        )
    };
    let (mut sink, root_id) = sink_with_row(vec![same("a", 400), same("b", 400), same("c", 700)]);

    assert_eq!(equalize_sibling_items(&mut sink, &root_id), 1);
    assert_eq!(weight_of(&sink, "c-text"), json!(400));
}

#[test]
fn a_different_text_colour_alone_does_not_exempt() {
    let (mut sink, root_id) = sink_with_row(vec![
        plain("a", 400),
        plain("b", 400),
        chip(
            "c",
            json!([]),
            json!({"thickness": 0, "fill": []}),
            700,
            "#2563EB",
        ),
    ]);

    assert_eq!(equalize_sibling_items(&mut sink, &root_id), 1);
    assert_eq!(weight_of(&sink, "c-text"), json!(400));
}
