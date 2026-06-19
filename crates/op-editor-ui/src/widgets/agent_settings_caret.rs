use crate::theme::Theme;
use crate::widgets::text_input_backend::BaselineAdjustingBackend;
use crate::widgets::PaintCx;
use crate::Rect;
use jian_widgets::components::text_input::TextInputView;
use jian_widgets::Tokens;
use op_editor_core::agent_settings::{AgentSettings, SettingsFocus};
use op_editor_core::editor_ui_state::EditorUiState;

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
