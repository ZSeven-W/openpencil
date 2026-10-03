//! Tests for the mobile bottom-breathing-room echo. Every case runs the real
//! jian layout through `geometry_diagnostics`, so the assertions are about
//! RESOLVED geometry, not authored numbers.
//!
//! The bottom-nav exception is deliberately name-blind: the negative cases
//! below include a nav bar named nothing nav-ish, and the positive cases
//! include a trailing section that a name heuristic would have mistaken for
//! chrome.

use crate::geometry_validation::{
    geometry_diagnostics, repair_mobile_bottom_breathing,
    repair_mobile_bottom_breathing_for_all_roots,
};
use op_editor_core::PenNodeExt;
use serde_json::json;

const ECHO: &str = "mobile-bottom-flush";

fn diagnostics_for(root: serde_json::Value) -> Vec<String> {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [root]
    }))
    .expect("valid document");
    let state = op_editor_core::EditorState::from_document(doc);
    geometry_diagnostics(&state)
}

fn echoed(root: serde_json::Value) -> bool {
    diagnostics_for(root).iter().any(|d| d.contains(ECHO))
}

fn state_for(root: serde_json::Value) -> op_editor_core::EditorState {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [root]
    }))
    .expect("valid document");
    op_editor_core::EditorState::from_document(doc)
}

fn repair(root: serde_json::Value) -> (op_editor_core::EditorState, bool) {
    let mut state = state_for(root);
    let changed = {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        repair_mobile_bottom_breathing(&mut sink, "root")
    };
    (state, changed)
}

fn resolved_direct_child_gap(state: &op_editor_core::EditorState) -> f64 {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let page = scene.active_page().expect("active page");
    let root = page.children.first().expect("root scene");
    let root_bounds = root.aggregate_bounds();
    let content_bottom = root
        .children
        .iter()
        .map(|child| {
            let bounds = child.aggregate_bounds();
            f64::from(bounds.origin.y + bounds.size.y)
        })
        .fold(f64::NEG_INFINITY, f64::max);
    f64::from(root_bounds.origin.y + root_bounds.size.y) - content_bottom
}

fn fixed_gap_screen(gap: f64) -> serde_json::Value {
    json!({
        "type": "frame", "id": "root", "name": "Mobile Screen",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "clipContent": false,
        "children": [
            { "type": "frame", "id": "body", "name": "Body",
              "width": "fill_container", "height": 844.0 - gap }
        ]
    })
}

/// A movie-detail screen: no bottom nav, last section ends exactly at the
/// screen bottom. This is the reported failure (2026-07-28 screenshot).
fn detail_screen(trailing_bottom_padding: f64) -> serde_json::Value {
    json!({
        "type": "frame", "id": "root", "name": "Movie Detail",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "hero", "name": "Hero",
              "width": "fill_container", "height": 520 },
            { "type": "frame", "id": "cast", "name": "Cast",
              "width": "fill_container", "height": 324,
              "layout": "vertical", "padding": [0, 24, trailing_bottom_padding, 24] }
        ]
    })
}

#[test]
fn detail_screen_without_a_bottom_nav_echoes_the_flush_edge() {
    let issues = diagnostics_for(detail_screen(0.0));
    let echo = issues
        .iter()
        .find(|d| d.contains(ECHO))
        .unwrap_or_else(|| panic!("expected the flush-bottom echo, got {issues:?}"));
    assert!(echo.contains("Movie Detail"), "{echo}");
    assert!(
        echo.contains("no bottom navigation bar") && echo.contains("24-32px"),
        "the echo must state the fact and the remedy: {echo}"
    );
}

#[test]
fn a_screen_with_bottom_breathing_room_is_silent() {
    // Same screen, 32px of trailing room — the shorter last section leaves the
    // gap the corpus rule asks for.
    let root = json!({
        "type": "frame", "id": "root", "name": "Movie Detail",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "hero", "name": "Hero",
              "width": "fill_container", "height": 520 },
            { "type": "frame", "id": "cast", "name": "Cast",
              "width": "fill_container", "height": 292 }
        ]
    });
    assert!(!echoed(root), "32px of room must not be reported");
}

#[test]
fn a_screen_closed_by_a_bottom_nav_shape_is_silent_without_reading_its_name() {
    // No `role`, and a name a keyword heuristic would never match — the nav is
    // recognised purely from its resolved shape: full-width trailing band in
    // the nav height range with five evenly-sized tap targets.
    let tab = |id: &str| {
        json!({ "type": "frame", "id": id, "width": "fill_container", "height": 68,
                "layout": "vertical" })
    };
    let root = json!({
        "type": "frame", "id": "root", "name": "Home",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "feed", "name": "Feed",
              "width": "fill_container", "height": 776 },
            { "type": "frame", "id": "chrome", "name": "Chrome Strip",
              "width": "fill_container", "height": 68, "layout": "horizontal",
              "children": [tab("t1"), tab("t2"), tab("t3"), tab("t4"), tab("t5")] }
        ]
    });
    assert!(
        !echoed(root),
        "a nav-shaped trailing band closes the screen legitimately"
    );
}

#[test]
fn a_role_tagged_bottom_nav_is_silent() {
    let root = json!({
        "type": "frame", "id": "root", "name": "Home",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "feed", "name": "Feed",
              "width": "fill_container", "height": 774 },
            { "type": "frame", "id": "nav", "name": "Nav",
              "role": "bottom-tab-bar", "width": "fill_container", "height": 70 }
        ]
    });
    assert!(!echoed(root), "the authored role closes the screen");
}

#[test]
fn a_trailing_band_that_is_not_nav_shaped_still_echoes() {
    // A full-width 68px trailing band with only TWO children is a footer CTA
    // row, not a tab bar — the screen still ends flush and still gets the echo.
    // A name heuristic ("Bottom Bar") would have silenced this one.
    let half = |id: &str| {
        json!({ "type": "frame", "id": id, "width": "fill_container", "height": 68,
                "layout": "vertical" })
    };
    let root = json!({
        "type": "frame", "id": "root", "name": "Checkout",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "summary", "name": "Summary",
              "width": "fill_container", "height": 776 },
            { "type": "frame", "id": "bar", "name": "Bottom Bar",
              "width": "fill_container", "height": 68, "layout": "horizontal",
              "children": [half("a"), half("b")] }
        ]
    });
    assert!(echoed(root), "a two-target footer row is not a tab bar");
}

#[test]
fn an_uneven_trailing_row_is_not_treated_as_a_tab_bar() {
    // A 20/80 split is a price + CTA row, not evenly-distributed tab targets.
    let root = json!({
        "type": "frame", "id": "root", "name": "Product",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "children": [
            { "type": "frame", "id": "body", "name": "Body",
              "width": "fill_container", "height": 776 },
            { "type": "frame", "id": "bar", "name": "Action Bar",
              "width": "fill_container", "height": 68, "layout": "horizontal",
              "children": [
                  { "type": "frame", "id": "price", "width": 78, "height": 68 },
                  { "type": "frame", "id": "cta", "width": 312, "height": 68 },
                  { "type": "frame", "id": "pad", "width": 0, "height": 68 }
              ] }
        ]
    });
    assert!(echoed(root), "unequal targets are not a tab bar");
}

#[test]
fn a_desktop_width_root_is_out_of_scope() {
    let root = json!({
        "type": "frame", "id": "root", "name": "Dashboard",
        "width": 1440, "height": 900, "layout": "vertical", "gap": 0,
        "children": [{ "type": "frame", "id": "body", "name": "Body",
                       "width": "fill_container", "height": 900 }]
    });
    assert!(!echoed(root), "the rule is mobile-only");
}

#[test]
fn a_short_mobile_width_component_is_out_of_scope() {
    // 390×320 is a card/component, not a screen — it is supposed to end at its
    // content.
    let root = json!({
        "type": "frame", "id": "root", "name": "Card",
        "width": 390, "height": 320, "layout": "vertical", "gap": 0,
        "children": [{ "type": "frame", "id": "body", "name": "Body",
                       "width": "fill_container", "height": 320 }]
    });
    assert!(!echoed(root), "a component is not a screen");
}

#[test]
fn content_overflowing_the_root_is_left_to_the_spill_diagnostics() {
    // A negative gap is a different fact; this echo must not claim it.
    let root = json!({
        "type": "frame", "id": "root", "name": "Overflowing Screen",
        "width": 390, "height": 844, "layout": "vertical", "gap": 0,
        "clipContent": true,
        "children": [{ "type": "frame", "id": "body", "name": "Body",
                       "width": "fill_container", "height": 1200 }]
    });
    assert!(!echoed(root), "overflow is not a flush-bottom report");
}

#[test]
fn fixed_artboards_do_not_claim_padding_created_room_below_fixed_content() {
    let mut root = fixed_gap_screen(0.0);
    root.as_object_mut().unwrap().remove("clipContent");
    let before = state_for(root.clone()).doc;
    let (state, changed) = repair(root);
    assert!(!changed);
    assert_eq!(state.doc, before);
    assert_eq!(resolved_direct_child_gap(&state), 0.0);
    assert!(geometry_diagnostics(&state)
        .iter()
        .any(|issue| issue.contains(ECHO)));
}

#[test]
fn cleanup_repairs_zero_and_eleven_pixel_gaps_to_twenty_eight() {
    for initial_gap in [0.0, 11.0] {
        let (state, changed) = repair(fixed_gap_screen(initial_gap));
        assert!(changed, "{initial_gap}px must be repaired");
        let gap = resolved_direct_child_gap(&state);
        assert!(
            (gap - 28.0).abs() <= 1.0,
            "expected 28px after repairing {initial_gap}px, got {gap}"
        );
        assert!(
            !geometry_diagnostics(&state)
                .iter()
                .any(|issue| issue.contains(ECHO)),
            "repair and echo must share one fact predicate"
        );
    }
}

#[test]
fn cleanup_preserves_compliant_gap_navigation_desktop_and_business_nodes() {
    let (compliant, changed) = repair(fixed_gap_screen(24.0));
    assert!(!changed, "an existing 24px gap is already compliant");
    assert!((resolved_direct_child_gap(&compliant) - 24.0).abs() <= 1.0);

    let nav = json!({
        "type": "frame", "id": "root", "name": "Home",
        "width": 390, "height": 844, "layout": "vertical",
        "children": [
            { "type": "frame", "id": "body", "width": "fill_container", "height": 772 },
            { "type": "frame", "id": "nav", "role": "bottom-tab-bar",
              "width": "fill_container", "height": 72 }
        ]
    });
    let (with_nav, changed) = repair(nav);
    assert!(!changed, "bottom navigation deliberately closes the screen");
    assert_eq!(with_nav.active_children()[0].children().unwrap().len(), 2);

    let desktop = json!({
        "type": "frame", "id": "root", "name": "Dashboard",
        "width": 1440, "height": 900, "layout": "vertical",
        "children": [
            { "type": "frame", "id": "body", "width": "fill_container", "height": 900 }
        ]
    });
    let (desktop, changed) = repair(desktop);
    assert!(!changed, "desktop roots are outside the mobile contract");
    assert_eq!(desktop.active_children()[0].children().unwrap().len(), 1);
}

#[test]
fn cleanup_preserves_a_last_wrapper_closed_by_nested_bottom_navigation() {
    let root = json!({
        "type": "frame", "id": "root", "name": "Explore",
        "width": 390, "height": 844, "layout": "vertical",
        "children": [
            { "type": "frame", "id": "status", "role": "status-bar",
              "width": "fill_container", "height": 62 },
            {
                "type": "frame", "id": "wrapper", "name": "Content Wrapper",
                "width": "fill_container", "height": 782, "layout": "vertical",
                "children": [
                    { "type": "frame", "id": "content",
                      "width": "fill_container", "height": 710 },
                    { "type": "frame", "id": "nav", "role": "bottom-tab-bar",
                      "width": "fill_container", "height": 72,
                      "layout": "horizontal" }
                ]
            }
        ]
    });
    let (state, changed) = repair(root);

    assert!(
        !changed,
        "a nested trailing bottom navigation already closes the screen"
    );
    let value = serde_json::to_value(&state.active_children()[0]).expect("root serializes");
    assert!(
        value.get("padding").is_none(),
        "cleanup must not add blank space below nested navigation: {value}"
    );
    assert!(
        !geometry_diagnostics(&state)
            .iter()
            .any(|issue| issue.contains(ECHO)),
        "the shared diagnostic must recognize the same closing nav fact"
    );
}

#[test]
fn cleanup_repairs_a_last_wrapper_without_nested_bottom_navigation() {
    let root = json!({
        "type": "frame", "id": "root", "name": "Article Detail",
        "width": 390, "height": 844, "layout": "vertical",
        "clipContent": false,
        "children": [{
            "type": "frame", "id": "wrapper", "name": "Content Wrapper",
            "width": "fill_container", "height": 844, "layout": "vertical",
            "children": [{
                "type": "frame", "id": "content",
                "width": "fill_container", "height": 844
            }]
        }]
    });
    let (state, changed) = repair(root);

    assert!(
        changed,
        "an ordinary trailing wrapper still needs breathing room"
    );
    assert!((resolved_direct_child_gap(&state) - 28.0).abs() <= 1.0);
    let value = serde_json::to_value(&state.active_children()[0]).expect("root serializes");
    assert_eq!(value["padding"], json!([0.0, 0.0, 28.0, 0.0]));
}

#[test]
fn all_roots_driver_repairs_every_mobile_screen() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(json!({
        "version": "1.0",
        "children": [
            {
                "type": "frame", "id": "root-a", "width": 390, "height": 844,
                "clipContent": false,
                "layout": "vertical",
                "children": [
                    { "type": "frame", "id": "body-a",
                      "width": "fill_container", "height": 844 }
                ]
            },
            {
                "type": "frame", "id": "root-b", "x": 440, "width": 390, "height": 844,
                "clipContent": false,
                "layout": "vertical",
                "children": [
                    { "type": "frame", "id": "body-b",
                      "width": "fill_container", "height": 844 }
                ]
            }
        ]
    }))
    .expect("valid two-screen document");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let changed = {
        let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
        repair_mobile_bottom_breathing_for_all_roots(&mut sink)
    };
    assert!(changed);
    for root in state.active_children() {
        let value = serde_json::to_value(root).expect("root serializes");
        assert_eq!(
            value["padding"],
            json!([0.0, 0.0, 28.0, 0.0]),
            "every root must be visited"
        );
    }
}
