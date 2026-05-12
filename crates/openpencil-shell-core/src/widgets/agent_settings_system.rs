//! System tab of the settings modal.
//!
//! No interactive controls today: the TS app's auto-update toggle
//! routes through `window.electronAPI.updater.{getAutoCheck,
//! setAutoCheck}`, but the Rust shell has no updater backend yet.
//! Surfacing a togglable switch here would lie to the user (flip
//! the boolean → nothing happens), so we render a read-only status
//! row instead. When the updater lands, this can grow back into a
//! real toggle + `ToggleAutoUpdate` hit.

use crate::document::{AgentSettings, Document};
use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

const TITLE_H: f32 = 36.0;
const CARD_H: f32 = 64.0;

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

    let label_layout = TextLayout::single_run(
        t_settings(doc, "settings.system.autoUpdate"),
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
        t_settings(doc, "settings.system.autoUpdateUnavailable"),
        "system-ui",
        11.0,
        to_jian(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &desc_layout,
        Point2D::new(card.origin.x + 16.0, card.origin.y + 44.0),
    );
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
