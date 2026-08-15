use crate::theme::Theme;
use crate::widgets::text_input_backend::BaselineAdjustingBackend;
use crate::widgets::PaintCx;
use crate::Rect;
use jian_widgets::components::text_area::TextArea;
use jian_widgets::components::text_input::TextInputView;
use jian_widgets::Tokens;
use op_editor_core::agent_settings::{AgentSettings, SettingsFocus};
use op_editor_core::editor_ui_state::EditorUiState;

pub(super) fn model_text_area_rect(input: Rect, touch: bool) -> Rect {
    let trailing = if touch { 44.0 } else { 24.0 };
    Rect {
        origin: input.origin,
        size: crate::Point2D::new((input.size.x - trailing).max(0.0), input.size.y),
    }
}

pub(super) fn model_text_font_size(touch: bool) -> f32 {
    if touch {
        15.0
    } else {
        11.0
    }
}

pub(super) fn model_text_pad_x(touch: bool) -> f32 {
    if touch {
        12.0
    } else {
        6.0
    }
}

pub(super) fn model_text_max_visible_lines(touch: bool) -> usize {
    if touch {
        4
    } else {
        3
    }
}

/// Map a click/tap in the focused provider Model editor back to the source
/// byte offset using the same multiline layout and visible-line window as
/// paint. The chevron trigger is deliberately excluded from the text area.
pub(super) fn model_text_byte_offset_at(
    ui: &EditorUiState,
    input: Rect,
    point: crate::Point2D,
    touch: bool,
) -> Option<usize> {
    crate::widgets::text_input::multiline_byte_offset_at(
        &ui.settings_input,
        model_text_area_rect(input, touch),
        point,
        model_text_font_size(touch),
        model_text_pad_x(touch),
        model_text_max_visible_lines(touch),
    )
}

pub(super) fn settings_input_text<'a>(
    settings: &AgentSettings,
    ui: &'a EditorUiState,
    focus: SettingsFocus,
    fallback: &'a str,
) -> &'a str {
    if settings.focus == Some(focus) {
        ui.settings_input.text()
    } else {
        fallback
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_settings_input_view(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    rect: Rect,
    font_size: f32,
    pad_x: f32,
    text_baseline_y: f32,
    now_ms: u64,
    placeholder: &str,
) {
    let view = TextInputView {
        state: &ui.settings_input,
        placeholder,
        focused: true,
        font_size,
        now_ms,
        pad_x,
        baseline_delta_y: 0.0,
        mask: None,
    };
    let text_top_y = rect.origin.y + (rect.size.y - font_size) / 2.0;
    let mut backend = BaselineAdjustingBackend {
        inner: cx.backend,
        baseline_delta_y: text_baseline_y - text_top_y,
    };
    view.paint(&mut backend, rect, &tokens_from_theme(theme));
}

/// Multiline settings editor used by the built-in provider model list.
/// The shared `TextInputState` still owns selection/IME/caret state; this
/// view only changes wrapping and vertical presentation.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_settings_text_area(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    rect: Rect,
    font_size: f32,
    pad_x: f32,
    now_ms: u64,
    placeholder: &str,
    max_visible_lines: usize,
) {
    let area = TextArea {
        state: &ui.settings_input,
        placeholder,
        focused: true,
        font_size,
        now_ms,
        pad_x,
        max_visible_lines,
    };
    let mut backend = BaselineAdjustingBackend {
        inner: cx.backend,
        baseline_delta_y: font_size,
    };
    area.paint(&mut backend, rect, &tokens_from_theme(theme));
}

fn tokens_from_theme(theme: &Theme) -> Tokens {
    crate::widgets::button::tokens_from_theme(theme)
}
