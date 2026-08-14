use super::{NodeDragState, WidgetHostNative};
use op_editor_core::NodeId;
use op_editor_ui::Rect;

#[test]
fn starting_node_drag_cancels_prior_layout_transition() {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"moving","name":"Moving","x":100,"y":80,
           "width":120,"height":60}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("moving"));
    host.mark_paint_dirty_for_test();
    let _ = host.layout_scene();

    host.set_now_ms(1_000);
    host.start_layout_transition_from_bounds(
        &NodeId::new("moving"),
        Rect::xywh(20.0, 80.0, 120.0, 60.0),
    );
    assert!(host.layout_transition.is_some());

    host.node_drag = Some(NodeDragState {
        last_screen_x: 500.0,
        last_screen_y: 500.0,
        press_screen_x: 500.0,
        press_screen_y: 500.0,
        moved: false,
        total_dx: 0.0,
        total_dy: 0.0,
        overlay_bounds: None,
    });
    assert!(host.apply_cursor_move(520.0, 500.0));

    assert!(host.layout_transition.is_none());
    let scene_x = host
        .layout_scene()
        .active_page()
        .and_then(|page| page.find("moving"))
        .expect("moving scene node")
        .bounds
        .origin
        .x;
    assert_eq!(scene_x, 120.0);
}
