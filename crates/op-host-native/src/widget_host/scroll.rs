//! Wheel + trackpad-pan input — extracted from `input.rs` to keep it
//! under the repo's 800-line cap. Both handlers route a scroll over
//! the floating Git panel's open diff into the diff view, and
//! otherwise zoom / pan the canvas.

use super::helpers::rect_contains;
use super::WidgetHostNative;
use op_editor_ui::widgets::GitPanel;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Wheel event — zoom centered at (x, y) over the canvas.
    pub fn apply_wheel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // Any top-most floating panel (Design-MD / Component-Browser)
        // owns the wheel before lower layers — a scroll over them
        // never reaches the modal / Git panel / canvas.
        if self.over_topmost_panel(x, y, viewport_width, viewport_height) {
            return true;
        }
        // Open chat model-picker — a wheel over its dropdown scrolls
        // the model list instead of zooming the canvas.
        if self.editor_state.editor_ui.chat_model_picker_open {
            use op_editor_ui::widgets::ai_chat_model_picker::max_picker_scroll;
            use op_editor_ui::widgets::AIChatPlaceholder;
            let picker = self
                .ai_chat_rect(viewport_width, viewport_height)
                .and_then(|chat_rect| {
                    AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                        .model_picker_bounds(chat_rect)
                });
            if let Some(picker) = picker {
                if rect_contains(picker, Point2D::new(x, y)) {
                    let max = max_picker_scroll(&self.editor_state.chat.available_models);
                    let next = (self.editor_state.editor_ui.chat_model_picker_scroll - delta_y)
                        .clamp(0.0, max);
                    self.editor_state.editor_ui.chat_model_picker_scroll = next;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Agent-settings modal owns wheel.
        if self.editor_state.editor_ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            self.refresh_layout_scene();
            let panel = AgentSettingsPanel::for_editor(&self.editor_state);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            if panel_rect.origin.x <= x
                && x <= panel_rect.origin.x + panel_rect.size.x
                && panel_rect.origin.y <= y
                && y <= panel_rect.origin.y + panel_rect.size.y
            {
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                let next = (self.editor_state.editor_ui.agent_settings.scroll_y - delta_y)
                    .clamp(0.0, max_scroll);
                self.editor_state.editor_ui.agent_settings.scroll_y = next;
                self.mark_dirty();
                return true;
            }
        }
        // Floating Git panel — a wheel over its open diff view
        // scrolls the diff (vertically; horizontally with Shift held)
        // instead of zooming the canvas.
        if let Some(panel_rect) = self.git_panel_rect(viewport_width, viewport_height) {
            if rect_contains(panel_rect, Point2D::new(x, y))
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
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        // Canvas-local coords keep the zoom anchor under the cursor.
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.editor_state.viewport.zoom_at(cursor, delta_y);
        // No `mark_dirty()`: a zoom only changes the viewport
        // transform, not the document tree, so the cached
        // `layout_scene` stays valid — re-running the taffy layout
        // solve + skia text measurement every wheel tick was the
        // canvas-zoom jank. The `true` return still drives the
        // repaint, which re-applies the new viewport transform.
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
        // Any top-most floating panel owns trackpad scroll first.
        if self.over_topmost_panel(x, y, viewport_width, viewport_height) {
            return true;
        }
        // Open chat model-picker owns trackpad scroll over its
        // dropdown, same as the wheel path.
        if self.editor_state.editor_ui.chat_model_picker_open {
            use op_editor_ui::widgets::ai_chat_model_picker::max_picker_scroll;
            use op_editor_ui::widgets::AIChatPlaceholder;
            let picker = self
                .ai_chat_rect(viewport_width, viewport_height)
                .and_then(|chat_rect| {
                    AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                        .model_picker_bounds(chat_rect)
                });
            if let Some(picker) = picker {
                if rect_contains(picker, Point2D::new(x, y)) {
                    let max = max_picker_scroll(&self.editor_state.chat.available_models);
                    let next =
                        (self.editor_state.editor_ui.chat_model_picker_scroll - dy).clamp(0.0, max);
                    self.editor_state.editor_ui.chat_model_picker_scroll = next;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Agent-settings modal owns trackpad scroll same as wheel.
        if self.editor_state.editor_ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            self.refresh_layout_scene();
            let panel = AgentSettingsPanel::for_editor(&self.editor_state);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            if panel_rect.origin.x <= x
                && x <= panel_rect.origin.x + panel_rect.size.x
                && panel_rect.origin.y <= y
                && y <= panel_rect.origin.y + panel_rect.size.y
            {
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                let next = (self.editor_state.editor_ui.agent_settings.scroll_y - dy)
                    .clamp(0.0, max_scroll);
                self.editor_state.editor_ui.agent_settings.scroll_y = next;
                self.mark_dirty();
                return true;
            }
        }
        // Floating Git panel — a trackpad scroll over its open diff
        // pans the diff (dy vertically, dx sideways) like the wheel.
        if let Some(panel_rect) = self.git_panel_rect(viewport_width, viewport_height) {
            if rect_contains(panel_rect, Point2D::new(x, y))
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
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.editor_state.viewport.pan(dx, dy);
        // No `mark_dirty()`: a pan only translates the viewport, not
        // the document tree — see the `apply_wheel` zoom branch. The
        // `true` return drives the repaint.
        true
    }
}
