//! The motion timeline as a projection of node declarations.
//!
//! There is no timeline object in the document: the panel lays one screen's
//! animations for one trigger out on absolute time. Because Preview eases a
//! whole animation and places keyframes in eased progress, keyframe `k` of
//! an animation starting at `s` with iteration length `d` sits at
//! `s + d * E⁻¹(offset)`, which is plain `s + d * offset` only for linear
//! easing. Projecting never writes anything back; edits go through
//! `motion_timeline_edit`.

use crate::pen_node_ext::PenNodeExt;
use jian_core::action::services::Easing;
use jian_core::motion::{inverse, ClipTrigger};
use jian_ops_schema::motion::{Iterations, MotionTrigger, NodeAnimation};
use jian_ops_schema::node::PenNode;

/// One animation laid out on the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineStrip {
    pub node_id: String,
    /// Index into the node's `animations`.
    pub animation: usize,
    /// `delayMs`.
    pub start_ms: f64,
    /// One iteration, `durationMs`.
    pub iteration_ms: f64,
    /// End of the last iteration; `None` when it loops forever.
    pub end_ms: Option<f64>,
    /// Each keyframe's time in the first iteration.
    pub keyframe_ms: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TimelineProjection {
    pub strips: Vec<TimelineStrip>,
    /// How long the panel's clip runs: the latest finite end, or two of the
    /// longest iterations when every animation loops forever.
    pub clip_ms: f64,
}

/// Every animation under `root` (itself included) that `trigger` starts, in
/// document order. The manual tab collects the animations sharing an id.
pub fn project(root: &PenNode, trigger: &ClipTrigger) -> TimelineProjection {
    let mut strips = Vec::new();
    collect(root, trigger, &mut strips);
    let clip_ms = clip_ms(&strips);
    TimelineProjection { strips, clip_ms }
}

/// Lay out one animation.
pub fn strip(node_id: &str, index: usize, animation: &NodeAnimation) -> TimelineStrip {
    let start_ms = animation.delay_ms as f64;
    let iteration_ms = animation.duration_ms as f64;
    let end_ms = match animation.iterations {
        Iterations::Infinite => None,
        Iterations::Count(count) => Some(start_ms + iteration_ms * f64::from(count.max(1))),
    };
    TimelineStrip {
        node_id: node_id.to_owned(),
        animation: index,
        start_ms,
        iteration_ms,
        end_ms,
        keyframe_ms: animation
            .keyframes
            .iter()
            .map(|keyframe| keyframe_ms(animation, keyframe.offset))
            .collect(),
    }
}

/// Where an eased-progress `offset` lands in time on the first iteration.
pub fn keyframe_ms(animation: &NodeAnimation, offset: f32) -> f64 {
    let easing = Easing::from(&animation.easing);
    animation.delay_ms as f64 + animation.duration_ms as f64 * f64::from(inverse(easing, offset))
}

/// The manual animation ids used under `root`, sorted, for the manual tab.
pub fn manual_ids(root: &PenNode) -> Vec<String> {
    let mut ids = Vec::new();
    collect_manual_ids(root, &mut ids);
    ids.sort();
    ids.dedup();
    ids
}

fn clip_ms(strips: &[TimelineStrip]) -> f64 {
    let finite = strips
        .iter()
        .filter_map(|strip| strip.end_ms)
        .fold(0.0, f64::max);
    if strips.iter().any(|strip| strip.end_ms.is_some()) {
        return finite;
    }
    strips
        .iter()
        .map(|strip| strip.start_ms + strip.iteration_ms * 2.0)
        .fold(0.0, f64::max)
}

fn started_by(animation: &NodeAnimation, trigger: &ClipTrigger) -> bool {
    match trigger {
        ClipTrigger::Mount => animation.trigger == MotionTrigger::Mount,
        ClipTrigger::InView => animation.trigger == MotionTrigger::InView,
        ClipTrigger::Manual(id) => {
            animation.trigger == MotionTrigger::Manual
                && animation.id.as_deref().map(str::trim) == Some(id.trim())
        }
    }
}

fn collect(node: &PenNode, trigger: &ClipTrigger, out: &mut Vec<TimelineStrip>) {
    if let Some(animations) = node.motion_declarations().1 {
        for (index, animation) in animations.iter().enumerate() {
            if started_by(animation, trigger) {
                out.push(strip(node.id_str(), index, animation));
            }
        }
    }
    for child in node.children().into_iter().flatten() {
        collect(child, trigger, out);
    }
}

fn collect_manual_ids(node: &PenNode, out: &mut Vec<String>) {
    if let Some(animations) = node.motion_declarations().1 {
        out.extend(
            animations
                .iter()
                .filter(|animation| animation.trigger == MotionTrigger::Manual)
                .filter_map(|animation| animation.id.as_deref())
                .map(|id| id.trim().to_owned())
                .filter(|id| !id.is_empty()),
        );
    }
    for child in node.children().into_iter().flatten() {
        collect_manual_ids(child, out);
    }
}

#[cfg(test)]
#[path = "motion_timeline_projection_tests.rs"]
mod tests;
