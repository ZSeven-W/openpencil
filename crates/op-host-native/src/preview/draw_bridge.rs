//! DrawOp → RenderBackend translation (Phase D5 crux).
//!
//! jian-core's scene walker emits `jian_core::render::DrawOp`s against
//! its OWN paint vocabulary (`scene::Color` packed-u32,
//! `geometry::{Point, Rect}` euclid-based). The OP editor canvas paints
//! through the `RenderBackend` trait (= `jian_widgets::painter::Painter`,
//! which uses `jian_widgets::geometry::{Color, Rect, Point2D}` glam-based
//! + `jian_core::render::{TextRun, TextAlign}`).
//!
//! These are different abstractions, but the OP `RenderBackend` exposes
//! a primitive set rich enough to render every `DrawOp` variant with a
//! bounded, lossless-enough mapping. This module is that translator:
//! one `paint_draw_op` per op, plus the two color/rect converters that
//! bridge the jian-core geometry to the widget geometry.
//!
//! Coordinate space: the walker lays the scene out in DOCUMENT space.
//! Callers pre-apply the editor's canvas transform (`translate` by the
//! canvas origin + pan, `scale` by zoom) on the backend before issuing
//! these ops, so this module emits doc-space coordinates verbatim.

use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

use jian_core::render::{
    BorderRadii, DrawOp, GradientStop, LinearGradient, Paint, RadialGradient, ShadowSpec, StrokeOp,
    TextRun,
};
use jian_core::scene::Color as JColor;

/// jian-core packed-RGBA `Color` → widget float `Color`.
pub(super) fn color(c: JColor) -> Color {
    Color::rgba_u8(c.r(), c.g(), c.b(), c.a() as f32 / 255.0)
}

/// jian-core euclid `Rect` → widget glam `Rect`.
pub(super) fn rect(r: jian_core::geometry::Rect) -> Rect {
    Rect {
        origin: Point2D::new(r.origin.x, r.origin.y),
        size: Point2D::new(r.size.width, r.size.height),
    }
}

/// jian-core euclid `Point` → widget glam `Point2D`.
fn point(p: jian_core::geometry::Point) -> Point2D {
    Point2D::new(p.x, p.y)
}

/// Uniform corner radius from `BorderRadii` — the OP `RenderBackend`'s
/// round-rect primitives take a single radius. Use the max of the four
/// so a pill-ish authored radius still reads as rounded; below ~0.5 px
/// the caller collapses to a plain rect anyway.
fn uniform_radius(radii: BorderRadii) -> f32 {
    radii.tl.max(radii.tr).max(radii.br).max(radii.bl)
}

/// Convert a jian-core gradient stop list into the `(offset, Color)`
/// tuples the OP gradient primitives consume.
fn stops(src: &[GradientStop]) -> Vec<(f32, Color)> {
    src.iter().map(|s| (s.offset, color(s.color))).collect()
}

/// Paint one rounded/plain rect with a solid `Paint` (fill + optional
/// stroke + opacity). Shared by `Rect` and `RoundedRect` ops.
fn paint_solid_rect(b: &mut dyn RenderBackend, r: Rect, radius: f32, paint: &Paint) {
    let rounded = radius > 0.5;
    if let Some(fill) = paint.fill {
        let c = fold(color(fill), paint.opacity);
        if rounded {
            b.fill_round_rect(r, radius, c);
        } else {
            b.fill_rect(r, c);
        }
    }
    if let Some(stroke) = &paint.stroke {
        let c = fold(color(stroke.color), paint.opacity);
        if rounded {
            b.stroke_round_rect(r, radius, c, stroke.width);
        } else {
            b.stroke_rect(r, c, stroke.width);
        }
    }
}

/// Multiply a color's alpha by a paint-level `opacity`.
fn fold(c: Color, opacity: f32) -> Color {
    Color {
        a: c.a * opacity.clamp(0.0, 1.0),
        ..c
    }
}

/// Translate a single `DrawOp` onto the OP `RenderBackend`. Coordinates
/// are in document space; the caller has already pushed the canvas
/// transform onto the backend.
pub(super) fn paint_draw_op(b: &mut dyn RenderBackend, op: &DrawOp) {
    match op {
        DrawOp::Rect { rect: r, paint } => {
            paint_solid_rect(b, rect(*r), 0.0, paint);
        }
        DrawOp::RoundedRect {
            rect: r,
            radii,
            paint,
        } => {
            paint_solid_rect(b, rect(*r), uniform_radius(*radii), paint);
        }
        DrawOp::Path { commands, paint } => {
            paint_path(b, commands, paint);
        }
        DrawOp::Text(run) => {
            paint_text(b, run);
        }
        DrawOp::LinearGradientRect {
            rect: r,
            radii,
            gradient,
            stroke,
        } => {
            paint_linear_gradient(
                b,
                rect(*r),
                uniform_radius(*radii),
                gradient,
                stroke.as_ref(),
            );
        }
        DrawOp::RadialGradientRect {
            rect: r,
            radii,
            gradient,
            stroke,
        } => {
            paint_radial_gradient(
                b,
                rect(*r),
                uniform_radius(*radii),
                gradient,
                stroke.as_ref(),
            );
        }
        DrawOp::ShadowedRect {
            rect: r,
            radii,
            shadow,
        } => {
            paint_shadow(b, rect(*r), uniform_radius(*radii), shadow);
        }
        DrawOp::Icon {
            rect: r,
            name,
            family,
            color: c,
        } => {
            // Reuse the editor's lucide d-string catalog so icon nodes
            // render with the same glyphs as the design surface. The
            // helper takes a `&mut dyn RenderBackend`, so reborrow `b`.
            op_editor_ui::widgets::icons::paint_icon_font_node(
                b,
                family.as_deref().unwrap_or(""),
                name,
                rect(*r),
                Some(color(*c)),
            );
        }
        DrawOp::Image { dst, opacity, .. } => {
            // The runtime preview does not resolve image bytes (the host
            // image cache lives on the editor paint path, not the
            // runtime). Draw a neutral placeholder so layout still reads
            // — same degrade the .op loader uses for unresolved images.
            let r = rect(*dst);
            b.fill_round_rect(
                r,
                0.0,
                Color::rgba_u8(120, 120, 120, 0.35 * opacity.clamp(0.0, 1.0)),
            );
        }
    }
}

fn paint_path(
    b: &mut dyn RenderBackend,
    commands: &[jian_core::render::PathCommand],
    paint: &Paint,
) {
    use jian_core::render::PathCommand as P;
    // The OP `RenderBackend` has no generic path-fill primitive that
    // accepts `PathCommand`s; approximate by stroking the polyline
    // through the anchor points (curves are flattened to their end
    // points). This matches how the editor canvas paints `Path` nodes
    // (polyline through `node.points`).
    let mut pts: Vec<Point2D> = Vec::with_capacity(commands.len());
    for cmd in commands {
        match cmd {
            P::MoveTo(p) | P::LineTo(p) => pts.push(point(*p)),
            P::QuadTo(_, end) => pts.push(point(*end)),
            P::CubicTo(_, _, end) => pts.push(point(*end)),
            P::Close => {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
        }
    }
    if pts.len() < 2 {
        return;
    }
    let stroke = paint.stroke.as_ref();
    let (sc, sw) = match stroke {
        Some(StrokeOp { color: c, width }) => (fold(color(*c), paint.opacity), *width),
        None => match paint.fill {
            // No stroke authored — outline with the fill color so the
            // path is at least visible.
            Some(f) => (fold(color(f), paint.opacity), 1.0),
            None => return,
        },
    };
    for win in pts.windows(2) {
        b.stroke_line(win[0], win[1], sc, sw);
    }
}

fn paint_text(b: &mut dyn RenderBackend, run: &TextRun) {
    // `TextRun` is shared verbatim between jian-core and the OP painter,
    // so a `TextLayout` wraps it directly. The painter derives the
    // baseline + alignment from `run.origin` / `run.max_width` / `align`.
    let layout = TextLayout::single_run(
        &run.content,
        &run.font_family,
        run.font_size,
        run.color,
        Point2D::new(run.origin.x, run.origin.y),
    )
    .with_font_weight(run.font_weight);
    b.draw_text(&layout, Point2D::new(run.origin.x, run.origin.y));
}

fn paint_linear_gradient(
    b: &mut dyn RenderBackend,
    r: Rect,
    radius: f32,
    g: &LinearGradient,
    stroke: Option<&StrokeOp>,
) {
    b.fill_round_rect_linear_gradient(r, radius, &stops(&g.stops), g.angle_deg, g.opacity);
    if let Some(s) = stroke {
        if radius > 0.5 {
            b.stroke_round_rect(r, radius, color(s.color), s.width);
        } else {
            b.stroke_rect(r, color(s.color), s.width);
        }
    }
}

fn paint_radial_gradient(
    b: &mut dyn RenderBackend,
    r: Rect,
    radius: f32,
    g: &RadialGradient,
    stroke: Option<&StrokeOp>,
) {
    b.fill_round_rect_radial_gradient(r, radius, &stops(&g.stops), g.cx, g.cy, g.radius, g.opacity);
    if let Some(s) = stroke {
        if radius > 0.5 {
            b.stroke_round_rect(r, radius, color(s.color), s.width);
        } else {
            b.stroke_rect(r, color(s.color), s.width);
        }
    }
}

fn paint_shadow(b: &mut dyn RenderBackend, r: Rect, radius: f32, shadow: &ShadowSpec) {
    // Offset the shadow rect by (dx, dy) and grow it by `spread`, then
    // ask the backend's drop-shadow primitive to draw it under the
    // node. The paint layer on top is emitted as a separate DrawOp by
    // the walker, so here we only lay the blurred shadow.
    let shadow_rect = Rect {
        origin: Point2D::new(
            r.origin.x + shadow.dx - shadow.spread,
            r.origin.y + shadow.dy - shadow.spread,
        ),
        size: Point2D::new(
            r.size.x + shadow.spread * 2.0,
            r.size.y + shadow.spread * 2.0,
        ),
    };
    b.fill_drop_shadow(shadow_rect, radius, shadow.blur, color(shadow.color));
}
