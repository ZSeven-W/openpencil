use super::ai_chat_transcript::{paint_transcript, transcript_content_height, LINE_H};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::{ChatMessage, ChatToolCall};

fn body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
}

fn color_close(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

#[derive(Default)]
struct IdentityPaintBackend {
    texts: Vec<String>,
    oval_colors: Vec<Color>,
}

impl RenderBackend for IdentityPaintBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn fill_oval(&mut self, _: Rect, color: Color) {
        self.oval_colors.push(color);
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn agent_identity_adds_height_while_absence_preserves_prior_geometry() {
    let without_identity = ChatMessage::assistant("answer");
    let prior_height = transcript_content_height(
        std::slice::from_ref(&without_identity),
        body(),
        op_editor_core::Locale::EnUs,
    );
    assert_eq!(prior_height, LINE_H, "identity None keeps the prior height");

    let mut with_identity = without_identity;
    with_identity.agent_name = Some("Kiki".into());
    let identified_height =
        transcript_content_height(&[with_identity], body(), op_editor_core::Locale::EnUs);

    assert!(identified_height > prior_height);
}

#[test]
fn paint_agent_identity_uses_name_and_parsed_identity_color() {
    let mut message = ChatMessage::assistant("answer");
    message.agent_name = Some("Kiki".into());
    message.agent_color = Some("#FF6B6B".into());
    let expected_color = crate::util::parse_hex_color("#FF6B6B").unwrap();
    let mut backend = IdentityPaintBackend::default();
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

    assert!(backend.texts.iter().any(|text| text == "Kiki"));
    assert!(backend
        .oval_colors
        .iter()
        .any(|color| color_close(*color, expected_color)));
}

#[test]
fn paint_tool_card_uses_narrative_verb_instead_of_raw_tool_name() {
    let mut message = ChatMessage::assistant("");
    message.tools_collapsed = false;
    message.tool_calls.push(ChatToolCall {
        name: "batch_design".into(),
        args: r#"{"status":"done"}"#.into(),
    });
    let mut backend = IdentityPaintBackend::default();
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

    assert!(backend.texts.iter().any(|text| text == "Designed"));
    assert!(!backend.texts.iter().any(|text| text == "batch_design"));
}
