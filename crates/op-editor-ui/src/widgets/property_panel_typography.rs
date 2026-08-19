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

use std::rc::Rc;

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
///
/// `family` is owned (not `&'a str` borrowed from the caller's family
/// lists) so a built `Vec<FontPickerEntry>` can be memoized cross-frame
/// in `font_picker_cache` without a self-referential lifetime — see
/// that module's docs. The allocation only happens when the cache
/// actually rebuilds (query or family-list change), not on every call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontPickerEntry {
    pub family: String,
    pub bundled: bool,
    pub imported: bool,
}

/// Build the picker's visible entries: imported group first (user
/// files), then the bundled group, then the system group (host
/// enumeration, else the TS fallback list). Every group is filtered
/// by the same case-insensitive `search` substring (TS `filtered`
/// memo). The host resolves `SetFontFamilyIndex(i)` against this same
/// function, so paint / hit / dispatch agree.
///
/// Memoized cross-frame in `font_picker_cache`, keyed by value on
/// `(imported_families, system_families, query)` — see that module's
/// docs for why a shared, un-owner-scoped thread-local slot is safe
/// here. Returns the cache's shared `Rc` directly (not a fresh clone
/// of the `Vec`), so a hit — the common case: the panel re-reads this
/// from up to five accessors per frame with unchanged inputs — costs a
/// refcount bump, not a deep clone of every entry's owned `family`.
pub fn font_picker_entries(
    imported_families: &[String],
    bundled_families: &[String],
    system_families: &[String],
    search: &str,
) -> Rc<Vec<FontPickerEntry>> {
    let q = search.trim().to_lowercase();
    super::font_picker_cache::resolve(
        imported_families,
        bundled_families,
        system_families,
        &q,
        || build_font_picker_entries(imported_families, bundled_families, system_families, &q),
    )
}

/// The actual filter pass — `q` is already trimmed + lowercased.
fn build_font_picker_entries(
    imported_families: &[String],
    bundled_families: &[String],
    system_families: &[String],
    q: &str,
) -> Vec<FontPickerEntry> {
    let matches = |family: &str| q.is_empty() || family.to_lowercase().contains(q);
    let mut out: Vec<FontPickerEntry> = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let mut seen_system_families = Vec::<String>::new();
    for family in imported_families {
        let key = family.trim().to_ascii_lowercase();
        if !key.is_empty() && matches(family) && seen.insert(key) {
            out.push(FontPickerEntry {
                family: family.clone(),
                bundled: false,
                imported: true,
            });
        }
    }
    for family in bundled_families {
        let key = family.trim().to_ascii_lowercase();
        if !key.is_empty() && matches(family) && seen.insert(key) {
            out.push(FontPickerEntry {
                family: family.clone(),
                bundled: true,
                imported: false,
            });
        }
    }
    if system_families.is_empty() {
        for family in FALLBACK_SYSTEM_FONTS {
            let key = family.trim().to_ascii_lowercase();
            if matches(family) && seen.insert(key) {
                out.push(FontPickerEntry {
                    family: family.to_string(),
                    bundled: false,
                    imported: false,
                });
            }
        }
    } else {
        for family in system_families {
            let key = family.trim().to_ascii_lowercase();
            let is_alias_duplicate = seen_system_families.iter().any(|existing| {
                op_editor_core::font_catalog::is_same_font_family(existing, family)
            });
            if !key.is_empty() && matches(family) && !is_alias_duplicate && seen.insert(key) {
                seen_system_families.push(family.clone());
                out.push(FontPickerEntry {
                    family: family.clone(),
                    bundled: false,
                    imported: false,
                });
            }
        }
    }
    out
}

/// First family in a CSS stack, quotes stripped (TS `displayName`).
///
/// A quoted family may itself contain a comma, so a plain `split(',')`
/// would turn `"ACME, Display", sans-serif` into the misleading label
/// `ACME`. Follow the same quote and escape boundaries as the canonical
/// stack parser while keeping the borrowed return value used by paint code.
pub fn display_font_family(value: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    let mut end = value.len();
    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if let Some(active) = quote {
            if ch == active {
                quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            quote = Some(ch);
        } else if ch == ',' {
            end = index;
            break;
        }
    }
    let family = value[..end].trim();
    let Some(quote) = family.chars().next().filter(|ch| *ch == '"' || *ch == '\'') else {
        return family;
    };
    family
        .strip_prefix(quote)
        .and_then(|without_start| without_start.strip_suffix(quote))
        .unwrap_or(family)
}

pub const FONT_PICKER_ROW_H: f32 = 24.0;
pub const FONT_PICKER_SEARCH_H: f32 = 28.0;
const TOUCH_FONT_PICKER_ROW_H: f32 = 44.0;
const TOUCH_FONT_PICKER_SEARCH_H: f32 = 44.0;
const GROUP_HEADER_H: f32 = 16.0;
const TOUCH_GROUP_HEADER_H: f32 = 28.0;
const NO_RESULTS_H: f32 = 40.0;
const LIST_PAD_Y: f32 = 4.0;
/// The bottom "Import font…" action row height.
const IMPORT_ACTION_H: f32 = 28.0;
/// Side of the inline remove-x hit square on an imported entry row.
const REMOVE_X_SIZE: f32 = 16.0;
const TOUCH_REMOVE_X_SIZE: f32 = 44.0;
/// Desktop list viewport cap. Touch keeps the same 288-point popup cap while
/// accounting for its taller search field.
#[cfg(test)]
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
    pub touch_controls: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontPickerAction {
    Select(usize),
    Import,
    Remove(usize),
}

/// Anchor y of the family trigger row inside the Typography section
/// (mirrors the trigger rect in `text_action_rects`).
fn family_trigger_rect(panel_rect: Rect, visible: VisibleSections) -> Option<Rect> {
    let text_y = crate::widgets::property_panel_text::text_section_top(panel_rect, visible)?;
    let y = text_y
        + crate::widgets::property_panel_text::text_layout_block_height(visible.touch_controls)
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
    entries: &[FontPickerEntry],
    allow_import: bool,
    scroll: f32,
) -> Option<FontPickerLayout> {
    let trigger = family_trigger_rect(panel_rect, visible)?;
    Some(font_picker_layout_at_with_density(
        trigger,
        trigger.size.x,
        panel_rect,
        entries,
        allow_import,
        true,
        scroll,
        visible.touch_controls,
    ))
}

/// Compute the shared searchable picker against an arbitrary trigger. Missing
/// font rows use this entry point so they share the exact same grouping,
/// scrolling, hit geometry, and search behavior as the Typography picker.
#[allow(clippy::too_many_arguments)]
pub fn font_picker_layout_at(
    trigger: Rect,
    popup_width: f32,
    bounds: Rect,
    entries: &[FontPickerEntry],
    allow_import: bool,
    allow_remove: bool,
    scroll: f32,
) -> FontPickerLayout {
    font_picker_layout_at_for_ui(
        trigger,
        popup_width,
        bounds,
        entries,
        allow_import,
        allow_remove,
        scroll,
        false,
    )
}

/// Density-aware arbitrary-anchor picker geometry. Settings surfaces use this
/// entry point so touch paint, hit-testing, and scrolling share 44-point rows.
#[allow(clippy::too_many_arguments)]
pub fn font_picker_layout_at_for_ui(
    trigger: Rect,
    popup_width: f32,
    bounds: Rect,
    entries: &[FontPickerEntry],
    allow_import: bool,
    allow_remove: bool,
    scroll: f32,
    touch_controls: bool,
) -> FontPickerLayout {
    font_picker_layout_at_with_density(
        trigger,
        popup_width,
        bounds,
        entries,
        allow_import,
        allow_remove,
        scroll,
        touch_controls,
    )
}

#[allow(clippy::too_many_arguments)]
fn font_picker_layout_at_with_density(
    trigger: Rect,
    popup_width: f32,
    bounds: Rect,
    entries: &[FontPickerEntry],
    allow_import: bool,
    allow_remove: bool,
    scroll: f32,
    touch_controls: bool,
) -> FontPickerLayout {
    let row_h = if touch_controls {
        TOUCH_FONT_PICKER_ROW_H
    } else {
        FONT_PICKER_ROW_H
    };
    let search_h = if touch_controls {
        TOUCH_FONT_PICKER_SEARCH_H
    } else {
        FONT_PICKER_SEARCH_H
    };
    let import_h = if touch_controls {
        TOUCH_FONT_PICKER_ROW_H
    } else {
        IMPORT_ACTION_H
    };
    let remove_size = if touch_controls {
        TOUCH_REMOVE_X_SIZE
    } else {
        REMOVE_X_SIZE
    };
    let group_header_h = if touch_controls {
        TOUCH_GROUP_HEADER_H
    } else {
        GROUP_HEADER_H
    };
    // Walk the list content (unscrolled, y from 0). Groups render in
    // Imported → Bundled → System order, matching `font_picker_entries`.
    let imported_count = entries.iter().filter(|e| e.imported).count();
    let bundled_count = entries.iter().filter(|e| e.bundled).count();
    let system_count = entries.len() - imported_count - bundled_count;
    let mut content: Vec<(FontPickerRow, f32, f32)> = Vec::new();
    let mut cy = 0.0_f32;
    if imported_count > 0 {
        content.push((FontPickerRow::GroupImported, cy, group_header_h));
        cy += group_header_h;
        for (i, e) in entries.iter().enumerate() {
            if e.imported {
                content.push((FontPickerRow::Entry(i), cy, row_h));
                // Inline remove-x, vertically centred in the row. Its
                // horizontal extent is derived in the mapper below (a
                // REMOVE_X_SIZE square inset by PAD_X from the right
                // edge). Registered AFTER the entry so hit-tests that
                // check RemoveEntry first win the overlap.
                if allow_remove {
                    let x_top = cy + (row_h - remove_size) / 2.0;
                    content.push((FontPickerRow::RemoveEntry(i), x_top, remove_size));
                }
                cy += row_h;
            }
        }
    }
    if bundled_count > 0 {
        content.push((FontPickerRow::GroupBundled, cy, group_header_h));
        cy += group_header_h;
        for (i, e) in entries.iter().enumerate() {
            if e.bundled {
                content.push((FontPickerRow::Entry(i), cy, row_h));
                cy += row_h;
            }
        }
    }
    if system_count > 0 {
        content.push((FontPickerRow::GroupSystem, cy, group_header_h));
        cy += group_header_h;
        for (i, e) in entries.iter().enumerate() {
            if !e.bundled && !e.imported {
                content.push((FontPickerRow::Entry(i), cy, row_h));
                cy += row_h;
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
        content.push((FontPickerRow::ImportAction, cy, import_h));
        cy += import_h;
    }
    let content_h = cy + LIST_PAD_Y * 2.0;
    let max_list_viewport_h = (288.0 - search_h).min((bounds.size.y - search_h).max(0.0));
    let viewport_h = content_h.min(max_list_viewport_h);
    let max_scroll = (content_h - viewport_h).max(0.0);
    let scroll = scroll.clamp(0.0, max_scroll);

    let popup_w = popup_width.min(bounds.size.x).max(1.0);
    let popup_h = search_h + viewport_h;
    let min_x = bounds.origin.x;
    let max_x = (bounds.origin.x + bounds.size.x - popup_w).max(min_x);
    let popup_x = (trigger.origin.x + trigger.size.x - popup_w).clamp(min_x, max_x);
    let bounds_bottom = bounds.origin.y + bounds.size.y;
    let below = trigger.origin.y + trigger.size.y + 4.0;
    let above = trigger.origin.y - popup_h - 4.0;
    let preferred_y = if below + popup_h <= bounds_bottom || above < bounds.origin.y {
        below
    } else {
        above
    };
    let max_y = (bounds_bottom - popup_h).max(bounds.origin.y);
    let popup_y = preferred_y.clamp(bounds.origin.y, max_y);

    let popup = Rect {
        origin: Point2D::new(popup_x, popup_y),
        size: Point2D::new(popup_w, popup_h),
    };
    let search = Rect {
        origin: popup.origin,
        size: Point2D::new(popup_w, search_h),
    };
    let viewport = Rect {
        origin: Point2D::new(popup_x, popup_y + search_h),
        size: Point2D::new(popup_w, viewport_h),
    };
    let list_top = viewport.origin.y + LIST_PAD_Y - scroll;
    let rows = content
        .into_iter()
        .map(|(row, y, h)| {
            // The remove-x is a small square inset from the right edge;
            // every other row spans the popup width.
            let (x, w) = if matches!(row, FontPickerRow::RemoveEntry(_)) {
                (popup_x + popup_w - PAD_X - remove_size, remove_size)
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
    FontPickerLayout {
        popup,
        search,
        viewport,
        max_scroll,
        rows,
        touch_controls,
    }
}

pub fn font_picker_hit_in_layout(layout: &FontPickerLayout, point: Point2D) -> SelectHit {
    if !layout.popup.contains(point) {
        return SelectHit::Outside;
    }
    if !layout.viewport.contains(point) {
        return SelectHit::Inside;
    }
    layout
        .rows
        .iter()
        .find_map(|(row, rect)| match row {
            FontPickerRow::Entry(index) if rect.contains(point) => Some(SelectHit::Row(*index)),
            _ if rect.contains(point) => Some(SelectHit::Inside),
            _ => None,
        })
        .unwrap_or(SelectHit::Inside)
}

pub fn font_picker_action_in_layout(
    layout: &FontPickerLayout,
    point: Point2D,
) -> Option<FontPickerAction> {
    if !layout.viewport.contains(point) {
        return None;
    }
    let mut entry_hit = None;
    for (row, rect) in &layout.rows {
        if !rect.contains(point) {
            continue;
        }
        match row {
            FontPickerRow::RemoveEntry(index) => return Some(FontPickerAction::Remove(*index)),
            FontPickerRow::ImportAction => return Some(FontPickerAction::Import),
            FontPickerRow::Entry(index) => entry_hit = Some(*index),
            _ => {}
        }
    }
    entry_hit.map(FontPickerAction::Select)
}

pub fn font_picker_import_hover_in_layout(layout: &FontPickerLayout, point: Point2D) -> bool {
    layout.viewport.contains(point)
        && layout
            .rows
            .iter()
            .any(|(row, rect)| matches!(row, FontPickerRow::ImportAction) && rect.contains(point))
}

/// Whether `point` falls inside the open dropdown (search row or
/// list). Used by the host's outside-click dismiss so a click inside
/// the popup body (e.g. the search box) is swallowed without closing.
pub fn font_picker_contains(
    state: &SelectState,
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry],
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
    entries: &[FontPickerEntry],
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
    entries: &[FontPickerEntry],
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
    entries: &[FontPickerEntry],
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
    entries: &[FontPickerEntry],
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
    entries: &[FontPickerEntry],
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
pub use paint::{paint_font_picker, paint_font_picker_at, paint_font_picker_at_for_ui};

#[cfg(test)]
#[path = "property_panel_typography_tests.rs"]
mod tests;
