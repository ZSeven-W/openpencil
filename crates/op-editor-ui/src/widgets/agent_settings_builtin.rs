//! Built-in provider section for the Agent settings panel.

use crate::theme::Theme;
use crate::widgets::agent_settings_builtin_draft;
use crate::widgets::agent_settings_form_actions::{
    cancel_button_rect, paint_form_actions, save_button_rect,
};
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::brand_icons::{paint_brand_logo, BrandLogo};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentField, BuiltinAgentKind, SettingsFocus,
};
use op_editor_core::editor_ui_state::EditorUiState;

const HEADER_HEIGHT: f32 = 28.0;
const SUBTITLE_HEIGHT: f32 = 28.0;
const EMPTY_HEIGHT: f32 = 64.0;
const COMPACT_CARD_HEIGHT: f32 = 60.0;
const EXPANDED_CARD_HEIGHT: f32 = 168.0;
const DRAFT_CARD_HEIGHT: f32 = 204.0;
const CARD_GAP: f32 = 8.0;
const ADD_W: f32 = 96.0;
const TOP_HEADER_RIGHT_INSET: f32 = 12.0;
const FIELD_LABEL_W: f32 = 68.0;
const FIELD_H: f32 = 24.0;
const SWITCH_W: f32 = 34.0;
const SWITCH_H: f32 = 20.0;
const ACTION_W: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinHit {
    AddProvider,
    Focus {
        index: usize,
        field: BuiltinAgentField,
    },
    FocusDraft(BuiltinAgentField),
    ToggleKind(usize),
    ToggleDraftKind,
    SaveDraft,
    CancelDraft,
    ToggleEnabled(usize),
    Edit(usize),
    Remove(usize),
    None,
}

pub fn content_height(settings: &AgentSettings) -> f32 {
    let has_draft = settings.builtin_agent_draft.is_some();
    let list_h = if settings.builtin_agents.is_empty() && !has_draft {
        EMPTY_HEIGHT
    } else {
        let saved_h: f32 = settings
            .builtin_agents
            .iter()
            .enumerate()
            .map(|(index, _)| card_height(settings, index) + CARD_GAP)
            .sum();
        saved_h
            + if has_draft {
                DRAFT_CARD_HEIGHT + CARD_GAP
            } else {
                0.0
            }
    };
    HEADER_HEIGHT + SUBTITLE_HEIGHT + list_h
}

pub fn hit_test(content: Rect, settings: &AgentSettings, point: Point2D) -> BuiltinHit {
    let y = content.origin.y + 12.0;
    if rect_contains(add_provider_rect(content, y), point) {
        return BuiltinHit::AddProvider;
    }
    let mut card_y = y + HEADER_HEIGHT + SUBTITLE_HEIGHT;
    for (index, _) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height(settings, index),
        );
        if is_editing(settings, index) {
            if rect_contains(kind_rect(card), point) {
                return BuiltinHit::ToggleKind(index);
            }
            for (row, field) in [
                BuiltinAgentField::DisplayName,
                BuiltinAgentField::ApiKey,
                BuiltinAgentField::Model,
                BuiltinAgentField::BaseUrl,
            ]
            .into_iter()
            .enumerate()
            {
                if rect_contains(field_input_rect(card, row), point) {
                    return BuiltinHit::Focus { index, field };
                }
            }
        } else if rect_contains(compact_switch_rect(card), point) {
            return BuiltinHit::ToggleEnabled(index);
        } else if rect_contains(compact_edit_rect(card), point) {
            return BuiltinHit::Edit(index);
        } else if rect_contains(compact_remove_rect(card), point) {
            return BuiltinHit::Remove(index);
        }
        card_y += card.size.y + CARD_GAP;
    }
    if settings.builtin_agent_draft.is_some() {
        let card = card_rect(content.origin.x, card_y, content.size.x, DRAFT_CARD_HEIGHT);
        if rect_contains(kind_rect(card), point) {
            return BuiltinHit::ToggleDraftKind;
        }
        for (row, field) in [
            BuiltinAgentField::DisplayName,
            BuiltinAgentField::ApiKey,
            BuiltinAgentField::Model,
            BuiltinAgentField::BaseUrl,
        ]
        .into_iter()
        .enumerate()
        {
            if rect_contains(field_input_rect(card, row), point) {
                return BuiltinHit::FocusDraft(field);
            }
        }
        if rect_contains(save_button_rect(card, EXPANDED_CARD_HEIGHT), point) {
            return BuiltinHit::SaveDraft;
        }
        if rect_contains(cancel_button_rect(card, EXPANDED_CARD_HEIGHT), point) {
            return BuiltinHit::CancelDraft;
        }
    }
    BuiltinHit::None
}

pub fn card_at(content: Rect, settings: &AgentSettings, point: Point2D) -> Option<usize> {
    let mut card_y = content.origin.y + 12.0 + HEADER_HEIGHT + SUBTITLE_HEIGHT;
    for (index, _) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            card_y,
            content.size.x,
            card_height(settings, index),
        );
        if rect_contains(card, point) {
            return Some(index);
        }
        card_y += card.size.y + CARD_GAP;
    }
    None
}

pub fn paint_builtin_section(
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
        t_settings(ui, "settings.agents.builtin"),
        t_settings(ui, "settings.agents.addProvider"),
        content.origin.x,
        y,
        content.size.x,
    );
    y = paint_subtitle(
        cx,
        theme,
        t_settings(ui, "settings.agents.builtinSubtitle"),
        content.origin.x,
        y,
    );
    if settings.builtin_agents.is_empty() && settings.builtin_agent_draft.is_none() {
        return paint_empty(
            cx,
            theme,
            t_settings(ui, "settings.agents.builtinEmpty"),
            content.origin.x,
            y,
            content.size.x,
        );
    }
    for (index, agent) in settings.builtin_agents.iter().enumerate() {
        let card = card_rect(
            content.origin.x,
            y,
            content.size.x,
            card_height(settings, index),
        );
        paint_builtin_agent_card(cx, theme, settings, ui, agent, index, card, now_ms);
        y += card.size.y + CARD_GAP;
    }
    if let Some(draft) = settings.builtin_agent_draft.as_ref() {
        let card = card_rect(content.origin.x, y, content.size.x, DRAFT_CARD_HEIGHT);
        paint_builtin_agent_form(cx, theme, settings, ui, draft, None, card, now_ms);
        paint_form_actions(
            cx,
            theme,
            ui,
            card,
            EXPANDED_CARD_HEIGHT,
            agent_settings_builtin_draft::ready(settings, ui),
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
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let layout = TextLayout::single_run(
        title,
        "system-ui",
        15.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y + 18.0));
    let action_w = cx.backend.measure_text(action, 12.0);
    let act = TextLayout::single_run(
        action,
        "system-ui",
        12.0,
        to_jian(theme.primary),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &act,
        Point2D::new(x + w - TOP_HEADER_RIGHT_INSET - action_w, y + 18.0),
    );
    y + HEADER_HEIGHT
}

fn paint_subtitle(cx: &mut PaintCx<'_>, theme: &Theme, text: &str, x: f32, y: f32) -> f32 {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        12.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y + 16.0));
    y + SUBTITLE_HEIGHT
}

fn paint_empty(cx: &mut PaintCx<'_>, theme: &Theme, text: &str, x: f32, y: f32, w: f32) -> f32 {
    let text_w = cx.backend.measure_text(text, 13.0);
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        13.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(x + (w - text_w) / 2.0, y + 44.0));
    y + EMPTY_HEIGHT
}

#[allow(clippy::too_many_arguments)]
fn paint_builtin_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: usize,
    card: Rect,
    now_ms: u64,
) {
    if !is_editing(settings, index) {
        paint_compact_builtin_agent_card(cx, theme, settings, ui, agent, index, card);
        return;
    }
    paint_builtin_agent_form(cx, theme, settings, ui, agent, Some(index), card, now_ms);
}

#[allow(clippy::too_many_arguments)]
fn paint_builtin_agent_form(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    card: Rect,
    now_ms: u64,
) {
    cx.backend.fill_round_rect(card, 10.0, theme.muted);
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);
    let avatar = Rect {
        origin: Point2D::new(card.origin.x + 12.0, card.origin.y + 10.0),
        size: Point2D::new(28.0, 28.0),
    };
    cx.backend.fill_round_rect(avatar, 6.0, theme.background);
    let logo = match agent.kind {
        BuiltinAgentKind::Anthropic => BrandLogo::Claude,
        BuiltinAgentKind::OpenAiCompat => BrandLogo::OpenAI,
    };
    paint_brand_logo(
        cx.backend,
        logo,
        Point2D::new(avatar.origin.x + 6.0, avatar.origin.y + 6.0),
        16.0,
        theme.foreground,
    );
    let ready = agent.ready();
    let status = if ready { "ready" } else { "api key required" };
    let status_color = if ready {
        Color {
            r: 0.34,
            g: 0.78,
            b: 0.45,
            a: 1.0,
        }
    } else {
        theme.muted_foreground
    };
    draw_text(
        cx,
        status,
        11.0,
        status_color,
        card.origin.x + 48.0,
        card.origin.y + 28.0,
    );

    paint_kind_toggle(cx, theme, agent, card);
    paint_field(
        cx,
        theme,
        settings,
        ui,
        agent,
        index,
        BuiltinAgentField::DisplayName,
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
        BuiltinAgentField::ApiKey,
        1,
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
        BuiltinAgentField::Model,
        2,
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
        BuiltinAgentField::BaseUrl,
        3,
        card,
        now_ms,
    );
}

fn paint_compact_builtin_agent_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    agent: &BuiltinAgentConfig,
    index: usize,
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
    paint_key_glyph(cx, theme, avatar);

    let text_x = card.origin.x + 60.0;
    let name = ellipsize(cx, &agent.display_name, 250.0, 13.0);
    draw_text(
        cx,
        &name,
        13.0,
        theme.foreground,
        text_x,
        card.origin.y + 22.0,
    );
    let api_key = if agent.api_key.trim().is_empty() {
        "api key required".to_string()
    } else {
        mask_key(&agent.api_key)
    };
    let detail = format!("{}  ·  {}", agent.model, api_key);
    let detail = ellipsize(cx, &detail, 300.0, 11.0);
    draw_text(
        cx,
        &detail,
        11.0,
        theme.muted_foreground,
        text_x,
        card.origin.y + 38.0,
    );
    if agent.ready() {
        let green = Color {
            r: 0.34,
            g: 0.78,
            b: 0.45,
            a: 1.0,
        };
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(text_x, card.origin.y + 44.0),
            10.0,
            green,
            2.2,
        );
        draw_text(
            cx,
            op_i18n::translate(ui.locale, "builtin.ready"),
            11.0,
            green,
            text_x + 14.0,
            card.origin.y + 53.0,
        );
    }

    paint_switch(cx, theme, compact_switch_rect(card), agent.enabled);
    if settings.hover_builtin_agent == index {
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
    let focused = match index {
        Some(index) => settings.focus == Some(SettingsFocus::BuiltinAgent { index, field }),
        None => settings.focus == Some(SettingsFocus::BuiltinAgentDraft(field)),
    };
    let value = if focused {
        ui.settings_input_draft.as_str()
    } else {
        match field {
            BuiltinAgentField::DisplayName => agent.display_name.as_str(),
            BuiltinAgentField::ApiKey if !agent.api_key.is_empty() => "********",
            BuiltinAgentField::ApiKey => "",
            BuiltinAgentField::Model => agent.model.as_str(),
            BuiltinAgentField::BaseUrl => agent.base_url.as_str(),
        }
    };
    let label = match field {
        BuiltinAgentField::DisplayName => "Name",
        BuiltinAgentField::ApiKey => "API Key",
        BuiltinAgentField::Model => "Model",
        BuiltinAgentField::BaseUrl => "Base URL",
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
    if focused && jian_core::anim::blink_visible(now_ms, ui.settings_input_caret_anchor_ms, 500) {
        let caret_x = (text_x + cx.backend.measure_text(&clipped, 11.0) + 1.0)
            .min(input.origin.x + input.size.x - 8.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(caret_x, input.origin.y + 4.5),
                size: Point2D::new(1.5, 15.0),
            },
            theme.foreground,
        );
    }
}

fn paint_kind_toggle(cx: &mut PaintCx<'_>, theme: &Theme, agent: &BuiltinAgentConfig, card: Rect) {
    let r = kind_rect(card);
    cx.backend.stroke_round_rect(r, 6.0, theme.border, 1.0);
    let half = r.size.x / 2.0;
    for (i, (label, kind)) in [
        ("Anthropic", BuiltinAgentKind::Anthropic),
        ("OpenAI", BuiltinAgentKind::OpenAiCompat),
    ]
    .iter()
    .enumerate()
    {
        let item = Rect {
            origin: Point2D::new(r.origin.x + i as f32 * half, r.origin.y),
            size: Point2D::new(half, r.size.y),
        };
        let active = agent.kind == *kind;
        if active {
            cx.backend.fill_round_rect(item, 5.0, theme.primary);
        }
        let color = if active {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        };
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

fn paint_key_glyph(cx: &mut PaintCx<'_>, theme: &Theme, avatar: Rect) {
    let color = theme.foreground;
    let cx0 = avatar.origin.x + 13.0;
    let cy0 = avatar.origin.y + 17.0;
    cx.backend.stroke_round_rect(
        Rect {
            origin: Point2D::new(cx0 - 4.0, cy0 - 4.0),
            size: Point2D::new(8.0, 8.0),
        },
        4.0,
        color,
        1.6,
    );
    cx.backend.stroke_line(
        Point2D::new(cx0 + 4.0, cy0),
        Point2D::new(avatar.origin.x + 27.0, cy0),
        color,
        1.6,
    );
    cx.backend.stroke_line(
        Point2D::new(avatar.origin.x + 23.0, cy0),
        Point2D::new(avatar.origin.x + 23.0, cy0 + 4.0),
        color,
        1.6,
    );
    cx.backend.stroke_line(
        Point2D::new(avatar.origin.x + 27.0, cy0),
        Point2D::new(avatar.origin.x + 27.0, cy0 + 4.0),
        color,
        1.6,
    );
}

fn paint_switch(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, enabled: bool) {
    let bg = if enabled { theme.primary } else { theme.muted };
    cx.backend.fill_round_rect(rect, rect.size.y / 2.0, bg);
    cx.backend
        .stroke_round_rect(rect, rect.size.y / 2.0, theme.border, 1.0);
    let knob_x = if enabled {
        rect.origin.x + rect.size.x - 17.0
    } else {
        rect.origin.x + 3.0
    };
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(knob_x, rect.origin.y + 3.0),
            size: Point2D::new(14.0, 14.0),
        },
        7.0,
        theme.primary_foreground,
    );
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

fn mask_key(api_key: &str) -> String {
    if api_key.len() > 12 {
        format!("{}***{}", &api_key[..7], &api_key[api_key.len() - 3..])
    } else {
        "***".to_string()
    }
}

fn is_editing(settings: &AgentSettings, index: usize) -> bool {
    matches!(
        settings.focus,
        Some(SettingsFocus::BuiltinAgent { index: i, .. }) if i == index
    )
}

fn card_height(settings: &AgentSettings, index: usize) -> f32 {
    if is_editing(settings, index) {
        EXPANDED_CARD_HEIGHT
    } else {
        COMPACT_CARD_HEIGHT
    }
}

fn add_provider_rect(content: Rect, y: f32) -> Rect {
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

fn compact_switch_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 12.0 - SWITCH_W - 8.0 - ACTION_W * 2.0 - 4.0,
            card.origin.y + (card.size.y - SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SWITCH_W, SWITCH_H),
    }
}

fn compact_edit_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            compact_switch_rect(card).origin.x + SWITCH_W + 8.0,
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

fn kind_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(card.origin.x + card.size.x - 172.0, card.origin.y + 10.0),
        size: Point2D::new(156.0, 24.0),
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
