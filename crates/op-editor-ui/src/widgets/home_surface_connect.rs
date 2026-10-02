//! The Home composer's 接入卡 — the modal card that offers the three
//! first-run ways to connect a model (free tier, own API key, local
//! CLI) when no chat agent can answer yet.

use super::{fade, HomeLayout, HomeSurface, StudioPalette};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::HomeHit;

/// Card size — a studio panel card, 440 wide with room for the title
/// and three 56 px rows.
pub const CONNECT_CARD_W: f32 = 440.0;
pub const CONNECT_CARD_H: f32 = 320.0;
/// Row height / gap inside the card.
pub const CONNECT_ROW_H: f32 = 56.0;
pub const CONNECT_ROW_GAP: f32 = 10.0;
/// Inset from the card's left/right edge to the rows.
const CONNECT_ROW_INSET_X: f32 = 16.0;
/// Title block height above the first row.
const CONNECT_TITLE_H: f32 = 104.0;

/// The card rect centred over `composer` plus its three action rows.
/// The card is never wider than the composer it covers: at a fixed 440 it
/// ran off both edges of a 390 pt phone.
pub(super) fn connect_card_rects(composer: Rect) -> (Rect, [Rect; 3]) {
    let width = CONNECT_CARD_W.min(composer.size.x);
    let card = Rect::xywh(
        composer.origin.x + (composer.size.x - width) / 2.0,
        composer.origin.y + (composer.size.y - CONNECT_CARD_H) / 2.0,
        width,
        CONNECT_CARD_H,
    );
    let rows = [0, 1, 2].map(|index| {
        Rect::xywh(
            card.origin.x + CONNECT_ROW_INSET_X,
            card.origin.y + CONNECT_TITLE_H + index as f32 * (CONNECT_ROW_H + CONNECT_ROW_GAP),
            width - CONNECT_ROW_INSET_X * 2.0,
            CONNECT_ROW_H,
        )
    });
    (card, rows)
}

/// Offer only paths this host can actually complete. An account alone does
/// not imply hosted AI quota, and a browser/phone cannot run local CLIs.
pub(super) fn adapt_connect_card(layout: &mut HomeLayout, ui: &op_editor_core::EditorUiState) {
    let available = [
        ui.account_ui_available && ui.agent_settings.web_served_models,
        true,
        ui.external_cli_available && !ui.touch_chrome(),
    ];
    let count = available.iter().filter(|&&on| on).count();
    let height = CONNECT_TITLE_H
        + count as f32 * CONNECT_ROW_H
        + (count.saturating_sub(1)) as f32 * CONNECT_ROW_GAP
        + 16.0;
    let center = layout.connect_card.origin.y + layout.connect_card.size.y / 2.0;
    layout.connect_card.size.y = height;
    layout.connect_card.origin.y = center - height / 2.0;
    let mut y = layout.connect_card.origin.y + CONNECT_TITLE_H;
    for (index, row) in layout.connect_rows.iter_mut().enumerate() {
        if !available[index] {
            *row = Rect::ZERO;
            continue;
        }
        *row = Rect::xywh(
            layout.connect_card.origin.x + CONNECT_ROW_INSET_X,
            y,
            layout.connect_card.size.x - CONNECT_ROW_INSET_X * 2.0,
            CONNECT_ROW_H,
        );
        y += CONNECT_ROW_H + CONNECT_ROW_GAP;
    }
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
            false,
        ),
        (
            HomeHit::ConnectApiKey,
            "home.connect.apiKey",
            "home.connect.apiKeyNote",
            true,
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
    let guidance = op_i18n::translate(surface.ui.locale, "home.connect.guide");
    let lines = super::paint::explore_copy::fit_lines(guidance, card.size.x - 40.0, 2, |s| {
        cx.backend.measure_text_family(s, 12.0, "system-ui")
    });
    for (index, line) in lines.iter().enumerate() {
        cx.backend.draw_text(
            &TextLayout::single_run(
                line,
                "system-ui",
                12.0,
                palette.sub.to_jian(),
                Point2D::ZERO,
            ),
            Point2D::new(
                card.origin.x + 20.0,
                card.origin.y + 59.0 + index as f32 * 17.0,
            ),
        );
    }
    for (index, (hit, title_key, note_key, primary)) in specs.into_iter().enumerate() {
        let row = rows[index];
        if row == Rect::ZERO {
            continue;
        }
        let hovered = surface.state.hover == Some(hit);
        let pressed = surface.state.pressed == Some(hit);
        let (fill, title_color, note_color, border) = if primary {
            (
                palette.blue,
                Color::WHITE,
                fade(Color::WHITE, 0.88),
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
        let title = crate::util::ellipsize_to_width(
            op_i18n::translate(surface.ui.locale, title_key),
            row.size.x - 32.0,
            |s| {
                cx.backend
                    .measure_text_family_styled(s, 14.0, "system-ui", 500, false)
            },
        );
        let note = crate::util::ellipsize_to_width(
            op_i18n::translate(surface.ui.locale, note_key),
            row.size.x - 32.0,
            |s| cx.backend.measure_text_family(s, 12.0, "system-ui"),
        );
        let title_layout = TextLayout::single_run(
            &title,
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
            &note,
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
