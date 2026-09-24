//! Workspace chrome painting + deck-strip thumbnail blitting on the
//! native host.
//!
//! Paint order for a workspace frame (assembled by `paint.rs`):
//! workspace chrome (this module) → the real canvas at the docked
//! `canvas_region` (the ordinary CanvasViewport pass) → the pinned chat
//! panel (the ordinary chat pass at `ai_chat_rect`) → the deck strip's
//! real board rasters (this module, blitted over the strip's
//! placeholder plates). Topmost overlays (export dialog, settings
//! modal) paint after, exactly like over the ordinary editor.

use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::{Widget, WorkspaceSurface};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Paint the workspace chrome (header / toolbar / dock background /
    /// strip plates / failed banner). Stamps the entrance clock on the
    /// first frame after a show.
    pub(in crate::widget_host) fn paint_workspace_chrome(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        if self.editor_state.editor_ui.workspace.shown_at_ms == 0 {
            self.editor_state.editor_ui.workspace.shown_at_ms = self.now_ms.max(1);
        }
        // The dock always shows the EXPANDED chat — a minimized bar
        // dragged in from an earlier session would have no working
        // expand affordance (its floating controls are pinned-gated),
        // so reconcile once here like the Home entrance stamp.
        if self.editor_state.chat.is_minimized() {
            self.editor_state.chat.expand();
        }
        let Some(surface) = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms) else {
            return;
        };
        let mut cx = op_editor_ui::widgets::PaintCx {
            backend: &mut *frame,
        };
        surface.paint(&mut cx, Rect::xywh(0.0, 0.0, viewport_w, viewport_h));
    }

    /// Blit the deck strip's real board rasters over the placeholder
    /// plates and top the shared `SlideThumbCache` up within the frame
    /// budget (the same retain/tick/render pump the retired result view
    /// and the slides rail used).
    pub(in crate::widget_host) fn paint_workspace_strip(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        let Some(surface) = WorkspaceSurface::for_editor_at(&self.editor_state, self.now_ms) else {
            return;
        };
        let layout = surface.layout(viewport_w, viewport_h);
        let Some(strip) = layout.strip else {
            // No strip family — make sure no stale rasters linger.
            self.slide_thumbs.retain_boards(&[]);
            return;
        };
        let _ = strip;
        let boards = &surface.boards;
        // Blit whatever rasters are already cached (stale is fine — a
        // board one edit behind reads far better than a hole).
        for (index, board_id) in boards.iter().enumerate() {
            let Some(image) = self.slide_thumbs.image(board_id) else {
                continue;
            };
            let Some(thumb) = layout.thumb_rect(index) else {
                continue;
            };
            let plate = Rect::xywh(
                thumb.origin.x,
                thumb.origin.y,
                thumb.size.x,
                thumb.size.y - 16.0,
            );
            let image = image.clone();
            frame.draw_offscreen_layer_to(&image, plate, plate);
        }
        // Top the cache up at the strip's raster size.
        let revision = self.editor_state.document_revision();
        let ids: Vec<String> = boards.clone();
        self.slide_thumbs.retain_boards(&ids);
        if !self.slide_thumbs.tick(revision, self.now_ms) {
            return;
        }
        let Some(page) = self.layout_scene.active_page() else {
            return;
        };
        let mut wanted: Vec<(String, &op_editor_ui::layout_scene::SceneNode, Point2D)> = Vec::new();
        for (index, board_id) in boards.iter().enumerate() {
            if let Some(node) = page.find(board_id) {
                if let Some(thumb) = layout.thumb_rect(index) {
                    wanted.push((
                        board_id.clone(),
                        node,
                        Point2D::new(thumb.size.x, thumb.size.y - 16.0),
                    ));
                }
            }
        }
        let rendered = self.slide_thumbs.render_pending(frame, &wanted, revision);
        if rendered || self.slide_thumbs.has_pending(frame, &wanted, revision) {
            self.slide_thumbs.wake_in(self.now_ms, 16);
        } else {
            self.slide_thumbs.settle();
        }
    }
}
