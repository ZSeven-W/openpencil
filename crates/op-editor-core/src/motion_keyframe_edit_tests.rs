use super::*;
use serde_json::json;

fn animation() -> NodeAnimation {
    serde_json::from_value(json!({
        "trigger": "mount", "durationMs": 400, "easing": "linear",
        "keyframes": [
            {"offset": 0, "values": {"opacity": 0, "translateY": 16}},
            {"offset": 0.5, "values": {"translateY": 4}},
            {"offset": 1, "values": {"opacity": 1, "translateY": 0}}
        ]
    }))
    .expect("animation")
}

fn node() -> PenNode {
    serde_json::from_value(json!({
        "type": "rectangle", "id": "a", "width": 10, "height": 10, "opacity": 0.8,
        "fill": [{"type": "solid", "color": "#ff0000"}]
    }))
    .expect("node")
}

#[test]
fn only_whitelisted_properties_the_frame_lacks_are_offered() {
    let addable = addable_properties(&animation(), 1);
    assert_eq!(addable.first(), Some(&"opacity"));
    assert!(!addable.contains(&"translateY"));
    assert!(addable.iter().all(|p| P1_MOTION_PROPERTIES.contains(p)));
    assert!(addable_properties(&animation(), 9).is_empty());
}

#[test]
fn an_added_property_takes_the_value_the_curve_already_has() {
    // opacity runs 0 → 1 across the middle keyframe: 0.5 there.
    let edited = add_keyframe_value(&animation(), 1, "opacity", &node());
    assert_eq!(edited.keyframes[1].values["opacity"], json!(0.5));
    // A property no keyframe animates starts from the node's own value.
    let edited = add_keyframe_value(&animation(), 1, "fill", &node());
    assert_eq!(edited.keyframes[1].values["fill"], json!("#ff0000"));
    let edited = add_keyframe_value(&animation(), 1, "scaleX", &node());
    assert_eq!(edited.keyframes[1].values["scaleX"], json!(1.0));
    // Outside the whitelist nothing changes.
    assert_eq!(
        add_keyframe_value(&animation(), 1, "width", &node()),
        animation()
    );
}

#[test]
fn values_set_and_remove_on_one_keyframe_only() {
    let edited = set_keyframe_value(&animation(), 1, "translateY", json!(8));
    assert_eq!(edited.keyframes[1].values["translateY"], json!(8));
    assert_eq!(edited.keyframes[0], animation().keyframes[0]);
    let edited = remove_keyframe_value(&edited, 0, "opacity");
    assert!(!edited.keyframes[0].values.contains_key("opacity"));
    assert!(edited.keyframes[2].values.contains_key("opacity"));
}

#[test]
fn typed_values_parse_by_property_kind() {
    assert_eq!(parse_keyframe_value("opacity", " 0.25 "), Some(json!(0.25)));
    assert_eq!(
        parse_keyframe_value("translateX", "-12"),
        Some(json!(-12.0))
    );
    assert_eq!(parse_keyframe_value("opacity", "half"), None);
    assert_eq!(parse_keyframe_value("fill", "0af"), Some(json!("#0af")));
    assert_eq!(
        parse_keyframe_value("fill", "#00aaff"),
        Some(json!("#00aaff"))
    );
    assert_eq!(parse_keyframe_value("fill", "#00aaf"), None);
    assert_eq!(format_keyframe_value(&json!(16.0)), "16");
    assert_eq!(format_keyframe_value(&json!(0.3333333)), "0.333");
    assert_eq!(format_keyframe_value(&json!("#fff")), "#fff");
}

#[test]
fn bezier_inputs_show_the_runtime_curve_and_write_a_custom_one() {
    assert_eq!(bezier_controls(&Easing::Standard), [0.2, 0.0, 0.0, 1.0]);
    assert_eq!(
        bezier_controls(&Easing::CubicBezier(0.1, 0.2, 0.3, 0.4)),
        [0.1, 0.2, 0.3, 0.4]
    );
    assert_eq!(bezier_controls(&Easing::EaseInOut), [0.42, 0.0, 0.58, 1.0]);
    assert_eq!(
        with_bezier_control(&Easing::Standard, 1, 0.5),
        Easing::CubicBezier(0.2, 0.5, 0.0, 1.0)
    );
    // x is clamped into [0, 1]; y may overshoot.
    assert_eq!(
        with_bezier_control(&Easing::Standard, 2, 1.7),
        Easing::CubicBezier(0.2, 0.0, 1.0, 1.0)
    );
    assert_eq!(
        with_bezier_control(&Easing::Standard, 3, 1.4),
        Easing::CubicBezier(0.2, 0.0, 0.0, 1.4)
    );
}
