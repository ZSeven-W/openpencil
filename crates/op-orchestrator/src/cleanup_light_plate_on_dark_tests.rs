//! arena-m01 (Opus 5.5, 2026-10-01): a `$--border` plate inside the dark cart
//! pill painted a white square behind the cart icon.

use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn sink_with(root: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.doc.variables = Some(
        [
            ("--border", "#FFFFFF"),
            ("--input", "#EEEAE2"),
            ("--primary", "#FF5A1F"),
            ("--card", "#FFFFFF"),
        ]
        .into_iter()
        .map(|(name, hex)| {
            (
                name.to_string(),
                serde_json::from_value(json!({
                    "type": "color",
                    "value": [{"value": hex, "theme": {"Mode": "Light"}}]
                }))
                .expect("variable"),
            )
        })
        .collect(),
    );
    sink.state.doc.themes = Some(
        [("Mode".to_string(), vec!["Light".to_string()])]
            .into_iter()
            .collect(),
    );
    sink.state.doc.children = vec![serde_json::from_value(root).expect("root")];
    sink
}

fn fill_of(sink: &VecDocSink, id: &str) -> Value {
    let root = serde_json::to_value(&sink.state.active_children()[0]).unwrap();
    fn find<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
        if v.get("id").and_then(Value::as_str) == Some(id) {
            return Some(v);
        }
        v.get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find_map(|c| find(c, id))
    }
    find(&root, id).unwrap_or_else(|| panic!("{id}"))["fill"].clone()
}

/// The measured shape: dark pill → unfilled summary row → `$--border`
/// none-stack (52×48, square) → dark disc + badge.
fn store_page(plate_fill: &str, plate_radius: Option<f64>, pill_fill: &str) -> Value {
    let mut plate = json!({
        "type": "frame", "id": "plate", "name": "cart-disc-stack", "layout": "none",
        "width": 52, "height": 48, "fill": [{"type": "solid", "color": plate_fill}],
        "children": [
            {"type": "frame", "id": "badge", "name": "cart-count-badge", "width": 16, "height": 16,
             "cornerRadius": 999, "fill": [{"type": "solid", "color": "$--primary"}], "children": []},
            {"type": "frame", "id": "disc", "name": "cart-disc", "width": 44, "height": 44,
             "fill": [{"type": "solid", "color": "#3A2A22"}], "children": []}
        ]
    });
    if let Some(radius) = plate_radius {
        plate["cornerRadius"] = json!(radius);
    }
    json!({
        "type": "frame", "id": "root", "name": "商家详情", "width": 375, "height": 812,
        "layout": "vertical", "children": [{
            "type": "frame", "id": "pill", "name": "cart-pill", "layout": "horizontal",
            "width": "fill_container", "height": 60, "cornerRadius": 30,
            "fill": [{"type": "solid", "color": pill_fill}],
            "children": [{
                "type": "frame", "id": "summary", "name": "cart-summary", "layout": "horizontal",
                "width": "fit_content", "height": "fit_content", "children": [plate]
            }]
        }]
    })
}

#[test]
fn a_border_token_plate_on_a_dark_pill_loses_its_fill() {
    let mut sink = sink_with(store_page("$--border", None, "#21140F"));
    assert_eq!(strip_light_plates_on_dark(&mut sink, "root"), 1);
    assert_eq!(fill_of(&sink, "plate"), json!([]));
    // What the plate wrapped keeps its own paint.
    assert_eq!(fill_of(&sink, "disc")[0]["color"], "#3A2A22");
}

#[test]
fn the_same_plate_on_a_light_card_is_the_authored_soft_tile() {
    let mut sink = sink_with(store_page("$--border", None, "$--card"));
    assert_eq!(strip_light_plates_on_dark(&mut sink, "root"), 0);
    assert_eq!(fill_of(&sink, "plate")[0]["color"], "$--border");
}

#[test]
fn a_rounded_plate_or_a_literal_white_is_left_alone() {
    let mut sink = sink_with(store_page("$--border", Some(12.0), "#21140F"));
    assert_eq!(strip_light_plates_on_dark(&mut sink, "root"), 0);
    let mut sink = sink_with(store_page("#FFFFFF", None, "#21140F"));
    assert_eq!(strip_light_plates_on_dark(&mut sink, "root"), 0);
}
