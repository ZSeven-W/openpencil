//! Left-rail painting (slides navigator / Pages + Layers tree) — split
//! out of `paint.rs` for the 800-line ceiling.

use super::WidgetHostNative;
use crate::NativeFrameBackend;
use op_editor_ui::widgets::{LayerPanel, PaintCx, Widget};
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// The left rail (slides navigator, Pages + Layers tree, or the
    /// Chat tab's column). Painted BEFORE the canvas on the desktop
    /// (the rail pushes the canvas) and AFTER it in mobile layout (the
    /// rail overlays the canvas).
    pub(in crate::widget_host) fn paint_left_rail(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // 3. The visible left rail — either a persistent sidebar or the
        //    open touch Layers sheet, and never while presenting. It shows
        //    the deck's slides navigator (which OWNS the rail when it is
        //    on show), the Chat tab's column (the pinned chat paints over
        //    it in the ordinary chat pass, but the rail keeps its card
        //    behind the panel and the tab row on top), or the Pages +
        //    Layers tree.
        let presenting = self.preview_slideshow_active();
        let rail_open = self.left_rail_visible() && !presenting;
        let slides_panel = if rail_open {
            self.slides_panel_frame(viewport_width, viewport_height)
        } else {
            None
        };
        if let Some(slides) = &slides_panel {
            self.paint_slides_panel(frame, slides);
        }
        // The Chat tab owns the rail's BODY — there is no layer tree to
        // paint under the pinned chat, but the rail still paints its
        // card (the chat panel draws over it at the ordinary chat pass)
        // and its tab row, which is how the user gets back.
        let chat_owns_rail = rail_open
            && slides_panel.is_none()
            && op_editor_ui::widgets::slides_panel_flow::chat_tab_active(&self.editor_state);
        if chat_owns_rail {
            let panel = self.layers_content_rect(viewport_width, viewport_height);
            use op_editor_ui::RenderBackend;
            frame.fill_rect(panel, self.theme.card);
            // The dock always shows the EXPANDED chat — a minimized bar
            // carried in from a floating session has no working expand
            // affordance while pinned, so reconcile here exactly like
            // the workspace chrome paint does.
            if self.editor_state.chat.is_minimized() {
                self.editor_state.chat.expand();
            }
            if let Some(tabs) = self.slides_tab_row(viewport_width, viewport_height) {
                self.paint_slides_tab_row(frame, &tabs);
            }
        }
        if rail_open && slides_panel.is_none() && !chat_owns_rail {
            // Compute the active drop target so the panel can paint
            // the drop-indicator line during a drag-to-reorder.
            // The rail is the tab row's leftovers when a tab row shows,
            // so paint and hit-test both start from the same rect.
            let layer_panel_rect = self.layers_content_rect(viewport_width, viewport_height);
            // Build the panel for paint. While a drag is active,
            // exclude the source's subtree so the rendered row stack
            // mirrors the post-commit layout — both the visible rows
            // and the drop-indicator y the user sees are then exactly
            // what `reorder_before/after` produces on release.
            // The panel walks the canonical `PenNode` tree directly
            // off `EditorState`; the drag source id is shell-core's
            // `NodeId` (from the input path), losslessly accepted.
            let active_drag = self.layer_drag.clone().filter(|d| {
                d.active
                    && self
                        .layout_scene
                        .active_page()
                        .map(|p| p.find(d.source.as_str()).is_some())
                        .unwrap_or(false)
            });
            let mut layer_panel = if let Some(d) = &active_drag {
                LayerPanel::from_editor_with_drag_source(&self.editor_state, &d.source)
            } else {
                // Per-frame paint: resolve the row model through the
                // owner-scoped cache so idle / streaming / hover repaints
                // that don't touch the layer tree skip the walk + measure.
                self.layer_panel()
            };

            // Auto-reveal selected node: if the selection changed and differs
            // from the last-revealed anchor, expand ancestors and reveal.
            // This covers MCP set_selection, undo/redo, and programmatic
            // selection changes (not just canvas clicks).
            if active_drag.is_none() {
                // Only auto-reveal when not dragging; explicit drag interactions
                // take precedence and manual collapse should be respected.
                let should_reveal = match (
                    &self.editor_state.selection.anchor,
                    &self.editor_state.editor_ui.last_revealed_layer_anchor,
                ) {
                    (anchor, last) if anchor.is_real() => Some(anchor) != last.as_ref(),
                    _ => false,
                };
                if should_reveal {
                    op_editor_ui::widgets::scroll_flow::reveal_layer_panel_selection(
                        &mut self.editor_state,
                        &layer_panel,
                        layer_panel_rect,
                    );
                    // Rebuild the panel after reveal to reflect any expanded ancestors.
                    layer_panel = self.layer_panel();
                }
            }

            if let Some(d) = &active_drag {
                layer_panel.drop_target = layer_panel
                    .drop_target_at(layer_panel_rect, Point2D::new(d.current_x, d.current_y));
                // Floating ghost — keeps the source visible mid-drag.
                if let Some(item) = LayerPanel::ghost_item_for(&self.editor_state, &d.source) {
                    layer_panel.drag_ghost = Some((item, d.current_y));
                }
            }
            layer_panel.now_ms = self.now_ms;
            {
                let mut cx = PaintCx {
                    backend: &mut *frame,
                };
                layer_panel.paint(&mut cx, layer_panel_rect);
            }
            // The tab row heads the rail in BOTH tabs — it is how the
            // user gets back to the slides — so it paints over the
            // layer tree's own card background here.
            if let Some(tabs) = self.slides_tab_row(viewport_width, viewport_height) {
                self.paint_slides_tab_row(frame, &tabs);
            }
        }
    }

    /// One hard edge between the rail and the canvas.
    ///
    /// Painted AFTER the pinned chat, not at the end of the rail pass:
    /// the chat fills the rail's body with its own ground, which buried
    /// everything below the tab row and left the rule stopping halfway
    /// down. The rail's card and the canvas ground are close enough in
    /// the light theme that the seam read as a smudge without it.
    pub(in crate::widget_host) fn paint_rail_canvas_edge(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_height: f32,
    ) {
        if !self.editor_state.editor_ui.sidebar_open
            || self.editor_state.editor_ui.touch_chrome()
            || self.editor_state.editor_ui.preview.mode
        {
            return;
        }
        use op_editor_ui::RenderBackend;
        let panel = op_editor_ui::widgets::host_canvas_geometry::layer_panel_rect(
            &self.editor_state,
            viewport_height,
        );
        frame.fill_rect(
            op_editor_ui::Rect::xywh(
                panel.origin.x + panel.size.x - 1.0,
                panel.origin.y,
                1.0,
                panel.size.y,
            ),
            self.theme.border,
        );
    }
}
