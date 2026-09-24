//! Studio workspace painting on the web host.
//!
//! Paint order for a workspace frame (assembled by `paint.rs`), the same as
//! native: workspace chrome (header / toolbar / dock background / strip
//! plates) → the real canvas at the docked `canvas_region` → the pinned chat
//! → the deck strip's board previews, the canvas banners and the quality
//! report. Topmost overlays paint after, exactly like over the editor.
//!
//! **Strip previews.** Desktop blits cached offscreen rasters into the strip
//! (`SlideThumbCache`). The browser bundle has no second surface to raster
//! into (see `slides_panel.rs`), so each visible thumb paints its board's
//! scene subtree directly — scaled into the plate and clipped to it — with
//! the same canvas painter the stage uses. The strip shows at most a window
//! of boards, so the per-frame cost stays bounded.

use super::WidgetHost;
use op_editor_ui::widgets::{PaintCx, Widget, WorkspaceSurface};
use op_editor_ui::{Point2D, Rect, RenderBackend};

/// The thumb slot's bottom band reserved for the board number label.
const THUMB_LABEL_H: f32 = 16.0;

impl WidgetHost {
    /// Header, toolbar, dock background and strip plates. Stamps the
    /// entrance clock on the first frame after a show.
    pub(in crate::widget_host) fn paint_workspace_chrome(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        if self.editor_state.editor_ui.workspace.shown_at_ms == 0 {
            self.editor_state.editor_ui.workspace.shown_at_ms = self.now_ms.max(1);
        }
        // The dock always shows the EXPANDED chat — a minimized bar carried
        // in from the floating editor has no expand affordance while pinned.
        if self.editor_state.chat.is_minimized() {
            self.editor_state.chat.expand();
        }
        let Some(surface) = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms) else {
            return;
        };
        let mut cx = PaintCx { backend };
        surface.paint(&mut cx, Rect::xywh(0.0, 0.0, viewport_w, viewport_h));
    }

    /// Strip previews, canvas banners (failed / stopped Retry, the draft's
    /// connect-or-refine) and the quality report, over the canvas + chat.
    pub(in crate::widget_host) fn paint_workspace_overlays(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        self.paint_workspace_strip(backend, viewport_w, viewport_h);
        let viewport = Rect::xywh(0.0, 0.0, viewport_w, viewport_h);
        if let Some(surface) = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms) {
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            surface.paint_canvas_banners(&mut cx, viewport);
            if self.editor_state.editor_ui.workspace.quality_open {
                surface.paint_quality_overlay(&mut cx, viewport);
            }
        }
        // The entrance fade + rise settles on its own clock.
        if self
            .editor_state
            .editor_ui
            .workspace
            .entrance_deadline_ms(self.now_ms)
            .is_some()
        {
            crate::repaint_coalescer::request();
        }
    }

    /// Paint each visible board into its strip plate (see the module docs).
    fn paint_workspace_strip(
        &mut self,
        backend: &mut dyn RenderBackend,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let Some(surface) = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms) else {
            return;
        };
        let layout = surface.layout(viewport_w, viewport_h);
        if layout.strip.is_none() {
            return;
        }
        let Some(page) = self.layout_scene.active_page() else {
            return;
        };
        for (index, board_id) in surface.boards.iter().enumerate() {
            let Some(thumb) = layout.thumb_rect(index) else {
                continue;
            };
            let plate = Rect::xywh(
                thumb.origin.x,
                thumb.origin.y,
                thumb.size.x,
                (thumb.size.y - THUMB_LABEL_H).max(0.0),
            );
            let Some(node) = page.find(board_id) else {
                continue;
            };
            let Some((origin, zoom)) = thumb_fit(plate, node.bounds) else {
                continue;
            };
            backend.save();
            backend.clip_rect(plate);
            let mut cx = PaintCx {
                backend: &mut *backend,
            };
            op_editor_ui::widgets::scene_paint_options::paint_scene_subtree(
                &mut cx, page, board_id, origin, zoom,
            );
            backend.restore();
        }
    }
}

/// Where a board of `bounds` lands, letterboxed into `plate`: the screen
/// origin of its top-left and the uniform scale. `None` for a degenerate
/// board or plate.
pub(in crate::widget_host) fn thumb_fit(plate: Rect, bounds: Rect) -> Option<(Point2D, f32)> {
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 || plate.size.x <= 0.0 || plate.size.y <= 0.0 {
        return None;
    }
    let zoom = (plate.size.x / bounds.size.x).min(plate.size.y / bounds.size.y);
    let origin = Point2D::new(
        plate.origin.x + (plate.size.x - bounds.size.x * zoom) / 2.0,
        plate.origin.y + (plate.size.y - bounds.size.y * zoom) / 2.0,
    );
    Some((origin, zoom))
}
