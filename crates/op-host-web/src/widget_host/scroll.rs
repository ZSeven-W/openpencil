//! Web wheel-scroll routing for the side panels — extracted from
//! `widget_host.rs` so the spine stays under the 800-line cap.
//! Mirrors the native host's `widget_host/scroll.rs`.

use op_editor_ui::util::scroll_by_max;
use op_editor_ui::widgets::scroll_flow;
use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

use super::WidgetHost;

impl WidgetHost {
    pub(in crate::widget_host) fn try_scroll_settings_font_picker(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.agent_settings_open {
            return false;
        }
        let panel = op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel::for_web_editor(
            &self.editor_state,
        );
        let panel_rect = panel.rect(viewport_width, viewport_height);
        let Some(layout) = panel.font_picker_layout(panel_rect) else {
            return false;
        };
        if !layout.popup.contains(Point2D::new(x, y)) {
            return false;
        }
        let ui = &mut self.editor_state.editor_ui;
        let next = (ui.font_picker.scroll.offset - delta_y).clamp(0.0, layout.max_scroll);
        if next != ui.font_picker.scroll.offset {
            ui.font_picker.scroll.offset = next;
            ui.font_picker.hover = None;
            ui.font_picker_import_hover = false;
            self.mark_dirty();
        }
        true
    }

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
        let panel_rect = self.variables_panel_rect(viewport_width, viewport_height);
        let Some(dirty) = scroll_flow::scroll_variables_panel(
            &mut self.editor_state,
            panel_rect,
            Point2D::new(x, y),
            delta_y,
        ) else {
            return false;
        };
        if dirty {
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
        let picker_rect = self.locale_picker_rect(viewport_width);
        let Some(dirty) = scroll_flow::scroll_locale_picker(
            &mut self.editor_state,
            picker_rect,
            Point2D::new(x, y),
            delta_y,
        ) else {
            return false;
        };
        if dirty {
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
        let panel_rect = self.design_md_panel_rect(viewport_width, viewport_height);
        let Some(dirty) = scroll_flow::scroll_design_md_panel(
            &mut self.editor_state,
            panel_rect,
            Point2D::new(x, y),
            delta_y,
        ) else {
            return false;
        };
        if dirty {
            self.mark_dirty();
        }
        true
    }

    /// Scroll the open icon picker's list when the pointer is over its panel.
    /// The picker loads up to 120 local + remote icons — far more than fit — so
    /// the list must scroll. Mirrors the native host.
    pub(in crate::widget_host) fn try_scroll_icon_picker(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let panel_rect = self.icon_picker_panel_rect(viewport_width, viewport_height);
        let Some(dirty) = scroll_flow::scroll_icon_picker(
            &mut self.editor_state,
            panel_rect,
            Point2D::new(x, y),
            delta_y,
        ) else {
            return false;
        };
        if dirty {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn try_scroll_scene_template_center(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let Some(panel_rect) = self.scene_template_panel_rect(viewport_width, viewport_height)
        else {
            return false;
        };
        let Some(dirty) = op_editor_ui::widgets::press_flow::scroll_scene_template_center(
            &mut self.editor_state,
            panel_rect,
            Point2D::new(x, y),
            delta_y,
        ) else {
            return false;
        };
        if dirty {
            self.mark_dirty();
        }
        true
    }

    pub(in crate::widget_host) fn try_scroll_prompt_center(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let panel_rect = self.prompt_center_panel_rect(viewport_width, viewport_height);
        let Some(dirty) = scroll_flow::scroll_prompt_center(
            &mut self.editor_state,
            panel_rect,
            Point2D::new(x, y),
            delta_y,
        ) else {
            return false;
        };
        if dirty {
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
        self.try_scroll_property_panel_2d(x, y, 0.0, delta_y, viewport_width, viewport_height)
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
        let Some(panel) = PropertyPanel::for_selection(&self.editor_state) else {
            return false;
        };
        let pw = self.editor_state.editor_ui.property_panel_width;
        let property_rect = Rect {
            origin: Point2D::new(viewport_width - pw, TOP_BAR_HEIGHT),
            size: Point2D::new(pw, (viewport_height - TOP_BAR_HEIGHT).max(0.0)),
        };
        if self.editor_state.editor_ui.compositing_picker.open
            && panel.compositing_picker_contains(property_rect, Point2D::new(x, y))
        {
            return true;
        }
        // A wheel over the open colour-variable popup scrolls ITS list,
        // not the inspector behind it (mirrors the native host).
        if panel.color_variable_picker_contains(property_rect, Point2D::new(x, y)) {
            let max = panel.color_variable_picker_max_scroll(property_rect);
            let scroll = &mut self
                .editor_state
                .editor_ui
                .property_color_variable_picker_scroll;
            if scroll_by_max(scroll, -delta_y, max) {
                self.mark_dirty();
            }
            return true;
        }
        // A wheel over the open font-family picker scrolls ITS list,
        // not the panel behind it (mirrors the native host).
        if self.editor_state.editor_ui.font_picker.open
            && panel.font_picker_contains(property_rect, Point2D::new(x, y))
        {
            let max = panel.font_picker_max_scroll(property_rect);
            let ui = &mut self.editor_state.editor_ui;
            // Positive delta shrinks the offset — same convention as
            // every other scroll surface (see the native host).
            let next = (ui.font_picker.scroll.offset - delta_y).clamp(0.0, max);
            if next != ui.font_picker.scroll.offset {
                ui.font_picker.scroll.offset = next;
                ui.font_picker.hover = None;
                self.mark_dirty();
            }
            return true;
        }
        let Some(dirty) = scroll_flow::scroll_property_panel_body_2d(
            &mut self.editor_state,
            &panel,
            property_rect,
            Point2D::new(x, y),
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
        // The slides tab owns the rail's wheel while it is on show; the
        // layer tree only sees the event when the tree is what the rail
        // is showing.
        if let Some(dirty) = self.slides_panel_scroll(
            Point2D::new(x, y),
            delta_y,
            self.last_viewport_w,
            viewport_height,
        ) {
            if dirty {
                self.mark_dirty();
            }
            return true;
        }
        let rect = self.layer_panel_rect(viewport_height);
        let panel = self.layer_panel();
        let Some(dirty) = scroll_flow::scroll_layer_panel(
            &mut self.editor_state,
            &panel,
            rect,
            Point2D::new(x, y),
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

    pub(in crate::widget_host) fn scroll_layer_panel_selection_into_view(
        &mut self,
        viewport_height: f32,
    ) -> bool {
        let rect = self.layer_panel_rect(viewport_height);
        let panel = self.layer_panel();
        if !scroll_flow::reveal_layer_panel_selection(&mut self.editor_state, &panel, rect) {
            return false;
        }
        self.mark_dirty();
        true
    }
}
