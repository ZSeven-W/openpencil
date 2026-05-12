//! Selection overlay + fill/stroke node-paint helpers split out
//! of `canvas_viewport.rs` to keep that file under the 800-line
//! ceiling.

use crate::document::{Document, Node, Viewport};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

/// Pen-tool rubber-band — from the last committed anchor of the
/// in-progress path to the cursor (when both are known). Painted
/// half-alpha so it reads as a draft segment.
pub fn paint_pen_rubber_band(
    cx: &mut PaintCx<'_>,
    doc: &Document,
    canvas_rect: Rect,
    viewport: &Viewport,
) {
    let (Some(pen_id), Some(cursor_doc)) = (doc.ui.pen_in_progress, doc.ui.pen_cursor_doc) else {
        return;
    };
    let Some(page) = doc.active_page() else {
        return;
    };
    let Some(node) = page.find(pen_id) else {
        return;
    };
    let Some(last) = node.points.last().copied() else {
        return;
    };
    let zoom = viewport.zoom;
    let origin = Point2D::new(
        canvas_rect.origin.x + viewport.pan_x,
        canvas_rect.origin.y + viewport.pan_y,
    );
    let from = Point2D::new(origin.x + last.x * zoom, origin.y + last.y * zoom);
    let to = Point2D::new(
        origin.x + cursor_doc.x * zoom,
        origin.y + cursor_doc.y * zoom,
    );
    let width = node
        .stroke
        .map(|s| (s.width * zoom).max(1.0))
        .unwrap_or((1.5_f32).max(zoom));
    let mut color = node.stroke.map(|s| s.color).unwrap_or(Color::BLACK);
    color.a *= 0.5;
    cx.backend.stroke_line(from, to, color, width);
}

/// Paint a 1 px primary-tinted outline around `world_rect` plus
/// 8 handle anchors. Container selections paint with a softened
/// outline alpha so child content stays visually dominant. When
/// `show_handles` is false the outline paints without the 8 grab
/// dots — used for multi-select where only one bounding box per
/// node is shown (Figma parity; handles only on single-select).
pub fn paint_selection_overlay(
    cx: &mut PaintCx<'_>,
    world_rect: Rect,
    theme: &Theme,
    is_container: bool,
    show_handles: bool,
) {
    // Outline — draw as 4 individual stroke_line calls so the
    // skia AA path runs (stroke_rect goes through jian which
    // doesn't enable AA by default).
    let left = world_rect.origin.x;
    let right = world_rect.origin.x + world_rect.size.x;
    let top = world_rect.origin.y;
    let bottom = world_rect.origin.y + world_rect.size.y;
    let stroke_w = 1.0;
    let outline_color = if is_container {
        crate::Color {
            r: theme.primary.r,
            g: theme.primary.g,
            b: theme.primary.b,
            a: theme.primary.a * 0.55,
        }
    } else {
        theme.primary
    };
    cx.backend.stroke_line(
        Point2D::new(left, top),
        Point2D::new(right, top),
        outline_color,
        stroke_w,
    );
    cx.backend.stroke_line(
        Point2D::new(right, top),
        Point2D::new(right, bottom),
        outline_color,
        stroke_w,
    );
    cx.backend.stroke_line(
        Point2D::new(right, bottom),
        Point2D::new(left, bottom),
        outline_color,
        stroke_w,
    );
    cx.backend.stroke_line(
        Point2D::new(left, bottom),
        Point2D::new(left, top),
        outline_color,
        stroke_w,
    );

    if !show_handles {
        return;
    }

    // 8 handle anchors.
    let mid_x = world_rect.origin.x + world_rect.size.x / 2.0;
    let mid_y = world_rect.origin.y + world_rect.size.y / 2.0;
    let anchors = [
        (left, top),
        (mid_x, top),
        (right, top),
        (right, mid_y),
        (right, bottom),
        (mid_x, bottom),
        (left, bottom),
        (left, mid_y),
    ];

    let handle_size = 7.0;
    let half = handle_size / 2.0;
    let radius = 1.0;
    let bg = crate::Color::WHITE;
    for (x, y) in anchors {
        let rect = Rect {
            origin: Point2D::new(x - half, y - half),
            size: Point2D::new(handle_size, handle_size),
        };
        cx.backend.fill_round_rect(rect, radius, bg);
        cx.backend
            .stroke_round_rect(rect, radius, theme.primary, 1.0);
    }
}

/// Paint a node's fill rect followed by its stroke rect. Stroke
/// width is scaled by `zoom` so it stays visually constant under
/// canvas zoom.
pub fn paint_fill_then_stroke(cx: &mut PaintCx<'_>, node: &Node, world_rect: Rect, zoom: f32) {
    // Scale doc-space radius into world-space alongside the rect.
    // 0.5px is below the visible threshold for most renders; collapse
    // to a square fill so the round-rect path doesn't accidentally
    // soften 0-radius corners due to sub-pixel rounding.
    let r = node.corner_radius * zoom;
    let use_round = r > 0.5;
    if let Some(fill) = node.fill {
        if use_round {
            cx.backend.fill_round_rect(world_rect, r, fill);
        } else {
            cx.backend.fill_rect(world_rect, fill);
        }
    }
    if let Some(stroke) = node.stroke {
        if use_round {
            cx.backend
                .stroke_round_rect(world_rect, r, stroke.color, stroke.width * zoom);
        } else {
            cx.backend
                .stroke_rect(world_rect, stroke.color, stroke.width * zoom);
        }
    }
}
