//! Layer-collapse / panel-visibility / validation mutator tests.

use super::support::three_rects;
use crate::node_id::NodeId;
use crate::test_support::{rect, state_with};

#[test]
fn toggle_node_collapsed_inserts_then_removes() {
    let mut s = three_rects();
    let id = NodeId::new("n1");
    assert!(!s.is_node_collapsed(&id));
    // First toggle collapses (returns true = now collapsed).
    assert!(s.toggle_node_collapsed(&id));
    assert!(s.is_node_collapsed(&id));
    // Second toggle expands (returns false = now expanded).
    assert!(!s.toggle_node_collapsed(&id));
    assert!(!s.is_node_collapsed(&id));
}

#[test]
fn toggle_node_collapsed_none_id_is_noop() {
    let mut s = three_rects();
    assert!(!s.toggle_node_collapsed(&NodeId::NONE));
    assert!(s.editor_ui.collapsed_layers.is_empty());
}

// --- Panel visibility predicates (Gap 4) ----------------------------

#[test]
fn property_panel_visible_tracks_selection() {
    let mut s = three_rects();
    s.clear_selection();
    assert!(!s.property_panel_visible());
    s.editor_ui.property_tab = crate::PropertyTab::Interact;
    assert!(!s.property_panel_visible());
    s.editor_ui.property_tab = crate::PropertyTab::Design;
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.property_panel_visible());
    // A selection of an id that does not resolve is not visible.
    s.set_single_selection(NodeId::new("nope"));
    assert!(!s.property_panel_visible());
}

#[test]
fn right_rail_visible_true_on_selection_only() {
    let mut s = three_rects();
    s.clear_selection();
    // No selection → hidden.
    assert!(!s.right_rail_visible());
    // The VariablesPanel is a floating canvas overlay, not a right-rail panel.
    s.editor_ui.variables_panel_open = true;
    assert!(!s.right_rail_visible());
    s.editor_ui.variables_panel_open = false;
    // Selection makes it visible.
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.right_rail_visible());
}

#[test]
fn right_rail_stays_visible_on_code_tab_without_selection() {
    let mut s = three_rects();
    s.clear_selection();
    // Design tab + no selection → hidden (baseline).
    assert!(!s.right_rail_visible());
    // The Code tab is selection-independent (TS falls back to the active
    // page's children), so the rail must stay open with no selection.
    s.editor_ui.property_tab = crate::PropertyTab::Code;
    assert!(s.property_panel_visible());
    assert!(s.right_rail_visible());
    // Back to Design without a selection → hidden again.
    s.editor_ui.property_tab = crate::PropertyTab::Design;
    assert!(!s.right_rail_visible());
}

#[test]
fn compact_legacy_code_tab_obeys_design_selection_visibility() {
    let mut s = three_rects();
    s.clear_selection();
    s.editor_ui.touch = true;
    s.editor_ui.size_class = crate::size_class::EditorSizeClass::Compact;
    s.editor_ui.property_tab = crate::PropertyTab::Code;

    assert_eq!(
        s.editor_ui.effective_property_tab(),
        crate::PropertyTab::Design
    );
    assert!(!s.property_panel_visible());

    s.set_single_selection(NodeId::new("n1"));
    assert!(s.property_panel_visible());
}

#[test]
fn validate_catches_duplicate_ids() {
    let s = state_with(vec![
        rect("dup", "A", 0.0, 0.0, 10.0, 10.0),
        rect("dup", "B", 0.0, 0.0, 10.0, 10.0),
    ]);
    assert!(s.validate().is_err());
}
