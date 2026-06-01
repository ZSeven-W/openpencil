use super::WidgetHost;
use op_editor_core::walkers::find_node;
use op_editor_core::{ChatMessage, NodeId, PenNodeExt};

fn expanded_design_message() -> ChatMessage {
    let mut message = ChatMessage::assistant(
        r#"```json
[{"type":"frame","id":"apply-root","name":"Apply Root","x":0,"y":0,"width":100,"height":80,"children":[]}]
```"#,
    );
    message.design_block_expanded_overrides = vec![Some(true)];
    message
}

fn apply_button_point(chat_rect: op_editor_ui::Rect) -> (f32, f32) {
    let body_x = chat_rect.origin.x + 16.0;
    let body_top = chat_rect.origin.y + 36.0;
    let body_w = chat_rect.size.x - 32.0;
    let one_line_code_preview_h = 8.0 * 2.0 + 13.0;
    let apply_y = body_top + 32.0 + 4.0 + one_line_code_preview_h + 16.0;
    (body_x + body_w / 2.0, apply_y)
}

#[test]
fn clicking_apply_design_block_inserts_nodes_once_like_ts() {
    let mut host = WidgetHost::new();
    host.editor_state
        .chat
        .messages
        .push(expanded_design_message());
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let chat_rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    let before_count = host.editor_state.active_children().len();
    let target = NodeId::new("apply-root");
    assert!(find_node(host.editor_state.active_children(), &target).is_none());

    let (x, y) = apply_button_point(chat_rect);
    assert!(host.apply_click(x, y, viewport_w, viewport_h));

    assert!(host
        .editor_state
        .active_children()
        .iter()
        .any(|node| node.base().name.as_deref() == Some("Apply Root")));
    assert_eq!(host.editor_state.active_children().len(), before_count + 1);
    assert_eq!(
        host.editor_state.chat.messages[0]
            .content
            .matches("<!-- APPLIED -->")
            .count(),
        1
    );

    let _ = host.apply_click(x, y, viewport_w, viewport_h);

    assert_eq!(host.editor_state.active_children().len(), before_count + 1);
    assert_eq!(
        host.editor_state.chat.messages[0]
            .content
            .matches("<!-- APPLIED -->")
            .count(),
        1
    );
}
