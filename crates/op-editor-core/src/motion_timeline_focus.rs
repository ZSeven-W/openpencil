//! Timeline → Inspector. Pressing a lane selects its node, so the
//! Inspector shows what the timeline holds; double-clicking a strip opens
//! that animation's row in the Motion section; and while a keyframe of the
//! selected node is selected on the timeline, the Motion section edits
//! that keyframe's values.

use crate::editor_ui_state::TimelineKeyframeRef;
use crate::motion_keyframe_edit::addable_properties;
use crate::motion_summary::{KeyframeSummary, MotionSummary};
use crate::motion_timeline_projection::{keyframe_ms, manual_ids};
use crate::{EditorState, NodeId, PropertyTab};

/// Two presses on one strip this close together are a double-click.
pub const STRIP_DOUBLE_CLICK_MS: u64 = 400;

impl EditorState {
    /// Select the node a timeline lane belongs to; the Inspector follows.
    pub fn select_motion_lane(&mut self, node_id: &str) {
        if self.selection.anchor.as_str() != node_id {
            self.set_single_selection(NodeId::new(node_id));
        }
    }

    /// Record a press on a strip. Returns whether it completes a
    /// double-click on the same strip.
    pub fn press_motion_strip(&mut self, node_id: &str, animation: usize, now_ms: u64) -> bool {
        let timeline = &mut self.editor_ui.motion_timeline;
        let double = matches!(
            &timeline.last_strip_press,
            Some((id, index, at))
                if id == node_id
                    && *index == animation
                    && now_ms.saturating_sub(*at) < STRIP_DOUBLE_CLICK_MS
        );
        timeline.last_strip_press = (!double).then(|| (node_id.to_owned(), animation, now_ms));
        double
    }

    /// Show animation `index` of `node_id` in the Inspector: select the
    /// node, switch to the Design tab and open its Motion row. Keyframe
    /// selection goes, so the section lists the animation's fields rather
    /// than one keyframe's values.
    pub fn focus_motion_animation(&mut self, node_id: &str, index: usize) {
        self.select_motion_lane(node_id);
        self.editor_ui.property_tab = PropertyTab::Design;
        self.editor_ui.motion_expanded = Some((node_id.to_owned(), index));
        self.editor_ui.motion_timeline.selection.clear();
    }

    /// The keyframe the Motion section edits: the last one selected on an
    /// open timeline, when it belongs to the selected node.
    pub fn motion_keyframe_focus(&self) -> Option<&TimelineKeyframeRef> {
        let timeline = &self.editor_ui.motion_timeline;
        let key = timeline.selection.last().filter(|_| timeline.open)?;
        (key.node_id == self.selection.anchor.as_str()).then_some(key)
    }

    /// Fill in what the Motion section needs beyond the node itself: the
    /// open row, the manual ids used on the node's screen, and the
    /// keyframe being edited.
    pub fn fill_motion_summary(&self, summary: &mut MotionSummary) {
        let anchor = &self.selection.anchor;
        summary.expanded = self.editor_ui.motion_row_expanded(anchor.as_str());
        summary.manual_ids = self
            .motion_screen_of(anchor)
            .map(manual_ids)
            .unwrap_or_default();
        summary.keyframe = self.motion_keyframe_focus().and_then(|key| {
            let node = self.selected_node()?;
            let animation = node.motion_declarations().1?.get(key.animation)?;
            let frame = animation.keyframes.get(key.keyframe)?;
            Some(KeyframeSummary {
                animation: key.animation,
                keyframe: key.keyframe,
                count: animation.keyframes.len(),
                time_ms: keyframe_ms(animation, frame.offset),
                values: frame
                    .values
                    .iter()
                    .map(|(property, value)| (property.clone(), value.clone()))
                    .collect(),
                addable: addable_properties(animation, key.keyframe),
            })
        });
    }
}

#[cfg(test)]
#[path = "motion_timeline_focus_tests.rs"]
mod tests;
