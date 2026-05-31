use super::*;

#[test]
fn transcript_hit_resolves_design_json_copy_button_like_ts() {
    let code = r#"[{"id":"frame-1","type":"Frame"}]"#;
    let message = ChatMessage::assistant(format!(
        r#"```json
{code}
```"#
    ));
    let messages = std::slice::from_ref(&message);
    let block =
        &build_transcript(messages, body(), op_editor_core::Locale::EnUs)[0].design_blocks[0];
    let x = block.header.origin.x + block.header.size.x - 38.0;
    let y = block.header.origin.y + block.header.size.y / 2.0;

    assert_eq!(
        transcript_hit(messages, body(), x, y, op_editor_core::Locale::EnUs),
        Some(TranscriptHit::CopyDesignBlock(code.to_string()))
    );
}

fn body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
}

#[test]
fn paint_design_json_copy_icon_is_hidden_until_card_hover_like_ts() {
    let message = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    let messages = [message];
    let mut backend = CopyPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &messages,
        0,
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(backend.copy_icon_strokes, 0);
}

#[test]
fn paint_design_json_copy_icon_is_visible_for_hovered_card_like_ts() {
    let message = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    let messages = [message];
    let mut backend = CopyPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript_with_design_hover(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &messages,
        0,
        op_editor_core::Locale::EnUs,
        Some((0, 0)),
    );

    assert!(backend.copy_icon_strokes > 0);
}

#[test]
fn paint_design_json_block_uses_ts_wand_icon() {
    const WAND_SPARKLES_MAIN_PATH: &str =
        "m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72";
    let message = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    let messages = [message];
    let mut backend = CopyPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &messages,
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.svg_strokes.iter().any(|(path, point, size)| {
            *path == WAND_SPARKLES_MAIN_PATH
                && (*size - 10.0).abs() < 1e-4
                && (point.x - 15.0).abs() < 1e-4
                && (point.y - 11.0).abs() < 1e-4
        }),
        "TS DesignJsonBlock uses Wand2 size=10 centered inside a 16px icon circle"
    );
}

#[derive(Default)]
struct CopyPaintBackend {
    copy_icon_strokes: usize,
    svg_strokes: Vec<(String, Point2D, f32)>,
}

impl crate::RenderBackend for CopyPaintBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: crate::Color) {}
    fn stroke_rect(&mut self, _: Rect, _: crate::Color, _: f32) {}
    fn draw_text(&mut self, _: &crate::TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: crate::Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: crate::Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: crate::Color, _: f32) {}
    fn stroke_svg_path(&mut self, d: &str, point: Point2D, size: f32, _: crate::Color, _: f32) {
        self.svg_strokes.push((d.to_owned(), point, size));
        if (290.0..=305.0).contains(&point.x) {
            self.copy_icon_strokes += 1;
        }
    }
    fn fill_oval(&mut self, _: Rect, _: crate::Color) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}
