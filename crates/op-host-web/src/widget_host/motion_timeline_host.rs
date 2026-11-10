//! Web arm of the motion timeline panel. Layout, paint, input and every
//! state change are shared (`op_editor_ui::widgets::motion_timeline*`,
//! `op_editor_core::motion_timeline_*`); paint, press and drag call them
//! straight from their ladders. Collaboration refuses edits on the daemon
//! side, like the web host's other document writes, so there is no local
//! gate here.

use super::WidgetHost;
use op_editor_ui::widgets::canvas_paint_scene;
use op_editor_ui::widgets::motion_timeline_flow as flow;

impl WidgetHost {
    pub(in crate::widget_host) fn release_motion_timeline(&mut self) -> bool {
        let consumed = flow::release(&mut self.editor_state);
        if consumed {
            self.mark_dirty();
        }
        consumed
    }

    /// Delete / Backspace on selected timeline keyframes.
    pub(in crate::widget_host) fn delete_motion_keyframes(&mut self) -> bool {
        let deleted = flow::delete_keyframes(&mut self.editor_state);
        if deleted {
            self.mark_dirty();
        }
        deleted
    }

    /// Whether the shell must keep requesting frames: the canvas plays the
    /// motion timeline.
    pub fn motion_timeline_playing(&self) -> bool {
        canvas_paint_scene::motion_timeline_playing(&self.editor_state)
    }
}
