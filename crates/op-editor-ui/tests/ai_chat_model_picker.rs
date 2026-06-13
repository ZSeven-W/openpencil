use jian_core::text_input::TextInputState;
use op_editor_core::chat::{AgentProvider, ModelEntry};
use op_editor_core::Locale;
use op_editor_ui::theme::Theme;
use op_editor_ui::widgets::ai_chat_model_picker::{paint_model_picker, picker_view_height};
use op_editor_ui::widgets::PaintCx;
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
struct CaptureBackend {
    fills: Vec<(Rect, Color)>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fills.push((rect, color));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

fn caret_fills(fills: &[(Rect, Color)], theme: Theme) -> Vec<Rect> {
    fills
        .iter()
        .filter_map(|(rect, color)| {
            (color_eq(*color, theme.foreground)
                && (rect.size.x - 1.5).abs() < 0.01
                && (rect.size.y - 15.0).abs() < 0.01)
                .then_some(*rect)
        })
        .collect()
}

#[test]
fn model_picker_search_paints_visible_caret_at_blink_on_phase() {
    let theme = Theme::dark();
    let models = vec![ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "gpt-5")];
    let rect = Rect::xywh(10.0, 20.0, 240.0, picker_view_height(&models, ""));
    let input = TextInputState::default();
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_model_picker(
        &mut cx,
        &theme,
        rect,
        &models,
        0,
        0.0,
        None,
        &input,
        100,
        Locale::EnUs,
    );

    assert_eq!(caret_fills(&backend.fills, theme).len(), 1);
}

#[test]
fn model_picker_search_hides_caret_at_blink_off_phase() {
    let theme = Theme::dark();
    let models = vec![ModelEntry::new(AgentProvider::CodexCli, "gpt-5", "gpt-5")];
    let rect = Rect::xywh(10.0, 20.0, 240.0, picker_view_height(&models, ""));
    let input = TextInputState::default();
    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_model_picker(
        &mut cx,
        &theme,
        rect,
        &models,
        0,
        0.0,
        None,
        &input,
        500,
        Locale::EnUs,
    );

    assert!(caret_fills(&backend.fills, theme).is_empty());
}
