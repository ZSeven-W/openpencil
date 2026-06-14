//! ACP Agent section for the Agent settings panel.

use crate::theme::Theme;
use crate::widgets::agent_settings_caret::{paint_settings_input_view, settings_input_text};
use crate::widgets::agent_settings_form_actions::{
    cancel_button_rect, paint_form_actions, save_button_rect,
};
use crate::widgets::agent_settings_header_action::{
    header_action_rect, header_action_text_baseline_y, header_action_text_x,
};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::button::{paint_ghost_button_feedback, tokens_from_theme};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use jian_widgets::components::button::{Button, ButtonVariant};
use op_editor_core::agent_settings::{
    AcpAgentConfig, AcpAgentField, AcpConnectionType, AgentSettings, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::{AgentSettingsButton, ButtonPressTarget};

const HEADER_H: f32 = 28.0;
const SUBTITLE_H: f32 = 28.0;
const EMPTY_H: f32 = 64.0;
const COMPACT_CARD_H: f32 = 60.0;
const EXPANDED_CARD_H: f32 = 332.0;
const DRAFT_CARD_H: f32 = 370.0;
const CARD_GAP: f32 = 8.0;
const FIELD_H: f32 = 28.0;
const ENV_FIELD_H: f32 = 64.0;
const ACTION_W: f32 = 24.0;
const CONNECT_BTN_W: f32 = 96.0;
const CONNECT_BTN_H: f32 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpHit {
    AddAgent,
    Focus { index: usize, field: AcpAgentField },
    FocusDraft(AcpAgentField),
    ToggleConnectionType(usize),
    ToggleDraftConnectionType,
    SaveDraft,
    CancelDraft,
    Edit(usize),
    Remove(usize),
    ToggleConnected(usize),
    None,
}

pub fn content_height(settings: &AgentSettings) -> f32 {
    let has_draft = settings.acp_agent_draft.is_some();
    let list_h = if settings.acp_agents.is_empty() && !has_draft {
        EMPTY_H
    } else {
        let saved_h: f32 = settings
            .acp_agents
            .iter()
            .enumerate()
            .map(|(index, _)| card_height(settings, index) + CARD_GAP)
            .sum();
        saved_h
            + if has_draft {
                DRAFT_CARD_H + CARD_GAP
            } else {
                0.0
            }
    };
    HEADER_H + SUBTITLE_H + list_h
}

pub fn hit_test(content: Rect, settings: &AgentSettings, point: Point2D, y: f32) -> AcpHit {
    if (add_agent_rect(content, y)).contains(point) {
        return AcpHit::AddAgent;
    }
    let mut card_y = y + HEADER_H + SUBTITLE_H;
    for (index, agent) in settings.acp_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height(settings, index),
        );
        if is_editing(settings, index) {
            if (type_toggle_rect(card)).contains(point) {
                return AcpHit::ToggleConnectionType(index);
            }
            for field in form_fields(agent.connection_type) {
                if (field_input_rect(card, *field)).contains(point) {
                    return AcpHit::Focus {
                        index,
                        field: *field,
                    };
                }
            }
        } else if settings.hover_acp_agent == index && (compact_edit_rect(card)).contains(point) {
            return AcpHit::Edit(index);
        } else if settings.hover_acp_agent == index && (compact_remove_rect(card)).contains(point) {
            return AcpHit::Remove(index);
        } else if (connection_button_rect(card)).contains(point) {
            return AcpHit::ToggleConnected(index);
        }
        card_y += card.size.y + CARD_GAP;
    }
    if let Some(agent) = settings.acp_agent_draft.as_ref() {
        let card = card_rect(content.origin.x, card_y, content.size.x, DRAFT_CARD_H);
        if (type_toggle_rect(card)).contains(point) {
            return AcpHit::ToggleDraftConnectionType;
        }
        for field in form_fields(agent.connection_type) {
            if (field_input_rect(card, *field)).contains(point) {
                return AcpHit::FocusDraft(*field);
            }
        }
        if (save_button_rect(card, EXPANDED_CARD_H)).contains(point) {
            return AcpHit::SaveDraft;
        }
        if (cancel_button_rect(card, EXPANDED_CARD_H)).contains(point) {
            return AcpHit::CancelDraft;
        }
    }
    AcpHit::None
}

pub fn card_at(
    content: Rect,
    settings: &AgentSettings,
    point: Point2D,
    section_y: f32,
) -> Option<usize> {
    let mut card_y = section_y + HEADER_H + SUBTITLE_H;
    for (index, _) in settings.acp_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height(settings, index),
        );
        if (card).contains(point) {
            return Some(index);
        }
        card_y += card.size.y + CARD_GAP;
    }
    None
}

pub fn paint_acp_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
    y: f32,
    now_ms: u64,
) -> f32 {
    let mut y = paint_header(
        cx,
        theme,
        t_settings(ui, "settings.agents.acp"),
        t_settings(ui, "settings.agents.addAcp"),
        content,
        y,
        settings.hover_add_acp_agent,
        ui.button_pressed(ButtonPressTarget::AgentSettings(
            AgentSettingsButton::AddAcpAgent,
        )),
    );
    y = paint_subtitle(
        cx,
        theme,
        t_settings(ui, "settings.agents.acpSubtitle"),
        content,
        y,
    );
    if settings.acp_agents.is_empty() && settings.acp_agent_draft.is_none() {
        return paint_empty(
            cx,
            theme,
            t_settings(ui, "settings.agents.acpEmpty"),
            content,
            y,
        );
    }
    for (index, agent) in settings.acp_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            y,
            content.size.x,
            card_height(settings, index),
        );
        paint_acp_card(cx, theme, settings, ui, agent, index, card, now_ms);
        y += card.size.y + CARD_GAP;
    }
    if let Some(draft) = settings.acp_agent_draft.as_ref() {
        let card = card_rect(content.origin.x, y, content.size.x, DRAFT_CARD_H);
        paint_acp_form(cx, theme, settings, ui, draft, None, card, now_ms);
        paint_form_actions(
            cx,
            theme,
            ui,
            card,
            EXPANDED_CARD_H,
            ui.acp_agent_draft_ready(),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::AcpCancelDraft,
            )),
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::AcpSaveDraft,
            )),
        );
        y += card.size.y + CARD_GAP;
    }
    y
}

#[allow(clippy::too_many_arguments)]
fn paint_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    content: Rect,
    y: f32,
    action_hover: bool,
    action_pressed: bool,
) -> f32 {
    draw_text(
        cx,
        title,
        15.0,
        theme.foreground,
        content.origin.x,
        y + 18.0,
    );
    let action_w = cx.backend.measure_text(action, 12.0);
    let action_rect = header_action_rect(content, y, action_w);
    paint_ghost_button_feedback(cx.backend, theme, action_rect, action_hover, action_pressed);
    draw_text(
        cx,
        action,
        12.0,
        theme.primary,
        header_action_text_x(action_rect, action_w),
        header_action_text_baseline_y(action_rect),
    );
    y + HEADER_H
}

fn paint_subtitle(cx: &mut PaintCx<'_>, theme: &Theme, text: &str, content: Rect, y: f32) -> f32 {
    draw_text(
        cx,
        text,
        12.0,
        theme.muted_foreground,
        content.origin.x,
        y + 16.0,
    );
    y + SUBTITLE_H
}

fn paint_empty(cx: &mut PaintCx<'_>, theme: &Theme, text: &str, content: Rect, y: f32) -> f32 {
    let text_w = cx.backend.measure_text(text, 13.0);
    draw_text(
        cx,
        text,
        13.0,
        theme.muted_foreground,
        content.origin.x + (content.size.x - text_w) / 2.0,
        y + 44.0,
    );
    y + EMPTY_H
}

#[allow(clippy::too_many_arguments)]
fn paint_acp_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    index: usize,
    card: Rect,
    now_ms: u64,
) {
    if is_editing(settings, index) {
        paint_acp_form(cx, theme, settings, ui, agent, Some(index), card, now_ms);
    } else {
        paint_compact_acp_card(cx, theme, settings, ui, agent, index, card);
    }
}

fn paint_compact_acp_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    index: usize,
    card: Rect,
) {
    cx.backend.fill_round_rect(
        card,
        8.0,
        if agent.connected {
            theme.accent
        } else {
            theme.muted
        },
    );
    cx.backend.stroke_round_rect(card, 8.0, theme.border, 1.0);
    paint_avatar(cx, theme, ui, agent, card);

    let text_x = card.origin.x + 60.0;
    let name = ellipsize(cx, &agent.display_name, 190.0, 13.0);
    let name_w = cx.backend.measure_text(&name, 13.0);
    draw_text(
        cx,
        &name,
        13.0,
        theme.foreground,
        text_x,
        card.origin.y + 22.0,
    );
    draw_text(
        cx,
        connection_type_label(ui, agent.connection_type),
        10.0,
        theme.muted_foreground,
        text_x + name_w + 8.0,
        card.origin.y + 22.0,
    );
    let detail = ellipsize(cx, &acp_detail(agent), 245.0, 11.0);
    draw_text(
        cx,
        &detail,
        11.0,
        theme.muted_foreground,
        text_x,
        card.origin.y + 39.0,
    );

    if settings.hover_acp_agent == index {
        paint_action(
            cx,
            theme,
            compact_edit_rect(card),
            Icon::Pencil,
            theme.muted_foreground,
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::AcpEdit(index),
            )),
        );
        paint_action(
            cx,
            theme,
            compact_remove_rect(card),
            Icon::Trash,
            theme.muted_foreground,
            ui.button_pressed(ButtonPressTarget::AgentSettings(
                AgentSettingsButton::AcpRemove(index),
            )),
        );
    }
    paint_connection_button(cx, theme, ui, agent, index, card);
}

#[allow(clippy::too_many_arguments)]
fn paint_acp_form(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    index: Option<usize>,
    card: Rect,
    now_ms: u64,
) {
    cx.backend.fill_round_rect(card, 10.0, theme.muted);
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);
    draw_text(
        cx,
        t_settings(ui, "acp.connectionType"),
        11.0,
        theme.muted_foreground,
        card.origin.x + 12.0,
        type_toggle_rect(card).origin.y - 8.0,
    );
    paint_type_toggle(cx, theme, ui, agent, card);
    for field in form_fields(agent.connection_type) {
        paint_field(cx, theme, settings, ui, agent, index, *field, card, now_ms);
    }
}

fn paint_avatar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    card: Rect,
) {
    let avatar = Rect {
        origin: Point2D::new(card.origin.x + 12.0, card.origin.y + 12.0),
        size: Point2D::new(36.0, 36.0),
    };
    cx.backend.fill_round_rect(avatar, 8.0, theme.card);
    let icon = match agent.connection_type {
        AcpConnectionType::Local => Icon::Terminal,
        AcpConnectionType::Remote => Icon::Globe,
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(avatar.origin.x + 9.0, avatar.origin.y + 9.0),
        18.0,
        theme.foreground,
        1.6,
    );
    let _ = ui;
}

fn paint_connection_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    index: usize,
    card: Rect,
) {
    let btn = connection_button_rect(card);
    let enabled = agent.ready() || agent.connected;
    if agent.connected {
        cx.backend.fill_round_rect(btn, 6.0, theme.muted);
        cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
    } else {
        Button {
            label: "",
            icon_d: None,
            variant: if enabled {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Outline
            },
            enabled: true,
            hovered: !enabled,
            pressed: false,
            font_size: 12.0,
        }
        .paint(cx.backend, btn, &tokens_from_theme(theme));
    }
    if ui.button_pressed(ButtonPressTarget::AgentSettings(
        AgentSettingsButton::AcpConnection(index),
    )) {
        paint_ghost_button_feedback(cx.backend, theme, btn, false, true);
    }
    let fg = if agent.connected {
        Color {
            r: 0.93,
            g: 0.30,
            b: 0.30,
            a: 1.0,
        }
    } else if enabled {
        theme.primary_foreground
    } else {
        theme.muted_foreground
    };
    let label = if agent.connected {
        t_settings(ui, "settings.agents.disconnect")
    } else if agent.ready() {
        t_settings(ui, "settings.agents.connect")
    } else {
        "Configure"
    };
    let lw = cx.backend.measure_text(label, 12.0);
    draw_text(
        cx,
        label,
        12.0,
        fg,
        btn.origin.x + (btn.size.x - lw) / 2.0,
        btn.origin.y + 18.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_field(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    index: Option<usize>,
    field: AcpAgentField,
    card: Rect,
    now_ms: u64,
) {
    let focus = match index {
        Some(index) => SettingsFocus::AcpAgent { index, field },
        None => SettingsFocus::AcpAgentDraft(field),
    };
    let focused = settings.focus == Some(focus);
    let fallback = match field {
        AcpAgentField::DisplayName => agent.display_name.clone(),
        AcpAgentField::Command => agent.command.clone(),
        AcpAgentField::Args => agent.args_text(),
        AcpAgentField::Env => agent.env_text(),
        AcpAgentField::Url => agent.url.clone().unwrap_or_default(),
    };
    let value = settings_input_text(settings, ui, focus, &fallback);
    let label = match field {
        AcpAgentField::DisplayName => t_settings(ui, "acp.displayName"),
        AcpAgentField::Command => t_settings(ui, "acp.command"),
        AcpAgentField::Args => t_settings(ui, "acp.args"),
        AcpAgentField::Env => t_settings(ui, "acp.env"),
        AcpAgentField::Url => "URL",
    };
    let input = field_input_rect(card, field);
    let label_y = input.origin.y - 8.0;
    draw_text(
        cx,
        label,
        11.0,
        theme.muted_foreground,
        card.origin.x + 12.0,
        label_y,
    );
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
    let placeholder = match field {
        AcpAgentField::DisplayName => t_settings(ui, "acp.displayNamePlaceholder"),
        AcpAgentField::Command => t_settings(ui, "acp.commandPlaceholder"),
        AcpAgentField::Args => t_settings(ui, "acp.argsPlaceholder"),
        AcpAgentField::Env => t_settings(ui, "acp.envPlaceholder"),
        AcpAgentField::Url => "https://agent.example.com",
    };
    let text_x = input.origin.x + 6.0;
    if focused {
        paint_settings_input_view(
            cx,
            theme,
            ui,
            input,
            11.0,
            text_x - input.origin.x,
            input.origin.y + 16.0,
            now_ms,
            placeholder,
        );
    } else {
        let shown = if value.is_empty() { placeholder } else { value };
        let text_color = if value.is_empty() {
            theme.muted_foreground
        } else {
            theme.foreground
        };
        let clipped = ellipsize(cx, shown, input.size.x - 12.0, 11.0);
        draw_text(
            cx,
            &clipped,
            11.0,
            text_color,
            text_x,
            input.origin.y + 16.0,
        );
    }
}

fn paint_type_toggle(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    card: Rect,
) {
    let r = type_toggle_rect(card);
    cx.backend.stroke_round_rect(r, 6.0, theme.border, 1.0);
    let half = r.size.x / 2.0;
    for (i, kind) in [AcpConnectionType::Local, AcpConnectionType::Remote]
        .iter()
        .enumerate()
    {
        let item = Rect {
            origin: Point2D::new(r.origin.x + i as f32 * half, r.origin.y),
            size: Point2D::new(half, r.size.y),
        };
        let active = agent.connection_type == *kind;
        if active {
            cx.backend.fill_round_rect(item, 5.0, theme.primary);
        }
        let color = if active {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        };
        let label = connection_type_label(ui, *kind);
        let icon = match kind {
            AcpConnectionType::Local => Icon::Terminal,
            AcpConnectionType::Remote => Icon::Globe,
        };
        let tw = cx.backend.measure_text(label, 11.0);
        let group_w = 16.0 + 6.0 + tw;
        let group_x = item.origin.x + (item.size.x - group_w) / 2.0;
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(group_x, item.origin.y + 6.0),
            14.0,
            color,
            1.5,
        );
        draw_text(cx, label, 11.0, color, group_x + 22.0, item.origin.y + 18.0);
    }
}

fn paint_action(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    icon: Icon,
    color: Color,
    pressed: bool,
) {
    paint_ghost_button_feedback(cx.backend, theme, rect, true, pressed);
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 6.0, rect.origin.y + 6.0),
        12.0,
        color,
        1.4,
    );
}

fn acp_detail(agent: &AcpAgentConfig) -> String {
    match agent.connection_type {
        AcpConnectionType::Local if agent.command.trim().is_empty() => "Command required".into(),
        AcpConnectionType::Local if agent.args.is_empty() => agent.command.clone(),
        AcpConnectionType::Local => format!("{} {}", agent.command, agent.args.join(" ")),
        AcpConnectionType::Remote => agent.url.clone().unwrap_or_else(|| "URL required".into()),
    }
}

fn connection_type_label(ui: &EditorUiState, kind: AcpConnectionType) -> &'static str {
    match kind {
        AcpConnectionType::Local => t_settings(ui, "acp.local"),
        AcpConnectionType::Remote => t_settings(ui, "acp.remote"),
    }
}

fn form_fields(kind: AcpConnectionType) -> &'static [AcpAgentField] {
    const LOCAL: &[AcpAgentField] = &[
        AcpAgentField::DisplayName,
        AcpAgentField::Command,
        AcpAgentField::Args,
        AcpAgentField::Env,
    ];
    const REMOTE: &[AcpAgentField] = &[AcpAgentField::DisplayName, AcpAgentField::Url];
    match kind {
        AcpConnectionType::Local => LOCAL,
        AcpConnectionType::Remote => REMOTE,
    }
}

fn is_editing(settings: &AgentSettings, index: usize) -> bool {
    matches!(
        settings.focus,
        Some(SettingsFocus::AcpAgent { index: i, .. }) if i == index
    )
}

fn card_height(settings: &AgentSettings, index: usize) -> f32 {
    if is_editing(settings, index) {
        EXPANDED_CARD_H
    } else {
        COMPACT_CARD_H
    }
}

fn add_agent_rect(content: Rect, y: f32) -> Rect {
    header_action_rect(content, y, 0.0)
}

fn card_rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, h),
    }
}

fn compact_edit_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            connection_button_rect(card).origin.x - 8.0 - ACTION_W * 2.0 - 4.0,
            card.origin.y + (card.size.y - ACTION_W) / 2.0,
        ),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

fn compact_remove_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            compact_edit_rect(card).origin.x + ACTION_W + 4.0,
            card.origin.y + (card.size.y - ACTION_W) / 2.0,
        ),
        size: Point2D::new(ACTION_W, ACTION_W),
    }
}

fn connection_button_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 12.0 - CONNECT_BTN_W,
            card.origin.y + (card.size.y - CONNECT_BTN_H) / 2.0,
        ),
        size: Point2D::new(CONNECT_BTN_W, CONNECT_BTN_H),
    }
}

fn type_toggle_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(card.origin.x + 12.0, card.origin.y + 100.0),
        size: Point2D::new(card.size.x - 24.0, 28.0),
    }
}

fn field_input_rect(card: Rect, field: AcpAgentField) -> Rect {
    let y = match field {
        AcpAgentField::DisplayName => card.origin.y + 34.0,
        AcpAgentField::Command | AcpAgentField::Url => card.origin.y + 154.0,
        AcpAgentField::Args => card.origin.y + 208.0,
        AcpAgentField::Env => card.origin.y + 262.0,
    };
    let h = if field == AcpAgentField::Env {
        ENV_FIELD_H
    } else {
        FIELD_H
    };
    Rect {
        origin: Point2D::new(card.origin.x + 12.0, y),
        size: Point2D::new(card.size.x - 24.0, h),
    }
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y));
}

fn ellipsize(cx: &mut PaintCx<'_>, value: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text(value, size) <= max_w {
        return value.to_string();
    }
    let mut out = value.to_string();
    while !out.is_empty() && cx.backend.measure_text(&format!("{out}..."), size) > max_w {
        out.pop();
    }
    format!("{out}...")
}
