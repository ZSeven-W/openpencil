//! Native arm of the motion timeline panel. Layout, paint, input and every
//! state change are shared (`op_editor_ui::widgets::motion_timeline*`,
//! `op_editor_core::motion_timeline_*`); this hands over the backend and
//! the pointer, and gates writes on the collaboration policy.

use super::WidgetHostNative;
use op_editor_core::{CollabDocumentMutation, CollabUnsupportedFeature};
use op_editor_ui::widgets::motion_timeline_flow as flow;
use op_editor_ui::widgets::PaintCx;
use op_editor_ui::Point2D;

/// A collaboration session refuses motion edits, like the Motion section.
const MOTION_EDIT: CollabDocumentMutation =
    CollabDocumentMutation::Unsupported(CollabUnsupportedFeature::UnsupportedNodeProperty);

impl WidgetHostNative {
    pub(in crate::widget_host) fn paint_motion_timeline(
        &self,
        backend: &mut dyn op_editor_ui::RenderBackend,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        op_editor_ui::widgets::motion_timeline::paint_for_editor(
            &mut PaintCx { backend },
            &self.theme,
            &self.editor_state,
            viewport_width,
            viewport_height,
        );
    }

    /// Press tier: whether the press landed on the panel.
    pub(in crate::widget_host) fn press_motion_timeline(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let consumed = flow::press(
            &mut self.editor_state,
            Point2D::new(x, y),
            viewport_width,
            viewport_height,
            self.now_ms,
            self.shift_held,
        );
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// Pointer capture while a timeline drag is in flight.
    pub(in crate::widget_host) fn drag_motion_timeline(&mut self, x: f32, y: f32) -> bool {
        let consumed = flow::drag(
            &mut self.editor_state,
            Point2D::new(x, y),
            self.last_viewport_w,
            self.last_viewport_h,
            self.now_ms,
        );
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

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
