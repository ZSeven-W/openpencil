use op_editor_core::{ChatMessage, ChatState};

#[test]
fn stop_streaming_marks_current_turn_done_and_queues_host_abort() {
    let mut chat = ChatState {
        pending_send: Some("queued prompt".into()),
        ..Default::default()
    };
    chat.messages.push(ChatMessage::user("queued prompt"));
    chat.messages.push(ChatMessage::assistant_streaming());

    assert!(chat.stop_streaming());

    assert!(chat.pending_send.is_none());
    assert!(chat.pending_stop_chat);
    assert!(
        chat.messages.iter().all(|message| !message.streaming),
        "stop must freeze the transcript so stale worker deltas cannot keep animating"
    );
}

#[test]
fn stop_streaming_is_inert_when_no_turn_is_active() {
    let mut chat = ChatState::default();

    assert!(!chat.stop_streaming());
    assert!(!chat.pending_stop_chat);
}
