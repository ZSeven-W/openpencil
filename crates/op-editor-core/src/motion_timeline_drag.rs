//! Timeline gestures: drags, snapping, keyframe selection and deletion.
//!
//! A drag edits only `MotionTimelineState::drag` while the pointer moves;
//! releasing it turns the draft into motion commands through
//! `motion_timeline_edit` and applies them as one undo step, so an
//! abandoned drag leaves the document untouched.

use crate::editor_ui_state::{MotionTimelineDrag, TimelineDragAnchor, TimelineKeyframeRef};
use crate::motion_edit::MotionCommand;
use crate::motion_timeline_edit::{delete_keyframe, move_strip, resize_strip, shift_keyframes};
use crate::walkers::find_node;
use crate::{EditorState, NodeId};
use jian_ops_schema::motion::NodeAnimation;
use std::collections::BTreeMap;

/// Grid a drag snaps to when no keyframe or the playhead is in reach.
pub const SNAP_GRID_MS: f64 = 10.0;

/// `t` snapped to the nearest target within `threshold_ms`, else to the
/// 10 ms grid.
pub fn snap_time(t: f64, targets: &[f64], threshold_ms: f64) -> f64 {
    targets
        .iter()
        .copied()
        .filter(|target| (target - t).abs() <= threshold_ms)
        .min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
        .unwrap_or_else(|| (t / SNAP_GRID_MS).round() * SNAP_GRID_MS)
}

impl EditorState {
    /// Times a drag snaps to: every projected keyframe except the dragged
    /// ones, and the playhead.
    pub fn motion_snap_targets(&self, dragged: &[TimelineKeyframeRef]) -> Vec<f64> {
        let mut targets = vec![self.editor_ui.motion_timeline.playhead_ms];
        for strip in self.motion_timeline_projection().strips {
            for (keyframe, time) in strip.keyframe_ms.iter().enumerate() {
                let key = TimelineKeyframeRef {
                    node_id: strip.node_id.clone(),
                    animation: strip.animation,
                    keyframe,
                };
                if !dragged.contains(&key) {
                    targets.push(*time);
                }
            }
        }
        targets
    }

    /// Select `key`; `additive` toggles it within the current selection.
    pub fn select_motion_keyframe(&mut self, key: TimelineKeyframeRef, additive: bool) {
        let selection = &mut self.editor_ui.motion_timeline.selection;
        if !additive {
            selection.clear();
            selection.push(key);
        } else if let Some(at) = selection.iter().position(|existing| *existing == key) {
            selection.remove(at);
        } else {
            selection.push(key);
        }
    }

    pub fn clear_motion_keyframe_selection(&mut self) {
        self.editor_ui.motion_timeline.selection.clear();
    }

    pub fn begin_motion_timeline_drag(&mut self, drag: MotionTimelineDrag) {
        self.editor_ui.motion_timeline.drag = Some(drag);
        self.editor_ui.motion_timeline.drag_anchor = None;
    }

    /// Start a drag that [`Self::drag_motion_timeline_to`] follows from
    /// `anchor`.
    pub fn begin_motion_timeline_drag_at(
        &mut self,
        drag: MotionTimelineDrag,
        anchor: TimelineDragAnchor,
    ) {
        self.begin_motion_timeline_drag(drag);
        self.editor_ui.motion_timeline.drag_anchor = Some(anchor);
    }

    /// Follow the pointer to `pointer_ms`. The held time moves by the
    /// pointer's travel since the anchor and snaps to another keyframe or
    /// the playhead within `snap_ms`, else to the grid; `None` (the
    /// modifier held) leaves it where the pointer puts it. A scrub moves
    /// the playhead to the pointer.
    pub fn drag_motion_timeline_to(&mut self, pointer_ms: f64, snap_ms: Option<f64>, now_ms: u64) {
        let timeline = &self.editor_ui.motion_timeline;
        let Some(drag) = timeline.drag.clone() else {
            return;
        };
        let Some(anchor) = timeline.drag_anchor else {
            if drag == MotionTimelineDrag::Playhead {
                self.update_motion_timeline_drag(pointer_ms, now_ms);
            }
            return;
        };
        let held = (anchor.held_ms + pointer_ms - anchor.pointer_ms).max(0.0);
        let snapped = match snap_ms {
            Some(snap_ms) => snap_time(
                held,
                &self.motion_snap_targets(&self.held_keys(&drag)),
                snap_ms,
            ),
            None => held,
        };
        let value = match drag {
            MotionTimelineDrag::StripEnd { .. } | MotionTimelineDrag::Playhead => snapped,
            MotionTimelineDrag::Strip { .. } | MotionTimelineDrag::Keyframes { .. } => {
                snapped - anchor.held_ms
            }
            MotionTimelineDrag::Height { .. } => return,
        };
        self.update_motion_timeline_drag(value, now_ms);
    }

    /// Follow a top-edge resize to `pointer_y`: dragging up grows the
    /// panel, never past `max_height` (the share of the canvas area it
    /// may take) nor out of the design's range.
    pub fn resize_motion_timeline_to(&mut self, pointer_y: f32, max_height: f32) {
        let timeline = &mut self.editor_ui.motion_timeline;
        let Some(MotionTimelineDrag::Height {
            pointer_y: from,
            height,
        }) = timeline.drag
        else {
            return;
        };
        timeline.set_height((height + from - pointer_y).min(max_height));
    }

    /// Keyframes a drag carries, left out of its own snap targets: the
    /// dragged ones, or every keyframe of a moved or stretched strip.
    fn held_keys(&self, drag: &MotionTimelineDrag) -> Vec<TimelineKeyframeRef> {
        let (node_id, animation) = match drag {
            MotionTimelineDrag::Keyframes { keys, .. } => return keys.clone(),
            MotionTimelineDrag::Playhead | MotionTimelineDrag::Height { .. } => return Vec::new(),
            MotionTimelineDrag::Strip {
                node_id, animation, ..
            }
            | MotionTimelineDrag::StripEnd {
                node_id, animation, ..
            } => (node_id, *animation),
        };
        let count = self
            .motion_timeline_projection()
            .strips
            .iter()
            .find(|strip| strip.node_id == *node_id && strip.animation == animation)
            .map_or(0, |strip| strip.keyframe_ms.len());
        (0..count)
            .map(|keyframe| TimelineKeyframeRef {
                node_id: node_id.clone(),
                animation,
                keyframe,
            })
            .collect()
    }

    /// Feed the pointer: a delta for strip and keyframe drags, the new end
    /// for an end drag, the playhead position for a scrub.
    pub fn update_motion_timeline_drag(&mut self, value_ms: f64, now_ms: u64) {
        let Some(drag) = self.editor_ui.motion_timeline.drag.as_mut() else {
            return;
        };
        match drag {
            MotionTimelineDrag::Strip { delta_ms, .. }
            | MotionTimelineDrag::Keyframes { delta_ms, .. } => *delta_ms = value_ms,
            MotionTimelineDrag::StripEnd { end_ms, .. } => *end_ms = value_ms,
            MotionTimelineDrag::Playhead => self.scrub_motion_playhead(value_ms, now_ms),
            MotionTimelineDrag::Height { .. } => {}
        }
    }

    /// Drop the drag without writing anything; a resize goes back to the
    /// height it started from. Returns whether a drag was in flight.
    pub fn cancel_motion_timeline_drag(&mut self) -> bool {
        let timeline = &mut self.editor_ui.motion_timeline;
        timeline.drag_anchor = None;
        match timeline.drag.take() {
            Some(MotionTimelineDrag::Height { height, .. }) => {
                timeline.set_height(height);
                true
            }
            Some(_) => true,
            None => false,
        }
    }

    /// Release the drag: write its draft as one undo step. Returns whether
    /// the document changed.
    pub fn finish_motion_timeline_drag(&mut self) -> bool {
        self.editor_ui.motion_timeline.drag_anchor = None;
        let Some(drag) = self.editor_ui.motion_timeline.drag.take() else {
            return false;
        };
        let mut commands = Vec::new();
        match drag {
            MotionTimelineDrag::Strip {
                node_id,
                animation,
                delta_ms,
            } => {
                if let Some(current) = self.motion_animation(&node_id, animation) {
                    commands.push(update(
                        &node_id,
                        animation,
                        move_strip(&current, delta_ms.round() as i64),
                    ));
                }
            }
            MotionTimelineDrag::StripEnd {
                node_id,
                animation,
                end_ms,
            } => {
                if let Some(current) = self.motion_animation(&node_id, animation) {
                    commands.push(update(&node_id, animation, resize_strip(&current, end_ms)));
                }
            }
            MotionTimelineDrag::Keyframes { keys, delta_ms } => {
                for ((node_id, animation), indices) in by_animation(&keys) {
                    if let Some(current) = self.motion_animation(&node_id, animation) {
                        let shifted = shift_keyframes(&current, &indices, delta_ms);
                        commands.push(update(&node_id, animation, shifted));
                    }
                }
            }
            MotionTimelineDrag::Playhead | MotionTimelineDrag::Height { .. } => return false,
        }
        self.apply_motion_edits(commands).unwrap_or(false)
    }

    /// Delete the selected keyframes as one undo step. An animation left
    /// with fewer than two keyframes is removed.
    pub fn delete_selected_motion_keyframes(&mut self) -> bool {
        let selection = std::mem::take(&mut self.editor_ui.motion_timeline.selection);
        let mut updates = Vec::new();
        let mut removals: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for ((node_id, animation), mut indices) in by_animation(&selection) {
            let Some(mut current) = self.motion_animation(&node_id, animation) else {
                continue;
            };
            indices.sort_unstable_by(|a, b| b.cmp(a));
            let mut removed = false;
            for index in indices {
                match delete_keyframe(&current, index) {
                    Some(next) => current = next,
                    None => {
                        removed = true;
                        break;
                    }
                }
            }
            if removed {
                removals.entry(node_id).or_default().push(animation);
            } else {
                updates.push(update(&node_id, animation, current));
            }
        }
        // Updates first, while indices still hold; then removals from the
        // highest index down so earlier ones do not shift.
        let mut commands = updates;
        for (node_id, mut animations) in removals {
            animations.sort_unstable_by(|a, b| b.cmp(a));
            commands.extend(
                animations
                    .into_iter()
                    .map(|index| MotionCommand::RemoveAnimation {
                        node_id: NodeId::new(node_id.clone()),
                        index,
                    }),
            );
        }
        self.apply_motion_edits(commands).unwrap_or(false)
    }

    fn motion_animation(&self, node_id: &str, index: usize) -> Option<NodeAnimation> {
        find_node(self.active_children(), &NodeId::new(node_id))?
            .motion_declarations()
            .1?
            .get(index)
            .cloned()
    }
}

fn update(node_id: &str, index: usize, animation: NodeAnimation) -> MotionCommand {
    MotionCommand::UpdateAnimation {
        node_id: NodeId::new(node_id),
        index,
        animation,
    }
}

/// Keyframe indices grouped by the animation they belong to.
fn by_animation(keys: &[TimelineKeyframeRef]) -> BTreeMap<(String, usize), Vec<usize>> {
    let mut groups: BTreeMap<(String, usize), Vec<usize>> = BTreeMap::new();
    for key in keys {
        groups
            .entry((key.node_id.clone(), key.animation))
            .or_default()
            .push(key.keyframe);
    }
    groups
}

#[cfg(test)]
#[path = "motion_timeline_drag_tests.rs"]
mod tests;
