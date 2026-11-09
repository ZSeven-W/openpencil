//! What the Motion section lists for a node, and the labels it paints.
//!
//! One row per animation: its trigger, the preset it still matches (or
//! Custom), and the timing fields the row and its expanded editor show.
//! Label functions return an i18n key with its English fallback, like the
//! Interactions section's.

use crate::motion_presets::MotionPreset;
use crate::pen_node_ext::PenNodeExt;
use jian_ops_schema::motion::{
    Easing, Iterations, MotionTrigger, NodeAnimation, NodeAnimationFillMode, Transition,
};
use jian_ops_schema::node::PenNode;
use serde_json::Value;
use std::collections::BTreeMap;

/// Triggers in menu order.
pub const MOTION_TRIGGERS: [MotionTrigger; 3] = [
    MotionTrigger::Mount,
    MotionTrigger::InView,
    MotionTrigger::Manual,
];

/// The named curves the easing menu offers, in menu order. A custom
/// cubic-bezier keeps its value and shows as "Custom curve".
pub const MOTION_EASINGS: [Easing; 9] = [
    Easing::Standard,
    Easing::Emphasized,
    Easing::EmphasizedDecelerate,
    Easing::EmphasizedAccelerate,
    Easing::Linear,
    Easing::Ease,
    Easing::EaseIn,
    Easing::EaseOut,
    Easing::EaseInOut,
];

#[derive(Debug, Clone, PartialEq)]
pub struct MotionRow {
    pub trigger: MotionTrigger,
    /// The preset this animation still is, if any.
    pub preset: Option<MotionPreset>,
    pub duration_ms: u64,
    pub delay_ms: u64,
    pub easing: Easing,
    pub iterations: Iterations,
    pub fill_mode: NodeAnimationFillMode,
    pub once: bool,
    pub id: Option<String>,
}

impl MotionRow {
    pub fn of(animation: &NodeAnimation) -> Self {
        Self {
            trigger: animation.trigger,
            preset: matching_preset(animation),
            duration_ms: animation.duration_ms,
            delay_ms: animation.delay_ms,
            easing: animation.easing.clone(),
            iterations: animation.iterations,
            fill_mode: animation.fill_mode,
            once: animation.once,
            id: animation.id.clone(),
        }
    }

    /// The row's name: its preset, or Custom.
    pub fn title(&self) -> (&'static str, &'static str) {
        self.preset.unwrap_or(MotionPreset::Custom).label()
    }
}

/// Everything the Motion section needs about the selected node.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MotionSummary {
    pub rows: Vec<MotionRow>,
    pub transition: Option<Transition>,
    /// The node has direct children, so it can stagger a preset over them.
    pub can_stagger: bool,
}

impl MotionSummary {
    pub fn of(node: &PenNode) -> Self {
        let (transition, animations) = node.motion_declarations();
        Self {
            rows: animations
                .into_iter()
                .flatten()
                .map(MotionRow::of)
                .collect(),
            transition: transition.cloned(),
            can_stagger: node.children().is_some_and(|children| !children.is_empty()),
        }
    }
}

/// The preset `animation` still is: same keyframes, duration, curve and
/// repeat. Trigger, delay, id, fill mode and `once` are free, since applying
/// a preset leaves those to the person.
pub fn matching_preset(animation: &NodeAnimation) -> Option<MotionPreset> {
    MotionPreset::ALL.into_iter().find(|preset| {
        let shape = preset.animation(animation.trigger);
        shape.duration_ms == animation.duration_ms
            && shape.easing == animation.easing
            && shape.iterations == animation.iterations
            && shape.keyframes.len() == animation.keyframes.len()
            && shape
                .keyframes
                .iter()
                .zip(&animation.keyframes)
                .all(|(expected, actual)| {
                    expected.offset == actual.offset
                        && same_values(&expected.values, &actual.values)
                })
    })
}

pub fn trigger_label(trigger: MotionTrigger) -> (&'static str, &'static str) {
    match trigger {
        MotionTrigger::Mount => ("motion.trigger.mount", "On load"),
        MotionTrigger::InView => ("motion.trigger.inView", "In view"),
        MotionTrigger::Manual => ("motion.trigger.manual", "Manual"),
    }
}

pub fn easing_label(easing: &Easing) -> (&'static str, &'static str) {
    match easing {
        Easing::Linear => ("motion.easing.linear", "Linear"),
        Easing::Ease => ("motion.easing.ease", "Ease"),
        Easing::EaseIn => ("motion.easing.easeIn", "Ease in"),
        Easing::EaseOut => ("motion.easing.easeOut", "Ease out"),
        Easing::EaseInOut => ("motion.easing.easeInOut", "Ease in-out"),
        Easing::Standard => ("motion.easing.standard", "Standard"),
        Easing::Emphasized => ("motion.easing.emphasized", "Emphasized"),
        Easing::EmphasizedDecelerate => (
            "motion.easing.emphasizedDecelerate",
            "Emphasized decelerate",
        ),
        Easing::EmphasizedAccelerate => (
            "motion.easing.emphasizedAccelerate",
            "Emphasized accelerate",
        ),
        Easing::CubicBezier(..) => ("motion.easing.custom", "Custom curve"),
    }
}

pub fn fill_label(fill: NodeAnimationFillMode) -> (&'static str, &'static str) {
    match fill {
        NodeAnimationFillMode::Forwards => ("motion.fill.forwards", "Keep last frame"),
        NodeAnimationFillMode::None => ("motion.fill.none", "Return to start"),
    }
}

/// Numbers compare by value, so an authored `16` matches a preset's `16.0`.
fn same_values(expected: &BTreeMap<String, Value>, actual: &BTreeMap<String, Value>) -> bool {
    expected.len() == actual.len()
        && expected.iter().all(|(property, value)| {
            actual
                .get(property)
                .is_some_and(|other| match (value.as_f64(), other.as_f64()) {
                    (Some(a), Some(b)) => (a - b).abs() < 1e-9,
                    _ => value == other,
                })
        })
}

#[cfg(test)]
#[path = "motion_summary_tests.rs"]
mod tests;
