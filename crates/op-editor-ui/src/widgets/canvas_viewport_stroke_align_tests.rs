//! Sibling test file for `canvas_viewport_paint.rs` — stroke-align
//! placement (INSIDE / CENTER / OUTSIDE) against the shared
//! `align_stroke_rect` painter path.

use crate::layout_scene::{NodeKind, SceneNode, SceneStroke, SceneStrokeAlign};
use crate::widgets::canvas_viewport_paint::paint_node_with_options;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct StrokeCaptureBackend {
    strokes: Vec<(f32, f32, f32, f32, f32)>,
}

impl RenderBackend for StrokeCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, rect: Rect, _: Color, w: f32) {
        self.strokes
            .push((rect.origin.x, rect.origin.y, rect.size.x, rect.size.y, w));
    }
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn stroked_rect(align: SceneStrokeAlign) -> SceneNode {
    let mut n = SceneNode::leaf("s", NodeKind::Rect);
    n.bounds = Rect::xywh(0.0, 0.0, 100.0, 50.0);
    n.stroke = Some(SceneStroke {
        color: Color::BLACK,
        width: 4.0,
        sides: None,
        align,
    });
    n
}

fn paint(node: &SceneNode) -> Vec<(f32, f32, f32, f32, f32)> {
    let mut backend = StrokeCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    let _ = paint_node_with_options(
        &mut cx,
        node,
        Point2D::ZERO,
        1.0,
        None,
        Rect::xywh(0.0, 0.0, 4000.0, 4000.0),
        None,
        None,
        None,
        None,
    );
    backend.strokes
}

/// Figma's default stroke is INSIDE: the painted band must sit
/// entirely within the node bounds, so the centered stroke_rect call
/// gets a half-width inset.
#[test]
fn inside_stroke_insets_by_half_width() {
    let strokes = paint(&stroked_rect(SceneStrokeAlign::Inside));
    assert_eq!(strokes, vec![(2.0, 2.0, 96.0, 46.0, 4.0)]);
}

/// OUTSIDE strokes outset by half a width.
#[test]
fn outside_stroke_outsets_by_half_width() {
    let strokes = paint(&stroked_rect(SceneStrokeAlign::Outside));
    assert_eq!(strokes, vec![(-2.0, -2.0, 104.0, 54.0, 4.0)]);
}

/// CENTER keeps the authored rect.
#[test]
fn center_stroke_keeps_rect() {
    let strokes = paint(&stroked_rect(SceneStrokeAlign::Center));
    assert_eq!(strokes, vec![(0.0, 0.0, 100.0, 50.0, 4.0)]);
}
