//! Wheel and trackpad-pan routing for panels, previews, and the canvas.

use super::WidgetHostNative;
use op_editor_ui::util::scroll_by_max;
use op_editor_ui::widgets::press_flow;
use op_editor_ui::widgets::scroll_flow;
use op_editor_ui::widgets::GitPanel;
use op_editor_ui::Point2D;
impl WidgetHostNative {
    pub(in crate::widget_host) fn refresh_agent_settings_hover_after_scroll(
        &mut self,
        x: f32,
        y: f32,
    ) {
        if self.editor_state.editor_ui.touch_chrome() {
            let settings = &mut self.editor_state.editor_ui.agent_settings;
            settings.builtin_model_menu_hover = None;
            settings.builtin_preset_menu_hover = None;
        } else {
            self.update_agent_settings_hover(x, y);
        }
    }

    /// Scroll the chat transcript message list when a wheel / trackpad
    /// pan lands over the panel body. The body swallows the event so a
    /// wheel over a long reply never zooms the canvas beneath. Mirrors
    /// the model-picker's clamp: the offset rides `[0, max]`, and
    /// reaching the bottom re-pins the transcript to auto-follow new
    /// streamed content.
    fn try_scroll_chat_transcript(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::AIChatPlaceholder;
        if self.editor_state.chat.messages.is_empty() {
            return false;
        }
        let point = Point2D::new(x, y);
        let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) else {
            return false;
        };
        let (body, max) = {
            // Owner-stamp: `transcript_scroll_max` resolves the cache, so a
            // rebuild here tags the slot with this host's owner (not UNOWNED),
            // keeping the cursor hint consistent across wheel + paint.
            let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                .owned_by(self.chat_panel_owner);
            (
                panel.body_rect(chat_rect),
                panel.transcript_scroll_max(chat_rect),
            )
        };
        if !(body).contains(point) {
            return false;
        }
        let chat = &mut self.editor_state.chat;
        // Nothing overflows — swallow so the canvas doesn't zoom under the
        // panel, and keep the view pinned to the bottom.
        if max <= 0.0 {
            if !chat.transcript_pinned || chat.transcript_scroll.offset != 0.0 {
                chat.transcript_pinned = true;
                chat.transcript_scroll.offset = 0.0;
                self.mark_dirty();
            }
            return true;
        }
        let cur = if chat.transcript_pinned {
            max
        } else {
            chat.transcript_scroll.offset.clamp(0.0, max)
        };
        let next = (cur - delta).clamp(0.0, max);
        let pinned = next >= max - 0.5;
        if (next - chat.transcript_scroll.offset).abs() > f32::EPSILON
            || chat.transcript_pinned != pinned
        {
            chat.transcript_scroll.offset = next;
            chat.transcript_pinned = pinned;
            self.mark_dirty();
        }
        true
    }

    /// Scroll the chat panel's draft input when the wheel lands over a
    /// prompt too long for the box. Runs ahead of the transcript handler
    /// (which bails on an empty message list) so a long first prompt is
    /// scrollable before any turn has been sent.
    fn try_scroll_chat_input(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let chat_rect = self.ai_chat_rect(viewport_width, viewport_height);
        let Some(dirty) = scroll_flow::scroll_chat_input(
            &mut self.editor_state,
            chat_rect,
            self.now_ms,
            Point2D::new(x, y),
            delta,
        ) else {
            return false;
        };
        if dirty {
            self.mark_dirty();
        }
        true
    }

    fn try_scroll_agent_model_menu(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.refresh_layout_scene();
        let (panel, panel_rect) = self.agent_settings_geometry(viewport_width, viewport_height);
        let point = Point2D::new(x, y);
        let Some(max) = panel.builtin_model_scroll_max_at(panel_rect, point) else {
            return false;
        };
        let changed = scroll_by_max(
            &mut self
                .editor_state
                .editor_ui
                .agent_settings
                .builtin_model_menu_scroll,
            -delta,
            max,
        );
        if changed {
            self.refresh_agent_settings_hover_after_scroll(x, y);
            self.mark_dirty();
        }
        changed || !self.editor_state.editor_ui.touch_chrome()
    }

    fn try_scroll_agent_preset_menu(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.refresh_layout_scene();
        let (panel, panel_rect) = self.agent_settings_geometry(viewport_width, viewport_height);
        let point = Point2D::new(x, y);
        let Some(max) = panel.builtin_preset_scroll_max_at(panel_rect, point) else {
            return false;
        };
        let changed = scroll_by_max(
            &mut self
                .editor_state
                .editor_ui
                .agent_settings
                .builtin_preset_menu_scroll,
            -delta,
            max,
        );
        if changed {
            self.refresh_agent_settings_hover_after_scroll(x, y);
            self.mark_dirty();
        }
        true
    }

    /// Route a vertical delta to Agent Settings without leaking to the canvas.
    pub(in crate::widget_host) fn scroll_agent_settings_at(
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
        self.refresh_layout_scene();
        let point = Point2D::new(x, y);
        let (panel, panel_rect) = self.agent_settings_geometry(viewport_width, viewport_height);
        if !panel_rect.contains(point) {
            return false;
        }
        drop(panel);
        if self.try_scroll_agent_model_menu(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_agent_preset_menu(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }

        let (panel, panel_rect) = self.agent_settings_geometry(viewport_width, viewport_height);
        if !panel.resolved_content_viewport(panel_rect).contains(point) {
            return true;
        }
        let max_scroll = panel.max_scroll(panel_rect);
        let before = self.editor_state.editor_ui.agent_settings.scroll_y.offset;
        self.editor_state
            .editor_ui
            .agent_settings
            .scroll_y
            .scroll_by(-delta_y, max_scroll, 0.0);
        let changed = (self.editor_state.editor_ui.agent_settings.scroll_y.offset - before).abs()
            > f32::EPSILON;
        let menu_open = self
            .editor_state
            .editor_ui
            .agent_settings
            .builtin_model_menu_open
            .is_some();
        let menu_revealed = menu_open
            && self.ensure_focused_agent_settings_visible(viewport_width, viewport_height);
        let changed = changed || menu_revealed;
        if changed {
            self.refresh_agent_settings_hover_after_scroll(x, y);
            self.mark_dirty();
        }
        true
    }

    /// Scroll the floating VariablesPanel row list when the wheel /
    /// trackpad fires over the open panel.
    fn try_scroll_variables_panel(
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

    fn try_scroll_design_md_panel(
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

    fn try_scroll_locale_picker(
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

    /// Scroll the left-rail LayerPanel when a wheel / trackpad pan
    /// lands over it — the Pages section if the cursor is above the
    /// Layers row viewport, otherwise the Layers section. Returns
    /// `true` when the cursor was over the panel.
    pub(in crate::widget_host) fn try_scroll_layer_panel(
        &mut self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.layers_panel_visible() {
            return false;
        }
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
        let rect = self.layers_content_rect(viewport_width, viewport_height);
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
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let rect = self.layers_content_rect(viewport_width, viewport_height);
        let panel = self.layer_panel();
        if !scroll_flow::reveal_layer_panel_selection(&mut self.editor_state, &panel, rect) {
            return false;
        }
        self.mark_dirty();
        true
    }

    /// Wheel event — zoom centered at (x, y) over the canvas.
    /// Scroll the open icon picker's list when the pointer is over its panel.
    /// Shared by `apply_wheel` and `apply_pan_gesture` so trackpad pans scroll
    /// it too. The picker loads up to 120 local + remote icons — far more than
    /// fit — so the list must scroll; runs before `over_topmost_panel`, which
    /// would otherwise swallow the event without advancing the rows.
    fn try_scroll_icon_picker(
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
        let Some(dirty) = press_flow::scroll_scene_template_center(
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

    fn try_scroll_prompt_center(
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

    pub fn apply_wheel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let cancelled = self.cancel_native_touch_gestures();
        let handled = self.apply_wheel_inner(x, y, delta_y, viewport_width, viewport_height, false);
        handled || cancelled
    }

    /// Pinch or modifier-promoted zoom intent.
    pub fn apply_pinch_gesture(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let cancelled = self.cancel_native_touch_gestures();
        let handled = self.apply_wheel_inner(x, y, delta_y, viewport_width, viewport_height, true);
        handled || cancelled
    }

    fn apply_wheel_inner(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
        zoom_intent: bool,
    ) -> bool {
        if self.try_scroll_figma_import(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_missing_fonts_picker(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_html_import_diagnostics(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_settings_font_picker(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Floating VariablesPanel owns the wheel over its rect — run this
        // BEFORE `over_topmost_panel`, which also lists the variables panel
        // and would otherwise swallow the event WITHOUT scrolling (its rows
        // never advanced because the topmost-panel guard returned first).
        // `try_scroll_variables_panel` swallows the wheel when over the
        // panel, so the "don't zoom the canvas beneath" guarantee holds.
        if self.try_scroll_variables_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_locale_picker(x, y, delta_y, viewport_width) {
            return true;
        }
        if self.try_scroll_design_md_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_icon_picker(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_scene_template_center(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_prompt_center(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Any top-most floating panel (Design-MD / Component-Browser)
        // owns the wheel before lower layers — a scroll over them
        // never reaches the modal / Git panel / canvas.
        if self.over_topmost_panel(x, y, viewport_width, viewport_height) {
            return true;
        }
        // Open chat model-picker — a wheel over its dropdown scrolls
        // the model list instead of zooming the canvas.
        if self.editor_state.editor_ui.chat_model_picker.open {
            use op_editor_ui::widgets::ai_chat_model_picker::max_picker_scroll;
            use op_editor_ui::widgets::AIChatPlaceholder;
            let picker = self
                .ai_chat_rect(viewport_width, viewport_height)
                .and_then(|chat_rect| {
                    AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                        .model_picker_bounds(chat_rect)
                });
            if let Some(picker) = picker {
                if (picker).contains(Point2D::new(x, y)) {
                    let max = max_picker_scroll(
                        &self.editor_state.chat.available_models,
                        self.editor_state.editor_ui.chat_model_picker_input.text(),
                    );
                    let next = (self.editor_state.editor_ui.chat_model_picker.scroll.offset
                        - delta_y)
                        .clamp(0.0, max);
                    self.editor_state.editor_ui.chat_model_picker.scroll.offset = next;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Chat transcript message list — a wheel over the body scrolls
        // the conversation; the pinned-to-bottom auto-follow resumes once
        // the user scrolls back to the bottom.
        if self.try_scroll_chat_input(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_chat_transcript(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Agent-settings modal owns wheel.
        if self.scroll_agent_settings_at(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Floating Git panel — a wheel over its open diff view
        // scrolls the diff (vertically; horizontally with Shift held)
        // instead of zooming the canvas.
        if let Some(panel_rect) = self.git_panel_outer_rect(viewport_width, viewport_height) {
            if (panel_rect).contains(Point2D::new(x, y))
                && self.editor_state.editor_ui.git_panel.diff.is_some()
            {
                let panel = GitPanel::for_editor(&self.editor_state);
                if self.shift_held {
                    // Shift+wheel — scroll the diff sideways.
                    let max = panel.map(|p| p.diff_max_h_scroll()).unwrap_or(0);
                    let cols = (delta_y.abs() / 6.0).ceil().max(1.0) as usize;
                    if let Some(diff) = &mut self.editor_state.editor_ui.git_panel.diff {
                        diff.h_scroll = if delta_y > 0.0 {
                            diff.h_scroll.saturating_sub(cols)
                        } else {
                            (diff.h_scroll + cols).min(max)
                        };
                    }
                } else {
                    let max = panel.map(|p| p.diff_max_scroll()).unwrap_or(0);
                    // Convert the (pixel or line) delta into diff rows.
                    let rows = (delta_y.abs() / 14.0).ceil().max(1.0) as usize;
                    if let Some(diff) = &mut self.editor_state.editor_ui.git_panel.diff {
                        diff.scroll = if delta_y > 0.0 {
                            diff.scroll.saturating_sub(rows)
                        } else {
                            (diff.scroll + rows).min(max)
                        };
                    }
                }
                self.mark_dirty();
                return true;
            }
        }
        // Route inspector scroll before the canvas.
        if self.try_scroll_property_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Route Layers scroll before the canvas.
        let (layer_dx, layer_dy) = if self.shift_held {
            (delta_y, 0.0)
        } else {
            (0.0, delta_y)
        };
        if self.try_scroll_layer_panel(x, y, layer_dx, layer_dy, viewport_width, viewport_height) {
            return true;
        }
        if self.editor_state.editor_ui.touch_chrome()
            && self.editor_state.editor_ui.mobile_sheet.is_some()
        {
            return true;
        }
        // A device frame owns every wheel over the canvas. Branch on
        // mode, not cached geometry, so missing geometry fails closed.
        if self.device_mode_active() && self.over_canvas(x, y, viewport_width, viewport_height) {
            if !zoom_intent {
                if self.preview_dispatch_wheel(x, y, 0.0, delta_y, viewport_width, viewport_height)
                {
                    return true;
                }
                self.apply_device_scroll(delta_y);
            }
            return true;
        }
        // Canvas-mode preview preserves its existing runtime-first routing.
        if self.preview.is_some()
            && self.preview_dispatch_wheel(x, y, 0.0, delta_y, viewport_width, viewport_height)
        {
            return true;
        }
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.editor_state.viewport.zoom_at(cursor, delta_y);
        self.note_viewport_zoom_gesture();
        // Viewport-only changes do not invalidate the layout scene.
        true
    }

    /// 2-finger trackpad pan — translate viewport by (dx, dy).
    pub fn apply_pan_gesture(
        &mut self,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let cancelled = self.cancel_native_touch_gestures();
        if self.try_scroll_missing_fonts_picker(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_html_import_diagnostics(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_settings_font_picker(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        // Floating VariablesPanel owns trackpad pans over its rect.
        // See `apply_wheel` for why this must precede the topmost
        // overlay guard.
        if self.try_scroll_variables_panel(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_locale_picker(x, y, dy, viewport_width) {
            return true;
        }
        if self.try_scroll_design_md_panel(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_icon_picker(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        // Trackpad pans arrive here rather than through `apply_wheel_inner`;
        // a panel wired into only one of the two ladders still lets the
        // canvas move under a two-finger scroll (reported 2026-08-02).
        if self.try_scroll_scene_template_center(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_prompt_center(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        // Any top-most floating panel owns trackpad scroll first.
        if self.over_topmost_panel(x, y, viewport_width, viewport_height) {
            return true;
        }
        // Open chat model-picker owns trackpad scroll over its
        // dropdown, same as the wheel path.
        if self.editor_state.editor_ui.chat_model_picker.open {
            use op_editor_ui::widgets::ai_chat_model_picker::max_picker_scroll;
            use op_editor_ui::widgets::AIChatPlaceholder;
            let picker = self
                .ai_chat_rect(viewport_width, viewport_height)
                .and_then(|chat_rect| {
                    AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                        .model_picker_bounds(chat_rect)
                });
            if let Some(picker) = picker {
                if (picker).contains(Point2D::new(x, y)) {
                    let max = max_picker_scroll(
                        &self.editor_state.chat.available_models,
                        self.editor_state.editor_ui.chat_model_picker_input.text(),
                    );
                    let next = (self.editor_state.editor_ui.chat_model_picker.scroll.offset - dy)
                        .clamp(0.0, max);
                    self.editor_state.editor_ui.chat_model_picker.scroll.offset = next;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if self.try_scroll_chat_input(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_chat_transcript(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        // Agent-settings modal owns trackpad scroll same as wheel.
        if self.scroll_agent_settings_at(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        // Floating Git panel — a trackpad scroll over its open diff
        // pans the diff (dy vertically, dx sideways) like the wheel.
        if let Some(panel_rect) = self.git_panel_outer_rect(viewport_width, viewport_height) {
            if (panel_rect).contains(Point2D::new(x, y))
                && self.editor_state.editor_ui.git_panel.diff.is_some()
            {
                let panel = GitPanel::for_editor(&self.editor_state);
                let max_v = panel.as_ref().map(|p| p.diff_max_scroll()).unwrap_or(0);
                let max_h = panel.map(|p| p.diff_max_h_scroll()).unwrap_or(0);
                if let Some(diff) = &mut self.editor_state.editor_ui.git_panel.diff {
                    // Below a 1 px dead-zone the axis is jitter and
                    // stays put; any real delta moves at least one
                    // step so a slow trackpad scroll is never lost.
                    let steps = |delta: f32, unit: f32| -> usize {
                        if delta.abs() < 1.0 {
                            0
                        } else {
                            (delta.abs() / unit).round().max(1.0) as usize
                        }
                    };
                    let rows = steps(dy, 14.0);
                    diff.scroll = if dy > 0.0 {
                        diff.scroll.saturating_sub(rows)
                    } else {
                        (diff.scroll + rows).min(max_v)
                    };
                    let cols = steps(dx, 6.0);
                    diff.h_scroll = if dx > 0.0 {
                        diff.h_scroll.saturating_sub(cols)
                    } else {
                        (diff.h_scroll + cols).min(max_h)
                    };
                }
                self.mark_dirty();
                return true;
            }
        }
        // Route inspector pan before the canvas.
        if self.try_scroll_property_panel_2d(x, y, dx, dy, viewport_width, viewport_height) {
            return true;
        }
        // Route Layers pan before the canvas.
        if self.try_scroll_layer_panel(x, y, dx, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.editor_state.editor_ui.touch_chrome()
            && self.editor_state.editor_ui.mobile_sheet.is_some()
        {
            return true;
        }
        if self.device_mode_active() && self.over_canvas(x, y, viewport_width, viewport_height) {
            if self.preview_dispatch_wheel(x, y, dx, dy, viewport_width, viewport_height) {
                return true;
            }
            self.apply_device_scroll(dy);
            return true;
        }
        // Canvas-mode preview preserves its existing runtime-first routing.
        if self.preview.is_some()
            && self.preview_dispatch_wheel(x, y, dx, dy, viewport_width, viewport_height)
        {
            return true;
        }
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return cancelled;
        }
        if dx == 0.0 && dy == 0.0 {
            return cancelled;
        }
        self.editor_state.viewport.pan(dx, dy);
        self.note_viewport_gesture();
        // No `mark_dirty()`: a pan only translates the viewport, not
        // the document tree — see the `apply_wheel` zoom branch. The
        // `true` return drives the repaint.
        true
    }
}
