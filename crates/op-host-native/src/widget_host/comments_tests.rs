//! End-to-end comment flow through the native host's real input ladders.

use super::WidgetHostNative;
use op_editor_core::comment_ui::CommentPopover;
use op_editor_core::comments::CommentStatus;
use op_editor_core::EditorState;
use op_editor_ui::widgets::comment_pins::doc_to_screen;
use op_editor_ui::Point2D;

const VW: f32 = 1440.0;
const VH: f32 = 900.0;
const DOC: &str = r#"{"version":"1.0.0","children":[
  {"type":"rectangle","id":"n1","name":"One","x":10,"y":20,"width":100,"height":50}
]}"#;

fn host() -> WidgetHostNative {
    let document = jian_ops_schema::load_str(DOC).expect("fixture").value;
    let mut host = WidgetHostNative::new();
    *host.editor_state_mut() = EditorState::from_document(document);
    host.editor_state_mut().editor_ui.home.visible = false;
    host
}

#[test]
fn comment_mode_press_type_enter_posts_a_thread_on_the_node() {
    let mut host = host();
    assert!(host.apply_toggle_comment_mode());
    let _ = host.layout_scene();
    let at = doc_to_screen(host.editor_state_mut(), Point2D::new(60.0, 45.0));
    assert!(host.apply_press(at.x, at.y, VW, VH));
    assert!(
        host.input_active_pub(),
        "the popover input owns the keyboard"
    );
    for c in "Rounder".chars() {
        assert!(host.apply_text(c));
    }
    // Backspace edits the draft, never the selected node.
    assert!(host.apply_backspace());
    assert!(host.apply_send());

    let state = host.editor_state_mut();
    let thread = state.comments()[0].clone();
    assert_eq!(thread.anchor.node_id.as_deref(), Some("n1"));
    assert_eq!(thread.messages[0].text, "Rounde");
    assert_eq!(thread.status, CommentStatus::Pending);
    assert_eq!(
        state.editor_ui.comments.open,
        Some(CommentPopover::Thread { id: thread.id })
    );
    assert_eq!(state.doc.children.len(), 1);

    assert!(host.apply_escape(), "closes the popover");
    assert!(host.editor_state_mut().editor_ui.comments.mode);
    assert!(host.apply_escape(), "leaves comment mode");
    assert!(!host.editor_state_mut().editor_ui.comments.mode);
}

#[test]
fn picking_a_tool_leaves_comment_mode() {
    let mut host = host();
    host.apply_toggle_comment_mode();
    host.apply_set_tool(op_editor_core::Tool::Rect);
    assert!(!host.editor_state_mut().editor_ui.comments.mode);
}
