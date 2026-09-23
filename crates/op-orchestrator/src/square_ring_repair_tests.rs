//! Unit tests for the square ring-wrapper predicate and its diagnostic twin.

use super::*;
use serde_json::json;

fn dot_wrapper(corner_radius: Option<f64>, children: Value) -> Value {
    let mut v = json!({
        "type": "frame", "id": "dot", "name": "在线绿点",
        "width": 16, "height": 16,
        "fill": [{ "type": "solid", "color": "$--background" }],
        "children": children,
    });
    if let Some(r) = corner_radius {
        v["cornerRadius"] = json!(r);
    }
    v
}

fn ellipse(size: f64) -> Value {
    json!({
        "type": "ellipse", "id": "dot-ellipse", "name": "在线状态圆",
        "width": size, "height": size,
        "fill": [{ "type": "solid", "color": "#16A34A" }],
    })
}

/// The lane0/app-08 hit: 16×16 frame over a 10×10 ellipse → radius 8.
#[test]
fn app08_status_dot_wrapper_is_repaired() {
    let mut out = Vec::new();
    collect_square_ring_ids(
        &dot_wrapper(None, json!([ellipse(10.0)])),
        &HashMap::new(),
        &mut out,
    );
    assert_eq!(out, vec![("dot".to_string(), 8.0)]);
}

/// The lane1/app-09 hit: 28×28 swatch rings → radius 14.
#[test]
fn app09_swatch_ring_is_repaired() {
    let mut v = dot_wrapper(None, json!([ellipse(20.0)]));
    v["width"] = json!(28);
    v["height"] = json!(28);
    v["name"] = json!("swatch-ring-1");
    let mut out = Vec::new();
    collect_square_ring_ids(&v, &HashMap::new(), &mut out);
    assert_eq!(out, vec![("dot".to_string(), 14.0)]);
}

/// Two children mean the frame composes content — not a dot shell.
#[test]
fn a_frame_with_two_ellipses_is_not_touched() {
    let v = dot_wrapper(None, json!([ellipse(10.0), ellipse(6.0)]));
    let mut out = Vec::new();
    collect_square_ring_ids(&v, &HashMap::new(), &mut out);
    assert!(out.is_empty(), "{out:?}");
}

/// Above 40px the square is a board/card, not a dot.
#[test]
fn a_large_square_frame_is_not_touched() {
    let mut v = dot_wrapper(None, json!([ellipse(48.0)]));
    v["width"] = json!(60);
    v["height"] = json!(60);
    let mut out = Vec::new();
    collect_square_ring_ids(&v, &HashMap::new(), &mut out);
    assert!(out.is_empty(), "{out:?}");
}

/// An already-circular wrapper (cornerRadius ≥ width/2) is left alone.
#[test]
fn an_already_round_wrapper_is_not_touched() {
    let v = dot_wrapper(Some(12.0), json!([ellipse(10.0)]));
    let mut out = Vec::new();
    collect_square_ring_ids(&v, &HashMap::new(), &mut out);
    assert!(out.is_empty(), "{out:?}");
}

/// A wrapper with SOME radius but short of circular (4 on 24) still reads as a
/// square stamp — the pass completes it to width/2.
#[test]
fn a_partially_rounded_wrapper_is_completed() {
    let mut v = dot_wrapper(Some(4.0), json!([ellipse(20.0)]));
    v["width"] = json!(24);
    v["height"] = json!(24);
    let mut out = Vec::new();
    collect_square_ring_ids(&v, &HashMap::new(), &mut out);
    assert_eq!(out, vec![("dot".to_string(), 12.0)]);
}

/// A transparent (or missing) fill means nothing is painted over the ellipse.
#[test]
fn a_transparent_wrapper_is_not_touched() {
    let mut v = dot_wrapper(None, json!([ellipse(10.0)]));
    v["fill"] = json!([{ "type": "solid", "color": "#FFFFFF", "opacity": 0.0 }]);
    let mut out = Vec::new();
    collect_square_ring_ids(&v, &HashMap::new(), &mut out);
    assert!(out.is_empty(), "{out:?}");
}

/// The diagnostic mirrors the repair predicate and carries the
/// `square-ring-wrapper` key.
#[test]
fn diagnostic_reports_the_wrapper_with_its_name() {
    let line = square_ring_wrapper_diagnostic(&dot_wrapper(None, json!([ellipse(10.0)])), None);
    let line = line.expect("diagnostic fires");
    assert!(line.contains("square-ring-wrapper"), "{line}");
    assert!(line.contains("在线绿点"), "{line}");
}

#[test]
fn diagnostic_is_silent_for_healthy_shapes() {
    assert!(
        square_ring_wrapper_diagnostic(&dot_wrapper(Some(8.0), json!([ellipse(10.0)])), None)
            .is_none()
    );
    assert!(square_ring_wrapper_diagnostic(&ellipse(10.0), None).is_none());
}
