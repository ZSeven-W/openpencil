use super::*;

#[test]
fn an_ambiguous_ordinary_edit_finishes_without_launching_a_design() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.active_children_mut().clear();
    for id in ["a", "b"] {
        state.active_children_mut().push(
            serde_json::from_value(serde_json::json!({
                "type": "frame", "id": id, "name": "Home", "width": 390, "height": 844,
                "children": [{"type": "text", "id": format!("{id}-title"), "content": "Title"}]
            }))
            .unwrap(),
        );
    }
    state
        .editor_ui
        .workspace
        .open_for_reading(op_editor_core::HomeFamily::AppUi, 1);
    state
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant(""));
    let before = state.doc.clone();
    let mut chat = None;
    let mut design = None;
    assert!(launch_workspace_edit(
        &mut host,
        "把首页标题改成咖啡",
        &mut chat,
        &mut design
    ));
    assert!(chat.is_none() && design.is_none());
    assert_eq!(host.editor_state().doc, before);
    assert!(!host.editor_state().chat.messages.last().unwrap().streaming);
    assert!(!host
        .editor_state()
        .chat
        .messages
        .last()
        .unwrap()
        .content
        .is_empty());
}
