//! ACP Agent section for the Agent settings panel.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{AcpAgentConfig, AcpConnectionType, AgentSettings};
use op_editor_core::editor_ui_state::EditorUiState;

const HEADER_H: f32 = 28.0;
const SUBTITLE_H: f32 = 28.0;
const EMPTY_H: f32 = 64.0;
const CARD_H: f32 = 60.0;
const CARD_GAP: f32 = 8.0;
const TOP_HEADER_RIGHT_INSET: f32 = 12.0;
const ADD_W: f32 = 96.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpHit {
    AddAgent,
    None,
}

pub fn content_height(settings: &AgentSettings) -> f32 {
    let list_h = if settings.acp_agents.is_empty() {
        EMPTY_H
    } else {
        settings.acp_agents.len() as f32 * (CARD_H + CARD_GAP)
    };
    HEADER_H + SUBTITLE_H + list_h
}

pub fn hit_test(content: Rect, point: Point2D, y: f32) -> AcpHit {
    if rect_contains(add_agent_rect(content, y), point) {
        return AcpHit::AddAgent;
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
    if settings.acp_agents.is_empty() {
        return paint_empty(
            cx,
            theme,
            t_settings(ui, "settings.agents.acpEmpty"),
            content,
            y,
        );
    }
    for agent in &settings.acp_agents {
        paint_acp_card(
            cx,
            theme,
            ui,
            agent,
            card_rect(content.origin.x, y, content.size.x),
        );
        y += CARD_H + CARD_GAP;
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

fn paint_acp_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    agent: &AcpAgentConfig,
    card: Rect,
) {
    if agent.enabled {
        cx.backend.fill_round_rect(card, 8.0, theme.muted);
        cx.backend.stroke_round_rect(card, 8.0, theme.border, 1.0);
    }
    let avatar = Rect {
        origin: Point2D::new(card.origin.x + 12.0, card.origin.y + 12.0),
        size: Point2D::new(36.0, 36.0),
    };
    cx.backend.fill_round_rect(avatar, 8.0, theme.card);
    let (icon, type_label) = match agent.connection_type {
        AcpConnectionType::Local => (Icon::Terminal, t_settings(ui, "acp.local")),
        AcpConnectionType::Remote => (Icon::Globe, t_settings(ui, "acp.remote")),
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(avatar.origin.x + 9.0, avatar.origin.y + 9.0),
        18.0,
        theme.foreground,
        1.6,
    );

    let text_x = card.origin.x + 60.0;
    let name = ellipsize(cx, &agent.display_name, 260.0, 13.0);
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
        type_label,
        10.0,
        theme.muted_foreground,
        text_x + name_w + 8.0,
        card.origin.y + 22.0,
    );
    let detail = acp_detail(agent);
    let detail = ellipsize(cx, &detail, 310.0, 11.0);
    draw_text(
        cx,
        &detail,
        11.0,
        theme.muted_foreground,
        text_x,
        card.origin.y + 39.0,
    );
    draw_text(
        cx,
        t_settings(ui, "acp.notConnected"),
        11.0,
        theme.muted_foreground,
        card.origin.x + card.size.x - 96.0,
        card.origin.y + 35.0,
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

fn add_agent_rect(content: Rect, y: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            content.origin.x + content.size.x - TOP_HEADER_RIGHT_INSET - ADD_W,
            y,
        ),
        size: Point2D::new(ADD_W, 24.0),
    }
}

fn card_rect(x: f32, y: f32, w: f32) -> Rect {
    Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(w, CARD_H),
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
