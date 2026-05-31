//! Small drawing/hit-test helpers for the Images settings tab.

use crate::theme::Theme;
use crate::widgets::agent_settings_caret::{
    caret_x_for_text, paint_caret, settings_caret_for_focus,
};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, ImageGenField, ImageGenProfile, ImageGenProvider, ImageSearchField,
    SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;

const ROW_H: f32 = 36.0;
const LABEL_W: f32 = 110.0;
const PROVIDER_OPTION_H: f32 = 24.0;

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

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_profile_field(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    profile: &ImageGenProfile,
    index: usize,
    field: ImageGenField,
    input: Rect,
    label_x: f32,
    now_ms: u64,
) {
    let focus = SettingsFocus::ImageGenProfile { index, field };
    let focused = settings.focus == Some(focus);
    let value = if focused {
        ui.settings_input_draft.as_str()
    } else {
        match field {
            ImageGenField::Name => profile.name.as_str(),
            ImageGenField::ApiKey if !profile.api_key.is_empty() => "********",
            ImageGenField::ApiKey => "",
            ImageGenField::Model => profile.model.as_str(),
            ImageGenField::BaseUrl => profile.base_url.as_deref().unwrap_or(""),
        }
    };
    let label = match field {
        ImageGenField::Name => "Name",
        ImageGenField::ApiKey => "API Key",
        ImageGenField::Model => "Model",
        ImageGenField::BaseUrl => "Base URL",
    };
    let label_lay = TextLayout::single_run(
        label,
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_lay, Point2D::new(label_x, input.origin.y + 16.0));
    cx.backend.fill_round_rect(
        input,
        6.0,
        if focused {
            theme.background
        } else {
            theme.card
        },
    );
    cx.backend.stroke_round_rect(
        input,
        6.0,
        if focused { theme.primary } else { theme.border },
        1.0,
    );
    let clipped = ellipsize(cx, value, input.size.x - 12.0, 11.0);
    let text_x = input.origin.x + 6.0;
    let value_lay = TextLayout::single_run(
        &clipped,
        "system-ui",
        11.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&value_lay, Point2D::new(text_x, input.origin.y + 16.0));
    if focused {
        let caret = settings_caret_for_focus(ui, focus);
        let caret_x = caret_x_for_text(
            cx,
            value,
            caret,
            text_x,
            input.origin.x + input.size.x - 8.0,
            11.0,
        );
        let caret_y = input.origin.y + 4.5;
        let anchor = ui.settings_input_caret_anchor_ms;
        paint_caret(cx, theme, now_ms, anchor, caret_x, caret_y);
    }
}

pub(super) fn paint_profile_test_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    profile: &ImageGenProfile,
    btn: Rect,
) {
    let enabled = !profile.api_key.trim().is_empty();
    cx.backend.fill_round_rect(btn, 6.0, theme.muted);
    cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
    let label = crate::widgets::agent_settings_i18n::t(ui, "settings.images.test");
    let label_w = cx.backend.measure_text(label, 11.0);
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        11.0,
        to_jian(if enabled {
            theme.foreground
        } else {
            theme.muted_foreground
        }),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            btn.origin.x + (btn.size.x - label_w) / 2.0,
            btn.origin.y + btn.size.y / 2.0 + 4.0,
        ),
    );
}

pub(super) fn paint_provider_field(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    profile: &ImageGenProfile,
    input: Rect,
    label_x: f32,
) {
    let label_lay = TextLayout::single_run(
        "Provider",
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label_lay, Point2D::new(label_x, input.origin.y + 16.0));
    cx.backend.fill_round_rect(input, 6.0, theme.card);
    cx.backend.stroke_round_rect(input, 6.0, theme.border, 1.0);
    let value = ellipsize(cx, profile.provider.label(), input.size.x - 28.0, 11.0);
    let value_lay = TextLayout::single_run(
        &value,
        "system-ui",
        11.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &value_lay,
        Point2D::new(input.origin.x + 6.0, input.origin.y + 16.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(input.origin.x + input.size.x - 18.0, input.origin.y + 5.0),
        14.0,
        theme.muted_foreground,
        1.5,
    );
}

pub(super) fn paint_provider_menu(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    input: Rect,
    selected: ImageGenProvider,
) {
    let menu = Rect {
        origin: Point2D::new(input.origin.x, input.origin.y + input.size.y),
        size: Point2D::new(
            input.size.x,
            PROVIDER_OPTION_H * ImageGenProvider::ALL.len() as f32,
        ),
    };
    cx.backend.fill_round_rect(menu, 6.0, theme.card);
    cx.backend.stroke_round_rect(menu, 6.0, theme.border, 1.0);
    for (index, provider) in ImageGenProvider::ALL.iter().enumerate() {
        let option_y = menu.origin.y + index as f32 * PROVIDER_OPTION_H;
        if *provider == selected {
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(menu.origin.x + 2.0, option_y + 2.0),
                    size: Point2D::new(menu.size.x - 4.0, PROVIDER_OPTION_H - 4.0),
                },
                4.0,
                theme.muted,
            );
        }
        let label = TextLayout::single_run(
            provider.label(),
            "system-ui",
            11.0,
            to_jian(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(
                menu.origin.x + 8.0,
                option_y + PROVIDER_OPTION_H / 2.0 + 4.0,
            ),
        );
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
