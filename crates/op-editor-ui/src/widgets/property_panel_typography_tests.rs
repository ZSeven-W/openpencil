//! Font-family picker unit tests — split out of
//! `property_panel_typography.rs` to keep that module under the
//! 800-line ceiling. A `#[path]` submodule so it retains `super::`
//! access to the parent's private constants + helpers.

use super::*;
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, RenderBackend, TextLayout};

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

/// Desktop capability — the Import row + imported group render.
const ALLOW_IMPORT: bool = true;

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
    let entries = font_picker_entries(&[], &[], "");
    assert_eq!(
        entries.len(),
        BUNDLED_FONT_FAMILIES.len() + FALLBACK_SYSTEM_FONTS.len()
    );
    assert!(entries[0].bundled);
    assert!(!entries[0].imported);
    assert_eq!(entries[0].family, "Inter");
    let first_system = entries.iter().find(|e| !e.bundled).unwrap();
    assert_eq!(first_system.family, "Arial");
}

#[test]
fn entries_use_host_enumeration_and_filter_by_search() {
    let system = vec!["Avenir".to_string(), "Menlo".to_string()];
    let entries = font_picker_entries(&[], &system, "");
    assert_eq!(entries.len(), BUNDLED_FONT_FAMILIES.len() + 2);
    // Case-insensitive substring filter (TS `filtered` memo).
    let filtered = font_picker_entries(&[], &system, "men");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].family, "Menlo");
    assert!(!filtered[0].bundled);
    // No matches → empty (panel paints the no-results row).
    assert!(font_picker_entries(&[], &system, "zzz").is_empty());
}

#[test]
fn imported_group_is_ordered_first_and_filters_by_search() {
    let imported = vec!["My Brand Sans".to_string(), "My Serif".to_string()];
    let system = vec!["Avenir".to_string()];
    let entries = font_picker_entries(&imported, &system, "");
    // Imported entries lead, flagged imported (never bundled).
    assert!(entries[0].imported);
    assert!(!entries[0].bundled);
    assert_eq!(entries[0].family, "My Brand Sans");
    assert!(entries[1].imported);
    assert_eq!(entries[1].family, "My Serif");
    // Bundled group starts right after the imported ones.
    assert!(entries[2].bundled);
    // Same substring filter applies to the imported group.
    let filtered = font_picker_entries(&imported, &system, "serif");
    assert_eq!(filtered.len(), 1);
    assert!(filtered[0].imported);
    assert_eq!(filtered[0].family, "My Serif");
}

#[test]
fn display_name_strips_fallback_stack_and_quotes() {
    assert_eq!(display_font_family("Arial, sans-serif"), "Arial");
    assert_eq!(display_font_family("\"DM Sans\", sans-serif"), "DM Sans");
    assert_eq!(display_font_family("Inter"), "Inter");
}

#[test]
fn entry_hit_maps_back_to_the_clicked_family() {
    let entries = font_picker_entries(&[], &[], "");
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &entries, ALLOW_IMPORT, 0.0).unwrap();
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
            ALLOW_IMPORT,
            &open_state(0.0),
            centre
        ),
        Some(*expect)
    );
    assert!(matches!(
        font_picker_action_at(
            panel_rect(),
            visible_text(),
            &entries,
            ALLOW_IMPORT,
            &open_state(0.0),
            centre
        ),
        Some(PropertyPanelAction::SetFontFamilyIndex(i)) if i == *expect
    ));
}

#[test]
fn remove_x_hit_wins_over_entry_and_maps_to_remove_imported_font() {
    let imported = vec!["My Brand Sans".to_string()];
    let entries = font_picker_entries(&imported, &[], "");
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &entries, ALLOW_IMPORT, 0.0).unwrap();
    // The remove-x sub-rect resolves to RemoveImportedFont.
    let (row, rrect) = layout
        .rows
        .iter()
        .find(|(row, _)| matches!(row, FontPickerRow::RemoveEntry(_)))
        .unwrap();
    let FontPickerRow::RemoveEntry(remove_idx) = row else {
        unreachable!()
    };
    let x_centre = Point2D::new(
        rrect.origin.x + rrect.size.x / 2.0,
        rrect.origin.y + rrect.size.y / 2.0,
    );
    assert!(matches!(
        font_picker_action_at(
            panel_rect(),
            visible_text(),
            &entries,
            ALLOW_IMPORT,
            &open_state(0.0),
            x_centre
        ),
        Some(PropertyPanelAction::RemoveImportedFont(i)) if i == *remove_idx
    ));
    // A click on the imported entry body (left of the x) still selects.
    let (erow, erect) = layout
        .rows
        .iter()
        .find(|(row, _)| matches!(row, FontPickerRow::Entry(_)))
        .unwrap();
    let FontPickerRow::Entry(entry_idx) = erow else {
        unreachable!()
    };
    let body = Point2D::new(erect.origin.x + 12.0, erect.origin.y + erect.size.y / 2.0);
    assert!(matches!(
        font_picker_action_at(
            panel_rect(),
            visible_text(),
            &entries,
            ALLOW_IMPORT,
            &open_state(0.0),
            body
        ),
        Some(PropertyPanelAction::SetFontFamilyIndex(i)) if i == *entry_idx
    ));
}

#[test]
fn import_action_row_is_always_present_and_maps_to_import_font() {
    // Even a no-results search keeps the bottom import action.
    let empty = font_picker_entries(&[], &[], "zzzznomatch");
    assert!(empty.is_empty());
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &empty, ALLOW_IMPORT, 0.0).unwrap();
    assert!(layout
        .rows
        .iter()
        .any(|(r, _)| matches!(r, FontPickerRow::NoResults)));
    let (_, arect) = layout
        .rows
        .iter()
        .find(|(r, _)| matches!(r, FontPickerRow::ImportAction))
        .expect("import action always present");
    let centre = Point2D::new(
        arect.origin.x + arect.size.x / 2.0,
        arect.origin.y + arect.size.y / 2.0,
    );
    assert_eq!(
        font_picker_action_at(
            panel_rect(),
            visible_text(),
            &empty,
            ALLOW_IMPORT,
            &open_state(0.0),
            centre
        ),
        Some(PropertyPanelAction::ImportFont)
    );
}

#[test]
fn no_import_capability_omits_the_import_row_entirely() {
    // Web host (allow_import = false): the layout must carry NO
    // ImportAction row, and a click where it would sit yields no
    // ImportFont action (no dead control on web).
    let entries = font_picker_entries(&[], &[], "playfair");
    let layout = font_picker_layout(panel_rect(), visible_text(), &entries, false, 0.0).unwrap();
    assert!(
        !layout
            .rows
            .iter()
            .any(|(r, _)| matches!(r, FontPickerRow::ImportAction)),
        "Import row must be absent when the host cannot import"
    );
    // Sweep the whole viewport: no click resolves to ImportFont.
    let vp = layout.viewport;
    for step in 0..=20 {
        let y = vp.origin.y + (vp.size.y * step as f32 / 20.0);
        let p = Point2D::new(vp.origin.x + vp.size.x / 2.0, y);
        assert_ne!(
            font_picker_action_at(
                panel_rect(),
                visible_text(),
                &entries,
                false,
                &open_state(0.0),
                p
            ),
            Some(PropertyPanelAction::ImportFont)
        );
    }
}

#[test]
fn list_viewport_caps_at_max_height_and_scrolls() {
    // 22 entries (11 + 11 fallback) + headers + import row exceed the
    // viewport, so the dropdown must report scrollable overflow.
    let entries = font_picker_entries(&[], &[], "");
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &entries, ALLOW_IMPORT, 0.0).unwrap();
    assert!(layout.viewport.size.y <= MAX_LIST_VIEWPORT_H + 0.01);
    assert!(layout.max_scroll > 0.0);
    // A row scrolled above the viewport is not hittable.
    let entries2 = font_picker_entries(&[], &[], "");
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
        ALLOW_IMPORT,
        &open_state(layout.max_scroll),
        centre,
    );
    assert_ne!(hit, Some(0));
}

#[test]
fn picker_contains_covers_search_row() {
    let entries = font_picker_entries(&[], &[], "");
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &entries, ALLOW_IMPORT, 0.0).unwrap();
    let in_search = Point2D::new(layout.search.origin.x + 5.0, layout.search.origin.y + 5.0);
    assert!(font_picker_contains(
        &open_state(0.0),
        panel_rect(),
        visible_text(),
        &entries,
        ALLOW_IMPORT,
        in_search
    ));
    // ... and a search-row click maps to NO action (swallowed,
    // keeps the popup open).
    assert!(font_picker_action_at(
        panel_rect(),
        visible_text(),
        &entries,
        ALLOW_IMPORT,
        &open_state(0.0),
        in_search
    )
    .is_none());
}

#[test]
fn font_picker_hit_uses_shared_select_state_protocol() {
    let entries = font_picker_entries(&[], &[], "");
    let state = SelectState {
        open: true,
        ..Default::default()
    };
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &entries, ALLOW_IMPORT, 0.0).unwrap();
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
            ALLOW_IMPORT,
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
            ALLOW_IMPORT,
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
            ALLOW_IMPORT,
            Point2D::new(layout.popup.origin.x - 1.0, layout.popup.origin.y)
        ),
        SelectHit::Outside
    );
}

#[test]
fn pressed_font_picker_entry_uses_shared_select_feedback() {
    let entries = font_picker_entries(&[], &[], "");
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &entries, ALLOW_IMPORT, 0.0).unwrap();
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
        ALLOW_IMPORT,
        "",
        &state,
        false,
        "Not A Listed Font",
        0,
    );

    assert!(
        backend.fills.iter().any(|(fill, radius, color)| {
            *fill == expected_rect && (*radius - 5.0).abs() < 0.01 && *color == expected
        }),
        "pressed font-picker entry should paint the shared pressed feedback token"
    );
}

#[test]
fn import_action_at_returns_true_over_the_import_row() {
    // The bottom "Import font…" row is hoverable (BUG 2): a point at its
    // centre resolves to `true`, while a point on a plain entry does not.
    // A narrow search keeps the list short so the import row (bottom of
    // content) stays inside the viewport at scroll 0.
    let entries = font_picker_entries(&[], &[], "playfair");
    let layout =
        font_picker_layout(panel_rect(), visible_text(), &entries, ALLOW_IMPORT, 0.0).unwrap();
    let (_, import_rect) = layout
        .rows
        .iter()
        .find(|(r, _)| matches!(r, FontPickerRow::ImportAction))
        .expect("import action always present");
    let import_centre = Point2D::new(
        import_rect.origin.x + import_rect.size.x / 2.0,
        import_rect.origin.y + import_rect.size.y / 2.0,
    );
    assert!(
        font_picker_import_action_at(
            panel_rect(),
            visible_text(),
            &entries,
            ALLOW_IMPORT,
            &open_state(0.0),
            import_centre
        ),
        "cursor over the Import row must report an import hover"
    );
    // A plain entry row must NOT read as the import row.
    let (_, entry_rect) = layout
        .rows
        .iter()
        .find(|(r, _)| matches!(r, FontPickerRow::Entry(_)))
        .expect("at least one entry");
    let entry_centre = Point2D::new(
        entry_rect.origin.x + entry_rect.size.x / 2.0,
        entry_rect.origin.y + entry_rect.size.y / 2.0,
    );
    assert!(!font_picker_import_action_at(
        panel_rect(),
        visible_text(),
        &entries,
        ALLOW_IMPORT,
        &open_state(0.0),
        entry_centre
    ));
    // Web host (no import capability) never reports an import hover.
    assert!(!font_picker_import_action_at(
        panel_rect(),
        visible_text(),
        &entries,
        false,
        &open_state(0.0),
        import_centre
    ));
}
