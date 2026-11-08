//! Motion edit commands: the undoable writes behind the Motion section and
//! the timeline panel.
//!
//! Every command writes only the node-level `animations` and `transition`
//! fields, and is checked against the schema's own load rules
//! (`validate_animations`) before anything changes, so a refused command
//! leaves the document untouched and an accepted one never produces a
//! document that would fail to load. Timeline drags edit a UI draft and
//! submit one command (or one [`MotionCommand`] list) when they end, which
//! makes each gesture one undo step.

use crate::motion_presets::MotionPreset;
use crate::NodeId;
use jian_ops_schema::motion::{Easing, Keyframe, MotionTrigger, NodeAnimation, Transition};

#[derive(Debug, Clone, PartialEq)]
pub enum MotionCommand {
    /// Insert `animation` at `index`, or append it.
    AddAnimation {
        node_id: NodeId,
        animation: NodeAnimation,
        index: Option<usize>,
    },
    /// Replace animation `index` wholesale (an inspector field edit or a
    /// timeline gesture computed by `motion_timeline_edit`).
    UpdateAnimation {
        node_id: NodeId,
        index: usize,
        animation: NodeAnimation,
    },
    RemoveAnimation {
        node_id: NodeId,
        index: usize,
    },
    /// Move animation `from` to position `to` (clamped to the list).
    ReorderAnimation {
        node_id: NodeId,
        from: usize,
        to: usize,
    },
    SetKeyframes {
        node_id: NodeId,
        index: usize,
        keyframes: Vec<Keyframe>,
    },
    /// Set or clear the node's `transition`.
    SetTransition {
        node_id: NodeId,
        transition: Option<Transition>,
    },
    /// Append `preset` to every node (multi-select applies it to each).
    /// A manual trigger needs `id`.
    ApplyPreset {
        node_ids: Vec<NodeId>,
        preset: MotionPreset,
        trigger: MotionTrigger,
        id: Option<String>,
    },
    /// Append `preset` to the container's direct children with
    /// `delay = base + i × step`.
    Stagger {
        container_id: NodeId,
        preset: MotionPreset,
        trigger: MotionTrigger,
        id: Option<String>,
        base_delay_ms: u64,
        step_ms: u64,
    },
    /// Set delay and duration on several animations at once (moving or
    /// stretching a timeline selection).
    RetimeMany {
        retimes: Vec<MotionRetime>,
    },
    /// Change animation `index`'s curve while every keyframe keeps its time.
    SetEasingKeepTimes {
        node_id: NodeId,
        index: usize,
        easing: Easing,
    },
}

/// New timing for one animation in [`MotionCommand::RetimeMany`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionRetime {
    pub node_id: NodeId,
    pub index: usize,
    pub delay_ms: u64,
    pub duration_ms: u64,
}

/// Why a motion command was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotionEditError {
    /// No node with this id on the active page.
    NodeMissing(NodeId),
    AnimationMissing {
        node_id: NodeId,
        index: usize,
    },
    /// A stagger target with no children.
    NothingToStagger(NodeId),
    /// The result would break a schema rule; `path` is under the node's
    /// `animations`.
    Invalid {
        node_id: NodeId,
        path: String,
        reason: String,
    },
}

impl std::fmt::Display for MotionEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeMissing(node_id) => write!(f, "node {} not found", node_id.as_str()),
            Self::AnimationMissing { node_id, index } => {
                write!(f, "node {} has no animation {index}", node_id.as_str())
            }
            Self::NothingToStagger(node_id) => {
                write!(f, "node {} has no children to stagger", node_id.as_str())
            }
            Self::Invalid {
                node_id,
                path,
                reason,
            } => write!(f, "node {}: {path}: {reason}", node_id.as_str()),
        }
    }
}

impl std::error::Error for MotionEditError {}

#[cfg(test)]
#[path = "motion_edit_tests.rs"]
mod tests;
