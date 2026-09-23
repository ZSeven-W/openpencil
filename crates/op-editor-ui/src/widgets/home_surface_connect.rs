//! The Home composer's 接入卡 — the modal card that offers the three
//! first-run ways to connect a model (free tier, own API key, local
//! CLI) when no chat agent can answer yet.

use super::{fade, HomeLayout, HomeSurface, StudioPalette};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::HomeHit;

/// Card size — a studio panel card, 440 wide with room for the title
/// and three 56 px rows.
pub const CONNECT_CARD_W: f32 = 440.0;
pub const CONNECT_CARD_H: f32 = 280.0;
/// Row height / gap inside the card.
pub const CONNECT_ROW_H: f32 = 56.0;
pub const CONNECT_ROW_GAP: f32 = 10.0;
/// Inset from the card's left/right edge to the rows.
const CONNECT_ROW_INSET_X: f32 = 16.0;
/// Title block height above the first row.
const CONNECT_TITLE_H: f32 = 64.0;

/// The card rect centred over `composer` plus its three action rows.
pub(super) fn connect_card_rects(composer: Rect) -> (Rect, [Rect; 3]) {
    let card = Rect::xywh(
        composer.origin.x + (composer.size.x - CONNECT_CARD_W) / 2.0,
        composer.origin.y + (composer.size.y - CONNECT_CARD_H) / 2.0,
        CONNECT_CARD_W,
        CONNECT_CARD_H,
    );
    let rows = [0, 1, 2].map(|index| {
        Rect::xywh(
            card.origin.x + CONNECT_ROW_INSET_X,
            card.origin.y + CONNECT_TITLE_H + index as f32 * (CONNECT_ROW_H + CONNECT_ROW_GAP),
            CONNECT_CARD_W - CONNECT_ROW_INSET_X * 2.0,
            CONNECT_ROW_H,
        )
    });
    (card, rows)
}

/// Map a point to a connect-card hit. `None` outside the card; the
/// caller turns that into `ConnectClose`.
pub(super) fn connect_card_hit(layout: &HomeLayout, point: Point2D) -> Option<HomeHit> {
    if !layout.connect_card.contains(point) {
        return None;
    }
    let index = layout
        .connect_rows
        .iter()
        .position(|row| row.contains(point))?;
    Some(match index {
        0 => HomeHit::ConnectFreeTier,
        1 => HomeHit::ConnectApiKey,
        _ => HomeHit::ConnectCli,
    })
}

/// Paint the card: panel fill, hairline border, pool shadow, sans
/// title, then the three rows — the free tier in primary blue, the
/// other two as quiet outline rows.
pub(super) fn paint_connect_card(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    palette: StudioPalette,
) {
    let (card, rows) = (&layout.connect_card, &layout.connect_rows);
    let title = op_i18n::translate(surface.ui.locale, "home.connect.title");
    let specs = [
        (
            HomeHit::ConnectFreeTier,
            "home.connect.free",
            "home.connect.freeNote",
            true,
        ),
        (
            HomeHit::ConnectApiKey,
            "home.connect.apiKey",
            "home.connect.apiKeyNote",
            false,
        ),
        (
            HomeHit::ConnectCli,
            "home.connect.cli",
            "home.connect.cliNote",
            false,
        ),
    ];
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            card.origin.x + 6.0,
            card.origin.y + 10.0,
            card.size.x - 12.0,
            card.size.y - 4.0,
        ),
        14.0,
        16.0,
        fade(palette.ink, 0.18),
    );
    cx.backend.fill_round_rect(*card, 14.0, palette.raised);
    cx.backend
        .stroke_round_rect(*card, 14.0, palette.raised_line, 1.0);
    let title_layout = TextLayout::single_run(
        title,
        "system-ui",
        19.0,
        palette.ink.to_jian(),
        Point2D::new(0.0, 0.0),
    )
    .with_font_weight(650);
    cx.backend.draw_text(
        &title_layout,
        Point2D::new(card.origin.x + 20.0, card.origin.y + 36.0),
    );
    for (index, (hit, title_key, note_key, primary)) in specs.into_iter().enumerate() {
        let row = rows[index];
        let hovered = surface.state.hover == Some(hit);
        let pressed = surface.state.pressed == Some(hit);
        let (fill, title_color, note_color, border) = if primary {
            (
                palette.blue,
                palette.panel,
                fade(palette.panel, 0.82),
                palette.blue,
            )
        } else {
            (
                if pressed {
                    fade(palette.blue, 0.10)
                } else if hovered {
                    palette.button_hover
                } else {
                    palette.panel
                },
                palette.ink,
                palette.muted,
                palette.line,
            )
        };
        cx.backend.fill_round_rect(row, 10.0, fill);
        if !primary {
            cx.backend.stroke_round_rect(row, 10.0, border, 1.0);
        }
        let title = op_i18n::translate(surface.ui.locale, title_key);
        let note = op_i18n::translate(surface.ui.locale, note_key);
        let title_layout = TextLayout::single_run(
            title,
            "system-ui",
            14.0,
            title_color.to_jian(),
            Point2D::new(0.0, 0.0),
        )
        .with_font_weight(500);
        cx.backend.draw_text(
            &title_layout,
            Point2D::new(row.origin.x + 16.0, row.origin.y + 23.0),
        );
        let note_layout = TextLayout::single_run(
            note,
            "system-ui",
            12.0,
            note_color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &note_layout,
            Point2D::new(row.origin.x + 16.0, row.origin.y + 42.0),
        );
    }
}
