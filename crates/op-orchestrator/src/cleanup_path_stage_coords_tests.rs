//! showcase s03 (Opus 5.5, 2026-10-02): map paths boxed to the whole stage.

use super::*;
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn sink_with(root: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.doc.children = vec![serde_json::from_value(root).expect("root")];
    sink
}

fn node_box(sink: &VecDocSink, id: &str) -> (f64, f64, f64, f64) {
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
    let n = find(&root, id).unwrap_or_else(|| panic!("{id}"));
    let f = |k: &str| n.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    (f("x"), f("y"), f("width"), f("height"))
}

fn path(id: &str, d: &str, w: f64, h: f64) -> Value {
    json!({"type": "path", "id": id, "name": id, "x": 0, "y": 0, "width": w, "height": h,
           "d": d, "fill": [{"type": "solid", "color": "#06B6D4"}]})
}

/// The measured stage: an 888×612 `layout: "none"` map holding a route
/// line, an arrowhead and a full-stage grid texture, all boxed to the stage.
fn city_map() -> Value {
    json!({
        "type": "frame", "id": "root", "name": "大屏", "width": 1920, "height": 1080,
        "layout": "vertical", "children": [{
            "type": "frame", "id": "stage", "name": "map-stage", "layout": "none",
            "width": 888, "height": 612, "children": [
                path("arrow", "M522.2 303.3 L508.6 295.6 L520.3 287.8 Z", 888.0, 612.0),
                path("route", "M120 520 Q170 360 300 240 Q420 150 520 300", 888.0, 612.0),
                path("grid", "M0 0 L888 0 L888 612 L0 612 Z", 888.0, 612.0)
            ]
        }]
    })
}

#[test]
fn a_stage_boxed_arrowhead_is_cropped_to_its_own_bounds() {
    let mut sink = sink_with(city_map());
    assert_eq!(crop_stage_boxed_paths(&mut sink, "root"), 2);
    let (x, y, w, h) = node_box(&sink, "arrow");
    assert!(
        (x - 508.6).abs() < 0.01 && (y - 287.8).abs() < 0.01,
        "{x} {y}"
    );
    assert!(
        (w - 13.6).abs() < 0.01 && (h - 15.5).abs() < 0.01,
        "{w} {h}"
    );
    // The route keeps its drawn extent; the full-stage grid is untouched.
    let (_, _, rw, _) = node_box(&sink, "route");
    assert!(rw < 888.0 && rw > 300.0, "{rw}");
    assert_eq!(node_box(&sink, "grid"), (0.0, 0.0, 888.0, 612.0));
}

#[test]
fn a_path_boxed_smaller_than_its_parent_is_left_alone() {
    // A 40×40 icon box inside a 375-wide absolute stage: its small `d` is a
    // glyph meant to scale into the box, not stage-space geometry.
    let mut sink = sink_with(json!({
        "type": "frame", "id": "root", "width": 375, "height": 812, "layout": "vertical",
        "children": [{
            "type": "frame", "id": "stage", "layout": "none", "width": 375, "height": 300,
            "children": [path("icon", "M4 4 L20 4 L12 18 Z", 40.0, 40.0)]
        }]
    }));
    assert_eq!(crop_stage_boxed_paths(&mut sink, "root"), 0);
    assert_eq!(node_box(&sink, "icon"), (0.0, 0.0, 40.0, 40.0));
}

#[test]
fn a_path_in_a_flex_parent_is_left_alone() {
    let mut sink = sink_with(json!({
        "type": "frame", "id": "root", "width": 375, "height": 812, "layout": "vertical",
        "children": [{
            "type": "frame", "id": "row", "layout": "horizontal", "width": 375, "height": 120,
            "children": [path("spark", "M10 60 L40 20 L80 50", 375.0, 120.0)]
        }]
    }));
    assert_eq!(crop_stage_boxed_paths(&mut sink, "root"), 0);
}
