//! Applying [`MotionCommand`]s to the editor state.
//!
//! A command is first planned against a copy of each node's motion fields,
//! every touched node is checked with the schema's load rules, and only then
//! is anything written. A list of commands plans in order (a later command
//! sees an earlier one's result), so a multi-select gesture or a preset over
//! many nodes is all-or-nothing and one undo step.

use crate::motion_edit::{MotionCommand, MotionEditError};
use crate::motion_presets::{stagger_delay, MotionPreset};
use crate::motion_timeline_edit::set_easing_keep_times;
use crate::pen_node_ext::PenNodeExt;
use crate::walkers::{find_node, find_node_mut};
use crate::{EditorCommand, EditorState, NodeId};
use jian_ops_schema::motion::{validate_animations, MotionTrigger, NodeAnimation, Transition};
use std::collections::BTreeMap;

/// A node's motion fields as a plan sees them.
#[derive(Debug, Clone)]
struct NodeMotion {
    node_id: NodeId,
    transition: Option<Transition>,
    animations: Vec<NodeAnimation>,
}

/// Planned writes, keyed by node id.
type Plan = BTreeMap<String, NodeMotion>;

impl EditorState {
    /// Whether `cmd` would apply, and why not.
    pub fn check_motion_command(&self, cmd: &MotionCommand) -> Result<(), MotionEditError> {
        self.plan_motion(std::slice::from_ref(cmd)).map(|_| ())
    }

    /// Apply `commands` as one undo step, all or nothing. Returns whether
    /// the document changed.
    pub fn apply_motion_edits(
        &mut self,
        commands: Vec<MotionCommand>,
    ) -> Result<bool, MotionEditError> {
        let plan = self.plan_motion(&commands)?;
        if !self.motion_plan_changes(&plan) {
            return Ok(false);
        }
        self.commit_history();
        let mut changed = false;
        for cmd in commands {
            changed |= self.apply(EditorCommand::Motion(cmd));
        }
        Ok(changed)
    }

    /// `EditorCommand::Motion`: no history entry of its own (the caller
    /// brackets it, like every other command); a refused command is a no-op.
    pub(crate) fn apply_motion_command(&mut self, cmd: MotionCommand) -> bool {
        match self.plan_motion(std::slice::from_ref(&cmd)) {
            Ok(plan) => self.write_motion_plan(plan),
            Err(_) => false,
        }
    }

    fn plan_motion(&self, commands: &[MotionCommand]) -> Result<Plan, MotionEditError> {
        let mut plan = Plan::new();
        for cmd in commands {
            self.plan_motion_command(cmd, &mut plan)?;
        }
        for motion in plan.values() {
            validate_animations(&motion.animations, "animations").map_err(|(path, reason)| {
                MotionEditError::Invalid {
                    node_id: motion.node_id.clone(),
                    path,
                    reason,
                }
            })?;
        }
        Ok(plan)
    }

    fn plan_motion_command(
        &self,
        cmd: &MotionCommand,
        plan: &mut Plan,
    ) -> Result<(), MotionEditError> {
        match cmd {
            MotionCommand::AddAnimation {
                node_id,
                animation,
                index,
            } => {
                let motion = self.planned(plan, node_id)?;
                let at = index
                    .unwrap_or(motion.animations.len())
                    .min(motion.animations.len());
                motion.animations.insert(at, animation.clone());
            }
            MotionCommand::UpdateAnimation {
                node_id,
                index,
                animation,
            } => *planned_animation(self.planned(plan, node_id)?, *index)? = animation.clone(),
            MotionCommand::RemoveAnimation { node_id, index } => {
                let motion = self.planned(plan, node_id)?;
                planned_animation(motion, *index)?;
                motion.animations.remove(*index);
            }
            MotionCommand::ReorderAnimation { node_id, from, to } => {
                let motion = self.planned(plan, node_id)?;
                planned_animation(motion, *from)?;
                let animation = motion.animations.remove(*from);
                let at = (*to).min(motion.animations.len());
                motion.animations.insert(at, animation);
            }
            MotionCommand::SetKeyframes {
                node_id,
                index,
                keyframes,
            } => {
                planned_animation(self.planned(plan, node_id)?, *index)?.keyframes =
                    keyframes.clone()
            }
            MotionCommand::SetTransition {
                node_id,
                transition,
            } => self.planned(plan, node_id)?.transition = transition.clone(),
            MotionCommand::ApplyPreset {
                node_ids,
                preset,
                trigger,
                id,
            } => {
                for node_id in node_ids {
                    let animation = preset_animation(*preset, *trigger, id.as_deref(), 0);
                    self.planned(plan, node_id)?.animations.push(animation);
                }
            }
            MotionCommand::Stagger {
                container_id,
                preset,
                trigger,
                id,
                base_delay_ms,
                step_ms,
            } => {
                let container = find_node(self.active_children(), container_id)
                    .ok_or_else(|| MotionEditError::NodeMissing(container_id.clone()))?;
                let children: Vec<NodeId> = container
                    .children()
                    .into_iter()
                    .flatten()
                    .map(|child| NodeId::new(child.id_str()))
                    .collect();
                if children.is_empty() {
                    return Err(MotionEditError::NothingToStagger(container_id.clone()));
                }
                for (index, child) in children.iter().enumerate() {
                    let delay = stagger_delay(*base_delay_ms, *step_ms, index);
                    let animation = preset_animation(*preset, *trigger, id.as_deref(), delay);
                    self.planned(plan, child)?.animations.push(animation);
                }
            }
            MotionCommand::RetimeMany { retimes } => {
                for retime in retimes {
                    let animation =
                        planned_animation(self.planned(plan, &retime.node_id)?, retime.index)?;
                    animation.delay_ms = retime.delay_ms;
                    animation.duration_ms = retime.duration_ms;
                }
            }
            MotionCommand::SetEasingKeepTimes {
                node_id,
                index,
                easing,
            } => {
                let animation = planned_animation(self.planned(plan, node_id)?, *index)?;
                *animation = set_easing_keep_times(animation, easing.clone());
            }
        }
        Ok(())
    }

    /// The node's motion as planned so far, read from the document the
    /// first time a command touches it.
    fn planned<'a>(
        &self,
        plan: &'a mut Plan,
        node_id: &NodeId,
    ) -> Result<&'a mut NodeMotion, MotionEditError> {
        if !plan.contains_key(node_id.as_str()) {
            let node = find_node(self.active_children(), node_id)
                .ok_or_else(|| MotionEditError::NodeMissing(node_id.clone()))?;
            let (transition, animations) = node.motion_declarations();
            plan.insert(
                node_id.as_str().to_owned(),
                NodeMotion {
                    node_id: node_id.clone(),
                    transition: transition.cloned(),
                    animations: animations.cloned().unwrap_or_default(),
                },
            );
        }
        Ok(plan
            .get_mut(node_id.as_str())
            .expect("inserted above when missing"))
    }

    fn motion_plan_changes(&self, plan: &Plan) -> bool {
        plan.values().any(|motion| {
            find_node(self.active_children(), &motion.node_id).is_some_and(|node| {
                let (transition, animations) = node.motion_declarations();
                transition != motion.transition.as_ref()
                    || animations.map_or(&[][..], Vec::as_slice) != motion.animations.as_slice()
            })
        })
    }

    fn write_motion_plan(&mut self, plan: Plan) -> bool {
        let mut changed = false;
        for motion in plan.into_values() {
            let Some(node) = find_node_mut(self.active_children_mut(), &motion.node_id) else {
                continue;
            };
            let base = node.base_mut();
            let animations = (!motion.animations.is_empty()).then_some(motion.animations);
            if base.animations != animations || base.transition != motion.transition {
                base.animations = animations;
                base.transition = motion.transition;
                changed = true;
            }
        }
        changed
    }
}

fn planned_animation(
    motion: &mut NodeMotion,
    index: usize,
) -> Result<&mut NodeAnimation, MotionEditError> {
    let node_id = motion.node_id.clone();
    motion
        .animations
        .get_mut(index)
        .ok_or(MotionEditError::AnimationMissing { node_id, index })
}

fn preset_animation(
    preset: MotionPreset,
    trigger: MotionTrigger,
    id: Option<&str>,
    delay_ms: u64,
) -> NodeAnimation {
    let mut animation = preset.animation(trigger);
    animation.id = id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned);
    animation.delay_ms = delay_ms;
    animation
}

#[cfg(test)]
#[path = "motion_edit_apply_tests.rs"]
mod tests;
