//! System tab of the settings modal.
//!
//! Renders the auto-update preference row. The TS desktop settings
//! page keeps this tab compact and leaves release probe status out of
//! the modal chrome.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_switch::{
    paint_settings_switch, SETTINGS_SWITCH_H, SETTINGS_SWITCH_W,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::AgentSettings;
use op_editor_core::editor_ui_state::EditorUiState;

const TITLE_H: f32 = 36.0;
const CARD_H: f32 = 58.0;

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
            card.origin.x + card.size.x - 16.0 - SETTINGS_SWITCH_W,
            card.origin.y + (CARD_H - SETTINGS_SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SETTINGS_SWITCH_W, SETTINGS_SWITCH_H),
    }
}

pub fn hit_test(content: Rect, scrolled: Point2D) -> SystemHit {
    if (auto_update_switch_rect(content)).contains(scrolled) {
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

    paint_settings_switch(
        cx,
        theme,
        auto_update_switch_rect(content),
        settings.auto_update_enabled,
    );
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
