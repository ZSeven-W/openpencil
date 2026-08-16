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
        use op_editor_ui::widgets::PropertyPanel;
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.image_fill_popover_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = self.property_rect(viewport_width, viewport_height);
            let point = Point2D::new(x, y);
            if panel.image_fill_popover_contains(property_rect, point) {
                // Tile scale is a real text input inside the floating popup.
                // It must win before the popup's Pick/Mode actions and before
                // any inspector/body target covered underneath it.
                if let Some(focus) = panel.image_fill_popover_input_at(property_rect, point) {
                    self.commit_property_focus_if_any();
                    self.refresh_layout_scene();
                    let initial = PropertyPanel::for_selection_with_scene(
                        &self.editor_state,
                        &self.layout_scene,
                    )
                    .as_ref()
                    .map(|next| super::press_helpers::property_focus_initial(focus, next))
                    .unwrap_or_default();
                    let ui = &mut self.editor_state.ui;
                    ui.property_focus = Some(focus);
                    ui.property_input.set_text(initial.clone());
                    ui.property_input.touch(self.now_ms);
                    ui.property_input_draft = initial;
                    ui.property_caret_pos = ui.property_input.caret();
                    ui.property_caret_anchor_ms = self.now_ms;
                    ui.property_draft_select_all = false;
                    self.editor_state.chat.focused = false;
                    self.reveal_property_keyboard_owner();
                    self.mark_dirty();
                    return true;
                }
                if let Some(action) = panel.hit_test_action(property_rect, point) {
                    self.apply_property_action(action);
                }
                return true;
            }
        }
        self.commit_image_tile_scale_focus_if_any();
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
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A};
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.export_scale_picker_open
            && !self.editor_state.editor_ui.export_format_picker_open
        {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = self.property_rect(viewport_width, viewport_height);
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
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A};
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.font_weight_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = self.property_rect(viewport_width, viewport_height);
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
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A};
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.padding_mode_popover_open
            && !self.editor_state.editor_ui.stroke_mode_popover_open
        {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = self.property_rect(viewport_width, viewport_height);
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
        panel: Option<&op_editor_ui::widgets::PropertyPanel>,
    ) -> bool {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.padding_mode_popover_open
            && !self.editor_state.editor_ui.stroke_mode_popover_open
        {
            return false;
        }
        let new_hover = panel.and_then(|panel| {
            let property_rect = self.property_rect(self.last_viewport_w, self.last_viewport_h);
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
        panel: Option<&op_editor_ui::widgets::PropertyPanel>,
    ) -> bool {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        use op_editor_ui::Point2D;
        if !self.editor_state.editor_ui.font_weight_picker_open {
            return false;
        }
        let new_hover = panel.and_then(|panel| {
            let property_rect = self.property_rect(self.last_viewport_w, self.last_viewport_h);
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
