//! Web Select-tool path-anchor context menu parity with native.

use super::WidgetHost;
use jian_ops_schema::node::{PenNode, PenPathPointType};
use op_editor_core::{NodeId, Tool};

const VW: f32 = 1440.0;
const VH: f32 = 900.0;

fn seed(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn path_host() -> WidgetHost {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"path","id":"n60","name":"p","x":300,"y":300,
           "anchors":[{"x":300,"y":300},{"x":380,"y":300},{"x":380,"y":360}]}
        ]}"#,
    );
    host
}

fn screen(host: &WidgetHost, doc_x: f32, doc_y: f32) -> (f32, f32) {
    let (cx0, cy0, _, _) = host.canvas_region(VW, VH);
    (cx0 + doc_x, cy0 + doc_y)
}

fn find_path<'a>(host: &'a WidgetHost, id: &str) -> &'a jian_ops_schema::node::PathNode {
    match op_editor_core::walkers::find_node(host.editor_state.active_children(), &NodeId::new(id))
    {
        Some(PenNode::Path(p)) => p,
        _ => panic!("path node {id} missing"),
    }
}

#[test]
fn select_tool_press_on_anchor_starts_the_drag_like_native() {
    let mut host = path_host();
    host.editor_state.tool = Tool::Select;
    host.editor_state.set_single_selection(NodeId::new("n60"));
    let (px, py) = screen(&host, 380.0, 300.0);

    assert!(host.apply_press(px, py, VW, VH));

    let drag = host.path_anchor_drag.as_ref().expect("anchor drag armed");
    assert_eq!(drag.anchor_index, 1);
    assert!(matches!(drag.target, super::AnchorDragTarget::Anchor));
}

#[test]
fn handle_drag_preserves_grab_offset_and_anchor_type_like_native() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"path","id":"n60","name":"p","x":300,"y":300,
           "anchors":[{"x":300,"y":300},
                      {"x":380,"y":300,"handleOut":{"x":20,"y":0}},
                      {"x":380,"y":360}]}
        ]}"#,
    );
    host.editor_state.tool = Tool::Select;
    host.editor_state.set_single_selection(NodeId::new("n60"));
    let (px, py) = screen(&host, 398.0, 298.0);

    assert!(host.apply_press(px, py, VW, VH));
    let drag = host.path_anchor_drag.as_ref().expect("handle drag armed");
    assert!(matches!(
        drag.target,
        super::AnchorDragTarget::Handle(op_editor_core::pen::PathHandleSide::Out)
    ));
    let grab = drag.grab_offset.expect("existing handle records grab");
    assert!((grab.x - 20.0).abs() < 0.5 && grab.y.abs() < 0.5);

    assert!(host.apply_cursor_move(px + 10.0, py));
    let a = &find_path(&host, "n60").anchors.as_ref().unwrap()[1];
    let ho = a.handle_out.clone().expect("handle kept");
    assert!(
        (ho.x - 30.0).abs() < 0.5 && ho.y.abs() < 0.5,
        "offset = grab + delta (no snap-to-cursor); got ({}, {})",
        ho.x,
        ho.y
    );
    assert_eq!(a.point_type, None, "untyped anchor stays untyped");
    assert!(a.handle_in.is_none(), "no opposite handle minted");

    let history_before = host.editor_state.history.past.len();
    assert!(host.apply_release_with_viewport(VW, VH));
    assert_eq!(host.editor_state.history.past.len(), history_before + 1);
    assert!(host.editor_state.undo());
    let a = &find_path(&host, "n60").anchors.as_ref().unwrap()[1];
    let ho = a.handle_out.clone().expect("handle restored");
    assert!((ho.x - 20.0).abs() < 0.5);
}

#[test]
fn select_tool_ignores_ghost_handles_like_native() {
    let mut host = path_host();
    host.editor_state.tool = Tool::Select;
    host.editor_state.set_single_selection(NodeId::new("n60"));
    let (px, py) = screen(&host, 406.0, 300.0);

    host.apply_press(px, py, VW, VH);

    assert!(
        host.path_anchor_drag.is_none(),
        "ghost handle must not arm a drag under Select"
    );
}

#[test]
fn right_click_anchor_opens_menu_and_action_commits_history_like_native() {
    let mut host = path_host();
    host.editor_state.tool = Tool::Select;
    host.editor_state.set_single_selection(NodeId::new("n60"));
    let (px, py) = screen(&host, 380.0, 300.0);

    assert!(host.apply_right_press(px, py, VW, VH));
    let menu = host
        .editor_state
        .ui
        .path_anchor_menu
        .clone()
        .expect("menu open");
    assert_eq!(menu.anchor_index, 1);

    let history_before = host.editor_state.history.past.len();
    let row_y = py + 4.0 + 26.0 * 1.5;
    assert!(host.apply_press(px + 10.0, row_y, VW, VH));

    assert!(
        host.editor_state.ui.path_anchor_menu.is_none(),
        "menu closed"
    );
    let p = find_path(&host, "n60");
    let a = &p.anchors.as_ref().unwrap()[1];
    assert_eq!(a.point_type, Some(PenPathPointType::Mirrored));
    assert!(a.handle_in.is_some() && a.handle_out.is_some());
    assert_eq!(
        host.editor_state.history.past.len(),
        history_before + 1,
        "menu action commits exactly one history entry"
    );
    assert!(host.editor_state.undo());
    let p = find_path(&host, "n60");
    assert!(p.anchors.as_ref().unwrap()[1].handle_in.is_none());
}

#[test]
fn right_click_miss_closes_menu_without_consuming_like_native() {
    let mut host = path_host();
    host.editor_state.tool = Tool::Select;
    host.editor_state.set_single_selection(NodeId::new("n60"));
    let (px, py) = screen(&host, 380.0, 300.0);
    assert!(host.apply_right_press(px, py, VW, VH));
    assert!(host.editor_state.ui.path_anchor_menu.is_some());

    let (qx, qy) = screen(&host, 600.0, 500.0);
    host.apply_press(qx, qy, VW, VH);

    assert!(host.editor_state.ui.path_anchor_menu.is_none());
    assert!(
        host.marquee_drag.is_some(),
        "the dismissing press still starts the canvas marquee"
    );
}
