//! The touch-tablet adaptation of the wide Home.
//!
//! A tablet (touch chrome, Medium or Expanded size class) keeps the wide
//! composition — a 768–1366 pt screen has room for the task row, the
//! composer beside (or over) the example, and the explore cards, and a
//! stretched phone column would waste it. Three things change:
//!
//! - the page gets real side margins (the desktop shell's 8 px gutter
//!   reads as content glued to the bezel on a tablet);
//! - the one-line 最近项目 chip row (32 px chips, names cut at 9 chars)
//!   becomes a 作品 grid — the current work plus the recent documents
//!   as 44 pt+ cards, the tablet's works list;
//! - small desktop targets (38 px top-bar buttons, 28 px tool buttons,
//!   the 24 px example link) get a 44 pt touch slop in hit-testing.

use super::layout::HomeLayout;
use super::WORKS_RECENT_CAP;
use crate::{Point2D, Rect};
use op_editor_core::{EditorUiState, HomeHit};

/// Side padding of the tablet page column.
pub(crate) const TABLET_PAD_X: f32 = 28.0;
/// The 作品 heading row (heading + 新建空白画布 at 44 pt).
pub const WORKS_GRID_HEADING_H: f32 = 44.0;
/// One work card.
pub const WORKS_CARD_H: f32 = 96.0;
const HEADING_GAP: f32 = 12.0;
const CARD_GAP: f32 = 16.0;
const EMPTY_H: f32 = 48.0;
/// Space kept under the grid when the page scrolls to its end.
pub(crate) const PAGE_PAD_BOTTOM: f32 = 28.0;
const TOUCH: f32 = 44.0;

/// Whether the Home surface takes the tablet adaptation.
pub fn is_touch_tablet(ui: &EditorUiState) -> bool {
    ui.touch_chrome() && !ui.compact_layout()
}

/// The tablet 作品 grid.
#[derive(Debug, Clone, PartialEq)]
pub struct WorksGrid {
    pub heading: Rect,
    /// `(the press it answers, card rect)`: the current work first, then
    /// the recent documents in `recent_files` order.
    pub cards: Vec<(HomeHit, Rect)>,
    /// The empty-state note when there is nothing to list.
    pub empty: Option<Rect>,
}

impl WorksGrid {
    pub fn bottom(&self) -> f32 {
        let last = self
            .cards
            .last()
            .map(|(_, rect)| *rect)
            .or(self.empty)
            .unwrap_or(self.heading);
        last.origin.y + last.size.y
    }
}

/// Columns for a `content_w`-wide page.
fn columns(content_w: f32) -> usize {
    if content_w >= 1000.0 {
        4
    } else if content_w >= 640.0 {
        3
    } else {
        2
    }
}

/// Pure grid geometry from the section's top-left.
pub fn works_grid(
    x: f32,
    y: f32,
    content_w: f32,
    has_current: bool,
    recent_count: usize,
) -> WorksGrid {
    let heading = Rect::xywh(x, y, content_w, WORKS_GRID_HEADING_H);
    let top = y + WORKS_GRID_HEADING_H + HEADING_GAP;
    let cols = columns(content_w);
    let card_w = ((content_w - CARD_GAP * (cols - 1) as f32) / cols as f32).max(TOUCH);
    let hits = has_current
        .then_some(HomeHit::WorksCurrent)
        .into_iter()
        .chain((0..recent_count.min(WORKS_RECENT_CAP)).map(HomeHit::WorksRecent));
    let cards: Vec<(HomeHit, Rect)> = hits
        .enumerate()
        .map(|(slot, hit)| {
            let (row, col) = (slot / cols, slot % cols);
            (
                hit,
                Rect::xywh(
                    x + col as f32 * (card_w + CARD_GAP),
                    top + row as f32 * (WORKS_CARD_H + CARD_GAP),
                    card_w,
                    WORKS_CARD_H,
                ),
            )
        })
        .collect();
    let empty = cards
        .is_empty()
        .then(|| Rect::xywh(x, top, content_w, EMPTY_H));
    WorksGrid {
        heading,
        cards,
        empty,
    }
}

/// Swap the chip row for the grid: `layout.recent` becomes the grid's
/// band (so the scroll range reaches its last card), the chips go, and
/// 新建空白画布 moves into the heading row at 44 pt.
pub(super) fn adapt_layout(layout: &mut HomeLayout, has_current: bool, recent_count: usize) {
    let recent = layout.recent;
    let grid = works_grid(
        recent.origin.x,
        recent.origin.y,
        recent.size.x,
        has_current,
        recent_count,
    );
    layout.recent.size.y = grid.bottom() - recent.origin.y;
    layout.recent_chips = [Rect::ZERO; 5];
    let new_canvas_w = layout.new_canvas.size.x + 12.0;
    layout.new_canvas = Rect::xywh(
        recent.origin.x + recent.size.x - new_canvas_w,
        recent.origin.y,
        new_canvas_w,
        TOUCH,
    );
}

/// `rect`, grown about its centre to at least 44 × 44 — the hit slop
/// small desktop-sized targets get on a touch tablet.
pub fn touch_slop(rect: Rect) -> Rect {
    if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
        return rect;
    }
    let dx = (TOUCH - rect.size.x).max(0.0) / 2.0;
    let dy = (TOUCH - rect.size.y).max(0.0) / 2.0;
    Rect::xywh(
        rect.origin.x - dx,
        rect.origin.y - dy,
        rect.size.x + dx * 2.0,
        rect.size.y + dy * 2.0,
    )
}

/// Whether a tablet press at `point` lands on `rect` (with slop).
pub(super) fn hits(tablet: bool, rect: Rect, point: Point2D) -> bool {
    if tablet {
        touch_slop(rect).contains(point)
    } else {
        rect.contains(point)
    }
}

#[cfg(test)]
#[path = "home_surface_tablet_tests.rs"]
mod tests;
