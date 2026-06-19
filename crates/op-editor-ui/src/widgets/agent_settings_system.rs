//! System tab of the settings modal.
//!
//! Renders the auto-update preference row plus the experimental-features
//! opt-in (gates canvas Preview mode + the property-panel Widget section).

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::agent_settings_switch::{
    paint_settings_switch, SETTINGS_SWITCH_H, SETTINGS_SWITCH_W,
};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::AgentSettings;
use op_editor_core::editor_ui_state::EditorUiState;

const TITLE_H: f32 = 36.0;
const CARD_H: f32 = 58.0;
const CARD_GAP: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHit {
    ToggleAutoUpdate,
    ToggleExperimental,
    None,
}

pub(super) fn content_height() -> f32 {
    12.0 + TITLE_H + CARD_H + CARD_GAP + CARD_H + 24.0
}

fn auto_update_card_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + 12.0 + TITLE_H),
        size: Point2D::new(content.size.x, CARD_H),
    }
}

fn experimental_card_rect(content: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            content.origin.x,
            content.origin.y + 12.0 + TITLE_H + CARD_H + CARD_GAP,
        ),
        size: Point2D::new(content.size.x, CARD_H),
    }
}

/// The switch hit/paint rect for a given card — right-aligned, vertically
/// centred. Shared by paint + hit-test so the two can't drift.
fn switch_rect_for(card: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 16.0 - SETTINGS_SWITCH_W,
            card.origin.y + (CARD_H - SETTINGS_SWITCH_H) / 2.0,
        ),
        size: Point2D::new(SETTINGS_SWITCH_W, SETTINGS_SWITCH_H),
    }
}

pub fn hit_test(content: Rect, scrolled: Point2D) -> SystemHit {
    if switch_rect_for(auto_update_card_rect(content)).contains(scrolled) {
        return SystemHit::ToggleAutoUpdate;
    }
    if switch_rect_for(experimental_card_rect(content)).contains(scrolled) {
        return SystemHit::ToggleExperimental;
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
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(content.origin.x, content.origin.y + 20.0),
    );

    paint_toggle_card(
        cx,
        theme,
        ui,
        auto_update_card_rect(content),
        "agents.autoUpdate",
        "settings.autoUpdateDesc",
        settings.auto_update_enabled,
    );
    paint_toggle_card(
        cx,
        theme,
        ui,
        experimental_card_rect(content),
        "settings.experimental",
        "settings.experimentalDesc",
        settings.experimental_features_enabled,
    );
}

/// Paint one labelled toggle card: rounded background, title row, muted
/// description row, and a right-aligned switch reflecting `enabled`.
fn paint_toggle_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    card: Rect,
    label_key: &'static str,
    desc_key: &'static str,
    enabled: bool,
) {
    cx.backend.fill_round_rect(card, 10.0, theme.muted);
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);

    let label_layout = TextLayout::single_run(
        t_settings(ui, label_key),
        "system-ui",
        13.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 24.0),
    );

    let desc_layout = TextLayout::single_run(
        t_settings(ui, desc_key),
        "system-ui",
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &desc_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 46.0),
    );

    paint_settings_switch(cx, theme, switch_rect_for(card), enabled);
}
