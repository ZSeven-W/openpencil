//! The tab row's text / icon modes: how three tabs share the rail at
//! each end of the 240–440 drag range, what a board-less document
//! drops, and the touch row's 44-point targets.
//!
//! Sibling of `slides_panel_tests.rs`, split out at the 800-line cap.

use super::*;

fn touch_tabs_of(width: f32, active: LeftPanelTab, row: &SlidesTabRow<'static>) -> SlidesPanelTabs {
    SlidesPanelTabs::new_touch(rail(width), active, row)
}

/// The row's geometry and hit-test at the two ends of the drag range:
/// three tabs at 240 (text mode in English) and at 440, every tab still
/// its own click target in row order.
#[test]
fn three_tabs_hit_test_across_the_drag_range() {
    for width in [240.0, 440.0] {
        let tabs = tabs_at(width, LeftPanelTab::Chat, &EN_ROW);
        assert!(!tabs.compact, "English keeps its words at {width}");
        let mid_y = tabs.row.origin.y + tabs.row.size.y / 2.0;
        let expected = [
            (tabs.chat, SlidesPanelTarget::ChatTab),
            (tabs.layers, SlidesPanelTarget::LayersTab),
            (tabs.slides, SlidesPanelTarget::SlidesTab),
        ];
        for (rect, target) in expected {
            assert!(rect.size.x > 0.0, "the tab exists at {width}: {rect:?}");
            let inside = Point2D::new(rect.origin.x + rect.size.x / 2.0, mid_y);
            assert_eq!(tabs.hit(inside), Some(target), "at {width}");
        }
        // The three rects run left to right inside the rail without
        // overlapping, each at its own label's width — they no longer
        // tile it, because equal shares are what made the row look like
        // a segmented control.
        let inner_w = width - 32.0;
        let laid = tabs.chat.size.x + tabs.layers.size.x + tabs.slides.size.x;
        assert!(laid <= inner_w + 0.01, "the tabs fit the rail at {width}");
        assert!(tabs.chat.origin.x + tabs.chat.size.x <= tabs.layers.origin.x);
        assert!(tabs.layers.origin.x + tabs.layers.size.x <= tabs.slides.origin.x);
    }
}

/// Vietnamese is the counterweight: at the chat-bumped default width its
/// labels do not fit three ways, so the row drops to icons — and at the
/// maximum width it gets its words back.
#[test]
fn a_rail_too_narrow_for_its_labels_falls_back_to_icons() {
    let tabs = tabs_at(240.0, LeftPanelTab::Slides, &VI_ROW);
    assert!(
        tabs.compact,
        "Vietnamese labels do not fit three ways across a 240 px rail"
    );
    // The active tab keeps `[icon label]`; the others shrink to glyphs.
    assert!(
        tabs.slides.size.x > tabs.layers.size.x,
        "the active tab is the one that keeps its label: {:?} vs {:?}",
        tabs.slides.size,
        tabs.layers.size
    );
    let inner_w = 240.0 - 8.0 * 2.0;
    assert!(
        tabs.chat.size.x + tabs.layers.size.x + tabs.slides.size.x <= inner_w + 0.01,
        "the tabs must not overhang the row"
    );
    // All three are still clickable, exactly where they paint.
    let mid_y = tabs.row.origin.y + tabs.row.size.y / 2.0;
    for (rect, target) in [
        (tabs.chat, SlidesPanelTarget::ChatTab),
        (tabs.layers, SlidesPanelTarget::LayersTab),
        (tabs.slides, SlidesPanelTarget::SlidesTab),
    ] {
        assert_eq!(
            tabs.hit(Point2D::new(rect.origin.x + rect.size.x / 2.0, mid_y)),
            Some(target)
        );
    }
    // And the widest rail gives the words back.
    assert!(
        !tabs_at(440.0, LeftPanelTab::Slides, &VI_ROW).compact,
        "Vietnamese keeps its words at 440"
    );
}

/// A document with no boards shows two tabs, not a dead Slides pill:
/// the row splits in halves and the slides rect is nobody's.
#[test]
fn a_boardless_document_drops_the_slides_tab_not_the_row() {
    let row = SlidesTabRow {
        slides_available: false,
        ..EN_ROW
    };
    let tabs = SlidesPanelTabs::new(PANEL, LeftPanelTab::Layers, &row);
    assert!(!tabs.compact);
    // Two tabs at their own label widths, not two halves.
    assert!(tabs.chat.size.x > 0.0 && tabs.layers.size.x > 0.0);
    assert!(tabs.chat.origin.x + tabs.chat.size.x <= tabs.layers.origin.x);
    assert!(
        tabs.layers.origin.x + tabs.layers.size.x < tabs.row.size.x,
        "two labels never fill the rail"
    );
    assert_eq!(tabs.slides, Rect::ZERO, "no slides rect to click");
    let mid_y = tabs.row.origin.y + tabs.row.size.y / 2.0;
    assert_eq!(
        tabs.hit(Point2D::new(
            tabs.chat.origin.x + tabs.chat.size.x / 2.0,
            mid_y
        )),
        Some(SlidesPanelTarget::ChatTab)
    );
    assert_eq!(
        tabs.hit(Point2D::new(
            tabs.layers.origin.x + tabs.layers.size.x / 2.0,
            mid_y
        )),
        Some(SlidesPanelTarget::LayersTab)
    );
    assert_eq!(tabs.hit(Point2D::new(236.0, mid_y)), None);
}

#[test]
fn touch_icon_tabs_keep_full_44_point_targets() {
    for active in [LeftPanelTab::Layers, LeftPanelTab::Slides] {
        let tabs = touch_tabs_of(180.0, active, &TOUCH_ROW);
        assert!(tabs.compact, "Vietnamese labels use icon mode at 180pt");
        assert_eq!(
            tabs.chat,
            Rect::ZERO,
            "touch never offers the chat tab in its rail"
        );
        for rect in [tabs.layers, tabs.slides] {
            assert!(
                rect.size.x >= 44.0 && rect.size.y >= 44.0,
                "every painted touch tab stays at least 44x44: {rect:?}"
            );
            assert!(
                tabs.row.contains(rect.origin)
                    && tabs.row.contains(Point2D::new(
                        rect.origin.x + rect.size.x,
                        rect.origin.y + rect.size.y,
                    )),
                "the full target stays inside the visible row: {rect:?}"
            );
        }
    }
}

#[test]
fn the_labelled_pill_follows_whichever_tab_is_active() {
    let on_chat = tabs_at(240.0, LeftPanelTab::Chat, &VI_ROW);
    let on_slides = tabs_at(240.0, LeftPanelTab::Slides, &VI_ROW);
    let on_layers = tabs_at(240.0, LeftPanelTab::Layers, &VI_ROW);
    assert!(on_chat.compact && on_slides.compact && on_layers.compact);
    assert!(on_chat.chat.size.x > on_chat.layers.size.x);
    assert!(on_slides.slides.size.x > on_slides.layers.size.x);
    assert!(on_layers.layers.size.x > on_layers.slides.size.x);
    // The un-labelled tabs are the same squares either way — each holds
    // one glyph and nothing else, wherever it happens to sit.
    assert!((on_chat.layers.size.x - on_layers.slides.size.x).abs() < 0.01);
}

/// Dragging the rail switches modes at the measured width, and never
/// switches back the wrong way: once the labels stop fitting they stay
/// not-fitting as the rail keeps narrowing.
#[test]
fn resizing_the_rail_switches_modes_monotonically() {
    let compact_at = |w: f32| tabs_at(w, LeftPanelTab::Slides, &VI_ROW).compact;
    assert!(compact_at(240.0), "at the minimum rail width, icons");
    assert!(!compact_at(440.0), "at the maximum, words");
    let mut seen_text = false;
    for step in 0..=40 {
        let width = 240.0 + step as f32 * 5.0;
        let compact = compact_at(width);
        if !compact {
            seen_text = true;
        }
        assert!(
            !(compact && seen_text),
            "the row went back to icons at {width} after already fitting its labels"
        );
    }
    assert!(seen_text);
}

/// English is short enough that three tabs fit at every rail width we
/// allow, so the fallback is inert for it today — that is the measured
/// answer, not an oversight, and it is what stops us shrinking a row
/// that has room.
#[test]
fn the_fit_rule_scales_with_the_tab_count() {
    let inner_w = 240.0 - 8.0 * 2.0;
    let slides_w = crate::widgets::top_bar_geometry::estimated_text_width("Slides", 12.0);
    assert!(
        text_tabs_fit(inner_w, 3, slides_w),
        "three English tabs fit the default rail"
    );
    assert!(
        !text_tabs_fit(inner_w, 5, slides_w),
        "five would not — the row compacts on tab count, not only on rail width"
    );
    // And the three we ship never compact anywhere in the resize range.
    for step in 0..=40 {
        let width = 240.0 + step as f32 * 5.0;
        assert!(
            !tabs_at(width, LeftPanelTab::Slides, &EN_ROW).compact,
            "English compacted at {width}, which means the estimate drifted"
        );
    }
}
