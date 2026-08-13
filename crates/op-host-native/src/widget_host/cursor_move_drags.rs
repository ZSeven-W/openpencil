//! `apply_cursor_move` tier 9 — the late pointer-capture drags.
//!
//! These run AFTER the chat hover probe (the align-toolbar tier gates
//! itself on them being idle) and before the base canvas tier. Each one
//! owns the cursor for the duration of its gesture.

use super::helpers::{resize_bounds, PANEL_MAX_WIDTH, PANEL_MIN_WIDTH};
use super::input::rect_to_doc_rect;
use super::{PanelResizeKind, WidgetHostNative};
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::{
    ChatResizeEdge, AI_CHAT_MAX_RATIO, AI_CHAT_MIN_HEIGHT, AI_CHAT_MIN_WIDTH,
};
use op_editor_ui::Rect;

impl WidgetHostNative {
    /// A live rail-resize drag owns the cursor outright. `None` — no
    /// resize is in flight.
    ///
    /// This runs with the other pointer-capture drags (spine tier 3) and
    /// NOT with the late drags below, because a rail resize is the one
    /// gesture whose pointer spends half its travel INSIDE the surface it
    /// is resizing. From the late tier the hover tiers above it — the
    /// slides tab in particular, which claims every point on the left
    /// rail — swallowed each move that went back over the rail, so the
    /// left rail could be dragged wider but never narrower: dragging
    /// right left the rail and reached this code, dragging left re-entered
    /// it and never did.
    pub(in crate::widget_host) fn cursor_move_panel_resize_tier(&mut self, x: f32) -> Option<bool> {
        let resize = self.panel_resize?;
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
        Some(true)
    }

    /// `None` — no late drag owns the cursor.
    pub(in crate::widget_host) fn cursor_move_late_drag_tiers(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        if self.rotate_drag.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodeProperty(
                    op_editor_core::CollabNodeField::Rotation,
                ),
            )
        {
            self.rotate_drag = None;
            return Some(true);
        }
        if let Some(drag) = self.rotate_drag {
            let cursor_angle = (y - drag.center_screen_y).atan2(x - drag.center_screen_x);
            let new_rotation = drag.start_rotation + (cursor_angle - drag.start_cursor_angle);
            self.editor_state.set_selected_rotation(new_rotation);
            self.mark_dirty();
            return Some(true);
        }
        if self.handle_drag.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodePropertyBatch,
            )
        {
            self.handle_drag = None;
            return Some(true);
        }
        if let Some(drag) = self.handle_drag {
            let zoom = self.editor_state.viewport.zoom.max(0.0001);
            let dx = (x - drag.start_screen_x) / zoom;
            let dy = (y - drag.start_screen_y) / zoom;
            let new_bounds = resize_bounds(drag.start_bounds, drag.handle, dx, dy);
            let new_x = drag.handle.moves_left_edge().then(|| {
                drag.start_authored_x.unwrap_or(0.0)
                    + f64::from(new_bounds.origin.x - drag.start_bounds.origin.x)
            });
            let new_y = drag.handle.moves_top_edge().then(|| {
                drag.start_authored_y.unwrap_or(0.0)
                    + f64::from(new_bounds.origin.y - drag.start_bounds.origin.y)
            });
            self.editor_state.resize_selected_bounds(
                rect_to_doc_rect(new_bounds),
                drag.handle.resize_axes(),
                new_x,
                new_y,
            );
            self.mark_dirty();
            return Some(true);
        }
        if self.create_drag.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodePropertyBatch,
            )
        {
            self.create_drag = None;
            return Some(true);
        }
        if let Some(drag) = self.create_drag {
            let cur = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
            let min_x = drag.start_doc_x.min(cur.x);
            let min_y = drag.start_doc_y.min(cur.y);
            // Text needs room for placeholder glyphs.
            let (min_w, min_h) = match self.editor_state.tool {
                op_editor_core::Tool::Text => (
                    op_editor_core::DEFAULT_TEXT_NODE_WIDTH as f32,
                    op_editor_core::DEFAULT_TEXT_NODE_HEIGHT as f32,
                ),
                _ => (1.0_f32, 1.0_f32),
            };
            let w = (drag.start_doc_x - cur.x).abs().max(min_w);
            let h = (drag.start_doc_y - cur.y).abs().max(min_h);
            let new_bounds = Rect::xywh(min_x, min_y, w, h);
            self.editor_state
                .set_selected_bounds(rect_to_doc_rect(new_bounds));
            self.mark_dirty();
            return Some(true);
        }
        // Path-anchor / handle drag — TS `movePathControl` semantics
        // (`pen_press.rs::apply_path_anchor_drag_move`).
        if self.apply_path_anchor_drag_move(x, y) {
            return Some(true);
        }
        // Ellipse arc-handle drag: recompute arc geometry from the cursor.
        if self.arc_handle_drag.is_some() {
            if !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::UnsupportedNodeProperty,
                ),
            ) {
                self.arc_handle_drag = None;
                return Some(true);
            }
            let doc = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
            let (id, handle, start, already_moved) = {
                let d = self.arc_handle_drag.as_ref().unwrap();
                (d.node_id.clone(), d.handle, d.start_doc, d.moved)
            };
            // Do not mutate until the cursor first travels.
            let is_move = (doc.x - start.x).abs() > 0.001 || (doc.y - start.y).abs() > 0.001;
            if is_move || already_moved {
                self.refresh_layout_scene();
                if let Some(cmd) = self.arc_drag_command(&id, handle, doc) {
                    if self.editor_state.apply(cmd) {
                        self.mark_dirty();
                        if let Some(d) = self.arc_handle_drag.as_mut() {
                            d.moved = true;
                        }
                    }
                }
            }
            return Some(true);
        }
        if let Some(m) = self.marquee_drag.as_mut() {
            m.current_screen_x = x;
            m.current_screen_y = y;
            return Some(true);
        }
        if self.layer_drag.is_some() {
            let activation_threshold = if self.editor_state.editor_ui.touch_chrome() {
                12.0
            } else {
                4.0
            };
            let should_activate = self.layer_drag.as_ref().is_some_and(|drag| {
                !drag.active && (y - drag.start_y).abs() > activation_threshold
            });
            if should_activate
                && !self.collab_allows_document_mutation(
                    op_editor_core::CollabDocumentMutation::NodeMove,
                )
            {
                self.layer_drag = None;
                return Some(true);
            }
            self.refresh_layout_scene();
            let source_id = self.layer_drag.as_ref().unwrap().source.clone();
            let still_present = self
                .layout_scene
                .active_page()
                .map(|p| p.find(source_id.as_str()).is_some())
                .unwrap_or(false);
            if !still_present {
                self.layer_drag = None;
                return Some(true);
            }
            let d = self.layer_drag.as_mut().unwrap();
            d.current_x = x;
            d.current_y = y;
            // Vertical-only activation — horizontal wiggle preserved
            // for selection / eye / lock click-feel.
            if !d.active && (y - d.start_y).abs() > activation_threshold {
                d.active = true;
            }
            return Some(true);
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
            return Some(true);
        }
        if let Some(d) = self.chat_drag.as_mut() {
            d.pos_x = x - d.grab_dx;
            d.pos_y = y - d.grab_dy;
            return Some(true);
        }
        if let Some(drag) = self.drag.as_mut() {
            let dx = x - drag.last_x;
            let dy = y - drag.last_y;
            drag.last_x = x;
            drag.last_y = y;
            self.editor_state.viewport.pan(dx, dy);
            self.note_viewport_gesture();
            // Canvas pan only translates the viewport; keep layout cache intact.
            return Some(true);
        }
        None
    }
}
