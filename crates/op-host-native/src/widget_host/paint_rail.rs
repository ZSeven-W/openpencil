//! Left-rail painting (slides navigator / Pages + Layers tree) — split
//! out of `paint.rs` for the 800-line ceiling.

use super::WidgetHostNative;
use crate::NativeFrameBackend;
use op_editor_ui::widgets::{LayerPanel, PaintCx, Widget};
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// The left rail (slides navigator or Pages + Layers tree). Painted
    /// BEFORE the canvas on the desktop (the rail pushes the canvas) and
    /// AFTER it in mobile layout (the rail overlays the canvas).
    pub(in crate::widget_host) fn paint_left_rail(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // 3. The visible left rail — either a persistent sidebar or the
        //    open touch Layers sheet, and never while presenting. It shows
        //    either the deck's slides navigator, which OWNS the rail when
        //    it is on show, or the Pages + Layers tree.
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
        if rail_open && slides_panel.is_none() {
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
}
