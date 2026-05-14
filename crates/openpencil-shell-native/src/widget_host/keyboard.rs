//! Keyboard + click handlers on `WidgetHostNative`. Split out of
//! `input.rs` to honor the 800-line cap (Task #49). Pointer + wheel
//! + cursor-move + release stay in `input.rs`; this file owns text
//! input + delete + duplicate + nudge + send + escape + click
//! routing.

use super::helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use openpencil_shell_core::document::{PropertyFocus, VariableRowFocus};
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayoutCx, Toolbar, Widget, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

impl WidgetHostNative {
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
        if let Some(focus) = self.document.ui.variable_row_focus {
            self.document.ui.property_draft_select_all = false;
            let allowed = match focus {
                VariableRowFocus::Number(_) => {
                    c.is_ascii_digit()
                        || (c == '-' && self.document.ui.property_input_draft.is_empty())
                        || (c == '.' && !self.document.ui.property_input_draft.contains('.'))
                }
                VariableRowFocus::String(_) => !c.is_control(),
            };
            if !allowed {
                return false;
            }
            self.document.ui.property_input_draft.push(c);
            self.document.ui.property_caret_anchor_ms = self.now_ms;
            return true;
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
        if self.document.ui.variable_row_focus.is_some() {
            self.document.ui.property_draft_select_all = false;
            if self.document.ui.property_input_draft.pop().is_some() {
                self.document.ui.property_caret_anchor_ms = self.now_ms;
                return true;
            }
            return false;
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
        // Variable-row inline editor: Delete should clear a
        // dirty draft via the same path as apply_backspace
        // (pop one char). Once empty, fall through.
        if self.document.ui.variable_row_focus.is_some() {
            self.document.ui.property_draft_select_all = false;
            if self.document.ui.property_input_draft.pop().is_some() {
                self.document.ui.property_caret_anchor_ms = self.now_ms;
                return true;
            }
            return false;
        }
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
        if self.document.ui.variable_row_focus.is_some() { self.commit_variable_row_focus_if_any(); return true; }
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
        if self.document.ui.variable_row_focus.take().is_some() {
            self.document.ui.property_input_draft.clear();
            self.document.ui.property_draft_select_all = false;
            return true;
        }
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
