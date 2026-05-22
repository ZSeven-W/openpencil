//! System tab of the settings modal.
//!
//! Renders the auto-update status card. The desktop host runs a
//! background probe against the GitHub releases API and writes the
//! outcome into `EditorUiState::update_status`; this tab paints a
//! status dot + label + description from it. The card stays
//! read-only — a manual re-check is offered through the native
//! "Check for Updates" menu item, so no clickable hit is needed
//! here (`SystemHit::None`).

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::agent_settings::AgentSettings;
use op_editor_core::editor_ui_state::{EditorUiState, UpdateStatus};

const TITLE_H: f32 = 36.0;
const CARD_H: f32 = 96.0;
const STATUS_DOT_RADIUS: f32 = 5.0;

/// Status-dot palette. The theme exposes no status tokens, so these
/// mirror the TS app's Tailwind hues directly.
const GREEN: Color = Color { r: 0.22, g: 0.78, b: 0.42, a: 1.0 };
const AMBER: Color = Color { r: 0.96, g: 0.62, b: 0.04, a: 1.0 };
const BLUE: Color = Color { r: 0.23, g: 0.51, b: 0.96, a: 1.0 };
const RED: Color = Color { r: 0.94, g: 0.27, b: 0.27, a: 1.0 };
const GREY: Color = Color { r: 0.55, g: 0.55, b: 0.58, a: 1.0 };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHit {
    None,
}

pub(super) fn content_height() -> f32 {
    12.0 + TITLE_H + CARD_H + 24.0
}

pub fn hit_test(_content: Rect, _scrolled: Point2D) -> SystemHit {
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
    _settings: &AgentSettings,
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

    let card = Rect {
        origin: Point2D::new(content.origin.x, content.origin.y + 12.0 + TITLE_H),
        size: Point2D::new(content.size.x, CARD_H),
    };
    cx.backend.fill_round_rect(card, 10.0, theme.muted);
    cx.backend.stroke_round_rect(card, 10.0, theme.border, 1.0);

    let (dot_color, status_key, desc_key) = status_view(&ui.update_status);

    // Status dot — coloured per the probe outcome.
    let dot_x = card.origin.x + 20.0;
    let dot_y = card.origin.y + 28.0;
    let dot_rect = Rect {
        origin: Point2D::new(dot_x - STATUS_DOT_RADIUS, dot_y - STATUS_DOT_RADIUS),
        size: Point2D::new(STATUS_DOT_RADIUS * 2.0, STATUS_DOT_RADIUS * 2.0),
    };
    cx.backend.fill_oval(dot_rect, dot_color);

    // "Auto-update" header.
    let label_layout = TextLayout::single_run(
        t_settings(ui, "settings.system.autoUpdate"),
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(card.origin.x + 38.0, card.origin.y + 24.0),
    );

    // Right-side status text. `Available` formats the version in;
    // every other state uses a static i18n string.
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
    // Float the status to a fixed column past the label so the card
    // layout stays predictable without per-locale text measurement.
    cx.backend.draw_text(
        &status_layout,
        Point2D::new(card.origin.x + card.size.x - 160.0, card.origin.y + 24.0),
    );

    // Description line.
    let desc_layout = TextLayout::single_run(
        t_settings(ui, desc_key),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &desc_layout,
        Point2D::new(card.origin.x + 38.0, card.origin.y + 46.0),
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
        Point2D::new(card.origin.x + 38.0, card.origin.y + 68.0),
    );
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
