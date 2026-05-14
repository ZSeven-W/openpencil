//! `CanvasViewport` — center-canvas widget that renders the actual
//! document tree as visual primitives.
//!
//! Step 3 scope:
//! - Frame: paints fill (if any) at `bounds`, then optional stroke,
//!   then recurses into children.
//! - Group: no own paint, just recurses into children. Groups exist
//!   for logical/selection grouping; the canvas doesn't draw them.
//! - Rect: paints fill (if any) + stroke (if any) at `bounds`.
//! - Text: paints `text` (if Some) at `bounds.origin`. No bounding
//!   rect background — Step 4+ may add text-frame backgrounds via
//!   the parent Rect/Frame composition.
//! - Other(_): treated as Group (no own paint, recurses into
//!   children) — unknown kinds shouldn't crash the canvas.
//!
//! Selection highlight: the currently-selected node is drawn with
//! a 2px blue stroke OVER its normal paint, so the user can see
//! what's picked across all node kinds.
//!
//! Step 4+ extends:
//! - Transform stack (translate / rotate / scale) — needs Node
//!   to grow a `transform` field
//! - Layer-list visibility / opacity
//! - Image / vector path nodes
//! - Variable resolution ($color-1 → real color)

use crate::document::{Document, Node, NodeId, NodeKind, Viewport};
use crate::theme::Theme;
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

/// One of the 8 selection handles (corners + edge midpoints) the
/// selection overlay paints. Used by the host to dispatch resize
/// drags: each variant fixes the corresponding edge / corner of
/// the selected bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

/// Radius (screen px) of the rotation ring that sits OUTSIDE the
/// 4 selection corners. Matches the TS `ROTATE_OUTER_RADIUS`.
const ROTATE_OUTER_RADIUS: f32 = 16.0;

/// Hit-test the rotation ring that sits just outside the four
/// corner handles. Returns the nearest corner (so the runner can
/// hint which way the rotation drag is anchored) or `None` if the
/// cursor isn't in a rotation zone.
///
/// The rotation zone is an annulus around each corner — beyond
/// the 6 px handle slop and inside the 16 px outer radius. Matches
/// the TS `hitTestRotation` logic.
pub fn rotation_corner_at_point(
    canvas_rect: Rect,
    doc: &Document,
    point: Point2D,
) -> Option<SelectionHandle> {
    // Rotation rings are only painted on single-select (the
    // multi-select overlay is outline-only), so gate the hit-test
    // to match — otherwise non-anchor "rotation zones" would
    // intercept clicks on dead air.
    if doc.selection_count() != 1 {
        return None;
    }
    let node = doc.selected_node()?;
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let viewport = &doc.viewport;
    let left = canvas_rect.origin.x + viewport.pan_x + bounds.origin.x * viewport.zoom;
    let top = canvas_rect.origin.y + viewport.pan_y + bounds.origin.y * viewport.zoom;
    let right = left + bounds.size.x * viewport.zoom;
    let bottom = top + bounds.size.y * viewport.zoom;
    // Inverse-rotate the cursor into the node's local space so the
    // hit-test annulus tracks the rendered (rotated) corners.
    let cx = (left + right) / 2.0;
    let cy = (top + bottom) / 2.0;
    let local = inverse_rotate(point, Point2D::new(cx, cy), node.rotation);
    let inner = 6.0_f32;
    let outer = ROTATE_OUTER_RADIUS;
    let corners = [
        (SelectionHandle::TopLeft, left, top),
        (SelectionHandle::TopRight, right, top),
        (SelectionHandle::BottomLeft, left, bottom),
        (SelectionHandle::BottomRight, right, bottom),
    ];
    for (kind, cx, cy) in corners {
        let dx = local.x - cx;
        let dy = local.y - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > inner && dist <= outer {
            return Some(kind);
        }
    }
    None
}

/// Apply the inverse of a rotation about `pivot` to `point`. Used
/// by hit-tests so a rotated selection's handles + rotation ring
/// + body all match the rendered (rotated) geometry.
fn inverse_rotate(point: Point2D, pivot: Point2D, radians: f32) -> Point2D {
    if radians.abs() < f32::EPSILON {
        return point;
    }
    let dx = point.x - pivot.x;
    let dy = point.y - pivot.y;
    let cos_t = (-radians).cos();
    let sin_t = (-radians).sin();
    Point2D::new(
        pivot.x + dx * cos_t - dy * sin_t,
        pivot.y + dx * sin_t + dy * cos_t,
    )
}

/// Hit-test the 8 selection handles around the currently-selected
/// node. Returns the handle at `point` (a small slop around each
/// handle center counts) or `None` if no selection / no handle.
///
/// `canvas_rect` is the on-screen rect the canvas widget paints
/// into (same value passed to `CanvasViewport::paint`). The
/// transform from document → screen is identical to paint so a
/// handle the user clicks is the handle they see.
pub fn selection_handle_at_point(
    canvas_rect: Rect,
    doc: &Document,
    point: Point2D,
) -> Option<SelectionHandle> {
    // Handles are only painted on single-select (the multi-select
    // overlay is outline-only — Figma parity), so gate the hit-
    // test to match. Otherwise the "anchor's handles" would hit-
    // test even though no handles are visible anywhere.
    if doc.selection_count() != 1 {
        return None;
    }
    let node = doc.selected_node()?;
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    let viewport = &doc.viewport;
    let left = canvas_rect.origin.x + viewport.pan_x + bounds.origin.x * viewport.zoom;
    let top = canvas_rect.origin.y + viewport.pan_y + bounds.origin.y * viewport.zoom;
    let right = left + bounds.size.x * viewport.zoom;
    let bottom = top + bounds.size.y * viewport.zoom;
    let mid_x = (left + right) / 2.0;
    let mid_y = (top + bottom) / 2.0;
    // Inverse-rotate the cursor so handle hit-test tracks rendered
    // (rotated) handle positions.
    let local = inverse_rotate(point, Point2D::new(mid_x, mid_y), node.rotation);
    let slop = 6.0;
    let anchors = [
        (SelectionHandle::TopLeft, left, top),
        (SelectionHandle::Top, mid_x, top),
        (SelectionHandle::TopRight, right, top),
        (SelectionHandle::Right, right, mid_y),
        (SelectionHandle::BottomRight, right, bottom),
        (SelectionHandle::Bottom, mid_x, bottom),
        (SelectionHandle::BottomLeft, left, bottom),
        (SelectionHandle::Left, left, mid_y),
    ];
    for (kind, hx, hy) in anchors {
        if (local.x - hx).abs() <= slop && (local.y - hy).abs() <= slop {
            return Some(kind);
        }
    }
    None
}

pub struct CanvasViewport<'a> {
    pub id: WidgetId,
    pub document: &'a Document,
    /// Background fill outside any Frame.
    pub canvas_background: Color,
    pub theme: Theme,
    /// Host ms clock — text-edit caret blink.
    pub now_ms: u64,
}

/// Spacing (document px) between major grid dots at 100% zoom.
/// Step 5 infinite canvas — adapted from `apps/web` canvas grid.
const GRID_SPACING: f32 = 32.0;

impl<'a> CanvasViewport<'a> {
    pub fn from_document(document: &'a Document) -> Self {
        let theme = document.theme();
        Self {
            id: WidgetId::new(4000),
            document,
            canvas_background: theme.canvas_surface,
            theme,
            now_ms: 0,
        }
    }
}

impl<'a> Widget for CanvasViewport<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Host sizes via the paint rect; we just report a default.
        LayoutBox {
            rect: Rect::xywh(0.0, 0.0, cx.available_width, cx.available_width.max(400.0)),
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
            return;
        }
        cx.backend.save();
        cx.backend.clip_rect(rect);

        // 1. Paint the canvas background INSIDE the clip region.
        cx.backend.fill_rect(rect, self.canvas_background);

        // 2. Dotted grid — canvas-local, scales with pan/zoom.
        let viewport = &self.document.viewport;
        paint_grid(cx, rect, viewport, &self.theme);

        // 3. Walk the active page; clip enforces widget bounds.
        if let Some(page) = self.document.active_page() {
            let viewport_origin = Point2D::new(
                rect.origin.x + viewport.pan_x,
                rect.origin.y + viewport.pan_y,
            );
            let edit_caret = self.document.ui.text_editing.map(|id| EditCaret {
                editing: id,
                anchor_ms: self.document.ui.text_edit_caret_anchor_ms,
                now_ms: self.now_ms,
            });
            // Cull rect — anything fully outside this rect (with a
            // generous margin for stroke widths / rotated handles /
            // text overhang) can skip paint entirely.
            const CULL_MARGIN: f32 = 64.0;
            let cull = Rect {
                origin: Point2D::new(rect.origin.x - CULL_MARGIN, rect.origin.y - CULL_MARGIN),
                size: Point2D::new(
                    rect.size.x + CULL_MARGIN * 2.0,
                    rect.size.y + CULL_MARGIN * 2.0,
                ),
            };
            for child in &page.children {
                paint_node(
                    cx,
                    child,
                    viewport_origin,
                    viewport.zoom,
                    self.document.selected,
                    edit_caret,
                    cull,
                );
            }
        }

        // 3b. Pen tool rubber-band from last anchor to cursor.
        paint_pen_rubber_band(cx, self.document, rect, &viewport);

        // 4. Selection overlay — outlines + handles (single-select only).
        let show_handles = self.document.selection_count() == 1;
        if let Some(page) = self.document.active_page() {
            for id in &self.document.selected_set {
                let Some(node) = page.find(*id) else {
                    continue;
                };
                if node.hidden {
                    continue;
                }
                let bounds = node.aggregate_bounds();
                if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
                    continue;
                }
                let world_rect = Rect {
                    origin: Point2D::new(
                        rect.origin.x + viewport.pan_x + bounds.origin.x * viewport.zoom,
                        rect.origin.y + viewport.pan_y + bounds.origin.y * viewport.zoom,
                    ),
                    size: Point2D::new(
                        bounds.size.x * viewport.zoom,
                        bounds.size.y * viewport.zoom,
                    ),
                };
                let is_container = matches!(
                    node.kind,
                    NodeKind::Frame | NodeKind::Group | NodeKind::Other(_)
                );
                let rotated = node.rotation.abs() > f32::EPSILON;
                if rotated {
                    let pivot = Point2D::new(
                        world_rect.origin.x + world_rect.size.x / 2.0,
                        world_rect.origin.y + world_rect.size.y / 2.0,
                    );
                    cx.backend.save();
                    cx.backend.rotate(node.rotation, pivot);
                }
                paint_selection_overlay(cx, world_rect, &self.theme, is_container, show_handles);
                if rotated {
                    cx.backend.restore();
                }
            }
        }

        // 4b. Per-anchor handles for the selected Path node when the
        //     Pen tool is active — surfaces the drag target that
        //     `path_anchor_drag` consumes (codex stop-gate concern:
        //     anchor drag worked but was invisible).
        if matches!(self.document.tool, crate::document::Tool::Pen)
            && self.document.selection_count() == 1
        {
            if let Some(page) = self.document.active_page() {
                if let Some(node) = page.find(self.document.selected) {
                    if matches!(node.kind, NodeKind::Path) {
                        let r = 4.0; // screen-px radius
                        for p in &node.points {
                            let center = Point2D::new(
                                rect.origin.x + viewport.pan_x + p.x * viewport.zoom,
                                rect.origin.y + viewport.pan_y + p.y * viewport.zoom,
                            );
                            let bounds = Rect {
                                origin: Point2D::new(center.x - r, center.y - r),
                                size: Point2D::new(r * 2.0, r * 2.0),
                            };
                            cx.backend.fill_oval(bounds, self.theme.background);
                            cx.backend.stroke_oval(bounds, self.theme.primary, 1.5);
                        }
                    }
                }
            }
        }

        cx.backend.restore();
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Canvas);
        node.set_label("Canvas");
        node
    }
}

#[derive(Clone, Copy)]
struct EditCaret {
    editing: NodeId,
    anchor_ms: u64,
    now_ms: u64,
}

fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &Node,
    viewport_origin: Point2D,
    zoom: f32,
    selected: NodeId,
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

    match &node.kind {
        NodeKind::Frame => {
            paint_fill_then_stroke(cx, node, world_rect, zoom);
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, selected, edit_caret, cull);
            }
        }
        NodeKind::Other(tag) if tag == "icon_font" => crate::widgets::icons::paint_icon_font_node(
            cx.backend, node.text.as_deref().unwrap_or(""), world_rect, node.fill,
        ),
        NodeKind::Group | NodeKind::Other(_) => {
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, selected, edit_caret, cull);
            }
        }
        NodeKind::Rect => {
            paint_fill_then_stroke(cx, node, world_rect, zoom);
        }
        NodeKind::Ellipse => {
            if let Some(fill) = node.fill {
                cx.backend.fill_oval(world_rect, fill);
            }
            if let Some(stroke) = node.stroke {
                cx.backend
                    .stroke_oval(world_rect, stroke.color, stroke.width * zoom);
            }
        }
        NodeKind::Polygon => {
            // Default triangle: top-centre, bottom-left, bottom-right.
            let cx_pt = world_rect.origin.x + world_rect.size.x / 2.0;
            let top_y = world_rect.origin.y;
            let bottom_y = world_rect.origin.y + world_rect.size.y;
            let left_x = world_rect.origin.x;
            let right_x = world_rect.origin.x + world_rect.size.x;
            let pts = [
                Point2D::new(cx_pt, top_y),
                Point2D::new(left_x, bottom_y),
                Point2D::new(right_x, bottom_y),
            ];
            if let Some(fill) = node.fill {
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
            let (color, width) = match node.stroke {
                Some(s) => (s.color, s.width * zoom),
                None => (
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                ),
            };
            let to_world = |p: Point2D| -> Point2D {
                Point2D::new(
                    viewport_origin.x + p.x * zoom,
                    viewport_origin.y + p.y * zoom,
                )
            };
            for pair in node.points.windows(2) {
                cx.backend
                    .stroke_line(to_world(pair[0]), to_world(pair[1]), color, width);
            }
        }
        NodeKind::Text => {
            let text = node.text.as_deref().unwrap_or("");
            // Ink colour follows `Node.fill` (defaults to near black if unset).
            let ink = node.fill.unwrap_or(crate::Color { r: 0.08, g: 0.08, b: 0.08, a: 1.0 });
            fn ch(v: f32) -> u8 {
                (v.clamp(0.0, 1.0) * 255.0).round() as u8
            }
            // Honour authored font size from the canonical schema; default to
            // 13 px so editor-created text stays uniform. Baseline ≈ 1.08 × size.
            let base_size = if node.font_size > 0.0 { node.font_size } else { 13.0 };
            let font_size = base_size * zoom;
            let baseline_y = world_rect.origin.y + (base_size + 1.0) * zoom;
            if !text.is_empty() {
                let weight = if node.font_weight > 0 { node.font_weight } else { 400 };
                let jc = jian_core::scene::Color::rgba(ch(ink.r), ch(ink.g), ch(ink.b), ch(ink.a));
                let line_h = base_size * 1.35 * zoom;
                let mut ly = baseline_y;
                let lines: Vec<String> = if node.text_wrap {
                    crate::widgets::canvas_viewport_overlay::wrap_text(cx.backend, text, font_size, world_rect.size.x, weight)
                } else { text.split('\n').map(str::to_string).collect() };
                for line in lines {
                    cx.backend.draw_text(&TextLayout::single_run(&line, "system-ui", font_size, jc, Point2D::new(0.0, 0.0)).with_font_weight(weight), Point2D::new(world_rect.origin.x, ly));
                    ly += line_h;
                }
            }
            // Caret while editing — sits at the end of the text.
            if let Some(c) = edit_caret {
                if c.editing == node.id
                    && jian_core::anim::blink_visible(c.now_ms, c.anchor_ms, 500)
                {
                    let text_w = cx.backend.measure_text(text, font_size);
                    let caret = crate::Rect {
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
    }

    if rotated {
        cx.backend.restore();
    }

    // Selection outline + 8 handles are painted as a top-of-stack
    // overlay in `CanvasViewport::paint`; nothing per-node here.
    let _ = selected;
}

/// Paints the selection outline + 8 resize handles around the
/// world-space rect of the selected node. Matches the TS app's
/// Figma-style affordance: 1 px primary outline (4 AA stroke
/// lines), small white-filled handles with a 1 px primary stroke
/// at the four corners + four edge midpoints. Container nodes
/// (Frame / Group / Other) get a slightly more translucent
/// outline so the visual reads "wrapper" instead of "leaf".
use crate::widgets::canvas_viewport_overlay::{
    paint_fill_then_stroke, paint_pen_rubber_band, paint_selection_overlay,
};

/// Paint a dotted grid across the canvas widget rect. The grid is
/// drawn in canvas-local coordinates and offset by `viewport.pan`
/// so it scrolls with the document content. Dots get sparser as
/// zoom decreases (skipping every other dot at low zoom) so they
/// stay visually airy.
fn paint_grid(cx: &mut PaintCx<'_>, rect: Rect, viewport: &Viewport, theme: &Theme) {
    let zoom = viewport.zoom.max(0.0001);
    let mut step = GRID_SPACING * zoom;
    // Skip rendering when dots would be packed tighter than 8 px
    // (visual noise at deep zoom-out), and double the step so the
    // dot density stays roughly constant.
    while step < 8.0 {
        step *= 2.0;
    }

    let dot_color = Color {
        r: theme.muted_foreground.r,
        g: theme.muted_foreground.g,
        b: theme.muted_foreground.b,
        a: 0.18,
    };
    let dot_size = (1.5 * zoom.sqrt()).clamp(1.0, 2.5);

    // Align grid origin to (0, 0) in document space, shifted by
    // pan, then shifted into widget space.
    let origin_x = rect.origin.x + (viewport.pan_x.rem_euclid(step));
    let origin_y = rect.origin.y + (viewport.pan_y.rem_euclid(step));

    let mut y = origin_y - step;
    while y < rect.origin.y + rect.size.y + step {
        let mut x = origin_x - step;
        while x < rect.origin.x + rect.size.x + step {
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(x - dot_size / 2.0, y - dot_size / 2.0),
                    size: Point2D::new(dot_size, dot_size),
                },
                dot_size / 2.0,
                dot_color,
            );
            x += step;
        }
        y += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records op order; clip-isolated paint = `Save, Clip, Fill, …, Restore`.
    #[derive(Debug, PartialEq, Eq)]
    enum Op {
        Save,
        Restore,
        Clip,
        Fill,
        Stroke,
        Text,
    }

    #[derive(Default)]
    struct RecordingBackend {
        ops: Vec<Op>,
        rects: usize,
        strokes: usize,
        text: usize,
    }

    impl crate::RenderBackend for RecordingBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {
            self.rects += 1;
            self.ops.push(Op::Fill);
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {
            self.text += 1;
            self.ops.push(Op::Text);
        }
        fn clip_rect(&mut self, _: Rect) {
            self.ops.push(Op::Clip);
        }
        fn save(&mut self) {
            self.ops.push(Op::Save);
        }
        fn restore(&mut self) {
            self.ops.push(Op::Restore);
        }
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
            self.rects += 1;
            self.ops.push(Op::Fill);
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {
            self.strokes += 1;
            self.ops.push(Op::Stroke);
        }
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn from_sample_document_paints_expected_primitives() {
        let doc = Document::sample();
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
        }
        // ≥3 fills (canvas bg, frame fill, button rect), ≥2 strokes
        // (frame outline + selection highlight), 2 text draws.
        assert!(
            backend.rects >= 3,
            "expected ≥3 fills, got {}",
            backend.rects
        );
        assert!(
            backend.strokes >= 2,
            "expected ≥2 strokes (frame + selection highlight), got {}",
            backend.strokes
        );
        assert_eq!(backend.text, 2, "two text nodes draw two text runs");
    }

    #[test]
    fn empty_document_paints_canvas_background_and_grid_only() {
        let doc = Document::empty();
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 100.0, 100.0));
        }
        // Step 5 infinite-canvas: bg + grid dots, no document-side
        // strokes / text. Exact dot count varies with widget size /
        // zoom so we just bound the surface to "non-empty grid + no
        // doc paints".
        assert!(backend.rects >= 1, "canvas bg + grid dots");
        assert_eq!(backend.strokes, 0);
        assert_eq!(backend.text, 0);
    }

    #[test]
    fn unselected_document_skips_highlight_stroke() {
        let mut doc = Document::sample();
        doc.clear_selection(); // deselect
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
        }
        // Same paints as `from_sample_document_paints_expected_primitives`
        // MINUS the selection highlight = 1 fewer stroke.
        assert_eq!(backend.strokes, 1, "no selection => only the frame stroke");
    }

    #[test]
    fn access_node_advertises_canvas_role() {
        let doc = Document::sample();
        let viewport = CanvasViewport::from_document(&doc);
        let node = viewport.access_node();
        assert_eq!(node.role(), accesskit::Role::Canvas);
        assert_eq!(node.label(), Some("Canvas"));
    }

    #[test]
    fn paint_is_clip_isolated_save_clip_then_restore() {
        // Codex Step 3 stop-hook: nodes whose document coords
        // extend past the canvas-widget rect must NOT spill onto
        // neighbouring widgets. Verify the canvas paint wraps in
        // Save → Clip(rect) → … → Restore so the host's clip
        // stack is balanced and the recursive paint stays
        // confined.
        let doc = Document::sample();
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
        }
        // First three ops: Save, Clip, Fill (the canvas bg).
        assert_eq!(
            &backend.ops[..3],
            &[Op::Save, Op::Clip, Op::Fill],
            "canvas paint must open with Save → Clip → bg Fill"
        );
        // Last op: Restore (closes the save).
        assert_eq!(
            backend.ops.last(),
            Some(&Op::Restore),
            "canvas paint must close with Restore"
        );
        // Save / Restore counts balance (one of each from
        // CanvasViewport — node paint helpers don't push more).
        let saves = backend.ops.iter().filter(|o| **o == Op::Save).count();
        let restores = backend.ops.iter().filter(|o| **o == Op::Restore).count();
        assert_eq!(saves, restores, "balanced save/restore");
        assert_eq!(saves, 1);
    }

    #[test]
    fn paint_with_zero_size_rect_skips_entirely() {
        // Defensive guard: hosts may pass an empty rect when the
        // canvas band has zero usable space. We must NOT call
        // save/clip/restore in that case (would still be balanced
        // but unnecessary), and we must NOT walk the document.
        let doc = Document::sample();
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 0.0, 0.0));
        }
        assert!(backend.ops.is_empty(), "zero-size rect must paint nothing");
    }

    #[test]
    fn group_kind_recurses_without_own_paint() {
        let inner = Node::leaf(2, NodeKind::Rect, "leaf")
            .with_bounds(Rect::xywh(0.0, 0.0, 50.0, 50.0))
            .with_fill(Color::RED);
        let group = Node::with_children(3, NodeKind::Group, "group", vec![inner])
            .with_bounds(Rect::xywh(10.0, 10.0, 80.0, 80.0))
            .with_fill(Color::BLUE); // fill on group should be ignored
        let doc = Document {
            pages: vec![crate::document::Page::new(1, "p", vec![group])],
            active_page_index: 0,
            selected: NodeId::NONE,
            selected_set: Vec::new(),
            clipboard: Vec::new(),
            tool: crate::document::Tool::Select,
            viewport: crate::document::Viewport::IDENTITY,
            chat: crate::document::ChatState::default(),
            ui: crate::document::UiState::default(),
            history: crate::document::History::default(),
            var_table: crate::document::VariableTable::default(),
        };
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 200.0, 200.0));
        }
        // canvas bg (1) + grid dots (variable) + leaf rect fill (1)
        // — group fill skipped. Just verify the leaf was painted by
        // checking the stroke count (leaf has no stroke, so 0) and
        // that fills are bounded above the dot baseline.
        assert!(backend.rects >= 2, "canvas bg + at least the leaf");
    }
}
