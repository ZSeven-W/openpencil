//! Non-press input handlers on `WidgetHostNative`. press -> press.rs.

use super::helpers::{resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH};
use super::{PanelResizeKind, WidgetHostNative};
use op_editor_ui::widgets::{
    ChatResizeEdge, AI_CHAT_MAX_RATIO, AI_CHAT_MIN_HEIGHT, AI_CHAT_MIN_WIDTH,
};
use op_editor_ui::{Point2D, Rect};

/// Minimum cursor travel (logical px) from the node-drag press point
/// before a move is committed. A pure click with sub-pixel jitter then
/// never mutates the document — kills "first click breaks the layout".
const NODE_DRAG_THRESHOLD_PX: f32 = 4.0;

impl WidgetHostNative {
    /// True iff a text-input surface owns the keyboard.
    pub(in crate::widget_host) fn input_active(&self) -> bool {
        let ui = &self.editor_state.ui;
        ui.layer_rename.is_some()
            || ui.text_editing.is_some()
            || ui.property_focus.is_some()
            || self.editor_state.editor_ui.variable_row_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
            || self.editor_state.editor_ui.agent_settings.focus.is_some()
            || self.editor_state.editor_ui.icon_picker_open
            || self.editor_state.editor_ui.chat_model_picker_open
            || self.editor_state.editor_ui.component_browser_open
            || self.editor_state.chat.focused
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.git_https_focus_active()
            || self.git_branch_create_focus_active()
            || self.git_clone_input_active()
    }

    pub fn settings_focus_active(&self) -> bool {
        self.editor_state.editor_ui.agent_settings.focus.is_some()
    }

    /// Whether the visible Git commit-message input owns the keyboard.
    pub fn git_commit_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        // The branch-picker dropdown has no commit input; while it is open a
        // stale `commit_focused` must not route keys (text / Enter) to the
        // hidden commit box.
        panel.open && panel.commit_focused && !panel.loading && !panel.branch_picker_open
    }

    /// Whether the visible Git remote-URL input owns the keyboard.
    pub fn git_remote_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.remote_focused && !panel.loading && !panel.branch_picker_open
    }

    /// Whether the visible Git HTTPS-credential input owns the keyboard.
    pub fn git_https_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.https_focused && !panel.loading && !panel.branch_picker_open
    }

    /// Whether the inline create-branch name input owns the keyboard.
    pub fn git_branch_create_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.branch_create_focused && !panel.loading
    }

    /// Whether a ready-state Git popover (branch picker / overflow menu) is
    /// actually visible — the panel is open, in the ready view, and a popover
    /// flag is set. Scopes the Enter swallow so a stale flag while the panel
    /// is closed / loading / merging / showing a diff can't eat global Enter.
    pub fn git_ready_popover_open(&self) -> bool {
        let p = &self.editor_state.editor_ui.git_panel;
        p.open
            && p.in_repo
            && !p.loading
            && !p.merging
            && p.diff.is_none()
            && p.merge_resolve.is_none()
            && (p.branch_picker_open || p.overflow_open)
    }

    /// Whether the inline Git clone wizard is up. While it is, the
    /// wizard owns the keyboard: a focused URL / destination field takes
    /// text, and every other key is swallowed so no canvas shortcut
    /// (tool letters, Delete, arrow nudges, …) leaks to the document
    /// while the user types a URL. View-level (not field-level) because
    /// the wizard covers the panel even between field focuses.
    pub fn git_clone_input_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.clone_form.is_some()
    }

    /// Snap node drags to nearby top-level edge/centre guides.
    fn apply_smart_guides(&mut self) -> (f64, f64) {
        use op_editor_core::align_guides::compute_alignment_guides;
        /// Snap range in doc-px.
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
            return (0.0, 0.0);
        };
        let others: Vec<(f64, f64, f64, f64)> =
            others.iter().map(|a| (a[0], a[1], a[2], a[3])).collect();
        let result = compute_alignment_guides((m[0], m[1], m[2], m[3]), &others, GUIDE_THRESHOLD);
        let snap = (result.snap_dx, result.snap_dy);
        if result.snap_dx != 0.0 || result.snap_dy != 0.0 {
            self.editor_state
                .translate_selected(result.snap_dx, result.snap_dy);
        }
        self.editor_state.editor_ui.active_guides = result.guides;
        snap
    }

    pub fn apply_cursor_move(&mut self, x: f32, y: f32) -> bool {
        // Keep hit-tests on the current layout-resolved scene.
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
        // Top-most floating panel drags own cursor movement.
        if let Some(d) = self.design_md_drag {
            self.editor_state.editor_ui.design_md_panel_pos = Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        if let Some(d) = self.component_browser_drag {
            self.editor_state.editor_ui.component_browser_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        if let Some(d) = self.icon_picker_drag {
            self.editor_state.editor_ui.icon_picker_panel_pos =
                Some((x - d.grab_dx, y - d.grab_dy));
            self.mark_dirty();
            return true;
        }
        if let Some(field) = self.image_adjustment_drag {
            if let Some(panel) =
                op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)
            {
                let property_rect = Rect {
                    origin: Point2D::new(
                        self.last_viewport_w - self.editor_state.editor_ui.property_panel_width,
                        op_editor_ui::widgets::TOP_BAR_HEIGHT,
                    ),
                    size: Point2D::new(
                        self.editor_state.editor_ui.property_panel_width,
                        (self.last_viewport_h - op_editor_ui::widgets::TOP_BAR_HEIGHT).max(0.0),
                    ),
                };
                if let Some(action) = panel.image_adjustment_drag_action(property_rect, field, x) {
                    self.apply_property_action(action);
                    return true;
                }
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
        // Git empty-state Init-card hover — toggles the disabled hint
        // pill (a no-op repaint=false when the panel is closed / the
        // cursor isn't over that card).
        if self.update_git_panel_empty_hover(x, y) {
            return true;
        }
        // Git ready-view branch-button hover — `hover:bg-accent` wash on
        // the `⎇ <branch> ▾` trigger.
        if self.update_git_panel_ready_hover(x, y) {
            return true;
        }
        // Suppress lower-overlay hover while a floating panel is on top.
        let over_topmost =
            self.over_topmost_panel(x, y, self.last_viewport_w, self.last_viewport_h);
        // Fold stale-hover clearing into the final repaint signal.
        let cleared = over_topmost && self.clear_lower_overlay_hover();
        if let Some(state) = self
            .editor_state
            .editor_ui
            .layer_context_menu
            .clone()
            .filter(|_| !over_topmost)
        {
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
        // File-menu / locale / shape dropdown hover (`geometry.rs`).
        if self.update_dropdown_hover(x, y, over_topmost) {
            return true;
        }
        // Export-section select-popup row hover (no-op when closed).
        if !over_topmost
            && self.update_export_picker_hover(x, y, self.last_viewport_w, self.last_viewport_h)
        {
            return true;
        }
        // Padding-mode gear popover row hover (no-op when closed).
        if self.update_padding_mode_popover_hover(x, y) {
            return true;
        }
        // Font-weight dropdown row hover (no-op when closed).
        if self.update_font_weight_picker_hover(x, y) {
            return true;
        }
        // TopBar window-control cluster — hovering it reveals the
        // close / minimise / maximise glyphs on the 3 dots.
        {
            use op_editor_ui::widgets::{TopBar, TOP_BAR_HEIGHT};
            let tb_rect = Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.last_viewport_w, TOP_BAR_HEIGHT),
            };
            let over = super::helpers::rect_contains(
                TopBar::traffic_cluster_rect(tb_rect),
                Point2D::new(x, y),
            );
            if over != self.editor_state.editor_ui.topbar_traffic_hover {
                self.editor_state.editor_ui.topbar_traffic_hover = over;
                self.mark_dirty();
                return true;
            }
        }
        // Open chat model-picker — track the model row under the
        // cursor so the dropdown paints a hover wash. `model_at`
        // returns `None` off the rows (headers / padding / off the
        // card), which clears any stale highlight.
        if self.editor_state.editor_ui.chat_model_picker_open && !over_topmost {
            use op_editor_ui::widgets::ai_chat_model_picker::model_at;
            use op_editor_ui::widgets::AIChatPlaceholder;
            let picker = self
                .ai_chat_rect(self.last_viewport_w, self.last_viewport_h)
                .and_then(|chat_rect| {
                    AIChatPlaceholder::from_editor_at(&self.editor_state, self.now_ms)
                        .model_picker_bounds(chat_rect)
                });
            if let Some(picker) = picker {
                let scroll = self.editor_state.editor_ui.chat_model_picker_scroll;
                let new_hover = model_at(
                    picker,
                    Point2D::new(x, y),
                    &self.editor_state.chat.available_models,
                    scroll,
                    &self.editor_state.editor_ui.chat_model_picker_search,
                );
                if new_hover != self.editor_state.editor_ui.chat_model_picker_hover {
                    self.editor_state.editor_ui.chat_model_picker_hover = new_hover;
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if self.update_chat_design_hover(x, y, over_topmost) {
            return true;
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
            // Text needs room for placeholder glyphs.
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
        if let Some(drag) = self.node_drag {
            // Threshold gate: a pure click with sub-pixel jitter must not
            // move (or re-flow) anything. Until the cursor has travelled
            // past NODE_DRAG_THRESHOLD_PX from the press point, swallow the
            // move; once it crosses, the drag latches and moves for the
            // rest of the gesture.
            if !drag.moved
                && (x - drag.press_screen_x).abs() <= NODE_DRAG_THRESHOLD_PX
                && (y - drag.press_screen_y).abs() <= NODE_DRAG_THRESHOLD_PX
            {
                return false;
            }
            if !drag.moved {
                if let Some(d) = self.node_drag.as_mut() {
                    d.moved = true;
                }
            }
            let zoom = self.editor_state.viewport.zoom.max(0.0001);
            let prev_screen_x = drag.last_screen_x;
            let prev_screen_y = drag.last_screen_y;
            let dx = (x - prev_screen_x) / zoom;
            let dy = (y - prev_screen_y) / zoom;
            if dx != 0.0 || dy != 0.0 {
                if let Some(drag) = self.node_drag.as_mut() {
                    drag.last_screen_x = x;
                    drag.last_screen_y = y;
                }
                self.editor_state.translate_selected(dx as f64, dy as f64);
                let (snap_dx, snap_dy) = self.apply_smart_guides();
                if let Some(drag) = self.node_drag.as_mut() {
                    // Keep snapped axes accumulating instead of eating small moves.
                    if snap_dx != 0.0 {
                        drag.last_screen_x = prev_screen_x;
                    }
                    if snap_dy != 0.0 {
                        drag.last_screen_y = prev_screen_y;
                    }
                }
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Path-anchor / handle drag: always write current cursor position.
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
            // Motion detection is frame-agnostic; writes start after first move.
            let is_move = (doc.x - start.x).abs() > 0.001 || (doc.y - start.y).abs() > 0.001;
            if is_move || already_moved {
                // Write anchor / handle coords in the path's local frame.
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
                        // First real move sets the anchor's point type.
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
        // Ellipse arc-handle drag: recompute arc geometry from the cursor.
        if self.arc_handle_drag.is_some() {
            let (cx0, cy0) = self.canvas_origin();
            let canvas_local = Point2D::new(x - cx0, y - cy0);
            let doc = self.editor_state.viewport.to_document(canvas_local);
            let (id, handle, start, already_moved) = {
                let d = self.arc_handle_drag.as_ref().unwrap();
                (d.node_id.clone(), d.handle, d.start_doc, d.moved)
            };
            // Do not mutate until the cursor first travels.
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
        if let Some(resize) = self.chat_resize {
            let (cx0, cy0, cw, ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
            let max_w = (cw * AI_CHAT_MAX_RATIO).max(AI_CHAT_MIN_WIDTH);
            let max_h = (ch * AI_CHAT_MAX_RATIO).max(AI_CHAT_MIN_HEIGHT);
            let dx = x - resize.start_x;
            let dy = y - resize.start_y;
            let mut new_w = resize.start_rect.size.x;
            let mut new_h = resize.start_rect.size.y;
            let mut new_left = resize.start_rect.origin.x;
            let mut new_top = resize.start_rect.origin.y;

            if matches!(
                resize.edge,
                ChatResizeEdge::E | ChatResizeEdge::Ne | ChatResizeEdge::Se
            ) {
                new_w = resize.start_rect.size.x + dx;
            }
            if matches!(
                resize.edge,
                ChatResizeEdge::W | ChatResizeEdge::Nw | ChatResizeEdge::Sw
            ) {
                new_w = resize.start_rect.size.x - dx;
                new_left = resize.start_rect.origin.x + dx;
            }
            if matches!(
                resize.edge,
                ChatResizeEdge::S | ChatResizeEdge::Se | ChatResizeEdge::Sw
            ) {
                new_h = resize.start_rect.size.y + dy;
            }
            if matches!(
                resize.edge,
                ChatResizeEdge::N | ChatResizeEdge::Ne | ChatResizeEdge::Nw
            ) {
                new_h = resize.start_rect.size.y - dy;
                new_top = resize.start_rect.origin.y + dy;
            }

            if new_w < AI_CHAT_MIN_WIDTH {
                let diff = AI_CHAT_MIN_WIDTH - new_w;
                new_w = AI_CHAT_MIN_WIDTH;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::W | ChatResizeEdge::Nw | ChatResizeEdge::Sw
                ) {
                    new_left -= diff;
                }
            }
            if new_w > max_w {
                let diff = new_w - max_w;
                new_w = max_w;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::W | ChatResizeEdge::Nw | ChatResizeEdge::Sw
                ) {
                    new_left += diff;
                }
            }
            if new_h < AI_CHAT_MIN_HEIGHT {
                let diff = AI_CHAT_MIN_HEIGHT - new_h;
                new_h = AI_CHAT_MIN_HEIGHT;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::N | ChatResizeEdge::Ne | ChatResizeEdge::Nw
                ) {
                    new_top -= diff;
                }
            }
            if new_h > max_h {
                let diff = new_h - max_h;
                new_h = max_h;
                if matches!(
                    resize.edge,
                    ChatResizeEdge::N | ChatResizeEdge::Ne | ChatResizeEdge::Nw
                ) {
                    new_top += diff;
                }
            }

            let max_left = cx0 + cw - new_w;
            let max_top = cy0 + ch - new_h;
            new_left = new_left.clamp(cx0, max_left.max(cx0));
            new_top = new_top.clamp(cy0, max_top.max(cy0));
            self.editor_state.chat.panel_width = new_w.round();
            self.editor_state.chat.panel_height = new_h.round();
            self.editor_state.chat.panel_position = Some((new_left.round(), new_top.round()));
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
            // Canvas pan only translates the viewport; keep layout cache intact.
            return true;
        }
        // Toolbar hover after drag detection.
        if self.update_toolbar_hover(x, y, over_topmost) {
            return true;
        }
        // Align toolbar hover after drag detection.
        let new_hover = if self.editor_state.selection_count() >= 2 && !over_topmost {
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
        // Fold stale-hover clearing into the repaint signal.
        cleared
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
        if self.chat_resize.take().is_some() {
            return true;
        }
        if self.rotate_drag.take().is_some() {
            return true;
        }
        if self.handle_drag.take().is_some() {
            return true;
        }
        if self.create_drag.take().is_some() {
            // Switch back to Select for immediate shape refinement.
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
            // Push history only when the anchor actually moved.
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
        if self.design_md_drag.take().is_some() {
            // Position was updated live; release only ends the drag.
            return true;
        }
        if self.component_browser_drag.take().is_some() {
            return true;
        }
        if self.icon_picker_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
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
            // Snap using the live expanded/collapsed panel size.
            let (panel_w, panel_h) = self.ai_chat_size();
            let center = Point2D::new(d.pos_x + panel_w / 2.0, d.pos_y + panel_h / 2.0);
            let (cx0, cy0, cw, ch) = self.canvas_region(viewport_w, viewport_h);
            self.editor_state.chat.anchor =
                op_editor_core::ChatAnchor::nearest(center, cx0, cy0, cw, ch);
            self.editor_state.chat.panel_position = None;
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
        if self.chat_resize.take().is_some() {
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
            // No viewport: drop without committing.
            return true;
        }
        if self.layer_drag.take().is_some() {
            // No viewport: drop the candidate.
            return true;
        }
        // Commit path / arc history when the drag actually moved.
        if let Some(drag) = self.path_anchor_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
            }
            return true;
        }
        if let Some(drag) = self.arc_handle_drag.take() {
            if drag.moved {
                self.editor_state.history_push_past(drag.pre_drag_snapshot);
            }
            return true;
        }
        // Chat drag without viewport — drop it (best effort).
        if self.chat_drag.take().is_some() {
            return true;
        }
        if self.design_md_drag.take().is_some() {
            return true;
        }
        if self.component_browser_drag.take().is_some() {
            return true;
        }
        if self.icon_picker_drag.take().is_some() {
            return true;
        }
        if self.image_adjustment_drag.take().is_some() {
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
