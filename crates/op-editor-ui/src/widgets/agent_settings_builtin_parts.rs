//! Shared paint and hit helpers for built-in provider forms.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::settings_form::ellipsize;
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
const TOUCH_FIELD_LABEL_W: f32 = 84.0;
const TOUCH_FIELD_H: f32 = 44.0;
const TOUCH_PRESET_MENU_ITEM_H: f32 = 44.0;
const TOUCH_PRESET_MENU_PAD: f32 = 6.0;
const TOUCH_PRESET_MENU_MAX_VISIBLE_ITEMS: usize = 6;
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
    api_format_label: &str,
    touch: bool,
) {
    let options = kind_options(agent);
    if touch {
        paint_touch_field_label(cx, theme, card, api_format_label, kind_rect(card, true));
    }
    if touch && options.len() == 1 {
        let rect = kind_rect(card, true);
        cx.backend.fill_round_rect(rect, 10.0, theme.card);
        cx.backend.stroke_round_rect(rect, 10.0, theme.border, 1.0);
        draw_text(
            cx,
            options[0].label,
            15.0,
            theme.foreground,
            rect.origin.x + 12.0,
            jian_widgets::centered_text_baseline_y(rect, 15.0),
        );
        return;
    }
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
        font_size: if touch { 14.0 } else { 10.0 },
    }
    .paint(
        cx.backend,
        kind_rect(card, touch),
        &crate::widgets::button::tokens_from_theme(theme),
    );
}

pub fn kind_toggle_target(
    agent: &BuiltinAgentConfig,
    card: Rect,
    point: Point2D,
    touch: bool,
) -> Option<BuiltinAgentKind> {
    let options = kind_options(agent);
    let idx = jian_widgets::components::toggle_group::ToggleGroup::segment_at(
        kind_rect(card, touch),
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
    provider_label: &str,
    touch: bool,
) {
    let input = provider_select_rect(card, touch);
    let label_size = if touch { 14.0 } else { 11.0 };
    let label_y = if touch {
        jian_widgets::centered_text_baseline_y(input, label_size)
    } else {
        input.origin.y + 16.0
    };
    let provider_label = if touch {
        ellipsize(cx, provider_label, TOUCH_FIELD_LABEL_W - 8.0, label_size)
    } else {
        provider_label.to_string()
    };
    draw_text(
        cx,
        &provider_label,
        label_size,
        theme.muted_foreground,
        card.origin.x + if touch { 16.0 } else { 12.0 },
        label_y,
    );
    jian_widgets::components::select_trigger::SelectTrigger {
        icon_paths: None,
        label: preset_label(agent.preset),
        placeholder: "",
        hovered: false,
        pressed: false,
        enabled: true,
        font_size: if touch { 15.0 } else { 11.0 },
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
    touch: bool,
) {
    let target = match index {
        Some(index) => BuiltinAgentPresetMenuTarget::Agent(index),
        None => BuiltinAgentPresetMenuTarget::Draft,
    };
    if settings.builtin_preset_menu_open != Some(target) {
        return;
    }
    let menu = preset_menu_rect(card, touch);
    cx.backend
        .fill_round_rect(menu, if touch { 10.0 } else { 6.0 }, theme.card);
    cx.backend
        .stroke_round_rect(menu, if touch { 10.0 } else { 6.0 }, theme.border, 1.0);
    cx.backend.save();
    cx.backend.clip_rect(menu);
    cx.backend.translate(Point2D::new(
        0.0,
        -settings.builtin_preset_menu_scroll.offset,
    ));
    for (i, preset) in BUILTIN_AGENT_PRESETS.iter().enumerate() {
        let item = preset_item_rect(card, i, touch);
        let active = agent.preset == preset.key;
        let hovered = settings.builtin_preset_menu_hover == Some(preset.key);
        if active {
            cx.backend
                .fill_round_rect(item, if touch { 8.0 } else { 5.0 }, theme.muted);
        } else if hovered {
            cx.backend
                .fill_round_rect(item, if touch { 8.0 } else { 5.0 }, theme.button_hover);
        }
        if active {
            let icon_size = if touch { 18.0 } else { 12.0 };
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(
                    item.origin.x + if touch { 12.0 } else { 8.0 },
                    item.origin.y + (item.size.y - icon_size) / 2.0,
                ),
                icon_size,
                theme.foreground,
                1.7,
            );
        }
        let font_size = if touch { 15.0 } else { 11.0 };
        draw_text(
            cx,
            preset.display_name,
            font_size,
            theme.foreground,
            item.origin.x + if touch { 44.0 } else { 28.0 },
            if touch {
                jian_widgets::centered_text_baseline_y(item, font_size)
            } else {
                item.origin.y + 16.0
            },
        );
    }
    cx.backend.restore();
    paint_menu_scrollbar(cx, theme, menu, settings.builtin_preset_menu_scroll, touch);
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

pub fn provider_select_rect(card: Rect, touch: bool) -> Rect {
    let (pad_x, label_w, y, height) = if touch {
        (16.0, TOUCH_FIELD_LABEL_W, 108.0, TOUCH_FIELD_H)
    } else {
        (12.0, FIELD_LABEL_W, 48.0, FIELD_H)
    };
    Rect {
        origin: Point2D::new(card.origin.x + pad_x + label_w, card.origin.y + y),
        size: Point2D::new(card.size.x - pad_x * 2.0 - label_w, height),
    }
}

pub fn kind_rect(card: Rect, touch: bool) -> Rect {
    let (right, top, width, height) = if touch {
        (16.0, 56.0, card.size.x - 32.0 - TOUCH_FIELD_LABEL_W, 44.0)
    } else {
        (16.0, 10.0, 156.0, 24.0)
    };
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - right - width,
            card.origin.y + top,
        ),
        size: Point2D::new(width, height),
    }
}

fn paint_touch_field_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    card: Rect,
    label: &str,
    input: Rect,
) {
    let font_size = 14.0;
    let label_x = card.origin.x + 16.0;
    let shown = ellipsize(
        cx,
        label,
        (input.origin.x - label_x - 8.0).max(0.0),
        font_size,
    );
    draw_text(
        cx,
        &shown,
        font_size,
        theme.muted_foreground,
        label_x,
        jian_widgets::centered_text_baseline_y(input, font_size),
    );
}

pub fn preset_menu_height(settings: &AgentSettings, index: Option<usize>, touch: bool) -> f32 {
    let target = match index {
        Some(index) => BuiltinAgentPresetMenuTarget::Agent(index),
        None => BuiltinAgentPresetMenuTarget::Draft,
    };
    if settings.builtin_preset_menu_open == Some(target) {
        preset_menu_view_height(touch) + if touch { 8.0 } else { 6.0 }
    } else {
        0.0
    }
}

pub fn preset_at(
    card: Rect,
    point: Point2D,
    scroll: f32,
    touch: bool,
) -> Option<BuiltinAgentPresetKey> {
    let index = preset_index_at(card, point, scroll, touch)?;
    BUILTIN_AGENT_PRESETS.get(index).map(|preset| preset.key)
}

pub fn preset_hover_at(
    card: Rect,
    point: Point2D,
    scroll: f32,
    touch: bool,
) -> Option<BuiltinAgentPresetKey> {
    preset_at(card, point, scroll, touch)
}

pub fn preset_scroll_max(touch: bool) -> f32 {
    (preset_content_height(touch) - preset_menu_view_height(touch)).max(0.0)
}

pub fn preset_menu_contains(card: Rect, point: Point2D, touch: bool) -> bool {
    (preset_menu_rect(card, touch)).contains(point)
}

fn preset_label(key: BuiltinAgentPresetKey) -> &'static str {
    BUILTIN_AGENT_PRESETS
        .iter()
        .find(|preset| preset.key == key)
        .map(|preset| preset.display_name)
        .unwrap_or("Custom")
}

pub fn preset_menu_rect(card: Rect, touch: bool) -> Rect {
    let base = provider_select_rect(card, touch);
    let menu = preset_menu_rect_from_y(
        base.origin.y + base.size.y + if touch { 8.0 } else { 4.0 },
        touch,
    );
    Rect {
        origin: Point2D::new(base.origin.x, menu.origin.y),
        size: Point2D::new(base.size.x, menu.size.y),
    }
}

fn preset_menu_rect_from_y(y: f32, touch: bool) -> Rect {
    Rect {
        origin: Point2D::new(0.0, y),
        size: Point2D::new(0.0, preset_menu_view_height(touch)),
    }
}

fn preset_item_rect(card: Rect, index: usize, touch: bool) -> Rect {
    let menu = preset_menu_rect(card, touch);
    let pad = if touch {
        TOUCH_PRESET_MENU_PAD
    } else {
        PRESET_MENU_PAD
    };
    let item_h = if touch {
        TOUCH_PRESET_MENU_ITEM_H
    } else {
        PRESET_MENU_ITEM_H
    };
    Rect {
        origin: Point2D::new(
            menu.origin.x + pad,
            menu.origin.y + pad + index as f32 * item_h,
        ),
        size: Point2D::new(menu.size.x - pad * 2.0, item_h),
    }
}

fn preset_index_at(card: Rect, point: Point2D, scroll: f32, touch: bool) -> Option<usize> {
    if !preset_menu_contains(card, point, touch) {
        return None;
    }
    let menu = preset_menu_rect(card, touch);
    let pad = if touch {
        TOUCH_PRESET_MENU_PAD
    } else {
        PRESET_MENU_PAD
    };
    let item_h = if touch {
        TOUCH_PRESET_MENU_ITEM_H
    } else {
        PRESET_MENU_ITEM_H
    };
    let local_y = point.y - menu.origin.y - pad + scroll.clamp(0.0, preset_scroll_max(touch));
    if local_y < 0.0 {
        return None;
    }
    let index = (local_y / item_h).floor() as usize;
    let inside_row = local_y - index as f32 * item_h <= item_h;
    (inside_row && index < BUILTIN_AGENT_PRESETS.len()).then_some(index)
}

fn preset_content_height(touch: bool) -> f32 {
    if touch {
        TOUCH_PRESET_MENU_PAD * 2.0 + BUILTIN_AGENT_PRESETS.len() as f32 * TOUCH_PRESET_MENU_ITEM_H
    } else {
        PRESET_MENU_PAD * 2.0 + BUILTIN_AGENT_PRESETS.len() as f32 * PRESET_MENU_ITEM_H
    }
}

fn preset_menu_view_height(touch: bool) -> f32 {
    let max_h = if touch {
        TOUCH_PRESET_MENU_PAD * 2.0
            + TOUCH_PRESET_MENU_MAX_VISIBLE_ITEMS as f32 * TOUCH_PRESET_MENU_ITEM_H
    } else {
        PRESET_MENU_PAD * 2.0 + PRESET_MENU_MAX_VISIBLE_ITEMS as f32 * PRESET_MENU_ITEM_H
    };
    preset_content_height(touch).min(max_h)
}

fn paint_menu_scrollbar(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    menu: Rect,
    scroll: jian_core::scroll::ScrollState,
    touch: bool,
) {
    let content_h = preset_content_height(touch);
    let track_h = (menu.size.y - 8.0).max(0.0);
    let Some(thumb_geom) = scroll.thumb(
        track_h,
        content_h,
        menu.size.y,
        if touch { 44.0 } else { 24.0 },
    ) else {
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

        let capped_h = preset_menu_height(&settings, None, false);

        assert!(
            capped_h < full_h,
            "preset dropdown should scroll instead of expanding to every option"
        );
        assert!(capped_h <= PRESET_MENU_PAD * 2.0 + PRESET_MENU_ITEM_H * 8.0 + 6.0);
        settings.builtin_preset_menu_open = Some(BuiltinAgentPresetMenuTarget::Agent(0));
        assert_eq!(preset_menu_height(&settings, None, false), 0.0);
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
            models: vec!["claude-sonnet-4-6-20250916".into()],
            base_url: "https://api.anthropic.com".into(),
            enabled: true,
        };
        let openai_half = Point2D::new(
            kind_rect(card, false).origin.x + 120.0,
            kind_rect(card, false).origin.y + 12.0,
        );
        assert_eq!(
            kind_toggle_target(&anthropic, card, openai_half, false),
            None
        );
        let touch_kind = kind_rect(card, true);
        assert_eq!(
            kind_toggle_target(
                &anthropic,
                card,
                Point2D::new(touch_kind.origin.x + 44.0, touch_kind.origin.y + 22.0),
                true,
            ),
            None,
            "a single supported API format must stay read-only on touch"
        );

        let openai = BuiltinAgentConfig {
            id: "openai".into(),
            preset: BuiltinAgentPresetKey::OpenAi,
            display_name: "OpenAI".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: String::new(),
            models: vec!["gpt-5.1".into()],
            base_url: "https://api.openai.com/v1".into(),
            enabled: true,
        };
        let anthropic_half = Point2D::new(
            kind_rect(card, false).origin.x + 30.0,
            kind_rect(card, false).origin.y + 12.0,
        );
        assert_eq!(
            kind_toggle_target(&openai, card, anthropic_half, false),
            None
        );

        let mut minimax = anthropic.clone();
        minimax.preset = BuiltinAgentPresetKey::MiniMax;
        minimax.display_name = "MiniMax".into();
        let target = kind_toggle_target(&minimax, card, openai_half, false);
        assert_eq!(target, Some(BuiltinAgentKind::OpenAiCompat));
    }
}
