//! Native arm of the motion timeline panel. Layout, paint, input and every
//! state change are shared (`op_editor_ui::widgets::motion_timeline*`,
//! `op_editor_core::motion_timeline_*`); paint, press and drag call them
//! straight from their ladders. What stays here is native-only: writes are
//! gated on the collaboration policy, and the desktop key router reaches
//! the M shortcut through a host method.

use super::WidgetHostNative;
use op_editor_core::{CollabDocumentMutation, CollabUnsupportedFeature};
use op_editor_ui::widgets::motion_timeline_flow as flow;

/// A collaboration session refuses motion edits, like the Motion section.
const MOTION_EDIT: CollabDocumentMutation =
    CollabDocumentMutation::Unsupported(CollabUnsupportedFeature::UnsupportedNodeProperty);

impl WidgetHostNative {
    /// Release a timeline drag; a refused write drops the draft.
    pub(in crate::widget_host) fn release_motion_timeline(&mut self) -> bool {
        if flow::drag_writes(&self.editor_state)
            && !self.collab_allows_document_mutation(MOTION_EDIT)
        {
            self.editor_state.cancel_motion_timeline_drag();
            self.mark_dirty();
            return true;
        }
        let consumed = flow::release(&mut self.editor_state);
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// The timeline shortcut (M), where the panel can show.
    pub fn apply_toggle_motion_timeline(&mut self) -> bool {
        let toggled = flow::toggle(&mut self.editor_state);
        if toggled {
            self.mark_dirty();
        }
        toggled
    }

    /// Delete / Backspace on selected timeline keyframes.
    pub(in crate::widget_host) fn delete_motion_keyframes(&mut self) -> bool {
        if !flow::has_keyframe_selection(&self.editor_state) {
            return false;
        }
        if self.collab_allows_document_mutation(MOTION_EDIT) {
            flow::delete_keyframes(&mut self.editor_state);
            self.mark_dirty();
        }
        true
    }
}

#[cfg(test)]
#[path = "motion_timeline_host_tests.rs"]
mod tests;
