//! `apply_press` tier 12 — the canvas itself, plus the Select-tool
//! sub-ladder.
//!
//! Reached only once every overlay, panel, and chrome tier has declined the
//! press. The Select sub-ladder has its own strict order, inherited from
//! the TS `handleSelectMouseDown`: path controls first (so anchor dots near
//! a corner stay grabbable), then arc handles (the sweep handle can overlap
//! the right-mid resize grip), then resize handles, then rotation corners,
//! then frame labels, then ordinary node hits, then the empty-canvas
//! marquee. Every path returns, which is why the sub-ladder yields a plain
//! `bool` rather than an `Option`.

use super::press_ctx::{CanvasPressCtx, PressCtx};
use super::{CreateDragState, DragState, HandleDragState, RotateDragState, WidgetHostNative};
use op_editor_core::host_press_transitions as core_press;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_ui::widgets::host_canvas_geometry as canvas_geometry;
use op_editor_ui::widgets::{rotation_corner_at_point, selection_handle_at_point, CanvasViewport};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// `None` — the press was not over the canvas at all.
    pub(in crate::widget_host) fn press_canvas_tier(&mut self, ctx: &PressCtx) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let rename_committed = ctx.rename_committed;
        let text_edit_committed = ctx.text_edit_committed;
        let text_edit_was_active = ctx.text_edit_was_active;
        if !self.over_canvas(x, y, viewport_width, viewport_height) {
            return None;
        }
        use op_editor_core::Tool;
        // Any canvas press blurs the chrome text inputs first —
        // node hit or not, the pointer left every input (DOM
        // parity: mousedown anywhere outside an input blurs it).
        let blurred = self.blur_text_inputs_on_blank_press();
        // A press on the canvas while the Git panel is open dismisses
        // it — the panel is a popover (TS parity: clicking off a
        // popover closes it), and a click inside it was already
        // consumed at §0.9, so anything reaching here is genuinely
        // outside. Swallow the press so the dismiss doesn't also
        // select / pan / start drawing.
        if self.editor_state.editor_ui.git_panel.open {
            self.close_git_panel();
            self.mark_dirty();
            return Some(true);
        }
        self.refresh_layout_scene();
        if matches!(self.editor_state.tool, Tool::Hand) || self.space_pan {
            self.drag = Some(DragState {
                last_x: x,
                last_y: y,
            });
            return Some(blurred || rename_committed || text_edit_committed);
        }
        let (cx0, cy0, cw, ch) = self.canvas_region(viewport_width, viewport_height);
        let canvas_rect = Rect {
            origin: Point2D::new(cx0, cy0),
            size: Point2D::new(cw, ch),
        };
        let doc_point = canvas_geometry::canvas_doc_point_unclamped(&self.editor_state, x, y);
        if matches!(self.editor_state.tool, Tool::Select) {
            return Some(self.press_canvas_select_tier(&CanvasPressCtx {
                x,
                y,
                viewport_width,
                viewport_height,
                canvas_rect,
                doc_point,
                text_edit_was_active,
                blurred,
            }));
        }

        // Pen tool: anchor edit / author (`pen_press.rs`).
        if matches!(self.editor_state.tool, Tool::Pen) {
            return Some(self.apply_pen_tool_press(x, y, viewport_width, viewport_height));
        }

        // Shape / Frame / Text tool: spawn a new node + drag.
        let pre_create = self.editor_state.snapshot_for_history();
        if let Some(node_id) = self.create_node_for_active_tool(doc_point) {
            self.editor_state.history_push_past(pre_create);
            self.editor_state.set_single_selection(node_id);
            self.create_drag = Some(CreateDragState {
                start_doc_x: doc_point.x,
                start_doc_y: doc_point.y,
            });
            self.mark_dirty();
            return Some(true);
        }

        // Tool didn't accept this point — fall back to pan.
        self.drag = Some(DragState {
            last_x: x,
            last_y: y,
        });
        Some(blurred || rename_committed || text_edit_committed)
    }

    /// Select-tool canvas press. Always consumes, so it returns the
    /// repaint signal directly.
    fn press_canvas_select_tier(&mut self, ctx: &CanvasPressCtx) -> bool {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let canvas_rect = ctx.canvas_rect;
        let doc_point = ctx.doc_point;
        let text_edit_was_active = ctx.text_edit_was_active;
        let blurred = ctx.blurred;
        if let Some(editing) = self.editor_state.editor_ui.image_crop_editing.clone() {
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
        // Path controls hit-test FIRST (TS handleSelectMouseDown,
        // skia-interaction.ts:277-306) — before arc / resize /
        // rotation, so anchor dots near a corner stay grabbable.
        if self.try_path_anchor_press(x, y, viewport_width, viewport_height) {
            return true;
        }
        // The selected anchor's resolved scene node — shared
        // by the handle-drag + rotation-drag branches below.
        let selected_anchor = self.editor_state.selection.anchor.as_str().to_string();
        // Arc handles take priority over the 8 resize handles —
        // the sweep handle can overlap the right-mid resize grip.
        if let Some((node_id, handle)) = self.arc_handle_hit(x, y, viewport_width, viewport_height)
        {
            if !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::UnsupportedNodeProperty,
                ),
            ) {
                return true;
            }
            let pre = self.editor_state.snapshot_for_history();
            self.arc_handle_drag = Some(super::ArcHandleDragState {
                node_id: op_editor_core::NodeId::new(&node_id),
                handle,
                start_doc: doc_point,
                moved: false,
                pre_drag_snapshot: pre,
            });
            return true;
        }
        if let Some(handle) = selection_handle_at_point(
            canvas_rect,
            &self.layout_scene,
            &self.editor_state,
            Point2D::new(x, y),
        ) {
            if !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodePropertyBatch,
            ) {
                return true;
            }
            let (start_authored_x, start_authored_y) = self
                .editor_state
                .selected_node()
                .map(|node| (node.base().x, node.base().y))
                .unwrap_or((None, None));
            if let Some(node) = self
                .layout_scene
                .active_page()
                .and_then(|p| p.find(&selected_anchor))
            {
                // Only handle-drag on nodes with real bounds.
                let raw = node.bounds;
                if raw.size.x > 0.0 || raw.size.y > 0.0 {
                    self.editor_state.commit_history();
                    self.handle_drag = Some(HandleDragState {
                        handle,
                        start_screen_x: x,
                        start_screen_y: y,
                        start_bounds: raw,
                        start_authored_x,
                        start_authored_y,
                    });
                    return true;
                }
            }
        }
        if rotation_corner_at_point(
            canvas_rect,
            &self.layout_scene,
            &self.editor_state,
            Point2D::new(x, y),
        )
        .is_some()
        {
            if !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodeProperty(
                    op_editor_core::CollabNodeField::Rotation,
                ),
            ) {
                return true;
            }
            if let Some(node) = self
                .layout_scene
                .active_page()
                .and_then(|p| p.find(&selected_anchor))
            {
                let bounds = node.aggregate_bounds();
                let cx_doc = bounds.origin.x + bounds.size.x / 2.0;
                let cy_doc = bounds.origin.y + bounds.size.y / 2.0;
                let center_screen_x = canvas_rect.origin.x
                    + self.editor_state.viewport.pan_x
                    + cx_doc * self.editor_state.viewport.zoom;
                let center_screen_y = canvas_rect.origin.y
                    + self.editor_state.viewport.pan_y
                    + cy_doc * self.editor_state.viewport.zoom;
                let start_cursor_angle = (y - center_screen_y).atan2(x - center_screen_x);
                let start_rotation = node.rotation;
                self.editor_state.commit_history();
                self.rotate_drag = Some(RotateDragState {
                    center_screen_x,
                    center_screen_y,
                    start_cursor_angle,
                    start_rotation,
                });
                return true;
            }
        }
        let canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
        if let Some(node_id) = canvas.frame_label_at_point(canvas_rect, Point2D::new(x, y)) {
            let ec_id = op_editor_core::NodeId::new(&node_id);
            return self.apply_canvas_node_press(
                vec![ec_id],
                x,
                y,
                text_edit_was_active,
                viewport_height,
            );
        }
        if let Some(hit_path) = self
            .layout_scene
            .node_path_at_doc_point(doc_point, self.editor_state.viewport.zoom)
        {
            // Relative-level selection / one-step drill / drag start —
            // see `canvas_select_drag.rs`.
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
        // Empty canvas press — start a marquee.
        self.editor_state.editor_ui.last_canvas_click = None;
        let cleared_now = core_press::clear_selection_on_empty_canvas_press(
            &mut self.editor_state,
            self.shift_held,
        );
        if cleared_now {
            self.mark_dirty();
        }
        self.marquee_drag = Some(super::MarqueeDragState {
            start_screen_x: x,
            start_screen_y: y,
            current_screen_x: x,
            current_screen_y: y,
            additive: self.shift_held,
        });
        cleared_now || blurred
    }
}
