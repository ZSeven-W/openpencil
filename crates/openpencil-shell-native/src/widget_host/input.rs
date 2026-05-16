//! Non-press input handlers on `WidgetHostNative`. press → press.rs.
//!
//! `EditorState` is the host's source of truth. Scalar / chrome
//! reads go straight to `editor_state`; node-tree hit-tests run
//! against the derived paint `Document` (`paint_doc`), refreshed at
//! the top of each handler. Every mutation flags `editor_state` so
//! the next paint re-derives.

use super::helpers::{
    resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH,
};
use super::{PanelResizeKind, WidgetHostNative};
use openpencil_shell_core::{Point2D, Rect};

impl WidgetHostNative {
    /// True iff a text-input surface owns the keyboard.
    pub(in crate::widget_host) fn input_active(&self) -> bool {
        let ui = &self.editor_state.ui;
        ui.layer_rename.is_some()
            || ui.text_editing.is_some()
            || ui.property_focus.is_some()
            || self.editor_state.editor_ui.variable_row_focus.is_some()
            || self.editor_state.editor_ui.agent_settings.focus.is_some()
            || self.editor_state.chat.focused
    }

    pub fn settings_focus_active(&self) -> bool {
        self.editor_state.editor_ui.agent_settings.focus.is_some()
    }

    /// Wheel event — zoom centered at (x, y) over the canvas.
    pub fn apply_wheel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // Agent-settings modal owns wheel.
        if self.editor_state.editor_ui.agent_settings_open {
            use openpencil_shell_core::widgets::agent_settings_panel::AgentSettingsPanel;
            self.refresh_paint_doc();
            let panel = AgentSettingsPanel::for_document(&self.paint_doc);
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
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        // Canvas-local coords keep the zoom anchor under the cursor.
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.editor_state.viewport.zoom_at(cursor, delta_y);
        self.mark_dirty();
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
        // Agent-settings modal owns trackpad scroll same as wheel.
        if self.editor_state.editor_ui.agent_settings_open {
            use openpencil_shell_core::widgets::agent_settings_panel::AgentSettingsPanel;
            self.refresh_paint_doc();
            let panel = AgentSettingsPanel::for_document(&self.paint_doc);
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
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.editor_state.viewport.pan(dx, dy);
        self.mark_dirty();
        true
    }

    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        if self.editor_state.editor_ui.agent_settings_open
            && self.update_agent_settings_hover(x, y)
        {
            return true;
        }
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            if let Some(kind) = state.drag {
                use op_editor_core::ui_draft::ColorPickerDrag;
                use openpencil_shell_core::widgets::color_picker::ColorPicker;
                self.refresh_paint_doc();
                let picker = ColorPicker::for_state(
                    &self.paint_doc,
                    self.paint_doc.ui.color_picker.clone().unwrap(),
                );
                let panel = picker.rect(self.last_viewport_w, self.last_viewport_h);
                let point = Point2D::new(x, y);
                match kind {
                    ColorPickerDrag::SvBox => {
                        let (s, v) = picker.sv_at(panel, point);
                        let _ = self.editor_state.color_picker_set_hsv(state.hue, s, v);
                    }
                    ColorPickerDrag::HueSlider => {
                        let h = picker.hue_at(panel, point);
                        let _ = self
                            .editor_state
                            .color_picker_set_hsv(h, state.sat, state.val);
                    }
                }
                self.mark_dirty();
                return true;
            }
        }
        // Pen rubber-band — track cursor doc coord for preview.
        if self.editor_state.ui.pen_in_progress.is_some() {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc = self.editor_state.viewport.to_document(canvas_local);
            self.editor_state.ui.pen_cursor_doc = Some(doc);
            self.mark_dirty();
            return true;
        }
        if let Some(state) = self.editor_state.editor_ui.layer_context_menu.clone() {
            use openpencil_shell_core::widgets::layer_context_menu::LayerContextMenu;
            self.refresh_paint_doc();
            let menu = LayerContextMenu::for_state(
                &self.paint_doc,
                self.paint_doc.ui.layer_context_menu.clone().unwrap(),
            );
            let new_hover = menu.hovered_row_at(Point2D::new(x, y)).map(|i| i as u8);
            if new_hover != state.hovered_row {
                self.editor_state.editor_ui.layer_context_menu =
                    Some(op_editor_core::editor_ui_state::LayerContextMenuState {
                        hovered_row: new_hover,
                        ..state
                    });
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.file_menu_open {
            use openpencil_shell_core::widgets::file_menu::FileMenu;
            use openpencil_shell_core::widgets::top_bar::TopBar;
            self.refresh_paint_doc();
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(
                    self.last_viewport_w,
                    openpencil_shell_core::widgets::TOP_BAR_HEIGHT,
                ),
            };
            let anchor = TopBar::file_menu_rect(top_bar_rect);
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let menu = FileMenu::from_document(&self.paint_doc, now_secs);
            let panel = menu.rect_at(anchor);
            let new_hover = menu.hovered_at(panel, Point2D::new(x, y));
            // shell-core `FileMenuChoice` option → op-editor-core.
            let new_hover_ec = new_hover.map(op_pen_loader::rev::file_menu_choice);
            if new_hover_ec != self.editor_state.editor_ui.file_menu_hover {
                self.editor_state.editor_ui.file_menu_hover = new_hover_ec;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.locale_picker_open {
            use openpencil_shell_core::widgets::locale_picker::LocalePicker;
            self.refresh_paint_doc();
            let panel = self.locale_picker_rect(self.last_viewport_w);
            let picker = LocalePicker::for_document(&self.paint_doc);
            let new_hover = picker.hit_test(panel, Point2D::new(x, y));
            let new_hover_ec = new_hover.map(op_pen_loader::rev::locale);
            if new_hover_ec != self.editor_state.editor_ui.locale_picker_hover {
                self.editor_state.editor_ui.locale_picker_hover = new_hover_ec;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.shape_picker_open {
            use openpencil_shell_core::widgets::shape_picker::ShapePicker;
            self.refresh_paint_doc();
            let panel = self.shape_picker_rect(self.last_viewport_w, self.last_viewport_h);
            let picker = ShapePicker::for_document(&self.paint_doc);
            let new_hover = picker.hit_test(panel, Point2D::new(x, y));
            let new_hover_ec = new_hover.map(op_pen_loader::rev::shape_choice);
            if new_hover_ec != self.editor_state.editor_ui.shape_picker_hover {
                self.editor_state.editor_ui.shape_picker_hover = new_hover_ec;
                self.mark_dirty();
                return true;
            }
        }
        if let Some(drag) = self.rotate_drag {
            let cursor_angle = (y - drag.center_screen_y).atan2(x - drag.center_screen_x);
            let new_rotation = drag.start_rotation + (cursor_angle - drag.start_cursor_angle);
            self.editor_state.set_selected_rotation(new_rotation);
            self.mark_dirty();
            return true;
        }
        if let Some(drag) = self.handle_drag {
            let zoom = self.editor_state.viewport.zoom.max(0.0001);
            let dx = (x - drag.start_screen_x) / zoom;
            let dy = (y - drag.start_screen_y) / zoom;
            let new_bounds = resize_bounds(drag.start_bounds, drag.handle, dx, dy);
            self.editor_state.set_selected_bounds(rect_to_doc_rect(new_bounds));
            self.mark_dirty();
            return true;
        }
        if let Some(drag) = self.create_drag {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let cur = self.editor_state.viewport.to_document(canvas_local);
            let min_x = drag.start_doc_x.min(cur.x);
            let min_y = drag.start_doc_y.min(cur.y);
            // Text needs room for the placeholder glyphs; shape
            // tools start at 1 px so the drag immediately sizes
            // the node to the cursor.
            let (min_w, min_h) = match self.editor_state.tool {
                op_editor_core::Tool::Text => (96.0_f32, 24.0_f32),
                _ => (1.0_f32, 1.0_f32),
            };
            let w = (drag.start_doc_x - cur.x).abs().max(min_w);
            let h = (drag.start_doc_y - cur.y).abs().max(min_h);
            let new_bounds = Rect::xywh(min_x, min_y, w, h);
            self.editor_state.set_selected_bounds(rect_to_doc_rect(new_bounds));
            self.mark_dirty();
            return true;
        }
        if let Some(drag) = self.node_drag.as_mut() {
            let zoom = self.editor_state.viewport.zoom.max(0.0001);
            let dx = (x - drag.last_screen_x) / zoom;
            let dy = (y - drag.last_screen_y) / zoom;
            drag.last_screen_x = x;
            drag.last_screen_y = y;
            if dx != 0.0 || dy != 0.0 {
                self.editor_state.translate_selected(dx as f64, dy as f64);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Path-anchor drag — always write the current cursor position
        // (codex BLOCK: drag-back-to-start was being silently dropped).
        if self.path_anchor_drag.is_some() {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc_point = self.editor_state.viewport.to_document(canvas_local);
            let (id, idx, start) = {
                let d = self.path_anchor_drag.as_ref().unwrap();
                (d.node_id.clone(), d.anchor_index, d.start_doc)
            };
            self.editor_state.set_path_anchor_position(
                id,
                idx,
                (doc_point.x as f64, doc_point.y as f64),
            );
            self.mark_dirty();
            if (doc_point.x - start.x).abs() > 0.001 || (doc_point.y - start.y).abs() > 0.001 {
                if let Some(d) = self.path_anchor_drag.as_mut() {
                    d.moved = true;
                }
            }
            return true;
        }
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if self.layer_drag.is_some() {
            self.refresh_paint_doc();
            let source_id = self.layer_drag.as_ref().unwrap().source.clone();
            let still_present = self
                .paint_doc
                .active_page()
                .map(|p| p.find(&source_id).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return true;
            }
            let d = self.layer_drag.as_mut().unwrap();
            d.current_x = x;
            d.current_y = y;
            // Vertical-only activation — horizontal wiggle preserved
            // for selection / eye / lock click-feel.
            if !d.active && (y - d.start_y).abs() > 4.0 {
                d.active = true;
            }
            return true;
        }
        if let Some(resize) = self.panel_resize {
            let dx = x - resize.start_x;
            match resize.kind {
                PanelResizeKind::LayerRight => {
                    let new_w =
                        (resize.start_width + dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.editor_state.editor_ui.layer_panel_width = new_w;
                }
                PanelResizeKind::PropertyLeft => {
                    let new_w =
                        (resize.start_width - dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.editor_state.editor_ui.property_panel_width = new_w;
                }
            }
            self.mark_dirty();
            return true;
        }
        if let Some(d) = self.chat_drag.as_mut() {
            d.pos_x = x - d.grab_dx;
            d.pos_y = y - d.grab_dy;
            return true;
        }
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            self.mark_dirty();
            return true;
        }
        // Align toolbar hover sync — AFTER drag detection.
        let new_hover = if self.editor_state.selection_count() >= 2 {
            self.align_toolbar_hit(x, y, self.last_viewport_w, self.last_viewport_h)
        } else {
            None
        };
        let new_hover_ec = new_hover.map(op_pen_loader::rev::align_action);
        if new_hover_ec != self.editor_state.editor_ui.align_toolbar_hover {
            self.editor_state.editor_ui.align_toolbar_hover = new_hover_ec;
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Mouse-release — ends active drag; chat-panel snaps corner.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        // Drop color-picker drag.
        if self.editor_state.ui.color_picker.is_some() {
            self.editor_state.color_picker_set_drag(None);
            self.mark_dirty();
        }
        if self.editor_state.editor_ui.agent_settings_drag.is_some() {
            self.editor_state.editor_ui.agent_settings_drag = None;
            self.mark_dirty();
        }
        if self.panel_resize.take().is_some() {
            return true;
        }
        if self.rotate_drag.take().is_some() {
            return true;
        }
        if self.handle_drag.take().is_some() {
            return true;
        }
        if self.create_drag.take().is_some() {
            // Switch back to Select so the user can immediately
            // refine the freshly-created shape.
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.mark_dirty();
            return true;
        }
        if self.node_drag.take().is_some() {
            return true;
        }
        if let Some(drag) = self.path_anchor_drag.take() {
            // Push history snapshot only when the anchor actually
            // moved (codex CONCERN — a press-release without motion
            // was polluting the undo stack with no-op entries).
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
                return true;
            }
            return false;
        }
        if let Some(m) = self.marquee_drag.take() {
            self.commit_marquee_selection(m, viewport_w, viewport_h);
            return true;
        }
        if let Some(d) = self.layer_drag.take() {
            return self.commit_layer_drag(d, viewport_h);
        }
        if let Some(d) = self.chat_drag.take() {
            // Use the live panel size (expanded vs collapsed) so a
            // dragged collapsed pill snaps to the corner closest to
            // its actual center.
            let (panel_w, panel_h) = self.ai_chat_size();
            let center = Point2D::new(d.pos_x + panel_w / 2.0, d.pos_y + panel_h / 2.0);
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
            self.editor_state.chat.anchor =
                op_editor_core::ChatAnchor::nearest(center, cx0, cy0, cw, ch);
            self.mark_dirty();
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }

    /// Viewport-less release variant — drops viewport-bound drags.
    pub fn apply_release(&mut self) -> bool {
        if self.panel_resize.take().is_some() {
            return true;
        }
        if self.rotate_drag.take().is_some() {
            return true;
        }
        if self.handle_drag.take().is_some() {
            return true;
        }
        if self.create_drag.take().is_some() {
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.mark_dirty();
            return true;
        }
        if self.node_drag.take().is_some() {
            return true;
        }
        if self.marquee_drag.take().is_some() {
            // Can't compute the doc-space marquee rect without a
            // viewport; drop without committing.
            return true;
        }
        if self.layer_drag.take().is_some() {
            // Same story as marquee — no viewport, drop the candidate.
            return true;
        }
        // Chat drag without viewport — drop it (best effort).
        if self.chat_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }
}

/// Convert a shell-core `Rect` (screen / doc px) into op-editor-core's
/// `DocRect`. Both crates carry `f32` rects; `DocRect` is `f64`.
pub(in crate::widget_host) fn rect_to_doc_rect(r: Rect) -> op_editor_core::DocRect {
    op_editor_core::DocRect {
        x: r.origin.x as f64,
        y: r.origin.y as f64,
        w: r.size.x as f64,
        h: r.size.y as f64,
    }
}
