//! Non-press input handlers on `WidgetHostNative`. press → press.rs.

use super::helpers::{
    parse_hex_color, resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH, TOOLBAR_INSET_X,
    TOOLBAR_INSET_Y,
};
use super::{PanelResizeKind, WidgetHostNative};
use openpencil_shell_core::document::{ChatAnchor, PropertyFocus};
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayoutCx, Toolbar, Widget, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

impl WidgetHostNative {
    /// True iff a text-input surface owns the keyboard.
    pub(in crate::widget_host) fn input_active(&self) -> bool {
        self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.ui.property_focus.is_some()
            || self.document.ui.agent_settings.focus.is_some()
            || self.document.chat.focused
    }

    pub fn settings_focus_active(&self) -> bool { self.document.ui.agent_settings.focus.is_some() }

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
        if self.document.ui.agent_settings_open {
            use openpencil_shell_core::widgets::agent_settings_panel::AgentSettingsPanel;
            let panel = AgentSettingsPanel::for_document(&self.document);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            if panel_rect.origin.x <= x
                && x <= panel_rect.origin.x + panel_rect.size.x
                && panel_rect.origin.y <= y
                && y <= panel_rect.origin.y + panel_rect.size.y
            {
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                let next =
                    (self.document.ui.agent_settings.scroll_y - delta_y).clamp(0.0, max_scroll);
                self.document.ui.agent_settings.scroll_y = next;
                return true;
            }
        }
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        // Canvas-local coords keep the zoom anchor under the cursor.
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.document.viewport.zoom_at(cursor, delta_y);
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
        if self.document.ui.agent_settings_open {
            use openpencil_shell_core::widgets::agent_settings_panel::AgentSettingsPanel;
            let panel = AgentSettingsPanel::for_document(&self.document);
            let panel_rect = panel.rect(viewport_width, viewport_height);
            if panel_rect.origin.x <= x
                && x <= panel_rect.origin.x + panel_rect.size.x
                && panel_rect.origin.y <= y
                && y <= panel_rect.origin.y + panel_rect.size.y
            {
                let total = panel.content_total_height();
                let viewport_h_inner = panel_rect.size.y - 48.0;
                let max_scroll = (total - viewport_h_inner).max(0.0);
                let next = (self.document.ui.agent_settings.scroll_y - dy).clamp(0.0, max_scroll);
                self.document.ui.agent_settings.scroll_y = next;
                return true;
            }
        }
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.document.viewport.pan(dx, dy);
        true
    }

    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        if self.document.ui.agent_settings_open && self.update_agent_settings_hover(x, y) { return true; }
        if let Some(state) = self.document.ui.color_picker.clone() {
            if let Some(kind) = state.drag {
                use openpencil_shell_core::document::ColorPickerDrag;
                use openpencil_shell_core::widgets::color_picker::ColorPicker;
                let picker = ColorPicker::for_state(&self.document, state.clone());
                let panel = picker.rect(self.last_viewport_w, self.last_viewport_h);
                let point = Point2D::new(x, y);
                match kind {
                    ColorPickerDrag::SvBox => {
                        let (s, v) = picker.sv_at(panel, point);
                        let _ = self.document.color_picker_set_hsv(state.hue, s, v);
                    }
                    ColorPickerDrag::HueSlider => {
                        let h = picker.hue_at(panel, point);
                        let _ = self.document.color_picker_set_hsv(h, state.sat, state.val);
                    }
                }
                return true;
            }
        }
        // Pen rubber-band — track cursor doc coord for preview.
        if self.document.ui.pen_in_progress.is_some() {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc = self.document.viewport.to_document(canvas_local);
            self.document.ui.pen_cursor_doc = Some(doc);
            return true;
        }
        if let Some(state) = self.document.ui.layer_context_menu {
            use openpencil_shell_core::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.document, state);
            let new_hover = menu.hovered_row_at(Point2D::new(x, y)).map(|i| i as u8);
            if new_hover != state.hovered_row {
                self.document.ui.layer_context_menu =
                    Some(openpencil_shell_core::document::LayerContextMenuState {
                        hovered_row: new_hover,
                        ..state
                    });
                return true;
            }
        }
        if self.document.ui.file_menu_open {
            use openpencil_shell_core::widgets::file_menu::FileMenu;
            use openpencil_shell_core::widgets::top_bar::TopBar;
            let top_bar_rect = openpencil_shell_core::Rect {
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
            let menu = FileMenu::from_document(&self.document, now_secs);
            let panel = menu.rect_at(anchor);
            let new_hover = menu.hovered_at(panel, Point2D::new(x, y));
            if new_hover != self.document.ui.file_menu_hover {
                self.document.ui.file_menu_hover = new_hover;
                return true;
            }
        }
        if self.document.ui.locale_picker_open {
            use openpencil_shell_core::widgets::locale_picker::LocalePicker;
            let panel = self.locale_picker_rect(self.last_viewport_w);
            let picker = LocalePicker::for_document(&self.document);
            let new_hover = picker.hit_test(panel, Point2D::new(x, y));
            if new_hover != self.document.ui.locale_picker_hover {
                self.document.ui.locale_picker_hover = new_hover;
                return true;
            }
        }
        if self.document.ui.shape_picker_open {
            use openpencil_shell_core::widgets::shape_picker::ShapePicker;
            let panel = self.shape_picker_rect(self.last_viewport_w, self.last_viewport_h);
            let picker = ShapePicker::for_document(&self.document);
            let new_hover = picker.hit_test(panel, Point2D::new(x, y));
            if new_hover != self.document.ui.shape_picker_hover {
                self.document.ui.shape_picker_hover = new_hover;
                return true;
            }
        }
        if let Some(drag) = self.rotate_drag {
            let cursor_angle = (y - drag.center_screen_y).atan2(x - drag.center_screen_x);
            let new_rotation = drag.start_rotation + (cursor_angle - drag.start_cursor_angle);
            self.document.set_selected_rotation(new_rotation);
            return true;
        }
        if let Some(drag) = self.handle_drag {
            let zoom = self.document.viewport.zoom.max(0.0001);
            let dx = (x - drag.start_screen_x) / zoom;
            let dy = (y - drag.start_screen_y) / zoom;
            let new_bounds = resize_bounds(drag.start_bounds, drag.handle, dx, dy);
            self.document.set_selected_bounds(new_bounds);
            return true;
        }
        if let Some(drag) = self.create_drag {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let cur = self.document.viewport.to_document(canvas_local);
            let min_x = drag.start_doc_x.min(cur.x);
            let min_y = drag.start_doc_y.min(cur.y);
            // Text needs room for the placeholder glyphs; shape
            // tools start at 1 px so the drag immediately sizes
            // the node to the cursor.
            let (min_w, min_h) = match self.document.tool {
                openpencil_shell_core::document::Tool::Text => (96.0_f32, 24.0_f32),
                _ => (1.0_f32, 1.0_f32),
            };
            let w = (drag.start_doc_x - cur.x).abs().max(min_w);
            let h = (drag.start_doc_y - cur.y).abs().max(min_h);
            let new_bounds = Rect::xywh(min_x, min_y, w, h);
            self.document.set_selected_bounds(new_bounds);
            return true;
        }
        if let Some(drag) = self.node_drag.as_mut() {
            let zoom = self.document.viewport.zoom.max(0.0001);
            let dx = (x - drag.last_screen_x) / zoom;
            let dy = (y - drag.last_screen_y) / zoom;
            drag.last_screen_x = x;
            drag.last_screen_y = y;
            if dx != 0.0 || dy != 0.0 {
                self.document.translate_selected(dx, dy);
                return true;
            }
            return false;
        }
        // Path-anchor drag — snap to cursor; flip `moved` only on real delta so release knows.
        if self.path_anchor_drag.is_some() {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc_point = self.document.viewport.to_document(canvas_local);
            let (id, idx, start) = {
                let d = self.path_anchor_drag.as_ref().unwrap();
                (d.node_id, d.anchor_index, d.start_doc)
            };
            if (doc_point.x - start.x).abs() > 0.001 || (doc_point.y - start.y).abs() > 0.001 {
                self.document.set_path_anchor_position(id, idx, doc_point);
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
        if let Some(d) = self.layer_drag.as_mut() {
            let source_id = d.source;
            let still_present = self
                .document
                .active_page()
                .map(|p| p.find(source_id).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return true;
            }
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
                    let new_w = (resize.start_width + dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.document.ui.layer_panel_width = new_w;
                }
                PanelResizeKind::PropertyLeft => {
                    let new_w = (resize.start_width - dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.document.ui.property_panel_width = new_w;
                }
            }
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
            self.document.viewport.pan(dx, dy);
            return true;
        }
        // Align toolbar hover sync — AFTER drag detection (codex: active drag must not be intercepted).
        let new_hover = if self.document.selection_count() >= 2 {
            self.align_toolbar_hit(x, y, self.last_viewport_w, self.last_viewport_h)
        } else {
            None
        };
        if new_hover != self.document.ui.align_toolbar_hover {
            self.document.ui.align_toolbar_hover = new_hover;
            return true;
        }
        false
    }

    /// Mouse-release — ends active drag; chat-panel snaps corner.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
        // Drop color-picker drag.
        if self.document.ui.color_picker.is_some() {
            self.document.color_picker_set_drag(None);
        }
        if self.document.ui.agent_settings_drag.is_some() {
            self.document.ui.agent_settings_drag = None;
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
            // refine the freshly-created shape — matches Figma's
            // post-create behaviour.
            self.document.tool = openpencil_shell_core::document::Tool::Select;
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
                self.document.history_push_past(drag.pre_drag_snapshot);
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
            // its actual center, not a phantom expanded center.
            let (panel_w, panel_h) = self.ai_chat_size();
            let center = Point2D::new(d.pos_x + panel_w / 2.0, d.pos_y + panel_h / 2.0);
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
            self.document.chat.anchor = ChatAnchor::nearest(center, cx0, cy0, cw, ch);
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
            self.document.tool = openpencil_shell_core::document::Tool::Select;
            return true;
        }
        if self.node_drag.take().is_some() {
            return true;
        }
        if self.marquee_drag.take().is_some() {
            // Can't compute the doc-space marquee rect without a
            // viewport; drop without committing. The viewport-
            // aware variant is the one runners should call.
            return true;
        }
        if self.layer_drag.take().is_some() {
            // Same story as marquee — no viewport, can't compute
            // drop target. Drop the candidate without committing.
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

    /// Typed-char router: rename → property → chat.
    pub fn apply_text(&mut self, c: char) -> bool {
        // Settings input owns the keyboard while focused — swallow every keystroke.
        if self.document.ui.agent_settings.focus.is_some() {
            if c.is_ascii_digit() && self.document.ui.settings_input_draft.len() < 5 {
                self.document.ui.settings_input_draft.push(c);
                return true;
            }
            return false;
        }
        if self.document.ui.layer_rename.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            let _ = self.document.rename_append(c.encode_utf8(&mut s));
            self.document.ui.rename_caret_anchor_ms = self.now_ms;
            return true;
        }
        if self.document.ui.text_editing.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            if self
                .document
                .text_edit_append(c.encode_utf8(&mut s), self.now_ms)
            {
                self.document.ui.text_edit_caret_anchor_ms = self.now_ms;
                return true;
            }
            return false;
        }
        if let Some(focus) = self.document.ui.property_focus {
            self.document.ui.property_draft_select_all = false;
            let is_hex_focus = matches!(focus, PropertyFocus::FillHex | PropertyFocus::StrokeHex);
            // Hex caps at 7 (`#RRGGBB`); numeric accepts digits, leading `-`, and one `.` for float focuses.
            let allowed = if is_hex_focus {
                self.document.ui.property_input_draft.len() < 7
                    && (c.is_ascii_hexdigit()
                        || (c == '#' && self.document.ui.property_input_draft.is_empty()))
            } else {
                c.is_ascii_digit()
                    || (c == '-' && self.document.ui.property_input_draft.is_empty())
                    || (c == '.'
                        && matches!(
                            focus,
                            PropertyFocus::Opacity
                                | PropertyFocus::Rotation
                                | PropertyFocus::PositionR
                                | PropertyFocus::StrokeWidth
                        )
                        && !self.document.ui.property_input_draft.contains('.'))
            };
            if !allowed {
                return false;
            }
            self.document.ui.property_input_draft.push(c);
            self.document.ui.property_caret_anchor_ms = self.now_ms;
            return true;
        }
        if !self.document.chat.focused {
            return false;
        }
        self.document.chat.input.push(c);
        self.document.chat.caret_anchor_ms = self.now_ms;
        true
    }

    pub fn apply_backspace(&mut self) -> bool {
        if self.document.ui.agent_settings.focus.is_some() { self.document.ui.settings_input_draft.pop(); return true; }
        if self.document.ui.layer_rename.is_some() {
            let ok = self.document.rename_backspace();
            if ok {
                self.document.ui.rename_caret_anchor_ms = self.now_ms;
            }
            return ok;
        }
        if self.document.ui.text_editing.is_some() {
            let ok = self.document.text_edit_backspace(self.now_ms);
            if ok {
                self.document.ui.text_edit_caret_anchor_ms = self.now_ms;
            }
            return ok;
        }
        if self.document.ui.property_focus.is_some() {
            self.document.ui.property_draft_select_all = false;
            if self.document.ui.property_input_draft.pop().is_some() {
                self.document.ui.property_caret_anchor_ms = self.now_ms;
                return true;
            }
            return false;
        }
        if self.document.chat.focused {
            if self.document.chat.input.pop().is_some() {
                self.document.chat.caret_anchor_ms = self.now_ms;
                return true;
            }
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        let snap = self.document.snapshot_for_history();
        if self.document.delete_selected() {
            self.document.history_push_past(snap);
            return true;
        }
        false
    }

    /// Delete — pops a char from rename / text-edit when active;
    /// otherwise deletes the selected node.
    pub fn apply_delete(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() {
            let ok = self.document.rename_backspace();
            if ok {
                self.document.ui.rename_caret_anchor_ms = self.now_ms;
            }
            return ok;
        }
        if self.document.ui.text_editing.is_some() {
            let ok = self.document.text_edit_backspace(self.now_ms);
            if ok {
                self.document.ui.text_edit_caret_anchor_ms = self.now_ms;
            }
            return ok;
        }
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        let snap = self.document.snapshot_for_history();
        if self.document.delete_selected() {
            self.document.history_push_past(snap);
            return true;
        }
        false
    }

    /// Cmd-D — duplicate selection as a sibling at +10 doc px.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document
            .duplicate_selected(&mut self.next_node_id, 10.0)
            .is_some()
    }

    /// Arrow-key nudge — translate selection by (dx, dy) doc px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.input_active() {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document.translate_selected(dx, dy);
        true
    }

    /// Layer drag release → reorder_before/after/into.
    pub(in crate::widget_host) fn commit_layer_drag(
        &mut self,
        d: super::LayerDragState,
        viewport_h: f32,
    ) -> bool {
        if !d.active {
            // Never moved past threshold — selection on press is the
            // only effect, nothing more to do.
            return false;
        }
        if self
            .document
            .active_page()
            .map(|p| p.find(d.source).is_none())
            .unwrap_or(true)
        {
            return false;
        }
        use openpencil_shell_core::widgets::{DropPosition, LayerPanel, TOP_BAR_HEIGHT};
        let layer_rect = openpencil_shell_core::Rect {
            origin: openpencil_shell_core::Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: openpencil_shell_core::Point2D::new(
                self.document.ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        // Build with source excluded so indicator y matches post-commit.
        let panel = LayerPanel::from_document_with_drag_source(&self.document, d.source);
        let cursor = openpencil_shell_core::Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            return true; // self-drop no-op
        }
        match drop.position {
            DropPosition::Before => {
                self.document.reorder_before(d.source, drop.anchor);
            }
            DropPosition::After => {
                self.document.reorder_after(d.source, drop.anchor);
            }
            DropPosition::Into => {
                self.document.reorder_into(d.source, drop.anchor);
            }
        }
        true
    }

    pub(in crate::widget_host) fn commit_marquee_selection(
        &mut self,
        m: super::MarqueeDragState,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        // 2 screen-px marquee threshold (TS `useMarqueeStart`).
        let screen_dx = (m.current_screen_x - m.start_screen_x).abs();
        let screen_dy = (m.current_screen_y - m.start_screen_y).abs();
        if screen_dx < 2.0 && screen_dy < 2.0 {
            return;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let to_doc = |sx: f32, sy: f32| -> Point2D {
            let local = Point2D::new(sx - cx0, sy - cy0);
            self.document.viewport.to_document(local)
        };
        let p0 = to_doc(m.start_screen_x, m.start_screen_y);
        let p1 = to_doc(m.current_screen_x, m.current_screen_y);
        let x = p0.x.min(p1.x);
        let y = p0.y.min(p1.y);
        let w = (p1.x - p0.x).abs();
        let h = (p1.y - p0.y).abs();
        let rect = Rect::xywh(x, y, w, h);
        let ids = self.document.nodes_intersecting_doc_rect(rect);
        if m.additive {
            // ADD-only: every hit joins the set; already-selected
            // hits stay selected. TS parity:
            // `setSelection([...prior, ...newHits], anchor)` —
            // shift-marquee never removes. If you want to remove
            // a single node from the set use shift+click.
            for id in ids {
                if !self.document.is_selected(id) {
                    // toggle adds it (since it's not in the set).
                    self.document.toggle_selection(id);
                }
            }
        } else if !ids.is_empty() {
            // Replace with the hit set. Anchor = last hit
            // (matches TS `setSelection(ids, ids[last])`).
            let anchor = *ids.last().unwrap();
            self.document.selected_set = ids;
            self.document.selected = anchor;
        }
        // Empty marquee on plain press already cleared at start;
        // nothing else to do.
    }

    pub fn apply_send(&mut self) -> bool {
        if self.document.ui.agent_settings.focus.is_some() { self.commit_settings_focus_if_any(); return true; }
        if self.document.ui.layer_rename.is_some() {
            return self.document.rename_commit();
        }
        if self.document.ui.text_editing.is_some() {
            return self.document.text_edit_commit();
        }
        if self.document.ui.pen_in_progress.is_some() {
            return self.document.finish_pen_path();
        }
        if self.document.ui.property_focus.is_some() { self.commit_property_focus_if_any(); return true; }
        if self.document.chat.input.trim().is_empty() { return false; }
        self.document.chat.send();
        true
    }

    /// Escape — priority cascade: rename → property → pickers →
    /// chat → selection. One layer per press.
    pub fn apply_escape(&mut self) -> bool {
        // Settings input focus: clear draft first, second press closes modal.
        if self.document.ui.agent_settings.focus.take().is_some() { self.document.ui.settings_input_draft.clear(); return true; }
        if self.document.ui.agent_settings_open { self.document.ui.agent_settings_open = false; self.document.ui.agent_settings_drag = None; return true; }
        if self.document.ui.export_dialog_open { self.document.ui.export_dialog_open = false; return true; }
        if self.document.ui.figma_import_open { self.document.ui.figma_import_open = false; return true; }
        if self.document.ui.file_menu_open { self.document.ui.file_menu_open = false; self.document.ui.file_menu_hover = None; return true; }
        if self.document.rename_cancel() { return true; }
        if self.document.text_edit_commit() { return true; }
        if self.document.finish_pen_path() { return true; }
        if self.document.ui.property_focus.take().is_some() {
            self.document.ui.property_input_draft.clear();
            self.document.ui.property_draft_select_all = false;
            return true;
        }
        if self.document.ui.locale_picker_open {
            self.document.ui.locale_picker_open = false; self.document.ui.locale_picker_hover = None;
            return true;
        }
        if self.document.ui.shape_picker_open {
            self.document.ui.shape_picker_open = false; self.document.ui.shape_picker_hover = None;
            return true;
        }
        if self.document.ui.fill_type_picker_open {
            self.document.ui.fill_type_picker_open = false;
            return true;
        }
        if self.document.chat.focused {
            self.document.chat.focused = false;
            return true;
        }
        if self.document.selected.is_real() {
            self.document.deselect_all();
            return true;
        }
        false
    }

    /// Primary-button click — routes to AI chat / Toolbar / Layer.
    pub fn apply_click(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // AI chat panel sits above canvas — check first.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel = AIChatPlaceholder::from_document(&self.document);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                match hit {
                    AIChatHit::FocusInput => {
                        self.document.chat.focused = true;
                        self.document.chat.caret_anchor_ms = self.now_ms;
                        return true;
                    }
                    AIChatHit::Send => {
                        self.document.chat.send();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.document.chat.input = text;
                        self.document.chat.focused = true;
                        self.document.chat.caret_anchor_ms = self.now_ms;
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        // Drag handle is handled in apply_press
                        // ahead of this; reaching here is a path
                        // bypass — ignore.
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        self.document.chat.collapsed = !self.document.chat.collapsed;
                        return true;
                    }
                }
            }
        }
        // Click outside chat panel — defocus the input.
        let was_focused = self.document.chat.focused;
        self.document.chat.focused = false;
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_document(&self.document);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            match hit {
                openpencil_shell_core::widgets::ToolbarHit::Tool(tool) => {
                    self.document.tool = tool;
                    return true;
                }
                openpencil_shell_core::widgets::ToolbarHit::Action(_) => return false,
                openpencil_shell_core::widgets::ToolbarHit::ToggleShapePicker => {
                    self.document.ui.shape_picker_open = !self.document.ui.shape_picker_open;
                    return true;
                }
            }
        }
        // Panel hits only when sidebar is open.
        if !self.document.ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.document.ui.layer_panel_width,
                (viewport_height - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let panel = LayerPanel::from_document(&self.document);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            use openpencil_shell_core::document::LayerContextTarget;
            use openpencil_shell_core::widgets::LayerPanelHit as H;
            let target_for_dbl = match hit {
                H::Layer(id) => Some(LayerContextTarget::Layer(id)),
                H::Page(idx) => Some(LayerContextTarget::Page(idx)),
                _ => None,
            };
            if let Some(target) = target_for_dbl {
                if let Some((prev, prev_ms)) = self.document.ui.last_layer_click {
                    if prev == target && self.now_ms.saturating_sub(prev_ms) < 400 {
                        let started = match target {
                            LayerContextTarget::Layer(id) => self.document.start_rename_layer(id),
                            LayerContextTarget::Page(idx) => self.document.start_rename_page(idx),
                        };
                        if started {
                            self.document.ui.rename_caret_anchor_ms = self.now_ms;
                        }
                        self.document.ui.last_layer_click = None;
                        return true;
                    }
                }
                self.document.ui.last_layer_click = Some((target, self.now_ms));
            }
            match hit {
                H::Page(idx) => {
                    self.document.active_page_index = idx;
                    self.document.clear_selection();
                    return true;
                }
                H::Layer(node_id) => {
                    if self.shift_held {
                        self.document.toggle_selection(node_id);
                    } else {
                        self.document.set_single_selection(node_id);
                    }
                    return true;
                }
                H::ToggleHidden(node_id) => {
                    self.document.toggle_node_hidden(node_id);
                    return true;
                }
                H::ToggleLocked(node_id) => {
                    self.document.toggle_node_locked(node_id);
                    return true;
                }
                H::ToggleCollapsed(node_id) => {
                    self.document.toggle_node_collapsed(node_id);
                    return true;
                }
                H::AddPage => {
                    let _ = self.document.add_page();
                    return true;
                }
                H::DeletePage(idx) => {
                    let _ = self.document.remove_page(idx);
                    return true;
                }
            }
        }
        // Click hit no chrome — repaint if focus changed.
        was_focused
    }
}
