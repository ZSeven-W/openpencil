//! Shared visibility and surface geometry for touch editor chrome.

use super::WidgetHostNative;
use op_editor_core::size_class::MobileSheetKind;
use op_editor_ui::Rect;

impl WidgetHostNative {
    pub(in crate::widget_host) fn selection_actions_visible(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        ui.touch_chrome()
            && !self.preview_active()
            && ui.mobile_sheet.is_none()
            && !ui.variables_panel_open
            && !self.editor_state.selection.is_empty()
    }

    pub(in crate::widget_host) fn layers_panel_visible(&self) -> bool {
        let ui = &self.editor_state.editor_ui;
        if !ui.touch_chrome() {
            return ui.sidebar_open;
        }
        (ui.expanded_touch_layout() && ui.sidebar_open)
            || ui.mobile_sheet == Some(MobileSheetKind::Layers)
    }

    pub(in crate::widget_host) fn mobile_sheet_rect(
        &self,
        viewport_w: f32,
        viewport_h: f32,
        kind: MobileSheetKind,
    ) -> Rect {
        match kind {
            MobileSheetKind::Layers if self.editor_state.editor_ui.compact_layout() => {
                op_editor_ui::widgets::mobile_chrome::sheet_rect(viewport_w, viewport_h, 0.68)
            }
            MobileSheetKind::Layers => {
                op_editor_ui::widgets::host_canvas_geometry::layer_panel_rect(
                    &self.editor_state,
                    viewport_h,
                )
            }
            MobileSheetKind::Properties => {
                op_editor_ui::widgets::host_canvas_geometry::property_panel_rect(
                    &self.editor_state,
                    viewport_w,
                    viewport_h,
                )
            }
            MobileSheetKind::Ai => self
                .ai_chat_rect(viewport_w, viewport_h)
                .unwrap_or_else(|| {
                    op_editor_ui::widgets::mobile_chrome::sheet_rect(viewport_w, viewport_h, 0.58)
                }),
            MobileSheetKind::More => op_editor_ui::widgets::mobile_chrome::more_panel_rect(
                &self.editor_state,
                viewport_w,
                viewport_h,
            ),
        }
    }
}
