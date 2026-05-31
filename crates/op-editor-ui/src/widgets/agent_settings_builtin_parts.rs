//! Shared paint and hit helpers for built-in provider forms.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::{
    AgentSettings, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetMenuTarget,
};
use op_editor_core::{builtin_agent_preset, BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS};

const FIELD_LABEL_W: f32 = 68.0;
const FIELD_H: f32 = 24.0;
const PRESET_MENU_ITEM_H: f32 = 24.0;
const PRESET_MENU_PAD: f32 = 4.0;
const PRESET_MENU_MAX_VISIBLE_ITEMS: usize = 8;
const ANTHROPIC_KIND_OPTION: [(&str, BuiltinAgentKind); 1] =
    [("Anthropic", BuiltinAgentKind::Anthropic)];
const OPENAI_KIND_OPTION: [(&str, BuiltinAgentKind); 1] =
    [("OpenAI", BuiltinAgentKind::OpenAiCompat)];
const BOTH_KIND_OPTIONS: [(&str, BuiltinAgentKind); 2] = [
    ("Anthropic", BuiltinAgentKind::Anthropic),
    ("OpenAI", BuiltinAgentKind::OpenAiCompat),
];

pub fn paint_kind_toggle(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    agent: &BuiltinAgentConfig,
    card: Rect,
) {
    let r = kind_rect(card);
    cx.backend.stroke_round_rect(r, 6.0, theme.border, 1.0);
    let options = kind_options(agent);
    for (i, (label, kind)) in options.iter().enumerate() {
        let item = kind_option_rect(card, i, options.len());
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

pub fn kind_toggle_target(
    agent: &BuiltinAgentConfig,
    card: Rect,
    point: Point2D,
) -> Option<BuiltinAgentKind> {
    if !rect_contains(kind_rect(card), point) {
        return None;
    }
    let options = kind_options(agent);
    options
        .iter()
        .enumerate()
        .find(|(index, _)| rect_contains(kind_option_rect(card, *index, options.len()), point))
        .map(|(_, (_, kind))| *kind)
        .filter(|kind| *kind != agent.kind)
}

fn kind_options(agent: &BuiltinAgentConfig) -> &'static [(&'static str, BuiltinAgentKind)] {
    let preset = builtin_agent_preset(agent.preset);
    if preset.alt_kind.is_some() {
        return &BOTH_KIND_OPTIONS;
    }
    match preset.kind {
        BuiltinAgentKind::Anthropic => &ANTHROPIC_KIND_OPTION,
        BuiltinAgentKind::OpenAiCompat => &OPENAI_KIND_OPTION,
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
    cx.backend.save();
    cx.backend.clip_rect(menu);
    cx.backend
        .translate(Point2D::new(0.0, -settings.builtin_preset_menu_scroll));
    for (i, preset) in BUILTIN_AGENT_PRESETS.iter().enumerate() {
        let item = preset_item_rect(card, i);
        let active = agent.preset == preset.key;
        let hovered = settings.builtin_preset_menu_hover == Some(preset.key);
        if active || hovered {
            cx.backend.fill_round_rect(item, 5.0, theme.muted);
        }
        if active {
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
    cx.backend.restore();
    paint_menu_scrollbar(cx, theme, menu, settings.builtin_preset_menu_scroll);
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

fn kind_option_rect(card: Rect, index: usize, count: usize) -> Rect {
    let r = kind_rect(card);
    let count = count.max(1) as f32;
    let w = r.size.x / count;
    Rect {
        origin: Point2D::new(r.origin.x + index as f32 * w, r.origin.y),
        size: Point2D::new(w, r.size.y),
    }
}

pub fn preset_menu_height(settings: &AgentSettings, index: Option<usize>) -> f32 {
    let target = match index {
        Some(index) => BuiltinAgentPresetMenuTarget::Agent(index),
        None => BuiltinAgentPresetMenuTarget::Draft,
    };
    if settings.builtin_preset_menu_open == Some(target) {
        preset_menu_view_height() + 6.0
    } else {
        0.0
    }
}

pub fn preset_at(card: Rect, point: Point2D, scroll: f32) -> Option<BuiltinAgentPresetKey> {
    let index = preset_index_at(card, point, scroll)?;
    BUILTIN_AGENT_PRESETS.get(index).map(|preset| preset.key)
}

pub fn preset_hover_at(card: Rect, point: Point2D, scroll: f32) -> Option<BuiltinAgentPresetKey> {
    preset_at(card, point, scroll)
}

pub fn preset_scroll_max() -> f32 {
    (preset_content_height() - preset_menu_view_height()).max(0.0)
}

pub fn preset_menu_contains(card: Rect, point: Point2D) -> bool {
    rect_contains(preset_menu_rect(card), point)
}

fn preset_label(key: BuiltinAgentPresetKey) -> &'static str {
    BUILTIN_AGENT_PRESETS
        .iter()
        .find(|preset| preset.key == key)
        .map(|preset| preset.display_name)
        .unwrap_or("Custom")
}

pub fn preset_menu_rect(card: Rect) -> Rect {
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
        size: Point2D::new(0.0, preset_menu_view_height()),
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

fn preset_index_at(card: Rect, point: Point2D, scroll: f32) -> Option<usize> {
    if !preset_menu_contains(card, point) {
        return None;
    }
    let menu = preset_menu_rect(card);
    let local_y =
        point.y - menu.origin.y - PRESET_MENU_PAD + scroll.clamp(0.0, preset_scroll_max());
    if local_y < 0.0 {
        return None;
    }
    let index = (local_y / PRESET_MENU_ITEM_H).floor() as usize;
    let inside_row = local_y - index as f32 * PRESET_MENU_ITEM_H <= PRESET_MENU_ITEM_H;
    (inside_row && index < BUILTIN_AGENT_PRESETS.len()).then_some(index)
}

fn preset_content_height() -> f32 {
    PRESET_MENU_PAD * 2.0 + BUILTIN_AGENT_PRESETS.len() as f32 * PRESET_MENU_ITEM_H
}

fn preset_menu_view_height() -> f32 {
    let max_h = PRESET_MENU_PAD * 2.0 + PRESET_MENU_MAX_VISIBLE_ITEMS as f32 * PRESET_MENU_ITEM_H;
    preset_content_height().min(max_h)
}

fn paint_menu_scrollbar(cx: &mut PaintCx<'_>, theme: &Theme, menu: Rect, scroll: f32) {
    let max = preset_scroll_max();
    if max <= 0.0 {
        return;
    }
    let content_h = preset_content_height();
    let thumb_h = (menu.size.y * menu.size.y / content_h).clamp(24.0, menu.size.y);
    let range = (menu.size.y - thumb_h).max(1.0);
    let t = (scroll.clamp(0.0, max) / max).clamp(0.0, 1.0);
    let thumb = Rect {
        origin: Point2D::new(
            menu.origin.x + menu.size.x - 5.0,
            menu.origin.y + 4.0 + t * range,
        ),
        size: Point2D::new(2.0, (thumb_h - 8.0).max(16.0)),
    };
    cx.backend.fill_round_rect(
        thumb,
        1.0,
        Color {
            a: 0.55,
            ..theme.muted_foreground
        },
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_menu_height_is_capped_to_visible_rows() {
        let mut settings = AgentSettings {
            builtin_preset_menu_open: Some(BuiltinAgentPresetMenuTarget::Draft),
            ..AgentSettings::default()
        };
        let full_h =
            PRESET_MENU_PAD * 2.0 + BUILTIN_AGENT_PRESETS.len() as f32 * PRESET_MENU_ITEM_H + 6.0;

        let capped_h = preset_menu_height(&settings, None);

        assert!(
            capped_h < full_h,
            "preset dropdown should scroll instead of expanding to every option"
        );
        assert!(capped_h <= PRESET_MENU_PAD * 2.0 + PRESET_MENU_ITEM_H * 8.0 + 6.0);
        settings.builtin_preset_menu_open = Some(BuiltinAgentPresetMenuTarget::Agent(0));
        assert_eq!(preset_menu_height(&settings, None), 0.0);
    }

    #[test]
    fn pure_provider_kind_toggle_hides_unsupported_format() {
        let card = Rect {
            origin: Point2D::new(20.0, 30.0),
            size: Point2D::new(500.0, 196.0),
        };
        let anthropic = BuiltinAgentConfig {
            id: "anthropic".into(),
            preset: BuiltinAgentPresetKey::Anthropic,
            display_name: "Anthropic".into(),
            kind: BuiltinAgentKind::Anthropic,
            api_key: String::new(),
            model: "claude-sonnet-4-6-20250916".into(),
            base_url: "https://api.anthropic.com".into(),
            enabled: true,
        };
        let openai_half = Point2D::new(
            kind_rect(card).origin.x + 120.0,
            kind_rect(card).origin.y + 12.0,
        );
        assert_eq!(kind_toggle_target(&anthropic, card, openai_half), None);

        let mut minimax = anthropic.clone();
        minimax.preset = BuiltinAgentPresetKey::MiniMax;
        minimax.display_name = "MiniMax".into();
        let target = kind_toggle_target(&minimax, card, openai_half);
        assert_eq!(target, Some(BuiltinAgentKind::OpenAiCompat));
    }
}
