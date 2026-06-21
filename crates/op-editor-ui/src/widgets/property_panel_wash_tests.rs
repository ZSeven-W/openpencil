//! Tests for `property_panel::action_wash_rect` — the ④ fit-content hover
//! wash for the Size checkboxes and the alignment segmented buttons. Split
//! into its own file so `property_panel_tests.rs` stays under the 800-line cap.
//!
//! The Size-section checkboxes and the alignment segmented buttons live in
//! half-width / quarter-width walker cells, but their visible content (a
//! checkbox + short label, or a centred ~16px icon) fills only a fraction of
//! the cell. `action_wash_rect` shrinks the painted hover/press highlight to
//! hug that content plus a little L/R padding, while the (wider) walker rect
//! stays the hit target. The wash is clamped so a long localized label can't
//! bleed past its hit rect into a neighbour.

use super::property_panel::{action_wash_rect, PropertyPanelAction, TextAlignValue};
use super::property_panel_sections as sections;
use super::property_panel_test_support::CountingBackend;
use crate::{Point2D, Rect};
use op_editor_core::EditorState;

#[test]
fn action_wash_hugs_size_checkbox_content() {
    let ui = EditorState::new().editor_ui;
    let labels = sections::PropertyLabels::for_editor_ui(&ui);
    let mut backend = CountingBackend::default();
    // A half-width cell like the size-checkbox walker emits.
    let cell = Rect {
        origin: Point2D::new(100.0, 50.0),
        size: Point2D::new(120.0, 22.0),
    };
    let wash = action_wash_rect(
        &PropertyPanelAction::ToggleSizeFillWidth,
        cell,
        &labels,
        op_editor_core::Locale::EnUs,
        &mut backend,
    );
    // Hugs checkbox(16) + label, left-padded — never spans the full cell.
    assert!(
        wash.size.x < cell.size.x,
        "size-checkbox wash should be narrower than its {}px cell, got {}",
        cell.size.x,
        wash.size.x
    );
    // L/R padding: the wash starts a touch left of the checkbox box.
    assert!(
        wash.origin.x < cell.origin.x,
        "wash should start left of the checkbox (L padding), got {} vs {}",
        wash.origin.x,
        cell.origin.x
    );
    assert!((wash.size.y - cell.size.y).abs() < f32::EPSILON);
}

#[test]
fn action_wash_centers_on_align_icon() {
    let ui = EditorState::new().editor_ui;
    let labels = sections::PropertyLabels::for_editor_ui(&ui);
    let mut backend = CountingBackend::default();
    let cell = Rect {
        origin: Point2D::new(40.0, 80.0),
        size: Point2D::new(64.0, 28.0),
    };
    let wash = action_wash_rect(
        &PropertyPanelAction::SetTextAlign(TextAlignValue::Center),
        cell,
        &labels,
        op_editor_core::Locale::EnUs,
        &mut backend,
    );
    // ~16px icon + 2 × 6px padding = 28px, centred in the 64px cell.
    assert!(
        (wash.size.x - 28.0).abs() < 0.5,
        "align wash should hug the icon (~28px), got {}",
        wash.size.x
    );
    let cell_center = cell.origin.x + cell.size.x / 2.0;
    let wash_center = wash.origin.x + wash.size.x / 2.0;
    assert!(
        (wash_center - cell_center).abs() < 0.5,
        "align wash should be centred on the cell"
    );
}

#[test]
fn action_wash_passthrough_for_non_fit_actions() {
    let ui = EditorState::new().editor_ui;
    let labels = sections::PropertyLabels::for_editor_ui(&ui);
    let mut backend = CountingBackend::default();
    let cell = Rect {
        origin: Point2D::new(10.0, 10.0),
        size: Point2D::new(200.0, 30.0),
    };
    let wash = action_wash_rect(
        &PropertyPanelAction::CreateComponent,
        cell,
        &labels,
        op_editor_core::Locale::EnUs,
        &mut backend,
    );
    assert_eq!(
        wash, cell,
        "non size/align actions should keep their full walker rect"
    );
}

#[test]
fn action_wash_size_checkbox_clamped_to_cell_for_long_label() {
    // A long localized label (Russian) must never wash past the cell's right
    // edge into the adjacent column — the wash clamps to the (unchanged) hit
    // rect even when the content would overflow it.
    let mut state = EditorState::new();
    state.editor_ui.locale = op_editor_core::Locale::Ru;
    let labels = sections::PropertyLabels::for_editor_ui(&state.editor_ui);
    let mut backend = CountingBackend::default();
    // Narrow cell so the long label overflows and the clamp must engage.
    let cell = Rect {
        origin: Point2D::new(100.0, 50.0),
        size: Point2D::new(60.0, 22.0),
    };
    let wash = action_wash_rect(
        &PropertyPanelAction::ToggleSizeFillWidth,
        cell,
        &labels,
        op_editor_core::Locale::EnUs,
        &mut backend,
    );
    let cell_right = cell.origin.x + cell.size.x;
    assert!(
        wash.origin.x + wash.size.x <= cell_right + 0.01,
        "long-label wash must not exceed the cell right edge ({} > {})",
        wash.origin.x + wash.size.x,
        cell_right
    );
    // …and the clamp actually engaged (wash reaches the cell's right edge).
    assert!(
        (wash.origin.x + wash.size.x - cell_right).abs() < 0.01,
        "expected the long-label wash to clamp to the cell's right edge"
    );
}
