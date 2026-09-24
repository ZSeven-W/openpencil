//! Tests for the host-agnostic workspace run verdicts.

use super::*;

fn editor_with_last(message: ChatMessage) -> EditorState {
    let mut state = EditorState::default();
    state.chat.messages.push(ChatMessage::user("做个设计"));
    state.chat.messages.push(message);
    state
}

fn assistant() -> ChatMessage {
    ChatMessage::assistant("")
}

fn editor_from(children: &str) -> EditorState {
    let source = format!(r#"{{ "version": "1.0.0", "children": [{children}] }}"#);
    let document = jian_ops_schema::load_str(&source)
        .expect("parse fixture")
        .value;
    EditorState::from_document(document)
}

#[test]
fn a_queued_send_is_not_an_idle_run() {
    let mut state = EditorState::default();
    assert!(!awaiting_launch(&state));
    state.chat.pending_send = Some("做一份产品介绍 PPT".into());
    assert!(awaiting_launch(&state));
}

#[test]
fn a_blank_starter_page_has_produced_no_boards() {
    // The starter frame is furniture, not a deliverable: a run that
    // drew nothing must settle Failed, never Done over an empty page.
    let starter = EditorState::starter();
    assert!(crate::blank_starter::active_page_is_blank_starter(&starter));
    assert_eq!(produced_board_count(&starter), 0);
}

#[test]
fn real_boards_are_counted() {
    let drawn = editor_from(
        r#"{ "type": "frame", "id": "b0", "x": 0, "y": 0, "width": 375, "height": 812,
             "children": [] },
           { "type": "frame", "id": "b1", "x": 420, "y": 0, "width": 375, "height": 812,
             "children": [] }"#,
    );
    assert!(!crate::blank_starter::active_page_is_blank_starter(&drawn));
    assert_eq!(produced_board_count(&drawn), 2);
}

#[test]
fn a_clean_completion_is_not_a_failure() {
    let mut message = assistant();
    message.completion = Some(crate::ChatCompletion {
        succeeded: 3,
        failed: 0,
        nodes: 42,
    });
    assert!(!last_assistant_failed(&editor_with_last(message)));
    assert!(!last_assistant_failed(&editor_with_last(assistant())));
}

#[test]
fn failed_subtasks_completions_and_error_rows_all_fail() {
    let mut subtasks = assistant();
    subtasks.failed_subtasks.push(crate::PendingSubtaskRetry {
        subtask_id: "s1".into(),
        subtask_json: "{}".into(),
        insert_after_sibling_id: None,
    });
    assert!(last_assistant_failed(&editor_with_last(subtasks)));

    let mut completion = assistant();
    completion.completion = Some(crate::ChatCompletion {
        succeeded: 1,
        failed: 2,
        nodes: 5,
    });
    assert!(last_assistant_failed(&editor_with_last(completion)));

    let mut activity = assistant();
    activity.activities.push(crate::ChatActivity {
        id: "a1".into(),
        title: "生成".into(),
        detail: None,
        status: crate::ChatActivityStatus::Error,
        content_offset: None,
    });
    assert!(last_assistant_failed(&editor_with_last(activity)));
}

#[test]
fn a_user_only_transcript_is_not_a_failure() {
    let mut state = EditorState::default();
    state.chat.messages.push(ChatMessage::user("做个设计"));
    assert!(!last_assistant_failed(&state));
    assert!(!assistant_streaming(&state));
}

#[test]
fn streaming_assistants_report_generating() {
    let mut message = assistant();
    message.streaming = true;
    let mut state = EditorState::default();
    state.chat.messages.push(message);
    assert!(assistant_streaming(&state));
}

#[test]
fn an_error_transcript_is_a_failure() {
    // Every provider transport ends a dead turn as `error: ...` text; a
    // refine over an existing draft has boards, so this is the only
    // signal its failure leaves behind.
    let errored = ChatMessage::assistant("error: 429 rate limited");
    assert!(last_assistant_failed(&editor_with_last(errored)));
    let prose = ChatMessage::assistant("No error: the layout is fine.");
    assert!(!last_assistant_failed(&editor_with_last(prose)));
}
