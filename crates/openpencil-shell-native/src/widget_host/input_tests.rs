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
    host.document.set_single_selection(NodeId::new("n10"));

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
    host.document.set_single_selection(NodeId::new("n10"));
    host.document.ui.property_focus = Some(PropertyFocus::PositionX);
    host.document.ui.property_input_draft = "123".to_string();

    assert!(host.apply_backspace());
    assert_eq!(host.document.ui.property_input_draft, "12");
    // Selection must be untouched.
    assert_eq!(host.document.selected, NodeId::new("n10"));
}

#[test]
fn backspace_without_focus_deletes_selected() {
    let mut host = WidgetHostNative::new();
    host.document.set_single_selection(NodeId::new("n10"));
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
        Node::leaf("n50", NodeKind::Rect, "a").with_bounds(Rect::xywh(50.0, 10.0, 20.0, 20.0)),
        Node::leaf("n51", NodeKind::Rect, "b").with_bounds(Rect::xywh(90.0, 10.0, 20.0, 20.0)),
        Node::leaf("n52", NodeKind::Rect, "c").with_bounds(Rect::xywh(200.0, 200.0, 20.0, 20.0)),
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
    let mut hits: Vec<&str> = host.document.selected_set.iter().map(|i| i.raw()).collect();
    hits.sort();
    assert_eq!(hits, vec!["n50", "n51"]);
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
        Node::leaf("n70", NodeKind::Rect, "a").with_bounds(Rect::xywh(50.0, 50.0, 20.0, 20.0)),
        Node::leaf("n71", NodeKind::Rect, "b").with_bounds(Rect::xywh(300.0, 300.0, 20.0, 20.0)),
    ];
    // Pre-select "a" — and the marquee will cover it too.
    host.document.set_single_selection(NodeId::new("n70"));
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
    assert!(host.document.is_selected(&NodeId::new("n70")));
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
        vec![Node::leaf("n80", NodeKind::Rect, "a").with_bounds(Rect::xywh(0.0, 0.0, 100.0, 100.0))];
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
        Node::leaf("n60", NodeKind::Rect, "a").with_bounds(Rect::xywh(10.0, 10.0, 20.0, 20.0)),
        Node::leaf("n61", NodeKind::Rect, "b").with_bounds(Rect::xywh(50.0, 10.0, 20.0, 20.0)),
        // "c" is far away so its handles can't interfere
        // with the press point used to start the marquee.
        Node::leaf("n62", NodeKind::Rect, "c").with_bounds(Rect::xywh(300.0, 300.0, 20.0, 20.0)),
    ];
    // Pre-select "c" (far from press point so handle hit-test
    // misses), then shift-marquee over "a" + "b".
    host.document.set_single_selection(NodeId::new("n62"));
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
    let mut ids: Vec<&str> = host.document.selected_set.iter().map(|i| i.raw()).collect();
    ids.sort();
    assert_eq!(ids, vec!["n60", "n61", "n62"]);
}

#[test]
fn layer_drag_to_reorder_commits_on_release_with_threshold_move() {
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    // Three top-level nodes, ids 70 / 71 / 72 — children
    // already painted as flat layer rows.
    host.document.pages[page_idx].children = vec![
        Node::leaf("n70", NodeKind::Rect, "A"),
        Node::leaf("n71", NodeKind::Rect, "B"),
        Node::leaf("n72", NodeKind::Rect, "C"),
    ];
    host.document.clear_selection();
    // LayerPanel row geometry — has to match the panel paint
    // (8 px top inset + Pages section header + 1 page row +
    // section gap + Layers section header, all walked from
    // TOP_BAR_HEIGHT).
    let row_h = 28.0; // LAYER_ROW_HEIGHT
    let page_row_h = 32.0; // PAGE_ROW_HEIGHT
    let section_header_h = 28.0;
    let section_gap = 8.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let layers_top =
        TOP_BAR_HEIGHT + 8.0 + section_header_h + page_row_h + section_gap + section_header_h;
    let row_y = |i: usize| layers_top + (i as f32) * row_h + row_h / 2.0;
    let row_x = host.document.ui.layer_panel_width / 2.0;
    // Press on row "A" (index 0) — seeds layer_drag.
    host.apply_press(row_x, row_y(0), viewport_w, viewport_h);
    assert!(host.layer_drag.is_some());
    assert!(!host.layer_drag.as_ref().unwrap().active);
    // Move past threshold to row "C" (index 2) — activates drag,
    // updates current_y so drop_target_at picks "C" After on
    // release.
    host.apply_cursor_move(row_x, row_y(2) + row_h / 2.0 - 4.0);
    assert!(host.layer_drag.as_ref().unwrap().active);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    assert!(host.layer_drag.is_none(), "drag must be cleared on release");
    // A moved after C → final order [B, C, A].
    let order: Vec<&str> = host.document.pages[page_idx]
        .children
        .iter()
        .map(|n| n.id.raw())
        .collect();
    assert_eq!(order, vec!["n71", "n72", "n70"]);
}

#[test]
fn layer_drag_below_activation_threshold_is_a_click_not_a_reorder() {
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    host.document.pages[page_idx].children = vec![
        Node::leaf("n80", NodeKind::Rect, "X"),
        Node::leaf("n81", NodeKind::Rect, "Y"),
    ];
    host.document.clear_selection();
    let row_y_first = TOP_BAR_HEIGHT + 8.0 + 28.0 + 32.0 + 8.0 + 28.0 + 14.0;
    let row_x = host.document.ui.layer_panel_width / 2.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.apply_press(row_x, row_y_first, viewport_w, viewport_h);
    // Sub-threshold move (2 px, less than 4 px activation).
    host.apply_cursor_move(row_x, row_y_first + 2.0);
    assert!(
        host.layer_drag.is_some() && !host.layer_drag.as_ref().unwrap().active,
        "sub-threshold move must not activate"
    );
    host.apply_release_with_viewport(viewport_w, viewport_h);
    // Click semantics: selection is on the first row, tree is
    // unchanged.
    let order: Vec<&str> = host.document.pages[page_idx]
        .children
        .iter()
        .map(|n| n.id.raw())
        .collect();
    assert_eq!(order, vec!["n80", "n81"]);
    assert_eq!(host.document.selected, NodeId::new("n80"));
}

#[test]
fn anchor_press_release_without_motion_does_not_push_history() {
    // Codex CONCERN: a press-release on an anchor without any
    // cursor motion in between must NOT pollute the undo stack.
    // Direct test of release semantics — seed the drag state by
    // hand (the press hit-test geometry is exercised elsewhere).
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::{Point2D, Rect};
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    let mut path = Node::leaf("n60", NodeKind::Path, "p");
    path.bounds = Rect::xywh(0.0, 0.0, 100.0, 50.0);
    path.points = vec![Point2D::new(0.0, 0.0), Point2D::new(50.0, 25.0)];
    host.document.pages[page_idx].children = vec![path];
    host.document.set_single_selection(NodeId::new("n60"));
    let snap = host.document.snapshot_for_history();
    host.path_anchor_drag = Some(crate::widget_host::PathAnchorDragState {
        node_id: NodeId::new("n60"),
        anchor_index: 1,
        start_doc: Point2D::new(50.0, 25.0),
        moved: false, // no cursor_move happened between press + release
        pre_drag_snapshot: snap,
    });
    let history_before = host.document.history.past.len();
    let consumed = host.apply_release_with_viewport(1440.0, 900.0);
    assert!(host.path_anchor_drag.is_none(), "drag state cleared");
    assert!(!consumed, "release with no motion is not a UI change");
    assert_eq!(
        host.document.history.past.len(),
        history_before,
        "no-motion press-release must not push a history entry"
    );
}

#[test]
fn anchor_drag_back_to_start_lands_at_start() {
    // Codex BLOCK: previous code only wrote the anchor when the
    // cursor differed from start_doc, so dragging away and back
    // skipped the final write — the anchor stuck at the last
    // off-start frame.
    use openpencil_shell_core::document::{Node, NodeKind, Tool};
    use openpencil_shell_core::{Point2D, Rect};
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    let mut path = Node::leaf("n60", NodeKind::Path, "p");
    path.bounds = Rect::xywh(0.0, 0.0, 100.0, 50.0);
    path.points = vec![Point2D::new(0.0, 0.0), Point2D::new(50.0, 25.0)];
    host.document.pages[page_idx].children = vec![path];
    host.document.set_single_selection(NodeId::new("n60"));
    host.document.tool = Tool::Pen;
    let snap = host.document.snapshot_for_history();
    // Seed drag state at the anchor (50, 25).
    host.path_anchor_drag = Some(crate::widget_host::PathAnchorDragState {
        node_id: NodeId::new("n60"),
        anchor_index: 1,
        start_doc: Point2D::new(50.0, 25.0),
        moved: false,
        pre_drag_snapshot: snap,
    });
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    // Drag away to doc (80, 25) — first move sets `moved = true`.
    host.apply_cursor_move(cx0 + 80.0, cy0 + 25.0);
    let after_first = host.document.pages[page_idx].children[0].points[1];
    assert!((after_first.x - 80.0).abs() < 0.5);
    // Drag BACK to start (50, 25) — must write the new position.
    host.apply_cursor_move(cx0 + 50.0, cy0 + 25.0);
    let after_return = host.document.pages[page_idx].children[0].points[1];
    assert!(
        (after_return.x - 50.0).abs() < 0.5,
        "anchor must follow cursor back to start; got {after_return:?}"
    );
}

#[test]
fn anchor_drag_with_motion_pushes_one_history_entry() {
    // Inverse of the above — when the user actually moved the
    // anchor, exactly one entry lands on the undo stack so Cmd-Z
    // reverts the whole drag.
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::{Point2D, Rect};
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    let mut path = Node::leaf("n60", NodeKind::Path, "p");
    path.bounds = Rect::xywh(0.0, 0.0, 100.0, 50.0);
    path.points = vec![Point2D::new(0.0, 0.0), Point2D::new(50.0, 25.0)];
    host.document.pages[page_idx].children = vec![path];
    host.document.set_single_selection(NodeId::new("n60"));
    let snap = host.document.snapshot_for_history();
    host.path_anchor_drag = Some(crate::widget_host::PathAnchorDragState {
        node_id: NodeId::new("n60"),
        anchor_index: 1,
        start_doc: Point2D::new(50.0, 25.0),
        moved: true, // simulating real cursor_move during drag
        pre_drag_snapshot: snap,
    });
    let history_before = host.document.history.past.len();
    let consumed = host.apply_release_with_viewport(1440.0, 900.0);
    assert!(consumed, "release after motion is a UI change");
    assert_eq!(
        host.document.history.past.len(),
        history_before + 1,
        "exactly one history entry per drag"
    );
}

#[test]
fn node_drag_not_intercepted_by_align_toolbar_hover() {
    // Codex CONCERN: with 2+ selected, an active node-drag must
    // continue moving the nodes when the cursor sweeps over the
    // floating align toolbar's hit region. Earlier code's early
    // return on hover-state change would have stolen the drag's
    // delta for that frame.
    use openpencil_shell_core::document::{Node, NodeKind};
    use openpencil_shell_core::widgets::TOP_BAR_HEIGHT;
    use openpencil_shell_core::Rect;
    let mut host = WidgetHostNative::new();
    let page_idx = host.document.active_page_index;
    host.document.pages[page_idx].children = vec![
        Node::leaf("n90", NodeKind::Rect, "a").with_bounds(Rect::xywh(50.0, 200.0, 20.0, 20.0)),
        Node::leaf("n91", NodeKind::Rect, "b").with_bounds(Rect::xywh(120.0, 200.0, 20.0, 20.0)),
    ];
    // Two-node selection so the align toolbar is shown.
    host.document.selected_set = vec![NodeId::new("n90"), NodeId::new("n91")];
    host.document.selected = NodeId::new("n91");
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    // Press on node "a" — promotes to a node-drag.
    let press_x = cx0 + 60.0;
    let press_y = cy0 + 210.0;
    host.apply_press(press_x, press_y, viewport_w, viewport_h);
    assert!(host.node_drag.is_some(), "node_drag must seed on press");
    // Move the cursor toward the canvas-top center — the align
    // toolbar's hit region sits there (y = TOP_BAR_HEIGHT + 16).
    let zoom = host.document.viewport.zoom.max(0.0001);
    let target_x = host.document.ui.layer_panel_width + 400.0;
    let target_y = TOP_BAR_HEIGHT + 24.0; // inside align toolbar y-band
    let expected_dx = (target_x - press_x) / zoom;
    let expected_dy = (target_y - press_y) / zoom;
    host.apply_cursor_move(target_x, target_y);
    // Nodes must have translated by (expected_dx, expected_dy).
    let bounds_a = host.document.pages[page_idx].children[0].bounds;
    let bounds_b = host.document.pages[page_idx].children[1].bounds;
    assert!(
        (bounds_a.origin.x - (50.0 + expected_dx)).abs() < 0.5
            && (bounds_a.origin.y - (200.0 + expected_dy)).abs() < 0.5,
        "node-drag delta lost on a; got {:?}, expected start+delta",
        bounds_a
    );
    assert!(
        (bounds_b.origin.x - (120.0 + expected_dx)).abs() < 0.5,
        "node-drag delta lost on b; got {:?}",
        bounds_b
    );
}
