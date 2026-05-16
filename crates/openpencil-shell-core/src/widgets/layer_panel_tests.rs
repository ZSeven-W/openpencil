//! Tests for `widgets::layer_panel` — moved to a sibling file to
//! keep `layer_panel.rs` under the 800-line cap.
//!
//! Phase 6: the panel now builds from `op_editor_core::EditorState`,
//! so the fixtures construct `EditorState` values instead of the old
//! shell-core `Document`.

use super::layer_panel::*;
use super::Widget;
use op_editor_core::NodeId;
use crate::{Point2D, Rect};
use op_editor_core::EditorState;

/// Build an `EditorState` from a canonical `.op` JSON string.
fn state_from(src: &str) -> EditorState {
    let doc = jian_ops_schema::load_str(src)
        .expect("layer-panel fixture parses")
        .value;
    EditorState::from_document(doc)
}

/// Four sibling rectangles `n1..n4` in a single-page document.
fn four_rects() -> EditorState {
    state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n1","name":"A","width":10,"height":10},
              {"type":"rectangle","id":"n2","name":"B","width":10,"height":10},
              {"type":"rectangle","id":"n3","name":"C","width":10,"height":10},
              {"type":"rectangle","id":"n4","name":"D","width":10,"height":10}
        ]}"##,
    )
}

#[test]
fn from_sample_doc_flattens_to_5_layer_rows() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    assert_eq!(panel.items.len(), 5);
    assert_eq!(panel.items[0].label, "Frame");
    assert_eq!(panel.items[0].depth, 0);
    assert_eq!(panel.items[1].depth, 1);
}

#[test]
fn from_sample_doc_has_one_active_page() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    assert_eq!(panel.pages.len(), 1);
    assert!(panel.pages[0].active);
    assert_eq!(panel.pages[0].label, "Page 1");
}

#[test]
fn selection_flag_marks_only_selected_row() {
    let state = EditorState::sample(); // selection anchors on n11
    let panel = LayerPanel::from_editor(&state);
    let selected = panel.items.iter().filter(|i| i.selected).count();
    assert_eq!(selected, 1);
}

#[test]
fn empty_document_yields_one_default_page_no_layers() {
    let state = EditorState::new();
    let panel = LayerPanel::from_editor(&state);
    assert_eq!(panel.pages.len(), 1);
    assert!(panel.items.is_empty());
}

#[test]
fn collapsed_node_hides_its_children() {
    let state = state_from(
        r##"{ "version": "0.8.0", "children": [
              {"type":"frame","id":"n1","name":"Frame","width":100,"height":100,
               "children":[
                 {"type":"rectangle","id":"n2","name":"Child","width":10,"height":10}
               ]}
        ]}"##,
    );
    let mut collapsed = state;
    collapsed
        .editor_ui
        .collapsed_layers
        .insert(op_editor_core::NodeId::new("n1"));
    let panel = LayerPanel::from_editor(&collapsed);
    // Only the frame row paints; the collapsed child is hidden.
    assert_eq!(panel.items.len(), 1);
    assert!(panel.items[0].collapsed);
}

#[test]
fn hit_test_resolves_first_layer_row() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
    };
    let layer_y = 8.0
        + SECTION_HEADER_HEIGHT
        + PAGE_ROW_HEIGHT
        + SECTION_GAP
        + SECTION_HEADER_HEIGHT
        + LAYER_ROW_HEIGHT / 2.0;
    let p = Point2D::new(rect.size.x / 2.0, layer_y);
    match panel.hit_test(rect, p) {
        Some(LayerPanelHit::Layer(id)) => assert_eq!(id, panel.items[0].node_id),
        other => panic!("expected first layer hit, got {:?}", other),
    }
}

#[test]
fn hit_test_resolves_add_page_plus_icon() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
    };
    let plus_x = rect.size.x - ROW_PAD_X - 12.0;
    let plus_y = 8.0 + (SECTION_HEADER_HEIGHT - 14.0) / 2.0;
    assert_eq!(
        panel.hit_test(rect, Point2D::new(plus_x + 7.0, plus_y + 7.0)),
        Some(LayerPanelHit::AddPage)
    );
    assert_eq!(
        panel.hit_test(rect, Point2D::new(plus_x - 3.0, plus_y + 7.0)),
        Some(LayerPanelHit::AddPage)
    );
}

#[test]
fn hit_test_resolves_first_page_row() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
    };
    let page_y = 8.0 + SECTION_HEADER_HEIGHT + PAGE_ROW_HEIGHT / 2.0;
    let p = Point2D::new(rect.size.x / 2.0, page_y);
    assert_eq!(panel.hit_test(rect, p), Some(LayerPanelHit::Page(0)));
}

#[test]
fn access_node_advertises_tree_role_and_layers_label() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    let node = panel.access_node();
    assert_eq!(node.role(), accesskit::Role::Tree);
    assert_eq!(node.label(), Some("Layers"));
}

#[test]
fn from_document_scopes_to_active_page_only() {
    let mut state = state_from(
        r##"{ "version": "0.8.0", "children": [], "pages": [
              {"id":"n1","name":"Page 1","children":[
                {"type":"frame","id":"n2","name":"P1-Node","width":10,"height":10}]},
              {"id":"n3","name":"Page 2","children":[
                {"type":"frame","id":"n4","name":"P2-Node","width":10,"height":10}]}
        ]}"##,
    );
    state.ui.active_page_index = 1;
    let panel = LayerPanel::from_editor(&state);
    assert_eq!(panel.items.len(), 1);
    assert_eq!(panel.items[0].label, "P2-Node");
    assert_eq!(panel.pages.len(), 2);
    assert!(!panel.pages[0].active);
    assert!(panel.pages[1].active);
}

#[test]
fn drop_indicator_matches_post_commit_layout_when_dragging_down() {
    let mut state = four_rects();
    let panel = LayerPanel::from_editor_with_drag_source(&state, &NodeId::new("n1"));
    assert_eq!(panel.items.len(), 3); // A excluded → [B, C, D]
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
    };
    let layers_top = 8.0
        + SECTION_HEADER_HEIGHT
        + panel.pages.len() as f32 * PAGE_ROW_HEIGHT
        + SECTION_GAP
        + SECTION_HEADER_HEIGHT;
    let row_top_of_d = layers_top + 2.0 * LAYER_ROW_HEIGHT;
    let drop = panel
        .drop_target_at(rect, Point2D::new(rect.size.x / 2.0, row_top_of_d + 4.0))
        .unwrap();
    assert_eq!(drop.position, DropPosition::Before);
    assert!((drop.indicator_y - row_top_of_d).abs() < 0.5);
    // Commit and check A's new row top matches indicator_y.
    assert!(state.reorder_before(op_editor_core::NodeId::new("n1"), drop.anchor.clone()));
    let post = LayerPanel::from_editor(&state);
    let a_idx = post
        .items
        .iter()
        .position(|i| i.node_id == NodeId::new("n1"))
        .unwrap();
    let a_row_top = layers_top + a_idx as f32 * LAYER_ROW_HEIGHT;
    assert!(
        (drop.indicator_y - a_row_top).abs() < 0.5,
        "preview/commit y mismatch: indicator at {} but A lands at {}",
        drop.indicator_y,
        a_row_top
    );
}

#[test]
fn drop_target_at_resolves_before_and_after_halves() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
    };
    let y0 = 8.0
        + SECTION_HEADER_HEIGHT
        + panel.pages.len() as f32 * PAGE_ROW_HEIGHT
        + SECTION_GAP
        + SECTION_HEADER_HEIGHT;
    let mid_x = rect.size.x / 2.0;
    let before = panel
        .drop_target_at(rect, Point2D::new(mid_x, y0 + 4.0))
        .unwrap();
    assert_eq!(before.anchor, panel.items[0].node_id);
    assert_eq!(before.position, DropPosition::Before);
    assert!((before.indicator_y - y0).abs() < 0.5);
    let after = panel
        .drop_target_at(rect, Point2D::new(mid_x, y0 + LAYER_ROW_HEIGHT - 4.0))
        .unwrap();
    assert_eq!(after.position, DropPosition::After);
    assert!((after.indicator_y - (y0 + LAYER_ROW_HEIGHT)).abs() < 0.5);
}

#[test]
fn drop_target_at_in_empty_area_below_rows_drops_at_end() {
    let state = EditorState::sample();
    let panel = LayerPanel::from_editor(&state);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height() + 200.0),
    };
    let layers_top = 8.0
        + SECTION_HEADER_HEIGHT
        + panel.pages.len() as f32 * PAGE_ROW_HEIGHT
        + SECTION_GAP
        + SECTION_HEADER_HEIGHT;
    let rows_bottom = layers_top + panel.items.len() as f32 * LAYER_ROW_HEIGHT;
    let drop = panel
        .drop_target_at(rect, Point2D::new(rect.size.x / 2.0, rows_bottom + 50.0))
        .expect("below-rows hit should drop at end");
    assert_eq!(drop.position, DropPosition::After);
    assert_eq!(drop.anchor, panel.items.last().unwrap().node_id);
    assert!((drop.indicator_y - rows_bottom).abs() < 0.5);
}
