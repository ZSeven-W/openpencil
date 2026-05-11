use super::*;

#[test]
fn delete_selected_removes_top_level_node_and_clears_selection() {
    let mut doc = Document::sample();
    let target = NodeId::new(10);
    doc.set_single_selection(target);
    assert!(doc.active_page().unwrap().find(target).is_some());
    assert!(doc.delete_selected());
    assert_eq!(doc.selected, NodeId::NONE);
    assert!(doc.active_page().unwrap().find(target).is_none());
}

#[test]
fn delete_selected_removes_nested_node() {
    let mut doc = Document::sample();
    let nested = NodeId::new(13); // Button background — descendant of Frame 10
    doc.set_single_selection(nested);
    assert!(doc.active_page().unwrap().find(nested).is_some());
    assert!(doc.delete_selected());
    assert!(doc.active_page().unwrap().find(nested).is_none());
    // Parent must remain.
    assert!(doc.active_page().unwrap().find(NodeId::new(10)).is_some());
}

#[test]
fn delete_selected_returns_false_when_unselected() {
    let mut doc = Document::sample();
    doc.clear_selection();
    assert!(!doc.delete_selected());
}

#[test]
fn duplicate_selected_clones_subtree_with_fresh_ids_and_selects_it() {
    let mut doc = Document::sample();
    doc.set_single_selection(NodeId::new(10)); // bounded Frame with children
    let mut next_id = 1_000u64;
    let before_descendant = doc.active_page().unwrap().find(NodeId::new(13)).cloned();
    assert!(before_descendant.is_some());

    let clone_id = doc
        .duplicate_selected(&mut next_id, 10.0)
        .expect("duplicate should return new id");
    assert!(clone_id.is_real());
    assert_eq!(doc.selected, clone_id);
    // Original still present.
    assert!(doc.active_page().unwrap().find(NodeId::new(10)).is_some());
    // Clone has fresh id (different from any original).
    assert!(doc.active_page().unwrap().find(clone_id).is_some());
    // Clone origin shifted by offset.
    let original = doc.active_page().unwrap().find(NodeId::new(10)).unwrap();
    let clone = doc.active_page().unwrap().find(clone_id).unwrap();
    assert!((clone.bounds.origin.x - original.bounds.origin.x - 10.0).abs() < 1e-3);
    assert!((clone.bounds.origin.y - original.bounds.origin.y - 10.0).abs() < 1e-3);
    // Clone preserves descendant count.
    assert_eq!(clone.children.len(), original.children.len());
    // No id collision in the page (would be caught by validate).
    assert!(doc.validate().is_ok());
}

#[test]
fn reorder_selected_up_moves_to_higher_index() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(1, NodeKind::Rect, "A"),
        Node::leaf(2, NodeKind::Rect, "B"),
        Node::leaf(3, NodeKind::Rect, "C"),
    ];
    doc.set_single_selection(NodeId::new(2)); // middle
    assert!(doc.reorder_selected(ReorderDirection::Up));
    let ids: Vec<u64> = doc.pages[root_idx]
        .children
        .iter()
        .map(|n| n.id.raw())
        .collect();
    assert_eq!(ids, vec![1, 3, 2]);
}

#[test]
fn reorder_selected_down_moves_to_lower_index() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(1, NodeKind::Rect, "A"),
        Node::leaf(2, NodeKind::Rect, "B"),
        Node::leaf(3, NodeKind::Rect, "C"),
    ];
    doc.set_single_selection(NodeId::new(2));
    assert!(doc.reorder_selected(ReorderDirection::Down));
    let ids: Vec<u64> = doc.pages[root_idx]
        .children
        .iter()
        .map(|n| n.id.raw())
        .collect();
    assert_eq!(ids, vec![2, 1, 3]);
}

#[test]
fn reorder_selected_at_edges_is_noop() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(1, NodeKind::Rect, "A"),
        Node::leaf(2, NodeKind::Rect, "B"),
    ];
    doc.set_single_selection(NodeId::new(1));
    assert!(!doc.reorder_selected(ReorderDirection::Down));
    doc.set_single_selection(NodeId::new(2));
    assert!(!doc.reorder_selected(ReorderDirection::Up));
}

#[test]
fn duplicate_selected_lifts_allocator_past_existing_max_id() {
    // Codex CONCERN-1 regression: external docs may carry ids
    // larger than the host's `next_node_id` counter. The
    // mutator must lift the allocator past the document's max
    // before minting a new id so no collision is possible.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(5000, NodeKind::Rect, "A"),
        Node::leaf(5001, NodeKind::Rect, "B"),
    ];
    doc.set_single_selection(NodeId::new(5000));
    let mut next_id = 100u64;
    let clone_id = doc
        .duplicate_selected(&mut next_id, 0.0)
        .expect("duplicate should succeed");
    assert!(
        clone_id.raw() > 5001,
        "clone id {} must exceed the document max id",
        clone_id.raw()
    );
    // Allocator is updated so subsequent calls also stay
    // collision-free.
    assert!(next_id > 5001);
    assert!(doc.validate().is_ok());
}

#[test]
fn duplicate_selected_returns_none_when_max_id_overflows() {
    // Codex CONCERN-3 follow-up: if a document somehow carries
    // `NodeId(u64::MAX)`, `max_node_id + 1` would overflow.
    // Return None instead of saturating and minting a
    // colliding id.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(u64::MAX, NodeKind::Rect, "boundary"),
        Node::leaf(7, NodeKind::Rect, "small"),
    ];
    doc.set_single_selection(NodeId::new(7));
    let mut next_id = 100u64;
    assert!(doc.duplicate_selected(&mut next_id, 0.0).is_none());
    // No id mutation occurred — count is unchanged.
    assert_eq!(doc.pages[root_idx].children.len(), 2);
}

#[test]
fn duplicate_selected_rejects_when_subtree_exhausts_id_space() {
    // Codex CONCERN-5: even when `max_node_id + 1` fits, a
    // multi-node subtree may still walk past `u64::MAX`
    // partway through, leaving the document half-cloned. The
    // headroom precheck must reject in that case so the page
    // is untouched.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    // 3-node subtree: parent + 2 children. Small ids so
    // `max_node_id + 1` doesn't trip the earlier guard.
    let parent = Node::with_children(
        7,
        NodeKind::Group,
        "group",
        vec![
            Node::leaf(8, NodeKind::Rect, "a"),
            Node::leaf(9, NodeKind::Rect, "b"),
        ],
    );
    doc.pages[root_idx].children = vec![parent];
    doc.set_single_selection(NodeId::new(7));

    // Allocator close enough to `u64::MAX` that minting all 3
    // ids would overflow `*next_id += 1` past the last mint.
    let mut next_id = u64::MAX - 2;
    assert!(doc.duplicate_selected(&mut next_id, 0.0).is_none());
    // Page untouched.
    assert_eq!(doc.pages[root_idx].children.len(), 1);
    // Allocator unchanged — no half-mint state.
    assert_eq!(next_id, u64::MAX - 2);
}

fn build_shape_doc() -> Document {
    // Single page with one of each shape kind at known
    // bounds. Each shape occupies a 100×100 rect.
    let mut doc = Document::empty();
    let page_idx = doc.active_page_index;
    doc.pages[page_idx].children = vec![
        Node::leaf(101, NodeKind::Rect, "R").with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0)),
        Node::leaf(102, NodeKind::Ellipse, "E")
            .with_bounds(crate::Rect::xywh(200.0, 0.0, 100.0, 100.0)),
        Node::leaf(103, NodeKind::Polygon, "P")
            .with_bounds(crate::Rect::xywh(400.0, 0.0, 100.0, 100.0)),
        Node::leaf(104, NodeKind::Line, "L")
            .with_bounds(crate::Rect::xywh(600.0, 0.0, 100.0, 100.0))
            .with_stroke(crate::Color::BLACK, 2.0),
    ];
    doc
}

#[test]
fn hit_test_rect_uses_bounds() {
    let doc = build_shape_doc();
    // Rect: anywhere inside the bounds is a hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
        Some(NodeId::new(101))
    );
    // Corner edge.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(0.5, 99.5)),
        Some(NodeId::new(101))
    );
}

#[test]
fn hit_test_ellipse_uses_ellipse_geometry() {
    let doc = build_shape_doc();
    // Centre of ellipse bounds (200..300, 0..100) is (250, 50).
    // Inside the inscribed oval → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(250.0, 50.0)),
        Some(NodeId::new(102))
    );
    // Corner of bounds rect → INSIDE rect-bounds but OUTSIDE
    // the ellipse → no hit (was a hit under the old rect-only
    // path; codex audit fix).
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(201.0, 1.0)), None);
    // Point on the perimeter (left edge of oval, at vertical
    // midpoint) → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(200.5, 50.0)),
        Some(NodeId::new(102))
    );
}

#[test]
fn hit_test_polygon_uses_triangle_geometry() {
    let doc = build_shape_doc();
    // Triangle vertices: top-center (450, 0), bottom-left
    // (400, 100), bottom-right (500, 100).
    // Centroid (~450, 66.7) → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(450.0, 70.0)),
        Some(NodeId::new(103))
    );
    // Top-left corner of bounds (401, 1) → outside the
    // triangle (above the left edge) → no hit.
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(401.0, 1.0)), None);
    // Top-right corner of bounds → outside the triangle.
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(499.0, 1.0)), None);
    // Bottom-center, on the base → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(450.0, 99.0)),
        Some(NodeId::new(103))
    );
}

#[test]
fn hit_test_line_uses_stroke_proximity() {
    let doc = build_shape_doc();
    // Line stroke = 2 px → threshold ≈ 1 + 4 = 5 doc px.
    // Diagonal from (600, 0) to (700, 100).
    // Midpoint (650, 50) → exactly on the line → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(650.0, 50.0)),
        Some(NodeId::new(104))
    );
    // Near the diagonal (within slack) → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(648.0, 51.0)),
        Some(NodeId::new(104))
    );
    // Far from the diagonal (top-right corner of bounds) →
    // no hit (was a hit under rect-only path).
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(699.0, 1.0)), None);
    // Bottom-left corner of bounds → far from the diagonal,
    // no hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(601.0, 99.0)),
        None
    );
}

#[test]
fn hit_test_horizontal_line_with_zero_height_bounds_is_grabbable() {
    // Codex CONCERN: zero-y bounds (horizontal line) was
    // rejected by the AABB pre-filter. Line kind now has its
    // own path that runs distance-to-segment unconditionally.
    let mut doc = Document::empty();
    let page_idx = doc.active_page_index;
    doc.pages[page_idx].children = vec![Node::leaf(50, NodeKind::Line, "horiz")
        .with_bounds(crate::Rect::xywh(0.0, 50.0, 100.0, 0.0))
        .with_stroke(crate::Color::BLACK, 2.0)];
    // Click directly on the segment.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
        Some(NodeId::new(50))
    );
    // Click 3 px above — within stroke (1) + 4 screen px slack
    // at zoom 1 = 5 doc px threshold.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(50.0, 47.0)),
        Some(NodeId::new(50))
    );
    // Click well above — no hit.
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 40.0)), None);
}

#[test]
fn hit_test_vertical_line_with_zero_width_bounds_is_grabbable() {
    let mut doc = Document::empty();
    let page_idx = doc.active_page_index;
    doc.pages[page_idx].children = vec![Node::leaf(51, NodeKind::Line, "vert")
        .with_bounds(crate::Rect::xywh(50.0, 0.0, 0.0, 100.0))
        .with_stroke(crate::Color::BLACK, 2.0)];
    // Click directly on the segment.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
        Some(NodeId::new(51))
    );
    // Click 3 px to the right — within threshold.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(53.0, 50.0)),
        Some(NodeId::new(51))
    );
    // Click well to the right — no hit.
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(60.0, 50.0)), None);
}

#[test]
fn hit_test_line_slack_scales_with_zoom() {
    // At zoom = 0.5 (zoomed out), 4 screen px = 8 doc px slack
    // so a click 7 doc px from the segment is still a hit.
    // At zoom = 2.0 (zoomed in), 4 screen px = 2 doc px, so
    // the same 7-px-offset click misses.
    let mut doc = Document::empty();
    let page_idx = doc.active_page_index;
    doc.pages[page_idx].children = vec![Node::leaf(60, NodeKind::Line, "diag")
        .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 0.0))
        .with_stroke(crate::Color::BLACK, 0.0)];

    // Threshold = stroke_half (0) + 4/zoom. Click 7 doc px
    // above the segment at zoom 0.5 → 4/0.5 = 8 doc px slack
    // → hit. Same click at zoom 2 → 4/2 = 2 doc px → miss.
    doc.viewport.zoom = 0.5;
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(50.0, -7.0)),
        Some(NodeId::new(60))
    );
    doc.viewport.zoom = 2.0;
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, -7.0)), None);
}

#[test]
fn hit_test_line_with_negative_size_bounds_is_grabbable() {
    // Codex CONCERN follow-up: `aggregate_bounds` collapses
    // negative-size leaves to `Rect::ZERO`, so a Line drawn
    // right-to-left or bottom-to-top would be unhittable if
    // the Line path read aggregate bounds. Reading `node.bounds`
    // directly preserves the sign and the distance helper is
    // sign-independent.
    let mut doc = Document::empty();
    let page_idx = doc.active_page_index;
    doc.pages[page_idx].children = vec![Node::leaf(70, NodeKind::Line, "rev")
        // Right-to-left: origin (100, 0), size (-100, 50)
        // → segment from (100, 0) to (0, 50).
        .with_bounds(crate::Rect::xywh(100.0, 0.0, -100.0, 50.0))
        .with_stroke(crate::Color::BLACK, 2.0)];
    // Midpoint of the segment (50, 25) → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(50.0, 25.0)),
        Some(NodeId::new(70))
    );
    // Endpoint (0, 50) → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(0.0, 50.0)),
        Some(NodeId::new(70))
    );
    // Far above the segment → no hit.
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 0.0)), None);
}

#[test]
fn hit_test_rotated_negative_size_line_uses_segment_midpoint_pivot() {
    // Codex CONCERN follow-up: rotation pivot was computed
    // from aggregate_bounds which collapses negative-size
    // Lines to ZERO — meaning a rotated negative Line rotated
    // around (0, 0) instead of its midpoint. The kind-aware
    // `rotation_pivot` helper fixes it.
    use std::f32::consts::PI;
    let mut doc = Document::empty();
    let page_idx = doc.active_page_index;
    // Bottom-to-top line: origin (50, 100), size (0, -100) →
    // collapses to aggregate=ZERO. Vertical segment from
    // (50, 100) up to (50, 0). Midpoint (50, 50). Rotating
    // 90° around (50, 50) gives a horizontal segment from
    // (100, 50) to (0, 50).
    doc.pages[page_idx].children = vec![Node {
        id: NodeId::new(80),
        kind: NodeKind::Line,
        name: "rev_vert".into(),
        bounds: crate::Rect::xywh(50.0, 100.0, 0.0, -100.0),
        fill: None,
        stroke: Some(Stroke {
            color: crate::Color::BLACK,
            width: 2.0,
        }),
        text: None,
        rotation: PI / 2.0,
        hidden: false,
        locked: false,
        collapsed: false,
        fill_type: FillType::Solid,
        children: Vec::new(),
    }];
    // Click at (50, 50) — midpoint (invariant under rotation).
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)),
        Some(NodeId::new(80))
    );
    // After 90° rotation, the segment is horizontal from
    // (100, 50) to (0, 50). Click at (25, 50) → hit.
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(25.0, 50.0)),
        Some(NodeId::new(80))
    );
    // Click at (50, 90) — was on the un-rotated segment, but
    // after 90° rotation it would land off the line → no hit.
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 90.0)), None);
}

#[test]
fn hit_test_zero_size_node_is_never_hit() {
    let mut doc = Document::empty();
    let page_idx = doc.active_page_index;
    doc.pages[page_idx].children =
        vec![Node::leaf(7, NodeKind::Rect, "z").with_bounds(crate::Rect::xywh(0.0, 0.0, 0.0, 0.0))];
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(0.0, 0.0)), None);
}

#[test]
fn deselect_all_clears_selection() {
    let mut doc = Document::sample();
    doc.set_single_selection(NodeId::new(10));
    doc.deselect_all();
    assert_eq!(doc.selected, NodeId::NONE);
    assert!(doc.selected_set.is_empty());
}

#[test]
fn set_single_selection_replaces_set_and_anchor() {
    let mut doc = Document::sample();
    doc.selected_set = vec![NodeId::new(11), NodeId::new(13)];
    doc.selected = NodeId::new(13);
    doc.set_single_selection(NodeId::new(14));
    assert_eq!(doc.selected_set, vec![NodeId::new(14)]);
    assert_eq!(doc.selected, NodeId::new(14));
    assert_eq!(doc.selection_count(), 1);
    doc.set_single_selection(NodeId::NONE);
    assert!(doc.selected_set.is_empty());
    assert_eq!(doc.selected, NodeId::NONE);
}

#[test]
fn toggle_selection_adds_then_removes_and_picks_new_anchor() {
    let mut doc = Document::sample();
    doc.clear_selection();
    doc.toggle_selection(NodeId::new(11));
    assert_eq!(doc.selected_set, vec![NodeId::new(11)]);
    assert_eq!(doc.selected, NodeId::new(11));
    doc.toggle_selection(NodeId::new(13));
    assert_eq!(doc.selected_set, vec![NodeId::new(11), NodeId::new(13)]);
    assert_eq!(doc.selected, NodeId::new(13));
    doc.toggle_selection(NodeId::new(13));
    assert_eq!(doc.selected_set, vec![NodeId::new(11)]);
    assert_eq!(doc.selected, NodeId::new(11));
    doc.toggle_selection(NodeId::new(11));
    assert!(doc.selected_set.is_empty());
    assert_eq!(doc.selected, NodeId::NONE);
}

#[test]
fn select_all_top_level_populates_set_with_active_page_children() {
    let mut doc = Document::sample();
    doc.clear_selection();
    assert!(doc.select_all_top_level());
    assert_eq!(doc.selected_set, vec![NodeId::new(10)]);
    assert_eq!(doc.selected, NodeId::new(10));
    let mut empty = Document::empty();
    assert!(!empty.select_all_top_level());
    assert!(empty.selected_set.is_empty());
}

#[test]
fn delete_selected_removes_every_node_in_the_set() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(1, NodeKind::Rect, "a"),
        Node::leaf(2, NodeKind::Rect, "b"),
        Node::leaf(3, NodeKind::Rect, "c"),
    ];
    doc.selected_set = vec![NodeId::new(1), NodeId::new(3)];
    doc.selected = NodeId::new(3);
    assert!(doc.delete_selected());
    let ids: Vec<u64> = doc.pages[root_idx]
        .children
        .iter()
        .map(|n| n.id.raw())
        .collect();
    assert_eq!(ids, vec![2]);
    assert!(doc.selected_set.is_empty());
    assert_eq!(doc.selected, NodeId::NONE);
}

#[test]
fn duplicate_selected_clones_every_node_in_the_set() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    // IDs 2 + 3 — page id 1 already occupies the namespace
    // so child id 1 would collide on `validate`.
    doc.pages[root_idx].children = vec![
        Node::leaf(2, NodeKind::Rect, "a"),
        Node::leaf(3, NodeKind::Rect, "b"),
    ];
    doc.selected_set = vec![NodeId::new(2), NodeId::new(3)];
    doc.selected = NodeId::new(3);
    let mut next_id = 100u64;
    let anchor = doc
        .duplicate_selected(&mut next_id, 0.0)
        .expect("duplicate set");
    assert_eq!(doc.pages[root_idx].children.len(), 4);
    assert_eq!(doc.selected_set.len(), 2);
    assert_eq!(doc.selected, anchor);
    assert!(doc.validate().is_ok());
}

#[test]
fn toggle_node_hidden_flips_flag_and_skips_canvas_hit_test() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children =
        vec![Node::leaf(7, NodeKind::Rect, "a")
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0))];
    let p = crate::Point2D::new(50.0, 50.0);
    assert_eq!(doc.node_at_doc_point(p), Some(NodeId::new(7)));
    assert!(doc.toggle_node_hidden(NodeId::new(7)));
    // Hidden → canvas hit-test ignores it.
    assert_eq!(doc.node_at_doc_point(p), None);
    // Toggle again to unhide.
    assert!(doc.toggle_node_hidden(NodeId::new(7)));
    assert_eq!(doc.node_at_doc_point(p), Some(NodeId::new(7)));
}

#[test]
fn set_selected_fill_type_writes_per_node_and_does_not_leak() {
    // Codex full-audit CONCERN regression: `fill_type` used
    // to live on `Document.ui`, so picking a different
    // selection inherited the prior picker choice. Now it's
    // a per-node field — each node remembers its own type.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(2, NodeKind::Rect, "a"),
        Node::leaf(3, NodeKind::Rect, "b"),
    ];
    // Both nodes start as Solid (Node::leaf default).
    assert_eq!(
        doc.active_page()
            .unwrap()
            .find(NodeId::new(2))
            .unwrap()
            .fill_type,
        FillType::Solid
    );
    // Select "a", set to LinearGradient.
    doc.set_single_selection(NodeId::new(2));
    assert!(doc.set_selected_fill_type(FillType::LinearGradient));
    assert_eq!(
        doc.active_page()
            .unwrap()
            .find(NodeId::new(2))
            .unwrap()
            .fill_type,
        FillType::LinearGradient
    );
    // Selecting "b" must NOT inherit "a"'s LinearGradient.
    doc.set_single_selection(NodeId::new(3));
    assert_eq!(
        doc.active_page()
            .unwrap()
            .find(NodeId::new(3))
            .unwrap()
            .fill_type,
        FillType::Solid
    );
    // Going back to "a" still shows LinearGradient.
    assert_eq!(
        doc.active_page()
            .unwrap()
            .find(NodeId::new(2))
            .unwrap()
            .fill_type,
        FillType::LinearGradient
    );
}

#[test]
fn set_selected_fill_type_respects_locked_and_hidden() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let mut locked = Node::leaf(4, NodeKind::Rect, "lock");
    locked.locked = true;
    let mut hidden = Node::leaf(5, NodeKind::Rect, "hide");
    hidden.hidden = true;
    doc.pages[root_idx].children = vec![locked, hidden];
    doc.set_single_selection(NodeId::new(4));
    assert!(!doc.set_selected_fill_type(FillType::Image));
    assert_eq!(
        doc.active_page()
            .unwrap()
            .find(NodeId::new(4))
            .unwrap()
            .fill_type,
        FillType::Solid
    );
    doc.set_single_selection(NodeId::new(5));
    assert!(!doc.set_selected_fill_type(FillType::Image));
    assert_eq!(
        doc.active_page()
            .unwrap()
            .find(NodeId::new(5))
            .unwrap()
            .fill_type,
        FillType::Solid
    );
}
