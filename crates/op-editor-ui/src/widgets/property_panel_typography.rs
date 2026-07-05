//! Font-family picker for the Typography section — a port of the TS
//! `apps/web/src/components/shared/font-picker.tsx` (searchable
//! dropdown, bundled + system groups) + `use-system-fonts.ts`
//! (bundled list, fallback system list).
//!
//! Differences from the TS browser picker, by design:
//! - System fonts come from the host's Skia `FontMgr` enumeration
//!   (no Local Font Access permission flow — native hosts can always
//!   enumerate). The TS `permissionState === 'denied'` info row has
//!   no native equivalent.
//! - The web (wasm) host ships bundled fonts only, so it paints the
//!   bundled group + the TS `FALLBACK_SYSTEM_FONTS` list — the same
//!   list the TS app shows when `queryLocalFonts` is unavailable.
//! - Keyboard ArrowUp/Down + Enter row navigation is not ported
//!   (mouse + type-ahead only); Escape closes via the host.
//!
//! The dropdown is an overlay (like the image-fill popover): its row
//! hit-rects are NOT part of the section walker — the panel checks
//! [`font_picker_action_at`] before the generic action walk.

use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{INPUT_HEIGHT, PAD_X};
use crate::widgets::property_panel_layout::VisibleSections;
use crate::{Point2D, Rect};
use jian_widgets::components::select::{SelectHit, SelectState};

pub use crate::font_catalog::{BUNDLED_FONT_FAMILIES, FALLBACK_SYSTEM_FONTS};

/// One selectable row (TS `FontInfo`). `imported` fonts are the
/// user's own files (registered via `FontStore` / `jian-skia`); they
/// paint in their own group above the bundled + system groups and
/// carry an inline remove affordance. `bundled` is always `false`
/// when `imported` is `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontPickerEntry<'a> {
    pub family: &'a str,
    pub bundled: bool,
    pub imported: bool,
}

/// Build the picker's visible entries: imported group first (user
/// files), then the bundled group, then the system group (host
/// enumeration, else the TS fallback list). Every group is filtered
/// by the same case-insensitive `search` substring (TS `filtered`
/// memo). The host resolves `SetFontFamilyIndex(i)` against this same
/// function, so paint / hit / dispatch agree.
pub fn font_picker_entries<'a>(
    imported_families: &'a [String],
    system_families: &'a [String],
    search: &str,
) -> Vec<FontPickerEntry<'a>> {
    let q = search.trim().to_lowercase();
    let matches = |family: &str| q.is_empty() || family.to_lowercase().contains(&q);
    let mut out: Vec<FontPickerEntry<'a>> = Vec::new();
    for family in imported_families {
        if matches(family) {
            out.push(FontPickerEntry {
                family,
                bundled: false,
                imported: true,
            });
        }
    }
    for family in BUNDLED_FONT_FAMILIES {
        if matches(family) {
            out.push(FontPickerEntry {
                family,
                bundled: true,
                imported: false,
            });
        }
    }
    if system_families.is_empty() {
        for family in FALLBACK_SYSTEM_FONTS {
            if matches(family) {
                out.push(FontPickerEntry {
                    family,
                    bundled: false,
                    imported: false,
                });
            }
        }
    } else {
        for family in system_families {
            if matches(family) {
                out.push(FontPickerEntry {
                    family,
                    bundled: false,
                    imported: false,
                });
            }
        }
    }
    out
}

/// First family before a comma, quotes stripped (TS `displayName`).
pub fn display_font_family(value: &str) -> &str {
    value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches(['"', '\''])
}

pub const FONT_PICKER_ROW_H: f32 = 24.0;
pub const FONT_PICKER_SEARCH_H: f32 = 28.0;
const GROUP_HEADER_H: f32 = 16.0;
const NO_RESULTS_H: f32 = 40.0;
const LIST_PAD_Y: f32 = 4.0;
/// The bottom "Import font…" action row height.
const IMPORT_ACTION_H: f32 = 28.0;
/// Side of the inline remove-x hit square on an imported entry row.
const REMOVE_X_SIZE: f32 = 16.0;
/// TS dropdown is `max-h-72` (288 px) including the search row.
const MAX_LIST_VIEWPORT_H: f32 = 288.0 - FONT_PICKER_SEARCH_H;

/// A row in the dropdown's scrolling list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontPickerRow {
    GroupImported,
    GroupBundled,
    GroupSystem,
    /// Index into [`font_picker_entries`]' result.
    Entry(usize),
    /// Inline remove-x sub-rect of an imported entry — index into
    /// [`font_picker_entries`]' result. Registered AFTER its `Entry`
    /// so a click on the x wins the overlap.
    RemoveEntry(usize),
    /// Bottom "Import font…" action row — always present regardless
    /// of the search filter (it is an action, not a filtered entry).
    ImportAction,
    NoResults,
}

/// Resolved dropdown geometry. All rects are in panel space with the
/// list scroll already applied; rows outside `viewport` must be
/// skipped by both paint and hit-test.
pub struct FontPickerLayout {
    pub popup: Rect,
    pub search: Rect,
    pub viewport: Rect,
    pub max_scroll: f32,
    pub rows: Vec<(FontPickerRow, Rect)>,
}

/// Anchor y of the family trigger row inside the Typography section
/// (mirrors the trigger rect in `text_action_rects`).
fn family_trigger_rect(panel_rect: Rect, visible: VisibleSections) -> Option<Rect> {
    let text_y = crate::widgets::property_panel_text::text_section_top(panel_rect, visible)?;
    let y = text_y
        + crate::widgets::property_panel_text::TEXT_LAYOUT_BLOCK_H
        + crate::widgets::property_panel_inputs::SECTION_HEADER_HEIGHT;
    Some(Rect {
        origin: Point2D::new(panel_rect.origin.x + PAD_X, y),
        size: Point2D::new(panel_rect.size.x - PAD_X * 2.0, INPUT_HEIGHT),
    })
}

/// Compute the dropdown layout below the trigger. `None` when the
/// Typography section is hidden.
pub fn font_picker_layout(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    allow_import: bool,
    scroll: f32,
) -> Option<FontPickerLayout> {
    let trigger = family_trigger_rect(panel_rect, visible)?;
    let popup_x = trigger.origin.x;
    let popup_w = trigger.size.x;
    let popup_y = trigger.origin.y + trigger.size.y + 4.0; // TS mt-1

    // Walk the list content (unscrolled, y from 0). Groups render in
    // Imported → Bundled → System order, matching `font_picker_entries`.
    let imported_count = entries.iter().filter(|e| e.imported).count();
    let bundled_count = entries.iter().filter(|e| e.bundled).count();
    let system_count = entries.len() - imported_count - bundled_count;
    let mut content: Vec<(FontPickerRow, f32, f32)> = Vec::new();
    let mut cy = 0.0_f32;
    if imported_count > 0 {
        content.push((FontPickerRow::GroupImported, cy, GROUP_HEADER_H));
        cy += GROUP_HEADER_H;
        for (i, e) in entries.iter().enumerate() {
            if e.imported {
                content.push((FontPickerRow::Entry(i), cy, FONT_PICKER_ROW_H));
                // Inline remove-x, vertically centred in the row. Its
                // horizontal extent is derived in the mapper below (a
                // REMOVE_X_SIZE square inset by PAD_X from the right
                // edge). Registered AFTER the entry so hit-tests that
                // check RemoveEntry first win the overlap.
                let x_top = cy + (FONT_PICKER_ROW_H - REMOVE_X_SIZE) / 2.0;
                content.push((FontPickerRow::RemoveEntry(i), x_top, REMOVE_X_SIZE));
                cy += FONT_PICKER_ROW_H;
            }
        }
    }
    if bundled_count > 0 {
        content.push((FontPickerRow::GroupBundled, cy, GROUP_HEADER_H));
        cy += GROUP_HEADER_H;
        for (i, e) in entries.iter().enumerate() {
            if e.bundled {
                content.push((FontPickerRow::Entry(i), cy, FONT_PICKER_ROW_H));
                cy += FONT_PICKER_ROW_H;
            }
        }
    }
    if system_count > 0 {
        content.push((FontPickerRow::GroupSystem, cy, GROUP_HEADER_H));
        cy += GROUP_HEADER_H;
        for (i, e) in entries.iter().enumerate() {
            if !e.bundled && !e.imported {
                content.push((FontPickerRow::Entry(i), cy, FONT_PICKER_ROW_H));
                cy += FONT_PICKER_ROW_H;
            }
        }
    }
    if entries.is_empty() {
        content.push((FontPickerRow::NoResults, cy, NO_RESULTS_H));
        cy += NO_RESULTS_H;
    }
    // The Import action sits at the bottom of the content, visible
    // regardless of the search filter — but only when the host can actually
    // import (desktop rfd dialog + web file-input both set the capability). A
    // host that can't import omits it so there is no dead row.
    if allow_import {
        content.push((FontPickerRow::ImportAction, cy, IMPORT_ACTION_H));
        cy += IMPORT_ACTION_H;
    }
    let content_h = cy + LIST_PAD_Y * 2.0;
    let viewport_h = content_h.min(MAX_LIST_VIEWPORT_H);
    let max_scroll = (content_h - viewport_h).max(0.0);
    let scroll = scroll.clamp(0.0, max_scroll);

    let popup = Rect {
        origin: Point2D::new(popup_x, popup_y),
        size: Point2D::new(popup_w, FONT_PICKER_SEARCH_H + viewport_h),
    };
    let search = Rect {
        origin: popup.origin,
        size: Point2D::new(popup_w, FONT_PICKER_SEARCH_H),
    };
    let viewport = Rect {
        origin: Point2D::new(popup_x, popup_y + FONT_PICKER_SEARCH_H),
        size: Point2D::new(popup_w, viewport_h),
    };
    let list_top = viewport.origin.y + LIST_PAD_Y - scroll;
    let rows = content
        .into_iter()
        .map(|(row, y, h)| {
            // The remove-x is a small square inset from the right edge;
            // every other row spans the popup width.
            let (x, w) = if matches!(row, FontPickerRow::RemoveEntry(_)) {
                (popup_x + popup_w - PAD_X - REMOVE_X_SIZE, REMOVE_X_SIZE)
            } else {
                (popup_x, popup_w)
            };
            (
                row,
                Rect {
                    origin: Point2D::new(x, list_top + y),
                    size: Point2D::new(w, h),
                },
            )
        })
        .collect();
    Some(FontPickerLayout {
        popup,
        search,
        viewport,
        max_scroll,
        rows,
    })
}

/// Whether `point` falls inside the open dropdown (search row or
/// list). Used by the host's outside-click dismiss so a click inside
/// the popup body (e.g. the search box) is swallowed without closing.
pub fn font_picker_contains(
    state: &SelectState,
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    allow_import: bool,
    point: Point2D,
) -> bool {
    if !state.open {
        return false;
    }
    font_picker_layout(
        panel_rect,
        visible,
        entries,
        allow_import,
        state.scroll.offset,
    )
    .is_some_and(|l| (l.popup).contains(point))
}

/// Shared select-style hit protocol for the searchable font picker.
/// Search/header/no-results chrome returns `Inside`; entry rows return
/// `Row(index)` where `index` addresses [`font_picker_entries`].
pub fn font_picker_hit(
    state: &SelectState,
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    allow_import: bool,
    point: Point2D,
) -> SelectHit {
    if !state.open {
        return SelectHit::Outside;
    }
    let Some(layout) = font_picker_layout(
        panel_rect,
        visible,
        entries,
        allow_import,
        state.scroll.offset,
    ) else {
        return SelectHit::Outside;
    };
    if !(layout.popup).contains(point) {
        return SelectHit::Outside;
    }
    if !(layout.viewport).contains(point) {
        return SelectHit::Inside;
    }
    layout
        .rows
        .iter()
        .find_map(|(row, rect)| match row {
            FontPickerRow::Entry(i) if (*rect).contains(point) => Some(SelectHit::Row(*i)),
            FontPickerRow::GroupImported
            | FontPickerRow::GroupBundled
            | FontPickerRow::GroupSystem
            | FontPickerRow::RemoveEntry(_)
            | FontPickerRow::ImportAction
            | FontPickerRow::NoResults
                if (*rect).contains(point) =>
            {
                Some(SelectHit::Inside)
            }
            _ => None,
        })
        .unwrap_or(SelectHit::Inside)
}

/// Action for a click at `point` while the dropdown is open. Priority
/// inside the viewport: an imported entry's remove-x
/// (`RemoveImportedFont`) wins over the entry body, then the bottom
/// `ImportFont` action, then a plain entry (`SetFontFamilyIndex`).
/// Chrome (search / group headers) and out-of-viewport clicks yield
/// `None` (swallowed, popup stays open).
pub fn font_picker_action_at(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    allow_import: bool,
    state: &SelectState,
    point: Point2D,
) -> Option<PropertyPanelAction> {
    if !state.open {
        return None;
    }
    let layout = font_picker_layout(
        panel_rect,
        visible,
        entries,
        allow_import,
        state.scroll.offset,
    )?;
    if !(layout.viewport).contains(point) {
        return None;
    }
    // The remove-x rect sits ON TOP of its entry, so it must be tested
    // before the entry body; the loop returns on RemoveEntry /
    // ImportAction and only records a plain entry hit to fall back on.
    let mut entry_hit: Option<usize> = None;
    for (row, rect) in &layout.rows {
        if !(*rect).contains(point) {
            continue;
        }
        match row {
            FontPickerRow::RemoveEntry(i) => {
                return Some(PropertyPanelAction::RemoveImportedFont(*i))
            }
            FontPickerRow::ImportAction => return Some(PropertyPanelAction::ImportFont),
            FontPickerRow::Entry(i) => entry_hit = Some(*i),
            _ => {}
        }
    }
    entry_hit.map(PropertyPanelAction::SetFontFamilyIndex)
}

/// Entry index under `point` (hover tracking + hit-test share this).
pub fn font_picker_entry_index_at(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    allow_import: bool,
    state: &SelectState,
    point: Point2D,
) -> Option<usize> {
    match font_picker_hit(state, panel_rect, visible, entries, allow_import, point) {
        SelectHit::Row(index) => Some(index),
        SelectHit::Inside | SelectHit::Outside => None,
    }
}

/// Whether `point` is over the bottom "Import font…" (`ImportAction`)
/// row of the open picker — drives the import-row hover wash. Only
/// meaningful while the picker is open and `allow_import` is set; the
/// point must be inside the list viewport (same clip the other hit
/// fns respect).
pub fn font_picker_import_action_at(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    allow_import: bool,
    state: &SelectState,
    point: Point2D,
) -> bool {
    if !state.open || !allow_import {
        return false;
    }
    let Some(layout) = font_picker_layout(
        panel_rect,
        visible,
        entries,
        allow_import,
        state.scroll.offset,
    ) else {
        return false;
    };
    if !(layout.viewport).contains(point) {
        return false;
    }
    layout
        .rows
        .iter()
        .any(|(row, rect)| matches!(row, FontPickerRow::ImportAction) && (*rect).contains(point))
}

/// Max scroll for the host's wheel handler.
pub fn font_picker_max_scroll(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    allow_import: bool,
) -> f32 {
    font_picker_layout(panel_rect, visible, entries, allow_import, 0.0)
        .map_or(0.0, |l| l.max_scroll)
}

// The dropdown paint pass lives in a sibling file (`_paint.rs`) to
// keep this module under the 800-line ceiling; re-exported here so the
// public path `property_panel_typography::paint_font_picker` is
// unchanged for the panel + hosts.
#[path = "property_panel_typography_paint.rs"]
mod paint;
pub use paint::paint_font_picker;

#[cfg(test)]
#[path = "property_panel_typography_tests.rs"]
mod tests;
