//! Property-panel popover dismissal + hover tracking, split out of
//! `property_dispatch.rs` to keep both files under the 800-line cap.

use super::WidgetHostNative;

impl WidgetHostNative {
    /// Image-fill popover outside-click dismiss. Returns `true`
    /// when the popover was open and the press was consumed.
    pub(in crate::widget_host) fn dismiss_image_fill_popover_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.image_fill_popover_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                self.apply_property_action(action);
                return true;
            }
            if panel.image_fill_popover_contains(property_rect, Point2D::new(x, y)) {
                return true;
            }
        }
        self.editor_state.editor_ui.image_fill_popover_open = false;
        self.mark_dirty();
        true
    }

    /// Outside-click dismiss for the Export section's inline scale /
    /// format select popups. Returns `true` when a picker was open
    /// and the press was consumed — an option / toggle was applied,
    /// or the press fell outside and dismissed the popup. The caller
    /// must stop dispatching the press in that case. `false` when no
    /// picker was open (press dispatch continues normally).
    pub(in crate::widget_host) fn dismiss_export_picker_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.export_scale_picker_open
            && !self.editor_state.editor_ui.export_format_picker_open
        {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                if matches!(
                    action,
                    A::SetExportScale(_)
                        | A::SetExportFormat(_)
                        | A::ToggleExportScalePicker
                        | A::ToggleExportFormatPicker
                ) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        let ui = &mut self.editor_state.editor_ui;
        ui.export_scale_picker_open = false;
        ui.export_format_picker_open = false;
        ui.export_picker_hover = None;
        self.mark_dirty();
        true
    }

    // The font-family picker's outside-click dismiss lives in
    // `font_picker_dispatch.rs` (`dismiss_font_picker_on_press`) —
    // the searchable overlay needs the contains-swallow the simple
    // pickers here don't.

    pub(in crate::widget_host) fn dismiss_font_weight_picker_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.font_weight_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                if matches!(action, A::SetFontWeight(_) | A::ToggleFontWeightPicker) {
                    if let A::SetFontWeight(choice) = action {
                        self.editor_state.editor_ui.pressed_button =
                            op_editor_ui::widgets::FontWeightChoice::ALL
                                .iter()
                                .position(|c| *c == choice)
                                .map(op_editor_core::ButtonPressTarget::FontWeightPicker);
                        self.mark_dirty();
                        return true;
                    }
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        self.editor_state.editor_ui.font_weight_picker_open = false;
        self.editor_state.editor_ui.font_weight_picker_hover = None;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn dismiss_padding_mode_popover_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.padding_mode_popover_open
            && !self.editor_state.editor_ui.stroke_mode_popover_open
        {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                if matches!(
                    action,
                    A::SetPaddingMode(_)
                        | A::TogglePaddingModePopover
                        | A::SetStrokeMode(_)
                        | A::ToggleStrokeModePopover
                ) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        self.editor_state.editor_ui.padding_mode_popover_open = false;
        self.editor_state.editor_ui.padding_mode_popover_hover = None;
        self.editor_state.editor_ui.stroke_mode_popover_open = false;
        self.editor_state.editor_ui.stroke_mode_popover_hover = None;
        self.mark_dirty();
        true
    }

    /// Track the padding-mode popover row under the cursor so the open
    /// popover paints a hover wash. Returns true when the hovered row
    /// changed (a repaint is due). No-op when the popover is closed.
    pub(in crate::widget_host) fn update_padding_mode_popover_hover(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.padding_mode_popover_open
            && !self.editor_state.editor_ui.stroke_mode_popover_open
        {
            return false;
        }
        self.refresh_layout_scene();
        let new_hover = PropertyPanel::for_selection(&self.editor_state).and_then(|panel| {
            let property_rect = Rect {
                origin: Point2D::new(
                    self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            match panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                Some(A::SetPaddingMode(mode)) => op_editor_core::PaddingEditMode::ALL
                    .iter()
                    .position(|m| *m == mode),
                Some(A::SetStrokeMode(mode)) => op_editor_core::PaddingEditMode::ALL
                    .iter()
                    .position(|m| *m == mode),
                _ => None,
            }
        });
        let padding_changed = if self.editor_state.editor_ui.padding_mode_popover_open
            && new_hover != self.editor_state.editor_ui.padding_mode_popover_hover
        {
            self.editor_state.editor_ui.padding_mode_popover_hover = new_hover;
            true
        } else {
            false
        };
        let stroke_changed = if self.editor_state.editor_ui.stroke_mode_popover_open
            && new_hover != self.editor_state.editor_ui.stroke_mode_popover_hover
        {
            self.editor_state.editor_ui.stroke_mode_popover_hover = new_hover;
            true
        } else {
            false
        };
        if padding_changed || stroke_changed {
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Track the font-weight dropdown row under the cursor so the open
    /// dropdown paints a hover wash. Returns true when the hovered row
    /// changed (a repaint is due). No-op when the dropdown is closed.
    pub(in crate::widget_host) fn update_font_weight_picker_hover(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.font_weight_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        let new_hover = PropertyPanel::for_selection(&self.editor_state).and_then(|panel| {
            let property_rect = Rect {
                origin: Point2D::new(
                    self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            match panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                Some(A::SetFontWeight(choice)) => op_editor_ui::widgets::FontWeightChoice::ALL
                    .iter()
                    .position(|c| *c == choice),
                _ => None,
            }
        });
        if new_hover != self.editor_state.editor_ui.font_weight_picker_hover {
            self.editor_state.editor_ui.font_weight_picker_hover = new_hover;
            self.mark_dirty();
            return true;
        }
        false
    }
}
