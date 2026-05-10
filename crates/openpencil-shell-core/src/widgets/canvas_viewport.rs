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

use crate::document::{Document, Node, NodeId, NodeKind};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub struct CanvasViewport<'a> {
    pub id: WidgetId,
    pub document: &'a Document,
    /// Background color of the canvas itself (the area outside
    /// any frame). Defaults to a light grey when constructed via
    /// `from_document`.
    pub canvas_background: Color,
}

impl<'a> CanvasViewport<'a> {
    /// Reserved WidgetId range 4000-4999 for the canvas viewport
    /// (per Step 1b convention: 1000s = LayerPanel, 2000s =
    /// PropertyPanel, 3000s = Toolbar, 4000s = canvas viewport).
    const CANVAS_BG: Color = Color {
        r: 0.94,
        g: 0.94,
        b: 0.94,
        a: 1.0,
    };

    pub fn from_document(document: &'a Document) -> Self {
        Self {
            id: WidgetId::new(4000),
            document,
            canvas_background: Self::CANVAS_BG,
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
        // 1. Paint the canvas background.
        cx.backend.fill_rect(rect, self.canvas_background);

        // 2. Walk the active page's nodes; each translated into
        //    the canvas rect (document-local origin → viewport
        //    origin offset). The host clips to `rect` upstream
        //    if needed; for Step 3 we trust the document's
        //    bounds to fit.
        if let Some(page) = self.document.active_page() {
            let viewport_origin = rect.origin;
            for child in &page.children {
                paint_node(cx, child, viewport_origin, self.document.selected);
            }
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Canvas);
        node.set_label("Canvas");
        node
    }
}

/// Recursive node painter. Translates document-space `bounds` by
/// `viewport_origin` so the canvas widget can be placed anywhere
/// in the host without document coords needing to know about it.
fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &Node,
    viewport_origin: Point2D,
    selected: NodeId,
) {
    let world_rect = Rect {
        origin: Point2D::new(
            viewport_origin.x + node.bounds.origin.x,
            viewport_origin.y + node.bounds.origin.y,
        ),
        size: node.bounds.size,
    };

    match &node.kind {
        NodeKind::Frame => {
            paint_fill_then_stroke(cx, node, world_rect);
            for child in &node.children {
                paint_node(cx, child, viewport_origin, selected);
            }
        }
        NodeKind::Group | NodeKind::Other(_) => {
            // Containers — no own paint; just recurse.
            for child in &node.children {
                paint_node(cx, child, viewport_origin, selected);
            }
        }
        NodeKind::Rect => {
            paint_fill_then_stroke(cx, node, world_rect);
        }
        NodeKind::Text => {
            if let Some(text) = node.text.as_deref() {
                let layout = TextLayout::single_run(
                    text,
                    "system-ui",
                    13.0,
                    jian_core::scene::Color::rgb(20, 20, 20),
                    Point2D::new(0.0, 0.0),
                );
                // Text origin = top-left of bounds shifted down
                // by ~font baseline (skia draws text from
                // baseline; ~10 px puts the baseline below the
                // bounds top so the rendered glyphs sit inside
                // the bounds rect).
                cx.backend.draw_text(
                    &layout,
                    Point2D::new(world_rect.origin.x, world_rect.origin.y + 14.0),
                );
            }
        }
    }

    // Selection highlight — drawn AFTER node paint so the blue
    // outline sits on top. Skip when bounds is zero (container
    // nodes that don't have their own visible rect).
    if node.id == selected && node.bounds.size.x > 0.0 && node.bounds.size.y > 0.0 {
        let highlight_color = Color {
            r: 0.18,
            g: 0.50,
            b: 1.0,
            a: 1.0,
        };
        cx.backend.stroke_rect(world_rect, highlight_color, 2.0);
    }
}

fn paint_fill_then_stroke(cx: &mut PaintCx<'_>, node: &Node, world_rect: Rect) {
    if let Some(fill) = node.fill {
        cx.backend.fill_rect(world_rect, fill);
    }
    if let Some(stroke) = node.stroke {
        cx.backend.stroke_rect(world_rect, stroke.color, stroke.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingBackend {
        rects: usize,
        strokes: usize,
        text: usize,
    }

    impl crate::RenderBackend for RecordingBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {
            self.rects += 1;
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
            self.strokes += 1;
        }
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {
            self.text += 1;
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
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
        assert!(backend.rects >= 3, "expected ≥3 fills, got {}", backend.rects);
        assert!(backend.strokes >= 2, "expected ≥2 strokes (frame + selection highlight), got {}", backend.strokes);
        assert_eq!(backend.text, 2, "two text nodes draw two text runs");
    }

    #[test]
    fn empty_document_paints_only_canvas_background() {
        let doc = Document::empty();
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 100.0, 100.0));
        }
        assert_eq!(backend.rects, 1, "one fill (canvas bg) on empty doc");
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
        };
        let viewport = CanvasViewport::from_document(&doc);
        let mut backend = RecordingBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            viewport.paint(&mut cx, Rect::xywh(0.0, 0.0, 200.0, 200.0));
        }
        // canvas bg (1) + leaf rect fill (1) = 2; group fill skipped.
        assert_eq!(backend.rects, 2);
    }
}
