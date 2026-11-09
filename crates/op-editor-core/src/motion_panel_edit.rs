//! The Motion section's edits on the selected node.
//!
//! The panel speaks in rows (`Remove(2)`, `SetTrigger(0, Manual)`); this
//! module turns each edit into [`MotionCommand`]s for the selected node and
//! applies them as one undo step through `apply_motion_edits`, so the
//! schema checks there guard every panel write.

use crate::motion_edit::MotionCommand;
use crate::motion_presets::{MotionPreset, STAGGER_STEP_MS};
use crate::{EditorState, NodeId};
use jian_ops_schema::events::ExtraJson;
use jian_ops_schema::motion::{
    Easing, Iterations, MotionTrigger, NodeAnimation, NodeAnimationFillMode, Transition,
};

/// Default transition the "+ Add transition" row writes.
pub const DEFAULT_TRANSITION_MS: u64 = 150;

#[derive(Debug, Clone, PartialEq)]
pub enum MotionPanelEdit {
    AddPreset(MotionPreset),
    /// Apply a preset to every direct child, `STAGGER_STEP_MS` apart.
    StaggerPreset(MotionPreset),
    Remove(usize),
    MoveUp(usize),
    MoveDown(usize),
    SetTrigger(usize, MotionTrigger),
    /// Change the curve; keyframes keep their times.
    SetEasing(usize, Easing),
    SetFill(usize, NodeAnimationFillMode),
    /// Loop forever, or back to one pass.
    ToggleLoop(usize),
    ToggleOnce(usize),
    AddTransition,
    RemoveTransition,
    SetTransitionEasing(Easing),
}

impl EditorState {
    /// Apply one Motion-section edit to the selected node as one undo step.
    pub fn apply_selected_motion_edit(&mut self, edit: &MotionPanelEdit) -> bool {
        let node_id = self.selection.anchor.clone();
        if !node_id.is_real() {
            return false;
        }
        let Some(commands) = self.motion_panel_commands(&node_id, edit) else {
            return false;
        };
        self.apply_motion_edits(commands).unwrap_or(false)
    }

    fn motion_panel_commands(
        &self,
        node_id: &NodeId,
        edit: &MotionPanelEdit,
    ) -> Option<Vec<MotionCommand>> {
        let (transition, animations) = self.selected_node()?.motion_declarations();
        let animations = animations.cloned().unwrap_or_default();
        let update = |index: usize, change: &dyn Fn(&mut NodeAnimation)| {
            let mut animation = animations.get(index)?.clone();
            change(&mut animation);
            Some(vec![MotionCommand::UpdateAnimation {
                node_id: node_id.clone(),
                index,
                animation,
            }])
        };
        Some(match edit {
            MotionPanelEdit::AddPreset(preset) => vec![MotionCommand::ApplyPreset {
                node_ids: vec![node_id.clone()],
                preset: *preset,
                trigger: MotionTrigger::Mount,
                id: None,
            }],
            MotionPanelEdit::StaggerPreset(preset) => vec![MotionCommand::Stagger {
                container_id: node_id.clone(),
                preset: *preset,
                trigger: MotionTrigger::Mount,
                id: None,
                base_delay_ms: 0,
                step_ms: STAGGER_STEP_MS,
            }],
            MotionPanelEdit::Remove(index) => vec![MotionCommand::RemoveAnimation {
                node_id: node_id.clone(),
                index: *index,
            }],
            MotionPanelEdit::MoveUp(index) => vec![MotionCommand::ReorderAnimation {
                node_id: node_id.clone(),
                from: *index,
                to: index.checked_sub(1)?,
            }],
            MotionPanelEdit::MoveDown(index) => vec![MotionCommand::ReorderAnimation {
                node_id: node_id.clone(),
                from: *index,
                to: index + 1,
            }],
            MotionPanelEdit::SetTrigger(index, trigger) => {
                let id = unused_motion_id(&animations);
                update(*index, &|animation| {
                    animation.trigger = *trigger;
                    // play_motion finds manual motion by id.
                    if *trigger == MotionTrigger::Manual && animation.id.is_none() {
                        animation.id = Some(id.clone());
                    }
                })?
            }
            MotionPanelEdit::SetEasing(index, easing) => vec![MotionCommand::SetEasingKeepTimes {
                node_id: node_id.clone(),
                index: *index,
                easing: easing.clone(),
            }],
            MotionPanelEdit::SetFill(index, fill) => {
                update(*index, &|animation| animation.fill_mode = *fill)?
            }
            MotionPanelEdit::ToggleLoop(index) => update(*index, &|animation| {
                animation.iterations = if animation.iterations.is_infinite() {
                    Iterations::Count(1)
                } else {
                    Iterations::Infinite
                };
            })?,
            MotionPanelEdit::ToggleOnce(index) => {
                update(*index, &|animation| animation.once = !animation.once)?
            }
            MotionPanelEdit::AddTransition => vec![MotionCommand::SetTransition {
                node_id: node_id.clone(),
                transition: Some(Transition {
                    duration_ms: DEFAULT_TRANSITION_MS,
                    easing: Easing::Standard,
                    properties: None,
                    extra: ExtraJson::default(),
                }),
            }],
            MotionPanelEdit::RemoveTransition => vec![MotionCommand::SetTransition {
                node_id: node_id.clone(),
                transition: None,
            }],
            MotionPanelEdit::SetTransitionEasing(easing) => {
                let mut transition = transition?.clone();
                transition.easing = easing.clone();
                vec![MotionCommand::SetTransition {
                    node_id: node_id.clone(),
                    transition: Some(transition),
                }]
            }
        })
    }
}

/// `motion-1`, `motion-2`, … : the first id no animation on the node uses.
fn unused_motion_id(animations: &[NodeAnimation]) -> String {
    (1..)
        .map(|n| format!("motion-{n}"))
        .find(|candidate| {
            !animations
                .iter()
                .any(|animation| animation.id.as_deref() == Some(candidate.as_str()))
        })
        .expect("an unused id exists")
}

#[cfg(test)]
#[path = "motion_panel_edit_tests.rs"]
mod tests;
