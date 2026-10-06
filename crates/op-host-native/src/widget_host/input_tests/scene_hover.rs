//! Hover / selection presses that must not dirty or stale the derived
//! `LayoutScene` — plus the smart-guide refresh budget.
//!
//! Split out of `input_tests.rs` to keep every file under the repo's
//! 800-line cap.

use super::*;

#[test]
fn layer_hover_does_not_refresh_stale_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    host.mark_paint_dirty_for_test();

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = host.layers_content_rect(1200.0, 800.0);
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(!host.update_layer_hover(x, y, 1200.0, 800.0));
    assert!(
        host.editor_state_dirty,
        "layer hover should not rebuild or clear the stale canvas layout scene"
    );
}

#[test]
fn file_menu_cursor_move_clears_stale_layer_hover() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut().editor_ui.file_menu_open = true;
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    let menu_rect = host
        .file_menu_rect(host.last_viewport_w)
        .expect("file menu rect");
    let menu = op_editor_ui::widgets::file_menu::FileMenu::from_editor_ui(
        &host.editor_state().editor_ui,
        0,
    );
    let x = menu_rect.origin.x + 80.0;
    let mut y = menu_rect.origin.y + 2.0;
    let mut point = None;
    while y < menu_rect.origin.y + menu_rect.size.y {
        let p = op_editor_ui::Point2D::new(x, y);
        if menu.hovered_at(menu_rect, p).is_some() {
            point = Some(p);
            break;
        }
        y += 2.0;
    }
    let point = point.expect("file menu row point");
    assert!(host.over_dropdown_overlay(
        point.x,
        point.y,
        host.last_viewport_w,
        host.last_viewport_h
    ));

    assert!(host.apply_cursor_move(point.x, point.y));

    assert_eq!(host.editor_state().editor_ui.hovered_layer_id, None);
    assert_eq!(host.editor_state().editor_ui.hovered_page_index, None);
    assert!(
        host.editor_state().editor_ui.file_menu.hover.is_some(),
        "file-menu hover itself should still be active"
    );
}

#[test]
fn property_panel_blank_hover_consumes_and_clears_lower_hover() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "1.0.0", "children": [
              {"type":"group","id":"shape_group","name":"Shape Group",
               "children":[
                 {"type":"rectangle","id":"box","name":"Box","width":80,"height":40}
               ]}
        ]}"##,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("shape_group"));
    host.editor_state_mut().editor_ui.canvas_hover_node = Some(NodeId::new("box"));

    let point = point_inside_property_panel_without_target(&host);

    assert!(
        host.apply_cursor_move(point.x, point.y),
        "right inspector should own cursor movement inside its bounds"
    );
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
}

#[test]
fn opening_file_menu_clears_stale_layer_hover() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    host.editor_state_mut().editor_ui.hovered_page_index = Some(0);

    let top_bar_rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, 0.0),
        size: op_editor_ui::Point2D::new(host.last_viewport_w, TOP_BAR_HEIGHT),
    };
    let top_bar = TopBar::for_editor_ui(&host.editor_state().editor_ui);
    let mut file_button = None;
    let mut x = top_bar_rect.origin.x;
    while x < top_bar_rect.origin.x + top_bar_rect.size.x {
        let p = op_editor_ui::Point2D::new(x, TOP_BAR_HEIGHT / 2.0);
        if top_bar.hit_test(top_bar_rect, p) == Some(TopBarHit::ToggleFileMenu) {
            file_button = Some(p);
            break;
        }
        x += 1.0;
    }
    let point = file_button.expect("top-bar file button point");

    assert!(host.apply_press(point.x, point.y, host.last_viewport_w, host.last_viewport_h));

    assert!(host.editor_state().editor_ui.file_menu_open);
    assert_eq!(host.editor_state().editor_ui.hovered_layer_id, None);
    assert_eq!(host.editor_state().editor_ui.hovered_page_index, None);
}

#[test]
fn layer_row_selection_does_not_dirty_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50},{"type":"rectangle","id":"n2","name":"n2","x":120,"y":0,"width":100,"height":50}]}"#,
    );
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = host.layers_content_rect(1200.0, 800.0);
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n1"));
    assert!(
        !host.editor_state_dirty,
        "selecting a layer row should not invalidate canvas layout"
    );
}

#[test]
fn layer_right_press_does_not_dirty_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50},{"type":"rectangle","id":"n2","name":"n2","x":120,"y":0,"width":100,"height":50}]}"#,
    );
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = host.layers_content_rect(1200.0, 800.0);
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(host.apply_right_press(x, y, 1200.0, 800.0));
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n1"));
    assert!(host.editor_state().editor_ui.layer_context_menu.is_some());
    assert!(
        !host.editor_state_dirty,
        "right-clicking a layer row should not invalidate canvas layout"
    );
}

#[test]
fn layer_right_press_does_not_refresh_stale_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.mark_paint_dirty_for_test();

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = host.layers_content_rect(1200.0, 800.0);
    let regions = panel.regions(rect);
    let x = 48.0;
    let y = regions.layers_rows_top + 8.0;

    assert!(host.apply_right_press(x, y, 1200.0, 800.0));
    assert!(
        host.editor_state_dirty,
        "right-clicking a layer row should not rebuild or clear a stale canvas layout scene"
    );
}

#[test]
fn canvas_node_selection_does_not_dirty_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (360.0, 180.0, 80.0, 40.0),
                (480.0, 180.0, 80.0, 40.0),
                (600.0, 180.0, 80.0, 40.0),
            ],
            ["n10", "n11", "n12"],
        ),
    );
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    assert!(host.apply_press(
        cx0 + 360.0 + 20.0,
        cy0 + 180.0 + 20.0,
        viewport_w,
        viewport_h
    ));

    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
    assert!(
        !host.editor_state_dirty,
        "selecting a canvas node should not invalidate canvas layout"
    );
}

#[test]
fn node_drag_smart_guides_do_not_refresh_layout_scene_on_each_move() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (360.0, 180.0, 80.0, 40.0),
                (480.0, 180.0, 80.0, 40.0),
                (600.0, 180.0, 80.0, 40.0),
            ],
            ["n10", "n11", "n12"],
        ),
    );
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);
    let scene_x_before = host
        .layout_scene
        .active_page()
        .and_then(|p| p.find("n10"))
        .expect("scene node")
        .bounds
        .origin
        .x;

    assert!(host.apply_press(
        cx0 + 360.0 + 20.0,
        cy0 + 180.0 + 20.0,
        viewport_w,
        viewport_h
    ));
    assert!(host.node_drag.is_some(), "press should start a node drag");
    assert!(host.apply_cursor_move(cx0 + 360.0 + 30.0, cy0 + 180.0 + 20.0));

    let moved = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("n10"),
    )
    .expect("moved node");
    assert_eq!(own_bounds(moved).x, 370.0);
    let scene_x_after = host
        .layout_scene
        .active_page()
        .and_then(|p| p.find("n10"))
        .expect("patched scene node")
        .bounds
        .origin
        .x;
    assert_eq!(
        scene_x_after,
        scene_x_before + 10.0,
        "node drag should patch the current layout scene without a rebuild"
    );
    assert!(
        !host.editor_state_dirty,
        "node drag should not force a full layout-scene rebuild on every move"
    );
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert!(
        host.editor_state_dirty,
        "node drag release should schedule one full layout-scene reconciliation"
    );
}
