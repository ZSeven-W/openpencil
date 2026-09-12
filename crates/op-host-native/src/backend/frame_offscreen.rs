//! Offscreen-layer helpers on `NativeFrameBackend` — the render-to-texture
//! path the compositing / mask passes use.
//!
//! Split out of `frame_backend.rs` (which is one long `impl RenderBackend`
//! that cannot itself be divided) to keep every file under the repo's
//! 800-line cap.

use super::frame_backend::*;
use super::NativeBackend;
use op_editor_ui::{Point2D, Rect};

impl<'a> NativeFrameBackend<'a> {
    pub fn new(inner: &'a mut NativeBackend, canvas: &'a skia_safe::Canvas) -> Self {
        Self { inner, canvas }
    }

    /// Generation of the backend's installed-raster set. A cached
    /// composition stamps this so a decode landing after the capture
    /// (blur-up placeholder → full-resolution image) is not served
    /// stale forever.
    pub fn raster_generation(&self) -> u64 {
        self.inner.raster_generation()
    }

    /// Render `f` into an offscreen surface covering `rect` grown by
    /// `margin` logical px on every side, and return the surface.
    /// The surface is allocated compatible with this frame's canvas
    /// (GPU-backed under GL, raster in tests). The closure paints in
    /// the same logical coordinate space as the window: logical point
    /// `rect.origin - margin` lands on the surface's top-left pixel.
    pub fn render_offscreen_layer(
        &mut self,
        rect: Rect,
        margin: f32,
        f: impl FnOnce(&mut NativeFrameBackend<'_>),
    ) -> Option<skia_safe::Surface> {
        let dpi = self.inner.dpi_scale();
        let w = ((rect.size.x + margin * 2.0) * dpi).ceil() as i32;
        let h = ((rect.size.y + margin * 2.0) * dpi).ceil() as i32;
        if w <= 0 || h <= 0 {
            return None;
        }
        let info = skia_safe::ImageInfo::new_n32_premul((w, h), None);
        let mut surface = self.canvas.new_surface(&info, None)?;
        surface.canvas().clear(skia_safe::Color::TRANSPARENT);
        self.paint_offscreen_region(&mut surface, rect, margin, None, f);
        Some(surface)
    }

    /// Paint `f` into an existing offscreen layer produced by
    /// [`Self::render_offscreen_layer`], optionally clipped to a
    /// logical `clip` rect. Uses the same logical mapping as the
    /// original render (logical `rect.origin - margin` = pixel 0,0).
    pub fn paint_offscreen_region(
        &mut self,
        surface: &mut skia_safe::Surface,
        rect: Rect,
        margin: f32,
        clip: Option<Rect>,
        f: impl FnOnce(&mut NativeFrameBackend<'_>),
    ) {
        let dpi = self.inner.dpi_scale();
        let offscreen = surface.canvas();
        let save = offscreen.save();
        offscreen.reset_matrix();
        offscreen.scale((dpi, dpi));
        offscreen.translate((-(rect.origin.x - margin), -(rect.origin.y - margin)));
        if let Some(clip) = clip {
            offscreen.clip_rect(
                skia_safe::Rect::from_xywh(clip.origin.x, clip.origin.y, clip.size.x, clip.size.y),
                None,
                Some(true),
            );
        }
        {
            let mut sub = NativeFrameBackend::new(self.inner, offscreen);
            f(&mut sub);
        }
        offscreen.restore_to_count(save);
    }

    /// Blit an offscreen layer produced by
    /// [`Self::render_offscreen_layer`] back onto the window canvas,
    /// clipped to `rect`, translated by `offset` logical px. With the
    /// same `rect`/`margin` and a zero offset this reproduces the
    /// captured pixels 1:1 (the destination logical size times the
    /// DPI scale equals the image's physical size).
    pub fn draw_offscreen_layer(
        &mut self,
        image: &skia_safe::Image,
        rect: Rect,
        margin: f32,
        offset: Point2D,
    ) {
        let dst = Rect {
            origin: Point2D::new(
                rect.origin.x - margin + offset.x,
                rect.origin.y - margin + offset.y,
            ),
            size: Point2D::new(rect.size.x + margin * 2.0, rect.size.y + margin * 2.0),
        };
        self.draw_offscreen_layer_to(image, rect, dst);
    }

    /// Blit an offscreen layer to an arbitrary destination rect
    /// (clipped to `clip`). The pan cache's approximate-zoom path uses
    /// a scaled destination — Figma-style soft zoom until the gesture
    /// ends and the sharp repaint lands.
    pub fn draw_offscreen_layer_to(&mut self, image: &skia_safe::Image, clip: Rect, dst: Rect) {
        self.draw_offscreen_layer_to_with_alpha(image, clip, dst, 1.0);
    }

    /// Alpha-blit variant — the result view's boards fade in over their
    /// entrance window, so their rasters must composite at the same
    /// phase the widget's placeholder slots paint at.
    pub fn draw_offscreen_layer_to_with_alpha(
        &mut self,
        image: &skia_safe::Image,
        clip: Rect,
        dst: Rect,
        alpha: f32,
    ) {
        let save = self.canvas.save();
        self.canvas.clip_rect(
            skia_safe::Rect::from_xywh(clip.origin.x, clip.origin.y, clip.size.x, clip.size.y),
            None,
            Some(true),
        );
        let sampling = skia_safe::SamplingOptions::new(
            skia_safe::FilterMode::Linear,
            skia_safe::MipmapMode::None,
        );
        let mut paint = skia_safe::Paint::default();
        paint.set_alpha_f(alpha.clamp(0.0, 1.0));
        self.canvas.draw_image_rect_with_sampling_options(
            image,
            None,
            skia_safe::Rect::from_xywh(dst.origin.x, dst.origin.y, dst.size.x, dst.size.y),
            sampling,
            &paint,
        );
        self.canvas.restore_to_count(save);
    }
}
