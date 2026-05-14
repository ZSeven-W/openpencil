//! System tab of the settings modal.
//!
//! The TS app's auto-update toggle routes through
//! `window.electronAPI.updater.{getAutoCheck, setAutoCheck}` but
//! the Rust shell has no updater backend yet. We render an
//! honest read-only status card: a green dot + "Up to date"
//! label + an explanatory line saying no update channel is
//! wired today. No togglable switch (flipping the boolean would
//! lie to the user); no real check happens. When the updater
//! lands, this can grow back into a real toggle + check-now
//! button + `ToggleAutoUpdate` hit.

use crate::document::{AgentSettings, Document};
use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

const TITLE_H: f32 = 36.0;
const CARD_H: f32 = 88.0;
const STATUS_DOT_RADIUS: f32 = 5.0;
/// Green used by the "Up to date" status dot. Distinct from
/// `theme.success` since the theme doesn't expose a status-success
/// token today; mirrors the TS app's green-500 hue.
const UP_TO_DATE_GREEN: Color = Color { r: 0.22, g: 0.78, b: 0.42, a: 1.0 };

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

pub(super) fn paint_system_tab(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    _settings: &AgentSettings,
    doc: &Document,
    content: Rect,
) {
    let title = TextLayout::single_run(
        t_settings(doc, "settings.system.title"),
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

    // Status dot — green when up to date. Honest indicator: no
    // real check happens (see module-level comment), but the
    // banner is informationally correct.
    let dot_x = card.origin.x + 20.0;
    let dot_y = card.origin.y + 28.0;
    let dot_rect = Rect {
        origin: Point2D::new(dot_x - STATUS_DOT_RADIUS, dot_y - STATUS_DOT_RADIUS),
        size: Point2D::new(STATUS_DOT_RADIUS * 2.0, STATUS_DOT_RADIUS * 2.0),
    };
    cx.backend
        .fill_oval(dot_rect, UP_TO_DATE_GREEN);
    // "Auto-update" header + "Up to date" status, side-by-side.
    let label_layout = TextLayout::single_run(
        t_settings(doc, "settings.system.autoUpdate"),
        "system-ui",
        13.0,
        to_jian(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(card.origin.x + 38.0, card.origin.y + 24.0),
    );
    let status_layout = TextLayout::single_run(
        t_settings(doc, "settings.system.upToDate"),
        "system-ui",
        12.0,
        to_jian(UP_TO_DATE_GREEN),
        Point2D::new(0.0, 0.0),
    );
    // Right-aligned-ish: float the status to a fixed column past
    // the label. Keeps the card layout predictable without per-
    // locale text-measurement.
    cx.backend.draw_text(
        &status_layout,
        Point2D::new(card.origin.x + card.size.x - 96.0, card.origin.y + 24.0),
    );
    let desc_layout = TextLayout::single_run(
        t_settings(doc, "settings.system.upToDateDescription"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &desc_layout,
        Point2D::new(card.origin.x + 38.0, card.origin.y + 46.0),
    );
    // Secondary descriptor — the original "not yet wired" message
    // stays as the explanatory subtitle so anyone wondering "why
    // no Check button?" sees the answer in-place.
    let sub_layout = TextLayout::single_run(
        t_settings(doc, "settings.system.autoUpdateUnavailable"),
        "system-ui",
        10.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &sub_layout,
        Point2D::new(card.origin.x + 38.0, card.origin.y + 66.0),
    );
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
