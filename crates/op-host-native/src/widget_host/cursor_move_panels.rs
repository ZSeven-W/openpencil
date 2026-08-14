//! `apply_cursor_move` tier 2 — floating-panel drags + panel hover, and
//! tier 3 — the in-flight property / text-selection / node drags.
//!
//! Tier 2 keeps the paint Z-order of the floating panels (design-md,
//! component browser, icon picker, dropdown overlays, variables preset
//! menu, variables panel). A panel whose hover changed but which does NOT
//! own the point folds its repaint into `higher_overlay_hover_changed` and
//! falls through, so a lower surface can still claim the move.
//!
//! Tier 3 is pointer capture: once one of these drags is live it owns the
//! cursor until release.

use super::cursor_move_ctx::CursorMoveCtx;
use super::WidgetHostNative;
use op_editor_ui::widgets::{CollabPanel, PropertyPanel};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// `None` — no floating panel consumed the move.
    pub(in crate::widget_host) fn cursor_move_floating_panel_tiers(
        &mut self,
        x: f32,
        y: f32,
        chat_or_picker_owns_point: bool,
        higher_overlay_hover_changed: &mut bool,
    ) -> Option<bool> {
        if let Some(d) = self.design_md_drag {
            self.editor_state.editor_ui.design_md_panel.pos = Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return Some(true);
        }
        // Design-MD panel hover (close / import / export / remove /
        // section headers).
        if self.editor_state.editor_ui.design_md_panel.open {
            use op_editor_ui::widgets::design_md_panel::DesignMdPanel;
            if let Some(panel_rect) =
                self.design_md_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let point = Point2D::new(x, y);
                let new_hover = DesignMdPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.design_md_panel.hover;
                if changed {
                    self.editor_state.editor_ui.design_md_panel.hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    if self
                        .editor_state
                        .editor_ui
                        .prompt_center
                        .hover
                        .take()
                        .is_some()
                    {
                        self.mark_dirty();
                    }
                    self.clear_lower_overlay_hover();
                    return Some(true);
                }
                if changed && !chat_or_picker_owns_point {
                    return Some(true);
                }
                *higher_overlay_hover_changed |= changed;
            }
        }
        if let Some(panel_rect) =
            self.scene_template_panel_rect(self.last_viewport_w, self.last_viewport_h)
        {
            let (owns_point, changed) =
                op_editor_ui::widgets::press_flow::hover_scene_template_center(
                    &mut self.editor_state,
                    panel_rect,
                    Point2D::new(x, y),
                );
            if changed {
                self.mark_dirty();
            }
            if owns_point {
                self.clear_lower_overlay_hover();
                return Some(true);
            }
        }
        if let Some(panel_rect) =
            self.prompt_center_panel_rect(self.last_viewport_w, self.last_viewport_h)
        {
            let (owns_point, changed) =
                op_editor_ui::widgets::cursor_hover_flow::prompt_center_hover(
                    &mut self.editor_state,
                    panel_rect,
                    Point2D::new(x, y),
                );
            if changed {
                self.mark_dirty();
            }
            if owns_point {
                if self
                    .editor_state
                    .editor_ui
                    .component_browser_hover
                    .take()
                    .is_some()
                {
                    self.mark_dirty();
                }
                self.clear_lower_overlay_hover();
                return Some(true);
            }
            if changed && !chat_or_picker_owns_point {
                return Some(true);
            }
            *higher_overlay_hover_changed |= changed;
        }
        if let Some(d) = self.component_browser_drag {
            self.editor_state.editor_ui.component_browser_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return Some(true);
        }
        // Component-browser panel hover (close / category pills / cards).
        if self.editor_state.editor_ui.component_browser_open {
            use op_editor_ui::widgets::component_browser_panel::ComponentBrowserPanel;
            if let Some(panel_rect) =
                self.component_browser_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let point = Point2D::new(x, y);
                let new_hover = ComponentBrowserPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.component_browser_hover;
                if changed {
                    self.editor_state.editor_ui.component_browser_hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return Some(true);
                }
                if changed && !chat_or_picker_owns_point {
                    return Some(true);
                }
                *higher_overlay_hover_changed |= changed;
            }
        }
        if let Some(d) = self.icon_picker_drag {
            self.editor_state.editor_ui.icon_picker_panel_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return Some(true);
        }
        // Icon-picker panel hover (close / icon rows / load-more).
        if self.editor_state.editor_ui.icon_picker.open {
            use op_editor_ui::widgets::icon_picker_panel::IconPickerPanel;
            if let Some(panel_rect) =
                self.icon_picker_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                let point = Point2D::new(x, y);
                let new_hover = IconPickerPanel::for_editor(&self.editor_state)
                    .and_then(|p| p.hover_at(panel_rect, point));
                let changed = new_hover != self.editor_state.editor_ui.icon_picker.hover;
                if changed {
                    self.editor_state.editor_ui.icon_picker.hover = new_hover;
                    self.mark_dirty();
                }
                if panel_rect.contains(point) {
                    self.clear_lower_overlay_hover();
                    return Some(true);
                }
                if changed && !chat_or_picker_owns_point {
                    return Some(true);
                }
                *higher_overlay_hover_changed |= changed;
            }
        }
        // Collaboration is painted above the remaining dropdown/variables
        // surfaces. Defer its point to the context-menu tier so the still
        // higher path/layer menus can win first.
        if self
            .collab_panel_probe_at(Point2D::new(x, y))
            .is_some_and(|(rect, _)| rect.contains(Point2D::new(x, y)))
        {
            return None;
        }
        if self.over_dropdown_overlay(x, y, self.last_viewport_w, self.last_viewport_h) {
            self.update_dropdown_hover(x, y, false);
            self.clear_chat_and_lower_hover();
            return Some(true);
        }
        // Preset dropdown is a top-most overlay over the variables panel — track
        // its per-row hover and swallow moves over it first.
        if !chat_or_picker_owns_point
            && self.editor_state.editor_ui.variables_preset_menu_open
            && self.update_variables_preset_menu_hover(
                x,
                self.last_viewport_w,
                self.last_viewport_h,
                y,
            )
        {
            self.clear_lower_overlay_hover();
            return Some(true);
        }
        if !chat_or_picker_owns_point && self.editor_state.editor_ui.variables_panel_open {
            let point = Point2D::new(x, y);
            if let Some(panel_rect) =
                self.variables_panel_rect(self.last_viewport_w, self.last_viewport_h)
            {
                if (panel_rect).contains(point) {
                    use op_editor_ui::widgets::variables_panel::VariablesPanel;
                    let new_hover = VariablesPanel::for_editor_at(&self.editor_state, self.now_ms)
                        .hover_at(panel_rect, point);
                    let changed = new_hover != self.editor_state.editor_ui.variables_panel_hover;
                    if changed {
                        self.editor_state.editor_ui.variables_panel_hover = new_hover;
                        self.mark_dirty();
                    }
                    self.clear_lower_overlay_hover();
                    return Some(true);
                }
            }
            if self
                .editor_state
                .editor_ui
                .variables_panel_hover
                .take()
                .is_some()
            {
                self.mark_dirty();
                return Some(true);
            }
        }
        None
    }

    /// Collaboration is above Git/dropdown/chrome/property overlays but below
    /// path/layer menus and the draggable panels. The caller invokes this
    /// after those higher tiers have had first refusal.
    pub(in crate::widget_host) fn cursor_move_collab_panel(
        &mut self,
        ctx: &mut CursorMoveCtx,
    ) -> Option<bool> {
        if !self.editor_state.editor_ui.collab.panel.open {
            return None;
        }
        let point = ctx.point;
        let probe = self.collab_panel_probe_at(point);
        let (panel_rect, new_hover) = probe?;
        let changed = new_hover != self.editor_state.editor_ui.collab.panel.hover;
        if changed {
            self.editor_state.editor_ui.collab.panel.hover = new_hover;
            self.mark_dirty();
        }
        if panel_rect.contains(point) {
            self.clear_hover_below_collab_panel();
            return Some(true);
        }
        if changed {
            if ctx.chat_or_picker_owns_point {
                ctx.upper_hover_changed = true;
            } else {
                return Some(true);
            }
        }
        None
    }

    fn collab_panel_probe_at(
        &self,
        point: Point2D,
    ) -> Option<(Rect, Option<op_editor_core::CollabPanelHover>)> {
        let ui = &self.editor_state.editor_ui;
        CollabPanel::for_editor_ui(ui).map(|panel| {
            let panel_rect =
                op_editor_ui::widgets::touch_overlay_geometry::collaboration_panel_rect(
                    &self.editor_state,
                    &panel,
                    self.last_viewport_w,
                    self.last_viewport_h,
                );
            (panel_rect, panel.hover_at(panel_rect, point))
        })
    }

    /// `None` — no in-flight drag owns the cursor.
    pub(in crate::widget_host) fn cursor_move_active_drag_tiers(
        &mut self,
        x: f32,
        y: f32,
        property_rect: op_editor_ui::Rect,
        property_panel_probe: &mut Option<Option<PropertyPanel>>,
    ) -> Option<bool> {
        if let Some(field) = self.image_adjustment_drag {
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
                if let Some(action) = panel.image_adjustment_drag_action(property_rect, field, x) {
                    self.apply_property_action(action);
                    return Some(true);
                }
            }
        }
        if let Some(effect) = self.effect_radius_drag {
            if let Some(panel) = property_panel_probe.as_ref().and_then(Option::as_ref) {
                if let Some(action) = panel.effect_radius_drag_action(property_rect, effect, x) {
                    self.apply_property_action(action);
                    return Some(true);
                }
            }
        }
        if self.apply_image_input_selection_drag_cursor_move(x, y) {
            return Some(true);
        }
        if self.apply_chat_text_selection_drag_cursor_move(x, y) {
            return Some(true);
        }
        if self.apply_chat_input_selection_drag_cursor_move(x, y) {
            return Some(true);
        }
        if self.apply_text_edit_selection_drag_cursor_move(x, y) {
            return Some(true);
        }
        if self.apply_code_selection_drag_cursor_move(x, y) {
            return Some(true);
        }
        if let Some(consumed) = self.apply_image_crop_drag_cursor_move(x, y) {
            return Some(consumed);
        }
        if let Some(consumed) = self.apply_node_drag_cursor_move(x, y) {
            return Some(consumed);
        }
        // Pen handle-drag minting + rubber-band (`pen_press.rs`).
        if let Some(consumed) = self.apply_pen_cursor_move(x, y) {
            return Some(consumed);
        }
        None
    }
}
