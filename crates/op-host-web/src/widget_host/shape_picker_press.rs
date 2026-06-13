//! Shape-picker dropdown press dispatch — mirror of the native host's
//! `widget_host/shape_picker_press.rs`.

use op_editor_ui::widgets::{ShapeChoice, ShapePicker};
use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
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
        let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
        match picker.hit_popup(panel_rect, Point2D::new(x, y)) {
            op_editor_ui::widgets::shape_picker::SelectHit::Row(idx) => {
                match picker.choice_at(idx) {
                    Some(choice) => match choice {
                        ShapeChoice::Tool(tool) => {
                            // TS onToolChange DISCARDS an in-progress pen path
                            // on tool switch (`skia-pen-tool.ts:38-50`).
                            if !matches!(tool, op_editor_core::Tool::Pen) {
                                let _ = self.editor_state.cancel_pen_path();
                            }
                            self.editor_state.editor_ui.shape_tool = tool;
                            self.editor_state.tool = tool;
                        }
                        ShapeChoice::OpenIconPicker => {
                            self.editor_state.editor_ui.icon_picker_open = true;
                            self.editor_state.editor_ui.icon_picker_replace_selection = false;
                            self.editor_state.editor_ui.icon_picker_search.clear();
                        }
                        ShapeChoice::ImportImageOrSvg => {
                            // No file-picker service on web yet — raise the same
                            // pending flag the native host does; the consumer is
                            // host-level.
                            self.editor_state.editor_ui.pending_file_action =
                                Some(op_editor_core::editor_ui_state::FileAction::ImportImageOrSvg);
                        }
                    },
                    None => return true,
                }
            }
            op_editor_ui::widgets::shape_picker::SelectHit::Inside => return true,
            op_editor_ui::widgets::shape_picker::SelectHit::Outside => {
                // Miss — the dismissing click is a blank press.
                self.blur_text_inputs_on_blank_press();
            }
        }
        self.editor_state.editor_ui.shape_picker.open = false;
        self.editor_state.editor_ui.shape_picker.hover = None;
        self.editor_state.editor_ui.shape_picker.pressed = None;
        self.mark_dirty();
        true
    }
}
