//! Raster + SVG export for the active page.
//!
//! Renders top-level nodes on the active page of a [`LayoutScene`]
//! into an off-screen `skia_safe::Surface`, encodes as one of
//! PNG/JPEG/WEBP, writes to the user-chosen path. SVG goes through a
//! separate hand-rolled serializer (`export_svg`). Coverage:
//!   - Rect / Frame / Group : fill + stroke + corner_radius
//!   - Ellipse              : fill + stroke
//!   - Polygon              : default-triangle fill + stroke
//!   - Line                 : diagonal stroke across `bounds`
//!   - Path                 : polyline through `node.points`
//!   - Text                 : authored content at the resolved bounds
//!
//! Per-node rotation honoured via `Canvas::rotate`.
//!
//! The scene is layout-resolved: every `SceneNode::bounds` is the
//! absolute doc-space AABB jian's flex pass produced, and fills are
//! already `$ref`-resolved. Export draws straight from `bounds` with
//! no second layout pass — same input the on-screen canvas paints
//! (`canvas_viewport_paint.rs`).
//!
//! Background: PNG / WEBP transparent; JPEG forced white (TS parity
//! — JPEG has no alpha so a "transparent" JPEG would read as black).
//! Scale: caller picks @1x / @2x / @3x (TS export dialog parity).

use op_editor_ui::layout_scene::{regular_polygon_points, LayoutScene, SceneNode, ScenePage};
use op_editor_ui::layout_scene::{Effect, NodeKind};
use op_editor_ui::{Color, Point2D, Rect};
use skia_safe::{Canvas, EncodedImageFormat, Paint, PaintStyle, Path, PathBuilder};
use std::path::Path as StdPath;

mod export_svg;

pub use export_svg::export_svg;

const MARGIN: f32 = 16.0;

/// Mirrors `canvas_viewport_paint.rs` text paint constants so PNG
/// export matches what the user sees on the editor canvas at zoom 1.
/// `canvas_viewport_paint.rs` uses a 13 px default and a baseline at
/// `(size + 1) * zoom`, with `1.35 × size` line height.
const TEXT_DEFAULT_FONT_SIZE: f32 = 13.0;
const TEXT_DEFAULT_FILL: Color = Color {
    r: 0.08,
    g: 0.08,
    b: 0.08,
    a: 1.0,
};

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
    /// extensions. Used by future drag-drop / scripted-export paths.
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
/// to 2× (NaN reaching `canvas.scale` produces a garbage transform).
pub fn export_raster(
    scene: &LayoutScene,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
) -> Result<(), String> {
    let scale = clamp_scale(scale);
    let Some(page) = scene.active_page() else {
        return Err("no active page".into());
    };
    let bounds = page_bounds(page).ok_or("nothing to export")?;
    render_raster(bounds, target, format, scale, |canvas| {
        for node in &page.children {
            paint_node(canvas, node);
        }
    })
}

/// Raster-export a single node + its subtree by id — the "export this
/// layer" path (TS parity: `exportLayerToRaster`). The surface is
/// cropped to the node's painted bounds via the same `collect_bounds`
/// traversal `export_raster` uses for the whole page. Errors when the
/// id is unknown on the active page or the node paints nothing.
pub fn export_node_raster(
    scene: &LayoutScene,
    node_id: &str,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
) -> Result<(), String> {
    let scale = clamp_scale(scale);
    let Some(page) = scene.active_page() else {
        return Err("no active page".into());
    };
    let node = page
        .find(node_id)
        .ok_or_else(|| format!("node {node_id} not found on the active page"))?;
    let mut acc = BoundsAcc::new();
    collect_bounds(node, glam::Affine2::IDENTITY, &mut acc);
    let bounds = acc
        .into_rect()
        .ok_or_else(|| format!("node {node_id} paints nothing"))?;
    render_raster(bounds, target, format, scale, |canvas| {
        paint_node(canvas, node);
    })
}

/// Clamp a caller-supplied export scale to the @0.5x..@8x range,
/// defaulting a non-finite value to @2x (TS export-dialog parity).
fn clamp_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(0.5, 8.0)
    } else {
        2.0
    }
}

/// Shared surface-alloc + background-clear + encode + write path for
/// the whole-page and single-node raster exporters. `bounds` is the
/// painted-content rect (doc-space); `paint` draws into the canvas
/// after it has been scaled + translated so `bounds` sits at `MARGIN`.
fn render_raster(
    bounds: Rect,
    target: &StdPath,
    format: RasterFormat,
    scale: f32,
    paint: impl FnOnce(&Canvas),
) -> Result<(), String> {
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
    paint(canvas);
    let image = surface.image_snapshot();
    let data = image
        .encode(None, format.skia(), format.quality())
        .ok_or_else(|| format!("encode {} failed", format.user_label()))?;
    std::fs::write(target, data.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Bounding rect over every paintable node on `page`. The scene is
/// already layout-resolved, so each node's `bounds` is its absolute
/// doc-space AABB — but rotation, strokes and `node.points` can push
/// painted pixels outside `bounds`, so traversal mirrors `paint_node`
/// exactly and threads the cumulative transform.
pub(crate) fn page_bounds(page: &ScenePage) -> Option<Rect> {
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
/// transform so a child painted under a rotated container ends up in
/// the same world coords paint will emit. `canvas_viewport_paint.rs`
/// pivots rotation around the node's `aggregate_bounds()` (own bounds
/// when bounded, child union otherwise); this mirrors that exactly so
/// the surface never clips a row a downstream draw would touch.
fn collect_bounds(n: &SceneNode, parent_xform: glam::Affine2, acc: &mut BoundsAcc) {
    if n.hidden {
        return;
    }
    let pivot_rect = n.aggregate_bounds();
    let rotate_self =
        n.rotation.abs() > f32::EPSILON && (pivot_rect.size.x != 0.0 || pivot_rect.size.y != 0.0);
    let local_xform = if rotate_self {
        let pivot = glam::Vec2::new(
            pivot_rect.origin.x + pivot_rect.size.x * 0.5,
            pivot_rect.origin.y + pivot_rect.size.y * 0.5,
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
/// applies the cumulative parent+self transform; each returned point
/// gets pushed into the BoundsAcc as a world-space coord.
///
/// Returns `None` for invisible kinds: Group never paints own
/// content; Frame/Other contribute only when fill or stroke is set;
/// Path with empty `points` is invisible.
fn own_paint_corners(n: &SceneNode) -> Option<Vec<glam::Vec2>> {
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
        NodeKind::Frame => {
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
        NodeKind::Other(_) => {
            // `icon_font` and any other tagged kind paint no fill rect
            // in export; their bounds still bound the glyph for the
            // surface size when a fill is present.
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
            // Text bounds are the layout-resolved "where the glyphs
            // sit" rect. Real glyph extents can overshoot for tails /
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

/// Recursively paint one resolved [`SceneNode`] and its subtree onto
/// `canvas`. Mirrors `canvas_viewport_paint.rs::paint_node` at zoom 1
/// so the exported pixels match the on-screen canvas.
pub(crate) fn paint_node(canvas: &Canvas, node: &SceneNode) {
    if node.hidden {
        return;
    }
    let world_rect = node.bounds;
    let pre_rotate = canvas.save();
    // Rotation pivots around `aggregate_bounds` — own bounds when the
    // node is bounded, child union for unbounded containers — exactly
    // like the editor canvas painter.
    if node.rotation.abs() > f32::EPSILON {
        let pivot = node.aggregate_bounds();
        if pivot.size.x != 0.0 || pivot.size.y != 0.0 {
            let cx = pivot.origin.x + pivot.size.x / 2.0;
            let cy = pivot.origin.y + pivot.size.y / 2.0;
            canvas.rotate(node.rotation.to_degrees(), Some((cx, cy).into()));
        }
    }
    // Drop shadows paint behind the node's own fill — only for kinds
    // a rounded rect / ellipse silhouette can represent (parity with
    // canvas_viewport_paint.rs).
    if !node.effects.is_empty()
        && world_rect.size.x.abs() > 0.0
        && world_rect.size.y.abs() > 0.0
        && matches!(
            node.kind,
            NodeKind::Frame | NodeKind::Rect | NodeKind::Ellipse
        )
    {
        paint_drop_shadows(canvas, node, world_rect);
    }
    match &node.kind {
        NodeKind::Rect | NodeKind::Frame => {
            paint_rect(canvas, world_rect, node);
        }
        NodeKind::Other(_) => {
            // Tagged kinds (e.g. `icon_font`) have no rect silhouette
            // in export; skip own paint, still recurse children.
        }
        NodeKind::Ellipse => {
            paint_oval(canvas, world_rect, node);
        }
        NodeKind::Polygon => {
            paint_polygon(canvas, world_rect, node);
        }
        NodeKind::Line => {
            let (color, width) = match node.stroke {
                Some(s) => (s.color, s.width),
                None => (node.fill.unwrap_or(Color::BLACK), 1.5),
            };
            let paint = make_stroke(color, width);
            canvas.draw_line(
                (world_rect.origin.x, world_rect.origin.y),
                (
                    world_rect.origin.x + world_rect.size.x,
                    world_rect.origin.y + world_rect.size.y,
                ),
                &paint,
            );
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

/// Paint every `Effect::DropShadow` as a blurred shape behind the
/// node's fill. Mirrors `canvas_viewport_paint.rs::paint_drop_shadows`
/// at zoom 1.
fn paint_drop_shadows(canvas: &Canvas, node: &SceneNode, world_rect: Rect) {
    let nrect = normalize_rect(world_rect);
    let radius = if node.kind == NodeKind::Ellipse {
        nrect.size.x.min(nrect.size.y) / 2.0
    } else {
        node.corner_radius
    };
    for effect in &node.effects {
        let Effect::DropShadow(s) = effect;
        let shadow_rect = Rect {
            origin: Point2D::new(nrect.origin.x + s.offset_x, nrect.origin.y + s.offset_y),
            size: nrect.size,
        };
        let mut paint = make_fill(s.color);
        if s.blur > 0.0 {
            paint.set_mask_filter(skia_safe::MaskFilter::blur(
                skia_safe::BlurStyle::Normal,
                s.blur * 0.5,
                false,
            ));
        }
        let sk_rect = to_sk_rect(shadow_rect);
        if node.kind == NodeKind::Ellipse {
            canvas.draw_oval(sk_rect, &paint);
        } else if radius > 0.5 {
            canvas.draw_round_rect(sk_rect, radius, radius, &paint);
        } else {
            canvas.draw_rect(sk_rect, &paint);
        }
    }
}

fn paint_rect(canvas: &Canvas, rect: Rect, node: &SceneNode) {
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

fn paint_oval(canvas: &Canvas, rect: Rect, node: &SceneNode) {
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

fn paint_polygon(canvas: &Canvas, rect: Rect, node: &SceneNode) {
    let rect = normalize_rect(rect);
    if rect.size.x == 0.0 || rect.size.y == 0.0 {
        return;
    }
    let points = regular_polygon_points(rect, node.polygon_sides);
    let mut builder = PathBuilder::new();
    let Some(first) = points.first() else {
        return;
    };
    builder.move_to((first.x, first.y));
    for p in points.iter().skip(1) {
        builder.line_to((p.x, p.y));
    }
    builder.close();
    let path: Path = builder.detach();
    if let Some(fill) = node.fill {
        canvas.draw_path(&path, &make_fill(fill));
    }
    if let Some(stroke) = node.stroke {
        canvas.draw_path(&path, &make_stroke(stroke.color, stroke.width));
    }
}

fn paint_text(canvas: &Canvas, rect: Rect, node: &SceneNode) {
    let Some(text) = node.text.as_deref() else {
        return;
    };
    if text.is_empty() {
        return;
    }
    let r = normalize_rect(rect);
    let Some(typeface) =
        skia_safe::FontMgr::new().legacy_make_typeface(None, skia_safe::FontStyle::default())
    else {
        return;
    };
    // Match the editor canvas (canvas_viewport_paint.rs): authored
    // font size with a 13 px default, baseline at origin.y + (size+1),
    // fill defaults to the near-black ink colour. Multi-line text
    // splits on `\n` with a 1.35 × size line height.
    let base_size = if node.font_size > 0.0 {
        node.font_size
    } else {
        TEXT_DEFAULT_FONT_SIZE
    };
    let font = skia_safe::Font::from_typeface(typeface, base_size);
    let color = node.fill.unwrap_or(TEXT_DEFAULT_FILL);
    let paint = make_fill(color);
    let line_h = base_size * 1.35;
    let mut baseline_y = r.origin.y + base_size + 1.0;
    for line in text.split('\n') {
        canvas.draw_str(line, (r.origin.x, baseline_y), &font, &paint);
        baseline_y += line_h;
    }
}

fn paint_polyline(canvas: &Canvas, node: &SceneNode) {
    if node.points.len() < 2 {
        return;
    }
    let (color, width) = match node.stroke {
        Some(s) => (s.color, s.width),
        None => (node.fill.unwrap_or(Color::BLACK), 1.5),
    };
    let mut builder = PathBuilder::new();
    builder.move_to((node.points[0].x, node.points[0].y));
    for p in &node.points[1..] {
        builder.line_to((p.x, p.y));
    }
    let path: Path = builder.detach();
    canvas.draw_path(&path, &make_stroke(color, width));
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

/// Defensive normalisation — the layout pass yields positive-extent
/// rects, but a negative size would otherwise paint nothing.
fn normalize_rect(r: Rect) -> Rect {
    let x0 = r.origin.x.min(r.origin.x + r.size.x);
    let y0 = r.origin.y.min(r.origin.y + r.size.y);
    Rect {
        origin: Point2D::new(x0, y0),
        size: Point2D::new(r.size.x.abs(), r.size.y.abs()),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Shared helpers for the export test modules — build a
    //! `LayoutScene` directly without going through `op-pen-loader`.

    use op_editor_ui::layout_scene::NodeKind;
    use op_editor_ui::layout_scene::{LayoutScene, SceneNode, ScenePage};
    use op_editor_ui::{Color, Rect};

    /// A single-page scene holding `children` as top-level nodes.
    pub fn scene_with(children: Vec<SceneNode>) -> LayoutScene {
        LayoutScene {
            pages: vec![ScenePage {
                id: "p1".into(),
                name: "Page 1".into(),
                children,
            }],
            active_page_index: 0,
        }
    }

    /// A filled rectangle scene node at `(x, y, w, h)`.
    pub fn filled_rect(id: &str, x: f32, y: f32, w: f32, h: f32, fill: Color) -> SceneNode {
        let mut n = SceneNode::leaf(id, NodeKind::Rect);
        n.bounds = Rect::xywh(x, y, w, h);
        n.fill = Some(fill);
        n
    }
}

#[cfg(test)]
mod tests;
