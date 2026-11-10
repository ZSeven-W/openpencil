//! Native arm of the motion timeline panel. Layout, paint and every
//! state change are shared (`op_editor_ui::widgets::motion_timeline`,
//! `op_editor_core::motion_timeline_*`); this only hands over the backend.

use super::WidgetHostNative;
use op_editor_ui::widgets::PaintCx;

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
}
