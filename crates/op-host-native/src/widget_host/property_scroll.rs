//! Property-panel wheel and pan routing, including the Code framework strip.

use super::WidgetHostNative;
use op_editor_ui::widgets::{host_canvas_geometry, scroll_flow, PropertyPanel};
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Vertical-wheel compatibility entry point. Two-axis pan paths call the
    /// sibling directly so the Code framework strip can consume `delta_x`.
    pub(in crate::widget_host) fn try_scroll_property_panel(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.try_scroll_property_panel_2d(x, y, 0.0, delta, viewport_width, viewport_height)
    }

    pub(in crate::widget_host) fn try_scroll_property_panel_2d(
        &mut self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // A wheel over the open font-family picker scrolls its list, not the
        // inspector behind it.
        if self.try_scroll_font_picker(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        let Some(panel) = PropertyPanel::for_selection_at(&self.editor_state, self.now_ms) else {
            return false;
        };
        if self.editor_state.editor_ui.touch_chrome()
            && !self.editor_state.editor_ui.expanded_touch_layout()
            && self.editor_state.editor_ui.mobile_sheet
                != Some(op_editor_core::size_class::MobileSheetKind::Properties)
        {
            return false;
        }
        let property_rect = host_canvas_geometry::property_panel_rect(
            &self.editor_state,
            viewport_width,
            viewport_height,
        );
        let point = Point2D::new(x, y);
        // This compact popup has no internal scroll but still owns events over
        // its chrome so the inspector cannot move underneath it.
        if self.editor_state.editor_ui.compositing_picker.open
            && panel.compositing_picker_contains(property_rect, point)
        {
            return true;
        }
        let Some(dirty) = scroll_flow::scroll_property_panel_body_2d(
            &mut self.editor_state,
            &panel,
            property_rect,
            point,
            delta_x,
            delta_y,
        ) else {
            return false;
        };
        if dirty {
            self.mark_dirty();
        }
        true
    }
}
