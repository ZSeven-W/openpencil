use super::*;

#[test]
fn locked_selection_blocks_translate_set_bounds_rotation_color_and_property_edit() {
    // Codex stop-hook BLOCK: locked nodes selected via the
    // layer panel must not be mutated by translate / resize /
    // rotate / property edits / color setters.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let mut locked = Node::leaf(40, NodeKind::Rect, "locked")
        .with_bounds(crate::Rect::xywh(10.0, 10.0, 50.0, 50.0))
        .with_fill(crate::Color::WHITE);
    locked.locked = true;
    doc.pages[root_idx].children = vec![locked];
    doc.set_single_selection(NodeId::new(40));

    let before = doc.selected_node().unwrap().bounds;
    doc.translate_selected(7.0, 3.0);
    assert_eq!(doc.selected_node().unwrap().bounds, before);
    doc.set_selected_bounds(crate::Rect::xywh(0.0, 0.0, 1.0, 1.0));
    assert_eq!(doc.selected_node().unwrap().bounds, before);
    doc.set_selected_rotation(1.5);
    assert!(doc.selected_node().unwrap().rotation.abs() < f32::EPSILON);
    assert!(!doc.set_selected_color(true, crate::Color::BLACK));
    assert_eq!(doc.selected_node().unwrap().fill, Some(crate::Color::WHITE));
    assert!(!doc.commit_property_edit(PropertyFocus::PositionX, 999.0));
    assert_eq!(
        doc.selected_node().unwrap().bounds.origin.x,
        before.origin.x
    );
}

#[test]
fn hidden_selection_blocks_translate_and_property_edit() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let mut hidden = Node::leaf(41, NodeKind::Rect, "hidden")
        .with_bounds(crate::Rect::xywh(10.0, 10.0, 50.0, 50.0));
    hidden.hidden = true;
    doc.pages[root_idx].children = vec![hidden];
    doc.set_single_selection(NodeId::new(41));
    let before = doc.selected_node().unwrap().bounds;
    doc.translate_selected(7.0, 3.0);
    assert_eq!(doc.selected_node().unwrap().bounds, before);
    assert!(!doc.commit_property_edit(PropertyFocus::PositionX, 999.0));
    assert_eq!(
        doc.selected_node().unwrap().bounds.origin.x,
        before.origin.x
    );
}

#[test]
fn delete_selected_protects_ancestor_of_locked_descendant() {
    // Codex stop-hook BLOCK: selecting an editable Frame +
    // its locked child must not let `delete_selected` wipe
    // the child by collapsing the parent.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let mut child = Node::leaf(61, NodeKind::Rect, "child");
    child.locked = true;
    let frame = Node::with_children(60, NodeKind::Frame, "frame", vec![child])
        .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
    doc.pages[root_idx].children = vec![frame];
    // Select the Frame only.
    doc.set_single_selection(NodeId::new(60));
    // delete_selected must refuse — the Frame's subtree
    // contains a locked child. is_subtree_editable returns
    // false → nothing deletable.
    assert!(!doc.delete_selected());
    // Frame still in document.
    assert!(doc.active_page().unwrap().find(NodeId::new(60)).is_some());
    assert!(doc.active_page().unwrap().find(NodeId::new(61)).is_some());

    // Same scenario, hidden descendant.
    let mut doc2 = Document::empty();
    let root_idx2 = doc2.active_page_index;
    let mut child2 = Node::leaf(71, NodeKind::Rect, "child");
    child2.hidden = true;
    let frame2 = Node::with_children(70, NodeKind::Frame, "frame", vec![child2])
        .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
    doc2.pages[root_idx2].children = vec![frame2];
    doc2.set_single_selection(NodeId::new(70));
    assert!(!doc2.delete_selected());
    assert!(doc2.active_page().unwrap().find(NodeId::new(70)).is_some());
}

#[test]
fn delete_selected_protects_locked_and_hidden() {
    // Mixed selection: one normal, one locked, one hidden.
    // Delete removes only the normal id; the others stay
    // selected (so the user can unlock/unhide).
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let normal = Node::leaf(50, NodeKind::Rect, "normal");
    let mut locked = Node::leaf(51, NodeKind::Rect, "locked");
    locked.locked = true;
    let mut hidden = Node::leaf(52, NodeKind::Rect, "hidden");
    hidden.hidden = true;
    doc.pages[root_idx].children = vec![normal, locked, hidden];
    doc.selected_set = vec![NodeId::new(50), NodeId::new(51), NodeId::new(52)];
    doc.selected = NodeId::new(52);
    assert!(doc.delete_selected());
    // Normal id removed; locked + hidden survive.
    let ids: Vec<u64> = doc.pages[root_idx]
        .children
        .iter()
        .map(|n| n.id.raw())
        .collect();
    assert_eq!(ids, vec![51, 52]);
    let mut surviving: Vec<u64> = doc.selected_set.iter().map(|i| i.raw()).collect();
    surviving.sort();
    assert_eq!(surviving, vec![51, 52]);
}

#[test]
fn locked_frame_children_remain_hittable() {
    // Codex CONCERN-Q5a regression: TS `spatial-index.ts`
    // filters locked per-node, not by subtree — children of
    // a locked Frame stay clickable. Rust must match.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let child = Node::leaf(21, NodeKind::Rect, "inner")
        .with_bounds(crate::Rect::xywh(20.0, 20.0, 40.0, 40.0));
    let mut frame = Node::with_children(20, NodeKind::Frame, "outer", vec![child])
        .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
    frame.locked = true;
    doc.pages[root_idx].children = vec![frame];
    // Click inside the child's rect → child selected (locked
    // doesn't propagate to children).
    assert_eq!(
        doc.node_at_doc_point(crate::Point2D::new(40.0, 40.0)),
        Some(NodeId::new(21))
    );
    // Click inside the Frame BUT outside the child → no hit
    // (the Frame body is locked).
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(80.0, 80.0)), None);
}

#[test]
fn hidden_frame_children_skip_hit_test() {
    // Hidden cascades to subtree — child of a hidden Frame
    // is unclickable even when the click is in its own
    // bounds.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let child = Node::leaf(31, NodeKind::Rect, "inner")
        .with_bounds(crate::Rect::xywh(20.0, 20.0, 40.0, 40.0));
    let mut frame = Node::with_children(30, NodeKind::Frame, "outer", vec![child])
        .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
    frame.hidden = true;
    doc.pages[root_idx].children = vec![frame];
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(40.0, 40.0)), None);
}

#[test]
fn toggle_node_locked_skips_canvas_hit_test_but_keeps_paint() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children =
        vec![Node::leaf(8, NodeKind::Rect, "a")
            .with_bounds(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0))];
    assert!(doc.toggle_node_locked(NodeId::new(8)));
    // Locked → canvas hit-test ignores it (TS parity:
    // locked layers can't be selected via canvas click; the
    // user has to click the layer-panel row to unlock).
    assert_eq!(doc.node_at_doc_point(crate::Point2D::new(50.0, 50.0)), None);
    // Node still in document (so it still paints; canvas
    // viewport's paint_node only skips on `hidden`, not
    // `locked`).
    let node = doc
        .active_page()
        .unwrap()
        .find(NodeId::new(8))
        .expect("locked node still in document");
    assert!(node.locked);
    assert!(!node.hidden);
}

#[test]
fn nodes_intersecting_doc_rect_returns_overlapping_top_level_ids() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children = vec![
        Node::leaf(2, NodeKind::Rect, "a").with_bounds(crate::Rect::xywh(0.0, 0.0, 50.0, 50.0)),
        Node::leaf(3, NodeKind::Rect, "b").with_bounds(crate::Rect::xywh(100.0, 100.0, 50.0, 50.0)),
        Node::leaf(4, NodeKind::Rect, "c").with_bounds(crate::Rect::xywh(40.0, 40.0, 30.0, 30.0)),
    ];
    // Rect (10, 10, 80, 80) overlaps "a" (touches origin) and
    // "c" (fully contained). Misses "b" at (100, 100).
    let hits = doc.nodes_intersecting_doc_rect(crate::Rect::xywh(10.0, 10.0, 80.0, 80.0));
    let ids: Vec<u64> = hits.iter().map(|i| i.raw()).collect();
    assert_eq!(ids, vec![2, 4]);
}

#[test]
fn nodes_intersecting_doc_rect_handles_negative_size() {
    // Marquee drags going right-to-left or bottom-to-top
    // produce negative-size rects. The helper normalizes both
    // the query rect and node bounds via abs().
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children =
        vec![Node::leaf(2, NodeKind::Rect, "a")
            .with_bounds(crate::Rect::xywh(20.0, 20.0, 10.0, 10.0))];
    // Negative-size rect spanning (10, 10) → (40, 40).
    let hits = doc.nodes_intersecting_doc_rect(crate::Rect::xywh(40.0, 40.0, -30.0, -30.0));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0], NodeId::new(2));
}

#[test]
fn nodes_intersecting_doc_rect_skips_degenerate_nodes() {
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    doc.pages[root_idx].children =
        vec![Node::leaf(2, NodeKind::Rect, "zero")
            .with_bounds(crate::Rect::xywh(5.0, 5.0, 0.0, 0.0))];
    let hits = doc.nodes_intersecting_doc_rect(crate::Rect::xywh(0.0, 0.0, 100.0, 100.0));
    assert!(hits.is_empty());
}

#[test]
fn property_panel_visible_only_for_single_selection() {
    // Codex stop-hook guard: the host's canvas_region uses
    // this gate to decide whether to reserve the right rail.
    // Multi-select must hide the panel AND give the canvas
    // back the rail's width.
    let mut doc = Document::sample();
    // No selection → hidden.
    doc.clear_selection();
    assert!(!doc.property_panel_visible());
    // Single selection → visible.
    doc.set_single_selection(NodeId::new(10));
    assert!(doc.property_panel_visible());
    // Multi-select → hidden.
    doc.toggle_selection(NodeId::new(11));
    assert_eq!(doc.selection_count(), 2);
    assert!(!doc.property_panel_visible());
    // Back to single → visible.
    doc.set_single_selection(NodeId::new(10));
    assert!(doc.property_panel_visible());
}

#[test]
fn property_panel_visible_hides_stale_single_anchor() {
    // Codex stop-hook BLOCK guard: the panel paint gate
    // requires `selected_node().is_some()`, so the canvas-
    // region gate must too. A single-anchor selection that
    // resolves to no node (id not in active page, or stale
    // after a delete) must NOT reserve the right rail.
    let mut doc = Document::sample();
    // Point selection at a non-existent id.
    doc.set_single_selection(NodeId::new(9999));
    assert_eq!(doc.selection_count(), 1);
    assert!(doc.selected_node().is_none());
    assert!(!doc.property_panel_visible());

    // Selection on a node from a non-active page is also
    // unresolved (selected_node only scopes to active page).
    let mut doc2 = Document::empty();
    doc2.pages.push(Page::new(
        2,
        "p2",
        vec![Node::leaf(99, NodeKind::Rect, "alpha")],
    ));
    doc2.active_page_index = 0; // page 1 active
    doc2.set_single_selection(NodeId::new(99)); // node on page 2
    assert_eq!(doc2.selection_count(), 1);
    assert!(doc2.selected_node().is_none());
    assert!(!doc2.property_panel_visible());
}

#[test]
fn multi_select_handle_hit_test_is_disabled() {
    // Codex CONCERN guard: handles aren't painted on multi-
    // select, so the hit-test must not return them either.
    use crate::widgets::{rotation_corner_at_point, selection_handle_at_point};
    let mut doc = Document::sample();
    // 2 selected → handles hidden, hit-test returns None.
    doc.selected_set = vec![NodeId::new(11), NodeId::new(13)];
    doc.selected = NodeId::new(13);
    let canvas_rect = crate::Rect::xywh(0.0, 0.0, 800.0, 600.0);
    // A point that would hit the anchor's top-left handle if
    // hit-test ran — at zoom 1, pan 0, anchor 13 bounds origin.
    let node = doc.selected_node().unwrap();
    let bounds = node.aggregate_bounds();
    let handle_point = crate::Point2D::new(
        canvas_rect.origin.x + bounds.origin.x,
        canvas_rect.origin.y + bounds.origin.y,
    );
    assert!(
        selection_handle_at_point(canvas_rect, &doc, handle_point).is_none(),
        "multi-select must not expose handle hit-tests"
    );
    assert!(
        rotation_corner_at_point(canvas_rect, &doc, handle_point).is_none(),
        "multi-select must not expose rotation hit-tests"
    );
    // Collapse to single-select → handles are interactive again.
    doc.set_single_selection(NodeId::new(13));
    assert!(selection_handle_at_point(canvas_rect, &doc, handle_point).is_some());
}

#[test]
fn translate_selected_dedups_ancestor_descendant_pairs() {
    // Codex CONCERN guard: selecting BOTH a container AND one
    // of its descendants must NOT double-shift the descendant.
    let mut doc = Document::empty();
    let root_idx = doc.active_page_index;
    let child = Node::leaf(11, NodeKind::Rect, "child")
        .with_bounds(crate::Rect::xywh(50.0, 50.0, 20.0, 20.0));
    let frame = Node::with_children(10, NodeKind::Frame, "frame", vec![child])
        .with_bounds(crate::Rect::xywh(0.0, 0.0, 200.0, 200.0));
    doc.pages[root_idx].children = vec![frame];
    doc.selected_set = vec![NodeId::new(10), NodeId::new(11)];
    doc.selected = NodeId::new(11);

    let child_before = doc
        .active_page()
        .unwrap()
        .find(NodeId::new(11))
        .unwrap()
        .bounds
        .origin;
    doc.translate_selected(10.0, 5.0);
    let child_after = doc
        .active_page()
        .unwrap()
        .find(NodeId::new(11))
        .unwrap()
        .bounds
        .origin;
    assert!((child_after.x - child_before.x - 10.0).abs() < 1e-3);
    assert!((child_after.y - child_before.y - 5.0).abs() < 1e-3);
}

#[test]
fn translate_selected_moves_bounded_frame_with_descendants() {
    // Codex stop-hook: translating a bounded Frame must also
    // shift its descendants (whose bounds are document-space
    // absolute) — otherwise the children "detach" visually.
    let mut doc = Document::sample();
    doc.set_single_selection(NodeId::new(10)); // Frame
    let frame_before = doc.selected_node().unwrap().bounds.origin;
    // Walk a known descendant ahead of time.
    let descendant_before = doc
        .active_page()
        .unwrap()
        .find(NodeId::new(13)) // Button background rect
        .unwrap()
        .bounds
        .origin;
    doc.translate_selected(50.0, 25.0);
    let frame_after = doc.selected_node().unwrap().bounds.origin;
    let descendant_after = doc
        .active_page()
        .unwrap()
        .find(NodeId::new(13))
        .unwrap()
        .bounds
        .origin;
    assert!((frame_after.x - frame_before.x - 50.0).abs() < 1e-3);
    assert!((frame_after.y - frame_before.y - 25.0).abs() < 1e-3);
    // Descendant must shift by the SAME delta — proves it
    // didn't detach when its parent moved.
    assert!((descendant_after.x - descendant_before.x - 50.0).abs() < 1e-3);
    assert!((descendant_after.y - descendant_before.y - 25.0).abs() < 1e-3);
}

#[test]
fn node_id_sentinel_is_zero() {
    assert_eq!(NodeId::NONE.raw(), 0);
    assert!(!NodeId::NONE.is_real());
    assert!(NodeId::new(1).is_real());
}

#[test]
fn node_id_to_widget_id_round_trips_inner_value() {
    let nid = NodeId::new(42);
    let wid = nid.to_widget_id();
    assert_eq!(wid.0, 42);
}

#[test]
fn node_find_walks_subtree() {
    let leaf = Node::leaf(3, NodeKind::Rect, "leaf");
    let mid = Node::with_children(2, NodeKind::Group, "mid", vec![leaf]);
    let root = Node::with_children(1, NodeKind::Frame, "root", vec![mid]);
    assert_eq!(root.find(NodeId::new(1)).unwrap().name, "root");
    assert_eq!(root.find(NodeId::new(2)).unwrap().name, "mid");
    assert_eq!(root.find(NodeId::new(3)).unwrap().name, "leaf");
    assert!(root.find(NodeId::new(99)).is_none());
}

#[test]
fn document_sample_has_expected_shape() {
    let doc = Document::sample();
    assert_eq!(doc.pages.len(), 1);
    assert_eq!(doc.pages[0].name, "Page 1");
    // Frame > [Title, Button > [bg, text]]
    let frame = &doc.pages[0].children[0];
    assert_eq!(frame.kind, NodeKind::Frame);
    assert_eq!(frame.children.len(), 2);
    assert_eq!(frame.children[0].kind, NodeKind::Text);
    assert_eq!(frame.children[1].kind, NodeKind::Group);
    assert_eq!(frame.children[1].children.len(), 2);
}

#[test]
fn document_selected_node_returns_real_hit() {
    let doc = Document::sample();
    let sel = doc.selected_node().unwrap();
    assert_eq!(sel.id, NodeId::new(11));
    assert_eq!(sel.name, "Title");
    assert_eq!(sel.kind, NodeKind::Text);
}

#[test]
fn document_empty_has_no_selection() {
    let doc = Document::empty();
    assert_eq!(doc.selected, NodeId::NONE);
    assert!(doc.selected_node().is_none());
}

#[test]
fn document_find_unknown_id_is_none() {
    let mut doc = Document::sample();
    doc.set_single_selection(NodeId::new(9999));
    assert!(doc.selected_node().is_none());
}

#[test]
fn node_kind_label_matches_variant() {
    assert_eq!(NodeKind::Frame.label(), "Frame");
    assert_eq!(NodeKind::Group.label(), "Group");
    assert_eq!(NodeKind::Rect.label(), "Rect");
    assert_eq!(NodeKind::Text.label(), "Text");
    assert_eq!(NodeKind::Other("Custom".into()).label(), "Custom");
}

#[test]
#[should_panic(expected = "id 0 is reserved for NodeId::NONE")]
fn node_id_new_zero_panics_in_release_too() {
    // Codex Step 2 R1 BLOCK fix: hard panic, not just
    // debug_assert. `#[should_panic]` test runs in both debug
    // and release configurations.
    let _ = NodeId::new(0);
}

#[test]
fn document_sample_passes_validate() {
    // Codex Step 2 R1 CONCERN-2: sample() must be invariant-
    // clean (no duplicate ids, valid active_page_index).
    let doc = Document::sample();
    assert!(doc.validate().is_ok());
}

#[test]
fn document_validate_catches_duplicate_node_id() {
    let doc = Document {
        pages: vec![Page::new(
            1,
            "p",
            vec![
                Node::leaf(2, NodeKind::Rect, "a"),
                Node::leaf(2, NodeKind::Rect, "b"), // dup id 2
            ],
        )],
        active_page_index: 0,
        selected: NodeId::NONE,
        selected_set: Vec::new(),
        clipboard: Vec::new(),
        tool: Tool::Select,
        viewport: Viewport::IDENTITY,
        chat: ChatState::default(),
        ui: UiState::default(),
    };
    let result = doc.validate();
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("duplicate NodeId"),
        "validate should mention duplicate"
    );
}

#[test]
fn document_validate_catches_active_page_index_out_of_range() {
    let mut doc = Document::sample();
    doc.active_page_index = 99;
    let result = doc.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("active_page_index"));
}

#[test]
fn document_validate_catches_empty_pages() {
    // Codex Step 2 R2 CONCERN-1: an empty-pages document with
    // any active_page_index used to silently pass validate().
    // Now empty pages is itself a violation.
    let doc = Document {
        pages: Vec::new(),
        active_page_index: 0,
        selected: NodeId::NONE,
        selected_set: Vec::new(),
        clipboard: Vec::new(),
        tool: Tool::Select,
        viewport: Viewport::IDENTITY,
        chat: ChatState::default(),
        ui: UiState::default(),
    };
    let result = doc.validate();
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("pages is empty"),
        "validate should mention empty pages"
    );

    // Also covers the previously-uncaught case: empty pages
    // + bogus active_page_index → still rejected, AND the
    // empty-check fires FIRST (not the range check). Codex
    // Step 2 R3 CONCERN: prior version only asserted
    // `is_err()`, leaving ordering ambiguous.
    let doc2 = Document {
        pages: Vec::new(),
        active_page_index: 99,
        selected: NodeId::NONE,
        selected_set: Vec::new(),
        clipboard: Vec::new(),
        tool: Tool::Select,
        viewport: Viewport::IDENTITY,
        chat: ChatState::default(),
        ui: UiState::default(),
    };
    let err2 = doc2.validate().unwrap_err();
    assert!(
        err2.contains("pages is empty"),
        "empty-pages check must fire before active_page_index range check; got: {err2}"
    );
    assert!(
        !err2.contains("active_page_index"),
        "empty-pages check must short-circuit; got both: {err2}"
    );
}

#[test]
fn document_selected_node_scopes_to_active_page() {
    // Codex Step 2 R1 CONCERN-1: a selection on page 1 must
    // NOT show up when page 2 is active.
    let page1 = Page::new(1, "p1", vec![Node::leaf(10, NodeKind::Rect, "P1-A")]);
    let page2 = Page::new(2, "p2", vec![Node::leaf(20, NodeKind::Rect, "P2-A")]);
    let doc = Document {
        pages: vec![page1, page2],
        active_page_index: 1,      // page 2 active
        selected: NodeId::new(10), // selection points at page 1's node
        selected_set: vec![NodeId::new(10)],
        clipboard: Vec::new(),
        tool: Tool::Select,
        viewport: Viewport::IDENTITY,
        chat: ChatState::default(),
        ui: UiState::default(),
    };
    // Selection is on a non-active page → returns None.
    assert!(doc.selected_node().is_none());

    // Switch active page to 0 and the same selection now
    // resolves.
    let doc2 = Document {
        active_page_index: 0,
        ..doc
    };
    assert_eq!(doc2.selected_node().unwrap().name, "P1-A");
}

#[test]
fn document_active_page_returns_indexed_page() {
    let doc = Document {
        pages: vec![
            Page::new(1, "first", vec![]),
            Page::new(2, "second", vec![]),
        ],
        active_page_index: 1,
        selected: NodeId::NONE,
        selected_set: Vec::new(),
        clipboard: Vec::new(),
        tool: Tool::Select,
        viewport: Viewport::IDENTITY,
        chat: ChatState::default(),
        ui: UiState::default(),
    };
    assert_eq!(doc.active_page().unwrap().name, "second");
}

#[test]
fn document_active_page_returns_none_when_index_out_of_range() {
    let doc = Document {
        pages: vec![Page::new(1, "only", vec![])],
        active_page_index: 5,
        selected: NodeId::NONE,
        selected_set: Vec::new(),
        clipboard: Vec::new(),
        tool: Tool::Select,
        viewport: Viewport::IDENTITY,
        chat: ChatState::default(),
        ui: UiState::default(),
    };
    assert!(doc.active_page().is_none());
}

#[test]
fn node_id_raw_returns_inner_value() {
    assert_eq!(NodeId::new(42).raw(), 42);
    assert_eq!(NodeId::NONE.raw(), 0);
}

#[test]
fn add_page_appends_named_page_and_switches() {
    let mut doc = Document::sample();
    let before = doc.pages.len();
    let max_before = doc.max_node_id();
    let new_idx = doc.add_page().expect("add_page should succeed");
    assert_eq!(doc.pages.len(), before + 1);
    assert_eq!(new_idx, before);
    assert_eq!(doc.active_page_index, before);
    let new_page = doc.active_page().unwrap();
    assert_eq!(new_page.name, format!("Page {}", before + 1));
    // Fresh id must NOT collide with any existing node id.
    assert!(new_page.id.raw() > max_before);
    // Newly-added page is empty; selection clears.
    assert!(new_page.children.is_empty());
    assert_eq!(doc.selected, NodeId::NONE);
    assert!(doc.selected_set.is_empty());

    // Second call exercises the `"Page N"` formula for n > 1.
    let new_idx_2 = doc.add_page().expect("second add_page");
    assert_eq!(new_idx_2, before + 1);
    assert_eq!(
        doc.active_page().unwrap().name,
        format!("Page {}", before + 2)
    );
}

#[test]
fn add_page_returns_none_on_id_overflow() {
    // Fabricate a document whose largest id is u64::MAX so the
    // next page mint would overflow.
    let doc = Document {
        pages: vec![Page::new(u64::MAX, "max", vec![])],
        active_page_index: 0,
        selected: NodeId::NONE,
        selected_set: Vec::new(),
        clipboard: Vec::new(),
        tool: Tool::Select,
        viewport: Viewport::IDENTITY,
        chat: ChatState::default(),
        ui: UiState::default(),
    };
    let mut doc = doc;
    assert_eq!(doc.add_page(), None);
    assert_eq!(doc.pages.len(), 1, "overflow must leave pages untouched");
}
