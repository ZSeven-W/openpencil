//! Chat input text hit-testing and selection paint helpers.

use crate::theme::Theme;
use crate::widgets::text_input_backend::BaselineAdjustingBackend;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use jian_core::text_input::TextInputState;
use jian_widgets::components::text_area::TextArea;
use jian_widgets::{Density, Tokens};
use op_editor_core::chat::ChatState;

pub(crate) const INPUT_LINE_H: f32 = 18.0;
pub(crate) const INPUT_TEXT_X_PAD: f32 = 8.0;
pub(crate) const INPUT_BASELINE_ASCENT: f32 = 14.0;
pub(crate) const INPUT_FONT: f32 = 14.0;
pub(crate) const INPUT_MAX_LINES: usize = 3;
const TEXT_AREA_PAD_Y: f32 = 4.0;

pub(crate) fn visible_input_line_count(text: &str, input_rect_width: f32) -> usize {
    if text.is_empty() {
        return 1;
    }
    let mut backend = MeasureOnlyBackend;
    let text_width = (input_rect_width - INPUT_TEXT_X_PAD * 2.0).max(0.0);
    TextArea::layout_lines(&mut backend, text, INPUT_FONT, text_width)
        .len()
        .clamp(1, INPUT_MAX_LINES)
}

pub(crate) fn input_first_baseline(visible_rows: usize, input_area_h: f32) -> f32 {
    let rows = visible_rows.max(1) as f32;
    let block_h = rows * INPUT_LINE_H;
    ((input_area_h - block_h) / 2.0 + INPUT_BASELINE_ASCENT)
        .clamp(INPUT_BASELINE_ASCENT, input_area_h - 4.0)
}

fn input_text_area_rect(input_rect: Rect, text_for_layout: &str, input_area_h: f32) -> Rect {
    let visible_rows = visible_input_line_count(text_for_layout, input_rect.size.x);
    let first_baseline = input_first_baseline(visible_rows, input_area_h);
    Rect {
        origin: Point2D::new(
            input_rect.origin.x,
            input_rect.origin.y + first_baseline - INPUT_BASELINE_ASCENT - TEXT_AREA_PAD_Y,
        ),
        size: input_rect.size,
    }
}

pub(crate) fn input_text_offset_at(
    input: &TextInputState,
    input_rect: Rect,
    point: Point2D,
) -> Option<usize> {
    if !(input_rect).contains(point) {
        return None;
    }
    let text = input.text();
    if text.is_empty() {
        return Some(0);
    }
    let text_rect = input_text_area_rect(input_rect, text, input_rect.size.y);
    let view = TextArea {
        state: input,
        placeholder: "",
        focused: true,
        font_size: INPUT_FONT,
        now_ms: 0,
        pad_x: INPUT_TEXT_X_PAD,
        max_visible_lines: INPUT_MAX_LINES,
    };
    let mut backend = MeasureOnlyBackend;
    Some(view.byte_offset_at(
        &mut backend,
        text_rect,
        point,
        &tokens_from_theme(&Theme::dark()),
    ))
}

pub(crate) fn paint_input_text_area(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    state: &ChatState,
    input_rect: Rect,
    input_area_h: f32,
    now_ms: u64,
    placeholder: &str,
) {
    let text_for_layout = if state.input.text().is_empty() {
        placeholder
    } else {
        state.input.text()
    };
    let text_rect = input_text_area_rect(input_rect, text_for_layout, input_area_h);
    let view = TextArea {
        state: &state.input,
        placeholder,
        focused: state.focused,
        font_size: INPUT_FONT,
        now_ms,
        pad_x: INPUT_TEXT_X_PAD,
        max_visible_lines: INPUT_MAX_LINES,
    };
    let mut backend = BaselineAdjustingBackend {
        inner: cx.backend,
        baseline_delta_y: INPUT_BASELINE_ASCENT,
    };
    backend.save();
    backend.clip_rect(input_rect);
    view.paint(&mut backend, text_rect, &tokens_from_theme(theme));
    backend.restore();
}

fn tokens_from_theme(theme: &Theme) -> Tokens {
    Tokens {
        background: theme.background,
        foreground: theme.foreground,
        card: theme.card,
        card_foreground: theme.card_foreground,
        popover: theme.popover,
        popover_foreground: theme.popover_foreground,
        primary: theme.primary,
        primary_foreground: theme.primary_foreground,
        muted: theme.muted,
        muted_foreground: theme.muted_foreground,
        border: theme.border,
        accent: theme.accent,
        accent_foreground: theme.accent_foreground,
        destructive: theme.destructive,
        button_hover: theme.button_hover,
        row_selected: theme.row_selected,
        row_selected_primary: theme.row_selected_primary,
        density: Density::Desktop,
    }
}

struct MeasureOnlyBackend;

impl RenderBackend for MeasureOnlyBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct CaptureBackend {
        text_draws: Vec<(String, Point2D)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
            if let Some(run) = layout.runs().first() {
                self.text_draws.push((run.content.clone(), origin));
            }
        }
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

    #[test]
    fn focused_input_text_preserves_original_first_baseline() {
        let input_rect = Rect::xywh(20.0, 100.0, 240.0, 40.0);
        let mut state = ChatState::default();
        state.set_input_text("hello");
        state.focused = true;

        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_input_text_area(
            &mut cx,
            &Theme::light(),
            &state,
            input_rect,
            input_rect.size.y,
            0,
            "",
        );

        let expected_baseline_y = input_rect.origin.y
            + ((input_rect.size.y - INPUT_LINE_H) / 2.0 + INPUT_FONT)
                .clamp(INPUT_FONT, input_rect.size.y - 4.0);
        let (_, first_origin) = backend.text_draws.first().expect("input text should paint");

        assert!(
            (first_origin.y - expected_baseline_y).abs() < 0.01,
            "expected first text baseline at {expected_baseline_y}, got {}",
            first_origin.y
        );
    }

    #[test]
    fn input_hit_testing_matches_painted_wrapping_for_chinese_text() {
        let input = "设计一个现代的移动端登录页面，包含邮箱输入框";
        let input_rect = Rect::xywh(20.0, 100.0, 202.0, 72.0);
        let mut state = ChatState::default();
        state.set_input_text(input);
        let theme = Theme::light();

        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_input_text_area(
            &mut cx,
            &theme,
            &state,
            input_rect,
            input_rect.size.y,
            0,
            "",
        );

        assert!(
            backend.text_draws.len() >= 2,
            "test input should paint across multiple lines: {:?}",
            backend.text_draws
        );
        let second_painted_origin = backend.text_draws[1].1;
        let click_near_start_of_second_line = Point2D::new(
            second_painted_origin.x + 1.0,
            second_painted_origin.y - INPUT_BASELINE_ASCENT + 1.0,
        );
        let lines = TextArea::layout_lines(
            &mut backend,
            input,
            INPUT_FONT,
            input_rect.size.x - INPUT_TEXT_X_PAD * 2.0,
        );

        assert_eq!(
            input_text_offset_at(&state.input, input_rect, click_near_start_of_second_line),
            Some(lines[1].start)
        );
    }
}
