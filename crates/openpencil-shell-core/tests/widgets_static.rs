//! Phase B1 widget facade smoke tests.
//!
//! Proves the `Widget` trait + `PaintCx` / `LayoutCx` shape compiles and
//! that a recording backend can be plugged in via the `&mut dyn
//! RenderBackend` field. B2 lands the four real widgets and extends this
//! file with per-widget paint-call assertions; today we only verify the
//! plumbing.

use openpencil_shell_core::widgets::{
    LayoutBox, LayoutCx, PaintCx, ROOT_WIDGET_ID, Widget, WidgetId, rect,
};
use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct RecordingBackend {
    rects: usize,
    strokes: usize,
    text: usize,
    saves: usize,
    restores: usize,
    clips: usize,
    translates: usize,
}

impl RenderBackend for RecordingBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _rect: Rect, _color: Color) {
        self.rects += 1;
    }
    fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {
        self.strokes += 1;
    }
    fn draw_text(&mut self, _layout: &TextLayout, _origin: Point2D) {
        self.text += 1;
    }
    fn clip_rect(&mut self, _rect: Rect) {
        self.clips += 1;
    }
    fn save(&mut self) {
        self.saves += 1;
    }
    fn restore(&mut self) {
        self.restores += 1;
    }
    fn translate(&mut self, _offset: Point2D) {
        self.translates += 1;
    }
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn paint_cx_dispatches_through_dyn_backend() {
    let mut backend = RecordingBackend::default();
    {
        let cx = PaintCx {
            backend: &mut backend,
        };
        cx.backend.fill_rect(rect(0.0, 0.0, 10.0, 10.0), Color::RED);
        cx.backend
            .stroke_rect(rect(1.0, 2.0, 8.0, 8.0), Color::WHITE, 1.5);
        cx.backend.save();
        cx.backend.translate(Point2D::new(2.0, 3.0));
        cx.backend.clip_rect(rect(0.0, 0.0, 4.0, 4.0));
        cx.backend.restore();
    }
    assert_eq!(backend.rects, 1, "fill_rect dispatch");
    assert_eq!(backend.strokes, 1, "stroke_rect dispatch");
    assert_eq!(backend.saves, 1, "save dispatch");
    assert_eq!(backend.restores, 1, "restore dispatch");
    assert_eq!(backend.translates, 1, "translate dispatch");
    assert_eq!(backend.clips, 1, "clip_rect dispatch");
}

/// A trivial widget impl proves the trait shape. Uses
/// `accesskit::Role::GenericContainer` (canonical "intentional placeholder",
/// ARIA `none`/`presentation`) so the test stays stable across unrelated
/// accesskit version bumps; B2 widgets use semantic roles.
struct StubWidget {
    id: WidgetId,
    box_rect: Rect,
}

impl Widget for StubWidget {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: self.box_rect,
        }
    }
    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        cx.backend.fill_rect(rect, Color::WHITE);
    }
    fn access_node(&self) -> accesskit::Node {
        // `GenericContainer` (ARIA `none` / `presentation`) is the canonical
        // "intentional placeholder" role. Real B2 widgets use semantic
        // roles (TreeItem / EditableText / etc); see codex B1 review NIT-5.
        accesskit::Node::new(accesskit::Role::GenericContainer)
    }
}

#[test]
fn widget_trait_dispatches_layout_and_paint() {
    let widget = StubWidget {
        id: WidgetId::new(7),
        box_rect: rect(0.0, 0.0, 100.0, 24.0),
    };
    let layout_cx = LayoutCx {
        available_width: 320.0,
        dpi: 1.0,
    };
    let layout = widget.layout(&layout_cx);
    assert_eq!(layout.rect.size.x, 100.0);

    let mut backend = RecordingBackend::default();
    {
        let mut paint_cx = PaintCx {
            backend: &mut backend,
        };
        widget.paint(&mut paint_cx, layout.rect);
    }
    assert_eq!(backend.rects, 1);
    assert_eq!(widget.id(), WidgetId::new(7));
    // Sanity check the root id constant survives codegen.
    assert_eq!(ROOT_WIDGET_ID.0, 0);

    // Trait surface check: access_node returns the placeholder
    // `Role::GenericContainer` advertised by `StubWidget`. Real B2 widgets
    // will assert their semantic roles (TreeItem / EditableText / etc).
    let node = widget.access_node();
    assert_eq!(node.role(), accesskit::Role::GenericContainer);
}
