//! Per-kind node painter for [`super::canvas_viewport::CanvasViewport`].
//!
//! Walks a [`LayoutScene`](crate::layout_scene::LayoutScene)
//! [`SceneNode`] tree and reproduces the canvas pixel-for-pixel:
//! per-kind paint (Frame / Group / Rect / Ellipse / Polygon / Line /
//! Path / Text / `icon_font`), per-node rotation, corner radius,
//! pre-resolved fills / strokes, drop-shadow effects, viewport culling
//! and CJK-aware text wrap.
//!
//! Split out of `canvas_viewport.rs` to keep that file under the
//! 800-line ceiling. The scene's geometry is already layout-resolved
//! and its fills are already `$ref`-resolved, so this painter applies
//! only the viewport transform — no second layout pass, no variable
//! lookup.

use crate::layout_scene::{regular_polygon_points, SceneNode};
use crate::layout_scene::{Effect, NodeKind};
use crate::widgets::canvas_viewport::EditCaret;
use crate::widgets::canvas_viewport_overlay::{paint_fill_then_stroke, wrap_text};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// Paint every `Effect::DropShadow` on `node` as a blurred shape
/// behind its fill. The shadow corner radius matches the node
/// kind — `corner_radius` for Frame / Rect, min-half for an
/// ellipse silhouette. Offset + blur scale by `zoom` so the
/// shadow tracks the node across viewport zoom.
fn paint_drop_shadows(cx: &mut PaintCx<'_>, node: &SceneNode, world_rect: Rect, zoom: f32) {
    let radius = if node.kind == NodeKind::Ellipse {
        world_rect.size.x.min(world_rect.size.y) / 2.0
    } else {
        node.corner_radius * zoom
    };
    for effect in &node.effects {
        let Effect::DropShadow(s) = effect;
        let shadow_rect = Rect {
            origin: Point2D::new(
                world_rect.origin.x + s.offset_x * zoom,
                world_rect.origin.y + s.offset_y * zoom,
            ),
            size: world_rect.size,
        };
        cx.backend
            .fill_drop_shadow(shadow_rect, radius, s.blur * zoom, s.color);
    }
}

/// Tessellate an ellipse arc / pie / donut-sector into a closed
/// polygon outline. `start_deg` / `sweep_deg` use the screen
/// convention (0° = +X, positive = clockwise); `inner` is the
/// donut-hole radius as a 0.0..=1.0 fraction.
pub(crate) fn arc_polygon(rect: Rect, start_deg: f32, sweep_deg: f32, inner: f32) -> Vec<Point2D> {
    let cx_pt = rect.origin.x + rect.size.x / 2.0;
    let cy_pt = rect.origin.y + rect.size.y / 2.0;
    let rx = rect.size.x / 2.0;
    let ry = rect.size.y / 2.0;
    // ~1 segment per 4° of sweep, clamped to a sane range.
    let segs = ((sweep_deg.abs() / 4.0).ceil() as usize).clamp(2, 512);
    let point = |frac: f32, scale: f32| -> Point2D {
        let ang = (start_deg + sweep_deg * frac).to_radians();
        Point2D::new(
            cx_pt + rx * scale * ang.cos(),
            cy_pt + ry * scale * ang.sin(),
        )
    };
    let mut poly = Vec::with_capacity(segs * 2 + 2);
    if inner > 0.001 {
        // Annular sector: outer arc start→end, inner arc end→start.
        for i in 0..=segs {
            poly.push(point(i as f32 / segs as f32, 1.0));
        }
        for i in (0..=segs).rev() {
            poly.push(point(i as f32 / segs as f32, inner));
        }
    } else {
        // Pie wedge: centre + outer arc.
        poly.push(Point2D::new(cx_pt, cy_pt));
        for i in 0..=segs {
            poly.push(point(i as f32 / segs as f32, 1.0));
        }
    }
    poly
}

/// Paint an Ellipse node — a full oval when no arc geometry is
/// authored, otherwise a tessellated pie / arc / donut sector.
fn paint_ellipse(cx: &mut PaintCx<'_>, node: &SceneNode, world_rect: Rect, zoom: f32) {
    let inner = node.arc_inner_radius.unwrap_or(0.0).clamp(0.0, 1.0);
    let has_arc = node.arc_start_angle.is_some() || node.arc_sweep_angle.is_some() || inner > 0.001;
    let sweep = node.arc_sweep_angle.unwrap_or(360.0);
    // A full-circle sweep with no donut hole is just a plain oval.
    if !has_arc || (sweep.abs() >= 359.9 && inner <= 0.001) {
        if let Some(fill) = node.fill {
            cx.backend.fill_oval(world_rect, fill);
        }
        if let Some(stroke) = node.stroke {
            cx.backend
                .stroke_oval(world_rect, stroke.color, stroke.width * zoom);
        }
        return;
    }
    let start = node.arc_start_angle.unwrap_or(0.0);
    let poly = arc_polygon(world_rect, start, sweep, inner);
    if let Some(fill) = node.fill {
        cx.backend.fill_polygon(&poly, fill);
    }
    if let Some(stroke) = node.stroke {
        let w = stroke.width * zoom;
        if sweep.abs() >= 359.9 && inner > 0.001 {
            // Full ring — stroke the two concentric ovals so the
            // polygon's radial seam isn't drawn.
            cx.backend.stroke_oval(world_rect, stroke.color, w);
            let iw = world_rect.size.x * inner;
            let ih = world_rect.size.y * inner;
            let inner_rect = Rect {
                origin: Point2D::new(
                    world_rect.origin.x + (world_rect.size.x - iw) / 2.0,
                    world_rect.origin.y + (world_rect.size.y - ih) / 2.0,
                ),
                size: Point2D::new(iw, ih),
            };
            cx.backend.stroke_oval(inner_rect, stroke.color, w);
        } else {
            cx.backend.stroke_polygon(&poly, stroke.color, w);
        }
    }
}

/// One point on the cubic Bezier `p0→p3` (control points `p1`,`p2`).
pub(crate) fn cubic_point(p0: Point2D, p1: Point2D, p2: Point2D, p3: Point2D, t: f32) -> Point2D {
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point2D::new(
        w0 * p0.x + w1 * p1.x + w2 * p2.x + w3 * p3.x,
        w0 * p0.y + w1 * p1.y + w2 * p2.y + w3 * p3.y,
    )
}

/// One flattened segment `a → b` appended onto `out` — a cubic when
/// either endpoint carries a handle, else a straight line.
fn flatten_segment(
    a: &crate::layout_scene::SceneAnchor,
    b: &crate::layout_scene::SceneAnchor,
    out: &mut Vec<Point2D>,
) {
    let (p0, p3) = (a.pos, b.pos);
    let p1 = a.handle_out.unwrap_or(p0);
    let p2 = b.handle_in.unwrap_or(p3);
    if p1 == p0 && p2 == p3 {
        out.push(p3); // straight segment
    } else {
        for i in 1..=16 {
            out.push(cubic_point(p0, p1, p2, p3, i as f32 / 16.0));
        }
    }
}

/// Flatten a Path scene node into a doc-space polyline — cubic
/// segments whose endpoints carry handles are tessellated; a
/// handle-free path falls back to the straight `points` polyline.
/// A closed path appends the last-anchor → first-anchor segment.
pub(crate) fn flatten_path(node: &SceneNode) -> Vec<Point2D> {
    let anchors = &node.path_anchors;
    let has_handle = anchors
        .iter()
        .any(|a| a.handle_in.is_some() || a.handle_out.is_some());
    if anchors.len() < 2 || !has_handle {
        let mut out = node.points.clone();
        // Closed handle-free path — link the polyline back to its
        // start so the closing edge is drawn.
        if node.path_closed && out.len() > 2 {
            out.push(out[0]);
        }
        return out;
    }
    let mut out = Vec::with_capacity(anchors.len() * 16 + 16);
    out.push(anchors[0].pos);
    for pair in anchors.windows(2) {
        flatten_segment(&pair[0], &pair[1], &mut out);
    }
    if node.path_closed {
        flatten_segment(&anchors[anchors.len() - 1], &anchors[0], &mut out);
    }
    out
}

/// Recursively paint one resolved [`SceneNode`] and its subtree.
///
/// `viewport_origin` is the canvas-rect origin shifted by the
/// viewport pan; `zoom` is the viewport zoom. The scene already
/// carries layout-resolved absolute doc-space bounds, so paint is a
/// straight `doc → world` transform.
pub fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
) {
    // Hidden nodes (and their subtree) skip canvas paint entirely.
    // Layer panel still shows them, dimmed, so the user can unhide.
    if node.hidden {
        return;
    }
    let world_rect = Rect {
        origin: Point2D::new(
            viewport_origin.x + node.bounds.origin.x * zoom,
            viewport_origin.y + node.bounds.origin.y * zoom,
        ),
        size: Point2D::new(node.bounds.size.x * zoom, node.bounds.size.y * zoom),
    };
    // Viewport culling — bounded leaves skip paint entirely when
    // off-screen. Containers (bounds = ZERO) always recurse.
    if world_rect.size.x > 0.0 && world_rect.size.y > 0.0 && node.children.is_empty() {
        let off = world_rect.origin.x + world_rect.size.x < cull.origin.x
            || world_rect.origin.x > cull.origin.x + cull.size.x
            || world_rect.origin.y + world_rect.size.y < cull.origin.y
            || world_rect.origin.y > cull.origin.y + cull.size.y;
        if off {
            return;
        }
    }

    // Wrap the paint in save/rotate/restore if the node carries a
    // non-zero rotation. Rotation pivots around the node's own
    // bounds centre — for containers, this is the aggregate centre.
    let rotated = node.rotation.abs() > f32::EPSILON;
    if rotated {
        let pivot_doc = node.aggregate_bounds();
        let pivot = Point2D::new(
            viewport_origin.x + (pivot_doc.origin.x + pivot_doc.size.x / 2.0) * zoom,
            viewport_origin.y + (pivot_doc.origin.y + pivot_doc.size.y / 2.0) * zoom,
        );
        cx.backend.save();
        cx.backend.rotate(node.rotation, pivot);
    }

    // Drop shadows paint behind the node's own fill. Only kinds
    // whose silhouette a rounded rect / ellipse can represent
    // faithfully (Frame / Rect / Ellipse) cast one; Polygon / Line
    // / Path shadows are deferred until a shape-mask path exists.
    if !node.effects.is_empty()
        && world_rect.size.x > 0.0
        && world_rect.size.y > 0.0
        && matches!(
            node.kind,
            NodeKind::Frame | NodeKind::Rect | NodeKind::Ellipse
        )
    {
        paint_drop_shadows(cx, node, world_rect, zoom);
    }

    match &node.kind {
        NodeKind::Frame => {
            // Image-fill Frames paint the bitmap behind their
            // children; gradient + solid fall back to the shared
            // fill/stroke painter. Without this branch a Frame whose
            // primary fill is `PenFill::Image { url }` only shows the
            // grey placeholder + its children, never the image.
            if let Some(src) = node.image_src.as_deref() {
                paint_image_node(cx, node, world_rect, zoom, src);
            } else {
                paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
            }
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, edit_caret.clone(), cull);
            }
        }
        NodeKind::Other(tag) if tag == "icon_font" => crate::widgets::icons::paint_icon_font_node(
            cx.backend,
            node.font_family.as_str(),
            node.text.as_deref().unwrap_or(""),
            world_rect,
            node.fill,
        ),
        NodeKind::Group | NodeKind::Other(_) => {
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, edit_caret.clone(), cull);
            }
        }
        NodeKind::Rect => {
            // Image nodes land as `kind="rect"` (the loader rewrites
            // their variant so non-image paths keep working). When a
            // `src` is carried, paint the bitmap; the grey `fill`
            // remains as the placeholder visible while the decoder
            // is missing the bytes (corrupt URL / unsupported codec).
            if let Some(src) = node.image_src.as_deref() {
                paint_image_node(cx, node, world_rect, zoom, src);
            } else {
                paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
            }
        }
        NodeKind::Ellipse => {
            if let Some(src) = node.image_src.as_deref() {
                // Image-fill ellipse: paint the bitmap clipped to the
                // ellipse silhouette via skia's `clip_oval`-style
                // approximation (no native clip_oval on the trait, so
                // fall back to the rect-clip path the painter has).
                paint_image_node(cx, node, world_rect, zoom, src);
                if let Some(stroke) = node.stroke {
                    cx.backend
                        .stroke_oval(world_rect, stroke.color, stroke.width * zoom);
                }
            } else {
                paint_ellipse(cx, node, world_rect, zoom);
            }
        }
        NodeKind::Polygon => {
            let pts = regular_polygon_points(world_rect, node.polygon_sides);
            // Image fills paint the bitmap in the AABB underneath the
            // polygon outline; the polygon silhouette is then drawn
            // by the stroke. A perfect clip-to-polygon path lands when
            // `RenderBackend` grows a polygon-clip primitive.
            if let Some(src) = node.image_src.as_deref() {
                paint_image_node(cx, node, world_rect, zoom, src);
            } else if let Some(fill) = node.fill {
                cx.backend.fill_polygon(&pts, fill);
            }
            if let Some(stroke) = node.stroke {
                cx.backend
                    .stroke_polygon(&pts, stroke.color, stroke.width * zoom);
            }
        }
        NodeKind::Line => {
            // Top-left → bottom-right diagonal across the bounds,
            // stroked at the stroke width (or 1.5 if no stroke).
            let from = Point2D::new(world_rect.origin.x, world_rect.origin.y);
            let to = Point2D::new(
                world_rect.origin.x + world_rect.size.x,
                world_rect.origin.y + world_rect.size.y,
            );
            let (color, width) = match node.stroke {
                Some(s) => (s.color, s.width * zoom),
                None => (
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                ),
            };
            cx.backend.stroke_line(from, to, color, width);
        }
        NodeKind::Path => {
            if let Some(d) = node.svg_path.as_deref() {
                paint_svg_path_node(cx, node, world_rect, zoom, d);
                if rotated {
                    cx.backend.restore();
                }
                return;
            }
            let to_world = |p: Point2D| -> Point2D {
                Point2D::new(
                    viewport_origin.x + p.x * zoom,
                    viewport_origin.y + p.y * zoom,
                )
            };
            // Bezier-aware: when the path carries anchors with control
            // handles, flatten each cubic segment; otherwise fall back
            // to the straight `points` polyline.
            let polyline = flatten_path(node);
            // A closed path with a fill paints its enclosed area.
            let filled = node.path_closed && node.fill.is_some();
            if filled {
                let world: Vec<Point2D> = polyline.iter().map(|p| to_world(*p)).collect();
                cx.backend.fill_polygon(&world, node.fill.unwrap());
            }
            // Stroke: an explicit stroke always paints; with no
            // stroke, only an UNfilled path strokes (so it stays
            // visible) — a filled path must not draw an implicit
            // outline.
            let stroke = match node.stroke {
                Some(s) => Some((s.color, s.width * zoom)),
                None if !filled => Some((
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                )),
                None => None,
            };
            if let Some((color, width)) = stroke {
                for pair in polyline.windows(2) {
                    cx.backend
                        .stroke_line(to_world(pair[0]), to_world(pair[1]), color, width);
                }
            }
        }
        NodeKind::Text => {
            paint_text_node(cx, node, world_rect, zoom, &edit_caret);
        }
    }

    if rotated {
        cx.backend.restore();
    }
}

fn paint_svg_path_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    d: &str,
) {
    if let Some(fill) = node.fill {
        cx.backend
            .fill_svg_path(d, world_rect.origin, zoom, 1.0, fill);
    }
    if let Some(stroke) = node.stroke {
        cx.backend.stroke_svg_path(
            d,
            world_rect.origin,
            24.0 * zoom,
            stroke.color,
            stroke.width * zoom,
        );
    }
}

/// Paint a Text `SceneNode` — wrapped or single-line text plus the
/// edit caret when the node is the one being edited.
/// Decode an inline-base64 `data:image/...;base64,...` URL into the
/// raw image bytes the backend's `draw_image` decoder expects. Returns
/// `None` for any URL that isn't an inline base64 payload (file paths,
/// remote URLs, malformed strings) — those paths are deferred to a
/// future loader.
fn data_url_bytes(src: &str) -> Option<Vec<u8>> {
    let after_scheme = src.strip_prefix("data:")?;
    let comma = after_scheme.find(',')?;
    let meta = &after_scheme[..comma];
    let payload = &after_scheme[comma + 1..];
    if !meta.contains(";base64") {
        return None;
    }
    // The base64 alphabet is ASCII; strip any embedded whitespace
    // (line breaks in a wrapped data URL) before decode.
    let clean: String = payload
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine as _;
    B64.decode(clean.as_bytes()).ok()
}

/// Hash a string into a stable u64 — drives the backend's image
/// decode cache so the same `src` doesn't re-decode every frame.
fn src_hash(src: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    src.hash(&mut h);
    h.finish()
}

/// Paint a raster image inside `world_rect`. Decodes the data URL
/// once, hands raw bytes + a stable id to the backend cache, then
/// strokes the corner-radius outline on top so a per-corner radius
/// authored on the schema still reads.
fn paint_image_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    src: &str,
) {
    let bytes = data_url_bytes(src);
    let r = node.corner_radius * zoom;
    let use_round = r > 0.5;
    // Only paint the grey placeholder when the URL can't be decoded
    // — painting it under a transparent raster would leave a grey
    // matte bleeding through the alpha channel.
    if bytes.is_none() {
        if let Some(fill) = node.fill {
            if use_round {
                cx.backend.fill_round_rect(world_rect, r, fill);
            } else {
                cx.backend.fill_rect(world_rect, fill);
            }
        }
    }
    if let Some(bytes) = bytes {
        let id = src_hash(src);
        cx.backend.draw_image_with_options(
            world_rect,
            id,
            &bytes,
            node.image_fit.to_draw_mode(),
            node.image_adjustments,
        );
    }
    if let Some(stroke) = node.stroke {
        let width = stroke.width * zoom;
        if use_round {
            cx.backend
                .stroke_round_rect(world_rect, r, stroke.color, width);
        } else {
            cx.backend.stroke_rect(world_rect, stroke.color, width);
        }
    }
}

fn paint_text_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    edit_caret: &Option<EditCaret>,
) {
    let text = node.text.as_deref().unwrap_or("");
    // Ink colour follows the resolved fill (defaults to near black).
    let ink = node.fill.unwrap_or(crate::Color {
        r: 0.08,
        g: 0.08,
        b: 0.08,
        a: 1.0,
    });
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    // Honour authored font size from the canonical schema; default to
    // 13 px so editor-created text stays uniform. Baseline ≈ 1.08 × size.
    let base_size = if node.font_size > 0.0 {
        node.font_size
    } else {
        13.0
    };
    let font_size = base_size * zoom;
    let weight = if node.font_weight > 0 {
        node.font_weight
    } else {
        400
    };
    let family = if node.font_family.trim().is_empty() {
        "system-ui"
    } else {
        node.font_family.as_str()
    };
    let line_height = if node.line_height > 0.0 {
        node.line_height
    } else {
        1.2
    };
    let letter_spacing = node.letter_spacing * zoom;
    let lines: Vec<String> = if node.text_wrap {
        wrap_text(cx.backend, text, font_size, world_rect.size.x, weight)
    } else {
        text.split('\n').map(str::to_string).collect()
    };
    if !text.is_empty() {
        let jc = jian_core::scene::Color::rgba(ch(ink.r), ch(ink.g), ch(ink.b), ch(ink.a));
        let line_h = base_size * line_height * zoom;
        let text_h = if lines.is_empty() {
            0.0
        } else {
            font_size + line_h * (lines.len().saturating_sub(1) as f32)
        };
        let first_baseline_y = match node.text_vertical_align {
            crate::layout_scene::SceneTextVerticalAlign::Middle => {
                world_rect.origin.y + ((world_rect.size.y - text_h).max(0.0) / 2.0) + font_size
            }
            crate::layout_scene::SceneTextVerticalAlign::Bottom => {
                world_rect.origin.y + (world_rect.size.y - text_h).max(0.0) + font_size
            }
            crate::layout_scene::SceneTextVerticalAlign::Top => world_rect.origin.y + font_size,
        };
        for (idx, line) in lines.iter().enumerate() {
            let line_w = measure_line_width(cx.backend, line, font_size, weight, letter_spacing);
            let x = match node.text_align {
                crate::layout_scene::SceneTextAlign::Center => {
                    world_rect.origin.x + (world_rect.size.x - line_w).max(0.0) / 2.0
                }
                crate::layout_scene::SceneTextAlign::Right => {
                    world_rect.origin.x + (world_rect.size.x - line_w).max(0.0)
                }
                crate::layout_scene::SceneTextAlign::Left
                | crate::layout_scene::SceneTextAlign::Justify => world_rect.origin.x,
            };
            let y = first_baseline_y + idx as f32 * line_h;
            draw_text_line(
                cx.backend,
                line,
                family,
                font_size,
                weight,
                jc,
                Point2D::new(x, y),
                letter_spacing,
            );
        }
    }
    // Caret while editing — sits at the end of the text.
    if let Some(c) = edit_caret {
        if c.editing == node.id && jian_core::anim::blink_visible(c.now_ms, c.anchor_ms, 500) {
            let caret_line = lines.last().map(String::as_str).unwrap_or("");
            let text_w =
                measure_line_width(cx.backend, caret_line, font_size, weight, letter_spacing);
            let caret = Rect {
                origin: Point2D::new(
                    world_rect.origin.x + text_w,
                    world_rect.origin.y + 2.0 * zoom,
                ),
                size: Point2D::new(1.0_f32.max(zoom), font_size * 1.15),
            };
            cx.backend.fill_rect(caret, ink);
        }
    }
}

fn measure_line_width(
    backend: &mut dyn crate::RenderBackend,
    line: &str,
    font_size: f32,
    weight: u16,
    letter_spacing: f32,
) -> f32 {
    let base = backend.measure_text_weighted(line, font_size, weight);
    let extra = line.chars().count().saturating_sub(1) as f32 * letter_spacing;
    base + extra
}

#[allow(clippy::too_many_arguments)]
fn draw_text_line(
    backend: &mut dyn crate::RenderBackend,
    line: &str,
    family: &str,
    font_size: f32,
    weight: u16,
    color: jian_core::scene::Color,
    origin: Point2D,
    letter_spacing: f32,
) {
    if letter_spacing.abs() < f32::EPSILON {
        backend.draw_text(
            &TextLayout::single_run(line, family, font_size, color, Point2D::new(0.0, 0.0))
                .with_font_weight(weight),
            origin,
        );
        return;
    }
    let mut x = origin.x;
    for ch in line.chars() {
        let s = ch.to_string();
        backend.draw_text(
            &TextLayout::single_run(&s, family, font_size, color, Point2D::new(0.0, 0.0))
                .with_font_weight(weight),
            Point2D::new(x, origin.y),
        );
        x += backend.measure_text_weighted(&s, font_size, weight) + letter_spacing;
    }
}

#[cfg(test)]
mod arc_tests {
    use super::arc_polygon;
    use crate::Rect;

    #[test]
    fn pie_polygon_starts_at_centre() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        assert_eq!(poly[0].x, 50.0);
        assert_eq!(poly[0].y, 50.0);
        assert!((poly[1].x - 100.0).abs() < 0.01);
        assert!((poly[1].y - 50.0).abs() < 0.01);
    }

    #[test]
    fn donut_polygon_has_outer_and_inner_rings() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 360.0, 0.5);
        assert_eq!(poly.len(), 2 * (90 + 1));
        let last = poly[poly.len() - 1];
        let dist = ((last.x - 50.0).powi(2) + (last.y - 50.0).powi(2)).sqrt();
        assert!((dist - 25.0).abs() < 0.5, "inner radius ~25, got {dist}");
    }

    #[test]
    fn quarter_sweep_end_point_at_90_degrees() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        let last = poly[poly.len() - 1];
        assert!((last.x - 50.0).abs() < 0.01);
        assert!((last.y - 100.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod text_tests {
    use super::paint_text_node;
    use crate::layout_scene::{NodeKind, SceneNode, SceneTextAlign, SceneTextVerticalAlign};
    use crate::widgets::PaintCx;
    use crate::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

    #[derive(Default)]
    struct TextCaptureBackend {
        origins: Vec<Point2D>,
        families: Vec<String>,
    }

    impl RenderBackend for TextCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
            self.origins.push(origin);
            if let Some(run) = layout.runs().first() {
                self.families.push(run.font_family.clone());
            }
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn measure_text_weighted(&mut self, text: &str, font_size: f32, _: u16) -> f32 {
            text.chars().count() as f32 * font_size * 0.5
        }
    }

    #[test]
    fn text_node_paint_honors_typography_alignment() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some("Hi".to_string());
        node.font_family = "Georgia".to_string();
        node.font_size = 20.0;
        node.line_height = 1.0;
        node.text_align = SceneTextAlign::Center;
        node.text_vertical_align = SceneTextVerticalAlign::Middle;
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        assert_eq!(backend.families, vec!["Georgia".to_string()]);
        let origin = backend.origins[0];
        assert!(
            origin.x > 80.0,
            "center-aligned text should move away from the left edge"
        );
        assert!(
            origin.y > 40.0,
            "middle-aligned text should move down from the top baseline"
        );
    }
}

#[cfg(test)]
mod path_tests {
    use super::flatten_path;
    use crate::layout_scene::{NodeKind, SceneAnchor, SceneNode, ScenePointType};
    use crate::{Point2D, Rect};

    fn anchor(x: f32, y: f32, hout: Option<Point2D>) -> SceneAnchor {
        SceneAnchor {
            pos: Point2D::new(x, y),
            handle_in: None,
            handle_out: hout,
            point_type: ScenePointType::Corner,
        }
    }

    #[test]
    fn handle_free_path_falls_back_to_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 0.0)];
        n.path_anchors = vec![anchor(0.0, 0.0, None), anchor(10.0, 0.0, None)];
        assert_eq!(flatten_path(&n), n.points);
    }

    #[test]
    fn curved_segment_tessellates_into_many_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(100.0, 0.0)];
        n.path_anchors = vec![
            anchor(0.0, 0.0, Some(Point2D::new(0.0, 50.0))),
            anchor(100.0, 0.0, None),
        ];
        let poly = flatten_path(&n);
        assert_eq!(poly.len(), 17);
        assert_eq!(poly[0], Point2D::new(0.0, 0.0));
        assert_eq!(poly[poly.len() - 1], Point2D::new(100.0, 0.0));
        assert!(poly[8].y > 1.0, "curve bows toward the handle");
    }

    #[test]
    fn bounds_kept_so_helper_is_pure() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.bounds = Rect::xywh(1.0, 2.0, 3.0, 4.0);
        let _ = flatten_path(&n);
        assert_eq!(n.bounds, Rect::xywh(1.0, 2.0, 3.0, 4.0));
    }
}
