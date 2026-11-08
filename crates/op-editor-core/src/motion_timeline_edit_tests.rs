use super::*;
use jian_ops_schema::motion::validate_animations;
use serde_json::json;

fn animation(easing: &str, keyframes: serde_json::Value) -> NodeAnimation {
    serde_json::from_value(json!({
        "trigger": "mount", "easing": easing, "durationMs": 400, "delayMs": 100,
        "keyframes": keyframes
    }))
    .expect("animation")
}

fn three_stops(easing: &str) -> NodeAnimation {
    animation(
        easing,
        json!([
            {"offset": 0, "values": {"opacity": 0, "translateY": 20}},
            {"offset": 0.25, "values": {"opacity": 0.5}},
            {"offset": 1, "values": {"opacity": 1, "translateY": 0}}
        ]),
    )
}

fn times(animation: &NodeAnimation) -> Vec<f64> {
    animation
        .keyframes
        .iter()
        .map(|keyframe| keyframe_ms(animation, keyframe.offset))
        .collect()
}

fn assert_valid(animation: &NodeAnimation) {
    assert_eq!(
        validate_animations(std::slice::from_ref(animation), "$"),
        Ok(())
    );
}

/// The value Preview paints at iteration progress `p`.
fn painted(animation: &NodeAnimation, property: &str, p: f32) -> f64 {
    let progress = ease(RuntimeEasing::from(&animation.easing), p);
    sample_values(animation, progress)[property]
        .as_f64()
        .expect("number")
}

#[test]
fn strips_move_and_resize_without_touching_offsets() {
    let base = three_stops("easeIn");
    let moved = move_strip(&base, -250);
    assert_eq!(moved.delay_ms, 0);
    assert_eq!(moved.keyframes, base.keyframes);
    let resized = resize_strip(&base, 900.0);
    assert_eq!(resized.duration_ms, 800);
    assert_eq!(resized.keyframes, base.keyframes);
    assert_eq!(resize_strip(&base, 50.0).duration_ms, 1);
}

#[test]
fn a_middle_keyframe_moves_in_time_and_stops_at_its_neighbours() {
    let base = three_stops("linear");
    let moved = move_keyframe(&base, 1, 300.0);
    assert!((moved.keyframes[1].offset - 0.5).abs() < 1e-6);
    assert_eq!((moved.delay_ms, moved.duration_ms), (100, 400));
    let past = move_keyframe(&base, 1, 9_000.0);
    assert_eq!(past.keyframes[1].offset, 1.0);
    assert_valid(&past);
}

#[test]
fn dragging_the_first_or_last_keyframe_keeps_the_others_in_place() {
    let base = three_stops("easeIn");
    let before = times(&base);
    let earlier = move_keyframe(&base, 0, 40.0);
    assert_eq!((earlier.delay_ms, earlier.duration_ms), (40, 460));
    let after = times(&earlier);
    assert!((after[1] - before[1]).abs() < 0.5, "{before:?} {after:?}");
    assert!((after[2] - before[2]).abs() < 0.5, "{before:?} {after:?}");
    assert_valid(&earlier);

    let longer = move_keyframe(&base, 2, 700.0);
    assert_eq!((longer.delay_ms, longer.duration_ms), (100, 600));
    assert!((times(&longer)[1] - before[1]).abs() < 0.5);
    // The first keyframe cannot pass the second.
    let pinned = move_keyframe(&base, 0, 5_000.0);
    assert!((pinned.delay_ms as f64 - before[1]).abs() < 1.0);
    assert_valid(&pinned);
}

#[test]
fn adding_a_keyframe_inside_the_strip_keeps_the_curve() {
    for easing in ["linear", "easeIn", "standard", "easeInOut"] {
        let base = three_stops(easing);
        let added = insert_keyframe(&base, 380.0);
        assert_eq!(added.keyframes.len(), 4, "{easing}");
        assert_valid(&added);
        for step in 0..=100 {
            let p = step as f32 / 100.0;
            for property in ["opacity", "translateY"] {
                let (before, after) = (painted(&base, property, p), painted(&added, property, p));
                assert!(
                    (before - after).abs() < 1e-6,
                    "{easing} {property} at {p}: {before} vs {after}"
                );
            }
        }
        // Adding at an existing keyframe changes nothing.
        assert_eq!(insert_keyframe(&base, 100.0), base);
    }
}

#[test]
fn adding_a_keyframe_outside_the_strip_grows_it() {
    let base = three_stops("linear");
    let before = times(&base);
    let earlier = insert_keyframe(&base, 20.0);
    assert_eq!((earlier.delay_ms, earlier.duration_ms), (20, 480));
    assert_eq!(earlier.keyframes[0].values, base.keyframes[0].values);
    let after = times(&earlier);
    for (old, new) in before.iter().zip(&after[1..]) {
        assert!((old - new).abs() < 0.5, "{before:?} {after:?}");
    }
    let later = insert_keyframe(&base, 700.0);
    assert_eq!((later.delay_ms, later.duration_ms), (100, 600));
    assert_eq!(
        later.keyframes.last().unwrap().values["opacity"].as_f64(),
        Some(1.0)
    );
    assert_valid(&earlier);
    assert_valid(&later);
}

#[test]
fn deleting_keyframes_hands_the_ends_to_the_neighbours() {
    let base = three_stops("linear");
    let without_first = delete_keyframe(&base, 0).expect("still animated");
    assert_eq!(
        (without_first.delay_ms, without_first.duration_ms),
        (200, 300)
    );
    assert_eq!(without_first.keyframes[0].values, base.keyframes[1].values);
    assert_valid(&without_first);

    let without_middle = delete_keyframe(&base, 1).expect("still animated");
    assert_eq!(without_middle.keyframes.len(), 2);
    assert_eq!(
        (without_middle.delay_ms, without_middle.duration_ms),
        (100, 400)
    );

    // Two keyframes left: deleting one removes the animation.
    assert_eq!(delete_keyframe(&without_middle, 0), None);
}

#[test]
fn changing_the_curve_keeps_every_keyframe_at_its_time() {
    let base = three_stops("linear");
    let before = times(&base);
    for easing in [Easing::EaseIn, Easing::Standard, Easing::EaseInOut] {
        let eased = set_easing_keep_times(&base, easing.clone());
        assert_eq!(eased.easing, easing);
        let after = times(&eased);
        for (old, new) in before.iter().zip(&after) {
            assert!((old - new).abs() < 0.5, "{easing:?}: {before:?} {after:?}");
        }
        assert_valid(&eased);
    }
}

#[test]
fn shifting_a_selection_moves_it_together() {
    let base = animation(
        "linear",
        json!([
            {"offset": 0, "values": {"opacity": 0}},
            {"offset": 0.25, "values": {"opacity": 0.4}},
            {"offset": 0.5, "values": {"opacity": 0.6}},
            {"offset": 1, "values": {"opacity": 1}}
        ]),
    );
    let shifted = shift_keyframes(&base, &[1, 2], 40.0);
    assert!((shifted.keyframes[1].offset - 0.35).abs() < 1e-6);
    assert!((shifted.keyframes[2].offset - 0.6).abs() < 1e-6);
    // Moved keyframes stop at the last keyframe, which stayed put.
    let far = shift_keyframes(&base, &[1, 2], 10_000.0);
    assert_eq!(far.keyframes[2].offset, 1.0);
    assert_valid(&far);
    // Selecting everything moves the strip.
    let all = shift_keyframes(&base, &[0, 1, 2, 3], 50.0);
    assert_eq!(all.delay_ms, 150);
    assert_eq!(all.keyframes, base.keyframes);
}
