//! ACP Agent section for the Agent settings panel.

use crate::theme::Theme;
use crate::widgets::agent_settings_acp_draft;
use crate::widgets::agent_settings_caret::paint_caret;
use crate::widgets::agent_settings_form_actions::{
    cancel_button_rect, paint_form_actions, save_button_rect,
};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AcpAgentConfig, AcpAgentField, AcpConnectionType, AgentSettings, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;

const HEADER_H: f32 = 28.0;
const SUBTITLE_H: f32 = 28.0;
const EMPTY_H: f32 = 64.0;
const COMPACT_CARD_H: f32 = 60.0;
const EXPANDED_CARD_H: f32 = 116.0;
const DRAFT_CARD_H: f32 = 152.0;
const CARD_GAP: f32 = 8.0;
const TOP_HEADER_RIGHT_INSET: f32 = 12.0;
const ADD_W: f32 = 96.0;
const FIELD_LABEL_W: f32 = 72.0;
const FIELD_H: f32 = 24.0;
const TYPE_TOGGLE_W: f32 = 156.0;
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
    if rect_contains(add_agent_rect(content, y), point) {
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
            if rect_contains(type_toggle_rect(card), point) {
                return AcpHit::ToggleConnectionType(index);
            }
            for (row, field) in [
                AcpAgentField::DisplayName,
                connection_field(agent.connection_type),
            ]
            .into_iter()
            .enumerate()
            {
                if rect_contains(field_input_rect(card, row), point) {
                    return AcpHit::Focus { index, field };
                }
            }
        } else if rect_contains(compact_edit_rect(card), point) {
            return AcpHit::Edit(index);
        } else if rect_contains(compact_remove_rect(card), point) {
            return AcpHit::Remove(index);
        } else if rect_contains(connection_button_rect(card), point) {
            return AcpHit::ToggleConnected(index);
        }
        card_y += card.size.y + CARD_GAP;
    }
    if let Some(agent) = settings.acp_agent_draft.as_ref() {
        let card = card_rect(content.origin.x, card_y, content.size.x, DRAFT_CARD_H);
        if rect_contains(type_toggle_rect(card), point) {
            return AcpHit::ToggleDraftConnectionType;
        }
        for (row, field) in [
            AcpAgentField::DisplayName,
            connection_field(agent.connection_type),
        ]
        .into_iter()
        .enumerate()
        {
            if rect_contains(field_input_rect(card, row), point) {
                return AcpHit::FocusDraft(field);
            }
        }
        if rect_contains(save_button_rect(card, EXPANDED_CARD_H), point) {
            return AcpHit::SaveDraft;
        }
        if rect_contains(cancel_button_rect(card, EXPANDED_CARD_H), point) {
            return AcpHit::CancelDraft;
        }
    }
    AcpHit::None
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
            agent_settings_acp_draft::ready(settings, ui),
        );
        y += card.size.y + CARD_GAP;
    }
    y
}

fn paint_header(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    title: &str,
    action: &str,
    content: Rect,
    y: f32,
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
    draw_text(
        cx,
        action,
        12.0,
        theme.primary,
        content.origin.x + content.size.x - TOP_HEADER_RIGHT_INSET - action_w,
        y + 18.0,
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
        paint_compact_acp_card(cx, theme, ui, agent, card);
    }
}

fn paint_compact_acp_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
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

    paint_action(
        cx,
        theme,
        compact_edit_rect(card),
        Icon::Pencil,
        theme.muted_foreground,
    );
    paint_action(
        cx,
        theme,
        compact_remove_rect(card),
        Icon::Trash,
        theme.muted_foreground,
    );
    paint_connection_button(cx, theme, ui, agent, card);
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
    paint_avatar(cx, theme, ui, agent, card);

    let (status, status_color) = acp_status(theme, agent);
    draw_text(
        cx,
        status,
        11.0,
        status_color,
        card.origin.x + 60.0,
        card.origin.y + 29.0,
    );
    paint_type_toggle(cx, theme, ui, agent, card);
    paint_field(
        cx,
        theme,
        settings,
        ui,
        agent,
        index,
        AcpAgentField::DisplayName,
        0,
        card,
        now_ms,
    );
    paint_field(
        cx,
        theme,
        settings,
        ui,
        agent,
        index,
        connection_field(agent.connection_type),
        1,
        card,
        now_ms,
    );
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
    card: Rect,
) {
    let btn = connection_button_rect(card);
    let enabled = agent.ready() || agent.connected;
    let bg = if agent.connected {
        theme.muted
    } else if enabled {
        theme.primary
    } else {
        theme.button_hover
    };
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
    cx.backend.fill_round_rect(btn, 6.0, bg);
    cx.backend.stroke_round_rect(btn, 6.0, theme.border, 1.0);
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
    row: usize,
    card: Rect,
    now_ms: u64,
) {
    let focused = match index {
        Some(index) => settings.focus == Some(SettingsFocus::AcpAgent { index, field }),
        None => settings.focus == Some(SettingsFocus::AcpAgentDraft(field)),
    };
    let value = if focused {
        ui.settings_input_draft.as_str()
    } else {
        match field {
            AcpAgentField::DisplayName => agent.display_name.as_str(),
            AcpAgentField::Command => agent.command.as_str(),
            AcpAgentField::Url => agent.url.as_deref().unwrap_or(""),
        }
    };
    let label = match field {
        AcpAgentField::DisplayName => "Name",
        AcpAgentField::Command => "Command",
        AcpAgentField::Url => "URL",
    };
    let label_y = field_input_rect(card, row).origin.y + 16.0;
    draw_text(
        cx,
        label,
        11.0,
        theme.muted_foreground,
        card.origin.x + 12.0,
        label_y,
    );
    let input = field_input_rect(card, row);
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
    draw_text(
        cx,
        &clipped,
        11.0,
        theme.foreground,
        text_x,
        input.origin.y + 16.0,
    );
    if focused {
        let caret_x = (text_x + cx.backend.measure_text(&clipped, 11.0) + 1.0)
            .min(input.origin.x + input.size.x - 8.0);
        let caret_y = input.origin.y + 4.5;
        let anchor = ui.settings_input_caret_anchor_ms;
        paint_caret(cx, theme, now_ms, anchor, caret_x, caret_y);
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
        let tw = cx.backend.measure_text(label, 10.0);
        draw_text(
            cx,
            label,
            10.0,
            color,
            item.origin.x + (item.size.x - tw) / 2.0,
            item.origin.y + 16.0,
        );
    }
}

fn paint_action(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, icon: Icon, color: Color) {
    cx.backend.fill_round_rect(rect, 6.0, theme.button_hover);
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

fn acp_status(theme: &Theme, agent: &AcpAgentConfig) -> (&'static str, Color) {
    if agent.connected {
        (
            "connected",
            Color {
                r: 0.34,
                g: 0.78,
                b: 0.45,
                a: 1.0,
            },
        )
    } else if agent.ready() {
        ("ready", theme.muted_foreground)
    } else {
        ("not configured", theme.muted_foreground)
    }
}

fn connection_type_label(ui: &EditorUiState, kind: AcpConnectionType) -> &'static str {
    match kind {
        AcpConnectionType::Local => t_settings(ui, "acp.local"),
        AcpConnectionType::Remote => t_settings(ui, "acp.remote"),
    }
}

fn connection_field(kind: AcpConnectionType) -> AcpAgentField {
    match kind {
        AcpConnectionType::Local => AcpAgentField::Command,
        AcpConnectionType::Remote => AcpAgentField::Url,
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
    Rect {
        origin: Point2D::new(
            content.origin.x + content.size.x - TOP_HEADER_RIGHT_INSET - ADD_W,
            y,
        ),
        size: Point2D::new(ADD_W, 24.0),
    }
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
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - TYPE_TOGGLE_W,
            card.origin.y + 10.0,
        ),
        size: Point2D::new(TYPE_TOGGLE_W, 24.0),
    }
}

fn field_input_rect(card: Rect, row: usize) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + 12.0 + FIELD_LABEL_W,
            card.origin.y + 48.0 + row as f32 * 28.0,
        ),
        size: Point2D::new(card.size.x - 24.0 - FIELD_LABEL_W, FIELD_H),
    }
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian(color),
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

fn rect_contains(r: Rect, p: Point2D) -> bool {
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
