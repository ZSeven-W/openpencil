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
