//! Web wheel-scroll routing for the side panels — extracted from
//! `widget_host.rs` so the spine stays under the 800-line cap.
//! Mirrors the native host's `widget_host/scroll.rs`.

use op_editor_ui::util::scroll_by_max;
use op_editor_ui::widgets::{LayerPanel, PropertyPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

use super::WidgetHost;

impl WidgetHost {
    /// Scroll the floating VariablesPanel row list when the wheel
    /// fires over the open panel (TS `overflow-y-auto` rows region).
    /// The whole panel rect swallows the event so a wheel over its
    /// header can't zoom the canvas beneath. Mirrors the native host.
    pub(in crate::widget_host) fn try_scroll_variables_panel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.variables_panel_open {
            return false;
        }
        let Some(panel_rect) = self.variables_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        if !(panel_rect).contains(Point2D::new(x, y)) {
            return false;
        }
        use op_editor_ui::widgets::variables_panel::VariablesPanel;
        let panel = VariablesPanel::for_editor(&self.editor_state);
        let max = panel.max_scroll(panel_rect);
        if scroll_by_max(
            &mut self.editor_state.editor_ui.variables_scroll,
            -delta_y,
            max,
        ) {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn try_scroll_locale_picker(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.locale_picker.open {
            return false;
        }
        if !(self.locale_picker_rect(viewport_width)).contains(Point2D::new(x, y)) {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui.locale_picker;
        let next = (ui.scroll.offset - delta_y)
            .clamp(0.0, op_editor_ui::widgets::LocalePicker::max_scroll());
        let changed = next != ui.scroll.offset || ui.hover.is_some();
        ui.scroll.offset = next;
        ui.hover = None;
        if changed {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn try_scroll_design_md_panel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.design_md_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        if !(panel_rect).contains(Point2D::new(x, y)) {
            return false;
        }
        let Some(panel) = op_editor_ui::widgets::DesignMdPanel::for_editor(&self.editor_state)
        else {
            return false;
        };
        let max = panel.max_scroll(panel_rect);
        if scroll_by_max(
            &mut self.editor_state.editor_ui.design_md_scroll,
            -delta_y,
            max,
        ) {
            self.mark_dirty();
        }
        true
    }

    /// Scroll the right-rail PropertyPanel when a wheel lands over
    /// it. Returns `true` when the cursor was over the inspector.
    pub(in crate::widget_host) fn try_scroll_property_panel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel) = PropertyPanel::for_selection(&self.editor_state) else {
            return false;
        };
        let pw = self.editor_state.editor_ui.property_panel_width;
        let property_rect = Rect {
            origin: Point2D::new(viewport_width - pw, TOP_BAR_HEIGHT),
            size: Point2D::new(pw, (viewport_height - TOP_BAR_HEIGHT).max(0.0)),
        };
        // A wheel over the open font-family picker scrolls ITS list,
        // not the panel behind it (mirrors the native host).
        if self.editor_state.editor_ui.font_picker.open
            && panel.font_picker_contains(property_rect, Point2D::new(x, y))
        {
            let max = panel.font_picker_max_scroll(property_rect);
            let ui = &mut self.editor_state.editor_ui;
            let next = (ui.font_picker.scroll.offset + delta_y).clamp(0.0, max);
            if next != ui.font_picker.scroll.offset {
                ui.font_picker.scroll.offset = next;
                ui.font_picker.hover = None;
                self.mark_dirty();
            }
            return true;
        }
        if !(property_rect).contains(Point2D::new(x, y)) {
            return false;
        }
        if matches!(
            self.editor_state.editor_ui.property_tab,
            op_editor_core::PropertyTab::Code
        ) {
            let point = Point2D::new(x, y);
            let (band_top, band_bottom) =
                op_editor_ui::widgets::property_panel_code::framework_row_band(
                    property_rect.origin.y,
                );
            if y >= band_top && y <= band_bottom {
                let max = op_editor_ui::widgets::property_panel_code::framework_row_overflow(pw);
                let cg = &mut self.editor_state.codegen;
                if scroll_by_max(&mut cg.framework_scroll, -delta_y, max) {
                    self.mark_dirty();
                }
                return true;
            }
            if op_editor_ui::widgets::property_panel_code::code_preview_rect(
                property_rect,
                &self.editor_state.codegen,
            )
            .is_some_and(|rect| (rect).contains(point))
            {
                let max = op_editor_ui::widgets::property_panel_code::code_preview_max_scroll(
                    property_rect,
                    &self.editor_state.codegen,
                )
                .unwrap_or(0.0);
                let cg = &mut self.editor_state.codegen;
                if scroll_by_max(&mut cg.code_scroll, -delta_y, max) {
                    self.mark_dirty();
                }
                return true;
            }
        }
        let max = (panel.content_height(property_rect) - property_rect.size.y).max(0.0);
        if scroll_by_max(
            &mut self.editor_state.editor_ui.property_panel_scroll,
            -delta_y,
            max,
        ) {
            self.mark_dirty();
        }
        true
    }

    /// Scroll the left-rail LayerPanel when a wheel lands over it —
    /// the Pages section above the Layers row viewport, otherwise
    /// the Layers section. Returns `true` when over the panel.
    pub(in crate::widget_host) fn try_scroll_layer_panel(
        &mut self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.sidebar_open {
            return false;
        }
        let pw = self.editor_state.editor_ui.layer_panel_width;
        let rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(pw, (viewport_height - TOP_BAR_HEIGHT).max(0.0)),
        };
        if !(rect).contains(Point2D::new(x, y)) {
            return false;
        }
        let r = LayerPanel::from_editor(&self.editor_state).regions(rect);
        let mut changed = false;
        if y >= r.layers_rows_top {
            if delta_y != 0.0
                && scroll_by_max(
                    &mut self.editor_state.editor_ui.layer_layers_scroll,
                    -delta_y,
                    r.layers.max_offset,
                )
            {
                changed = true;
            }
            if delta_x != 0.0
                && scroll_by_max(
                    &mut self.editor_state.editor_ui.layer_layers_h_scroll,
                    -delta_x,
                    r.layers.max_horizontal_offset,
                )
            {
                changed = true;
            }
        } else {
            if delta_y != 0.0
                && scroll_by_max(
                    &mut self.editor_state.editor_ui.layer_pages_scroll,
                    -delta_y,
                    r.pages.max_offset,
                )
            {
                changed = true;
            }
            if delta_x != 0.0
                && scroll_by_max(
                    &mut self.editor_state.editor_ui.layer_pages_h_scroll,
                    -delta_x,
                    r.pages.max_horizontal_offset,
                )
            {
                changed = true;
            }
        }
        if changed {
            self.mark_dirty();
        }
        true
    }
}
