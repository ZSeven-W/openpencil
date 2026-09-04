//! Focus-caret visual regressions kept separate from `tests.rs` so both files
//! remain below the repository's 800-line limit.

#![cfg(test)]

use super::{test_measure, PreviewSession};
use op_editor_ui::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct CaretRecorder {
    lines: Vec<Color>,
}

impl RenderBackend for CaretRecorder {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn scale(&mut self, _: Point2D, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, color: Color, _: f32) {
        self.lines.push(color);
    }
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
    fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
    fn measure_text_weighted(&mut self, _: &str, _: f32, _: u16) -> f32 {
        0.0
    }
}

fn dark_input_doc(opacity: f32) -> jian_ops_schema::PenDocument {
    let source = format!(
        r##"{{
            "version": "1.1",
            "formatVersion": "1.1",
            "id": "x",
            "app": {{ "name": "x", "version": "1", "id": "x" }},
            "children": [{{
                "type": "text_input",
                "id": "field",
                "width": 200,
                "height": 40,
                "value": "hello",
                "opacity": {opacity},
                "fill": [{{ "type": "solid", "color": "#180b2a" }}],
                "stroke": {{
                    "thickness": 1,
                    "fill": [{{ "type": "solid", "color": "#724aa0" }}]
                }}
            }}]
        }}"##
    );
    jian_ops_schema::load_str(&source)
        .expect("parse dark input")
        .value
}

fn paint_caret(opacity: f32) -> Color {
    let doc = dark_input_doc(opacity);
    let mut session = PreviewSession::enter(
        &doc,
        (400.0, 200.0),
        &Default::default(),
        0,
        false,
        false,
        test_measure(),
    )
    .expect("enter preview");
    session.set_now_ms(0);
    session.focus_next();
    let scene = session.preview_scene_for_test();
    let mut recorder = CaretRecorder::default();
    session.paint_focus_caret(&mut recorder, &scene, Point2D::ZERO, 1.0, 0);
    assert_eq!(recorder.lines.len(), 1, "one focused caret line");
    recorder.lines[0]
}

#[test]
fn dark_authored_input_gets_light_focused_caret() {
    assert_eq!(paint_caret(1.0), Color::WHITE);
}

#[test]
fn focused_caret_applies_scene_opacity_exactly_once() {
    assert_eq!(paint_caret(0.5).to_jian().a(), 128);
    assert_eq!(paint_caret(0.0).to_jian().a(), 0);
}
