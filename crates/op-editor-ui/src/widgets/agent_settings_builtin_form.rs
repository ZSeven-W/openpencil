//! Expanded built-in provider form with separate desktop and touch density.

use crate::theme::Theme;
use crate::widgets::agent_settings_builtin_layout::field_input_rect_for_ui;
use crate::widgets::agent_settings_builtin_parts;
use crate::widgets::agent_settings_caret::settings_input_text;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::brand_icons::{paint_brand_logo, BrandLogo};
use crate::widgets::button::tokens_from_theme;
use crate::widgets::settings_form::{self, draw_text, ellipsize};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use jian_widgets::components::card::Card;
use op_editor_core::agent_settings::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentField, BuiltinAgentKind, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_builtin_agent_form(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    card: Rect,
    now_ms: u64,
) {
    let touch = ui.touch_chrome();
    Card {
        fill: Some(theme.muted),
        border: Some(theme.border),
        radius: if touch { 14.0 } else { 10.0 },
    }
    .paint(cx.backend, card, &tokens_from_theme(theme));
    paint_identity(cx, theme, settings, ui, agent, index, card, touch);
    agent_settings_builtin_parts::paint_kind_toggle(
        cx,
        theme,
        agent,
        card,
        t_settings(ui, "builtin.apiFormat"),
        touch,
    );
    agent_settings_builtin_parts::paint_provider_select(
        cx,
        theme,
        agent,
        card,
        t_settings(ui, "builtin.provider"),
        touch,
    );
    for (row, field) in [
        BuiltinAgentField::DisplayName,
        BuiltinAgentField::ApiKey,
        BuiltinAgentField::Model,
        BuiltinAgentField::BaseUrl,
    ]
    .into_iter()
    .enumerate()
    {
        paint_field(
            cx, theme, settings, ui, agent, index, field, row, card, now_ms,
        );
    }
    agent_settings_builtin_parts::paint_preset_menu(cx, theme, settings, agent, index, card, touch);
}

#[allow(clippy::too_many_arguments)]
fn paint_identity(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    card: Rect,
    touch: bool,
) {
    let avatar_size = if touch { 36.0 } else { 28.0 };
    let avatar = Rect {
        origin: Point2D::new(
            card.origin.x + if touch { 16.0 } else { 12.0 },
            card.origin.y + if touch { 12.0 } else { 10.0 },
        ),
        size: Point2D::new(avatar_size, avatar_size),
    };
    cx.backend
        .fill_round_rect(avatar, if touch { 10.0 } else { 6.0 }, theme.background);
    let logo = match agent.kind {
        BuiltinAgentKind::Anthropic => BrandLogo::Claude,
        BuiltinAgentKind::OpenAiCompat => BrandLogo::OpenAI,
    };
    let logo_size = if touch { 20.0 } else { 16.0 };
    paint_brand_logo(
        cx.backend,
        logo,
        Point2D::new(
            avatar.origin.x + (avatar.size.x - logo_size) / 2.0,
            avatar.origin.y + (avatar.size.y - logo_size) / 2.0,
        ),
        logo_size,
        theme.foreground,
    );
    let api_key = effective_field_text(settings, ui, agent, index, BuiltinAgentField::ApiKey);
    let model = effective_field_text(settings, ui, agent, index, BuiltinAgentField::Model);
    let ready = agent.enabled && !api_key.trim().is_empty() && !model.trim().is_empty();
    let status_size = if touch { 14.0 } else { 11.0 };
    let status_band = Rect {
        origin: Point2D::new(card.origin.x, card.origin.y + if touch { 8.0 } else { 6.0 }),
        size: Point2D::new(card.size.x, 44.0),
    };
    let status = if api_key.trim().is_empty() {
        t_settings(ui, "builtin.errorApiKeyEmpty")
    } else if model.trim().is_empty() {
        t_settings(ui, "builtin.model")
    } else if ready {
        t_settings(ui, "builtin.ready")
    } else {
        ""
    };
    let status = ellipsize(
        cx,
        status,
        (card.size.x - if touch { 80.0 } else { 64.0 }).max(0.0),
        status_size,
    );
    if !status.is_empty() {
        draw_text(
            cx,
            &status,
            status_size,
            if ready {
                theme.status_success
            } else {
                theme.muted_foreground
            },
            card.origin.x + if touch { 64.0 } else { 48.0 },
            if touch {
                jian_widgets::centered_text_baseline_y(status_band, status_size)
            } else {
                card.origin.y + 28.0
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_field(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    field: BuiltinAgentField,
    row: usize,
    card: Rect,
    now_ms: u64,
) {
    let focus = match index {
        Some(index) => SettingsFocus::BuiltinAgent { index, field },
        None => SettingsFocus::BuiltinAgentDraft(field),
    };
    let focused = settings.focus == Some(focus);
    let fallback = match field {
        BuiltinAgentField::DisplayName => agent.display_name.as_str(),
        BuiltinAgentField::ApiKey if !agent.api_key.is_empty() => "********",
        BuiltinAgentField::ApiKey => "",
        BuiltinAgentField::Model => agent.model.as_str(),
        BuiltinAgentField::BaseUrl => agent.base_url.as_str(),
    };
    let value = settings_input_text(settings, ui, focus, fallback);
    let label = match field {
        BuiltinAgentField::DisplayName => t_settings(ui, "builtin.displayName"),
        BuiltinAgentField::ApiKey => t_settings(ui, "builtin.apiKey"),
        BuiltinAgentField::Model => t_settings(ui, "builtin.model"),
        BuiltinAgentField::BaseUrl => t_settings(ui, "builtin.baseUrl"),
    };
    let touch = ui.touch_chrome();
    let input = field_input_rect_for_ui(settings, card, index, row, touch);
    let editable = field != BuiltinAgentField::BaseUrl || agent.base_url_editable();
    let value_size = if touch { 15.0 } else { 11.0 };
    let text_x = input.origin.x + if touch { 12.0 } else { 6.0 };
    let painted_focused = settings_form::paint_field_frame_for_ui(
        cx,
        theme,
        ui,
        label,
        card.origin.x + if touch { 16.0 } else { 12.0 },
        input.origin.y + 16.0,
        input,
        focused,
        "",
        now_ms,
        touch,
    );
    if painted_focused {
        return;
    }
    let clipped = ellipsize(
        cx,
        value,
        input.size.x - if touch { 24.0 } else { 12.0 },
        value_size,
    );
    draw_text(
        cx,
        &clipped,
        value_size,
        if editable {
            theme.foreground
        } else {
            theme.muted_foreground
        },
        text_x,
        if touch {
            jian_widgets::centered_text_baseline_y(input, value_size)
        } else {
            input.origin.y + 16.0
        },
    );
}

fn effective_field_text<'a>(
    settings: &AgentSettings,
    ui: &'a EditorUiState,
    agent: &'a BuiltinAgentConfig,
    index: Option<usize>,
    field: BuiltinAgentField,
) -> &'a str {
    let focus = match index {
        Some(index) => SettingsFocus::BuiltinAgent { index, field },
        None => SettingsFocus::BuiltinAgentDraft(field),
    };
    let fallback = match field {
        BuiltinAgentField::DisplayName => agent.display_name.as_str(),
        BuiltinAgentField::ApiKey => agent.api_key.as_str(),
        BuiltinAgentField::Model => agent.model.as_str(),
        BuiltinAgentField::BaseUrl => agent.base_url.as_str(),
    };
    settings_input_text(settings, ui, focus, fallback)
}
