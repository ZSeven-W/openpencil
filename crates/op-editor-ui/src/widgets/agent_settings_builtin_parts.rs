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
struct KindOption {
    label: &'static str,
    kind: BuiltinAgentKind,
}

const ANTHROPIC_KIND_OPTION: [KindOption; 1] = [KindOption {
    label: "Anthropic",
    kind: BuiltinAgentKind::Anthropic,
}];
const OPENAI_KIND_OPTION: [KindOption; 1] = [KindOption {
    label: "OpenAI",
    kind: BuiltinAgentKind::OpenAiCompat,
}];
const BOTH_KIND_OPTIONS: [KindOption; 2] = [
    KindOption {
        label: "Anthropic",
        kind: BuiltinAgentKind::Anthropic,
    },
    KindOption {
        label: "OpenAI",
        kind: BuiltinAgentKind::OpenAiCompat,
    },
];

pub fn paint_kind_toggle(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    agent: &BuiltinAgentConfig,
    card: Rect,
) {
    let options = kind_options(agent);
    let labels: Vec<&str> = options.iter().map(|o| o.label).collect();
    let active = options
        .iter()
        .position(|o| o.kind == agent.kind)
        .unwrap_or(0);
    jian_widgets::components::toggle_group::ToggleGroup {
        options: &labels,
        icons: None,
        active,
        hover: None,
        font_size: 10.0,
    }
    .paint(
        cx.backend,
        kind_rect(card),
        &crate::widgets::button::tokens_from_theme(theme),
    );
}

pub fn kind_toggle_target(
    agent: &BuiltinAgentConfig,
    card: Rect,
    point: Point2D,
) -> Option<BuiltinAgentKind> {
    let options = kind_options(agent);
    let idx = jian_widgets::components::toggle_group::ToggleGroup::segment_at(
        kind_rect(card),
        options.len(),
        point,
    )?;
    options
        .get(idx)
        .map(|option| option.kind)
        .filter(|kind| *kind != agent.kind)
}

fn kind_options(agent: &BuiltinAgentConfig) -> &'static [KindOption] {
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
    jian_widgets::components::select_trigger::SelectTrigger {
        icon_paths: None,
        label: preset_label(agent.preset),
        placeholder: "",
        hovered: false,
        pressed: false,
        enabled: true,
        font_size: 11.0,
        bordered: true,
    }
    .paint(
        cx.backend,
        input,
        &crate::widgets::button::tokens_from_theme(theme),
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
    cx.backend.translate(Point2D::new(
        0.0,
        -settings.builtin_preset_menu_scroll.offset,
    ));
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
    (preset_menu_rect(card)).contains(point)
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

fn paint_menu_scrollbar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    menu: Rect,
    scroll: jian_core::scroll::ScrollState,
) {
    let content_h = preset_content_height();
    let track_h = (menu.size.y - 8.0).max(0.0);
    let Some(thumb_geom) = scroll.thumb(track_h, content_h, menu.size.y, 24.0) else {
        return;
    };
    let thumb = Rect {
        origin: Point2D::new(
            menu.origin.x + menu.size.x - 5.0,
            menu.origin.y + 4.0 + thumb_geom.offset,
        ),
        size: Point2D::new(2.0, thumb_geom.len),
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
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y));
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

        let openai = BuiltinAgentConfig {
            id: "openai".into(),
            preset: BuiltinAgentPresetKey::OpenAi,
            display_name: "OpenAI".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: String::new(),
            model: "gpt-5.1".into(),
            base_url: "https://api.openai.com/v1".into(),
            enabled: true,
        };
        let anthropic_half = Point2D::new(
            kind_rect(card).origin.x + 30.0,
            kind_rect(card).origin.y + 12.0,
        );
        assert_eq!(kind_toggle_target(&openai, card, anthropic_half), None);

        let mut minimax = anthropic.clone();
        minimax.preset = BuiltinAgentPresetKey::MiniMax;
        minimax.display_name = "MiniMax".into();
        let target = kind_toggle_target(&minimax, card, openai_half);
        assert_eq!(target, Some(BuiltinAgentKind::OpenAiCompat));
    }
}
