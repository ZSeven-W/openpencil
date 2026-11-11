//! Timeline → Inspector. Pressing a lane selects its node, so the
//! Inspector shows what the timeline holds, and double-clicking a strip
//! opens that animation's row in the Motion section.

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
}

#[cfg(test)]
#[path = "motion_timeline_focus_tests.rs"]
mod tests;
