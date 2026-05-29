//! System tab of the settings modal.
//!
//! Renders the auto-update preference + status card. The desktop
//! host runs a background probe against the GitHub releases API when
//! auto-check is enabled and writes the outcome into
//! `EditorUiState::update_status`; this tab paints the preference
//! switch plus the latest status.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::AgentSettings;
use op_editor_core::editor_ui_state::{EditorUiState, UpdateStatus};

const TITLE_H: f32 = 36.0;
const CARD_H: f32 = 112.0;
const STATUS_DOT_RADIUS: f32 = 5.0;
const SWITCH_W: f32 = 36.0;
const SWITCH_H: f32 = 20.0;
const SWITCH_KNOB: f32 = 14.0;

/// Status-dot palette. The theme exposes no status tokens, so these
/// mirror the TS app's Tailwind hues directly.
const GREEN: Color = Color {
    r: 0.22,
    g: 0.78,
    b: 0.42,
    a: 1.0,
};
const AMBER: Color = Color {
    r: 0.96,
    g: 0.62,
    b: 0.04,
    a: 1.0,
};
const BLUE: Color = Color {
    r: 0.23,
    g: 0.51,
    b: 0.96,
    a: 1.0,
};
const RED: Color = Color {
    r: 0.94,
    g: 0.27,
    b: 0.27,
    a: 1.0,
};
const GREY: Color = Color {
    r: 0.55,
    g: 0.55,
    b: 0.58,
    a: 1.0,
};

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
            card.origin.y + 18.0,
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

/// Resolve the per-status presentation: dot colour, the right-side
/// status label key and the description key. `Available` is handled
/// by the caller (it needs the version string formatted in).
fn status_view(status: &UpdateStatus) -> (Color, &'static str, &'static str) {
    match status {
        UpdateStatus::Idle => (
            GREY,
            "settings.system.idle",
            "settings.system.idleDescription",
        ),
        UpdateStatus::Checking => (
            BLUE,
            "settings.system.checking",
            "settings.system.checkingDescription",
        ),
        UpdateStatus::UpToDate => (
            GREEN,
            "settings.system.upToDate",
            "settings.system.upToDateDescription",
        ),
        UpdateStatus::Available { .. } => (
            AMBER,
            "settings.system.updateAvailable",
            "settings.system.updateAvailableDescription",
        ),
        UpdateStatus::Error => (
            RED,
            "settings.system.errorStatus",
            "settings.system.errorDescription",
        ),
    }
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

    let (dot_color, status_key, desc_key) = status_view(&ui.update_status);

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

    let dot_y = card.origin.y + 70.0;
    let dot_rect = Rect {
        origin: Point2D::new(card.origin.x + 16.0, dot_y - STATUS_DOT_RADIUS),
        size: Point2D::new(STATUS_DOT_RADIUS * 2.0, STATUS_DOT_RADIUS * 2.0),
    };
    cx.backend.fill_oval(dot_rect, dot_color);

    let status_text: String = match &ui.update_status {
        UpdateStatus::Available { version } => {
            format!("{} v{}", t_settings(ui, status_key), version)
        }
        _ => t_settings(ui, status_key).to_string(),
    };
    let status_layout = TextLayout::single_run(
        &status_text,
        "system-ui",
        12.0,
        to_jian(dot_color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &status_layout,
        Point2D::new(card.origin.x + 32.0, card.origin.y + 74.0),
    );

    let status_desc_layout = TextLayout::single_run(
        t_settings(ui, desc_key),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &status_desc_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 92.0),
    );

    // Current build version — same value the probe compares against
    // the latest release tag.
    let version_text = format!(
        "{}: v{}",
        t_settings(ui, "settings.system.currentVersion"),
        env!("CARGO_PKG_VERSION"),
    );
    let version_layout = TextLayout::single_run(
        &version_text,
        "system-ui",
        10.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &version_layout,
        Point2D::new(card.origin.x + card.size.x - 120.0, card.origin.y + 92.0),
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
