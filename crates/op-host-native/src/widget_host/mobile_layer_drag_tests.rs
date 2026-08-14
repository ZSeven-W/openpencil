use super::*;
use jian_ops_schema::PenDocument;
use op_editor_core::{EditorState, PenNodeExt};
use op_editor_ui::widgets::{DropPosition, LayerPanel, LayerPanelHit};

const VIEWPORT_W: f32 = 390.0;
const VIEWPORT_H: f32 = 844.0;

fn document_with_rects(count: usize) -> PenDocument {
    serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": (0..count).map(|index| serde_json::json!({
            "type": "rectangle",
            "id": format!("n{index}"),
            "name": format!("Rect {index}"),
            "x": 0,
            "y": index * 12,
            "width": 10,
            "height": 10
        })).collect::<Vec<_>>()
    }))
    .expect("valid mobile layer fixture")
}

fn layer_host(count: usize) -> WidgetHostNative {
    let mut state = EditorState::from_document(document_with_rects(count));
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    state.editor_ui.mobile_sheet = Some(MobileSheetKind::Layers);
    state.editor_ui.sidebar_open = false;
    let mut host = WidgetHostNative::new();
    assert!(host.replace_editor_state(state));
    host.publish_viewport_geometry(VIEWPORT_W, VIEWPORT_H);
    host
}

fn order(host: &WidgetHostNative) -> Vec<String> {
    host.editor_state()
        .active_children()
        .iter()
        .map(|node| node.id_str().to_string())
        .collect()
}

fn grip_point(host: &WidgetHostNative, source: &str) -> Point2D {
    let rect = host.layers_content_rect(VIEWPORT_W, VIEWPORT_H);
    let panel = host.layer_panel();
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if panel
                .drag_source_at(rect, point)
                .is_some_and(|id| id.as_str() == source)
            {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("visible reorder grip for {source}");
}

fn row_body_point(host: &WidgetHostNative) -> Point2D {
    let rect = host.layers_content_rect(VIEWPORT_W, VIEWPORT_H);
    let panel = host.layer_panel();
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            let point = Point2D::new(x, y);
            if matches!(panel.hit_test(rect, point), Some(LayerPanelHit::Layer(_)))
                && panel.drag_source_at(rect, point).is_none()
            {
                return point;
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("visible layer row body");
}

fn drop_point(host: &WidgetHostNative, source: &str, anchor: &str) -> Point2D {
    let rect = host.layers_content_rect(VIEWPORT_W, VIEWPORT_H);
    let panel = LayerPanel::from_editor_with_drag_source(host.editor_state(), &NodeId::new(source));
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let point = Point2D::new(rect.origin.x + 20.0, y);
        if panel.drop_target_at(rect, point).is_some_and(|drop| {
            drop.anchor.as_str() == anchor && drop.position == DropPosition::After
        }) {
            return point;
        }
        y += 1.0;
    }
    panic!("after-{anchor} drop target");
}

#[test]
fn touch_grip_reorders_once_and_pushes_undo_history() {
    let mut host = layer_host(3);
    let grip = grip_point(&host, "n0");
    let drop = drop_point(&host, "n0", "n2");
    let history_before = host.editor_state().history.past.len();

    assert!(host.apply_press(grip.x, grip.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_none());
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n0"));
    assert!(host.layer_drag.as_ref().is_some_and(|drag| !drag.active));
    assert!(host.apply_cursor_move(drop.x, drop.y));
    assert!(host.layer_drag.as_ref().is_some_and(|drag| drag.active));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(order(&host), vec!["n1", "n2", "n0"]);
    assert_eq!(host.editor_state().history.past.len(), history_before + 1);
    assert!(host.editor_state_mut().undo());
    assert_eq!(order(&host), vec!["n0", "n1", "n2"]);
}

#[test]
fn row_body_still_scrolls_and_never_seeds_reorder() {
    let mut host = layer_host(40);
    let point = row_body_point(&host);
    let before = order(&host);

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.touch_panel_gesture.is_some());
    assert!(host.layer_drag.is_none());
    assert!(host.apply_cursor_move(point.x, point.y - 24.0));
    assert!(host.layer_drag.is_none());
    assert!(host.editor_state().editor_ui.layer_layers_scroll.offset > 0.0);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(order(&host), before);
}

#[test]
fn committing_a_touch_rename_cannot_arm_the_newly_revealed_grip() {
    let mut host = layer_host(40);
    let former_grip = grip_point(&host, "n0");
    let order_before = order(&host);
    let history_before = host.editor_state().history.past.len();
    assert!(host
        .editor_state_mut()
        .start_rename_layer(NodeId::new("n0")));

    assert!(host.apply_press(former_grip.x, former_grip.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state().ui.layer_rename.is_none());
    assert!(host.layer_drag.is_none());
    assert!(host.touch_panel_gesture.is_some());
    assert!(host.apply_cursor_move(former_grip.x, former_grip.y - 24.0));
    assert!(host.layer_drag.is_none());
    assert!(host.editor_state().editor_ui.layer_layers_scroll.offset > 0.0);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(order(&host), order_before);
    assert_eq!(host.editor_state().history.past.len(), history_before);
}

#[test]
fn deferred_touch_replay_can_never_arm_a_reorder_grip() {
    let mut host = layer_host(3);
    let grip = grip_point(&host, "n0");

    assert!(host.replay_touch_panel_press(grip.x, grip.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.layer_drag.is_none());
}

#[test]
fn cancelling_an_active_touch_reorder_never_commits() {
    let mut host = layer_host(3);
    let grip = grip_point(&host, "n0");
    let drop = drop_point(&host, "n0", "n2");
    let before = order(&host);
    let history_before = host.editor_state().history.past.len();

    assert!(host.apply_press(grip.x, grip.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(drop.x, drop.y));
    assert!(host.layer_drag.as_ref().is_some_and(|drag| drag.active));
    assert!(host.cancel_native_touch_gestures());
    assert!(!host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(order(&host), before);
    assert_eq!(host.editor_state().history.past.len(), history_before);
}

#[test]
fn active_touch_reorder_auto_scrolls_at_both_list_edges() {
    let mut host = layer_host(40);
    let grip = grip_point(&host, "n0");
    let rect = host.layers_content_rect(VIEWPORT_W, VIEWPORT_H);
    let regions = host.layer_panel().regions(rect);
    let top = regions.layers_rows_top + 1.0;
    let bottom = regions.layers_rows_top + regions.layers_view_h - 1.0;

    assert!(host.apply_press(grip.x, grip.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_cursor_move(grip.x, grip.y + 13.0));
    assert!(host.layer_drag.as_ref().is_some_and(|drag| drag.active));
    let initial = host.editor_state().editor_ui.layer_layers_scroll.offset;
    for _ in 0..4 {
        assert!(host.apply_cursor_move(grip.x, bottom));
    }
    let after_bottom = host.editor_state().editor_ui.layer_layers_scroll.offset;
    assert!(after_bottom > initial);
    let drag = host.layer_drag.as_ref().expect("captured reorder");
    let panel = LayerPanel::from_editor_with_drag_source(host.editor_state(), &drag.source);
    assert!(panel
        .drop_target_at(rect, Point2D::new(drag.current_x, drag.current_y))
        .is_some());

    for _ in 0..4 {
        assert!(host.apply_cursor_move(grip.x, top));
    }
    assert!(host.editor_state().editor_ui.layer_layers_scroll.offset < after_bottom);
}
