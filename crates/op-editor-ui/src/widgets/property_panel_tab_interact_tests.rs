//! Tab-strip generalization tests — `PropertyTab::Interact` rect
//! geometry, hit-test mapping, and experimental-flag gating. Sibling
//! of `property_panel_tests.rs` (kept separate so the strip's own
//! geometry assertions don't grow that file past the 800-line cap).

#![cfg(test)]

use crate::widgets::property_panel_sections::{tab_strip_hit, tab_strip_rects, PropertyLabels};
use crate::Point2D;
use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::PropertyTab;

fn labels() -> PropertyLabels {
    PropertyLabels::for_editor_ui(&EditorUiState::new())
}

#[test]
fn three_rects_are_adjacent_and_non_overlapping_when_interact_shown() {
    let rects = tab_strip_rects(&labels(), 100.0, 0.0, true);
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0].0, PropertyTab::Design);
    assert_eq!(rects[1].0, PropertyTab::Interact);
    assert_eq!(rects[2].0, PropertyTab::Code);
    // Adjacent: each rect starts where the previous one's 6px gutter ends.
    let (r0, r1, r2) = (rects[0].1, rects[1].1, rects[2].1);
    assert_eq!(r1.origin.x, r0.origin.x + r0.size.x + 6.0);
    assert_eq!(r2.origin.x, r1.origin.x + r1.size.x + 6.0);
    // Non-overlapping: each rect's span ends strictly before the next starts.
    assert!(r0.origin.x + r0.size.x < r1.origin.x);
    assert!(r1.origin.x + r1.size.x < r2.origin.x);
    // Same y / height for all three (single pinned row).
    assert_eq!(r0.origin.y, r1.origin.y);
    assert_eq!(r1.origin.y, r2.origin.y);
    assert_eq!(r0.size.y, 26.0);
    assert_eq!(r1.size.y, 26.0);
    assert_eq!(r2.size.y, 26.0);
}

#[test]
fn interact_absent_and_code_directly_follows_design_when_flag_off() {
    let rects = tab_strip_rects(&labels(), 100.0, 0.0, false);
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].0, PropertyTab::Design);
    assert_eq!(rects[1].0, PropertyTab::Code);
    let (r0, r1) = (rects[0].1, rects[1].1);
    assert_eq!(r1.origin.x, r0.origin.x + r0.size.x + 6.0);
}

#[test]
fn tab_strip_hit_maps_clicks_onto_all_three_tabs() {
    let rects = tab_strip_rects(&labels(), 100.0, 0.0, true);
    for (tab, rect) in &rects {
        let center = Point2D::new(
            rect.origin.x + rect.size.x / 2.0,
            rect.origin.y + rect.size.y / 2.0,
        );
        assert_eq!(
            tab_strip_hit(&labels(), 100.0, 0.0, center, true),
            Some(*tab)
        );
    }
    // A click at the Interact tab's rect misses entirely when the
    // flag is off (only 2 rects exist, so the x-coordinate the
    // 3-tab layout used for Interact now falls in the gutter/Code).
    let interact_rect = rects[1].1;
    let interact_center = Point2D::new(
        interact_rect.origin.x + interact_rect.size.x / 2.0,
        interact_rect.origin.y + interact_rect.size.y / 2.0,
    );
    assert_ne!(
        tab_strip_hit(&labels(), 100.0, 0.0, interact_center, false),
        Some(PropertyTab::Interact)
    );
}
