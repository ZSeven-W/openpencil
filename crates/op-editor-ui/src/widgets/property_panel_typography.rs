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

use crate::theme::Theme;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::PropertyPanelAction;
use crate::widgets::property_panel_inputs::{INPUT_HEIGHT, PAD_X};
use crate::widgets::property_panel_layout::VisibleSections;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use jian_widgets::components::select::{SelectHit, SelectState};

pub use crate::font_catalog::{BUNDLED_FONT_FAMILIES, FALLBACK_SYSTEM_FONTS};

/// One selectable row (TS `FontInfo`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontPickerEntry<'a> {
    pub family: &'a str,
    pub bundled: bool,
}

/// Build the picker's visible entries: bundled group first, then the
/// system group (host enumeration, else the TS fallback list), both
/// filtered by the case-insensitive `search` substring (TS
/// `filtered` memo). The host resolves `SetFontFamilyIndex(i)`
/// against this same function, so paint / hit / dispatch agree.
pub fn font_picker_entries<'a>(
    system_families: &'a [String],
    search: &str,
) -> Vec<FontPickerEntry<'a>> {
    let q = search.trim().to_lowercase();
    let matches = |family: &str| q.is_empty() || family.to_lowercase().contains(&q);
    let mut out: Vec<FontPickerEntry<'a>> = Vec::new();
    for family in BUNDLED_FONT_FAMILIES {
        if matches(family) {
            out.push(FontPickerEntry {
                family,
                bundled: true,
            });
        }
    }
    if system_families.is_empty() {
        for family in FALLBACK_SYSTEM_FONTS {
            if matches(family) {
                out.push(FontPickerEntry {
                    family,
                    bundled: false,
                });
            }
        }
    } else {
        for family in system_families {
            if matches(family) {
                out.push(FontPickerEntry {
                    family,
                    bundled: false,
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
/// TS dropdown is `max-h-72` (288 px) including the search row.
const MAX_LIST_VIEWPORT_H: f32 = 288.0 - FONT_PICKER_SEARCH_H;

/// A row in the dropdown's scrolling list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontPickerRow {
    GroupBundled,
    GroupSystem,
    /// Index into [`font_picker_entries`]' result.
    Entry(usize),
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
    scroll: f32,
) -> Option<FontPickerLayout> {
    let trigger = family_trigger_rect(panel_rect, visible)?;
    let popup_x = trigger.origin.x;
    let popup_w = trigger.size.x;
    let popup_y = trigger.origin.y + trigger.size.y + 4.0; // TS mt-1

    // Walk the list content (unscrolled, y from 0).
    let bundled_count = entries.iter().filter(|e| e.bundled).count();
    let system_count = entries.len() - bundled_count;
    let mut content: Vec<(FontPickerRow, f32, f32)> = Vec::new();
    let mut cy = 0.0_f32;
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
            if !e.bundled {
                content.push((FontPickerRow::Entry(i), cy, FONT_PICKER_ROW_H));
                cy += FONT_PICKER_ROW_H;
            }
        }
    }
    if entries.is_empty() {
        content.push((FontPickerRow::NoResults, cy, NO_RESULTS_H));
        cy += NO_RESULTS_H;
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
            (
                row,
                Rect {
                    origin: Point2D::new(popup_x, list_top + y),
                    size: Point2D::new(popup_w, h),
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
    point: Point2D,
) -> bool {
    if !state.open {
        return false;
    }
    font_picker_layout(panel_rect, visible, entries, state.scroll.offset)
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
    point: Point2D,
) -> SelectHit {
    if !state.open {
        return SelectHit::Outside;
    }
    let Some(layout) = font_picker_layout(panel_rect, visible, entries, state.scroll.offset) else {
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
            FontPickerRow::GroupBundled | FontPickerRow::GroupSystem | FontPickerRow::NoResults
                if (*rect).contains(point) =>
            {
                Some(SelectHit::Inside)
            }
            _ => None,
        })
        .unwrap_or(SelectHit::Inside)
}

/// Action for a click at `point` while the dropdown is open —
/// `SetFontFamilyIndex(i)` when an entry row (visible inside the
/// viewport) is hit.
pub fn font_picker_action_at(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    state: &SelectState,
    point: Point2D,
) -> Option<PropertyPanelAction> {
    match font_picker_hit(state, panel_rect, visible, entries, point) {
        SelectHit::Row(index) => Some(PropertyPanelAction::SetFontFamilyIndex(index)),
        SelectHit::Inside | SelectHit::Outside => None,
    }
}

/// Entry index under `point` (hover tracking + hit-test share this).
pub fn font_picker_entry_index_at(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
    state: &SelectState,
    point: Point2D,
) -> Option<usize> {
    match font_picker_hit(state, panel_rect, visible, entries, point) {
        SelectHit::Row(index) => Some(index),
        SelectHit::Inside | SelectHit::Outside => None,
    }
}

/// Max scroll for the host's wheel handler.
pub fn font_picker_max_scroll(
    panel_rect: Rect,
    visible: VisibleSections,
    entries: &[FontPickerEntry<'_>],
) -> f32 {
    font_picker_layout(panel_rect, visible, entries, 0.0).map_or(0.0, |l| l.max_scroll)
}

/// Paint the dropdown (call as a late overlay, after the sections).
#[allow(clippy::too_many_arguments)]
pub fn paint_font_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    locale: op_editor_core::Locale,
    entries: &[FontPickerEntry<'_>],
    search: &str,
    state: &SelectState,
    active_family: &str,
) {
    if !state.open {
        return;
    }
    let Some(layout) = font_picker_layout(panel_rect, visible, entries, state.scroll.offset) else {
        return;
    };
    cx.backend.fill_round_rect(layout.popup, 8.0, theme.popover);
    cx.backend
        .stroke_round_rect(layout.popup, 8.0, theme.border, 1.0);

    // Search row — Search glyph + draft (or muted placeholder) +
    // steady caret, separated by a hairline (TS border-b).
    let s = layout.search;
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(s.origin.x + 8.0, s.origin.y + (s.size.y - 12.0) / 2.0),
        12.0,
        theme.muted_foreground,
        1.5,
    );
    let text_x = s.origin.x + 26.0;
    let baseline = s.origin.y + s.size.y / 2.0 + 4.0;
    if search.is_empty() {
        let placeholder = TextLayout::single_run(
            op_i18n::translate(locale, "text.font.search"),
            "system-ui",
            11.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&placeholder, Point2D::new(text_x, baseline));
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(text_x, s.origin.y + 7.0),
                size: Point2D::new(1.0, s.size.y - 14.0),
            },
            theme.foreground,
        );
    } else {
        let draft = TextLayout::single_run(
            search,
            "system-ui",
            11.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(&draft, Point2D::new(text_x, baseline));
        let w = cx.backend.measure_text(search, 11.0);
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(text_x + w + 1.0, s.origin.y + 7.0),
                size: Point2D::new(1.0, s.size.y - 14.0),
            },
            theme.foreground,
        );
    }
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(s.origin.x, s.origin.y + s.size.y - 1.0),
            size: Point2D::new(s.size.x, 1.0),
        },
        theme.border,
    );

    // Scrolling list — clipped to the viewport.
    cx.backend.save();
    cx.backend.clip_rect(layout.viewport);
    let active = display_font_family(active_family);
    for (row, rect) in &layout.rows {
        // Cull rows fully outside the viewport.
        if rect.origin.y + rect.size.y < layout.viewport.origin.y
            || rect.origin.y > layout.viewport.origin.y + layout.viewport.size.y
        {
            continue;
        }
        match row {
            FontPickerRow::GroupBundled | FontPickerRow::GroupSystem => {
                let key = if matches!(row, FontPickerRow::GroupBundled) {
                    "text.font.bundled"
                } else {
                    "text.font.system"
                };
                let label = TextLayout::single_run(
                    op_i18n::translate(locale, key),
                    "system-ui",
                    9.0,
                    (theme.muted_foreground).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &label,
                    Point2D::new(rect.origin.x + 10.0, rect.origin.y + rect.size.y - 4.0),
                );
            }
            FontPickerRow::NoResults => {
                let label_str = op_i18n::translate(locale, "text.font.noResults");
                let label = TextLayout::single_run(
                    label_str,
                    "system-ui",
                    11.0,
                    (theme.muted_foreground).to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                let w = cx.backend.measure_text(label_str, 11.0);
                cx.backend.draw_text(
                    &label,
                    Point2D::new(
                        rect.origin.x + (rect.size.x - w) / 2.0,
                        rect.origin.y + rect.size.y / 2.0 + 4.0,
                    ),
                );
            }
            FontPickerRow::Entry(i) => {
                let Some(entry) = entries.get(*i) else {
                    continue;
                };
                let is_active = entry.family == active;
                let row_rect = Rect {
                    origin: Point2D::new(rect.origin.x + 2.0, rect.origin.y),
                    size: Point2D::new(rect.size.x - 4.0, rect.size.y),
                };
                if is_active {
                    cx.backend
                        .fill_round_rect(row_rect, 5.0, theme.row_selected_primary);
                } else if state.hover == Some(*i) || state.pressed == Some(*i) {
                    paint_button_feedback_wash(
                        cx.backend,
                        theme,
                        row_rect,
                        5.0,
                        state.hover == Some(*i),
                        state.pressed == Some(*i),
                    );
                }
                // Each row renders in its own family (TS style
                // fontFamily: font.family).
                let label = TextLayout::single_run(
                    entry.family,
                    entry.family,
                    11.0,
                    (if is_active {
                        theme.primary
                    } else {
                        theme.foreground
                    })
                    .to_jian(),
                    Point2D::new(0.0, 0.0),
                );
                cx.backend.draw_text(
                    &label,
                    Point2D::new(
                        rect.origin.x + 10.0,
                        rect.origin.y + rect.size.y / 2.0 + 4.0,
                    ),
                );
                if is_active {
                    draw_icon(
                        cx.backend,
                        Icon::Check,
                        Point2D::new(
                            rect.origin.x + rect.size.x - 20.0,
                            rect.origin.y + (rect.size.y - 12.0) / 2.0,
                        ),
                        12.0,
                        theme.primary,
                        1.6,
                    );
                }
            }
        }
    }
    cx.backend.restore();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[derive(Default)]
    struct RoundFillBackend {
        fills: Vec<(Rect, f32, Color)>,
    }

    impl RenderBackend for RoundFillBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
            self.fills.push((rect, radius, color));
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    use jian_widgets::components::select::{SelectHit, SelectState};

    fn visible_text() -> VisibleSections {
        VisibleSections {
            text: true,
            create_component: false,
            flex_layout: false,
            size_options: false,
            icon: false,
            ..VisibleSections::ALL
        }
    }

    fn panel_rect() -> Rect {
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(280.0, 1200.0),
        }
    }

    fn open_state(scroll: f32) -> SelectState {
        SelectState {
            open: true,
            scroll: jian_core::scroll::ScrollState { offset: scroll },
            ..Default::default()
        }
    }

    #[test]
    fn entries_fall_back_to_ts_system_list_when_unenumerated() {
        let entries = font_picker_entries(&[], "");
        assert_eq!(
            entries.len(),
            BUNDLED_FONT_FAMILIES.len() + FALLBACK_SYSTEM_FONTS.len()
        );
        assert!(entries[0].bundled);
        assert_eq!(entries[0].family, "Inter");
        let first_system = entries.iter().find(|e| !e.bundled).unwrap();
        assert_eq!(first_system.family, "Arial");
    }

    #[test]
    fn entries_use_host_enumeration_and_filter_by_search() {
        let system = vec!["Avenir".to_string(), "Menlo".to_string()];
        let entries = font_picker_entries(&system, "");
        assert_eq!(entries.len(), BUNDLED_FONT_FAMILIES.len() + 2);
        // Case-insensitive substring filter (TS `filtered` memo).
        let filtered = font_picker_entries(&system, "men");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].family, "Menlo");
        assert!(!filtered[0].bundled);
        // No matches → empty (panel paints the no-results row).
        assert!(font_picker_entries(&system, "zzz").is_empty());
    }

    #[test]
    fn display_name_strips_fallback_stack_and_quotes() {
        assert_eq!(display_font_family("Arial, sans-serif"), "Arial");
        assert_eq!(display_font_family("\"DM Sans\", sans-serif"), "DM Sans");
        assert_eq!(display_font_family("Inter"), "Inter");
    }

    #[test]
    fn entry_hit_maps_back_to_the_clicked_family() {
        let entries = font_picker_entries(&[], "");
        let layout = font_picker_layout(panel_rect(), visible_text(), &entries, 0.0).unwrap();
        // First entry row (after the bundled group header).
        let (row, rect) = layout
            .rows
            .iter()
            .find(|(row, _)| matches!(row, FontPickerRow::Entry(_)))
            .unwrap();
        let FontPickerRow::Entry(expect) = row else {
            unreachable!()
        };
        let centre = Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        );
        assert_eq!(
            font_picker_entry_index_at(
                panel_rect(),
                visible_text(),
                &entries,
                &open_state(0.0),
                centre
            ),
            Some(*expect)
        );
        assert!(matches!(
            font_picker_action_at(panel_rect(), visible_text(), &entries, &open_state(0.0), centre),
            Some(PropertyPanelAction::SetFontFamilyIndex(i)) if i == *expect
        ));
    }

    #[test]
    fn list_viewport_caps_at_max_height_and_scrolls() {
        // 22 entries (11 + 11 fallback) + 2 headers exceed the 260 px
        // viewport, so the dropdown must report scrollable overflow.
        let entries = font_picker_entries(&[], "");
        let layout = font_picker_layout(panel_rect(), visible_text(), &entries, 0.0).unwrap();
        assert!(layout.viewport.size.y <= MAX_LIST_VIEWPORT_H + 0.01);
        assert!(layout.max_scroll > 0.0);
        // A row scrolled above the viewport is not hittable.
        let entries2 = font_picker_entries(&[], "");
        let first_entry_rect = layout
            .rows
            .iter()
            .find_map(|(row, r)| matches!(row, FontPickerRow::Entry(_)).then_some(*r))
            .unwrap();
        let centre = Point2D::new(
            first_entry_rect.origin.x + 10.0,
            first_entry_rect.origin.y + first_entry_rect.size.y / 2.0,
        );
        // With max scroll applied, the first row has moved up and the
        // same point now resolves to a LATER entry (or none) — never
        // the first one.
        let hit = font_picker_entry_index_at(
            panel_rect(),
            visible_text(),
            &entries2,
            &open_state(layout.max_scroll),
            centre,
        );
        assert_ne!(hit, Some(0));
    }

    #[test]
    fn picker_contains_covers_search_row() {
        let entries = font_picker_entries(&[], "");
        let layout = font_picker_layout(panel_rect(), visible_text(), &entries, 0.0).unwrap();
        let in_search = Point2D::new(layout.search.origin.x + 5.0, layout.search.origin.y + 5.0);
        assert!(font_picker_contains(
            &open_state(0.0),
            panel_rect(),
            visible_text(),
            &entries,
            in_search
        ));
        // ... and a search-row click maps to NO action (swallowed,
        // keeps the popup open).
        assert!(font_picker_action_at(
            panel_rect(),
            visible_text(),
            &entries,
            &open_state(0.0),
            in_search
        )
        .is_none());
    }

    #[test]
    fn font_picker_hit_uses_shared_select_state_protocol() {
        let entries = font_picker_entries(&[], "");
        let state = SelectState {
            open: true,
            ..Default::default()
        };
        let layout = font_picker_layout(panel_rect(), visible_text(), &entries, 0.0).unwrap();
        let (row, rect) = layout
            .rows
            .iter()
            .find(|(row, _)| matches!(row, FontPickerRow::Entry(_)))
            .unwrap();
        let FontPickerRow::Entry(expect) = row else {
            unreachable!()
        };

        assert_eq!(
            font_picker_hit(
                &state,
                panel_rect(),
                visible_text(),
                &entries,
                Point2D::new(rect.origin.x + 8.0, rect.origin.y + 8.0)
            ),
            SelectHit::Row(*expect)
        );
        assert_eq!(
            font_picker_hit(
                &state,
                panel_rect(),
                visible_text(),
                &entries,
                Point2D::new(layout.search.origin.x + 8.0, layout.search.origin.y + 8.0)
            ),
            SelectHit::Inside
        );
        assert_eq!(
            font_picker_hit(
                &state,
                panel_rect(),
                visible_text(),
                &entries,
                Point2D::new(layout.popup.origin.x - 1.0, layout.popup.origin.y)
            ),
            SelectHit::Outside
        );
    }

    #[test]
    fn pressed_font_picker_entry_uses_shared_select_feedback() {
        let entries = font_picker_entries(&[], "");
        let layout = font_picker_layout(panel_rect(), visible_text(), &entries, 0.0).unwrap();
        let (entry_index, entry_rect) = layout
            .rows
            .iter()
            .find_map(|(row, rect)| match row {
                FontPickerRow::Entry(i) => Some((*i, *rect)),
                _ => None,
            })
            .unwrap();
        let mut state = open_state(0.0);
        state.pressed = Some(entry_index);
        let theme = Theme::dark();
        let expected = theme.button_hover.with_alpha(theme.button_hover.a * 1.8);
        let expected_rect = Rect {
            origin: Point2D::new(entry_rect.origin.x + 2.0, entry_rect.origin.y),
            size: Point2D::new(entry_rect.size.x - 4.0, entry_rect.size.y),
        };
        let mut backend = RoundFillBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_font_picker(
            &mut cx,
            &theme,
            panel_rect(),
            visible_text(),
            op_editor_core::Locale::EnUs,
            &entries,
            "",
            &state,
            "Not A Listed Font",
        );

        assert!(
            backend.fills.iter().any(|(fill, radius, color)| {
                *fill == expected_rect && (*radius - 5.0).abs() < 0.01 && *color == expected
            }),
            "pressed font-picker entry should paint the shared pressed feedback token"
        );
    }
}
