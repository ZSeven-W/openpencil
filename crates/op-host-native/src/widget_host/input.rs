//! Non-press input handlers on `WidgetHostNative`. press → press.rs.
//!
//! `EditorState` is the host's source of truth. Scalar / chrome
//! reads go straight to `editor_state`; node-tree hit-tests run
//! against the layout-resolved `LayoutScene`, refreshed at the top
//! of each handler. Every mutation flags `editor_state` so the next
//! refresh re-derives.

use super::helpers::{resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH};
use super::{PanelResizeKind, WidgetHostNative};
use op_editor_ui::{Point2D, Rect};

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

    /// After a node-drag translate, compute smart-guide alignment
    /// against the other top-level nodes, snap the selection onto the
    /// nearest edge/centre alignment, and store the guide lines for
    /// the canvas painter. Cleared on drag release.
    fn apply_smart_guides(&mut self) {
        use op_editor_core::align_guides::compute_alignment_guides;
        /// Snap range in doc-px — an edge/centre this close to another
        /// node's edge/centre locks on.
        const GUIDE_THRESHOLD: f64 = 6.0;

        self.refresh_layout_scene();
        let selected = self.editor_state.selection.anchor.as_str().to_string();
        // Collect AABBs off the layout scene, then drop the borrow so
        // the snap translate can mutate `editor_state`.
        let (moving, others): (Option<[f64; 4]>, Vec<[f64; 4]>) =
            match self.layout_scene.active_page() {
                Some(page) => {
                    let mut moving = None;
                    let mut others = Vec::new();
                    for n in &page.children {
                        let b = n.bounds;
                        let aabb = [
                            b.origin.x as f64,
                            b.origin.y as f64,
                            b.size.x as f64,
                            b.size.y as f64,
                        ];
                        if n.id == selected {
                            moving = Some(aabb);
                        } else {
                            others.push(aabb);
                        }
                    }
                    (moving, others)
                }
                None => (None, Vec::new()),
            };
        let Some(m) = moving else {
            self.editor_state.editor_ui.active_guides.clear();
            return;
        };
        let others: Vec<(f64, f64, f64, f64)> =
            others.iter().map(|a| (a[0], a[1], a[2], a[3])).collect();
        let result = compute_alignment_guides((m[0], m[1], m[2], m[3]), &others, GUIDE_THRESHOLD);
        if result.snap_dx != 0.0 || result.snap_dy != 0.0 {
            self.editor_state
                .translate_selected(result.snap_dx, result.snap_dy);
        }
        self.editor_state.editor_ui.active_guides = result.guides;
    }

    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // Every hit-test below (color picker, layer context menu, align
        // toolbar, panel resize, …) reasons about the layout-resolved
        // render scene. Refresh it once up front so a mutation since the
        // last paint can't leave any of them hit-testing stale geometry.
        self.refresh_layout_scene();
        if self.editor_state.editor_ui.agent_settings_open && self.update_agent_settings_hover(x, y)
        {
            return true;
        }
        if let Some(state) = self.editor_state.ui.color_picker.clone() {
            if let Some(kind) = state.drag {
                use op_editor_core::ui_draft::ColorPickerDrag;
                use op_editor_ui::widgets::color_picker::ColorPicker;
                let picker = ColorPicker::for_state(&self.editor_state, state.clone());
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
            use op_editor_ui::widgets::layer_context_menu::LayerContextMenu;
            let menu = LayerContextMenu::for_state(&self.editor_state, state.clone());
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
            use op_editor_ui::widgets::file_menu::FileMenu;
            use op_editor_ui::widgets::top_bar::TopBar;
            self.refresh_layout_scene();
            let top_bar_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, op_editor_ui::widgets::TOP_BAR_HEIGHT),
            };
            let anchor = TopBar::file_menu_rect(top_bar_rect);
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, now_secs);
            let panel = menu.rect_at(anchor);
            let new_hover = menu.hovered_at(panel, Point2D::new(x, y));
            // shell-core `FileMenuChoice` option → op-editor-core.
            let new_hover_ec =
                new_hover.map(op_editor_ui::widgets::editor_state_ext::file_menu_choice);
            if new_hover_ec != self.editor_state.editor_ui.file_menu_hover {
                self.editor_state.editor_ui.file_menu_hover = new_hover_ec;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.locale_picker_open {
            use op_editor_ui::widgets::locale_picker::LocalePicker;
            self.refresh_layout_scene();
            let panel = self.locale_picker_rect(self.last_viewport_w);
            let picker = LocalePicker::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = picker.hit_test(panel, Point2D::new(x, y));
            let new_hover_ec = new_hover;
            if new_hover_ec != self.editor_state.editor_ui.locale_picker_hover {
                self.editor_state.editor_ui.locale_picker_hover = new_hover_ec;
                self.mark_dirty();
                return true;
            }
        }
        if self.editor_state.editor_ui.shape_picker_open {
            use op_editor_ui::widgets::shape_picker::ShapePicker;
            self.refresh_layout_scene();
            let panel = self.shape_picker_rect(self.last_viewport_w, self.last_viewport_h);
            let picker = ShapePicker::for_editor_ui(&self.editor_state.editor_ui);
            let new_hover = picker.hit_test(panel, Point2D::new(x, y));
            let new_hover_ec = new_hover.map(op_editor_ui::widgets::editor_state_ext::shape_choice);
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
            self.editor_state
                .set_selected_bounds(rect_to_doc_rect(new_bounds));
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
            self.editor_state
                .set_selected_bounds(rect_to_doc_rect(new_bounds));
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
                // `drag`'s last use was above — `self` is free to
                // re-borrow for the smart-guide alignment pass.
                self.apply_smart_guides();
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Path-anchor / handle drag — write the current cursor
        // position (codex BLOCK: drag-back-to-start was being
        // silently dropped, so always write).
        if self.path_anchor_drag.is_some() {
            use super::AnchorDragTarget;
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc = self.editor_state.viewport.to_document(canvas_local);
            let (id, idx, target, anchor_doc, start, shift, already_moved) = {
                let d = self.path_anchor_drag.as_ref().unwrap();
                (
                    d.node_id.clone(),
                    d.anchor_index,
                    d.target,
                    d.anchor_doc,
                    d.start_doc,
                    d.shift,
                    d.moved,
                )
            };
            // `is_move` uses the raw (rotation-independent) cursor —
            // motion detection is frame-agnostic. The drag mutates
            // nothing until the cursor first travels, so a
            // press-release leaves the document (and undo stack)
            // untouched; once it HAS moved, every event (incl. a
            // drag back to the start point) keeps writing.
            let is_move = (doc.x - start.x).abs() > 0.001 || (doc.y - start.y).abs() > 0.001;
            if is_move || already_moved {
                // Un-rotate the cursor into the path's local frame so
                // anchor / handle coords are written rotation-free.
                let local = match self
                    .layout_scene
                    .active_page()
                    .and_then(|p| p.find(id.as_str()))
                    .filter(|n| n.rotation.abs() > f32::EPSILON)
                    .map(|n| (n.rotation, n.aggregate_bounds()))
                {
                    Some((rot, b)) => {
                        let c =
                            Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
                        op_editor_ui::widgets::rotate_point(doc, c, -rot)
                    }
                    None => doc,
                };
                match target {
                    AnchorDragTarget::Anchor => {
                        self.editor_state.set_path_anchor_position(
                            id,
                            idx,
                            (local.x as f64, local.y as f64),
                        );
                    }
                    AnchorDragTarget::Handle(side) => {
                        // First real move sets the anchor's point type
                        // — Shift = independent (broken) handles, else
                        // mirrored (smooth).
                        if !self
                            .path_anchor_drag
                            .as_ref()
                            .map(|d| d.moved)
                            .unwrap_or(true)
                        {
                            let pt = if shift {
                                jian_ops_schema::node::PenPathPointType::Independent
                            } else {
                                jian_ops_schema::node::PenPathPointType::Mirrored
                            };
                            self.editor_state
                                .set_path_anchor_point_type(id.clone(), idx, pt);
                        }
                        let delta = (
                            (local.x - anchor_doc.x) as f64,
                            (local.y - anchor_doc.y) as f64,
                        );
                        self.editor_state
                            .set_path_anchor_handle(id, idx, side, Some(delta));
                    }
                }
                self.mark_dirty();
                if let Some(d) = self.path_anchor_drag.as_mut() {
                    d.moved = true;
                }
            }
            return true;
        }
        // Ellipse arc-handle drag — recompute the arc geometry from
        // the cursor and re-apply `SetEllipseArc` each move.
        if self.arc_handle_drag.is_some() {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc = self.editor_state.viewport.to_document(canvas_local);
            let (id, handle, start, already_moved) = {
                let d = self.arc_handle_drag.as_ref().unwrap();
                (d.node_id.clone(), d.handle, d.start_doc, d.moved)
            };
            // Mutate nothing until the cursor first travels — a
            // press-release must not write the arc or push an undo
            // entry. Once moved, keep writing every event.
            let is_move = (doc.x - start.x).abs() > 0.001 || (doc.y - start.y).abs() > 0.001;
            if is_move || already_moved {
                if let Some(cmd) = self.arc_drag_command(&id, handle, doc) {
                    if self.editor_state.apply(cmd) {
                        self.mark_dirty();
                        if let Some(d) = self.arc_handle_drag.as_mut() {
                            d.moved = true;
                        }
                    }
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
            self.refresh_layout_scene();
            let source_id = self.layer_drag.as_ref().unwrap().source.clone();
            let still_present = self
                .layout_scene
                .active_page()
                .map(|p| p.find(source_id.as_str()).is_some())
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
                    let new_w = (resize.start_width + dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
                    self.editor_state.editor_ui.layer_panel_width = new_w;
                }
                PanelResizeKind::PropertyLeft => {
                    let new_w = (resize.start_width - dx).clamp(PANEL_MIN_WIDTH, PANEL_MAX_WIDTH);
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
        let new_hover_ec = new_hover;
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
            // Drag ended — drop the transient smart-guide lines.
            self.editor_state.editor_ui.active_guides.clear();
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
        if let Some(drag) = self.arc_handle_drag.take() {
            // Commit history only when the arc actually changed.
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
            // Drag ended — drop the transient smart-guide lines.
            self.editor_state.editor_ui.active_guides.clear();
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

    /// Build the `SetEllipseArc` command for an in-progress arc-handle
    /// drag — converts the cursor doc point into start / sweep / inner
    /// geometry for the dragged handle. `None` for a missing or
    /// zero-size ellipse.
    fn arc_drag_command(
        &self,
        id: &op_editor_core::NodeId,
        handle: op_editor_ui::widgets::ArcHandle,
        doc: Point2D,
    ) -> Option<op_editor_core::EditorCommand> {
        use op_editor_core::EditorCommand;
        use op_editor_ui::widgets::ArcHandle;
        let node = self.layout_scene.active_page()?.find(id.as_str())?;
        let b = node.bounds;
        if b.size.x <= 0.0 || b.size.y <= 0.0 {
            return None;
        }
        let centre = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
        // Un-rotate the cursor into the ellipse's local frame.
        let doc = if node.rotation.abs() > f32::EPSILON {
            op_editor_ui::widgets::rotate_point(doc, centre, -node.rotation)
        } else {
            doc
        };
        // Cursor offset from the ellipse centre, normalised by the
        // radii so the angle is the same convention the painter uses.
        let nx = (doc.x - centre.x) / (b.size.x / 2.0);
        let ny = (doc.y - centre.y) / (b.size.y / 2.0);
        let old_start = node.arc_start_angle.unwrap_or(0.0);
        let old_sweep = node.arc_sweep_angle.unwrap_or(360.0);
        Some(match handle {
            ArcHandle::Start => {
                // Dragging the start handle keeps the end fixed; the
                // sweep keeps the sign of the existing arc.
                let new_start = norm360(ny.atan2(nx).to_degrees());
                let new_sweep = signed_sweep(old_start + old_sweep - new_start, old_sweep);
                EditorCommand::SetEllipseArc {
                    node_id: id.clone(),
                    start_angle: Some(new_start as f64),
                    sweep_angle: Some(new_sweep as f64),
                    inner_radius: None,
                }
            }
            ArcHandle::Sweep => {
                let new_sweep = signed_sweep(ny.atan2(nx).to_degrees() - old_start, old_sweep);
                EditorCommand::SetEllipseArc {
                    node_id: id.clone(),
                    start_angle: None,
                    sweep_angle: Some(new_sweep as f64),
                    inner_radius: None,
                }
            }
            ArcHandle::Inner => {
                let frac = (nx * nx + ny * ny).sqrt().clamp(0.0, 1.0);
                EditorCommand::SetEllipseArc {
                    node_id: id.clone(),
                    start_angle: None,
                    sweep_angle: None,
                    inner_radius: Some(frac as f64),
                }
            }
        })
    }
}

/// Normalise an angle into `[0, 360)` degrees.
fn norm360(deg: f32) -> f32 {
    let s = deg % 360.0;
    if s < 0.0 {
        s + 360.0
    } else {
        s
    }
}

/// Normalise a sweep into `(0, 360]` — a sweep that collapses to 0
/// snaps to a full 360° circle.
fn norm_sweep(deg: f32) -> f32 {
    let s = norm360(deg);
    if s <= 0.0001 {
        360.0
    } else {
        s
    }
}

/// A sweep that keeps the sign of the arc being edited — an
/// MCP-authored negative (counter-clockwise) sweep stays negative
/// under a canvas drag instead of flipping to the major arc. A
/// negative sweep that collapses to 0 snaps to a full -360° circle
/// (mirroring `norm_sweep`'s positive 0 → 360 rule).
fn signed_sweep(raw: f32, old_sweep: f32) -> f32 {
    let pos = norm_sweep(raw);
    if old_sweep < 0.0 {
        let neg = pos - 360.0;
        if neg == 0.0 {
            -360.0
        } else {
            neg
        }
    } else {
        pos
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
