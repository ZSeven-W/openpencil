//! Phase B1 + B2 widget facade smoke tests.
//!
//! B1 piece: proves the `Widget` trait + `PaintCx` / `LayoutCx` shape
//! compiles + that a recording backend plugs in via `&mut dyn
//! RenderBackend`. B2 piece: proves the remaining OP inspector widgets (Tree /
//! PropertyRow) paint and emit accesskit nodes with the expected semantic roles.

use op_editor_ui::widgets::{
    rect, LayoutBox, LayoutCx, PaintCx, PropertyRow, TreeWidget, Widget, WidgetId, ROOT_WIDGET_ID,
};
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

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
    fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {
        // Counted alongside strokes for the existing test asserts;
        // a separate counter is unnecessary for Step 4 visual lift.
        self.strokes += 1;
    }
    fn fill_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color) {
        self.rects += 1;
    }
    fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {
        self.strokes += 1;
    }
    fn stroke_svg_path(
        &mut self,
        _d: &str,
        _top_left: Point2D,
        _size: f32,
        _color: Color,
        _width: f32,
    ) {
        self.strokes += 1;
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
    // assert their semantic roles in the tests below.
    let node = widget.access_node();
    assert_eq!(node.role(), accesskit::Role::GenericContainer);
}

// ---------------------------------------------------------------------
// B2: inspector widgets paint static content + expose semantic
// accesskit roles.
// ---------------------------------------------------------------------

#[test]
fn inspector_widgets_paint_static_content() {
    let layout = LayoutCx {
        available_width: 240.0,
        dpi: 1.0,
    };
    let widgets: Vec<Box<dyn Widget>> = vec![
        Box::new(TreeWidget::sample()),
        Box::new(PropertyRow::new(200, "Width", "960")),
    ];
    let mut backend = RecordingBackend::default();
    for widget in widgets {
        let box_ = widget.layout(&layout);
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        widget.paint(&mut cx, box_.rect);
    }

    // Each widget paints at least its background fill (2); Tree adds one
    // more for the selected row. Stroked rects: PropertyRow strokes its
    // border. Text runs: PropertyRow has 2 (label+value); Tree has 3 items.
    assert!(
        backend.rects >= 3,
        "fill_rect dispatch >= 3 (got {})",
        backend.rects
    );
    assert!(
        backend.strokes >= 1,
        "stroke_rect dispatch >= 1 (got {})",
        backend.strokes
    );
    assert!(
        backend.text >= 5,
        "draw_text dispatch >= 5 (got {})",
        backend.text
    );
}

#[test]
fn tree_widget_advertises_tree_role_and_layers_label() {
    let tree = TreeWidget::sample();
    let node = tree.access_node();
    assert_eq!(node.role(), accesskit::Role::Tree);
    // accesskit::Node exposes label() returning Option<&str> in 0.24.
    assert_eq!(node.label(), Some("Layers"));
    // Sample tree has 3 items.
    assert_eq!(tree.items.len(), 3);
    assert!(tree.items.iter().any(|item| item.selected));
}

#[test]
fn property_row_advertises_label_and_value() {
    let row = PropertyRow::new(201, "Width", "960");
    let node = row.access_node();
    // `Role::Group` (not GenericContainer) so the label survives ARIA
    // filtering — see codex B2 R1 CONCERN + the fix in prop_row.rs.
    assert_eq!(node.role(), accesskit::Role::Group);
    assert_eq!(node.label(), Some("Width 960"));
}

#[test]
fn layer_panel_scroll_view_model_keeps_offsets_in_scroll_state() {
    let layer_panel = include_str!("../src/widgets/layer_panel.rs");
    let walkers = include_str!("../src/widgets/layer_panel_walkers.rs");
    for source in [layer_panel, walkers] {
        for needle in [
            "pages_scroll: f32",
            "layers_scroll: f32",
            "pages_h_scroll: f32",
            "layers_h_scroll: f32",
            "pages_max_scroll: f32",
            "layers_max_scroll: f32",
            "pages_max_h_scroll: f32",
            "layers_max_h_scroll: f32",
        ] {
            assert!(
                !source.contains(needle),
                "LayerPanel scroll state should not expose naked `{needle}` fields"
            );
        }
    }
}
