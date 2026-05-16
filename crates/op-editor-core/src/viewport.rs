//! Infinite-canvas pan + zoom state.
//!
//! Ported from `openpencil-shell-core::document::Viewport`. Pan is in
//! canvas-local px; zoom is a multiplier. `Point2D` is the crate's
//! `glam::Vec2` alias (the same one `render_backend.rs` uses), so the
//! coordinate math stays consistent across the editor-core layer.

use crate::render_backend::Point2D;

/// Pan + zoom state. Pan = canvas-local px; zoom = multiplier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl Viewport {
    /// Identity viewport — origin pan, 100% zoom.
    pub const IDENTITY: Viewport = Viewport {
        pan_x: 0.0,
        pan_y: 0.0,
        zoom: 1.0,
    };
    pub const MIN_ZOOM: f32 = 0.1;
    pub const MAX_ZOOM: f32 = 8.0;

    /// Wheel zoom anchored at `cursor` (canvas-local).
    pub fn zoom_at(&mut self, cursor: Point2D, delta: f32) {
        let prev_zoom = self.zoom;
        let factor = (delta * 0.0015).exp();
        let new_zoom = (prev_zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        // Recover the document-space point at cursor BEFORE zoom, then
        // re-anchor pan so it stays at cursor AFTER zoom.
        let doc_x = (cursor.x - self.pan_x) / prev_zoom;
        let doc_y = (cursor.y - self.pan_y) / prev_zoom;
        self.zoom = new_zoom;
        self.pan_x = cursor.x - doc_x * new_zoom;
        self.pan_y = cursor.y - doc_y * new_zoom;
    }

    /// Translate the pan origin by `(dx, dy)` canvas-local px.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.pan_x += dx;
        self.pan_y += dy;
    }

    /// Canvas-local → document space.
    pub fn to_document(&self, p: Point2D) -> Point2D {
        Point2D::new(
            (p.x - self.pan_x) / self.zoom,
            (p.y - self.pan_y) / self.zoom,
        )
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_default() {
        assert_eq!(Viewport::default(), Viewport::IDENTITY);
    }

    #[test]
    fn pan_accumulates() {
        let mut v = Viewport::IDENTITY;
        v.pan(10.0, -5.0);
        v.pan(2.0, 3.0);
        assert_eq!(v.pan_x, 12.0);
        assert_eq!(v.pan_y, -2.0);
    }

    #[test]
    fn zoom_clamps_to_range() {
        let mut v = Viewport::IDENTITY;
        v.zoom_at(Point2D::ZERO, 100_000.0);
        assert!(v.zoom <= Viewport::MAX_ZOOM);
        v.zoom_at(Point2D::ZERO, -100_000.0);
        assert!(v.zoom >= Viewport::MIN_ZOOM);
    }
}
