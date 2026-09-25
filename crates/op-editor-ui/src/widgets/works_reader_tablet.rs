//! Tablet geometry for the works reader.
//!
//! A tablet reads a work through the same `WorksReader` state, hit
//! targets and actions as the phone; only the composition differs:
//!
//! - **Landscape** — the board viewer on the left with a thumbnail strip
//!   under it, and a side panel on the right carrying the run status,
//!   the page on show, the latest AI reply and the 继续对话 / 改这一页
//!   actions. The chat opens INTO that panel (non-modal), so the board
//!   stays visible and pageable while the user talks about it.
//! - **Portrait** — the phone's vertical stack at tablet metrics: the
//!   stage, a thumbnail strip, the status line and a bottom bar whose
//!   buttons sit in a centred column. The chat opens as a floating
//!   bottom sheet; the stage shrinks above it (the strip, status and
//!   bar sit under the sheet and step aside), so the page being
//!   discussed stays in view.
//!
//! The stage always starts at `(0, READER_TABLET_HEADER_H)`, so the
//! canvas origin can be answered from the size class alone.

use super::{estimate_label_w, ReaderForm, ReaderLayout};
use crate::Rect;

/// Tablet header: 44 pt targets with 10 px breathing.
pub const READER_TABLET_HEADER_H: f32 = 64.0;
/// Thumbnail strip row under the stage.
pub const READER_STRIP_H: f32 = 112.0;
/// Status row (portrait).
pub const READER_TABLET_STATUS_H: f32 = 52.0;
/// Bottom bar (portrait): 56 px buttons with 18 px padding.
pub const READER_TABLET_BOTTOM_H: f32 = 92.0;

const TOUCH: f32 = 44.0;
const BUTTON_H: f32 = 56.0;
const BACK_X: f32 = 12.0;
const MODE_RIGHT_PAD: f32 = 16.0;
const MODE_SEG_MIN_W: f32 = 56.0;
const MODE_SEG_GAP: f32 = 2.0;
/// The centred column the portrait status + buttons live in.
const COLUMN_MAX_W: f32 = 680.0;
const COLUMN_MIN_PAD: f32 = 24.0;
const CONTINUE_FRACTION: f32 = 0.38;
const BUTTON_GAP: f32 = 12.0;
/// Side panel (landscape) width and inner padding.
const SIDE_W_WIDE: f32 = 380.0;
const SIDE_W: f32 = 340.0;
const SIDE_WIDE_FROM: f32 = 1200.0;
const SIDE_PAD: f32 = 20.0;
/// A strip tile: the board plate plus its number label under it.
pub const THUMB_TILE_H: f32 = 84.0;
/// The label band at the bottom of a tile (the plate is above it).
pub const THUMB_LABEL_H: f32 = 18.0;
const THUMB_MIN_W: f32 = TOUCH;
const THUMB_MAX_W: f32 = 160.0;
const THUMB_GAP: f32 = 12.0;
const STRIP_PAD_X: f32 = 12.0;
/// Floating chat sheet (portrait).
const SHEET_MAX_W: f32 = 900.0;
const SHEET_MARGIN: f32 = 12.0;
/// The sheet never gets shorter than this; a raised keyboard pushes its
/// top over the stage instead.
const SHEET_MIN_H: f32 = 280.0;
/// Share of the area under the header the portrait stage keeps while
/// the chat sheet is open.
const CHAT_STAGE_SHARE: f32 = 0.56;

/// The board plate inside a strip tile (the tile minus its label band).
/// The host blits the board raster into exactly this rect.
pub fn thumb_plate(tile: Rect) -> Rect {
    Rect::xywh(
        tile.origin.x,
        tile.origin.y,
        tile.size.x,
        (tile.size.y - THUMB_LABEL_H).max(0.0),
    )
}

/// Side panel width for a landscape tablet `viewport_w` wide.
pub fn side_panel_w(viewport_w: f32) -> f32 {
    if viewport_w >= SIDE_WIDE_FROM {
        SIDE_W_WIDE
    } else {
        SIDE_W
    }
}

/// The stage for a tablet form (see `reader_stage_rect_for`).
/// `chat_open`: the portrait chat sheet is up, and the stage keeps only
/// the band above it.
pub fn tablet_stage_rect(
    form: ReaderForm,
    viewport_w: f32,
    viewport_h: f32,
    paged: bool,
    chat_open: bool,
) -> Rect {
    let vw = viewport_w.max(1.0);
    let vh = viewport_h.max(1.0);
    let strip = if paged { READER_STRIP_H } else { 0.0 };
    let top = READER_TABLET_HEADER_H;
    if chat_open && form == ReaderForm::TabletPortrait {
        return Rect::xywh(0.0, top, vw, ((vh - top) * CHAT_STAGE_SHARE).floor());
    }
    match form {
        ReaderForm::TabletLandscape => Rect::xywh(
            0.0,
            top,
            (vw - side_panel_w(vw)).max(0.0),
            (vh - top - strip).max(0.0),
        ),
        _ => Rect::xywh(
            0.0,
            top,
            vw,
            (vh - top - strip - READER_TABLET_STATUS_H - READER_TABLET_BOTTOM_H).max(0.0),
        ),
    }
}

/// Where the chat opens over a tablet reader: the landscape side panel,
/// or the portrait floating bottom sheet. `visible_bottom` is the
/// viewport bottom minus any software keyboard.
pub fn tablet_chat_rect(
    form: ReaderForm,
    viewport_w: f32,
    viewport_h: f32,
    visible_bottom: f32,
) -> Rect {
    let vw = viewport_w.max(1.0);
    let bottom = visible_bottom.clamp(READER_TABLET_HEADER_H, viewport_h.max(1.0));
    if form == ReaderForm::TabletLandscape {
        let side_w = side_panel_w(vw);
        return Rect::xywh(
            vw - side_w,
            READER_TABLET_HEADER_H,
            side_w,
            (bottom - READER_TABLET_HEADER_H).max(0.0),
        );
    }
    let w = (vw - SHEET_MARGIN * 2.0).clamp(0.0, SHEET_MAX_W);
    let floor = bottom - SHEET_MARGIN;
    let stage = tablet_stage_rect(form, vw, viewport_h, false, true);
    let ceiling = READER_TABLET_HEADER_H + SHEET_MARGIN;
    let top = (stage.origin.y + stage.size.y + SHEET_MARGIN)
        .min(floor - SHEET_MIN_H)
        .max(ceiling);
    Rect::xywh((vw - w) / 2.0, top, w, (floor - top).max(0.0))
}

/// The strip's prev / next arrows and the window of tiles that fits
/// between them, centred on the current board.
struct Strip {
    prev: Rect,
    next: Rect,
    thumbs: Vec<(usize, Rect)>,
}

fn strip_layout(row: Rect, count: usize, current: usize, thumb_w: f32) -> Strip {
    let tile_y = row.origin.y + (row.size.y - THUMB_TILE_H) / 2.0;
    let arrow_y = row.origin.y + (row.size.y - TOUCH) / 2.0;
    let prev = Rect::xywh(row.origin.x + STRIP_PAD_X, arrow_y, TOUCH, TOUCH);
    let next = Rect::xywh(
        row.origin.x + row.size.x - STRIP_PAD_X - TOUCH,
        arrow_y,
        TOUCH,
        TOUCH,
    );
    let area_x = prev.origin.x + TOUCH + 8.0;
    let area_w = (next.origin.x - 8.0 - area_x).max(0.0);
    let w = thumb_w.clamp(THUMB_MIN_W, THUMB_MAX_W);
    let fits = (((area_w + THUMB_GAP) / (w + THUMB_GAP)).floor() as usize).max(1);
    let shown = count.min(fits);
    let first = current
        .saturating_sub(shown / 2)
        .min(count.saturating_sub(shown));
    let group_w = shown as f32 * w + shown.saturating_sub(1) as f32 * THUMB_GAP;
    let mut x = area_x + ((area_w - group_w) / 2.0).max(0.0);
    let thumbs = (first..first + shown)
        .map(|index| {
            let rect = Rect::xywh(x, tile_y, w, THUMB_TILE_H);
            x += w + THUMB_GAP;
            (index, rect)
        })
        .collect();
    Strip { prev, next, thumbs }
}

/// A tile's width for boards of `aspect` (width / height).
pub fn thumb_w_for(aspect: f32) -> f32 {
    let aspect = if aspect.is_finite() && aspect > 0.0 {
        aspect
    } else {
        16.0 / 9.0
    };
    ((THUMB_TILE_H - THUMB_LABEL_H) * aspect).clamp(THUMB_MIN_W, THUMB_MAX_W)
}

/// Labels and counts the tablet layout sizes against.
pub struct TabletInputs<'a> {
    pub paged: bool,
    /// The chat is open over the reader (side panel / portrait sheet).
    pub chat_open: bool,
    pub normal_label: &'a str,
    pub professional_label: &'a str,
    pub action_label: Option<&'a str>,
    pub board_count: usize,
    pub current: usize,
    pub thumb_w: f32,
}

/// Pure tablet layout (landscape or portrait).
pub fn tablet_reader_layout(
    form: ReaderForm,
    viewport_w: f32,
    viewport_h: f32,
    inputs: &TabletInputs<'_>,
) -> ReaderLayout {
    let vw = viewport_w.max(1.0);
    let vh = viewport_h.max(1.0);
    let landscape = form == ReaderForm::TabletLandscape;
    let header = Rect::xywh(0.0, 0.0, vw, READER_TABLET_HEADER_H);
    let top = (READER_TABLET_HEADER_H - TOUCH) / 2.0;
    let back = Rect::xywh(BACK_X, top, TOUCH, TOUCH);
    let seg_w = |label: &str| (estimate_label_w(label, 13.0) + 20.0).max(MODE_SEG_MIN_W);
    let normal_w = seg_w(inputs.normal_label);
    let professional_w = seg_w(inputs.professional_label);
    let switch_w = normal_w + MODE_SEG_GAP + professional_w;
    let mode_switch = Rect::xywh(vw - MODE_RIGHT_PAD - switch_w, top, switch_w, TOUCH);
    let mode_normal = Rect::xywh(mode_switch.origin.x, top, normal_w, TOUCH);
    let mode_professional = Rect::xywh(
        mode_switch.origin.x + normal_w + MODE_SEG_GAP,
        top,
        professional_w,
        TOUCH,
    );
    let title_x = back.origin.x + TOUCH + 8.0;
    let title = Rect::xywh(
        title_x,
        top,
        (mode_switch.origin.x - 12.0 - title_x).max(0.0),
        TOUCH,
    );

    let stage = tablet_stage_rect(form, vw, vh, inputs.paged, inputs.chat_open);
    // Portrait with the chat sheet up: the strip, status and bar are
    // under the sheet, so they neither paint nor answer presses.
    let covered = inputs.chat_open && !landscape;
    let strip = (inputs.paged && !covered).then(|| {
        Rect::xywh(
            0.0,
            stage.origin.y + stage.size.y,
            stage.size.x,
            READER_STRIP_H,
        )
    });
    let strip_parts =
        strip.map(|row| strip_layout(row, inputs.board_count, inputs.current, inputs.thumb_w));
    let action_w = |label: &str| (estimate_label_w(label, 14.0) + 32.0).max(84.0);

    let (side_panel, status, status_action, page_info, reply, bottom_bar, continue_chat, edit_page) =
        if covered {
            (
                None,
                Rect::ZERO,
                None,
                None,
                None,
                Rect::ZERO,
                Rect::ZERO,
                Rect::ZERO,
            )
        } else if landscape {
            let side_w = side_panel_w(vw);
            let panel = Rect::xywh(
                vw - side_w,
                READER_TABLET_HEADER_H,
                side_w,
                (vh - READER_TABLET_HEADER_H).max(0.0),
            );
            let px = panel.origin.x + SIDE_PAD;
            let pw = (side_w - SIDE_PAD * 2.0).max(0.0);
            let status = Rect::xywh(px, panel.origin.y + 16.0, pw, READER_TABLET_STATUS_H);
            let status_action = inputs.action_label.map(|label| {
                let w = action_w(label).min(pw * 0.5);
                Rect::xywh(px + pw - w, status.origin.y + 4.0, w, TOUCH)
            });
            let info = Rect::xywh(px, status.origin.y + status.size.y + 12.0, pw, 56.0);
            let edit_page = Rect::xywh(px, vh - SIDE_PAD - BUTTON_H, pw, BUTTON_H);
            let continue_chat =
                Rect::xywh(px, edit_page.origin.y - BUTTON_GAP - BUTTON_H, pw, BUTTON_H);
            let reply_top = info.origin.y + info.size.y + 16.0;
            let reply_h = continue_chat.origin.y - 20.0 - reply_top;
            let reply = (reply_h >= 96.0).then(|| Rect::xywh(px, reply_top, pw, reply_h));
            (
                Some(panel),
                status,
                status_action,
                Some(info),
                reply,
                Rect::ZERO,
                continue_chat,
                edit_page,
            )
        } else {
            let col_w = (vw - COLUMN_MIN_PAD * 2.0).clamp(0.0, COLUMN_MAX_W);
            let col_x = (vw - col_w) / 2.0;
            let bar = Rect::xywh(0.0, vh - READER_TABLET_BOTTOM_H, vw, READER_TABLET_BOTTOM_H);
            let status = Rect::xywh(
                0.0,
                bar.origin.y - READER_TABLET_STATUS_H,
                vw,
                READER_TABLET_STATUS_H,
            );
            let status_action = inputs.action_label.map(|label| {
                let w = action_w(label);
                Rect::xywh(
                    col_x + col_w - w,
                    status.origin.y + (READER_TABLET_STATUS_H - TOUCH) / 2.0,
                    w,
                    TOUCH,
                )
            });
            let inner_w = (col_w - BUTTON_GAP).max(0.0);
            let continue_w = (inner_w * CONTINUE_FRACTION).floor();
            let y = bar.origin.y + (READER_TABLET_BOTTOM_H - BUTTON_H) / 2.0;
            let continue_chat = Rect::xywh(col_x, y, continue_w, BUTTON_H);
            let edit_page = Rect::xywh(
                col_x + continue_w + BUTTON_GAP,
                y,
                (inner_w - continue_w).max(0.0),
                BUTTON_H,
            );
            (
                None,
                status,
                status_action,
                None,
                None,
                bar,
                continue_chat,
                edit_page,
            )
        };

    ReaderLayout {
        form,
        header,
        back,
        title,
        mode_switch,
        mode_normal,
        mode_professional,
        stage,
        pager: None,
        prev: strip_parts.as_ref().map(|parts| parts.prev),
        next: strip_parts.as_ref().map(|parts| parts.next),
        page_label: None,
        strip,
        thumbs: strip_parts.map(|parts| parts.thumbs).unwrap_or_default(),
        side_panel,
        page_info,
        reply,
        status,
        status_action,
        bottom_bar,
        continue_chat,
        edit_page,
    }
}

#[cfg(test)]
#[path = "works_reader_tablet_tests.rs"]
mod tests;
