//! Shared rect helpers for the VariablesPanel (header buttons, rows,
//! cells, menus) — split from the spine to honor the 800-line cap.
//! Re-exported by the spine so paint / hit / menus resolve them via
//! `use super::*` exactly as before.

use super::{VariablesPanelHit, ACTION_COLUMN_WIDTH, ADD_VARIABLE_MENU_ROW_HEIGHT,
    ADD_VARIABLE_MENU_ROWS, ADD_VARIABLE_MENU_WIDTH, FOOTER_HEIGHT, HEADER_HEIGHT,
    NAME_COLUMN_WIDTH, PAD_X, SWATCH_SIZE};
use crate::{Point2D, Rect};

pub(super) fn close_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(rect.origin.x + rect.size.x - 42.0, rect.origin.y + 9.0),
        size: Point2D::new(26.0, 26.0),
    }
}

pub(super) fn add_variant_rect(rect: Rect) -> Rect {
    Rect {
        origin: Point2D::new(
            rect.origin.x + rect.size.x - PAD_X - 28.0,
            rect.origin.y + HEADER_HEIGHT + 4.0,
        ),
        size: Point2D::new(28.0, 28.0),
    }
}

pub(super) fn add_variable_rect(rect: Rect) -> Rect {
    let height = 30.0;
    Rect {
        origin: Point2D::new(
            rect.origin.x + PAD_X,
            rect.origin.y + rect.size.y - FOOTER_HEIGHT + (FOOTER_HEIGHT - height) / 2.0,
        ),
        size: Point2D::new(164.0, height),
    }
}

pub(super) fn add_variable_menu_rect(rect: Rect) -> Rect {
    let height = ADD_VARIABLE_MENU_ROW_HEIGHT * ADD_VARIABLE_MENU_ROWS;
    Rect {
        origin: Point2D::new(
            rect.origin.x + PAD_X,
            rect.origin.y + rect.size.y - FOOTER_HEIGHT - height - 6.0,
        ),
        size: Point2D::new(ADD_VARIABLE_MENU_WIDTH, height),
    }
}

pub(super) fn menu_rows_height(sibling_count: usize) -> f32 {
    let rows = if sibling_count > 1 { 2.0 } else { 1.0 };
    ADD_VARIABLE_MENU_ROW_HEIGHT * rows
}

pub(super) fn value_column_x(rect: Rect) -> f32 {
    rect.origin.x + PAD_X + NAME_COLUMN_WIDTH
}

/// Swatch hit zone inside a Color value cell — the painted 18 px
/// swatch plus a little slop (TS's 20 px `<input type=color>`).
pub(super) fn color_swatch_hit_rect(cell: Rect) -> Rect {
    Rect {
        origin: Point2D::new(cell.origin.x - 2.0, cell.origin.y + 8.0),
        size: Point2D::new(SWATCH_SIZE + 6.0, SWATCH_SIZE + 4.0),
    }
}

/// Inline hex text zone inside a Color value cell (TS's 76 px hex
/// `<input>` right of the swatch).
pub(super) fn color_hex_rect(cell: Rect) -> Rect {
    Rect {
        origin: Point2D::new(cell.origin.x + SWATCH_SIZE + 6.0, cell.origin.y + 7.0),
        size: Point2D::new(84.0, 30.0),
    }
}

pub(super) fn variant_column_width(rect: Rect, count: usize) -> f32 {
    let count = count.max(1) as f32;
    let available = rect.size.x - PAD_X * 2.0 - NAME_COLUMN_WIDTH - ACTION_COLUMN_WIDTH;
    (available / count).max(156.0)
}

pub(super) fn label_width(text: &str, size: f32) -> f32 {
    text.chars()
        .map(|c| if c.is_ascii() { size * 0.55 } else { size })
        .sum()
}

pub(super) fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x < r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y < r.origin.y + r.size.y
}

