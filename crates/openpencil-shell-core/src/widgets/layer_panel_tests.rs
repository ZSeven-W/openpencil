//! Tests for `widgets::layer_panel` — moved to a sibling file to
//! keep `layer_panel.rs` under the 800-line cap.

use super::layer_panel::*;
use super::Widget;
use crate::document::{Document, Node, NodeId, NodeKind, Page};
use crate::{Point2D, Rect};

#[test]
fn from_sample_doc_flattens_to_5_layer_rows() {
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
    assert_eq!(panel.items.len(), 5);
    assert_eq!(panel.items[0].label, "Frame");
    assert_eq!(panel.items[0].depth, 0);
    assert_eq!(panel.items[1].depth, 1);
}

#[test]
fn from_sample_doc_has_one_active_page() {
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
    assert_eq!(panel.pages.len(), 1);
    assert!(panel.pages[0].active);
    assert_eq!(panel.pages[0].label, "Page 1");
}

#[test]
fn selection_flag_marks_only_selected_row() {
    let doc = Document::sample(); // selected = Title
    let panel = LayerPanel::from_document(&doc);
    let selected = panel.items.iter().filter(|i| i.selected).count();
    assert_eq!(selected, 1);
}

#[test]
fn empty_document_yields_one_default_page_no_layers() {
    let doc = Document::empty();
    let panel = LayerPanel::from_document(&doc);
    assert_eq!(panel.pages.len(), 1);
    assert!(panel.items.is_empty());
}

#[test]
fn hit_test_resolves_first_layer_row() {
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
    };
    // Skip pages section (header + 1 page row + section gap +
    // layers header) → land on the first layer row.
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
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(LAYER_PANEL_WIDTH, panel.intrinsic_height()),
    };
    // Mirror the paint geometry: plus_x = right edge - ROW_PAD_X
    // - 12, plus_y = 8 + (SECTION_HEADER_HEIGHT - 14) / 2.
    let plus_x = rect.size.x - ROW_PAD_X - 12.0;
    let plus_y = 8.0 + (SECTION_HEADER_HEIGHT - 14.0) / 2.0;
    // Centre of the 14 px icon.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(plus_x + 7.0, plus_y + 7.0)),
        Some(LayerPanelHit::AddPage)
    );
    // Edge-of-slop sample — 3 px LEFT of the icon's left edge,
    // inside the 4 px slop band. Locks the slop contract: a
    // regression that shrank slop below 3 px would fail here.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(plus_x - 3.0, plus_y + 7.0)),
        Some(LayerPanelHit::AddPage)
    );
}

#[test]
fn hit_test_resolves_first_page_row() {
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
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
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
    let node = panel.access_node();
    assert_eq!(node.role(), accesskit::Role::Tree);
    assert_eq!(node.label(), Some("Layers"));
}

#[test]
fn from_document_scopes_to_active_page_only() {
    let page1 = crate::document::Page::new(
        1,
        "Page 1",
        vec![Node::leaf(2, crate::document::NodeKind::Frame, "P1-Node")],
    );
    let page2 = crate::document::Page::new(
        3,
        "Page 2",
        vec![Node::leaf(4, crate::document::NodeKind::Frame, "P2-Node")],
    );
    let doc = Document {
        pages: vec![page1, page2],
        active_page_index: 1,
        selected: NodeId::NONE,
        selected_set: Vec::new(),
        clipboard: Vec::new(),
        tool: crate::document::Tool::Select,
        viewport: crate::document::Viewport::IDENTITY,
        chat: crate::document::ChatState::default(),
        ui: crate::document::UiState::default(),
        history: crate::document::History::default(),
    };
    let panel = LayerPanel::from_document(&doc);
    assert_eq!(panel.items.len(), 1);
    assert_eq!(panel.items[0].label, "P2-Node");
    assert_eq!(panel.pages.len(), 2);
    assert!(!panel.pages[0].active);
    assert!(panel.pages[1].active);
}

#[test]
fn drop_indicator_matches_post_commit_layout_when_dragging_down() {
    // Regression: the drop indicator could paint at one row, but
    // commit would land the source at a different row (preview lied
    // when dragging downward past other rows). With the source
    // excluded from the panel's item list, indicator_y == the source's
    // top edge in the post-commit panel.
    let mut doc = Document::empty();
    let p = doc.active_page_index;
    doc.pages[p].children = vec![
        Node::leaf(1, NodeKind::Rect, "A"),
        Node::leaf(2, NodeKind::Rect, "B"),
        Node::leaf(3, NodeKind::Rect, "C"),
        Node::leaf(4, NodeKind::Rect, "D"),
    ];
    let panel = LayerPanel::from_document_with_drag_source(&doc, NodeId::new(1));
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
    assert!(doc.reorder_before(NodeId::new(1), drop.anchor));
    let post = LayerPanel::from_document(&doc);
    let a_idx = post
        .items
        .iter()
        .position(|i| i.node_id == NodeId::new(1))
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
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
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
    let doc = Document::sample();
    let panel = LayerPanel::from_document(&doc);
    // Make the panel rect tall enough that there's real empty
    // space below the last layer row.
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
    // Cursor 50 px below the last row — still inside panel rect.
    let drop = panel
        .drop_target_at(rect, Point2D::new(rect.size.x / 2.0, rows_bottom + 50.0))
        .expect("below-rows hit should drop at end");
    assert_eq!(drop.position, DropPosition::After);
    assert_eq!(drop.anchor, panel.items.last().unwrap().node_id);
    assert!((drop.indicator_y - rows_bottom).abs() < 0.5);
}
