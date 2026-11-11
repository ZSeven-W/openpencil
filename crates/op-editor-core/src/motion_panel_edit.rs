//! The Motion section's edits on the selected node.
//!
//! The panel speaks in rows (`Remove(2)`, `SetTrigger(0, Manual)`); this
//! module turns each edit into [`MotionCommand`]s for the selected node and
//! applies them as one undo step through `apply_motion_edits`, so the
//! schema checks there guard every panel write.

use crate::motion_edit::MotionCommand;
use crate::motion_keyframe_edit::{
    add_keyframe_value, remove_keyframe_value, set_keyframe_value, with_bezier_control,
};
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
    SetDuration(usize, u64),
    SetDelay(usize, u64),
    /// Passes per run; at least one.
    SetRepeat(usize, u32),
    /// The id `play_motion` addresses; blank clears it.
    SetId(usize, String),
    AddTransition,
    RemoveTransition,
    SetTransitionEasing(Easing),
    SetTransitionDuration(u64),
    /// Set control `slot` (x1, y1, x2, y2) of the curve, which becomes a
    /// cubic-bezier; keyframes keep their times.
    SetCurveControl(usize, usize, f32),
    /// Keyframe-values mode: add a whitelisted property to keyframe
    /// `keyframe` of animation `animation`.
    AddKeyframeValue {
        animation: usize,
        keyframe: usize,
        property: String,
    },
    SetKeyframeValue {
        animation: usize,
        keyframe: usize,
        property: String,
        value: serde_json::Value,
    },
    RemoveKeyframeValue {
        animation: usize,
        keyframe: usize,
        property: String,
    },
}

impl EditorState {
    /// Apply one Motion-section edit to the selected node as one undo step.
    pub fn apply_selected_motion_edit(&mut self, edit: &MotionPanelEdit) -> bool {
        self.apply_selected_motion_edit_inner(edit, true)
    }

    /// [`Self::apply_selected_motion_edit`] without its own undo step, for
    /// the property-draft commit, which brackets its write already.
    pub fn apply_selected_motion_edit_untracked(&mut self, edit: &MotionPanelEdit) -> bool {
        self.apply_selected_motion_edit_inner(edit, false)
    }

    fn apply_selected_motion_edit_inner(&mut self, edit: &MotionPanelEdit, track: bool) -> bool {
        let node_id = self.selection.anchor.clone();
        if !node_id.is_real() {
            return false;
        }
        let Some(commands) = self.motion_panel_commands(&node_id, edit) else {
            return false;
        };
        if track {
            self.apply_motion_edits(commands).unwrap_or(false)
        } else {
            self.apply_motion_edits_untracked(commands).unwrap_or(false)
        }
    }

    fn motion_panel_commands(
        &self,
        node_id: &NodeId,
        edit: &MotionPanelEdit,
    ) -> Option<Vec<MotionCommand>> {
        let node = self.selected_node()?;
        let (transition, animations) = node.motion_declarations();
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
            MotionPanelEdit::SetDuration(index, ms) => {
                update(*index, &|animation| animation.duration_ms = *ms)?
            }
            MotionPanelEdit::SetDelay(index, ms) => {
                update(*index, &|animation| animation.delay_ms = *ms)?
            }
            MotionPanelEdit::SetRepeat(index, count) => update(*index, &|animation| {
                animation.iterations = Iterations::Count((*count).max(1));
            })?,
            MotionPanelEdit::SetId(index, id) => update(*index, &|animation| {
                let id = id.trim();
                animation.id = (!id.is_empty()).then(|| id.to_owned());
            })?,
            MotionPanelEdit::SetTransitionDuration(ms) => {
                let mut transition = transition?.clone();
                transition.duration_ms = *ms;
                vec![MotionCommand::SetTransition {
                    node_id: node_id.clone(),
                    transition: Some(transition),
                }]
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
            MotionPanelEdit::SetCurveControl(index, slot, value) => {
                let current = &animations.get(*index)?.easing;
                vec![MotionCommand::SetEasingKeepTimes {
                    node_id: node_id.clone(),
                    index: *index,
                    easing: with_bezier_control(current, *slot, *value),
                }]
            }
            MotionPanelEdit::AddKeyframeValue {
                animation,
                keyframe,
                property,
            } => update(*animation, &|edited| {
                *edited = add_keyframe_value(edited, *keyframe, property, node);
            })?,
            MotionPanelEdit::SetKeyframeValue {
                animation,
                keyframe,
                property,
                value,
            } => update(*animation, &|edited| {
                *edited = set_keyframe_value(edited, *keyframe, property, value.clone());
            })?,
            MotionPanelEdit::RemoveKeyframeValue {
                animation,
                keyframe,
                property,
            } => update(*animation, &|edited| {
                *edited = remove_keyframe_value(edited, *keyframe, property);
            })?,
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
