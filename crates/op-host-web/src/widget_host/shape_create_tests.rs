use super::WidgetHost;
use jian_ops_schema::node::{PenNode, TextContent};
use op_editor_core::{own_bounds, walkers::find_node, Tool};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

#[test]
fn frame_tool_drag_creates_and_keeps_a_frame() {
    let mut host = WidgetHost::new();
    host.editor_state.active_children_mut().clear();
    host.editor_state.clear_selection();
    host.editor_state.tool = Tool::Frame;
    host.editor_state_dirty = true;

    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let start = Point2D::new(cx0 + 100.0, cy0 + 100.0);
    let end = Point2D::new(cx0 + 260.0, cy0 + 190.0);

    assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.active_children().len(), 0);

    assert!(host.apply_cursor_move(end.x, end.y));
    assert_eq!(host.editor_state.active_children().len(), 1);
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));

    assert_eq!(host.editor_state.tool, Tool::Select);
    assert_eq!(host.editor_state.active_children().len(), 1);

    let selected = host.editor_state.selection.anchor.clone();
    let node = find_node(host.editor_state.active_children(), &selected)
        .expect("created frame remains selected");
    assert!(matches!(node, PenNode::Frame(_)));

    let bounds = own_bounds(node);
    assert_eq!(bounds.x, 100.0);
    assert_eq!(bounds.y, 100.0);
    assert_eq!(bounds.w, 160.0);
    assert_eq!(bounds.h, 90.0);
}

#[test]
fn frame_tool_press_without_drag_does_not_flash_a_temporary_frame() {
    let mut host = WidgetHost::new();
    host.editor_state.active_children_mut().clear();
    host.editor_state.clear_selection();
    host.editor_state.tool = Tool::Frame;
    host.editor_state_dirty = true;

    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let start = Point2D::new(cx0 + 100.0, cy0 + 100.0);

    assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.active_children().len(), 0);

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.tool, Tool::Select);
    assert_eq!(host.editor_state.active_children().len(), 0);
}

#[test]
fn text_tool_still_creates_on_press() {
    let mut host = WidgetHost::new();
    host.editor_state.active_children_mut().clear();
    host.editor_state.clear_selection();
    host.editor_state.tool = Tool::Text;
    host.editor_state_dirty = true;

    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let start = Point2D::new(cx0 + 100.0, cy0 + 100.0);

    assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state.active_children().len(), 1);
    let selected = host.editor_state.selection.anchor.clone();
    let node = find_node(host.editor_state.active_children(), &selected)
        .expect("created text remains selected");
    assert!(matches!(node, PenNode::Text(_)));
}

#[test]
fn text_tool_press_enters_editing_so_typing_updates_created_text() {
    let mut host = WidgetHost::new();
    host.editor_state.active_children_mut().clear();
    host.editor_state.clear_selection();
    host.editor_state.tool = Tool::Text;
    host.editor_state_dirty = true;

    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let start = Point2D::new(cx0 + 100.0, cy0 + 100.0);

    assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert!(
        host.editor_state.ui.text_editing.is_some(),
        "newly created text should be ready for typing"
    );
    assert!(host.apply_text('A'));

    let selected = host.editor_state.selection.anchor.clone();
    let node = find_node(host.editor_state.active_children(), &selected)
        .expect("created text remains selected");
    let PenNode::Text(text) = node else {
        panic!("created node should be text");
    };
    assert_eq!(text.content, TextContent::Plain("A".to_string()));
}

#[test]
fn text_tool_first_typing_burst_reuses_create_history_snapshot() {
    let mut host = WidgetHost::new();
    host.editor_state.active_children_mut().clear();
    host.editor_state.clear_selection();
    host.editor_state.tool = Tool::Text;
    host.editor_state_dirty = true;

    let (cx0, cy0, _, _) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let start = Point2D::new(cx0 + 100.0, cy0 + 100.0);

    assert!(host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    let history_after_create = host.editor_state.history.past.len();

    assert!(host.apply_text('A'));
    assert_eq!(
        host.editor_state.history.past.len(),
        history_after_create,
        "the first text burst after creating a text node should not deep-clone the document again"
    );
    assert!(host.editor_state.undo());
    assert_eq!(host.editor_state.active_children().len(), 0);
}
