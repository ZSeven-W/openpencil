//! Pan bitmap cache on `WidgetHostNative`.
//!
//! During a live pan gesture the whole `CanvasViewport::paint` output is
//! translation-covariant: every doc-anchored visual (nodes, grid, frame
//! labels, selection chrome) moves rigidly with the pan. So the first
//! hot frame renders the canvas region — grown by [`PAN_CACHE_MARGIN`]
//! on every side — into an offscreen layer, and every following
//! pure-pan frame blits that layer at the pan delta instead of
//! re-submitting the scene (measured ~230 ms/frame on a zoomed-out
//! 38k-node document). Any editor-state mutation, zoom change, resize,
//! or margin exhaustion drops the cache; the gesture-end frame repaints
//! at full quality and releases the layer.

use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::{Point2D, Rect};

/// Overdraw margin (logical px) rendered beyond the visible canvas on
/// every side. A pan can travel this far before content runs out.
pub(in crate::widget_host) const PAN_CACHE_MARGIN: f32 = 256.0;

/// Once the accumulated pan exceeds this, the cache scrolls in place —
/// the surviving pixels shift by a whole-device-pixel delta and only
/// the newly exposed strip repaints (culled to the strip). Strip cost
/// is dominated by the fixed per-refresh overhead (scene traversal +
/// an extra render-target submit), so refreshing less often with a
/// wider strip beats frequent narrow refreshes; 3/4 of the margin
/// still leaves headroom against a fast pan outrunning the content.
pub(in crate::widget_host) const PAN_CACHE_SCROLL_THRESHOLD: f32 = PAN_CACHE_MARGIN * 0.75;

pub(in crate::widget_host) struct CanvasPanCache {
    /// Offscreen layer holding the rendered canvas region + margins.
    pub surface: skia_safe::Surface,
    /// Viewport zoom at capture — any change invalidates.
    pub zoom: f32,
    /// Canvas region at capture — a resize / sidebar toggle invalidates.
    pub rect: Rect,
    /// Viewport pan the surface content is anchored to — the blit
    /// offset is measured from it; a scroll refresh advances it.
    pub pan: Point2D,
    /// DPI at capture — a monitor change invalidates.
    pub dpi: f32,
    /// Whether the VISIBLE region holds full-quality content (the
    /// progressive gesture-end restore repaints it tile by tile).
    /// Degraded content only ever shows mid-gesture.
    pub sharp: bool,
    /// Installed-raster generation baked into the layer. Image decodes
    /// land on a worker AFTER the frame that queued them, and they
    /// mutate no editor state, so nothing else would ever invalidate
    /// the layer — the rest-time restore watches this stamp and
    /// repaints the visible tiles when it moves.
    pub raster_generation: u64,
    /// Canvas hover id baked into the layer — rest-time blits must
    /// drop the cache when the hover changed (some hover clears skip
    /// `mark_dirty`), or the outline would freeze on screen.
    pub hovered: Option<op_editor_core::NodeId>,
}

/// Progressive gesture-end quality restore: the visible region splits
/// into horizontal tiles, one repainted at FULL quality per frame while
/// the (still mostly degraded) layer keeps blitting — no single frame
/// pays the whole-canvas repaint.
pub(in crate::widget_host) struct PanCacheRestore {
    pub tiles: Vec<Rect>,
    pub next: usize,
}

/// How many tiles the visible region splits into for the progressive
/// restore. Each tile costs roughly `full_paint / tiles` per frame.
pub(in crate::widget_host) const PAN_CACHE_RESTORE_TILES: usize = 6;

impl WidgetHostNative {
    /// Whether the current frame may serve or build the pan cache.
    /// Conservative: any time-driven canvas animation opts out so a
    /// blitted frame can never freeze it visibly.
    pub(in crate::widget_host) fn pan_cache_usable(&self) -> bool {
        self.fast_interaction_active()
            && self.preview.is_none()
            && self
                .layout_transition
                .as_ref()
                .is_none_or(|transition| !transition.is_active(self.now_ms))
            && op_editor_core::agent_indicators::snapshot_at_if_active(self.now_ms).is_none()
            && !op_editor_ui::widgets::canvas_shader_clock::animated_shader_painted()
            && self.node_drag.is_none()
            && self.marquee_drag.is_none()
            && self.editor_state.editor_ui.starter_ghost.is_none()
    }

    /// Blit offset for a cache hit: the pan delta since capture, if the
    /// cache matches this frame's viewport and stays within the margin.
    pub(in crate::widget_host) fn pan_cache_offset(&self, rect: Rect, dpi: f32) -> Option<Point2D> {
        let cache = self.pan_cache.as_ref()?;
        let viewport = &self.editor_state.viewport;
        if cache.zoom != viewport.zoom || cache.rect != rect || cache.dpi != dpi {
            return None;
        }
        let dx = viewport.pan_x - cache.pan.x;
        let dy = viewport.pan_y - cache.pan.y;
        (dx.abs() <= PAN_CACHE_MARGIN && dy.abs() <= PAN_CACHE_MARGIN)
            .then_some(Point2D::new(dx, dy))
    }

    /// Drop the cached layer (editor-state mutation etc.) and any
    /// in-flight progressive restore over it.
    pub(in crate::widget_host) fn drop_pan_cache(&mut self) {
        self.pan_cache = None;
        self.pan_cache_restore = None;
    }

    /// Store a freshly rendered layer stamped with this frame's state.
    pub(in crate::widget_host) fn store_pan_cache(
        &mut self,
        surface: skia_safe::Surface,
        rect: Rect,
        dpi: f32,
        raster_generation: u64,
    ) {
        self.pan_cache_builds = self.pan_cache_builds.wrapping_add(1);
        let hovered = self.editor_state.editor_ui.canvas_hover_node.clone();
        let viewport = &self.editor_state.viewport;
        self.pan_cache = Some(CanvasPanCache {
            surface,
            zoom: viewport.zoom,
            rect,
            pan: Point2D::new(viewport.pan_x, viewport.pan_y),
            dpi,
            // Layers are built mid-gesture in degrade mode.
            sharp: false,
            raster_generation,
            hovered,
        });
    }

    /// Approximate-zoom blit destination: when only the ZOOM differs
    /// from capture (same rect / dpi), the layer content maps onto the
    /// current frame through the affine
    /// `T(w) = rect.origin + pan1 + (w - rect.origin - pan0) * s`
    /// with `s = zoom1 / zoom0` — so a zoom tick can draw the cached
    /// bitmap scaled (Figma-style soft zoom) instead of re-rendering
    /// the scene. Beyond 5× in either direction the approximation is
    /// too blurry/pixelated; the caller falls back to a direct paint.
    pub(in crate::widget_host) fn pan_cache_zoom_dst(&self, rect: Rect, dpi: f32) -> Option<Rect> {
        let cache = self.pan_cache.as_ref()?;
        let viewport = &self.editor_state.viewport;
        if cache.rect != rect || cache.dpi != dpi || cache.zoom <= 0.0 {
            return None;
        }
        let s = viewport.zoom / cache.zoom;
        if !s.is_finite() || !(0.2..=5.0).contains(&s) {
            return None;
        }
        let m = PAN_CACHE_MARGIN;
        Some(Rect {
            origin: Point2D::new(
                rect.origin.x + viewport.pan_x + (-m - cache.pan.x) * s,
                rect.origin.y + viewport.pan_y + (-m - cache.pan.y) * s,
            ),
            size: Point2D::new((rect.size.x + m * 2.0) * s, (rect.size.y + m * 2.0) * s),
        })
    }

    /// Blit the resident cache scaled to `dst` (approximate zoom).
    pub(in crate::widget_host) fn blit_pan_cache_scaled(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        rect: Rect,
        dst: Rect,
    ) {
        let Some(cache) = self.pan_cache.as_mut() else {
            return;
        };
        let image = cache.surface.image_snapshot();
        frame.draw_offscreen_layer_to(&image, rect, dst);
        self.pan_cache_blits = self.pan_cache_blits.wrapping_add(1);
    }

    /// Blit the resident cache at `offset`. Callers have already
    /// validated the offset via [`Self::pan_cache_offset`].
    pub(in crate::widget_host) fn blit_pan_cache(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        rect: Rect,
        offset: Point2D,
    ) {
        let Some(cache) = self.pan_cache.as_mut() else {
            return;
        };
        let image = cache.surface.image_snapshot();
        frame.draw_offscreen_layer(&image, rect, PAN_CACHE_MARGIN, offset);
        self.pan_cache_blits = self.pan_cache_blits.wrapping_add(1);
    }

    /// Observability: `(blit_frames, scroll_refreshes, layer_builds)`
    /// counters — the desktop runner's slow-frame log uses the deltas
    /// to attribute a slow frame to blit / scroll / build work.
    pub fn pan_cache_stats(&self) -> (u64, u64, u64) {
        (
            self.pan_cache_blits,
            self.pan_cache_scrolls,
            self.pan_cache_builds,
        )
    }

    /// Observability: whether this frame paints in interactive-degrade
    /// mode (companion to [`Self::pan_cache_stats`] for frame logs).
    pub fn interaction_degrade_active(&self) -> bool {
        self.fast_interaction_active()
    }

    /// Test accessor: number of frames served by a cache blit.
    #[cfg(test)]
    pub(in crate::widget_host) fn pan_cache_blits_for_test(&self) -> u64 {
        self.pan_cache_blits
    }

    /// Test accessor: number of in-place scroll refreshes performed.
    #[cfg(test)]
    pub(in crate::widget_host) fn pan_cache_scrolls_for_test(&self) -> u64 {
        self.pan_cache_scrolls
    }

    /// Test accessor: number of full expanded-layer builds performed.
    #[cfg(test)]
    pub(in crate::widget_host) fn pan_cache_builds_for_test(&self) -> u64 {
        self.pan_cache_builds
    }

    /// Test accessor: whether a cached layer is currently resident.
    #[cfg(test)]
    pub(in crate::widget_host) fn pan_cache_resident_for_test(&self) -> bool {
        self.pan_cache.is_some()
    }

    /// Test accessor: whether the visible region is fully restored.
    #[cfg(test)]
    pub(in crate::widget_host) fn pan_cache_sharp_for_test(&self) -> bool {
        self.pan_cache.as_ref().is_some_and(|cache| cache.sharp)
    }

    /// Test accessor: whether a progressive restore is in flight.
    #[cfg(test)]
    pub(in crate::widget_host) fn pan_cache_restore_active_for_test(&self) -> bool {
        self.pan_cache_restore.is_some()
    }
}
