//! Geometry + hit-test tests for the rail's bottom action bar and the
//! export dropdown it opens.
//!
//! Everything here is asserted against the SAME `SlidesActionLayout` the
//! paint pass consumes, because that shared struct is the mechanism that
//! keeps a button's picture and its click target the same rect.

use super::*;
use crate::widgets::slides_panel::{
    SlidesPanelLayout, SlidesPanelTabs, SlidesTabRow, DEFAULT_BOARD_ASPECT,
};
use op_editor_core::LeftPanelTab;

const PANEL: Rect = Rect {
    origin: Point2D { x: 0.0, y: 48.0 },
    size: Point2D { x: 240.0, y: 700.0 },
};

fn bar(actions: SlidesActionState) -> SlidesActionLayout {
    SlidesActionLayout::new(PANEL, PANEL.origin.y + 36.0, actions)
}

fn open_menu(selected: usize, supported: bool) -> SlidesActionLayout {
    bar(SlidesActionState {
        export_menu_open: true,
        selected_slides: selected,
        selected_export_supported: supported,
    })
}

fn centre(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

/// The whole point of the bar: it is pinned to the rail's bottom edge,
/// side by side, and both halves are reachable.
#[test]
fn the_bar_sits_on_the_rails_bottom_edge_with_two_buttons_side_by_side() {
    let l = bar(SlidesActionState::default());
    assert_eq!(
        l.bar.origin.y + l.bar.size.y,
        PANEL.origin.y + PANEL.size.y,
        "the bar's bottom IS the rail's bottom"
    );
    assert_eq!(l.bar.size.y, ACTION_BAR_HEIGHT);
    assert_eq!(l.bar.size.x, PANEL.size.x);

    assert!(
        (l.present.size.x - l.export.size.x).abs() < 0.01,
        "the two buttons split the bar evenly: {:?} vs {:?}",
        l.present.size,
        l.export.size
    );
    assert!(
        l.export.origin.x > l.present.origin.x + l.present.size.x,
        "Present is on the left, Export on the right, with a gap between"
    );
    // Both inside the bar, vertically centred in it.
    for button in [l.present, l.export] {
        assert!(button.origin.y >= l.bar.origin.y);
        assert!(button.origin.y + button.size.y <= l.bar.origin.y + l.bar.size.y);
    }
    assert_eq!(
        l.button_at(centre(l.present)),
        Some(SlidesPanelTarget::Present)
    );
    assert_eq!(
        l.button_at(centre(l.export)),
        Some(SlidesPanelTarget::ExportMenu)
    );
    // The gap between them belongs to neither.
    let gap = Point2D::new(
        (l.present.origin.x + l.present.size.x + l.export.origin.x) / 2.0,
        centre(l.present).y,
    );
    assert_eq!(l.button_at(gap), None);
}

/// **Opens upward.** The bar is already on the bottom edge, so there is
/// no room below it; the menu grows towards the list and covers it.
#[test]
fn the_export_menu_opens_upward_and_stays_inside_the_rail() {
    let closed = bar(SlidesActionState::default());
    assert_eq!(closed.menu, None, "no menu until it is opened");

    let l = open_menu(2, true);
    let menu = l.menu.expect("the menu is laid out while open");
    assert!(
        menu.origin.y + menu.size.y <= l.bar.origin.y,
        "the menu's bottom clears the bar it hangs off: menu {menu:?}, bar {:?}",
        l.bar
    );
    assert!(menu.origin.y < l.bar.origin.y, "and it grew UP, not down");
    assert!(
        menu.origin.y >= PANEL.origin.y + 36.0,
        "and never over the tab row: menu top {} vs list top {}",
        menu.origin.y,
        PANEL.origin.y + 36.0
    );
    assert!(
        menu.origin.x >= PANEL.origin.x
            && menu.origin.x + menu.size.x <= PANEL.origin.x + PANEL.size.x,
        "and stays inside the rail horizontally"
    );

    // Two rows, stacked, both inside the menu.
    let first = l.menu_row_rect(0).expect("row 0");
    let second = l.menu_row_rect(1).expect("row 1");
    assert_eq!(l.menu_row_rect(2), None, "there are exactly two rows");
    assert!(second.origin.y > first.origin.y);
    for row in [first, second] {
        assert!(row.origin.y >= menu.origin.y);
        assert!(row.origin.y + row.size.y <= menu.origin.y + menu.size.y);
    }
}

/// A rail too short to hold the menu pushes it DOWN over the
/// thumbnails rather than up over the tab row — the tab row is how the
/// user gets back to the Layers tree and must stay reachable.
#[test]
fn a_short_rail_clamps_the_menu_to_the_list_top_rather_than_over_the_tabs() {
    let short = Rect {
        origin: Point2D::new(0.0, 48.0),
        size: Point2D::new(240.0, 140.0),
    };
    let list_top = short.origin.y + 36.0;
    let l = SlidesActionLayout::new(
        short,
        list_top,
        SlidesActionState {
            export_menu_open: true,
            selected_slides: 1,
            selected_export_supported: true,
        },
    );
    let menu = l.menu.expect("still laid out");
    assert!(
        menu.origin.y >= list_top,
        "clamped to the list top, not floated over the tab row: {}",
        menu.origin.y
    );
}

#[test]
fn menu_rows_hit_test_where_they_paint() {
    let l = open_menu(3, true);
    assert_eq!(
        l.menu_row_at(centre(l.menu_row_rect(0).unwrap())),
        Some(SlidesPanelTarget::ExportAllSlides)
    );
    assert_eq!(
        l.menu_row_at(centre(l.menu_row_rect(1).unwrap())),
        Some(SlidesPanelTarget::ExportSelectedSlides)
    );
    // The menu's own padding is chrome, not a row.
    let menu = l.menu.unwrap();
    let padding = Point2D::new(menu.origin.x + 4.0, menu.origin.y + 2.0);
    assert!(l.over_menu(padding), "padding is still the menu's surface");
    assert_eq!(l.menu_row_at(padding), None);
    // Off the menu entirely.
    assert!(!l.over_menu(Point2D::new(menu.origin.x - 5.0, menu.origin.y)));
}

/// The count is live at every value, including zero — the row states
/// what it would act on even while it cannot act.
#[test]
fn the_selected_row_carries_the_live_selection_count() {
    for selected in [0usize, 1, 4, 17] {
        let l = open_menu(selected, true);
        assert_eq!(l.selected_slides, selected);
    }
}

/// Zero selected slides disables the row: it cannot be hovered and
/// cannot be activated, so a click that would export nothing is never
/// offered in the first place.
#[test]
fn the_selected_row_is_disabled_with_nothing_selected() {
    let none = open_menu(0, true);
    assert!(!none.selected_enabled);
    assert_eq!(
        none.menu_row_at(centre(none.menu_row_rect(1).unwrap())),
        None,
        "a disabled row is not a target"
    );
    // Its sibling is unaffected — "export all" needs no selection.
    assert_eq!(
        none.menu_row_at(centre(none.menu_row_rect(0).unwrap())),
        Some(SlidesPanelTarget::ExportAllSlides)
    );

    let some = open_menu(2, true);
    assert!(some.selected_enabled);
    assert_eq!(
        some.menu_row_at(centre(some.menu_row_rect(1).unwrap())),
        Some(SlidesPanelTarget::ExportSelectedSlides)
    );
}

/// The second reason the row disables: no subset exporter exists to
/// route it to. Both gates are independent — a selection alone is not
/// enough while the capability is missing.
#[test]
fn the_selected_row_is_disabled_without_a_subset_exporter() {
    let l = open_menu(3, false);
    assert!(!l.selected_enabled);
    assert_eq!(
        l.menu_row_at(centre(l.menu_row_rect(1).unwrap())),
        None,
        "an unwired row is not a target"
    );
    assert_eq!(
        l.selected_slides, 3,
        "but the count still reports what a working row would take"
    );
}

/// The capability this build ships with. It is wired: turning it back off
/// should be a deliberate act with a reason, not a refactor's side effect,
/// so the assertion is here to be argued with rather than quietly deleted.
#[test]
fn the_subset_exporter_is_wired_in_this_build() {
    assert!(
        selected_slides_export_supported(),
        "the row is live: `FileAction::ExportDeckPdfSelection` reaches \
         `export_pdf::export_deck_pdf_boards` on both hosts"
    );
}

/// The bar does not scroll, so the list band has to end where the bar
/// begins — otherwise the last thumbnail would sit under it.
#[test]
fn the_list_band_stops_where_the_bar_starts() {
    let l = SlidesPanelLayout::new(
        PANEL,
        SlidesPanelTabs::new(
            PANEL,
            LeftPanelTab::Slides,
            &SlidesTabRow {
                chat_label: "Chat",
                layers_label: "Layers",
                slides_label: "Slides",
                chat_available: true,
                slides_available: true,
            },
        ),
        &[DEFAULT_BOARD_ASPECT; 12],
        0.0,
        SlidesActionState::default(),
    )
    .expect("a 240x700 rail fits the list");
    assert!(
        l.list.origin.y + l.list.size.y <= l.actions.bar.origin.y + 0.01,
        "the scrolling band {:?} runs under the bar at {:?}",
        l.list,
        l.actions.bar
    );

    // Scrolled to the very end, the last row's visible slice still stops
    // at the band's edge rather than reaching into the bar.
    let scrolled = SlidesPanelLayout::new(
        PANEL,
        SlidesPanelTabs::new(
            PANEL,
            LeftPanelTab::Slides,
            &SlidesTabRow {
                chat_label: "Chat",
                layers_label: "Layers",
                slides_label: "Slides",
                chat_available: true,
                slides_available: true,
            },
        ),
        &[DEFAULT_BOARD_ASPECT; 12],
        10_000.0,
        SlidesActionState::default(),
    )
    .expect("a 240x700 rail fits the list");
    let last = scrolled
        .visible_thumb_rect(11)
        .expect("the last row is on screen at the end of the scroll");
    assert!(
        last.origin.y + last.size.y <= scrolled.actions.bar.origin.y + 0.01,
        "the last thumbnail {last:?} is covered by the bar at {:?}",
        scrolled.actions.bar
    );
}
