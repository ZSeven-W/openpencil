use super::*;
use jian_ops_schema::motion::validate_animations;

#[test]
fn every_preset_is_a_valid_animation_for_every_trigger() {
    for preset in MotionPreset::ALL {
        for trigger in [MotionTrigger::Mount, MotionTrigger::InView] {
            let animation = preset.animation(trigger);
            assert_eq!(validate_animations(&[animation], "$"), Ok(()), "{preset:?}");
        }
        // Manual motion is valid once the caller names it.
        let mut manual = preset.animation(MotionTrigger::Manual);
        manual.id = Some("play".to_owned());
        assert_eq!(validate_animations(&[manual], "$"), Ok(()), "{preset:?}");
    }
}

#[test]
fn presets_carry_the_documented_shapes() {
    let slide = MotionPreset::SlideUp.animation(MotionTrigger::Mount);
    assert_eq!(slide.duration_ms, 400);
    assert_eq!(slide.easing, Easing::EmphasizedDecelerate);
    assert_eq!(slide.keyframes[0].values["translateY"], json!(16.0));
    assert_eq!(slide.keyframes[1].values["translateY"], json!(0));

    let left = MotionPreset::SlideLeft.animation(MotionTrigger::Mount);
    assert_eq!(left.keyframes[0].values["translateX"], json!(16.0));

    let pop = MotionPreset::Pop.animation(MotionTrigger::Mount);
    assert_eq!(pop.keyframes.len(), 3);
    assert_eq!(pop.easing, Easing::Linear);

    let pulse = MotionPreset::Pulse.animation(MotionTrigger::Mount);
    assert_eq!(pulse.iterations, Iterations::Infinite);
    assert_eq!(pulse.duration_ms, 1200);

    let custom = MotionPreset::Custom.animation(MotionTrigger::Mount);
    assert!(custom
        .keyframes
        .iter()
        .all(|keyframe| keyframe.values.is_empty()));
}

#[test]
fn preset_labels_are_unique() {
    let mut keys: Vec<_> = MotionPreset::ALL
        .iter()
        .map(|preset| preset.label().0)
        .collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), MotionPreset::ALL.len());
}

#[test]
fn stagger_spaces_children_by_the_step() {
    assert_eq!(stagger_delay(100, STAGGER_STEP_MS, 0), 100);
    assert_eq!(stagger_delay(100, STAGGER_STEP_MS, 3), 280);
    assert_eq!(stagger_delay(u64::MAX, 1, 2), u64::MAX);
}
