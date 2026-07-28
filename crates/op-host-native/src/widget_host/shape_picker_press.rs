//! Shape-picker dropdown press dispatch — a thin wrapper over the
//! host-shared `op_editor_ui::widgets::press_flow::press_shape_picker`
//! (the web host's sibling module wraps the same flow).

use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::press_flow::{self, ShapePickerPress};
use op_editor_ui::Point2D;

use super::WidgetHostNative;

impl WidgetHostNative {
    pub(in crate::widget_host) fn dispatch_shape_picker_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.shape_picker.open {
            return false;
        }
        self.refresh_layout_scene();
        let panel_rect = self.shape_picker_rect(viewport_width, viewport_height);
        match press_flow::press_shape_picker(&mut self.editor_state, panel_rect, Point2D::new(x, y))
        {
            ShapePickerPress::SetTool(tool) => self.apply_set_tool(tool),
            ShapePickerPress::Close => {
                if matches!(
                    self.editor_state.editor_ui.pending_file_action,
                    Some(op_editor_core::editor_ui_state::FileAction::ImportImageOrSvg)
                ) && !self.collab_allows_document_mutation_from(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::ExternalAssets,
                    ),
                    op_editor_core::CollabEditSource::Import,
                ) {
                    self.editor_state.editor_ui.pending_file_action = None;
                }
            }
            ShapePickerPress::Swallow => return true,
            ShapePickerPress::Outside => {
                // Miss — the dismissing click is a blank press.
                self.blur_text_inputs_on_blank_press();
            }
        }
        core_press::close_shape_picker(&mut self.editor_state.editor_ui);
        self.mark_dirty();
        true
    }
}
