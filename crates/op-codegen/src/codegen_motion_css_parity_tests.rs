//! End-to-end parity for the HTML motion export: evaluate the exported
//! `@keyframes` with CSS semantics (each keyframe interval eased by its own
//! timing function) and compare with Jian's runtime clip, which is what
//! Preview plays. One case per way the old export diverged: named curves,
//! easing scope across several keyframes, clamped overshoot, and a
//! piecewise curve.

use crate::{Codegen, Html};
use jian_core::action::services::AnimationProperty;
use jian_core::geometry::Rect;
use jian_core::motion::{Clip, ClipTrigger, MotionSceneView};
use serde_json::{json, Value};

/// Preview's untouched look for the properties the cases animate.
struct Authored;

impl MotionSceneView for Authored {
    fn node_rect(&self, _id: &str) -> Option<Rect> {
        Some(jian_core::geometry::rect(0.0, 0.0, 100.0, 100.0))
    }

    fn property(&self, _id: &str, property: &AnimationProperty) -> Option<Value> {
        Some(match property {
            AnimationProperty::Opacity => json!(1.0),
            _ => json!(0.0),
        })
    }

    fn root_rect(&self) -> Option<Rect> {
        self.node_rect("root")
    }

    fn switches(&self) -> Vec<(String, bool)> {
        Vec::new()
    }

    fn subtree_contains(&self, ancestor: &str, id: &str) -> bool {
        ancestor == id
    }
}

/// One exported keyframe: offset in [0, 1], values by property, and the
/// timing function of the interval it starts.
struct Keyframe {
    at: f64,
    values: Vec<(&'static str, f64)>,
    timing: Option<[f64; 4]>,
}

fn parse_keyframes(css: &str, name: &str) -> Vec<Keyframe> {
    let start = css
        .find(&format!("@keyframes {name} {{\n"))
        .expect("keyframes block");
    let mut keyframes = Vec::new();
    for line in css[start..].lines().skip(1) {
        let line = line.trim();
        if line == "}" && keyframes.is_empty() {
            break;
        }
        if let Some(percent) = line.strip_suffix("% {") {
            keyframes.push(Keyframe {
                at: percent.parse::<f64>().expect("percent") / 100.0,
                values: Vec::new(),
                timing: None,
            });
            continue;
        }
        if line == "}" {
            continue;
        }
        if line.starts_with('}') || line.starts_with("@") {
            break;
        }
        let Some(keyframe) = keyframes.last_mut() else {
            break;
        };
        let (property, value) = line
            .trim_end_matches(';')
            .split_once(": ")
            .expect("declaration");
        match property {
            "opacity" => keyframe
                .values
                .push(("opacity", value.parse().expect("opacity"))),
            "transform" => {
                let inner = value
                    .strip_prefix("translate(")
                    .and_then(|rest| rest.strip_suffix(')'))
                    .expect("translate");
                let (x, y) = inner.split_once(", ").expect("two lengths");
                let px = |length: &str| length.trim_end_matches("px").parse::<f64>().expect("px");
                keyframe.values.push(("translateX", px(x)));
                keyframe.values.push(("translateY", px(y)));
            }
            "animation-timing-function" => {
                keyframe.timing = Some(match value {
                    "linear" => [0.0, 0.0, 1.0, 1.0],
                    bezier => {
                        let numbers: Vec<f64> = bezier
                            .strip_prefix("cubic-bezier(")
                            .and_then(|rest| rest.strip_suffix(')'))
                            .expect("cubic-bezier")
                            .split(',')
                            .map(|number| number.parse().expect("number"))
                            .collect();
                        [numbers[0], numbers[1], numbers[2], numbers[3]]
                    }
                });
            }
            other => panic!("unexpected declaration {other}"),
        }
    }
    keyframes
}

/// CSS `cubic-bezier(x1, y1, x2, y2)` at input progress `x`.
fn css_bezier([x1, y1, x2, y2]: [f64; 4], x: f64) -> f64 {
    let curve = |t: f64, a: f64, b: f64| {
        let inverse = 1.0 - t;
        3.0 * inverse * inverse * t * a + 3.0 * inverse * t * t * b + t * t * t
    };
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..60 {
        let middle = (low + high) / 2.0;
        if curve(middle, x1, x2) < x {
            low = middle;
        } else {
            high = middle;
        }
    }
    curve((low + high) / 2.0, y1, y2)
}

/// The exported animation's value for `property` at progress `p`, as a
/// browser computes it.
fn css_value(keyframes: &[Keyframe], property: &str, p: f64) -> f64 {
    let value = |keyframe: &Keyframe| {
        keyframe
            .values
            .iter()
            .find(|(name, _)| *name == property)
            .map(|(_, value)| *value)
            .expect("every keyframe lists every property")
    };
    let index = keyframes
        .windows(2)
        .position(|pair| p <= pair[1].at)
        .unwrap_or(keyframes.len() - 2);
    let (from, to) = (&keyframes[index], &keyframes[index + 1]);
    let local = ((p - from.at) / (to.at - from.at)).clamp(0.0, 1.0);
    let eased = css_bezier(from.timing.unwrap_or([0.0, 0.0, 1.0, 1.0]), local);
    value(from) + (value(to) - value(from)) * eased
}

fn assert_export_matches_preview(case: &str, easing: Value, keyframes: Value, properties: &[&str]) {
    const DURATION_MS: u64 = 400;
    let doc = jian_ops_schema::load_str(
        &json!({
            "version": "1.0",
            "children": [{
                "type": "rectangle", "id": "n", "width": 100, "height": 100,
                "animations": [{
                    "trigger": "mount", "keyframes": keyframes,
                    "durationMs": DURATION_MS, "easing": easing
                }]
            }]
        })
        .to_string(),
    )
    .expect("parity document")
    .value;
    let html = Html.generate(&doc);
    let exported = parse_keyframes(&html, "op-n-0");
    let clip = Clip::build(&doc.children[0], &ClipTrigger::Mount);
    for &property in properties {
        let descriptor = jian_core::action::animation_registry::animatable_property_registry()
            .get(property)
            .expect("registered property");
        let key = (
            "n".to_owned(),
            AnimationProperty::from_registered(property, descriptor.apply),
        );
        let preview: Vec<f64> = (0..=DURATION_MS)
            .step_by(4)
            .map(|t_ms| {
                clip.sample(t_ms, &Authored)
                    .get(&key)
                    .cloned()
                    .or_else(|| Authored.property("n", &key.1))
                    .and_then(|value| value.as_f64())
                    .expect("numeric value")
            })
            .collect();
        let range = preview.iter().cloned().fold(f64::MIN, f64::max)
            - preview.iter().cloned().fold(f64::MAX, f64::min);
        for (step, expected) in preview.iter().enumerate() {
            let p = (step * 4) as f64 / DURATION_MS as f64;
            let actual = css_value(&exported, property, p);
            assert!(
                (actual - expected).abs() <= range.max(1e-9) * 0.005,
                "{case} {property} at {p}: export {actual} vs preview {expected}\n{html}"
            );
        }
    }
}

#[test]
fn named_curves_play_like_preview() {
    for easing in [
        "ease",
        "easeIn",
        "easeOut",
        "standard",
        "emphasizedAccelerate",
    ] {
        assert_export_matches_preview(
            easing,
            json!(easing),
            json!([{"offset": 0, "values": {"opacity": 0}}, {"offset": 1, "values": {"opacity": 1}}]),
            &["opacity"],
        );
    }
}

#[test]
fn several_keyframes_share_one_curve_like_preview() {
    assert_export_matches_preview(
        "easeIn, three stops",
        json!("easeIn"),
        json!([
            {"offset": 0, "values": {"opacity": 0, "translateY": 24}},
            {"offset": 0.25, "values": {"opacity": 1, "translateY": 0}},
            {"offset": 1, "values": {"opacity": 0.5, "translateY": -8}}
        ]),
        &["opacity", "translateY"],
    );
    assert_export_matches_preview(
        "easeInOut, five stops",
        json!("easeInOut"),
        json!([
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 0.25, "values": {"opacity": 1}},
            {"offset": 0.5, "values": {"opacity": 0.2}},
            {"offset": 0.75, "values": {"opacity": 0.8}},
            {"offset": 1, "values": {"opacity": 0.5}}
        ]),
        &["opacity"],
    );
}

#[test]
fn overshooting_curves_clamp_like_preview() {
    assert_export_matches_preview(
        "overshoot",
        json!({"cubicBezier": [0.34, 1.56, 0.64, 1.0]}),
        json!([
            {"offset": 0, "values": {"translateX": 0}},
            {"offset": 1, "values": {"translateX": 100}}
        ]),
        &["translateX"],
    );
}
