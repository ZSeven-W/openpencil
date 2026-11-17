//! Shape-picker dropdown press dispatch — a thin wrapper over the
//! host-shared `op_editor_ui::widgets::press_flow::press_shape_picker`
//! (the web host's sibling module wraps the same flow).

use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::press_flow::{self, ShapePickerPress};
use op_editor_ui::Point2D;

use super::WidgetHostNative;

impl WidgetHostNative {
    pub fn native_touch_dock_visible(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        ui.native_touch_dock
            && !self.home_visible()
            && !self.works_reader_visible()
            && !self.preview_slideshow_active()
            && !self.mobile_sheet_is_modal()
            && !ui.variables_panel_open
            && !ui.agent_settings_open
            && !ui.login_modal_open
    }

    pub fn press_native_touch_dock(&mut self, slot: usize) -> bool {
        if !self.native_touch_dock_visible() {
            return false;
        }
        let dock = op_editor_ui::widgets::MobileDock::for_editor(&self.editor_state);
        let Some(hit) = dock.hit_for_slot(slot) else {
            return false;
        };
        self.dispatch_mobile_dock_hit(hit);
        true
    }

    pub fn press_native_shape_menu(&mut self, row: usize) -> bool {
        if !self.native_touch_dock_visible() {
            return false;
        }
        let choice =
            op_editor_ui::widgets::ShapePicker::for_editor_ui(&self.editor_state.editor_ui)
                .choice_at(row);
        let action = press_flow::apply_shape_picker_choice(&mut self.editor_state, choice);
        self.finish_shape_picker_press(action)
    }

    pub(in crate::widget_host) fn dispatch_mobile_dock_hit(
        &mut self,
        hit: op_editor_ui::widgets::MobileDockHit,
    ) {
        match hit {
            op_editor_ui::widgets::MobileDockHit::Tool(tool) => {
                self.editor_state.tool = tool;
            }
            op_editor_ui::widgets::MobileDockHit::ShapeSlot => {
                op_editor_core::host_press_transitions::toggle_shape_picker(
                    &mut self.editor_state.editor_ui,
                );
            }
            op_editor_ui::widgets::MobileDockHit::More => {
                self.toggle_mobile_sheet(op_editor_core::size_class::MobileSheetKind::More);
            }
        }
        self.mark_dirty();
    }

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
        let action =
            press_flow::press_shape_picker(&mut self.editor_state, panel_rect, Point2D::new(x, y));
        self.finish_shape_picker_press(action)
    }

    fn finish_shape_picker_press(&mut self, action: ShapePickerPress) -> bool {
        match action {
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
