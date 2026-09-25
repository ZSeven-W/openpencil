//! Tablet chrome for the works reader: the thumbnail strip, and either
//! the landscape side panel (status, page, latest reply, actions) or the
//! portrait status line + bottom bar. The host blits real board rasters
//! over the strip's plates after this pass (`thumb_plate`).

use super::super::{thumb_plate, ReaderForm, ReaderLayout, WorksReader, THUMB_LABEL_H};
use super::{
    centered_label, fade, fit_text, inset, paint_actions, paint_arrows, paint_bar_background,
    paint_status_line, text_weighted, SANS,
};
use crate::widgets::home_surface::StudioPalette;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::ReaderHit;

pub(super) fn paint_tablet_chrome(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    paint_strip(reader, cx, layout, palette);
    if layout.form == ReaderForm::TabletLandscape {
        paint_side_panel(reader, cx, layout, palette);
        return;
    }
    if layout.bottom_bar.size.y <= 0.0 {
        // Portrait with the chat sheet up: the sheet covers the rest.
        return;
    }
    cx.backend.fill_rect(layout.status, palette.page);
    // The portrait status reads from the same column the buttons use.
    paint_status_line(reader, cx, layout, layout.continue_chat.origin.x, palette);
    paint_bar_background(cx, layout.bottom_bar, palette);
    paint_actions(reader, cx, layout, palette);
}

fn paint_strip(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    let Some(row) = layout.strip else {
        return;
    };
    cx.backend.fill_rect(row, palette.page);
    cx.backend.stroke_line(
        Point2D::new(row.origin.x, row.origin.y),
        Point2D::new(row.origin.x + row.size.x, row.origin.y),
        palette.line,
        1.0,
    );
    paint_arrows(reader, cx, layout, palette);
    let current = reader.current_index();
    for &(index, tile) in &layout.thumbs {
        let plate = thumb_plate(tile);
        let selected = index == current;
        if reader.state.reader_pressed == Some(ReaderHit::Page(index)) {
            cx.backend
                .fill_round_rect(inset(tile, -4.0), 10.0, palette.button_hover);
        }
        if selected {
            cx.backend
                .stroke_round_rect(inset(plate, -3.5), 8.0, palette.blue, 2.5);
        }
        // Placeholder plate — the host blits the board raster over it.
        cx.backend.fill_rect(plate, palette.preview);
        cx.backend.stroke_rect(plate, palette.line, 1.0);
        // Clear of the selection ring, which sits 3.5 px outside the plate.
        let label_rect = Rect::xywh(
            tile.origin.x,
            plate.origin.y + plate.size.y + 3.0,
            tile.size.x,
            THUMB_LABEL_H,
        );
        let (color, weight) = if selected {
            (palette.blue, 650)
        } else {
            (fade(palette.ink, 0.6), 500)
        };
        centered_label(
            cx,
            &(index + 1).to_string(),
            label_rect,
            12.0,
            color,
            weight,
        );
    }
}

fn paint_side_panel(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    layout: &ReaderLayout,
    palette: StudioPalette,
) {
    let Some(panel) = layout.side_panel else {
        return;
    };
    let locale = reader.ui.locale;
    cx.backend.fill_rect(panel, palette.topbar);
    cx.backend.stroke_line(
        Point2D::new(panel.origin.x, panel.origin.y),
        Point2D::new(panel.origin.x, panel.origin.y + panel.size.y),
        palette.topbar_line,
        1.0,
    );
    // The chat opened into the panel: it paints over this surface, and
    // nothing of the panel's own content may show through it.
    if super::super::reader_chat_open(reader.ui) {
        return;
    }
    // The status as a card: phase icon + line, Stop / Retry inside it.
    cx.backend
        .fill_round_rect(layout.status, 14.0, palette.page);
    cx.backend
        .stroke_round_rect(layout.status, 14.0, palette.line, 1.0);
    paint_status_line(reader, cx, layout, layout.status.origin.x + 14.0, palette);

    if let Some(info) = layout.page_info {
        let count = reader.boards.len();
        if count > 0 {
            let eyebrow = op_i18n::translate(locale, "reader.pageOf")
                .replace("{{page}}", &(reader.current_index() + 1).to_string())
                .replace("{{count}}", &count.to_string());
            let eyebrow = fit_text(cx, &eyebrow, 12.0, info.size.x);
            text_weighted(
                cx,
                &eyebrow,
                Point2D::new(info.origin.x, info.origin.y + 16.0),
                12.0,
                palette.muted,
                600,
            );
        }
        let name = reader.current_name.as_deref().unwrap_or(&reader.title);
        let name = fit_text(cx, name, 18.0, info.size.x);
        text_weighted(
            cx,
            &name,
            Point2D::new(info.origin.x, info.origin.y + 44.0),
            18.0,
            palette.ink,
            680,
        );
    }
    if let Some(card) = layout.reply {
        paint_reply_card(reader, cx, card, palette);
    }
    paint_actions(reader, cx, layout, palette);
}

/// The latest AI reply (or, before there is one, how to use the panel).
fn paint_reply_card(
    reader: &WorksReader<'_>,
    cx: &mut PaintCx<'_>,
    card: Rect,
    palette: StudioPalette,
) {
    let locale = reader.ui.locale;
    let pad = 16.0;
    let inner_w = (card.size.x - pad * 2.0).max(0.0);
    let (eyebrow, body, color) = match reader.last_reply.as_deref() {
        Some(reply) => (
            op_i18n::translate(locale, "reader.latestReply"),
            reply,
            fade(palette.ink, 0.85),
        ),
        None => (
            op_i18n::translate(locale, "reader.continueChat"),
            op_i18n::translate(locale, "reader.panelHint"),
            palette.muted,
        ),
    };
    let line_h = 22.0;
    let top = card.origin.y + pad + 22.0;
    let max_lines = (((card.origin.y + card.size.y - pad) - top) / line_h).floor() as usize;
    let lines = wrap_lines(cx, body, 14.0, inner_w, max_lines);
    // The card hugs its text; the layout rect is only the room it may use.
    let hug_h = (top - card.origin.y) + line_h * lines.len() as f32 + pad;
    let card = Rect::xywh(
        card.origin.x,
        card.origin.y,
        card.size.x,
        hug_h.min(card.size.y),
    );
    cx.backend.fill_round_rect(card, 14.0, palette.page);
    cx.backend.stroke_round_rect(card, 14.0, palette.line, 1.0);
    text_weighted(
        cx,
        eyebrow,
        Point2D::new(card.origin.x + pad, card.origin.y + pad + 11.0),
        11.0,
        palette.eyebrow,
        650,
    );
    for (row, line) in lines.iter().enumerate() {
        text_weighted(
            cx,
            line,
            Point2D::new(card.origin.x + pad, top + line_h * row as f32 + 16.0),
            14.0,
            color,
            450,
        );
    }
}

/// Greedy wrap for mixed CJK / Latin text: CJK breaks between any two
/// characters, Latin prefers the last space. Stops at `max_lines`,
/// ending the last kept line with an ellipsis when text was cut.
fn wrap_lines(
    cx: &mut PaintCx<'_>,
    text: &str,
    size: f32,
    max_w: f32,
    max_lines: usize,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cut = false;
    'paragraphs: for paragraph in text.split('\n') {
        let mut line = String::new();
        for ch in paragraph.chars() {
            let mut candidate = line.clone();
            candidate.push(ch);
            if line.is_empty() || cx.backend.measure_text_family(&candidate, size, SANS) <= max_w {
                line = candidate;
                continue;
            }
            if lines.len() + 1 >= max_lines {
                lines.push(line);
                cut = true;
                break 'paragraphs;
            }
            // Latin: carry the unfinished word to the next line.
            let carry = match line.rfind(' ') {
                Some(space) if ch != ' ' && ch.is_ascii_alphanumeric() => {
                    let rest = line[space + 1..].to_string();
                    line.truncate(space);
                    rest
                }
                _ => String::new(),
            };
            lines.push(std::mem::take(&mut line));
            line = carry;
            if ch != ' ' {
                line.push(ch);
            }
        }
        if lines.len() >= max_lines {
            cut = true;
            break;
        }
        lines.push(line);
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        cut = true;
    }
    if cut {
        if let Some(last) = lines.last_mut() {
            *last = fit_text(cx, &format!("{last}…"), size, max_w);
        }
    }
    lines
}
