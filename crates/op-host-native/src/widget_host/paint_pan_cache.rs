//! Pan-bitmap-cache fast paths for the native paint pass.
//!
//! Split out of `paint.rs` to honour the 800-line cap. These three
//! helpers are the only places allowed to short-circuit the full
//! composition pass: a pure-pan frame blits the resident offscreen
//! layer, a long pan scrolls it in place and repaints only the exposed
//! strips, and the gesture-end restore re-renders one tile per frame
//! until the visible region is sharp again. All three are native-only
//! (the browser has no offscreen GL layer of its own).

use super::WidgetHostNative;
use crate::backend::NativeFrameBackend;
use op_editor_ui::widgets::{CanvasViewport, PaintCx, Widget};
use op_editor_ui::{Point2D, Rect, RenderBackend};

/// The layer-edge strips a shift of `shift` logical px scrolls into
/// view, in the layer's logical space. Shifting content right (`x > 0`)
/// exposes the left edge and vice versa; the vertical strip runs full
/// height and the horizontal strip full width so a diagonal shift's
/// L-shape is fully covered. Each extent is rounded UP to a whole
/// device pixel: the layer copy lands on the pixel grid, so a
/// fractional shift would otherwise leave a one-pixel seam of stale
/// content at the edge.
fn exposed_strips(expanded: Rect, shift: Point2D, dpi: f32) -> Vec<Rect> {
    let extent = |v: f32| (v.abs() * dpi).ceil() / dpi;
    let mut strips = Vec::new();
    let sx = extent(shift.x);
    if sx > 0.0 {
        let x = if shift.x > 0.0 {
            expanded.origin.x
        } else {
            expanded.origin.x + expanded.size.x - sx
        };
        strips.push(Rect {
            origin: Point2D::new(x, expanded.origin.y),
            size: Point2D::new(sx, expanded.size.y),
        });
    }
    let sy = extent(shift.y);
    if sy > 0.0 {
        let y = if shift.y > 0.0 {
            expanded.origin.y
        } else {
            expanded.origin.y + expanded.size.y - sy
        };
        strips.push(Rect {
            origin: Point2D::new(expanded.origin.x, y),
            size: Point2D::new(expanded.size.x, sy),
        });
    }
    strips
}

impl WidgetHostNative {
    /// Serve the canvas region from the pan bitmap cache: a pure-pan
    /// frame blits the resident layer; past the scroll threshold the
    /// layer first scrolls in place and repaints only the exposed
    /// strips (so a long pan never pays a full re-render hitch).
    /// Returns whether the frame was painted here.
    pub(in crate::widget_host) fn serve_canvas_from_pan_cache(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        canvas_rect: Rect,
    ) -> bool {
        use super::canvas_pan_cache::PAN_CACHE_SCROLL_THRESHOLD;
        if !self.fast_interaction_active() {
            // Gesture over: restore quality progressively (one tile per
            // frame) while the layer keeps blitting, then keep serving
            // the sharp layer until any invalidation. The layer is KEPT
            // across gestures so the next pan skips the expanded rebuild.
            return self.serve_rest_restore(frame, canvas_rect);
        }
        // A live gesture supersedes any in-flight restore.
        self.pan_cache_restore = None;
        if !self.pan_cache_usable() {
            self.drop_pan_cache();
            return false;
        }
        let dpi = frame.dpi_scale();
        let Some(offset) = self.pan_cache_offset(canvas_rect, dpi) else {
            // Zoom gesture with a retained layer: serve an approximate
            // SCALED blit (soft zoom); the gesture-end repaint restores
            // sharpness. Only zoom gestures take this path — a pan
            // whose cache mismatches must rebuild sharp content.
            if self.last_gesture_was_zoom {
                if let Some(dst) = self.pan_cache_zoom_dst(canvas_rect, dpi) {
                    self.blit_pan_cache_scaled(frame, canvas_rect, dst);
                    return true;
                }
            }
            return false;
        };
        let offset = if offset.x.abs().max(offset.y.abs()) > PAN_CACHE_SCROLL_THRESHOLD {
            match self.scroll_refresh_pan_cache(frame, canvas_rect, dpi) {
                Some(residual) => residual,
                None => return false,
            }
        } else {
            offset
        };
        self.blit_pan_cache(frame, canvas_rect, offset);
        true
    }

    /// Scroll the cached layer by the whole-device-pixel part of the
    /// accumulated pan and repaint ONLY the strips scrolling exposed
    /// (nodes culled to each strip — a fraction of a full frame).
    /// Returns the residual sub-pixel blit offset.
    fn scroll_refresh_pan_cache(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        canvas_rect: Rect,
        dpi: f32,
    ) -> Option<Point2D> {
        use super::canvas_pan_cache::PAN_CACHE_MARGIN as M;
        let mut cache = self.pan_cache.take()?;
        let snap = |v: f32| (v * dpi).round() / dpi;
        let dx = self.editor_state.viewport.pan_x - cache.pan.x;
        let dy = self.editor_state.viewport.pan_y - cache.pan.y;
        // Refresh only the DOMINANT axis: one strip = one scene
        // traversal + one render-target pass per refresh. A diagonal
        // fling whose minor axis also crosses the threshold simply
        // refreshes it on the next frame — same total work, half the
        // single-frame budget.
        let (sdx, sdy) = if dx.abs() >= dy.abs() {
            (snap(dx), 0.0)
        } else {
            (0.0, snap(dy))
        };
        // 1. Shift the surviving pixels by a whole device-pixel delta —
        //    a lossless copy, so repeated refreshes never accumulate
        //    resampling blur.
        let snapshot = cache.surface.image_snapshot();
        {
            let offscreen = cache.surface.canvas();
            let save = offscreen.save();
            offscreen.reset_matrix();
            let mut paint = skia_safe::Paint::default();
            paint.set_blend_mode(skia_safe::BlendMode::Src);
            offscreen.draw_image(&snapshot, (sdx * dpi, sdy * dpi), Some(&paint));
            offscreen.restore_to_count(save);
        }
        cache.pan = Point2D::new(cache.pan.x + sdx, cache.pan.y + sdy);
        // The layer stays anchored at `cache.pan`, which the snap and
        // the dominant-axis-only refresh leave BEHIND the live viewport
        // pan: sub-device-pixel on the refreshed axis, but the whole
        // untouched delta on the other one. The blit below translates
        // the entire layer by that residual, so strips must be painted
        // as if the pan were `cache.pan` — painting them at the live
        // pan lands them at twice the residual, a mis-registered band
        // up to the full margin wide on a diagonal gesture.
        let residual = Point2D::new(dx - sdx, dy - sdy);
        // 2. Repaint only the exposed strips. Shifting content right
        //    (sdx > 0) exposes the left edge, and vice versa; the
        //    vertical strip runs full height and the horizontal strip
        //    full width so a diagonal pan's L-shape is fully covered.
        let expanded = Rect {
            origin: Point2D::new(canvas_rect.origin.x - M, canvas_rect.origin.y - M),
            size: Point2D::new(canvas_rect.size.x + M * 2.0, canvas_rect.size.y + M * 2.0),
        };
        let strips = exposed_strips(expanded, Point2D::new(sdx, sdy), dpi);
        if !strips.is_empty() {
            let mut canvas = CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
            canvas.now_ms = self.now_ms;
            canvas.fast_interaction = true;
            canvas.offset_paint_origin(M - residual.x, M - residual.y);
            for strip in strips {
                canvas.cull_override = Some(strip);
                frame.paint_offscreen_region(
                    &mut cache.surface,
                    canvas_rect,
                    M,
                    Some(strip),
                    |off| {
                        let mut cx = PaintCx { backend: off };
                        canvas.paint(&mut cx, expanded);
                    },
                );
            }
        }
        self.pan_cache_scrolls = self.pan_cache_scrolls.wrapping_add(1);
        // Strips paint in degrade mode — the layer needs a fresh
        // progressive restore once the gesture ends.
        cache.sharp = false;
        self.pan_cache = Some(cache);
        Some(residual)
    }

    /// Rest-time serve: progressively restore the visible region to
    /// full quality (one tile per frame) into the retained layer, then
    /// keep blitting the sharp layer until any invalidation. Returns
    /// whether this frame was painted here.
    fn serve_rest_restore(
        &mut self,
        frame: &mut NativeFrameBackend<'_>,
        canvas_rect: Rect,
    ) -> bool {
        use super::canvas_pan_cache::{PAN_CACHE_MARGIN as M, PAN_CACHE_RESTORE_TILES};
        // Rest blits must never freeze time-driven canvas visuals.
        let rest_usable = self.preview.is_none()
            && self
                .layout_transition
                .as_ref()
                .is_none_or(|transition| !transition.is_active(self.now_ms))
            && op_editor_core::agent_indicators::snapshot_at_if_active(self.now_ms).is_none()
            && self.node_drag.is_none()
            && self.marquee_drag.is_none()
            && self.editor_state.editor_ui.starter_ghost.is_none()
            && self.editor_state.ui.text_editing.is_none()
            && !op_editor_ui::shader_clock::animated_shader_on_screen();
        if !rest_usable {
            self.pan_cache_restore = None;
            return false;
        }
        let dpi = frame.dpi_scale();
        let raster_generation = frame.raster_generation();
        let hovered_now = self.editor_state.editor_ui.canvas_hover_node.clone();
        {
            let Some(cache) = self.pan_cache.as_ref() else {
                self.pan_cache_restore = None;
                return false;
            };
            // Hover is baked into the layer, and some hover clears skip
            // `mark_dirty` — a stamp mismatch must repaint direct.
            if cache.hovered != hovered_now {
                self.drop_pan_cache();
                return false;
            }
        }
        // Image decodes land on a worker after the frame that queued
        // them and touch no editor state, so no `mark_dirty` fires: a
        // sharp layer would keep serving the blur-up placeholder until
        // something unrelated invalidated it. Restart the progressive
        // restore instead of dropping the layer, so the fix costs a few
        // rest frames rather than a full rebuild — and costs a live
        // gesture nothing at all, which is the point of the cache.
        if let Some(cache) = self.pan_cache.as_mut() {
            if cache.raster_generation != raster_generation {
                cache.raster_generation = raster_generation;
                cache.sharp = false;
                self.pan_cache_restore = None;
            }
        }
        let Some(offset) = self.pan_cache_offset(canvas_rect, dpi) else {
            self.pan_cache_restore = None;
            return false;
        };
        if self.pan_cache.as_ref().is_some_and(|cache| cache.sharp)
            && self.pan_cache_restore.is_none()
        {
            self.blit_pan_cache(frame, canvas_rect, offset);
            return true;
        }
        // Start the restore: rebase the layer onto the exact current
        // pan (one resampled shift of the degraded content) so tiles
        // repaint at identity alignment and the final blit is 1:1.
        if self.pan_cache_restore.is_none() {
            if offset.x != 0.0 || offset.y != 0.0 {
                if let Some(cache) = self.pan_cache.as_mut() {
                    let snapshot = cache.surface.image_snapshot();
                    let offscreen = cache.surface.canvas();
                    let save = offscreen.save();
                    offscreen.reset_matrix();
                    let mut paint = skia_safe::Paint::default();
                    paint.set_blend_mode(skia_safe::BlendMode::Src);
                    offscreen.draw_image(&snapshot, (offset.x * dpi, offset.y * dpi), Some(&paint));
                    offscreen.restore_to_count(save);
                    cache.pan = Point2D::new(
                        self.editor_state.viewport.pan_x,
                        self.editor_state.viewport.pan_y,
                    );
                }
            }
            let tile_h = (canvas_rect.size.y / PAN_CACHE_RESTORE_TILES as f32).ceil();
            let mut tiles: Vec<Rect> = (0..PAN_CACHE_RESTORE_TILES)
                .map(|i| Rect {
                    origin: Point2D::new(
                        canvas_rect.origin.x,
                        canvas_rect.origin.y + i as f32 * tile_h,
                    ),
                    size: Point2D::new(canvas_rect.size.x, tile_h),
                })
                .collect();
            // The rebase shift vacates a strip at the layer edge that
            // the visible-region tiles never reach. It sits in the
            // margin, so nothing shows until the NEXT gesture scrolls
            // it back on screen — as a band still registered to the
            // pre-rebase anchor. Restoring it as a trailing tile keeps
            // the whole layer, margin included, one coherent image.
            let expanded = Rect {
                origin: Point2D::new(canvas_rect.origin.x - M, canvas_rect.origin.y - M),
                size: Point2D::new(canvas_rect.size.x + M * 2.0, canvas_rect.size.y + M * 2.0),
            };
            tiles.extend(exposed_strips(expanded, offset, dpi));
            self.pan_cache_restore =
                Some(super::canvas_pan_cache::PanCacheRestore { tiles, next: 0 });
        }
        // Repaint the next tile at FULL quality into the layer.
        if let Some(restore) = self.pan_cache_restore.take() {
            let mut restore = restore;
            if let (Some(tile), Some(mut cache)) = (
                restore.tiles.get(restore.next).copied(),
                self.pan_cache.take(),
            ) {
                let expanded = Rect {
                    origin: Point2D::new(canvas_rect.origin.x - M, canvas_rect.origin.y - M),
                    size: Point2D::new(canvas_rect.size.x + M * 2.0, canvas_rect.size.y + M * 2.0),
                };
                {
                    let mut canvas =
                        CanvasViewport::from_editor(&self.editor_state, &self.layout_scene);
                    canvas.now_ms = self.now_ms;
                    canvas.fast_interaction = false;
                    canvas.offset_paint_origin(M, M);
                    canvas.cull_override = Some(tile);
                    frame.paint_offscreen_region(
                        &mut cache.surface,
                        canvas_rect,
                        M,
                        Some(tile),
                        |off| {
                            let mut cx = PaintCx { backend: off };
                            canvas.paint(&mut cx, expanded);
                        },
                    );
                }
                restore.next += 1;
                if restore.next >= restore.tiles.len() {
                    cache.sharp = true;
                    self.pan_cache = Some(cache);
                } else {
                    self.pan_cache = Some(cache);
                    self.pan_cache_restore = Some(restore);
                }
            }
        }
        self.blit_pan_cache(frame, canvas_rect, Point2D::new(0.0, 0.0));
        true
    }
}
