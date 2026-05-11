//! `#[cfg(test)]` companion to `input.rs` — extracted here so the
//! input module stays under the 800-line ceiling.

use super::WidgetHostNative;
use openpencil_shell_core::document::{NodeId, PropertyFocus};

#[test]
fn escape_closes_one_overlay_per_press_in_priority_order() {
    // Codex CONCERN-2 regression: Escape used to clear all
    // three pickers in a single press. TS parity is one-at-a-
    // time, in the order property-focus → locale → shape →
    // fill-type → chat → selection.
    let mut host = WidgetHostNative::new();
    host.document.ui.property_focus = Some(PropertyFocus::PositionX);
    host.document.ui.property_input_draft = "12".to_string();
    host.document.ui.locale_picker_open = true;
    host.document.ui.shape_picker_open = true;
    host.document.ui.fill_type_picker_open = true;
    host.document.chat.focused = true;
    host.document.set_single_selection(NodeId::new(10));

    // 1. Property focus clears first.
    assert!(host.apply_escape());
    assert!(host.document.ui.property_focus.is_none());
    assert!(host.document.ui.property_input_draft.is_empty());
    assert!(host.document.ui.locale_picker_open);

    // 2. Locale picker next.
    assert!(host.apply_escape());
    assert!(!host.document.ui.locale_picker_open);
    assert!(host.document.ui.shape_picker_open);

    // 3. Shape picker.
    assert!(host.apply_escape());
    assert!(!host.document.ui.shape_picker_open);
    assert!(host.document.ui.fill_type_picker_open);

    // 4. Fill-type picker.
    assert!(host.apply_escape());
    assert!(!host.document.ui.fill_type_picker_open);
    assert!(host.document.chat.focused);

    // 5. Chat focus.
    assert!(host.apply_escape());
    assert!(!host.document.chat.focused);
    assert!(host.document.selected.is_real());

    // 6. Selection.
    assert!(host.apply_escape());
    assert_eq!(host.document.selected, NodeId::NONE);

    // 7. Nothing left — returns false.
    assert!(!host.apply_escape());
}

#[test]
fn backspace_with_property_draft_does_not_delete_selected() {
    // Codex confirmed-OK regression guard: with a non-empty
    // property draft buffer, Backspace must pop a char from
    // the draft, not delete the selected node.
    let mut host = WidgetHostNative::new();
    host.document.set_single_selection(NodeId::new(10));
    host.document.ui.property_focus = Some(PropertyFocus::PositionX);
    host.document.ui.property_input_draft = "123".to_string();

    assert!(host.apply_backspace());
    assert_eq!(host.document.ui.property_input_draft, "12");
    // Selection must be untouched.
    assert_eq!(host.document.selected, NodeId::new(10));
}

#[test]
fn backspace_without_focus_deletes_selected() {
    let mut host = WidgetHostNative::new();
    host.document.set_single_selection(NodeId::new(10));
    host.document.ui.property_focus = None;
    host.document.chat.focused = false;

    assert!(host.apply_backspace());
    assert_eq!(host.document.selected, NodeId::NONE);
}

#[test]
fn marquee_drag_replaces_selection_with_intersecting_nodes() {
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::Rect;
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    // 3 rects: two close together near origin, one far away.
    host.document.pages[page_idx].children = vec![
        Node::leaf(50, NodeKind::Rect, "a").with_bounds(Rect::xywh(50.0, 10.0, 20.0, 20.0)),
        Node::leaf(51, NodeKind::Rect, "b").with_bounds(Rect::xywh(90.0, 10.0, 20.0, 20.0)),
        Node::leaf(52, NodeKind::Rect, "c").with_bounds(Rect::xywh(200.0, 200.0, 20.0, 20.0)),
    ];
    host.document.clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    // Press at doc (5, 5) — INSIDE canvas, OUTSIDE every
    // node. Drag to doc (130, 50) — marquee covers "a" + "b",
    // misses "c".
    let press_x = cx0 + 5.0;
    let press_y = cy0 + 5.0;
    host.apply_press(press_x, press_y, viewport_w, viewport_h);
    assert!(
        host.marquee_drag.is_some(),
        "empty-canvas press should start a marquee"
    );
    host.apply_cursor_move(cx0 + 130.0, cy0 + 50.0);
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert!(host.marquee_drag.is_none(), "marquee consumed on release");
    let mut hits: Vec<u64> = host.document.selected_set.iter().map(|i| i.raw()).collect();
    hits.sort();
    assert_eq!(hits, vec![50, 51]);
}

#[test]
fn marquee_drag_with_shift_preserves_already_selected_hit() {
    // Codex CONCERN-Q2 regression: shift-marquee must be
    // ADD-only — a hit that's already in the set stays in,
    // doesn't get removed.
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::Rect;
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    host.document.pages[page_idx].children = vec![
        Node::leaf(70, NodeKind::Rect, "a").with_bounds(Rect::xywh(50.0, 50.0, 20.0, 20.0)),
        Node::leaf(71, NodeKind::Rect, "b").with_bounds(Rect::xywh(300.0, 300.0, 20.0, 20.0)),
    ];
    // Pre-select "a" — and the marquee will cover it too.
    host.document.set_single_selection(NodeId::new(70));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    // Press at doc (5, 5) — empty + far from "a"'s handles.
    // Drag to doc (90, 90) — covers "a" only.
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    host.apply_cursor_move(cx0 + 90.0, cy0 + 90.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    // "a" stays in the set (shift-marquee is ADD-only).
    assert!(host.document.is_selected(NodeId::new(70)));
    assert_eq!(host.document.selected_set.len(), 1);
}

#[test]
fn marquee_drag_below_screen_threshold_is_a_no_op() {
    // Codex CONCERN-Q5 regression: threshold is screen-px,
    // not doc-px — a tiny drag (under 2 screen px) at any
    // zoom is treated as a click, not a marquee.
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::Rect;
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    host.document.pages[page_idx].children =
        vec![Node::leaf(80, NodeKind::Rect, "a").with_bounds(Rect::xywh(0.0, 0.0, 100.0, 100.0))];
    // Zoom out to 0.1 — so 1 doc-px ≈ 0.1 screen-px. A drag
    // of 1 screen-px = 10 doc-px, well above the OLD doc-
    // space threshold of 0.5 doc-px.
    host.document.viewport.zoom = 0.1;
    host.document.clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    // Press far enough from the rect AND from the toolbar
    // (which lives at canvas-left+12, so we go further
    // right). At zoom 0.1, rect (0, 0, 100, 100) renders as
    // 10x10 screen px starting at (cx0, cy0); the toolbar is
    // a ~44-px column starting at (cx0 + 12). Press at
    // (cx0 + 100, cy0 + 50) is doc (1000, 500), outside both.
    host.apply_press(cx0 + 100.0, cy0 + 50.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    // Tiny drag: 1 screen-px. Old doc-space threshold (0.5)
    // would say "10 doc-px > 0.5 → real marquee" and select
    // the rect. New screen-space threshold (2) says "1 < 2
    // → no-op".
    host.apply_cursor_move(cx0 + 101.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    // Selection unchanged — sub-threshold marquee is no-op.
    assert!(host.document.selected_set.is_empty());
}

#[test]
fn marquee_drag_with_shift_extends_existing_selection() {
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::Rect;
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    host.document.pages[page_idx].children = vec![
        Node::leaf(60, NodeKind::Rect, "a").with_bounds(Rect::xywh(10.0, 10.0, 20.0, 20.0)),
        Node::leaf(61, NodeKind::Rect, "b").with_bounds(Rect::xywh(50.0, 10.0, 20.0, 20.0)),
        // "c" is far away so its handles can't interfere
        // with the press point used to start the marquee.
        Node::leaf(62, NodeKind::Rect, "c").with_bounds(Rect::xywh(300.0, 300.0, 20.0, 20.0)),
    ];
    // Pre-select "c" (far from press point so handle hit-test
    // misses), then shift-marquee over "a" + "b".
    host.document.set_single_selection(NodeId::new(62));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    // Press at doc (5, 5) — empty + far from "c"'s handles.
    // Drag to doc (130, 50) covers "a" (10-30 x) and "b"
    // (50-70 x), misses "c" (300+ x).
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    host.apply_cursor_move(cx0 + 130.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    // "c" still in set (additive marquee did not clear);
    // "a" + "b" toggled in.
    let mut ids: Vec<u64> = host.document.selected_set.iter().map(|i| i.raw()).collect();
    ids.sort();
    assert_eq!(ids, vec![60, 61, 62]);
}
