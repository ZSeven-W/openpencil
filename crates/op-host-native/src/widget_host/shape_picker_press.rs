//! Shape-picker dropdown press dispatch.

use op_editor_ui::widgets::{ShapeChoice, ShapePicker};
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
        if !self.editor_state.editor_ui.shape_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        let panel_rect = self.shape_picker_rect(viewport_width, viewport_height);
        let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
        if let Some(choice) = picker.hit_test(panel_rect, Point2D::new(x, y)) {
            match choice {
                ShapeChoice::Tool(tool) => {
                    let _ = self.editor_state.finish_pen_path();
                    self.editor_state.editor_ui.shape_tool = tool;
                    self.editor_state.tool = tool;
                }
                ShapeChoice::OpenIconPicker => {
                    self.editor_state.editor_ui.icon_picker_open = true;
                    self.editor_state.editor_ui.icon_picker_replace_selection = false;
                    self.editor_state.editor_ui.icon_picker_search.clear();
                    self.editor_state.editor_ui.icon_picker_caret_anchor_ms = self.now_ms;
                    self.editor_state.editor_ui.icon_picker_scroll = 0.0;
                }
                ShapeChoice::ImportImageOrSvg => {
                    self.editor_state.editor_ui.pending_file_action =
                        Some(op_editor_core::editor_ui_state::FileAction::ImportImageOrSvg);
                }
            }
        }
        self.editor_state.editor_ui.shape_picker_open = false;
        self.editor_state.editor_ui.shape_picker_hover = None;
        self.mark_dirty();
        true
    }
}
