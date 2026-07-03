//! Wheel + trackpad-pan input — extracted from `input.rs` to keep it
//! under the repo's 800-line cap. Both handlers route a scroll over
//! the floating Git panel's open diff into the diff view, and
//! otherwise zoom / pan the canvas.

use super::WidgetHostNative;
use op_editor_ui::util::scroll_by_max;
use op_editor_ui::widgets::GitPanel;
use op_editor_ui::Point2D;

impl WidgetHostNative {
    fn try_scroll_chat_checklist(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::AIChatPlaceholder;
        let point = Point2D::new(x, y);
        let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) else {
            return false;
        };
        let (checklist, max) = {
            let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms);
            let Some(checklist) = panel.fixed_checklist_bounds(chat_rect) else {
                return false;
            };
            (checklist, panel.fixed_checklist_scroll_max())
        };
        if !(checklist).contains(point) {
            return false;
        }
        if scroll_by_max(&mut self.editor_state.chat.checklist_scroll, -delta, max) {
            self.mark_dirty();
        }
        true
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
            let panel = AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms);
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

    fn try_scroll_agent_preset_menu(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
        self.refresh_layout_scene();
        let panel = AgentSettingsPanel::for_editor(&self.editor_state);
        let panel_rect = panel.rect(viewport_width, viewport_height);
        let point = Point2D::new(x, y);
        let Some(max) = panel.builtin_preset_scroll_max_at(panel_rect, point) else {
            return false;
        };
        let settings = &mut self.editor_state.editor_ui.agent_settings;
        if scroll_by_max(&mut settings.builtin_preset_menu_scroll, -delta, max) {
            self.mark_dirty();
        }
        true
    }

    /// Scroll the floating VariablesPanel row list when the wheel /
    /// trackpad fires over the open panel (TS `overflow-y-auto` rows
    /// region). The whole panel rect swallows the event so a wheel
    /// over its header can't zoom the canvas beneath.
    fn try_scroll_variables_panel(
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

    fn try_scroll_design_md_panel(
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

    fn try_scroll_locale_picker(
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

    /// Scroll the right-rail PropertyPanel when a wheel / trackpad
    /// pan lands over it. `delta` is the vertical scroll delta
    /// (wheel `delta_y` or pan `dy`). Returns `true` when the cursor
    /// was over the inspector, so the caller stops before zooming.
    fn try_scroll_property_panel(
        &mut self,
        x: f32,
        y: f32,
        delta: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
        use op_editor_ui::Rect;
        // A wheel over the open font-family picker scrolls ITS list,
        // not the panel behind it (font_picker_dispatch.rs).
        if self.try_scroll_font_picker(x, y, delta, viewport_width, viewport_height) {
            return true;
        }
        let Some(panel) = PropertyPanel::for_selection_at(&self.editor_state, self.now_ms) else {
            return false;
        };
        let pw = self.editor_state.editor_ui.property_panel_width;
        let property_rect = Rect {
            origin: Point2D::new(viewport_width - pw, TOP_BAR_HEIGHT),
            size: Point2D::new(pw, (viewport_height - TOP_BAR_HEIGHT).max(0.0)),
        };
        if !(property_rect).contains(Point2D::new(x, y)) {
            return false;
        }
        // Code tab: a wheel over the framework strip scrolls it horizontally
        // (it's a single row), not the panel vertically.
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
                if scroll_by_max(&mut cg.framework_scroll, -delta, max) {
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
                if scroll_by_max(&mut cg.code_scroll, -delta, max) {
                    self.mark_dirty();
                }
                return true;
            }
        }
        let max = (panel.content_height(property_rect) - property_rect.size.y).max(0.0);
        if scroll_by_max(
            &mut self.editor_state.editor_ui.property_panel_scroll,
            -delta,
            max,
        ) {
            self.mark_dirty();
        }
        true
    }

    /// Scroll the left-rail LayerPanel when a wheel / trackpad pan
    /// lands over it — the Pages section if the cursor is above the
    /// Layers row viewport, otherwise the Layers section. Returns
    /// `true` when the cursor was over the panel.
    fn try_scroll_layer_panel(
        &mut self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{LayerPanel, TOP_BAR_HEIGHT};
        use op_editor_ui::Rect;
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
        if !self.editor_state.editor_ui.icon_picker.open {
            return false;
        }
        use op_editor_ui::widgets::icon_picker_panel::IconPickerPanel;
        let Some(rect) = self.icon_picker_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        if !(rect).contains(Point2D::new(x, y)) {
            return false;
        }
        if let Some(panel) = IconPickerPanel::for_editor(&self.editor_state) {
            let max = panel.icon_picker_max_scroll(rect);
            let scroll = &mut self.editor_state.editor_ui.icon_picker.scroll;
            let next = (scroll.offset - delta_y).clamp(0.0, max);
            if next != scroll.offset {
                scroll.offset = next;
                self.mark_dirty();
            }
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
        if self.try_scroll_chat_checklist(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Chat transcript message list — a wheel over the body scrolls
        // the conversation; the pinned-to-bottom auto-follow resumes once
        // the user scrolls back to the bottom.
        if self.try_scroll_chat_transcript(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Agent-settings modal owns wheel.
        if self.editor_state.editor_ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            self.refresh_layout_scene();
            let panel_rect = AgentSettingsPanel::for_editor(&self.editor_state)
                .rect(viewport_width, viewport_height);
            if panel_rect.origin.x <= x
                && x <= panel_rect.origin.x + panel_rect.size.x
                && panel_rect.origin.y <= y
                && y <= panel_rect.origin.y + panel_rect.size.y
            {
                if self.try_scroll_agent_preset_menu(x, y, delta_y, viewport_width, viewport_height)
                {
                    return true;
                }
                let panel = AgentSettingsPanel::for_editor(&self.editor_state);
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .scroll_y
                    .scroll_by(-delta_y, max_scroll, 0.0);
                self.mark_dirty();
                return true;
            }
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
        // Right-rail inspector — a wheel over it scrolls the
        // PropertyPanel content instead of zooming the canvas.
        if self.try_scroll_property_panel(x, y, delta_y, viewport_width, viewport_height) {
            return true;
        }
        // Left-rail LayerPanel — a wheel over it scrolls its Pages /
        // Layers section instead of zooming.
        let (layer_dx, layer_dy) = if self.shift_held {
            (delta_y, 0.0)
        } else {
            (0.0, delta_y)
        };
        if self.try_scroll_layer_panel(x, y, layer_dx, layer_dy, viewport_height) {
            return true;
        }
        // Live preview: a wheel over a node carrying `events.onScroll`
        // goes to the runtime; otherwise fall through so the user can
        // still pan/zoom the canvas while previewing. Runs below the
        // panel/picker guards — they own the wheel over their rects.
        if self.preview.is_some()
            && self.preview_dispatch_wheel(x, y, delta_y, viewport_width, viewport_height)
        {
            return true;
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
        if self.try_scroll_chat_checklist(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        if self.try_scroll_chat_transcript(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        // Agent-settings modal owns trackpad scroll same as wheel.
        if self.editor_state.editor_ui.agent_settings_open {
            use op_editor_ui::widgets::agent_settings_panel::AgentSettingsPanel;
            self.refresh_layout_scene();
            let panel_rect = AgentSettingsPanel::for_editor(&self.editor_state)
                .rect(viewport_width, viewport_height);
            if panel_rect.origin.x <= x
                && x <= panel_rect.origin.x + panel_rect.size.x
                && panel_rect.origin.y <= y
                && y <= panel_rect.origin.y + panel_rect.size.y
            {
                if self.try_scroll_agent_preset_menu(x, y, dy, viewport_width, viewport_height) {
                    return true;
                }
                let panel = AgentSettingsPanel::for_editor(&self.editor_state);
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                self.editor_state
                    .editor_ui
                    .agent_settings
                    .scroll_y
                    .scroll_by(-dy, max_scroll, 0.0);
                self.mark_dirty();
                return true;
            }
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
        // Right-rail inspector — a trackpad pan over it scrolls the
        // PropertyPanel content instead of panning the canvas.
        if self.try_scroll_property_panel(x, y, dy, viewport_width, viewport_height) {
            return true;
        }
        // Left-rail LayerPanel — a trackpad pan over it scrolls its
        // Pages / Layers section instead of panning the canvas.
        if self.try_scroll_layer_panel(x, y, dx, dy, viewport_height) {
            return true;
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
