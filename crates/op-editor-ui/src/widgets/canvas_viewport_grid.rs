//! Dotted canvas grid painter for [`super::canvas_viewport::CanvasViewport`].
//!
//! Kept separate from the viewport widget so the hot grid path can
//! evolve without pushing `canvas_viewport.rs` over the 800-line cap.

use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::Viewport as DocViewport;

const GRID_SPACING: f32 = 32.0;

#[derive(Clone, Copy)]
struct GridMetrics {
    step: f32,
    origin_x: f32,
    origin_y: f32,
    dot_radius: f32,
    color: Color,
}

/// Paint a dotted grid across the canvas widget rect. The grid is
/// drawn in canvas-local coordinates and offset by `viewport.pan`
/// so it scrolls with the document content. Dots get sparser as
/// zoom decreases (skipping every other dot at low zoom) so they
/// stay visually airy.
pub(super) fn paint_grid(cx: &mut PaintCx<'_>, rect: Rect, viewport: &DocViewport, theme: &Theme) {
    let metrics = grid_metrics(rect, viewport, theme);
    let count = grid_dot_count_from_metrics(rect, metrics);
    let mut centers = Vec::with_capacity(count);
    let mut y = metrics.origin_y - metrics.step;
    while y < rect.origin.y + rect.size.y + metrics.step {
        let mut x = metrics.origin_x - metrics.step;
        while x < rect.origin.x + rect.size.x + metrics.step {
            centers.push(Point2D::new(x, y));
            x += metrics.step;
        }
        y += metrics.step;
    }
    cx.backend
        .fill_dots(&centers, metrics.dot_radius, metrics.color);
}

#[cfg(test)]
pub(crate) fn grid_dot_count(rect: Rect, viewport: &DocViewport) -> usize {
    grid_dot_count_from_metrics(rect, grid_metrics(rect, viewport, &Theme::light()))
}

fn grid_dot_count_from_metrics(rect: Rect, metrics: GridMetrics) -> usize {
    let cols = axis_dot_count(
        metrics.origin_x - metrics.step,
        rect.origin.x + rect.size.x + metrics.step,
        metrics.step,
    );
    let rows = axis_dot_count(
        metrics.origin_y - metrics.step,
        rect.origin.y + rect.size.y + metrics.step,
        metrics.step,
    );
    cols.saturating_mul(rows)
}

fn axis_dot_count(first: f32, limit: f32, step: f32) -> usize {
    if step <= 0.0 || limit <= first {
        return 0;
    }
    ((limit - first) / step).ceil() as usize
}

fn grid_metrics(rect: Rect, viewport: &DocViewport, theme: &Theme) -> GridMetrics {
    let zoom = viewport.zoom.max(0.0001);
    let mut step = GRID_SPACING * zoom;
    while step < 8.0 {
        step *= 2.0;
    }
    let dot_size = (1.5 * zoom.sqrt()).clamp(1.0, 2.5);
    GridMetrics {
        step,
        origin_x: rect.origin.x + viewport.pan_x.rem_euclid(step),
        origin_y: rect.origin.y + viewport.pan_y.rem_euclid(step),
        dot_radius: dot_size / 2.0,
        color: Color {
            r: theme.muted_foreground.r,
            g: theme.muted_foreground.g,
            b: theme.muted_foreground.b,
            a: 0.18,
        },
    }
}
