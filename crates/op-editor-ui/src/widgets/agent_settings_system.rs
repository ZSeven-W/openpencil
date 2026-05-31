//! System tab of the settings modal.
//!
//! Renders the auto-update preference row. The TS desktop settings
//! page keeps this tab compact and leaves release probe status out of
//! the modal chrome.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::AgentSettings;
use op_editor_core::editor_ui_state::EditorUiState;

const TITLE_H: f32 = 36.0;
const CARD_H: f32 = 58.0;
const SWITCH_W: f32 = 36.0;
const SWITCH_H: f32 = 20.0;
const SWITCH_KNOB: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHit {
    ToggleAutoUpdate,
    None,
}

pub(super) fn content_height() -> f32 {
    12.0 + TITLE_H + CARD_H + 24.0
}

fn auto_update_card_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + 12.0 + TITLE_H),
        size: Point2D::new(content.size.x, CARD_H),
    }
}

fn auto_update_switch_rect(content: Rect) -> Rect {
    let card = auto_update_card_rect(content);
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - SWITCH_W,
            card.origin.y + (CARD_H - SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SWITCH_W, SWITCH_H),
    }
}

pub fn hit_test(content: Rect, scrolled: Point2D) -> SystemHit {
    if rect_contains(auto_update_switch_rect(content), scrolled) {
        return SystemHit::ToggleAutoUpdate;
    }
    SystemHit::None
}

pub(super) fn paint_system_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    settings: &AgentSettings,
    ui: &EditorUiState,
    content: Rect,
) {
    let title = TextLayout::single_run(
        t_settings(ui, "settings.system.title"),
        "system-ui",
        15.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x, content.origin.y + 20.0),
    );

    let card = auto_update_card_rect(content);
    cx.backend.fill_round_rect(card, 10.0, theme.muted);
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);

    let label_layout = TextLayout::single_run(
        t_settings(ui, "agents.autoUpdate"),
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 24.0),
    );

    let desc_layout = TextLayout::single_run(
        t_settings(ui, "settings.autoUpdateDesc"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &desc_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 46.0),
    );

    paint_switch(
        cx,
        theme,
        auto_update_switch_rect(content),
        settings.auto_update_enabled,
    );
}

fn paint_switch(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, enabled: bool) {
    let track_color = if enabled {
        theme.primary
    } else {
        theme.background
    };
    cx.backend
        .fill_round_rect(rect, SWITCH_H / 2.0, track_color);
    if !enabled {
        cx.backend
            .stroke_round_rect(rect, SWITCH_H / 2.0, theme.border, 1.0);
    }
    let knob_x = if enabled {
        rect.origin.x + SWITCH_W - SWITCH_KNOB - 3.0
    } else {
        rect.origin.x + 3.0
    };
    let knob = Rect {
        origin: Point2D::new(knob_x, rect.origin.y + (SWITCH_H - SWITCH_KNOB) / 2.0),
        size: Point2D::new(SWITCH_KNOB, SWITCH_KNOB),
    };
    cx.backend.fill_oval(knob, theme.foreground);
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
