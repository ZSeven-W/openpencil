use super::*;
use serde_json::json;

fn node(animations: Value, extra: Value) -> PenNode {
    let mut source = json!({
        "type": "frame", "id": "card", "width": 100, "height": 100,
        "animations": animations
    });
    if let Value::Object(extra) = extra {
        source.as_object_mut().expect("object").extend(extra);
    }
    serde_json::from_value(source).expect("node")
}

#[test]
fn rows_name_the_preset_an_animation_still_is() {
    // Authored integers still match the preset's floats; trigger, delay,
    // id and fill are free.
    let slide = json!({
        "trigger": "inView", "delayMs": 120, "fillMode": "none",
        "durationMs": 400, "easing": "emphasizedDecelerate",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0, "translateY": 16}},
            {"offset": 1, "values": {"opacity": 1, "translateY": 0}}
        ]
    });
    let mut tweaked = slide.clone();
    tweaked["durationMs"] = json!(500);
    let summary = MotionSummary::of(&node(json!([slide, tweaked]), json!({})));
    assert_eq!(summary.rows[0].preset, Some(MotionPreset::SlideUp));
    assert_eq!(summary.rows[0].trigger, MotionTrigger::InView);
    assert_eq!(summary.rows[0].delay_ms, 120);
    assert_eq!(summary.rows[1].preset, None);
    assert_eq!(summary.rows[1].title(), ("motion.preset.custom", "Custom"));
}

#[test]
fn every_preset_recognises_itself() {
    for preset in MotionPreset::ALL {
        for trigger in MOTION_TRIGGERS {
            let animation = preset.animation(trigger);
            assert_eq!(matching_preset(&animation), Some(preset), "{preset:?}");
        }
    }
}

#[test]
fn the_summary_carries_the_transition_and_whether_children_can_stagger() {
    let leaf = MotionSummary::of(&node(json!([]), json!({})));
    assert!(leaf.rows.is_empty() && leaf.transition.is_none() && !leaf.can_stagger);

    let container = MotionSummary::of(&node(
        json!([]),
        json!({
            "transition": {"durationMs": 150, "easing": "standard"},
            "children": [{"type": "rectangle", "id": "a", "width": 1, "height": 1}]
        }),
    ));
    assert_eq!(container.transition.map(|t| t.duration_ms), Some(150));
    assert!(container.can_stagger);
}

#[test]
fn labels_cover_every_choice_with_distinct_keys() {
    let mut keys: Vec<&str> = MOTION_EASINGS
        .iter()
        .map(|easing| easing_label(easing).0)
        .chain(
            MOTION_TRIGGERS
                .iter()
                .map(|trigger| trigger_label(*trigger).0),
        )
        .chain(
            [NodeAnimationFillMode::Forwards, NodeAnimationFillMode::None]
                .map(|fill| fill_label(fill).0),
        )
        .chain([easing_label(&Easing::CubicBezier(0.1, 0.2, 0.3, 0.4)).0])
        .collect();
    let count = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), count);
}
