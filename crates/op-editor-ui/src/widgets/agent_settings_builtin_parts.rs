//! Shared paint and hit helpers for built-in provider forms.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetMenuTarget,
};
use op_editor_core::{BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS};

const FIELD_LABEL_W: f32 = 68.0;
const FIELD_H: f32 = 24.0;
const PRESET_MENU_ITEM_H: f32 = 24.0;
const PRESET_MENU_PAD: f32 = 4.0;

pub fn paint_kind_toggle(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    agent: &BuiltinAgentConfig,
    card: Rect,
) {
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

pub fn paint_provider_select(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    agent: &BuiltinAgentConfig,
    card: Rect,
) {
    let label_y = provider_select_rect(card).origin.y + 16.0;
    draw_text(
        cx,
        "Provider",
        11.0,
        theme.muted_foreground,
        card.origin.x + 12.0,
        label_y,
    );
    let input = provider_select_rect(card);
    cx.backend.fill_round_rect(input, 6.0, theme.card);
    cx.backend.stroke_round_rect(input, 6.0, theme.border, 1.0);
    let clipped = ellipsize(cx, preset_label(agent.preset), input.size.x - 30.0, 11.0);
    draw_text(
        cx,
        &clipped,
        11.0,
        theme.foreground,
        input.origin.x + 6.0,
        input.origin.y + 16.0,
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(input.origin.x + input.size.x - 18.0, input.origin.y + 6.0),
        12.0,
        theme.muted_foreground,
        1.5,
    );
}

pub fn paint_preset_menu(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    agent: &BuiltinAgentConfig,
    index: Option<usize>,
    card: Rect,
) {
    let target = match index {
        Some(index) => BuiltinAgentPresetMenuTarget::Agent(index),
        None => BuiltinAgentPresetMenuTarget::Draft,
    };
    if settings.builtin_preset_menu_open != Some(target) {
        return;
    }
    let menu = preset_menu_rect(card);
    cx.backend.fill_round_rect(menu, 6.0, theme.card);
    cx.backend.stroke_round_rect(menu, 6.0, theme.border, 1.0);
    for (i, preset) in BUILTIN_AGENT_PRESETS.iter().enumerate() {
        let item = preset_item_rect(card, i);
        let active = agent.preset == preset.key;
        if active {
            cx.backend.fill_round_rect(item, 5.0, theme.muted);
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(item.origin.x + 8.0, item.origin.y + 6.0),
                12.0,
                theme.foreground,
                1.7,
            );
        }
        draw_text(
            cx,
            preset.display_name,
            11.0,
            theme.foreground,
            item.origin.x + 28.0,
            item.origin.y + 16.0,
        );
    }
}

pub fn paint_key_glyph(cx: &mut PaintCx<'_>, theme: &Theme, avatar: Rect) {
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

pub fn provider_select_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(card.origin.x + 12.0 + FIELD_LABEL_W, card.origin.y + 48.0),
        size: Point2D::new(card.size.x - 24.0 - FIELD_LABEL_W, FIELD_H),
    }
}

pub fn kind_rect(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(card.origin.x + card.size.x - 172.0, card.origin.y + 10.0),
        size: Point2D::new(156.0, 24.0),
    }
}

pub fn preset_menu_height(settings: &AgentSettings, index: Option<usize>) -> f32 {
    let target = match index {
        Some(index) => BuiltinAgentPresetMenuTarget::Agent(index),
        None => BuiltinAgentPresetMenuTarget::Draft,
    };
    if settings.builtin_preset_menu_open == Some(target) {
        preset_menu_rect_from_y(0.0).size.y + 6.0
    } else {
        0.0
    }
}

pub fn preset_at(card: Rect, point: Point2D) -> Option<BuiltinAgentPresetKey> {
    BUILTIN_AGENT_PRESETS
        .iter()
        .enumerate()
        .find(|(index, _)| rect_contains(preset_item_rect(card, *index), point))
        .map(|(_, preset)| preset.key)
}

fn preset_label(key: BuiltinAgentPresetKey) -> &'static str {
    BUILTIN_AGENT_PRESETS
        .iter()
        .find(|preset| preset.key == key)
        .map(|preset| preset.display_name)
        .unwrap_or("Custom")
}

fn preset_menu_rect(card: Rect) -> Rect {
    let base = provider_select_rect(card);
    let menu = preset_menu_rect_from_y(base.origin.y + base.size.y + 4.0);
    Rect {
        origin: Point2D::new(base.origin.x, menu.origin.y),
        size: Point2D::new(base.size.x, menu.size.y),
    }
}

fn preset_menu_rect_from_y(y: f32) -> Rect {
    Rect {
        origin: Point2D::new(0.0, y),
        size: Point2D::new(
            0.0,
            PRESET_MENU_PAD * 2.0 + BUILTIN_AGENT_PRESETS.len() as f32 * PRESET_MENU_ITEM_H,
        ),
    }
}

fn preset_item_rect(card: Rect, index: usize) -> Rect {
    let menu = preset_menu_rect(card);
    Rect {
        origin: Point2D::new(
            menu.origin.x + PRESET_MENU_PAD,
            menu.origin.y + PRESET_MENU_PAD + index as f32 * PRESET_MENU_ITEM_H,
        ),
        size: Point2D::new(menu.size.x - PRESET_MENU_PAD * 2.0, PRESET_MENU_ITEM_H),
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
