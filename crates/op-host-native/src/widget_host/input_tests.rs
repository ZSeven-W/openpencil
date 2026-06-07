//! `#[cfg(test)]` companion to the input modules — extracted here so
//! the input module stays under the 800-line ceiling.
//!
//! `EditorState` is the host's source of truth, so the fixtures seed
//! `host.editor_state` from canonical-schema JSON and assert against
//! `editor_state` + the derived `LayoutScene` render scene.

use super::{CursorHint, WidgetHostNative};
use op_editor_core::ui_draft::PropertyFocus;
use op_editor_core::PenNodeExt;
use op_editor_core::{own_bounds, NodeId};

/// Seed a host's `editor_state` from a canonical `.op` JSON snippet.
fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

#[test]
fn layer_hover_does_not_refresh_stale_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.editor_state_mut().editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    host.mark_paint_dirty_for_test();

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
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
fn layer_row_selection_does_not_dirty_canvas_layout_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50},{"type":"rectangle","id":"n2","name":"n2","x":120,"y":0,"width":100,"height":50}]}"#,
    );
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
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
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50},{"type":"rectangle","id":"n2","name":"n2","x":120,"y":0,"width":100,"height":50}]}"#,
    );
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
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
        r#"{"version":"0.8.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.mark_paint_dirty_for_test();

    let panel = op_editor_ui::widgets::LayerPanel::from_editor(host.editor_state());
    let rect = op_editor_ui::Rect {
        origin: op_editor_ui::Point2D::new(0.0, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        size: op_editor_ui::Point2D::new(
            host.editor_state().editor_ui.layer_panel_width,
            800.0 - op_editor_ui::widgets::TOP_BAR_HEIGHT,
        ),
    };
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

/// Three top-level rect nodes at the given `(x, y, w, h)` boxes.
fn three_rects(boxes: [(f64, f64, f64, f64); 3], ids: [&str; 3]) -> String {
    let node = |id: &str, b: (f64, f64, f64, f64)| {
        format!(
            r#"{{"type":"rectangle","id":"{id}","name":"{id}",
               "x":{},"y":{},"width":{},"height":{}}}"#,
            b.0, b.1, b.2, b.3
        )
    };
    format!(
        r#"{{"version":"0.8.0","children":[{},{},{}]}}"#,
        node(ids[0], boxes[0]),
        node(ids[1], boxes[1]),
        node(ids[2], boxes[2]),
    )
}

#[test]
fn ai_chat_maximize_click_expands_panel_geometry() {
    let mut host = WidgetHostNative::new();
    let before = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = before.origin.x + before.size.x - 16.0 - 50.0 + 9.0;
    let y = before.origin.y + 17.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    let after = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("maximized chat panel visible");
    assert!(host.editor_state().chat.maximized);
    assert!(after.size.x > before.size.x);
    assert!(after.size.y > before.size.y);
}

#[test]
fn ai_chat_new_chat_click_clears_transcript_and_queues_abort() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant("old"));
    host.editor_state_mut().chat.input = "draft".into();
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 16.0 - 22.0 + 9.0;
    let y = rect.origin.y + 17.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(host.editor_state().chat.messages.is_empty());
    assert!(host.editor_state().chat.input.is_empty());
    assert!(host.editor_state().chat.pending_new_chat);
}

#[test]
fn ai_chat_east_edge_shows_resize_cursor() {
    let host = WidgetHostNative::new();
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 2.0;
    let y = rect.origin.y + rect.size.y / 2.0;

    assert_eq!(host.cursor_hint(x, y, 1200.0, 800.0), CursorHint::ResizeEw);
}

#[test]
fn ai_chat_east_edge_drag_resizes_panel_width() {
    let mut host = WidgetHostNative::new();
    let before = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = before.origin.x + before.size.x - 2.0;
    let y = before.origin.y + before.size.y / 2.0;

    assert!(host.apply_press(x, y, 1200.0, 800.0));
    assert!(host.apply_cursor_move(x + 72.0, y));

    let after = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible after resize");
    assert!(
        after.size.x > before.size.x + 60.0,
        "dragging the east edge should grow chat width; before={before:?}, after={after:?}"
    );
    assert_eq!(after.origin.x, before.origin.x);
}

#[test]
fn ai_chat_stop_click_keeps_transcript_and_queues_abort() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::user("make a dashboard"));
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + rect.size.x - 16.0 - 20.0;
    let y = rect.origin.y + toolbar_center_y_for_test();

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert_eq!(host.editor_state().chat.messages.len(), 2);
    assert!(!host.editor_state().chat.messages[1].streaming);
    assert!(host.editor_state().chat.pending_stop_chat);
}

#[test]
fn ai_chat_streaming_textarea_click_does_not_focus_disabled_input() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + 120.0;
    let y = rect.origin.y + textarea_center_y_for_test();

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(!host.editor_state().chat.focused);
}

#[test]
fn ai_chat_streaming_attachment_click_does_not_open_picker() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());
    let rect = host
        .ai_chat_rect(1200.0, 800.0)
        .expect("chat panel visible");
    let x = rect.origin.x + op_editor_ui::widgets::AI_CHAT_WIDTH - 16.0 - 52.0;
    let y = rect.origin.y + toolbar_center_y_for_test();

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    assert!(!host.editor_state().chat.pending_attachment_pick);
}

fn toolbar_center_y_for_test() -> f32 {
    op_editor_ui::widgets::AI_CHAT_HEIGHT - 19.0
}

fn textarea_center_y_for_test() -> f32 {
    const INPUT_AREA_HEIGHT: f32 = 56.0;
    const INPUT_TOOLBAR_HEIGHT: f32 = 40.0;
    op_editor_ui::widgets::AI_CHAT_HEIGHT - (INPUT_AREA_HEIGHT + INPUT_TOOLBAR_HEIGHT)
        + 1.0
        + INPUT_AREA_HEIGHT / 2.0
}

#[test]
fn escape_closes_one_overlay_per_press_in_priority_order() {
    // Codex CONCERN-2 regression: Escape used to clear all
    // three pickers in a single press. TS parity is one-at-a-
    // time, in the order property-focus → locale → shape →
    // fill-type → chat → selection.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input_draft = "12".to_string();
    // Focusing an input seeds the caret at the draft's end (the
    // press path does this); mirror it so the state is faithful.
    host.editor_state_mut().ui.property_caret_pos = 2;
    host.editor_state_mut().editor_ui.locale_picker_open = true;
    host.editor_state_mut().editor_ui.shape_picker_open = true;
    host.editor_state_mut().editor_ui.fill_type_picker_open = true;
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));

    // 1. Property focus clears first.
    assert!(host.apply_escape());
    assert!(host.editor_state().ui.property_focus.is_none());
    assert!(host.editor_state().ui.property_input_draft.is_empty());
    assert!(host.editor_state().editor_ui.locale_picker_open);

    // 2. Locale picker next.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.locale_picker_open);
    assert!(host.editor_state().editor_ui.shape_picker_open);

    // 3. Shape picker.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.shape_picker_open);
    assert!(host.editor_state().editor_ui.fill_type_picker_open);

    // 4. Fill-type picker.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.fill_type_picker_open);
    assert!(host.editor_state().chat.focused);

    // 5. Chat focus.
    assert!(host.apply_escape());
    assert!(!host.editor_state().chat.focused);
    assert!(!host.editor_state().selection.is_empty());

    // 6. Selection.
    assert!(host.apply_escape());
    assert!(host.editor_state().selection.is_empty());

    // 7. Nothing left — returns false.
    assert!(!host.apply_escape());
}

#[test]
fn rename_caret_arrows_move_caret_then_fall_through() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (20.0, 0.0, 10.0, 10.0),
                (40.0, 0.0, 10.0, 10.0),
            ],
            ["ab", "b", "c"],
        ),
    );
    assert!(host
        .editor_state_mut()
        .start_rename_layer(NodeId::new("ab")));
    // Draft "ab" seeds caret at the end (2).
    assert_eq!(
        host.editor_state().ui.layer_rename.as_ref().unwrap().caret,
        2
    );
    // Left arrow during rename is consumed and moves the caret.
    assert!(host.apply_rename_caret(false));
    assert_eq!(
        host.editor_state().ui.layer_rename.as_ref().unwrap().caret,
        1
    );
    assert!(host.apply_rename_caret(true));
    assert_eq!(
        host.editor_state().ui.layer_rename.as_ref().unwrap().caret,
        2
    );
    // With no rename active the arrow falls through (not consumed).
    host.editor_state_mut().rename_cancel();
    assert!(!host.apply_rename_caret(false));
}

#[test]
fn status_bar_search_click_frames_content_in_viewport() {
    // Three rects spread across doc space (union ≈ x[100,400] y[100,300]).
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (100.0, 100.0, 100.0, 100.0),
                (300.0, 200.0, 100.0, 100.0),
                (150.0, 150.0, 50.0, 50.0),
            ],
            ["a", "b", "c"],
        ),
    );
    // Pan + zoom far away so the design is off-screen.
    host.editor_state_mut().viewport.pan_x = -5000.0;
    host.editor_state_mut().viewport.pan_y = -5000.0;
    host.editor_state_mut().viewport.zoom = 0.2;

    let (vw, vh) = (1200.0, 800.0);
    let r = host
        .status_bar_rect(vw, vh)
        .expect("status bar visible at this size");
    // Click the search icon (left section of the pill).
    let consumed = host.apply_press(r.origin.x + 5.0, r.origin.y + r.size.y / 2.0, vw, vh);

    assert!(consumed, "search-icon click must be consumed");
    let v = host.editor_state().viewport;
    assert!((v.zoom - 1.0).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 230.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 180.0).abs() < 1e-2, "pan_y {}", v.pan_y);
}

#[test]
fn pick_fill_image_keeps_image_popover_open_for_mode_selection() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.image_fill_popover_open = true;

    host.apply_property_action(op_editor_ui::widgets::PropertyPanelAction::PickFillImage);

    assert_eq!(
        host.editor_state().editor_ui.pending_file_action,
        Some(op_editor_core::editor_ui_state::FileAction::PickFillImage),
    );
    assert!(
        host.editor_state().editor_ui.image_fill_popover_open,
        "the image popover must stay open so Fill/Fit/Crop/Tile remain selectable",
    );
}

#[test]
fn image_adjustment_drag_updates_live_after_press() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n60","name":"Photo fill",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"image","url":"","mode":"fill",
                 "exposure":0,"contrast":0,"saturation":0,
                 "temperature":0,"tint":0,"highlights":0,"shadows":0}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.editor_state_mut().editor_ui.image_fill_popover_open = true;
    host.image_adjustment_drag = Some(op_editor_core::ImageAdjustmentField::Exposure);
    host.last_viewport_w = 900.0;
    host.last_viewport_h = 760.0;

    assert!(host.apply_cursor_move(0.0, 0.0));

    let node = host
        .editor_state()
        .selected_node()
        .expect("selected image-fill node");
    match op_editor_core::fills::node_fills(node)
        .unwrap()
        .first()
        .unwrap()
    {
        jian_ops_schema::style::PenFill::Image(body) => {
            assert_eq!(body.exposure, Some(-100.0));
        }
        other => panic!("expected image fill, got {other:?}"),
    }
}

#[test]
fn image_fill_actions_refresh_the_render_scene() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n61","name":"Photo fill",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"image","url":"data:image/png;base64,AA==","mode":"fill",
                 "exposure":0,"contrast":0,"saturation":0,
                 "temperature":0,"tint":0,"highlights":0,"shadows":0}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n61"));
    host.mark_paint_dirty_for_test();

    let initial_fit = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("n61")
        .unwrap()
        .image_fit;
    assert_eq!(initial_fit, op_editor_ui::layout_scene::SceneImageFit::Fill);

    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::SetImageFillMode(
            op_editor_core::ImageFillMode::Fit,
        ),
    );
    host.apply_property_action(
        op_editor_ui::widgets::PropertyPanelAction::SetImageAdjustment {
            field: op_editor_core::ImageAdjustmentField::Exposure,
            value: 64.0,
        },
    );

    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("n61")
        .unwrap();
    assert_eq!(
        rendered.image_fit,
        op_editor_ui::layout_scene::SceneImageFit::Fit
    );
    assert_eq!(rendered.image_adjustments.exposure, 64.0);
}

#[test]
fn corner_radius_property_focus_updates_selected_rectangle() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Rounded",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionR);
    host.editor_state_mut().ui.property_input_draft = "24".to_string();

    host.commit_property_focus_if_any();

    let node = host.editor_state().selected_node().unwrap();
    match node {
        jian_ops_schema::node::PenNode::Rectangle(rect) => {
            assert_eq!(
                rect.container.corner_radius,
                Some(jian_ops_schema::node::container::CornerRadius::Uniform(
                    24.0
                )),
            );
        }
        other => panic!("expected rectangle, got {other:?}"),
    }
    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("n62")
        .unwrap();
    assert_eq!(rendered.corner_radius, 24.0);
}

#[test]
fn polygon_sides_property_focus_updates_selected_polygon() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"polygon","id":"poly","name":"Polygon",
               "x":40,"y":40,"width":120,"height":120,
               "polygonCount":3}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("poly"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PolygonSides);
    host.editor_state_mut().ui.property_input_draft = "7".to_string();

    host.commit_property_focus_if_any();

    let node = host.editor_state().selected_node().unwrap();
    match node {
        jian_ops_schema::node::PenNode::Polygon(poly) => {
            assert_eq!(poly.polygon_count, 7);
        }
        other => panic!("expected polygon, got {other:?}"),
    }
    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("poly")
        .unwrap();
    assert_eq!(rendered.polygon_sides, 7);
}

#[test]
fn ellipse_arc_property_focus_updates_selected_ellipse() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"ellipse","id":"ell","name":"Ellipse",
               "x":40,"y":40,"width":120,"height":100}
        ]}"##,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("ell"));

    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::EllipseStart);
    host.editor_state_mut().ui.property_input_draft = "45".to_string();
    host.commit_property_focus_if_any();

    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::EllipseSweep);
    host.editor_state_mut().ui.property_input_draft = "180".to_string();
    host.commit_property_focus_if_any();

    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::EllipseInnerRadius);
    host.editor_state_mut().ui.property_input_draft = "25".to_string();
    host.commit_property_focus_if_any();

    let node = host.editor_state().selected_node().unwrap();
    match node {
        jian_ops_schema::node::PenNode::Ellipse(ell) => {
            assert_eq!(ell.start_angle, Some(45.0));
            assert_eq!(ell.sweep_angle, Some(180.0));
            assert_eq!(ell.inner_radius, Some(0.25));
        }
        other => panic!("expected ellipse, got {other:?}"),
    }
    let rendered = host
        .layout_scene()
        .active_page()
        .unwrap()
        .find("ell")
        .unwrap();
    assert_eq!(rendered.arc_start_angle, Some(45.0));
    assert_eq!(rendered.arc_sweep_angle, Some(180.0));
    assert_eq!(rendered.arc_inner_radius, Some(0.25));
}

#[test]
fn backspace_with_property_draft_does_not_delete_selected() {
    // With a non-empty property draft buffer, Backspace must pop a
    // char from the draft, not delete the selected node.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input_draft = "123".to_string();
    // Caret at the draft's end, as a real focus seeds it — Backspace
    // deletes the char *before* the caret.
    host.editor_state_mut().ui.property_caret_pos = 3;

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().ui.property_input_draft, "12");
    assert_eq!(host.editor_state().ui.property_caret_pos, 2);
    // Selection must be untouched.
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn backspace_without_focus_deletes_selected() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().ui.property_focus = None;
    host.editor_state_mut().chat.focused = false;

    assert!(host.apply_backspace());
    assert!(host.editor_state().selection.is_empty());
}

#[test]
fn delete_with_model_picker_open_does_not_delete_selected() {
    // The chat model-picker search owns the keyboard while open, so
    // Delete must be swallowed instead of dropping the canvas node
    // behind the dropdown.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.chat_model_picker_open = true;

    assert!(!host.apply_delete());
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn backspace_with_model_picker_open_edits_search_not_selection() {
    // Backspace pops from the model-picker search query, never the
    // selected node.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.chat_model_picker_open = true;
    host.editor_state_mut().editor_ui.chat_model_picker_search = "gp".to_string();

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.chat_model_picker_search, "g");
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn shortcuts_gated_while_model_picker_open() {
    // Nudge / duplicate / reorder all route through `input_active`,
    // which must report the model picker as owning the keyboard.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));
    host.editor_state_mut().editor_ui.chat_model_picker_open = true;

    assert!(!host.apply_nudge(1.0, 0.0));
    assert!(!host.apply_duplicate());
    assert!(!host.apply_reorder(op_editor_core::ReorderDirection::Up));
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n10"));
}

#[test]
fn marquee_drag_replaces_selection_with_intersecting_nodes() {
    let mut host = WidgetHostNative::new();
    // 3 rects: two close together near origin, one far away.
    seed(
        &mut host,
        &three_rects(
            [
                (50.0, 10.0, 20.0, 20.0),
                (90.0, 10.0, 20.0, 20.0),
                (200.0, 200.0, 20.0, 20.0),
            ],
            ["n50", "n51", "n52"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
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
    let mut hits: Vec<String> = host
        .editor_state()
        .selection
        .set
        .iter()
        .map(|i| i.as_str().to_string())
        .collect();
    hits.sort();
    assert_eq!(hits, vec!["n50", "n51"]);
}

#[test]
fn marquee_drag_with_shift_preserves_already_selected_hit() {
    // Codex CONCERN-Q2 regression: shift-marquee must be ADD-only.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (50.0, 50.0, 20.0, 20.0),
                (300.0, 300.0, 20.0, 20.0),
                (900.0, 900.0, 20.0, 20.0),
            ],
            ["n70", "n71", "n72"],
        ),
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n70"));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    host.apply_cursor_move(cx0 + 90.0, cy0 + 90.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    // "n70" stays in the set (shift-marquee is ADD-only).
    assert!(host.editor_state().is_selected(&NodeId::new("n70")));
    assert_eq!(host.editor_state().selection.set.len(), 1);
}

#[test]
fn marquee_drag_below_screen_threshold_is_a_no_op() {
    // Codex CONCERN-Q5 regression: threshold is screen-px.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 100.0, 100.0),
                (5000.0, 5000.0, 10.0, 10.0),
                (6000.0, 6000.0, 10.0, 10.0),
            ],
            ["n80", "n81", "n82"],
        ),
    );
    host.editor_state_mut().viewport.zoom = 0.1;
    host.editor_state_mut().clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 100.0, cy0 + 50.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    // Tiny drag: 1 screen-px — below the 2-px threshold.
    host.apply_cursor_move(cx0 + 101.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    assert!(host.editor_state().selection.set.is_empty());
}

#[test]
fn marquee_drag_with_shift_extends_existing_selection() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (10.0, 10.0, 20.0, 20.0),
                (50.0, 10.0, 20.0, 20.0),
                (300.0, 300.0, 20.0, 20.0),
            ],
            ["n60", "n61", "n62"],
        ),
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    host.apply_cursor_move(cx0 + 130.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    let mut ids: Vec<String> = host
        .editor_state()
        .selection
        .set
        .iter()
        .map(|i| i.as_str().to_string())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["n60", "n61", "n62"]);
}

#[test]
fn layer_drag_to_reorder_commits_on_release_with_threshold_move() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    // Three top-level nodes painted as flat layer rows.
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
            ],
            ["n70", "n71", "n72"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let row_h = 28.0; // LAYER_ROW_HEIGHT
    let page_row_h = 32.0; // PAGE_ROW_HEIGHT
    let section_header_h = 28.0;
    let section_gap = 8.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let layers_top =
        TOP_BAR_HEIGHT + 8.0 + section_header_h + page_row_h + section_gap + section_header_h;
    let row_y = |i: usize| layers_top + (i as f32) * row_h + row_h / 2.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    host.apply_press(row_x, row_y(0), viewport_w, viewport_h);
    assert!(host.layer_drag.is_some());
    assert!(!host.layer_drag.as_ref().unwrap().active);
    host.apply_cursor_move(row_x, row_y(2) + row_h / 2.0 - 4.0);
    assert!(host.layer_drag.as_ref().unwrap().active);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    assert!(host.layer_drag.is_none(), "drag must be cleared on release");
    // A moved after C → final order [B, C, A].
    let order: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|n| n.base().id.clone())
        .collect();
    assert_eq!(order, vec!["n71", "n72", "n70"]);
}

#[test]
fn layer_drag_below_activation_threshold_is_a_click_not_a_reorder() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
                (5000.0, 5000.0, 10.0, 10.0),
            ],
            ["n80", "n81", "n82"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let row_y_first = TOP_BAR_HEIGHT + 8.0 + 28.0 + 32.0 + 8.0 + 28.0 + 14.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.apply_press(row_x, row_y_first, viewport_w, viewport_h);
    host.apply_cursor_move(row_x, row_y_first + 2.0);
    assert!(
        host.layer_drag.is_some() && !host.layer_drag.as_ref().unwrap().active,
        "sub-threshold move must not activate"
    );
    host.apply_release_with_viewport(viewport_w, viewport_h);
    let order: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|n| n.base().id.clone())
        .collect();
    assert_eq!(order, vec!["n80", "n81", "n82"]);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n80"));
}

#[test]
fn layer_context_create_component_click_promotes_frame() {
    use op_editor_core::editor_ui_state::LayerContextMenuState;
    use op_editor_core::ui_draft::LayerContextTarget;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"n10","name":"Card","x":0,"y":0,"width":100,"height":80,
           "children":[]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.layer_context_menu = Some(LayerContextMenuState {
        target: LayerContextTarget::Layer(NodeId::new("n10")),
        anchor_x: 100.0,
        anchor_y: 100.0,
        hovered_row: None,
    });

    let create_row_y = 100.0 + 6.0 + 32.0 * 2.0 + 16.0;
    assert!(host.apply_press(120.0, create_row_y, 1440.0, 900.0));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("n10"))
        .is_some());
    match &host.editor_state().doc.children[0] {
        jian_ops_schema::node::PenNode::Frame(f) => assert_eq!(f.reusable, Some(true)),
        _ => panic!("expected frame"),
    }
}

#[test]
fn property_panel_create_component_click_promotes_selected_frame() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"frame","id":"n20","name":"Hero","x":0,"y":0,"width":120,"height":90,
           "children":[]}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n20"));

    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let panel_left = viewport_w - host.editor_state().editor_ui.property_panel_width;
    let button_x = panel_left + 24.0;
    let button_y = TOP_BAR_HEIGHT + 36.0 + 30.0 + 8.0 + 18.0;
    assert!(host.apply_press(button_x, button_y, viewport_w, viewport_h));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("n20"))
        .is_some());
}

#[test]
fn layer_context_group_preserves_multi_selection_and_groups() {
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 20.0, 20.0),
                (40.0, 0.0, 20.0, 20.0),
                (120.0, 0.0, 20.0, 20.0),
            ],
            ["n30", "n31", "n32"],
        ),
    );
    host.editor_state_mut().selection.set = vec![NodeId::new("n30"), NodeId::new("n31")];
    host.editor_state_mut().selection.anchor = NodeId::new("n30");

    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    let first_row_y = TOP_BAR_HEIGHT + 8.0 + 28.0 + 32.0 + 8.0 + 28.0 + 14.0;
    assert!(host.apply_right_press(row_x, first_row_y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().selection.set.len(),
        2,
        "right-clicking an already-selected layer must keep the multi-selection"
    );

    let group_row_y = first_row_y + 6.0 + 32.0 * 2.0 + 16.0;
    assert!(host.apply_press(row_x + 20.0, group_row_y, viewport_w, viewport_h));
    assert!(matches!(
        host.editor_state().doc.children.first(),
        Some(jian_ops_schema::node::PenNode::Group(_))
    ));
}

#[test]
fn component_browser_open_owns_keyboard_search() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.component_browser_open = true;

    assert!(host.input_active_pub());
    assert!(host.apply_text('b'));
    assert!(host.apply_text('a'));
    assert_eq!(host.editor_state().editor_ui.component_browser_search, "ba");
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.component_browser_search, "b");
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.component_browser_open);
}

#[test]
fn chat_model_picker_open_owns_keyboard_search() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.chat_model_picker_open = true;
    host.editor_state_mut().chat.focused = true;

    assert!(host.input_active_pub());
    assert!(host.apply_text('g'));
    assert!(host.apply_text('p'));
    assert_eq!(host.editor_state().editor_ui.chat_model_picker_search, "gp");
    assert!(host.editor_state().chat.input.is_empty());
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.chat_model_picker_search, "g");
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.chat_model_picker_open);
}

#[test]
fn shape_picker_icon_row_opens_icon_picker() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.editor_state_mut().editor_ui.shape_picker_open = true;

    let panel = host.shape_picker_rect(viewport_w, viewport_h);
    let icon_row_y = panel.origin.y + 6.0 + 32.0 * 4.0 + 16.0;
    assert!(host.apply_press(panel.origin.x + 24.0, icon_row_y, viewport_w, viewport_h));

    assert!(!host.editor_state().editor_ui.shape_picker_open);
    assert!(host.editor_state().editor_ui.icon_picker_open);
}

#[test]
fn icon_picker_open_owns_keyboard_search() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.icon_picker_open = true;

    assert!(host.input_active_pub());
    assert!(host.apply_text('h'));
    assert!(host.apply_text('o'));
    assert_eq!(host.editor_state().editor_ui.icon_picker_search, "ho");
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().editor_ui.icon_picker_search, "h");
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.icon_picker_open);
}

#[test]
fn icon_picker_click_inserts_icon_font_node() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.editor_state_mut().editor_ui.icon_picker_open = true;
    host.editor_state_mut().editor_ui.icon_picker_search = "home".to_string();

    let panel = host
        .icon_picker_panel_rect(viewport_w, viewport_h)
        .expect("icon picker rect");
    let row_y = panel.origin.y + 40.0 + 42.0 + 20.0;
    assert!(host.apply_press(panel.origin.x + 40.0, row_y, viewport_w, viewport_h));

    assert!(!host.editor_state().editor_ui.icon_picker_open);
    let icon = host
        .editor_state()
        .doc
        .children
        .iter()
        .find_map(|node| match node {
            jian_ops_schema::node::PenNode::IconFont(icon) => Some(icon),
            _ => None,
        })
        .expect("inserted icon_font node");
    assert_eq!(icon.icon_font_name, "home");
    assert_eq!(icon.icon_font_family.as_deref(), Some("lucide"));
    assert_eq!(host.editor_state().selection.anchor.as_str(), icon.base.id);
}

#[test]
fn icon_picker_header_drag_moves_the_panel() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.editor_state_mut().editor_ui.icon_picker_open = true;

    let start = host
        .icon_picker_panel_rect(viewport_w, viewport_h)
        .expect("icon picker rect");
    let press_x = start.origin.x + 72.0;
    let press_y = start.origin.y + 20.0;
    assert!(host.apply_press(press_x, press_y, viewport_w, viewport_h));

    assert!(host.apply_cursor_move(press_x + 96.0, press_y + 44.0));
    let moved = host
        .icon_picker_panel_rect(viewport_w, viewport_h)
        .expect("icon picker rect after drag");

    assert_eq!(moved.origin.x, start.origin.x + 96.0);
    assert_eq!(moved.origin.y, start.origin.y + 44.0);
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
}

#[test]
fn codegen_select_framework_updates_state() {
    use op_editor_core::codegen::Framework;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();

    host.apply_property_action(PropertyPanelAction::Codegen(
        CodegenAction::SelectFramework(Framework::Vue),
    ));

    assert_eq!(host.editor_state().codegen.framework, Framework::Vue);
}

#[test]
fn codegen_generate_raises_pending_and_generating_phase() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));

    assert!(host.editor_state().codegen.pending_generate);
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Generating);
}

#[test]
fn codegen_copy_queues_code_to_system_clipboard() {
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    host.set_now_ms(4242);
    host.editor_state_mut().codegen.code = "export const App = () => null;".to_string();

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Copy));

    assert_eq!(host.editor_state().codegen.copied_at, Some(4242));
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("export const App = () => null;"),
    );
}

#[test]
fn codegen_cancel_resets_phase_by_code_presence() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    // No code yet → Cancel falls back to Idle.
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Cancel));
    assert!(!host.editor_state().codegen.pending_generate);
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Idle);

    // With code present → Cancel returns to Complete.
    host.editor_state_mut().codegen.code = "rendered".to_string();
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Cancel));
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Complete);
}
