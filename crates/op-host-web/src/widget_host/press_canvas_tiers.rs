//! Web `apply_press` tier 12 — the canvas, plus the Select-tool
//! sub-ladder.
//!
//! Reached only once every overlay, panel, and chrome tier has declined
//! the press. Every path inside the Select sub-ladder returns, which is
//! why it yields a plain `bool` rather than an `Option`.

use super::press_ctx::{CanvasPressCtx, PressCtx};
use super::{DragState, MarqueeDragState, WidgetHost};
use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::CanvasViewport;
use op_editor_ui::{Point2D, Rect};

impl WidgetHost {
    /// `None` — the press was not over the canvas at all.
    pub(in crate::widget_host) fn press_canvas_tier(&mut self, ctx: &PressCtx) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let property_focus_committed = ctx.property_focus_committed;
        let rename_committed = ctx.rename_committed;
        let text_edit_committed = ctx.text_edit_committed;
        let text_edit_was_active = ctx.text_edit_was_active;
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return None;
        }
        // Route canvas presses to preview when preview mode is active
        #[cfg(feature = "canvaskit")]
        if self.editor_state.editor_ui.preview.mode && self.preview.is_some() {
            // Preview chrome paints over the presentation, so it gets the
            // press first — otherwise a tap on a switcher pill would fall
            // through into the previewed screen underneath it.
            if self.preview_switcher_press(x, y, viewport_width, viewport_height)
                || self.screen_switcher_press(x, y, viewport_width, viewport_height)
            {
                self.mark_dirty();
                return Some(true);
            }
            // Commit the presentation surface this gesture started on, so a
            // held drag off the scrolled content does not remap through the
            // pinned nav when it crosses into that band.
            self.capture_device_preview_surface(x, y);
            let consumed = self.preview_dispatch_press(x, y, viewport_width, viewport_height);
            return Some(
                consumed || rename_committed || text_edit_committed || property_focus_committed,
            );
        }

        if matches!(self.editor_state.tool, op_editor_core::Tool::Hand) || self.space_pan {
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return Some(rename_committed || text_edit_committed || property_focus_committed);
        }
        if matches!(self.editor_state.tool, op_editor_core::Tool::Select) {
            return Some(self.press_canvas_select_tier(&CanvasPressCtx {
                x,
                y,
                viewport_width,
                viewport_height,
                rename_committed,
                text_edit_was_active,
                text_edit_committed,
                property_focus_committed,
            }));
        }
        let doc_point = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
        if self.start_create_drag_at(doc_point) {
            return Some(true);
        }

        // Tool didn't accept this point — fall back to pan.
        self.drag = Some(DragState {
            last_x: x,
            last_y: y,
        });
        Some(rename_committed || text_edit_committed || property_focus_committed)
    }

    /// Select-tool canvas press. Always consumes, so it returns the
    /// repaint signal directly.
    fn press_canvas_select_tier(&mut self, ctx: &CanvasPressCtx) -> bool {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let rename_committed = ctx.rename_committed;
        let text_edit_was_active = ctx.text_edit_was_active;
        let text_edit_committed = ctx.text_edit_committed;
        let property_focus_committed = ctx.property_focus_committed;
        if let Some(editing) = self.editor_state.editor_ui.image_crop_editing.clone() {
            let doc_point = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
            if let Some(hit_path) = self
                .layout_scene
                .node_path_at_doc_point(doc_point, self.editor_state.viewport.zoom)
                .filter(|path| path.iter().any(|id| id == editing.as_str()))
            {
                let hit_path = hit_path
                    .into_iter()
                    .map(op_editor_core::NodeId::new)
                    .collect();
                return self.apply_canvas_node_press(
                    hit_path,
                    x,
                    y,
                    text_edit_was_active,
                    viewport_height,
                );
            }
            self.exit_image_crop_edit();
        }
        if self.try_path_anchor_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        if self.try_selection_handle_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        // Convert screen → doc to ask which node (if any)
        // is under the cursor — `node_at_doc_point` queries
        // the layout-resolved render scene.
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let canvas_rect = Rect {
            origin: Point2D::new(cx0, cy0),
            size: Point2D::new(_cw, _ch),
        };
        let doc_point = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
        let canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
        if let Some(sc_node_id) = canvas.frame_label_at_point(canvas_rect, Point2D::new(x, y)) {
            let node_id = op_editor_core::NodeId::new(&sc_node_id);
            return self.apply_canvas_node_press(
                vec![node_id],
                x,
                y,
                text_edit_was_active,
                viewport_height,
            );
        }
        let hit_path = self
            .layout_scene
            .node_path_at_doc_point(doc_point, self.editor_state.viewport.zoom);
        if let Some(hit_path) = hit_path {
            let hit_path = hit_path
                .into_iter()
                .map(op_editor_core::NodeId::new)
                .collect();
            return self.apply_canvas_node_press(
                hit_path,
                x,
                y,
                text_edit_was_active,
                viewport_height,
            );
        }
        // Empty canvas with Select → marquee.
        self.editor_state.editor_ui.last_canvas_click = None;
        // Shared with native: clear the selection and step out of
        // the entered container (clearing-exits rule). The web copy
        // used to `take()` the container outright — after
        // `clear_selection` the shared `sync_entered_container_
        // with_selection` resolves to the same exit, so the two
        // spellings are behaviour-identical.
        let cleared_now = core_press::clear_selection_on_empty_canvas_press(
            &mut self.editor_state,
            self.shift_held,
        );
        if cleared_now {
            self.mark_dirty();
        }
        self.marquee_drag = Some(MarqueeDragState {
            start_screen_x: x,
            start_screen_y: y,
            current_screen_x: x,
            current_screen_y: y,
            additive: self.shift_held,
        });
        cleared_now || rename_committed || text_edit_committed || property_focus_committed
    }
}
