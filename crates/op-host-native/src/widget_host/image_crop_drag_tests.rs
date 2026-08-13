//! Native canvas regression coverage for Figma-style image-fill crop panning.

use super::WidgetHostNative;
use op_editor_core::{primary_image_fill_transform, NodeId, PenNodeExt};
use op_editor_ui::Rect;

const VIEWPORT_W: f32 = 1440.0;
const VIEWPORT_H: f32 = 900.0;
const PHOTO_ID: &str = "photo";

const CROP_PHOTO: &str = r#"{"version":"1.0.0","children":[
  {"type":"rectangle","id":"photo","name":"Photo","x":400,"y":100,
   "width":200,"height":100,
   "fill":[{"type":"image","url":"data:image/png;base64,AA==","mode":"crop",
     "originalSize":{"width":400,"height":100},
     "transform":{"m00":0.5,"m01":0.0,"m02":0.25,
                  "m10":0.0,"m11":1.0,"m12":0.0}}]}
]}"#;

const ROTATED_NESTED_CROP: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"parent","x":200,"y":100,"width":300,"height":300,
   "rotation":90,"children":[
    {"type":"rectangle","id":"photo","x":50,"y":50,"width":100,"height":100,
     "fill":[{"type":"image","url":"asset.png","mode":"crop",
       "originalSize":{"width":100,"height":200},
       "transform":{"m00":1.0,"m01":0.0,"m02":0.0,
                    "m10":0.0,"m11":0.5,"m12":0.25}}]}
  ]}
]}"#;

const DEEP_CROP_HIERARCHY: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"outer","x":100,"y":100,"width":500,"height":400,
   "children":[
    {"type":"frame","id":"middle","x":20,"y":20,"width":400,"height":300,
     "children":[
      {"type":"rectangle","id":"deep-photo","x":20,"y":20,"width":120,"height":80,
       "fill":[{"type":"image","url":"asset.png","mode":"crop",
         "originalSize":{"width":240,"height":80},
         "transform":{"m00":0.5,"m01":0.0,"m02":0.25,
                      "m10":0.0,"m11":1.0,"m12":0.0}}]},
      {"type":"frame","id":"crop-with-child","x":180,"y":20,"width":120,"height":80,
       "fill":[{"type":"image","url":"asset.png","mode":"crop",
         "originalSize":{"width":240,"height":80},
         "transform":{"m00":0.5,"m01":0.0,"m02":0.25,
                      "m10":0.0,"m11":1.0,"m12":0.0}}],
       "children":[
        {"type":"rectangle","id":"crop-child","x":10,"y":10,"width":40,"height":40}
       ]}
    ]}
  ]}
]}"#;

fn seed_crop(host: &mut WidgetHostNative) {
    let doc = jian_ops_schema::load_str(CROP_PHOTO)
        .expect("crop fixture parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new(PHOTO_ID));
    host.mark_paint_dirty_for_test();
}

fn seed_nested_crop(host: &mut WidgetHostNative) {
    let doc = jian_ops_schema::load_str(ROTATED_NESTED_CROP)
        .expect("nested crop fixture parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new(PHOTO_ID));
    host.mark_paint_dirty_for_test();
}

fn seed_deep_crop_hierarchy(host: &mut WidgetHostNative, selected: &str) {
    let doc = jian_ops_schema::load_str(DEEP_CROP_HIERARCHY)
        .expect("deep crop fixture parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new(selected));
    host.mark_paint_dirty_for_test();
}

fn screen_at(host: &WidgetHostNative, doc_x: f32, doc_y: f32) -> (f32, f32) {
    let (cx0, cy0, _cw, _ch) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    (cx0 + doc_x, cy0 + doc_y)
}

fn press_photo_center(host: &mut WidgetHostNative) -> (f32, f32) {
    let point = screen_at(host, 500.0, 150.0);
    assert!(host.apply_press(point.0, point.1, VIEWPORT_W, VIEWPORT_H));
    point
}

fn release(host: &mut WidgetHostNative) {
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
}

fn photo_transform(host: &WidgetHostNative) -> [f32; 6] {
    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new(PHOTO_ID),
    )
    .expect("photo exists");
    primary_image_fill_transform(node).expect("photo has an explicit crop transform")
}

fn authored_geometry(
    host: &WidgetHostNative,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let node = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new(PHOTO_ID),
    )
    .expect("photo exists");
    (
        node.base().x,
        node.base().y,
        node.width_px(),
        node.height_px(),
    )
}

fn resolved_bounds(host: &mut WidgetHostNative) -> Rect {
    host.refresh_layout_scene();
    host.layout_scene
        .active_page()
        .and_then(|page| page.find(PHOTO_ID))
        .expect("photo is in the active scene")
        .bounds
}

#[test]
fn image_crop_drag_changes_only_transform_and_is_one_undo_step() {
    let mut host = WidgetHostNative::new();
    seed_crop(&mut host);
    assert!(host.enter_selected_image_crop_edit());

    let geometry_before = authored_geometry(&host);
    let bounds_before = resolved_bounds(&mut host);
    let transform_before = photo_transform(&host);
    let history_before = host.editor_state().history.past.len();

    let (press_x, press_y) = press_photo_center(&mut host);
    assert!(host.image_crop_drag.is_some(), "crop drag owns the press");
    assert!(host.node_drag.is_none(), "node movement must stay disabled");
    assert!(host.apply_cursor_move(press_x + 20.0, press_y));
    let transform_after_move = photo_transform(&host);
    assert_ne!(
        transform_after_move, transform_before,
        "crop pan changes the image transform live"
    );
    assert_eq!(
        authored_geometry(&host),
        geometry_before,
        "crop pan must not rewrite node x/y/width/height"
    );
    assert_eq!(
        resolved_bounds(&mut host),
        bounds_before,
        "crop pan must not move or resize the resolved node"
    );
    assert_eq!(
        host.editor_state().history.past.len(),
        history_before,
        "live pointer moves do not push intermediate history entries"
    );

    release(&mut host);
    assert!(host.image_crop_drag.is_none());
    assert_eq!(
        host.editor_state().history.past.len(),
        history_before + 1,
        "release commits exactly one crop-pan history entry"
    );

    assert!(host.editor_state_mut().undo());
    assert_eq!(
        photo_transform(&host),
        transform_before,
        "undo restores the pre-drag crop window"
    );
    assert_eq!(authored_geometry(&host), geometry_before);
}

#[test]
fn crop_node_without_crop_editing_starts_an_ordinary_node_drag() {
    let mut host = WidgetHostNative::new();
    seed_crop(&mut host);

    press_photo_center(&mut host);

    assert!(
        host.node_drag.is_some(),
        "normal select gesture owns the node"
    );
    assert!(
        host.image_crop_drag.is_none(),
        "crop panning requires the dedicated edit mode"
    );
    assert_eq!(host.editor_state().editor_ui.image_crop_editing, None);
}

#[test]
fn double_click_on_selected_crop_enters_edit_without_node_drag() {
    let mut host = WidgetHostNative::new();
    seed_crop(&mut host);

    host.set_now_ms(1_000);
    press_photo_center(&mut host);
    assert!(host.node_drag.is_some(), "first click is an ordinary click");
    release(&mut host);

    host.set_now_ms(1_200);
    press_photo_center(&mut host);

    assert_eq!(
        host.editor_state().editor_ui.image_crop_editing,
        Some(NodeId::new(PHOTO_ID))
    );
    assert!(
        host.node_drag.is_none(),
        "the activating double-click must not move the node"
    );
    assert!(
        host.image_crop_drag.is_none(),
        "activation enters crop edit; a later press starts panning"
    );
}

#[test]
fn escape_exits_crop_edit_and_preserves_selection() {
    let mut host = WidgetHostNative::new();
    seed_crop(&mut host);
    assert!(host.enter_selected_image_crop_edit());
    let selection_before = host.editor_state().selection.clone();

    assert!(host.apply_escape());

    assert_eq!(host.editor_state().editor_ui.image_crop_editing, None);
    assert_eq!(
        host.editor_state().selection,
        selection_before,
        "Escape leaves the edited crop selected"
    );
}

#[test]
fn nested_crop_uses_editing_id_and_inverts_ancestor_rotation() {
    let mut host = WidgetHostNative::new();
    seed_nested_crop(&mut host);
    assert!(host.enter_selected_image_crop_edit());
    let before = photo_transform(&host);

    assert!(host.apply_canvas_node_press(
        vec![NodeId::new("parent"), NodeId::new(PHOTO_ID)],
        0.0,
        0.0,
        false,
        VIEWPORT_W,
        VIEWPORT_H,
    ));
    assert!(host.image_crop_drag.is_some());
    assert!(host.node_drag.is_none());
    assert_eq!(
        host.apply_image_crop_drag_cursor_move(20.0, 0.0),
        Some(true)
    );

    let after = photo_transform(&host);
    assert_eq!(after[2], before[2], "screen-x becomes local-y");
    assert!(
        after[5] > before[5],
        "inverse 90-degree parent rotation pans the vertical crop window"
    );
}

#[test]
fn layer_selected_deep_crop_leaf_enters_edit_after_two_canvas_presses() {
    let mut host = WidgetHostNative::new();
    seed_deep_crop_hierarchy(&mut host, "deep-photo");
    let path = vec![
        NodeId::new("outer"),
        NodeId::new("middle"),
        NodeId::new("deep-photo"),
    ];

    host.set_now_ms(1_000);
    assert!(host.apply_canvas_node_press(path.clone(), 0.0, 0.0, false, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().selection.anchor,
        NodeId::new("deep-photo"),
        "the first press preserves the exact Layer-panel crop selection"
    );
    release(&mut host);

    host.set_now_ms(1_200);
    assert!(host.apply_canvas_node_press(path, 0.0, 0.0, false, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.image_crop_editing,
        Some(NodeId::new("deep-photo"))
    );
    assert!(
        host.node_drag.is_none(),
        "the activating press must not start a node drag"
    );
}

#[test]
fn selected_crop_with_deeper_child_keeps_one_level_drill_behavior() {
    let mut host = WidgetHostNative::new();
    seed_deep_crop_hierarchy(&mut host, "crop-with-child");
    let path = vec![
        NodeId::new("outer"),
        NodeId::new("middle"),
        NodeId::new("crop-with-child"),
        NodeId::new("crop-child"),
    ];

    host.set_now_ms(1_000);
    assert!(host.apply_canvas_node_press(path.clone(), 0.0, 0.0, false, VIEWPORT_W, VIEWPORT_H));
    release(&mut host);
    host.set_now_ms(1_200);
    assert!(host.apply_canvas_node_press(path, 0.0, 0.0, false, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state().editor_ui.image_crop_editing, None);
    assert_eq!(
        host.editor_state().selection.anchor,
        NodeId::new("crop-with-child"),
        "double press drills one level instead of editing the ancestor crop"
    );
    assert_eq!(
        host.editor_state().editor_ui.entered_container,
        Some(NodeId::new("middle"))
    );
}
