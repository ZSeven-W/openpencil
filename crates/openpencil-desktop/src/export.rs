//! Raster + SVG export for the active page.
//!
//! Renders top-level nodes on the active page into an off-screen
//! `skia_safe::Surface`, encodes as one of PNG/JPEG/WEBP, writes to
//! the user-chosen path. SVG goes through a separate hand-rolled
//! serializer (`export_svg`). Coverage:
//!   - Rect / Frame / Group : fill + stroke + corner_radius
//!   - Ellipse              : fill + stroke
//!   - Polygon              : default-triangle fill + stroke
//!   - Line                 : stroke from `bounds.origin` to `+size`
//!   - Path                 : polyline through `node.points`
//!   - Text                 : skipped (font plumbing pending)
//! Per-node rotation honoured via `Canvas::rotate`.
//!
//! Background: PNG / WEBP transparent; JPEG forced white (TS parity
//! — JPEG has no alpha so a "transparent" JPEG would read as black).
//! Scale: caller picks @1x / @2x / @3x (TS export dialog parity).

use openpencil_shell_core::document::{Document, Node, NodeKind};
use openpencil_shell_core::{Color, Point2D, Rect};
use skia_safe::{Canvas, EncodedImageFormat, Paint, PaintStyle, Path, PathBuilder};
use std::fmt::Write as _;
use std::path::Path as StdPath;

const MARGIN: f32 = 16.0;

/// Raster export format. Matches TS ExportDialog's PNG / JPEG / WEBP
/// options; SVG has its own entry point (`export_svg`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RasterFormat {
    Png,
    Jpeg,
    Webp,
}

impl RasterFormat {
    /// Skia encoder format.
    fn skia(self) -> EncodedImageFormat {
        match self {
            RasterFormat::Png => EncodedImageFormat::PNG,
            RasterFormat::Jpeg => EncodedImageFormat::JPEG,
            RasterFormat::Webp => EncodedImageFormat::WEBP,
        }
    }
    /// Encoder quality. PNG ignores it (lossless); JPEG/WEBP land on
    /// 92 to match TS (quality 0.92 in canvas.toDataURL).
    fn quality(self) -> u32 {
        match self {
            RasterFormat::Png => 100,
            RasterFormat::Jpeg | RasterFormat::Webp => 92,
        }
    }
    /// True when the format supports transparency. JPEG doesn't, so
    /// the background must be filled before drawing.
    fn supports_alpha(self) -> bool {
        !matches!(self, RasterFormat::Jpeg)
    }
    /// Lookup by file extension (lowercase). Returns None for unknown
    /// extensions. Used by future drag-drop / scripted-export paths
    /// (current UI carries the format via `Document.ui.export_format`).
    #[allow(dead_code)]
    pub fn from_extension(ext: &str) -> Option<RasterFormat> {
        match ext {
            "png" => Some(RasterFormat::Png),
            "jpg" | "jpeg" => Some(RasterFormat::Jpeg),
            "webp" => Some(RasterFormat::Webp),
            _ => None,
        }
    }
    /// Human-readable label for error messages ("PNG" / "JPEG" / "WEBP").
    fn user_label(self) -> &'static str {
        match self {
            RasterFormat::Png => "PNG",
            RasterFormat::Jpeg => "JPEG",
            RasterFormat::Webp => "WEBP",
        }
    }
}

/// Raster export with explicit format + scale. Scale clamped to
/// [0.5, 8.0] to keep surface allocation sane; NaN / inf fall back
/// to 2× (codex CONCERN — NaN reaching `canvas.scale` produces a
/// garbage transform).
pub fn export_raster(
    doc: &Document,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
) -> Result<(), String> {
    let scale = if scale.is_finite() { scale.clamp(0.5, 8.0) } else { 2.0 };
    let Some(page) = doc.pages.get(doc.active_page_index) else {
        return Err("no active page".into());
    };
    let bounds = page_bounds(page).ok_or("nothing to export")?;
    let width_px = ((bounds.size.x + MARGIN * 2.0) * scale).round() as i32;
    let height_px = ((bounds.size.y + MARGIN * 2.0) * scale).round() as i32;
    let info = skia_safe::ImageInfo::new(
        (width_px.max(1), height_px.max(1)),
        skia_safe::ColorType::N32,
        skia_safe::AlphaType::Premul,
        None,
    );
    let mut surface = skia_safe::surfaces::raster(&info, None, None).ok_or("alloc surface")?;
    let canvas = surface.canvas();
    if format.supports_alpha() {
        canvas.clear(skia_safe::Color::TRANSPARENT);
    } else {
        canvas.clear(skia_safe::Color::WHITE);
    }
    canvas.scale((scale, scale));
    canvas.translate((MARGIN - bounds.origin.x, MARGIN - bounds.origin.y));
    for node in &page.children {
        paint_node(canvas, node);
    }
    let image = surface.image_snapshot();
    let data = image
        .encode(None, format.skia(), format.quality())
        .ok_or_else(|| format!("encode {} failed", format.user_label()))?;
    std::fs::write(target, data.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn page_bounds(page: &openpencil_shell_core::document::Page) -> Option<Rect> {
    let mut acc = BoundsAcc::new();
    for n in &page.children {
        collect_bounds(n, glam::Affine2::IDENTITY, &mut acc);
    }
    acc.into_rect()
}

struct BoundsAcc {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl BoundsAcc {
    fn new() -> Self {
        Self {
            min_x: f32::INFINITY,
            min_y: f32::INFINITY,
            max_x: f32::NEG_INFINITY,
            max_y: f32::NEG_INFINITY,
        }
    }
    fn add(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.min_x = self.min_x.min(x0);
        self.min_y = self.min_y.min(y0);
        self.max_x = self.max_x.max(x1);
        self.max_y = self.max_y.max(y1);
    }
    fn into_rect(self) -> Option<Rect> {
        if !self.min_x.is_finite() {
            return None;
        }
        Some(Rect {
            origin: Point2D::new(self.min_x, self.min_y),
            size: Point2D::new(self.max_x - self.min_x, self.max_y - self.min_y),
        })
    }
}

/// Mirror of `paint_node`'s traversal — visits the SAME nodes paint
/// visits, in the SAME order, threading the cumulative parent
/// transform so a child painted under a rotated container ends up
/// in the same world coords paint will emit. Bounds stay in
/// lockstep with paint, so the surface never clips a row a
/// downstream draw would touch (including children inside rotated
/// frames / groups).
fn collect_bounds(n: &Node, parent_xform: glam::Affine2, acc: &mut BoundsAcc) {
    if n.hidden {
        return;
    }
    // Compose this node's rotation around its own centre into the
    // cumulative parent transform. Mirrors `paint_node`'s guard
    // EXACTLY — paint checks the RAW (unnormalized) size, so a
    // negative-size rect (drag-from-right-to-left) skips rotation
    // even though `normalize_rect` would report a positive extent.
    // Using the normalized size here would rotate the AABB while
    // paint stayed unrotated → mismatch.
    let raw = n.bounds;
    let rotate_self = n.rotation != 0.0 && raw.size.x > 0.0 && raw.size.y > 0.0;
    let local_xform = if rotate_self {
        let pivot = glam::Vec2::new(
            raw.origin.x + raw.size.x * 0.5,
            raw.origin.y + raw.size.y * 0.5,
        );
        parent_xform
            * glam::Affine2::from_translation(pivot)
            * glam::Affine2::from_angle(n.rotation)
            * glam::Affine2::from_translation(-pivot)
    } else {
        parent_xform
    };
    if let Some(local_corners) = own_paint_corners(n) {
        for p in local_corners {
            let w = local_xform.transform_point2(p);
            acc.add(w.x, w.y, w.x, w.y);
        }
    }
    for child in &n.children {
        collect_bounds(child, local_xform, acc);
    }
}

/// Local-space corner points that bound `n`'s own paint (NOT its
/// children — those visit through `collect_bounds`). The caller
/// applies the cumulative parent+self transform; each returned
/// point gets pushed into the BoundsAcc as a world-space coord.
///
/// Returns `None` for invisible kinds: Group/Text never paint own
/// content; Frame/Other contribute only when fill or stroke is set;
/// Path with empty `points` is invisible.
fn own_paint_corners(n: &Node) -> Option<Vec<glam::Vec2>> {
    let stroke_pad = n.stroke.map(|s| s.width * 0.5).unwrap_or(0.0);
    let (x0, y0, x1, y1) = match &n.kind {
        NodeKind::Rect | NodeKind::Ellipse | NodeKind::Polygon | NodeKind::Line => {
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x,
                nr.origin.y + nr.size.y,
            )
        }
        NodeKind::Frame | NodeKind::Other(_) => {
            if n.fill.is_none() && n.stroke.is_none() {
                return None;
            }
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x,
                nr.origin.y + nr.size.y,
            )
        }
        NodeKind::Text => {
            // Text bounds are the user-set "where the glyphs sit"
            // rect. Real glyph extents can overshoot for tails /
            // accents, but `bounds` is the right approximation
            // without doing a per-glyph metric pass.
            let has_text = n.text.as_ref().is_some_and(|s| !s.is_empty());
            if !has_text {
                return None;
            }
            let nr = normalize_rect(n.bounds);
            (
                nr.origin.x,
                nr.origin.y,
                nr.origin.x + nr.size.x.max(1.0),
                nr.origin.y + nr.size.y.max(1.0),
            )
        }
        NodeKind::Path => {
            if n.points.is_empty() {
                return None;
            }
            // Each polyline anchor + stroke-pad cardinal offsets so
            // the cumulative parent transform doesn't clip them.
            let mut out = Vec::with_capacity(n.points.len() * 4);
            for p in &n.points {
                out.push(glam::Vec2::new(p.x - stroke_pad, p.y - stroke_pad));
                out.push(glam::Vec2::new(p.x + stroke_pad, p.y - stroke_pad));
                out.push(glam::Vec2::new(p.x - stroke_pad, p.y + stroke_pad));
                out.push(glam::Vec2::new(p.x + stroke_pad, p.y + stroke_pad));
            }
            return Some(out);
        }
        NodeKind::Group => return None,
    };
    if (x1 - x0).abs() == 0.0 && (y1 - y0).abs() == 0.0 {
        return None;
    }
    Some(vec![
        glam::Vec2::new(x0 - stroke_pad, y0 - stroke_pad),
        glam::Vec2::new(x1 + stroke_pad, y0 - stroke_pad),
        glam::Vec2::new(x1 + stroke_pad, y1 + stroke_pad),
        glam::Vec2::new(x0 - stroke_pad, y1 + stroke_pad),
    ])
}


pub(crate) fn paint_node(canvas: &Canvas, node: &Node) {
    if node.hidden {
        return;
    }
    let world_rect = node.bounds;
    let pre_rotate = canvas.save();
    if node.rotation != 0.0 && world_rect.size.x > 0.0 && world_rect.size.y > 0.0 {
        let cx = world_rect.origin.x + world_rect.size.x / 2.0;
        let cy = world_rect.origin.y + world_rect.size.y / 2.0;
        canvas.rotate(node.rotation.to_degrees(), Some((cx, cy).into()));
    }
    match &node.kind {
        NodeKind::Rect | NodeKind::Frame | NodeKind::Other(_) => {
            paint_rect(canvas, world_rect, node);
        }
        NodeKind::Ellipse => {
            paint_oval(canvas, world_rect, node);
        }
        NodeKind::Polygon => {
            paint_triangle(canvas, world_rect, node);
        }
        NodeKind::Line => {
            if let Some(stroke) = node.stroke {
                let mut paint = make_stroke(stroke.color, stroke.width);
                canvas.draw_line(
                    (world_rect.origin.x, world_rect.origin.y),
                    (
                        world_rect.origin.x + world_rect.size.x,
                        world_rect.origin.y + world_rect.size.y,
                    ),
                    &paint,
                );
                paint.set_anti_alias(true);
            }
        }
        NodeKind::Path => {
            paint_polyline(canvas, node);
        }
        NodeKind::Text => paint_text(canvas, world_rect, node),
        // Group has no own paint — children handle render.
        NodeKind::Group => {}
    }
    for child in &node.children {
        paint_node(canvas, child);
    }
    canvas.restore_to_count(pre_rotate);
}

fn paint_rect(canvas: &Canvas, rect: Rect, node: &Node) {
    let nrect = normalize_rect(rect);
    if nrect.size.x == 0.0 && nrect.size.y == 0.0 {
        return;
    }
    let r = node.corner_radius;
    let sk_rect = to_sk_rect(nrect);
    if let Some(fill) = node.fill {
        let paint = make_fill(fill);
        if r > 0.5 {
            canvas.draw_round_rect(sk_rect, r, r, &paint);
        } else {
            canvas.draw_rect(sk_rect, &paint);
        }
    }
    if let Some(stroke) = node.stroke {
        let paint = make_stroke(stroke.color, stroke.width);
        if r > 0.5 {
            canvas.draw_round_rect(sk_rect, r, r, &paint);
        } else {
            canvas.draw_rect(sk_rect, &paint);
        }
    }
}

fn paint_oval(canvas: &Canvas, rect: Rect, node: &Node) {
    let nrect = normalize_rect(rect);
    if nrect.size.x == 0.0 && nrect.size.y == 0.0 {
        return;
    }
    let sk_rect = to_sk_rect(nrect);
    if let Some(fill) = node.fill {
        canvas.draw_oval(sk_rect, &make_fill(fill));
    }
    if let Some(stroke) = node.stroke {
        canvas.draw_oval(sk_rect, &make_stroke(stroke.color, stroke.width));
    }
}

fn paint_triangle(canvas: &Canvas, rect: Rect, node: &Node) {
    let rect = normalize_rect(rect);
    if rect.size.x == 0.0 || rect.size.y == 0.0 {
        return;
    }
    let cx = rect.origin.x + rect.size.x / 2.0;
    let top = rect.origin.y;
    let left = rect.origin.x;
    let right = rect.origin.x + rect.size.x;
    let bottom = rect.origin.y + rect.size.y;
    let mut builder = PathBuilder::new();
    builder.move_to((cx, top));
    builder.line_to((left, bottom));
    builder.line_to((right, bottom));
    builder.close();
    let path: Path = builder.detach();
    if let Some(fill) = node.fill {
        canvas.draw_path(&path, &make_fill(fill));
    }
    if let Some(stroke) = node.stroke {
        canvas.draw_path(&path, &make_stroke(stroke.color, stroke.width));
    }
}

fn paint_text(canvas: &Canvas, rect: Rect, node: &Node) {
    let Some(text) = node.text.as_deref() else { return };
    if text.is_empty() {
        return;
    }
    let r = normalize_rect(rect);
    let Some(typeface) = skia_safe::FontMgr::new()
        .legacy_make_typeface(None, skia_safe::FontStyle::default())
    else {
        return;
    };
    // Match editor exactly (canvas_viewport.rs §NodeKind::Text):
    // 13pt font, baseline at origin.y + 14, fill defaults to the
    // near-black `(0.08, 0.08, 0.08)` ink colour — NOT pure black.
    let font = skia_safe::Font::from_typeface(typeface, TEXT_FONT_SIZE);
    let color = node.fill.unwrap_or(TEXT_DEFAULT_FILL);
    let paint = make_fill(color);
    canvas.draw_str(
        text,
        (r.origin.x, r.origin.y + TEXT_BASELINE_OFFSET),
        &font,
        &paint,
    );
}

/// Mirrors `canvas_viewport.rs` text paint constants so PNG export
/// matches what the user sees on the editor canvas at zoom 1.
const TEXT_FONT_SIZE: f32 = 13.0;
const TEXT_BASELINE_OFFSET: f32 = 14.0;
const TEXT_DEFAULT_FILL: Color = Color {
    r: 0.08,
    g: 0.08,
    b: 0.08,
    a: 1.0,
};

fn paint_polyline(canvas: &Canvas, node: &Node) {
    if node.points.len() < 2 {
        return;
    }
    let Some(stroke) = node.stroke else { return };
    let mut builder = PathBuilder::new();
    builder.move_to((node.points[0].x, node.points[0].y));
    for p in &node.points[1..] {
        builder.line_to((p.x, p.y));
    }
    let path: Path = builder.detach();
    canvas.draw_path(&path, &make_stroke(stroke.color, stroke.width));
}

fn make_fill(c: Color) -> Paint {
    let mut paint = Paint::new(color_to_sk(c), None);
    paint.set_anti_alias(true);
    paint
}

fn make_stroke(c: Color, width: f32) -> Paint {
    let mut paint = Paint::new(color_to_sk(c), None);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(width.max(0.5));
    paint
}

fn color_to_sk(c: Color) -> skia_safe::Color4f {
    skia_safe::Color4f::new(c.r, c.g, c.b, c.a)
}

fn to_sk_rect(r: Rect) -> skia_safe::Rect {
    skia_safe::Rect::from_xywh(r.origin.x, r.origin.y, r.size.x, r.size.y)
}

/// Negative-size bounds come from drag-from-right-to-left creates;
/// normalise so paint always has top-left origin + positive extent.
fn normalize_rect(r: Rect) -> Rect {
    let x0 = r.origin.x.min(r.origin.x + r.size.x);
    let y0 = r.origin.y.min(r.origin.y + r.size.y);
    Rect {
        origin: Point2D::new(x0, y0),
        size: Point2D::new(r.size.x.abs(), r.size.y.abs()),
    }
}

// ── SVG ───────────────────────────────────────────────────────────

pub fn export_svg(doc: &Document, target: &StdPath) -> Result<(), String> {
    let Some(page) = doc.pages.get(doc.active_page_index) else {
        return Err("no active page".into());
    };
    let bounds = page_bounds(page).ok_or("nothing to export")?;
    let view_x = bounds.origin.x - MARGIN;
    let view_y = bounds.origin.y - MARGIN;
    let view_w = bounds.size.x + MARGIN * 2.0;
    let view_h = bounds.size.y + MARGIN * 2.0;
    let mut svg = String::with_capacity(4096);
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{view_x} {view_y} {view_w} {view_h}" width="{view_w}" height="{view_h}">"#
    );
    for node in &page.children {
        emit_node(&mut svg, node);
    }
    svg.push_str("</svg>");
    std::fs::write(target, svg).map_err(|e| e.to_string())?;
    Ok(())
}

fn emit_node(out: &mut String, n: &Node) {
    if n.hidden {
        return;
    }
    let raw = n.bounds;
    // SVG groups inherit transform onto children — same composition
    // model as `paint_node` + `collect_bounds`. Match paint's guard.
    let needs_g = n.rotation != 0.0 && raw.size.x > 0.0 && raw.size.y > 0.0;
    if needs_g {
        let cx = raw.origin.x + raw.size.x * 0.5;
        let cy = raw.origin.y + raw.size.y * 0.5;
        let deg = n.rotation.to_degrees();
        let _ = write!(out, r#"<g transform="rotate({deg} {cx} {cy})">"#);
    }
    match &n.kind {
        NodeKind::Rect | NodeKind::Frame | NodeKind::Other(_) => emit_rect(out, n),
        NodeKind::Ellipse => emit_ellipse(out, n),
        NodeKind::Polygon => emit_polygon(out, n),
        NodeKind::Line => emit_line(out, n),
        NodeKind::Path => emit_path(out, n),
        NodeKind::Text => emit_text(out, n),
        NodeKind::Group => {}
    }
    for child in &n.children {
        emit_node(out, child);
    }
    if needs_g {
        out.push_str("</g>");
    }
}

fn emit_rect(out: &mut String, n: &Node) {
    if n.fill.is_none() && n.stroke.is_none() && !matches!(n.kind, NodeKind::Rect) {
        return;
    }
    let r = normalize_rect(n.bounds);
    if r.size.x == 0.0 && r.size.y == 0.0 {
        return;
    }
    let rx = if n.corner_radius > 0.0 {
        format!(r#" rx="{}""#, n.corner_radius)
    } else {
        String::new()
    };
    let _ = write!(
        out,
        r#"<rect x="{}" y="{}" width="{}" height="{}"{rx}{}/>"#,
        r.origin.x,
        r.origin.y,
        r.size.x,
        r.size.y,
        fill_stroke_attrs(n),
    );
}

fn emit_ellipse(out: &mut String, n: &Node) {
    let r = normalize_rect(n.bounds);
    if r.size.x == 0.0 || r.size.y == 0.0 {
        return;
    }
    let cx = r.origin.x + r.size.x * 0.5;
    let cy = r.origin.y + r.size.y * 0.5;
    let _ = write!(
        out,
        r#"<ellipse cx="{cx}" cy="{cy}" rx="{}" ry="{}"{}/>"#,
        r.size.x * 0.5,
        r.size.y * 0.5,
        fill_stroke_attrs(n),
    );
}

fn emit_polygon(out: &mut String, n: &Node) {
    let r = normalize_rect(n.bounds);
    if r.size.x == 0.0 || r.size.y == 0.0 {
        return;
    }
    let cx = r.origin.x + r.size.x * 0.5;
    let top = r.origin.y;
    let left = r.origin.x;
    let right = r.origin.x + r.size.x;
    let bottom = r.origin.y + r.size.y;
    let _ = write!(
        out,
        r#"<polygon points="{cx},{top} {left},{bottom} {right},{bottom}"{}/>"#,
        fill_stroke_attrs(n),
    );
}

fn emit_line(out: &mut String, n: &Node) {
    let Some(stroke) = n.stroke else { return };
    let r = n.bounds;
    let x2 = r.origin.x + r.size.x;
    let y2 = r.origin.y + r.size.y;
    let _ = write!(
        out,
        r#"<line x1="{}" y1="{}" x2="{x2}" y2="{y2}"{}/>"#,
        r.origin.x,
        r.origin.y,
        stroke_attrs(stroke.color, stroke.width),
    );
}

fn emit_text(out: &mut String, n: &Node) {
    let Some(text) = n.text.as_deref() else { return };
    if text.is_empty() {
        return;
    }
    let r = normalize_rect(n.bounds);
    // Same 13/14 constants as `paint_text` + editor canvas so the
    // SVG renders identically to what the user sees on screen.
    let color = n.fill.unwrap_or(TEXT_DEFAULT_FILL);
    let baseline_y = r.origin.y + TEXT_BASELINE_OFFSET;
    let fill_attr = if color.a < 0.999 {
        format!(r#" fill="{}" fill-opacity="{}""#, color_to_rgb(color), color.a)
    } else {
        format!(r#" fill="{}""#, color_to_rgb(color))
    };
    let _ = write!(
        out,
        r#"<text x="{}" y="{baseline_y}" font-family="system-ui, sans-serif" font-size="{TEXT_FONT_SIZE}"{fill_attr}>{}</text>"#,
        r.origin.x,
        xml_escape(text),
    );
}

fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

fn emit_path(out: &mut String, n: &Node) {
    if n.points.len() < 2 {
        return;
    }
    let Some(stroke) = n.stroke else { return };
    let mut d = String::with_capacity(n.points.len() * 16);
    let _ = write!(d, "M{} {}", n.points[0].x, n.points[0].y);
    for p in &n.points[1..] {
        let _ = write!(d, " L{} {}", p.x, p.y);
    }
    let _ = write!(
        out,
        r#"<path d="{d}" fill="none"{}/>"#,
        stroke_attrs(stroke.color, stroke.width),
    );
}

fn fill_stroke_attrs(n: &Node) -> String {
    let mut s = String::new();
    if let Some(fill) = n.fill {
        let _ = write!(s, r#" fill="{}""#, color_to_rgb(fill));
        if fill.a < 0.999 {
            let _ = write!(s, r#" fill-opacity="{}""#, fill.a);
        }
    } else {
        s.push_str(r#" fill="none""#);
    }
    if let Some(stroke) = n.stroke {
        s.push_str(&stroke_attrs(stroke.color, stroke.width));
    }
    s
}

/// Stroke `color` + `width` attributes for any element that paints
/// a stroke (rect/ellipse/polygon/line/path). Emits `stroke-opacity`
/// when the colour's alpha < 1 so semi-transparent strokes round-
/// trip through SVG instead of collapsing to opaque.
fn stroke_attrs(color: Color, width: f32) -> String {
    let mut s = format!(r#" stroke="{}" stroke-width="{}""#, color_to_rgb(color), width);
    if color.a < 0.999 {
        let _ = write!(s, r#" stroke-opacity="{}""#, color.a);
    }
    s
}

fn color_to_rgb(c: Color) -> String {
    format!(
        "rgb({},{},{})",
        (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.b.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_format_extension_lookup() {
        assert_eq!(RasterFormat::from_extension("png"), Some(RasterFormat::Png));
        assert_eq!(RasterFormat::from_extension("jpg"), Some(RasterFormat::Jpeg));
        assert_eq!(RasterFormat::from_extension("jpeg"), Some(RasterFormat::Jpeg));
        assert_eq!(RasterFormat::from_extension("webp"), Some(RasterFormat::Webp));
        assert_eq!(RasterFormat::from_extension("svg"), None);
        assert_eq!(RasterFormat::from_extension("gif"), None);
        assert_eq!(RasterFormat::from_extension(""), None);
    }

    #[test]
    fn raster_format_jpeg_does_not_support_alpha() {
        assert!(RasterFormat::Png.supports_alpha());
        assert!(RasterFormat::Webp.supports_alpha());
        assert!(!RasterFormat::Jpeg.supports_alpha());
    }

    #[test]
    fn raster_format_quality_matches_ts() {
        // TS export-section.tsx: quality = 100 for PNG, 92 for JPEG/WEBP.
        assert_eq!(RasterFormat::Png.quality(), 100);
        assert_eq!(RasterFormat::Jpeg.quality(), 92);
        assert_eq!(RasterFormat::Webp.quality(), 92);
    }

    #[test]
    fn export_raster_writes_png_for_minimal_doc() {
        use openpencil_shell_core::document::{Node, NodeId, NodeKind};
        let mut doc = openpencil_shell_core::document::Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut n = Node::leaf(10, NodeKind::Rect, "r");
        n.bounds = Rect::xywh(0.0, 0.0, 100.0, 50.0);
        n.fill = Some(Color { r: 0.5, g: 0.5, b: 0.5, a: 1.0 });
        page.children.push(n);
        let tmp = std::env::temp_dir().join(format!("op-export-test-{}.png", std::process::id()));
        let res = export_raster(&doc, &tmp, RasterFormat::Png, 2.0);
        assert!(res.is_ok(), "export_raster PNG failed: {res:?}");
        let bytes = std::fs::read(&tmp).unwrap();
        // PNG signature: 89 50 4E 47 0D 0A 1A 0A
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        let _ = std::fs::remove_file(&tmp);
        let _ = NodeId::new(10); // silence import-used warning
    }

    #[test]
    fn export_raster_writes_jpeg_with_white_background() {
        use openpencil_shell_core::document::{Node, NodeKind};
        let mut doc = openpencil_shell_core::document::Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut n = Node::leaf(10, NodeKind::Rect, "r");
        n.bounds = Rect::xywh(0.0, 0.0, 80.0, 40.0);
        n.fill = Some(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        page.children.push(n);
        let tmp = std::env::temp_dir().join(format!("op-export-test-{}.jpg", std::process::id()));
        let res = export_raster(&doc, &tmp, RasterFormat::Jpeg, 1.0);
        assert!(res.is_ok(), "export_raster JPEG failed: {res:?}");
        let bytes = std::fs::read(&tmp).unwrap();
        // JPEG SOI marker: FF D8 FF
        assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF]);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn export_raster_scale_clamps_extreme_values() {
        use openpencil_shell_core::document::{Node, NodeKind};
        let mut doc = openpencil_shell_core::document::Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut n = Node::leaf(10, NodeKind::Rect, "r");
        n.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        n.fill = Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 });
        page.children.push(n);
        // Both extremes should succeed (clamped silently) rather than
        // allocating a gigapixel surface or zero-sized output.
        let tmp = std::env::temp_dir().join(format!("op-export-clamp-{}.png", std::process::id()));
        assert!(export_raster(&doc, &tmp, RasterFormat::Png, 0.001).is_ok());
        assert!(export_raster(&doc, &tmp, RasterFormat::Png, 1000.0).is_ok());
        let _ = std::fs::remove_file(&tmp);
    }
}
