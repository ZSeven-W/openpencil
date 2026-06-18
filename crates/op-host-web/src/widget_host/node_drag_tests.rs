use super::WidgetHost;
use jian_ops_schema::node::PenNode;
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
    host.editor_state_dirty = true;
}

fn screen(host: &WidgetHost, doc_x: f32, doc_y: f32) -> Point2D {
    let (cx0, cy0, _, _) = host.canvas_region(VW, VH);
    Point2D::new(cx0 + doc_x, cy0 + doc_y)
}

fn child_order(host: &WidgetHost, parent: &str) -> Vec<String> {
    find_node(host.editor_state.active_children(), &NodeId::new(parent))
        .and_then(|n| n.children())
        .map(|cs| cs.iter().map(|c| c.id_str().to_string()).collect())
        .unwrap_or_default()
}

fn box_bounds(host: &WidgetHost) -> op_editor_core::DocRect {
    let node = find_node(host.editor_state.active_children(), &NodeId::new("box"))
        .expect("box remains in document");
    match node {
        PenNode::Rectangle(_) => own_bounds(node),
        _ => panic!("fixture box is not a rectangle"),
    }
}

#[test]
fn select_tool_dragging_selected_node_moves_it_without_resizing() {
    let mut host = WidgetHost::new();
    seed(&mut host);

    let press = screen(&host, 160.0, 140.0);
    let move_to = screen(&host, 200.0, 165.0);

    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert!(host.apply_cursor_move(move_to.x, move_to.y));
    assert!(host.apply_release_with_viewport(VW, VH));

    let bounds = box_bounds(&host);
    assert_eq!(bounds.x, 140.0);
    assert_eq!(bounds.y, 125.0);
    assert_eq!(bounds.w, 120.0);
    assert_eq!(bounds.h, 80.0);
}

#[test]
fn select_tool_dragging_flex_child_reorders_on_release_like_native() {
    let mut host = WidgetHost::new();
    let doc = jian_ops_schema::load_str(
        r#"{"version":"0.8.0","children":[{
          "type":"frame","id":"stack","name":"Stack","x":400,"y":60,"width":200,"height":300,
          "layout":"vertical","gap":8,
          "children":[
            {"type":"rectangle","id":"a","name":"A","width":80,"height":40},
            {"type":"rectangle","id":"b","name":"B","width":80,"height":40},
            {"type":"rectangle","id":"c","name":"C","width":80,"height":40}
          ]}
        ]}"#,
    )
    .expect("fixture JSON parses")
    .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state.tool = Tool::Select;
    host.editor_state.editor_ui.entered_container = Some(NodeId::new("stack"));
    host.editor_state_dirty = true;

    let press = screen(&host, 440.0, 80.0);
    assert!(host.apply_press(press.x, press.y, VW, VH));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("a"));

    let move_to = screen(&host, 440.0, 160.0);
    assert!(host.apply_cursor_move(move_to.x, move_to.y));

    let a = find_node(host.editor_state.active_children(), &NodeId::new("a")).expect("a present");
    assert_eq!(
        a.base().x,
        None,
        "flex child must not materialize x during drag"
    );

    assert!(host.apply_release_with_viewport(VW, VH));

    assert_eq!(child_order(&host, "stack"), vec!["b", "a", "c"]);
}
