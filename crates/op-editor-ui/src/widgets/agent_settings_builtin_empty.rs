//! Compact touch empty state for the built-in AI provider list.

use crate::theme::Theme;
use crate::widgets::agent_settings_builtin_layout::{touch_empty_card_rect, touch_empty_cta_rect};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::settings_form::{draw_text, ellipsize};
use crate::widgets::{text_metrics, PaintCx};
use crate::Rect;
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};

pub(super) fn paint(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    content: Rect,
    y: f32,
) -> f32 {
    let card = touch_empty_card_rect(content, y);
    cx.backend.fill_round_rect(card, 14.0, theme.muted);
    cx.backend.stroke_round_rect(card, 14.0, theme.border, 1.0);
    let cta = touch_empty_cta_rect(content, y);
    let pressed = ui.button_pressed(ButtonPressTarget::AgentSettings(
        AgentSettingsButton::AddProvider,
    ));
    let fill = if pressed {
        theme.primary.with_alpha(theme.primary.a * 0.8)
    } else {
        theme.primary
    };
    cx.backend.fill_round_rect(cta, 12.0, fill);

    let text_x = card.origin.x + 16.0;
    let available = (cta.origin.x - text_x - 12.0).max(0.0);
    let empty = ellipsize(
        cx,
        t_settings(ui, "settings.agents.builtinEmpty"),
        available,
        13.0,
    );
    draw_text(
        cx,
        &empty,
        13.0,
        theme.muted_foreground,
        text_x,
        card.origin.y + card.size.y / 2.0 + 4.0,
    );
    let action = ellipsize(
        cx,
        t_settings(ui, "settings.agents.addProvider"),
        cta.size.x - 24.0,
        13.0,
    );
    let action_w = text_metrics::measure_chrome(cx.backend, &action, 13.0);
    draw_text(
        cx,
        &action,
        13.0,
        theme.primary_foreground,
        cta.origin.x + (cta.size.x - action_w) / 2.0,
        cta.origin.y + cta.size.y / 2.0 + 4.0,
    );
    card.origin.y + card.size.y
}
