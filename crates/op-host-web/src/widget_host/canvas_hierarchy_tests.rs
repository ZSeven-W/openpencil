//! Web host regression coverage for Pencil-style canvas hierarchy depth.

use super::WidgetHost;
use op_editor_core::{NodeId, Tool};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

const FOUR_LEVELS: &str = r#"{"version":"1.0.0","children":[
  {"type":"frame","id":"root","name":"Root Frame","x":500,"y":100,"width":300,"height":300,
   "children":[
     {"type":"frame","id":"level-1","name":"Level 1","x":20,"y":20,"width":240,"height":240,
      "children":[
        {"type":"frame","id":"level-2","name":"Level 2","x":20,"y":20,"width":180,"height":180,
         "children":[
           {"type":"rectangle","id":"level-3","name":"Level 3","x":20,"y":20,"width":100,"height":100}
         ]}
      ]}
   ]}
]}"#;

fn seed() -> WidgetHost {
    let doc = jian_ops_schema::load_str(FOUR_LEVELS)
        .expect("four-level fixture parses")
        .value;
    let mut host = WidgetHost::new();
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state.tool = Tool::Select;
    host.editor_state_dirty = true;
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host
}

fn screen_at(host: &WidgetHost, doc_x: f32, doc_y: f32) -> Point2D {
    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    Point2D::new(cx0 + doc_x, cy0 + doc_y)
}

#[test]
fn hover_and_double_press_drill_exactly_one_level_then_consume_stamp() {
    let mut host = seed();
    // Absolute bounds nest at root 500, level-1 520, level-2 540,
    // level-3 560. This point is inside all four rendered nodes.
    let point = screen_at(&host, 580.0, 180.0);

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state.editor_ui.canvas_hover_node,
        Some(NodeId::new("level-1")),
        "idle hover resolves to the root scope's direct child"
    );

    host.set_now_ms(1_000);
    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.selection.anchor, NodeId::new("level-1"));
    assert_eq!(host.editor_state.editor_ui.entered_container, None);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    host.set_now_ms(1_200);
    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.selection.anchor,
        NodeId::new("level-2"),
        "the second press drills only to the direct child under the pointer"
    );
    assert_eq!(
        host.editor_state.editor_ui.entered_container,
        Some(NodeId::new("level-1"))
    );
    assert_eq!(
        host.editor_state.editor_ui.canvas_hover_node,
        Some(NodeId::new("level-2")),
        "stationary-pointer hover rebases to the newly entered depth"
    );
    assert_eq!(
        host.editor_state.editor_ui.last_canvas_click, None,
        "a completed double press consumes its stamp"
    );
    let _ = host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);

    host.set_now_ms(1_300);
    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state.selection.anchor,
        NodeId::new("level-2"),
        "a third press starts a fresh click instead of drilling to level-3"
    );
    assert_eq!(
        host.editor_state.editor_ui.entered_container,
        Some(NodeId::new("level-1"))
    );
}

#[test]
fn frame_label_hover_targets_the_root() {
    let mut host = seed();
    // Frame labels span y = root_top - 32 .. root_top - 4 and begin
    // four pixels left of the frame. Pick a point over "Root Frame".
    let label_point = screen_at(&host, 524.0, 82.0);

    assert!(host.apply_cursor_move(label_point.x, label_point.y));
    assert_eq!(
        host.editor_state.editor_ui.canvas_hover_node,
        Some(NodeId::new("root"))
    );
}
