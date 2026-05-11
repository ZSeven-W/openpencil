//! Non-press input handlers on `WidgetHostNative`: wheel, pan
//! gesture, cursor-move (drives every active drag), mouse release
//! (with + without viewport), keyboard text / backspace / send /
//! escape, property-panel action dispatch, click (AI chat panel +
//! Toolbar + LayerPanel rows + chat-defocus).
//!
//! Pulled out of `widget_host.rs` to keep the spine file under the
//! 800-line ceiling. `apply_press` lives in `press.rs`.

use super::helpers::{
    parse_hex_color, resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH, TOOLBAR_INSET_X,
    TOOLBAR_INSET_Y,
};
use super::{PanelResizeKind, WidgetHostNative};
use openpencil_shell_core::document::{ChatAnchor, PropertyFocus, ReorderDirection};
use openpencil_shell_core::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayoutCx, Toolbar, Widget, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use openpencil_shell_core::{Point2D, Rect};

impl WidgetHostNative {
    /// Apply a wheel event — zoom centered at `(x, y)` when over
    /// the canvas. Returns true if a redraw is needed.
    pub fn apply_wheel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        // Cursor in canvas-local coords — use canvas_region's
        // dynamic left edge so cursor-centered zoom stays anchored
        // when the sidebar is collapsed.
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let cursor = Point2D::new(x - cx0, y - cy0);
        self.document.viewport.zoom_at(cursor, delta_y);
        true
    }

    /// Apply a 2-finger trackpad pan gesture — translate the
    /// canvas viewport by `(dx, dy)` directly. Step 5 makes
    /// trackpad swipes feel native (Figma convention: 2-finger
    /// swipe pans, pinch / Cmd+swipe / mouse-wheel zoom). Returns
    /// true if a redraw is needed.
    pub fn apply_pan_gesture(
        &mut self,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return false;
        }
        if dx == 0.0 && dy == 0.0 {
            return false;
        }
        self.document.viewport.pan(dx, dy);
        true
    }

    /// Cursor-move handler. Drives canvas pan-drag, chat-panel
    /// drag, or no-op. Returns whether the host should repaint.
    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
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
            let w = (drag.start_doc_x - cur.x).abs().max(1.0);
            let h = (drag.start_doc_y - cur.y).abs().max(1.0);
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
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return true;
        }
        if let Some(d) = self.layer_drag.as_mut() {
            // Drop the gesture if the source has disappeared from the
            // active page (e.g., user deleted it via Cmd-X or
            // switched pages mid-drag). Avoids stale drop-indicator
            // paint that invites a no-op release.
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
            // Activation is VERTICAL-ONLY by design: layer drag-to-
            // reorder operates on a flat row stack, so the meaningful
            // axis is y. Pure horizontal wiggle on a row mustn't
            // activate (would steal click-feel from selection +
            // eye/lock-toggle gestures). 4 px screen-space matches
            // the marquee's small-motion threshold.
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
            true
        } else {
            false
        }
    }

    /// Mouse-release handler. Ends the active drag (if any). For
    /// chat-panel drag, snaps the panel to the nearest canvas
    /// corner via `ChatAnchor::nearest`. Returns true if anything
    /// visible changed.
    pub fn apply_release_with_viewport(&mut self, viewport_w: f32, viewport_h: f32) -> bool {
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

    /// Mouse-release handler — viewport-less variant kept for
    /// backwards compatibility with existing call sites.
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
        // If a chat drag was in flight without a known viewport,
        // we can't snap; just drop it (best effort).
        if self.chat_drag.take().is_some() {
            return true;
        }
        let was_dragging = self.drag.is_some();
        self.drag = None;
        was_dragging
    }

    /// Push a typed character into the focused chat input.
    /// Returns true if anything changed.
    pub fn apply_text(&mut self, c: char) -> bool {
        if let Some(focus) = self.document.ui.property_focus {
            self.document.ui.property_draft_select_all = false;
            let is_hex_focus = matches!(focus, PropertyFocus::FillHex | PropertyFocus::StrokeHex);
            let allowed = if is_hex_focus {
                // Hex inputs accept 0-9, a-f, A-F, and an optional
                // leading `#`. Length capped at 7 (`#RRGGBB`).
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
        // Reset blink so the caret is solid right after the
        // keystroke instead of mid-fade.
        self.document.chat.caret_anchor_ms = self.now_ms;
        true
    }

    /// Backspace — routes to whichever input is currently focused
    /// (property edit field, then chat). When no text is focused,
    /// Backspace deletes the selected canvas node (TS parity:
    /// `use-edit-shortcuts.ts` treats Delete and Backspace
    /// identically when the focus target is not an `<input>` /
    /// `<textarea>`).
    pub fn apply_backspace(&mut self) -> bool {
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
        self.document.delete_selected()
    }

    /// Delete key — same selected-node delete as
    /// `apply_backspace` when no text is focused, but never
    /// touches text drafts. Use this when the host wants Delete
    /// to be strictly destructive regardless of focus.
    pub fn apply_delete(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        self.document.delete_selected()
    }

    /// Cmd/Ctrl+D — duplicate the selected node as a sibling
    /// offset by ~10 doc px. Selection follows the clone.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        self.document
            .duplicate_selected(&mut self.next_node_id, 10.0)
            .is_some()
    }

    /// Arrow-key nudge — translate the selected node by
    /// `(dx, dy)` document px. Shift-arrow callers pass 10 px;
    /// plain arrows pass 1 px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.translate_selected(dx, dy);
        true
    }

    /// Convert a marquee drag (in screen-space) into a doc-space
    /// rect, ask the document which top-level nodes overlap it,
    /// and either replace or extend the selection.
    /// Resolve a layer drag-to-reorder gesture on release. Returns
    /// `true` if anything changed (caller repaints). Drops the
    /// drag silently if it never activated (treated as a click
    /// that already selected the row on press) or if the cursor
    /// isn't over a layer row at release time.
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
        // Defensive source-validity check — symmetric with the
        // cursor_move and paint guards. `reorder_before/after`
        // already silently no-ops on a missing source, but bailing
        // here keeps the rest of this method (drop_target_at +
        // dispatch) from running pointlessly on a dead drag.
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
        let panel = LayerPanel::from_document(&self.document);
        let cursor = openpencil_shell_core::Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            // Self-drop is a no-op.
            return true;
        }
        match drop.position {
            DropPosition::Before => {
                self.document.reorder_before(d.source, drop.anchor);
            }
            DropPosition::After => {
                self.document.reorder_after(d.source, drop.anchor);
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
        // Near-zero marquee = a click without drag. Threshold is
        // measured in SCREEN pixels (codex CONCERN: doc-space
        // threshold became zoom-dependent — at 10% zoom a 4-px
        // drag registered as a real marquee, at 1000% zoom a
        // huge drag could fall below). 2 screen px matches the
        // TS `useMarqueeStart` threshold.
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

    /// Cmd/Ctrl+C — copy the current selection into the
    /// clipboard. No-op when nothing is selected. Doesn't touch
    /// any UI focus state.
    pub fn apply_copy(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        self.document.copy_selected()
    }

    /// Cmd/Ctrl+X — copy then delete the selection.
    pub fn apply_cut(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        self.document.cut_selected()
    }

    /// Cmd/Ctrl+V — paste the clipboard at the active page,
    /// offset by 10 doc px from the original positions. Selection
    /// follows the new clones.
    pub fn apply_paste(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        !self
            .document
            .paste_clipboard(&mut self.next_node_id, 10.0)
            .is_empty()
    }

    /// Cmd/Ctrl+A — replace the selection with every top-level
    /// node on the active page. TS parity:
    /// `useCanvasStore.setSelection(topLevelIds, topLevelIds[0])`.
    pub fn apply_select_all(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        self.document.select_all_top_level()
    }

    /// `[` / `]` — bump the selected node down / up by one
    /// position in its parent's children vec (changing paint
    /// order).
    pub fn apply_reorder(&mut self, direction: ReorderDirection) -> bool {
        if self.document.ui.property_focus.is_some() || self.document.chat.focused {
            return false;
        }
        self.document.reorder_selected(direction)
    }

    /// Enter — commits the focused property edit (parses the draft
    /// as f32, writes to the selected node, clears focus) or
    /// sends the focused chat input.
    pub fn apply_send(&mut self) -> bool {
        if self.document.ui.property_focus.is_some() {
            self.commit_property_focus_if_any();
            return true;
        }
        if self.document.chat.input.trim().is_empty() {
            return false;
        }
        self.document.chat.send();
        true
    }

    /// Escape — handles one layer per press, in priority order
    /// (TS parity):
    ///   1. Active property-panel input → discard draft + clear focus
    ///   2. Locale picker open → close it
    ///   3. Shape picker open → close it
    ///   4. Fill-type picker open → close it
    ///   5. Chat input focused → defocus
    ///   6. Canvas selection → clear it
    pub fn apply_escape(&mut self) -> bool {
        if self.document.ui.property_focus.take().is_some() {
            self.document.ui.property_input_draft.clear();
            self.document.ui.property_draft_select_all = false;
            return true;
        }
        if self.document.ui.locale_picker_open {
            self.document.ui.locale_picker_open = false;
            return true;
        }
        if self.document.ui.shape_picker_open {
            self.document.ui.shape_picker_open = false;
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

    /// Dispatch a property-panel button / checkbox press.
    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: openpencil_shell_core::widgets::PropertyPanelAction,
    ) {
        use openpencil_shell_core::widgets::PropertyPanelAction as A;
        match action {
            A::SetFlexLayout(mode) => self.document.ui.flex_layout = mode,
            A::ToggleSizeFillWidth => {
                self.document.ui.size_fill_width = !self.document.ui.size_fill_width;
            }
            A::ToggleSizeFillHeight => {
                self.document.ui.size_fill_height = !self.document.ui.size_fill_height;
            }
            A::ToggleSizeHugWidth => {
                self.document.ui.size_hug_width = !self.document.ui.size_hug_width;
            }
            A::ToggleSizeHugHeight => {
                self.document.ui.size_hug_height = !self.document.ui.size_hug_height;
            }
            A::ToggleSizeClipContent => {
                self.document.ui.size_clip_content = !self.document.ui.size_clip_content;
            }
            A::ToggleFillTypePicker => {
                self.document.ui.fill_type_picker_open = !self.document.ui.fill_type_picker_open;
            }
            A::SetFillType(t) => {
                // Per-node now (was `doc.ui.fill_type` until
                // 2026-05-11). Editable-gated by the mutator so
                // locked / hidden selections silently no-op.
                self.document.set_selected_fill_type(t);
                self.document.ui.fill_type_picker_open = false;
            }
        }
    }

    /// Parse `property_input_draft` as f32 and apply it to the
    /// selected node via `Document::commit_property_edit`. Always
    /// clears focus + draft. No-op when nothing is focused.
    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        let Some(focus) = self.document.ui.property_focus.take() else {
            return;
        };
        self.document.ui.property_draft_select_all = false;
        let draft = std::mem::take(&mut self.document.ui.property_input_draft);
        match focus {
            PropertyFocus::FillHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self.document.set_selected_color(true, color);
                    }
                }
            }
            PropertyFocus::StrokeHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self.document.set_selected_color(false, color);
                    }
                }
            }
            _ => {
                if let Ok(value) = draft.trim().parse::<f32>() {
                    let _ = self.document.commit_property_edit(focus, value);
                }
            }
        }
    }

    /// Apply a primary-button mouse click — routes to Toolbar /
    /// LayerPanel / AI chat hit-test. Returns whether anything was
    /// consumed (caller should request a redraw if so).
    pub fn apply_click(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        // AI chat panel sits ABOVE the canvas — check it first so
        // clicks on the floating panel don't fall through to the
        // canvas / Hand-tool drag.
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
        // LayerPanel hits only land when the sidebar is open —
        // when collapsed the panel isn't painted (codex stop-hook
        // fix: native collapsed-sidebar input was still resolving
        // canvas clicks to the LayerPanel rect underneath).
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
            match hit {
                openpencil_shell_core::widgets::LayerPanelHit::Page(idx) => {
                    self.document.active_page_index = idx;
                    self.document.clear_selection();
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::Layer(node_id) => {
                    // Shift+click on a layer row toggles set
                    // membership (TS parity with the layer panel
                    // multi-select); plain click sets single.
                    if self.shift_held {
                        self.document.toggle_selection(node_id);
                    } else {
                        self.document.set_single_selection(node_id);
                    }
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleHidden(node_id) => {
                    self.document.toggle_node_hidden(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleLocked(node_id) => {
                    self.document.toggle_node_locked(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::ToggleCollapsed(node_id) => {
                    self.document.toggle_node_collapsed(node_id);
                    return true;
                }
                openpencil_shell_core::widgets::LayerPanelHit::AddPage => {
                    let _ = self.document.add_page();
                    return true;
                }
            }
        }
        // Click hit no chrome — return true if the prior focus
        // state changed so the chrome repaints to drop the caret.
        was_focused
    }
}
