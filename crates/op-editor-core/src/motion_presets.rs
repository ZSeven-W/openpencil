//! The Motion section's preset menu and the stagger rule.
//!
//! Every preset is an ordinary [`NodeAnimation`] built from existing schema
//! fields, so applying one writes nothing the document format does not
//! already have. Shapes that need more than the curve (pop, pulse) carry an
//! extra keyframe and play linearly, so they look the same whatever the
//! runtime's curve semantics.

use jian_ops_schema::events::ExtraJson;
use jian_ops_schema::motion::{
    Easing, Iterations, Keyframe, MotionTrigger, NodeAnimation, NodeAnimationFillMode,
};
use serde_json::{json, Value};

/// Gap between staggered children unless the person picks another.
pub const STAGGER_STEP_MS: u64 = 60;

/// Presets in menu order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionPreset {
    FadeIn,
    SlideUp,
    SlideDown,
    SlideLeft,
    SlideRight,
    ScaleIn,
    Pop,
    RotateIn,
    Pulse,
    Spin,
    /// Two empty keyframes to fill in by hand.
    Custom,
}

impl MotionPreset {
    pub const ALL: [MotionPreset; 11] = [
        Self::FadeIn,
        Self::SlideUp,
        Self::SlideDown,
        Self::SlideLeft,
        Self::SlideRight,
        Self::ScaleIn,
        Self::Pop,
        Self::RotateIn,
        Self::Pulse,
        Self::Spin,
        Self::Custom,
    ];

    /// i18n key + English fallback for the menu row.
    pub fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::FadeIn => ("motion.preset.fadeIn", "Fade in"),
            Self::SlideUp => ("motion.preset.slideUp", "Slide up"),
            Self::SlideDown => ("motion.preset.slideDown", "Slide down"),
            Self::SlideLeft => ("motion.preset.slideLeft", "Slide left"),
            Self::SlideRight => ("motion.preset.slideRight", "Slide right"),
            Self::ScaleIn => ("motion.preset.scaleIn", "Scale in"),
            Self::Pop => ("motion.preset.pop", "Pop"),
            Self::RotateIn => ("motion.preset.rotateIn", "Rotate in"),
            Self::Pulse => ("motion.preset.pulse", "Pulse"),
            Self::Spin => ("motion.preset.spin", "Spin"),
            Self::Custom => ("motion.preset.custom", "Custom"),
        }
    }

    /// The preset as an animation with `trigger`. A manual trigger also
    /// needs an id before the animation is valid; the caller supplies it.
    pub fn animation(self, trigger: MotionTrigger) -> NodeAnimation {
        let (keyframes, duration_ms, easing, iterations) = match self {
            Self::FadeIn => (
                two(json!({"opacity": 0}), json!({"opacity": 1})),
                300,
                Easing::Standard,
                Iterations::Count(1),
            ),
            Self::SlideUp => slide("translateY", 16.0),
            Self::SlideDown => slide("translateY", -16.0),
            // Slides toward the named side: left starts to the right.
            Self::SlideLeft => slide("translateX", 16.0),
            Self::SlideRight => slide("translateX", -16.0),
            Self::ScaleIn => (
                two(
                    json!({"opacity": 0, "scaleX": 0.92, "scaleY": 0.92}),
                    json!({"opacity": 1, "scaleX": 1, "scaleY": 1}),
                ),
                300,
                Easing::EmphasizedDecelerate,
                Iterations::Count(1),
            ),
            Self::Pop => (
                vec![
                    keyframe(0.0, json!({"scaleX": 0.8, "scaleY": 0.8})),
                    keyframe(0.6, json!({"scaleX": 1.05, "scaleY": 1.05})),
                    keyframe(1.0, json!({"scaleX": 1, "scaleY": 1})),
                ],
                400,
                Easing::Linear,
                Iterations::Count(1),
            ),
            Self::RotateIn => (
                two(
                    json!({"opacity": 0, "rotation": -8}),
                    json!({"opacity": 1, "rotation": 0}),
                ),
                400,
                Easing::Standard,
                Iterations::Count(1),
            ),
            Self::Pulse => (
                vec![
                    keyframe(0.0, json!({"scaleX": 1, "scaleY": 1})),
                    keyframe(0.5, json!({"scaleX": 1.04, "scaleY": 1.04})),
                    keyframe(1.0, json!({"scaleX": 1, "scaleY": 1})),
                ],
                1200,
                Easing::Linear,
                Iterations::Infinite,
            ),
            Self::Spin => (
                two(json!({"rotation": 0}), json!({"rotation": 360})),
                1000,
                Easing::Linear,
                Iterations::Infinite,
            ),
            Self::Custom => (
                two(json!({}), json!({})),
                300,
                Easing::Standard,
                Iterations::Count(1),
            ),
        };
        NodeAnimation {
            trigger,
            id: None,
            keyframes,
            duration_ms,
            delay_ms: 0,
            easing,
            iterations,
            fill_mode: NodeAnimationFillMode::Forwards,
            once: true,
            extra: ExtraJson::default(),
        }
    }
}

/// `delay = base + i × step` for the `index`-th staggered child.
pub fn stagger_delay(base_ms: u64, step_ms: u64, index: usize) -> u64 {
    base_ms.saturating_add(step_ms.saturating_mul(index as u64))
}

fn slide(property: &str, distance: f64) -> (Vec<Keyframe>, u64, Easing, Iterations) {
    (
        two(
            json!({"opacity": 0, property: distance}),
            json!({"opacity": 1, property: 0}),
        ),
        400,
        Easing::EmphasizedDecelerate,
        Iterations::Count(1),
    )
}

fn two(from: Value, to: Value) -> Vec<Keyframe> {
    vec![keyframe(0.0, from), keyframe(1.0, to)]
}

fn keyframe(offset: f32, values: Value) -> Keyframe {
    let Value::Object(values) = values else {
        unreachable!("presets spell their values as objects");
    };
    Keyframe {
        offset,
        values: values.into_iter().collect(),
        extra: ExtraJson::default(),
    }
}

#[cfg(test)]
#[path = "motion_presets_tests.rs"]
mod tests;
