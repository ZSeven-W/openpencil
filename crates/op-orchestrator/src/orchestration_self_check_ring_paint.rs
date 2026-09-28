//! Paint the ring a progress-ring wrapper was named for but never drew.
//!
//! `missing-progress-ring` flags a near-square wrapper that names itself a
//! ring/gauge, holds a numeric progress metric, and contains no circle or
//! arc. As a fatal self-check finding it rejected the attempt outright, and
//! a fitness home screen lost its whole hero card — ring, calorie count and
//! three sub-metrics — after two retries hit the same finding (arena m02,
//! GLM-5.3-Flash, 2026-09-27). Losing the section is far worse than the
//! defect: the wrapper already has the right size, centre and content, it
//! only lacks the stroke.
//!
//! The repair makes the wrapper itself the ring: a corner radius of half its
//! short side and, when it has no stroke of its own, a track stroke in the
//! document's `--primary`. That is exactly the "painted circular frame" the
//! detector accepts, so the finding clears instead of recurring.

use serde_json::{json, Value};

/// Track width as a share of the ring's diameter, clamped to a readable
/// range: thin enough to leave room for the centre metric.
const TRACK_SHARE: f64 = 0.07;
const TRACK_MIN: f64 = 6.0;
const TRACK_MAX: f64 = 14.0;

/// Round each flagged wrapper and give it a track stroke. Returns whether
/// anything changed.
pub(super) fn paint_missing_rings(value: &mut Value, ids: &[String]) -> bool {
    if ids.is_empty() {
        return false;
    }
    let mut changed = false;
    walk(value, ids, &mut changed);
    changed
}

fn walk(node: &mut Value, ids: &[String], changed: &mut bool) {
    match node {
        Value::Array(items) => {
            for item in items {
                walk(item, ids, changed);
            }
        }
        Value::Object(map) => {
            let flagged = map
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|id| ids.iter().any(|flagged| flagged == id));
            if flagged {
                let size = ["width", "height"]
                    .map(|key| map.get(key).and_then(Value::as_f64))
                    .into_iter()
                    .collect::<Option<Vec<f64>>>()
                    .and_then(|sizes| sizes.into_iter().reduce(f64::min));
                if let Some(diameter) = size.filter(|d| *d > 0.0) {
                    map.insert("cornerRadius".into(), json!(diameter / 2.0));
                    let has_stroke = map.get("stroke").is_some_and(|stroke| !stroke.is_null());
                    if !has_stroke {
                        let thickness =
                            (diameter * TRACK_SHARE).clamp(TRACK_MIN, TRACK_MAX).round();
                        map.insert(
                            "stroke".into(),
                            json!({
                                "thickness": thickness,
                                "fill": [{"type": "solid", "color": "$--primary"}]
                            }),
                        );
                    }
                    *changed = true;
                }
            }
            if let Some(children) = map.get_mut("children") {
                walk(children, ids, changed);
            }
        }
        _ => {}
    }
}
