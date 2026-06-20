use crate::theme::Theme;
use crate::widgets::text_input_backend::BaselineAdjustingBackend;
use crate::widgets::PaintCx;
use crate::Rect;
use jian_core::text_input::TextInputState;
use jian_widgets::components::text_input::TextInputView;
use jian_widgets::Tokens;

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_text_input_view_value(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    input: &TextInputState,
    rect: Rect,
    font_size: f32,
    pad_x: f32,
    text_baseline_y: f32,
    now_ms: u64,
) {
    // The "_value" variant always reads as focused with no placeholder — it
    // backs property-panel fields that only render while focused.
    paint_text_input_view(
        cx,
        theme,
        input,
        rect,
        font_size,
        pad_x,
        text_baseline_y,
        now_ms,
        "",
        true,
    );
}

/// Render a `TextInputView` for `input` inside `rect`, baselining the run at
/// `text_baseline_y`. Unlike `_value`, this exposes `placeholder` (drawn muted
/// when the buffer is empty) and `focused` (caret blinks only when true) — so
/// it serves search / filter boxes that must look right whether or not they
/// hold focus, on one unified caret implementation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_text_input_view(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    input: &TextInputState,
    rect: Rect,
    font_size: f32,
    pad_x: f32,
    text_baseline_y: f32,
    now_ms: u64,
    placeholder: &str,
    focused: bool,
) {
    let resolved_pad_x = if pad_x <= 0.0 { f32::EPSILON } else { pad_x };
    let view = TextInputView {
        state: input,
        placeholder,
        focused,
        font_size,
        now_ms,
        pad_x: resolved_pad_x,
        baseline_delta_y: 0.0,
    };
    let text_top_y = rect.origin.y + (rect.size.y - font_size) / 2.0;
    let mut backend = BaselineAdjustingBackend {
        inner: cx.backend,
        baseline_delta_y: text_baseline_y - text_top_y,
    };
    view.paint(&mut backend, rect, &tokens_from_theme(theme));
}

fn tokens_from_theme(theme: &Theme) -> Tokens {
    crate::widgets::button::tokens_from_theme(theme)
}
