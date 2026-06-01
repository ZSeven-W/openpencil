use super::ai_chat_transcript::paint_transcript;
use super::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::chat::ChatMessage;

#[derive(Default)]
struct PaintProbe {
    texts: Vec<String>,
}

impl crate::RenderBackend for PaintProbe {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: crate::Color) {}
    fn stroke_rect(&mut self, _: Rect, _: crate::Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, _: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: crate::Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: crate::Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: crate::Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: crate::Color, _: f32) {}
    fn fill_oval(&mut self, _: Rect, _: crate::Color) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
}

#[test]
fn paint_expanded_design_json_block_shows_apply_action_like_ts() {
    let mut message = ChatMessage::assistant(
        r#"```json
[{"type":"frame","id":"apply-root","name":"Apply Root","x":0,"y":0,"width":100,"height":80,"children":[]}]
```"#,
    );
    message.design_block_expanded_overrides = vec![Some(true)];
    let mut backend = PaintProbe::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[message],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.texts.iter().any(|text| text == "Apply to Canvas"),
        "TS expanded design JSON cards expose an Apply to Canvas action"
    );
}
