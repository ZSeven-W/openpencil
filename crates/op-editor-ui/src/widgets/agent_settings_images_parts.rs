//! Small drawing/hit-test helpers for the Images settings tab.

use crate::theme::Theme;
use crate::widgets::agent_settings_caret::{
    caret_x_for_text, paint_caret, settings_caret_for_focus,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{AgentSettings, ImageSearchField, SettingsFocus};
use op_editor_core::editor_ui_state::EditorUiState;

const ROW_H: f32 = 36.0;
const LABEL_W: f32 = 110.0;

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_search_input_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    field_kind: ImageSearchField,
    label: &str,
    placeholder: &str,
    x: f32,
    y: f32,
    w: f32,
    now_ms: u64,
) {
    let label_lay = TextLayout::single_run(
        label,
        "system-ui",
        13.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_lay, Point2D::new(x, y + ROW_H / 2.0 + 5.0));
    let field = Rect {
        origin: Point2D::new(x + LABEL_W, y),
        size: Point2D::new(w - LABEL_W, ROW_H),
    };
    cx.backend.fill_round_rect(field, 6.0, theme.background);
    let focus = SettingsFocus::ImageSearch(field_kind);
    let focused = settings.focus == Some(focus);
    cx.backend.stroke_round_rect(
        field,
        6.0,
        if focused { theme.primary } else { theme.border },
        1.0,
    );
    let stored = match field_kind {
        ImageSearchField::ClientId => settings.openverse_client_id.as_str(),
        ImageSearchField::ClientSecret => settings.openverse_client_secret.as_str(),
    };
    let text = if focused {
        ui.settings_input_draft.as_str()
    } else if matches!(field_kind, ImageSearchField::ClientSecret) && !stored.is_empty() {
        "********"
    } else {
        stored
    };
    let showing_placeholder = text.is_empty();
    let value = if showing_placeholder {
        placeholder
    } else {
        text
    };
    let value = ellipsize(cx, value, field.size.x - 24.0, 13.0);
    let text_x = field.origin.x + 12.0;
    let lay = TextLayout::single_run(
        &value,
        "system-ui",
        13.0,
        to_jian(if showing_placeholder {
            theme.muted_foreground
        } else {
            theme.foreground
        }),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &lay,
        Point2D::new(text_x, field.origin.y + ROW_H / 2.0 + 5.0),
    );
    if focused {
        let caret = settings_caret_for_focus(ui, focus);
        let caret_x = caret_x_for_text(
            cx,
            text,
            caret,
            text_x,
            field.origin.x + field.size.x - 12.0,
            13.0,
        );
        let caret_y = field.origin.y + (ROW_H - 15.0) / 2.0;
        let anchor = ui.settings_input_caret_anchor_ms;
        paint_caret(cx, theme, now_ms, anchor, caret_x, caret_y);
    }
}

pub(super) fn ellipsize(cx: &mut PaintCx<'_>, value: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text(value, size) <= max_w {
        return value.to_string();
    }
    let mut out = value.to_string();
    while !out.is_empty() && cx.backend.measure_text(&format!("{out}..."), size) > max_w {
        out.pop();
    }
    format!("{out}...")
}

pub(super) fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.y >= r.origin.y
        && p.x <= r.origin.x + r.size.x
        && p.y <= r.origin.y + r.size.y
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
