use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::SettingsFocus;
use op_editor_core::editor_ui_state::EditorUiState;

pub(super) fn settings_caret_for_focus(ui: &EditorUiState, focus: SettingsFocus) -> Option<usize> {
    if ui.settings_input_caret_focus == Some(focus) {
        ui.settings_input_caret
    } else {
        None
    }
}

pub(super) fn caret_x_for_text(
    cx: &mut PaintCx<'_>,
    value: &str,
    caret: Option<usize>,
    text_x: f32,
    max_x: f32,
    size: f32,
) -> f32 {
    let pos = clamp_boundary(value, caret.unwrap_or(value.len()));
    (text_x + cx.backend.measure_text(&value[..pos], size) + 1.0).min(max_x)
}

pub(super) fn paint_caret(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    now_ms: u64,
    anchor_ms: u64,
    x: f32,
    y: f32,
) {
    if !jian_core::anim::blink_visible(now_ms, anchor_ms, 500) {
        return;
    }
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(1.5, 15.0),
        },
        theme.foreground,
    );
}

pub(super) fn paint_settings_selection(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    value: &str,
    text_x: f32,
    baseline_y: f32,
    size: f32,
    max_x: f32,
) {
    crate::widgets::text_selection::paint_single_line_selection(
        cx, theme, value, text_x, baseline_y, size, max_x,
    );
}

fn clamp_boundary(value: &str, pos: usize) -> usize {
    let mut pos = pos.min(value.len());
    while pos > 0 && !value.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}
