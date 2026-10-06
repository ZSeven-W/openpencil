//! 2-finger trackpad pan routing — split from `scroll.rs` at the
//! 800-line cap; pure code motion, the ladder order is unchanged.

use crate::widget_host::WidgetHostNative;
use op_editor_ui::widgets::GitPanel;
use op_editor_ui::Point2D;

impl WidgetHostNative {
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
        if self
            .wheel_home_model_picker(x, y, dy, viewport_width, viewport_height)
            .is_some()
        {
            return true;
        }
        if self.try_scroll_home(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
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
        if self.mobile_sheet_owns_point(Point2D::new(x, y), viewport_width, viewport_height) {
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
