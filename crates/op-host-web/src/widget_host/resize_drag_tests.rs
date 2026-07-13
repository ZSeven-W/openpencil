use super::WidgetHost;
use op_editor_core::{own_bounds, walkers::find_node, NodeId, PenNodeExt, Tool};
use op_editor_ui::Point2D;

const VW: f32 = 1200.0;
const VH: f32 = 800.0;

fn seed(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"box","name":"Box","x":100,"y":100,"width":120,"height":80,
           "fill":[{"type":"solid","color":"#2563EB"}]}
        ]}"##,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state.tool = Tool::Select;
    host.editor_state.set_single_selection(NodeId::new("box"));
    host.editor_state_dirty = true;
}

fn seed_container(host: &mut WidgetHost) {
    let doc = jian_ops_schema::load_str(
        r##"{"version":"0.8.0","children":[
          {"type":"frame","id":"box","name":"Frame","x":100,"y":100,
           "width":120,"height":80,"layout":"none","children":[
             {"type":"rectangle","id":"child","name":"Child","x":10,"y":12,
              "width":30,"height":20,"fill":[{"type":"solid","color":"#22C55E"}]}
           ]}
        ]}"##,
    )
    .expect("container fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state.tool = Tool::Select;
    host.editor_state.set_single_selection(NodeId::new("box"));
    host.editor_state_dirty = true;
}

fn screen(host: &WidgetHost, doc_x: f32, doc_y: f32) -> Point2D {
    let (cx0, cy0, _, _) = host.canvas_region(VW, VH);
    Point2D::new(cx0 + doc_x, cy0 + doc_y)
}

fn box_bounds(host: &WidgetHost) -> op_editor_core::DocRect {
    let node = find_node(host.editor_state.active_children(), &NodeId::new("box"))
        .expect("box remains in document");
    own_bounds(node)
}

#[test]
fn select_tool_dragging_bottom_right_handle_resizes_selected_shape_like_native() {
    let mut host = WidgetHost::new();
    seed(&mut host);

    let press = screen(&host, 220.0, 180.0);
    let move_to = screen(&host, 260.0, 205.0);

    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    assert!(host.apply_release_with_viewport(VW, VH));

    let bounds = box_bounds(&host);
    assert_eq!(bounds.x, 100.0);
    assert_eq!(bounds.y, 100.0);
    assert_eq!(bounds.w, 160.0);
    assert_eq!(bounds.h, 105.0);
}

#[test]
fn dragging_container_handle_resizes_only_the_container() {
    let mut host = WidgetHost::new();
    seed_container(&mut host);

    let press = screen(&host, 220.0, 180.0);
    let move_to = screen(&host, 260.0, 205.0);

    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    assert!(host.apply_release_with_viewport(VW, VH));

    let bounds = box_bounds(&host);
    assert_eq!(bounds.w, 160.0);
    assert_eq!(bounds.h, 105.0);
    let child = find_node(host.editor_state.active_children(), &NodeId::new("child"))
        .expect("child remains in document");
    assert_eq!(child.base().x, Some(10.0));
    assert_eq!(child.base().y, Some(12.0));
    assert_eq!(child.width_px(), Some(30.0));
    assert_eq!(child.height_px(), Some(20.0));
}
