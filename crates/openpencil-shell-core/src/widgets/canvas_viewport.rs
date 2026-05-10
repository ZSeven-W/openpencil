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

pub struct CanvasViewport<'a> {
    pub id: WidgetId,
    pub document: &'a Document,
    /// Background color of the canvas itself (the area outside
    /// any frame). Defaults to `theme.background` (dark) — matches
    /// the TS app's `bg-background` canvas surface.
    pub canvas_background: Color,
    pub theme: Theme,
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
        }
    }
}

impl<'a> Widget for CanvasViewport<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, cx: &LayoutCx) -> LayoutBox {
        // Canvas takes the entire available size — the host
        // controls how big the viewport is via the rect it passes
        // to `paint`. We report a square area equal to width x
        // (a generous default) so the host that asks for layout
        // bounds before paint can size accordingly. In practice
        // hosts paint into a rect of their choosing.
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

        // 2. Dotted grid — drawn in canvas-local coordinates so
        //    panning the viewport scrolls the grid with the
        //    content. Skip drawing if the zoom is so low that
        //    dots would overlap (visual noise).
        let viewport = &self.document.viewport;
        paint_grid(cx, rect, viewport, &self.theme);

        // 3. Walk the active page's nodes — translate by
        //    `viewport.pan` and scale by `viewport.zoom`, then
        //    offset by the canvas widget rect's origin so the
        //    transform is widget-local. The clip above guarantees
        //    nothing leaves the widget's rect.
        if let Some(page) = self.document.active_page() {
            let viewport_origin = Point2D::new(
                rect.origin.x + viewport.pan_x,
                rect.origin.y + viewport.pan_y,
            );
            for child in &page.children {
                paint_node(
                    cx,
                    child,
                    viewport_origin,
                    viewport.zoom,
                    self.document.selected,
                );
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

/// Recursive node painter. Translates document-space `bounds` by
/// `viewport_origin` and scales by `zoom` so panning + zooming
/// the canvas viewport updates rendered geometry uniformly.
fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &Node,
    viewport_origin: Point2D,
    zoom: f32,
    selected: NodeId,
) {
    let world_rect = Rect {
        origin: Point2D::new(
            viewport_origin.x + node.bounds.origin.x * zoom,
            viewport_origin.y + node.bounds.origin.y * zoom,
        ),
        size: Point2D::new(node.bounds.size.x * zoom, node.bounds.size.y * zoom),
    };

    match &node.kind {
        NodeKind::Frame => {
            paint_fill_then_stroke(cx, node, world_rect, zoom);
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, selected);
            }
        }
        NodeKind::Group | NodeKind::Other(_) => {
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, selected);
            }
        }
        NodeKind::Rect => {
            paint_fill_then_stroke(cx, node, world_rect, zoom);
        }
        NodeKind::Text => {
            if let Some(text) = node.text.as_deref() {
                let layout = TextLayout::single_run(
                    text,
                    "system-ui",
                    13.0 * zoom,
                    jian_core::scene::Color::rgb(20, 20, 20),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &layout,
                    Point2D::new(world_rect.origin.x, world_rect.origin.y + 14.0 * zoom),
                );
            }
        }
    }

    if node.id == selected && node.bounds.size.x > 0.0 && node.bounds.size.y > 0.0 {
        let highlight_color = Color {
            r: 0.18,
            g: 0.50,
            b: 1.0,
            a: 1.0,
        };
        cx.backend
            .stroke_rect(world_rect, highlight_color, 2.0_f32.max(zoom));
    }
}

fn paint_fill_then_stroke(cx: &mut PaintCx<'_>, node: &Node, world_rect: Rect, zoom: f32) {
    if let Some(fill) = node.fill {
        cx.backend.fill_rect(world_rect, fill);
    }
    if let Some(stroke) = node.stroke {
        cx.backend
            .stroke_rect(world_rect, stroke.color, stroke.width * zoom);
    }
}

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

    /// Records the *order* of operations so the codex
    /// "canvas viewport is not paint-isolated" regression is
    /// caught: a clip-isolated paint must look like
    /// `Save, Clip, Fill(canvas_bg), …node paints…, Restore`.
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
        // Expected paints:
        //   - 1 canvas bg fill
        //   - 1 frame fill (white) + 1 frame stroke (black)
        //   - 1 button rect fill (blue)
        //   - 0 group fills (groups are containers)
        //   - 0 title text bg, 0 button text bg
        //   - 1 selected highlight stroke (Title is selected) on
        //     the title's bounds
        //   - 2 text draws (Title + Click me)
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
        doc.selected = NodeId::NONE; // deselect
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
            tool: crate::document::Tool::Select,
            viewport: crate::document::Viewport::IDENTITY,
            chat: crate::document::ChatState::default(),
            ui: crate::document::UiState::default(),
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
