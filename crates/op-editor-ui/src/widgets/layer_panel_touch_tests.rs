//! Focused touch-density regressions for the Pages/Layers surface.

use super::layer_panel::{DropPosition, LayerPanel, LayerPanelHit};
use super::layer_panel_metrics::{
    collapse_target, delete_page_target, glyph_rect_in, layer_action_targets, layer_drag_target,
    LayerPanelMetrics,
};
use super::scroll_flow::{reveal_layer_panel_selection, scroll_layer_panel};
use super::test_capture_backend::CaptureBackend;
use super::{PaintCx, Widget};
use crate::{Point2D, Rect};
use op_editor_core::size_class::{EditorSizeClass, MobileSheetKind};
use op_editor_core::{EditorState, NodeId, SelectionState};

fn center(rect: Rect) -> Point2D {
    Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    )
}

fn touch_state() -> EditorState {
    let page_json = (0..5)
        .map(|page| {
            let children = (0..12)
                .map(|row| {
                    format!(
                        r#"{{"type":"rectangle","id":"node-{page}-{row}","name":"Layer {row}","width":10,"height":10}}"#
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"{{"id":"page-{page}","name":"Page {page}","children":[{children}]}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let src = format!(r#"{{"version":"1.0.0","children":[],"pages":[{page_json}]}}"#);
    let doc = jian_ops_schema::load_str(&src)
        .expect("touch LayerPanel fixture parses")
        .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.sidebar_open = false;
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Layers);
    state
}

fn panel_rect() -> Rect {
    Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(390.0, 600.0),
    }
}

#[test]
fn touch_metrics_are_stored_and_compact_pages_leave_three_layer_rows() {
    let panel = LayerPanel::from_editor(&touch_state());
    let metrics = panel.metrics;
    assert_eq!(metrics.section_header_height, 44.0);
    assert_eq!(metrics.page_row_height, 48.0);
    assert_eq!(metrics.layer_row_height, 48.0);
    assert_eq!(metrics.row_font, 15.0);
    assert_eq!(metrics.glyph_size, 18.0);
    assert_eq!(metrics.action_target, 44.0);
    assert_eq!(metrics.pages_max_rows, 3);

    let regions = panel.regions(panel_rect());
    assert_eq!(regions.pages_view_h, 48.0 * 3.0);
    assert!(regions.layers_view_h >= 48.0 * 3.0);
}

#[test]
fn touch_page_delete_and_layer_actions_are_visible_and_hit_without_hover() {
    let state = touch_state();
    let panel = LayerPanel::from_editor(&state);
    assert_eq!(panel.hovered_page, None);
    assert_eq!(panel.hovered_layer, None);
    let rect = panel_rect();
    let regions = panel.regions(rect);

    let delete = delete_page_target(rect, regions.pages_rows_top, panel.metrics);
    assert_eq!(delete.size, Point2D::new(44.0, 44.0));
    assert_eq!(delete.origin.x + delete.size.x, rect.origin.x + rect.size.x);
    assert_eq!(
        panel.hit_test(rect, center(delete)),
        Some(LayerPanelHit::DeletePage(0))
    );

    let layer_row = Rect {
        origin: Point2D::new(rect.origin.x + 6.0, regions.layers_rows_top + 2.0),
        size: Point2D::new(rect.size.x - 12.0, panel.metrics.layer_row_height - 4.0),
    };
    let (eye, lock) = layer_action_targets(layer_row, panel.metrics);
    assert_eq!(eye.size, Point2D::new(44.0, 44.0));
    assert_eq!(lock.size, Point2D::new(44.0, 44.0));
    assert_eq!(
        panel.hit_test(rect, center(eye)),
        Some(LayerPanelHit::ToggleHidden(NodeId::new("node-0-0")))
    );
    assert_eq!(
        panel.hit_test(rect, center(lock)),
        Some(LayerPanelHit::ToggleLocked(NodeId::new("node-0-0")))
    );

    let mut backend = CaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    panel.paint(&mut cx, rect);
    let expected = [
        glyph_rect_in(delete, panel.metrics.glyph_size).origin,
        glyph_rect_in(eye, panel.metrics.trailing_glyph_size).origin,
        glyph_rect_in(lock, panel.metrics.trailing_glyph_size).origin,
    ];
    for origin in expected {
        assert!(backend.svg_strokes.iter().any(|(_, p, size, _, _)| {
            (p.x - origin.x).abs() < 0.01
                && (p.y - origin.y).abs() < 0.01
                && (*size - 18.0).abs() < 0.01
        }));
    }
}

#[test]
fn touch_reorder_grip_is_44_points_and_disjoint_from_row_controls() {
    let state = touch_state();
    let panel = LayerPanel::from_editor(&state);
    let rect = panel_rect();
    let regions = panel.regions(rect);
    let row = Rect {
        origin: Point2D::new(rect.origin.x + 6.0, regions.layers_rows_top + 2.0),
        size: Point2D::new(rect.size.x - 12.0, panel.metrics.layer_row_height - 4.0),
    };
    let grip = layer_drag_target(row, panel.metrics).expect("touch grip");
    let (eye, lock) = layer_action_targets(row, panel.metrics);
    assert_eq!(grip.size, Point2D::new(44.0, 44.0));
    assert_eq!(grip.origin.x + grip.size.x, eye.origin.x);
    assert_eq!(eye.origin.x + eye.size.x, lock.origin.x);
    assert_eq!(
        panel.drag_source_at(rect, center(grip)),
        Some(NodeId::new("node-0-0"))
    );
    assert_eq!(panel.drag_source_at(rect, center(eye)), None);
    assert_eq!(panel.drag_source_at(rect, center(lock)), None);

    let item = &panel.items[0];
    let indent = panel.metrics.row_pad_x + item.depth as f32 * 12.0;
    let chevron = collapse_target(row, indent, 0.0, panel.metrics);
    assert_eq!(panel.drag_source_at(rect, center(chevron)), None);
}

#[test]
fn touch_reorder_grip_paints_six_dots_but_desktop_has_no_grip() {
    let state = touch_state();
    let panel = LayerPanel::from_editor(&state);
    let rect = panel_rect();
    let regions = panel.regions(rect);
    let row = Rect {
        origin: Point2D::new(rect.origin.x + 6.0, regions.layers_rows_top + 2.0),
        size: Point2D::new(rect.size.x - 12.0, panel.metrics.layer_row_height - 4.0),
    };
    let grip = layer_drag_target(row, panel.metrics).expect("touch grip");
    let mut backend = CaptureBackend::default();
    panel.paint(
        &mut PaintCx {
            backend: &mut backend,
        },
        rect,
    );
    let dots = backend
        .round_fills
        .iter()
        .filter(|(dot, radius, _)| {
            grip.contains(center(*dot)) && dot.size == Point2D::new(3.0, 3.0) && *radius == 1.5
        })
        .count();
    assert_eq!(dots, 6);

    let mut desktop = touch_state();
    desktop.editor_ui.touch = false;
    desktop.editor_ui.sidebar_open = true;
    let desktop_panel = LayerPanel::from_editor(&desktop);
    assert_eq!(desktop_panel.drag_source_at(rect, center(grip)), None);
}

#[test]
fn touch_rename_row_has_neither_reorder_grip_nor_trailing_backing() {
    let mut state = touch_state();
    assert!(state.start_rename_layer(NodeId::new("node-0-0")));
    let panel = LayerPanel::from_editor(&state);
    let rect = panel_rect();
    let regions = panel.regions(rect);
    let row = Rect {
        origin: Point2D::new(rect.origin.x + 6.0, regions.layers_rows_top + 2.0),
        size: Point2D::new(rect.size.x - 12.0, panel.metrics.layer_row_height - 4.0),
    };
    let grip = layer_drag_target(row, panel.metrics).expect("touch grip geometry");
    assert_eq!(panel.drag_source_at(rect, center(grip)), None);

    let mut backend = CaptureBackend::default();
    panel.paint(
        &mut PaintCx {
            backend: &mut backend,
        },
        rect,
    );
    assert_eq!(
        backend
            .round_fills
            .iter()
            .filter(|(dot, radius, _)| {
                grip.contains(center(*dot)) && dot.size == Point2D::new(3.0, 3.0) && *radius == 1.5
            })
            .count(),
        0
    );
}

#[test]
fn touch_drop_and_reveal_use_the_same_48_point_rows() {
    let panel = LayerPanel::from_editor(&touch_state());
    let rect = panel_rect();
    let regions = panel.regions(rect);
    let row_top = regions.layers_rows_top;
    let drop = panel
        .drop_target_at(rect, Point2D::new(120.0, row_top + 40.0))
        .expect("first touch row is a drop target");
    assert_eq!(drop.position, DropPosition::After);
    assert!((drop.indicator_y - (row_top + 48.0)).abs() < 0.01);

    let next = panel
        .layers_offset_revealing(rect, &NodeId::new("node-0-11"))
        .expect("last layer can be revealed");
    assert!((next - (12.0 * 48.0 - regions.layers_view_h)).abs() < 0.01);
}

#[test]
fn layers_sheet_scrolls_and_reveals_while_sidebar_is_closed() {
    let mut state = touch_state();
    let rect = panel_rect();
    let panel = LayerPanel::from_editor(&state);
    let point = Point2D::new(120.0, panel.regions(rect).layers_rows_top + 20.0);
    assert_eq!(
        scroll_layer_panel(&mut state, &panel, rect, point, 0.0, -48.0),
        Some(true)
    );
    assert_eq!(state.editor_ui.layer_layers_scroll.offset, 48.0);

    state.selection = SelectionState {
        anchor: NodeId::new("node-0-11"),
        set: vec![NodeId::new("node-0-11")],
    };
    assert!(reveal_layer_panel_selection(&mut state, &panel, rect));
    assert!(state.editor_ui.layer_layers_scroll.offset > 48.0);
}

#[test]
fn desktop_metrics_and_hover_gates_remain_unchanged() {
    let mut state = touch_state();
    state.editor_ui.touch = false;
    state.editor_ui.sidebar_open = true;
    let panel = LayerPanel::from_editor(&state);
    assert_eq!(panel.metrics, LayerPanelMetrics::DESKTOP);
    let rect = panel_rect();
    let regions = panel.regions(rect);
    let delete = delete_page_target(rect, regions.pages_rows_top, panel.metrics);
    assert_eq!(
        panel.hit_test(rect, center(delete)),
        Some(LayerPanelHit::Page(0))
    );
}

#[test]
fn expand_ancestors_reveals_collapsed_selected_node() {
    // Build a nested hierarchy with many siblings to ensure scroll is needed
    let src = r#"{
        "version":"1.0.0",
        "children":[],
        "pages":[{
            "id":"page-0",
            "name":"Page 0",
            "children":[
                {"type":"rectangle","id":"node-0","name":"Sibling 0","width":10,"height":10},
                {"type":"rectangle","id":"node-1","name":"Sibling 1","width":10,"height":10},
                {"type":"rectangle","id":"node-2","name":"Sibling 2","width":10,"height":10},
                {"type":"group","id":"parent-group","name":"Parent Group",
                "children":[{
                    "type":"rectangle",
                    "id":"node-0-0-child",
                    "name":"Nested Child",
                    "width":10,
                    "height":10
                }]},
                {"type":"rectangle","id":"node-4","name":"Sibling 4","width":10,"height":10},
                {"type":"rectangle","id":"node-5","name":"Sibling 5","width":10,"height":10}
            ]
        }]
    }"#;
    let doc = jian_ops_schema::load_str(src)
        .expect("nested fixture parses")
        .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.sidebar_open = true;

    let parent_id = NodeId::new("parent-group");
    let child_id = NodeId::new("node-0-0-child");

    // Collapse the parent so the child is not visible
    state.editor_ui.collapsed_layers.insert(parent_id.clone());

    // Select the child
    state.selection.anchor = child_id.clone();
    let rect = panel_rect();

    // Build a fresh panel to check if child is initially hidden
    let panel = LayerPanel::from_editor(&state);
    assert!(
        !panel.items.iter().any(|item| item.node_id == child_id),
        "child should not be visible when parent is collapsed"
    );

    // Trigger reveal
    let _ = reveal_layer_panel_selection(&mut state, &panel, rect);

    // Expand should have happened
    assert!(
        !state.editor_ui.collapsed_layers.contains(&parent_id),
        "parent should be expanded after reveal"
    );

    // Record that we revealed this anchor
    assert_eq!(
        state.editor_ui.last_revealed_layer_anchor,
        Some(child_id.clone())
    );

    // After reveal, the panel can be rebuilt and should show the child
    let panel_after = LayerPanel::from_editor(&state);
    let child_found = panel_after
        .items
        .iter()
        .any(|item| item.node_id == child_id);
    assert!(
        child_found,
        "child should be visible after parent is expanded"
    );
}

#[test]
fn expand_ancestors_leaves_siblings_collapsed() {
    // Build a structure with multiple siblings at the same level
    let src = r#"{
        "version":"1.0.0",
        "children":[],
        "pages":[{
            "id":"page-0",
            "name":"Page 0",
            "children":[
                {
                    "type":"group",
                    "id":"group-1",
                    "name":"Group 1",
                    "children":[{
                        "type":"rectangle",
                        "id":"node-1-child",
                        "name":"Node 1 Child",
                        "width":10,
                        "height":10
                    }]
                },
                {
                    "type":"group",
                    "id":"group-2",
                    "name":"Group 2",
                    "children":[{
                        "type":"rectangle",
                        "id":"node-2-child",
                        "name":"Node 2 Child",
                        "width":10,
                        "height":10
                    }]
                }
            ]
        }]
    }"#;
    let doc = jian_ops_schema::load_str(src)
        .expect("sibling fixture parses")
        .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.sidebar_open = true;

    let group1_id = NodeId::new("group-1");
    let group2_id = NodeId::new("group-2");
    let node1_child_id = NodeId::new("node-1-child");

    // Collapse both groups.
    state.editor_ui.collapsed_layers.insert(group1_id.clone());
    state.editor_ui.collapsed_layers.insert(group2_id.clone());

    // Select a child in group 1.
    state.selection.anchor = node1_child_id.clone();
    let rect = panel_rect();
    let panel = LayerPanel::from_editor(&state);

    // Trigger reveal.
    reveal_layer_panel_selection(&mut state, &panel, rect);

    // Group 1 should be expanded, but group 2 should remain collapsed.
    assert!(
        !state.editor_ui.collapsed_layers.contains(&group1_id),
        "group-1 should be expanded"
    );
    assert!(
        state.editor_ui.collapsed_layers.contains(&group2_id),
        "group-2 should remain collapsed"
    );
}

#[test]
fn no_re_expand_on_unchanged_selection_after_manual_collapse() {
    // Build a nested structure with enough siblings for visible items
    let src = r#"{
        "version":"1.0.0",
        "children":[],
        "pages":[{
            "id":"page-0",
            "name":"Page 0",
            "children":[
                {"type":"rectangle","id":"node-0","name":"Sibling 0","width":10,"height":10},
                {"type":"group","id":"parent","name":"Parent",
                "children":[{
                    "type":"rectangle",
                    "id":"child",
                    "name":"Child",
                    "width":10,
                    "height":10
                }]},
                {"type":"rectangle","id":"node-2","name":"Sibling 2","width":10,"height":10}
            ]
        }]
    }"#;
    let doc = jian_ops_schema::load_str(src)
        .expect("parent-child fixture parses")
        .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.sidebar_open = true;

    let parent_id = NodeId::new("parent");
    let child_id = NodeId::new("child");
    let rect = panel_rect();

    // Select the child and reveal
    state.selection.anchor = child_id.clone();
    let panel = LayerPanel::from_editor(&state);
    reveal_layer_panel_selection(&mut state, &panel, rect);

    // Parent should be expanded
    assert!(
        !state.editor_ui.collapsed_layers.contains(&parent_id),
        "parent should be expanded after first reveal"
    );

    // Child should be recorded as last-revealed
    assert_eq!(
        state.editor_ui.last_revealed_layer_anchor,
        Some(child_id.clone()),
        "child should be recorded as last revealed"
    );

    // Now manually collapse the parent while selection stays on child
    state.editor_ui.collapsed_layers.insert(parent_id.clone());

    // Simulate a per-frame auto-reveal: anchor is same, so should not re-expand
    let should_reveal = match (
        &state.selection.anchor,
        &state.editor_ui.last_revealed_layer_anchor,
    ) {
        (anchor, last) if anchor.is_real() => Some(anchor) != last.as_ref(),
        _ => false,
    };

    assert!(!should_reveal, "anchor unchanged, should not re-reveal");
    assert!(
        state.editor_ui.collapsed_layers.contains(&parent_id),
        "manual collapse should be respected"
    );
}

#[test]
fn document_replacement_clears_last_revealed_anchor() {
    let doc = jian_ops_schema::load_str(r#"{"version":"1.0.0","children":[]}"#)
        .expect("empty doc parses")
        .value;
    let mut state = EditorState::from_document(doc);

    // Set last-revealed anchor.
    state.editor_ui.last_revealed_layer_anchor = Some(NodeId::new("old-node"));

    // Replace the document with a new one.
    let new_doc = jian_ops_schema::load_str(r#"{"version":"1.0.0","children":[]}"#)
        .expect("new doc parses")
        .value;
    state.replace_document(new_doc);

    // After document replacement, last-revealed anchor should be cleared.
    assert_eq!(
        state.editor_ui.last_revealed_layer_anchor, None,
        "last-revealed anchor should be cleared on document replacement"
    );
}

#[test]
fn reveal_deep_node_in_many_collapsed_rows_scrolls_into_view() {
    // Regression test for max_offset clamp defect: when expanding many collapsed
    // rows, the old r.layers.max_offset (computed from pre-expansion panel) would
    // clamp the scroll offset too tightly, leaving the selected row out of view.
    // This test creates a scenario where pre-expansion max_offset ≈ 0, but after
    // expansion the selected node needs significant scroll to become visible.

    // Build a document with a collapsed frame containing many children.
    let mut children = String::new();
    for i in 0..20 {
        if i > 0 {
            children.push(',');
        }
        children.push_str(&format!(
            r#"{{"type":"rectangle","id":"child-{i}","name":"Child {i}","width":10,"height":10}}"#
        ));
    }
    let src = format!(
        r#"{{
            "version":"1.0.0",
            "children":[],
            "pages":[{{
                "id":"page-0",
                "name":"Page 0",
                "children":[
                    {{"type":"frame","id":"container","name":"Container","children":[{children}]}}
                ]
            }}]
        }}"#
    );

    let doc = jian_ops_schema::load_str(&src)
        .expect("many-children fixture parses")
        .value;
    let mut state = EditorState::from_document(doc);
    state.editor_ui.sidebar_open = true;

    let container_id = NodeId::new("container");
    let deep_child_id = NodeId::new("child-10");

    // Collapse the container so the panel shows only the frame row + header.
    state
        .editor_ui
        .collapsed_layers
        .insert(container_id.clone());

    // Get the panel before expansion to observe pre-expansion max_offset.
    let rect = panel_rect();
    let panel_before = LayerPanel::from_editor(&state);
    let regions_before = panel_before.regions(rect);
    let max_offset_before = regions_before.layers.max_offset;

    // Select the deep child and trigger reveal.
    state.selection.anchor = deep_child_id.clone();
    let panel = LayerPanel::from_editor(&state);
    reveal_layer_panel_selection(&mut state, &panel, rect);

    // Container should be expanded.
    assert!(
        !state.editor_ui.collapsed_layers.contains(&container_id),
        "container should be expanded"
    );

    // After expansion, rebuild the panel to get fresh regions with the correct max_offset.
    let panel_after = LayerPanel::from_editor(&state);
    let regions_after = panel_after.regions(rect);

    // Verify the scroll offset brings the deep child into view.
    // The child is at row index (1 container + 10 child index) in the expanded items list.
    let row_height = panel_after.metrics.layer_row_height;
    let expected_row_index = 11.0; // Container at 0, child-10 at 11
    let row_top = expected_row_index * row_height;
    let row_bottom = row_top + row_height;

    let view_top = state.editor_ui.layer_layers_scroll.offset;
    let view_bottom = view_top + regions_after.layers_view_h;

    // The row must be within the visible viewport.
    assert!(
        row_top >= view_top && row_bottom <= view_bottom,
        "deep child row should be visible after reveal. view=[{}, {}], row=[{}, {}]",
        view_top,
        view_bottom,
        row_top,
        row_bottom
    );

    // Also verify that max_offset grew due to expansion (pre-expansion was ~0 or small).
    let max_offset_after = regions_after.layers.max_offset;
    assert!(
        max_offset_after > max_offset_before,
        "max_offset should grow with expansion. before={}, after={}",
        max_offset_before,
        max_offset_after
    );
}
