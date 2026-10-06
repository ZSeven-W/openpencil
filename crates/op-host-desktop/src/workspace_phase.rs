//! The pure decisions behind the generation workspace's phase pump.
//!
//! `app_handler/redraw.rs` observes the "everything is done" edge every
//! frame; this module keeps the epoch-fenced finish decision and the
//! last-assistant-error predicate testable without a window.

use op_editor_core::chat::ChatMessage;
use op_editor_core::{ChatRole, EditorState};

/// Whether the last assistant message reports a failed turn: a failed
/// orchestrator subtask, a completion with failures, or an errored
/// activity row. The workspace's Failed banner keys off this.
pub(crate) fn last_assistant_failed(state: &EditorState) -> bool {
    let Some(message) = last_assistant_message(state) else {
        return false;
    };
    if !message.failed_subtasks.is_empty() {
        return true;
    }
    if message
        .completion
        .is_some_and(|completion| completion.failed > 0)
    {
        return true;
    }
    message
        .activities
        .iter()
        .any(|activity| activity.status == op_editor_core::ChatActivityStatus::Error)
}

/// The active tab's last assistant message, if the transcript has one.
fn last_assistant_message(state: &EditorState) -> Option<&ChatMessage> {
    state
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
}

/// How many REAL boards the run produced.
///
/// `active_page_boards` counts the blank starter frame like any other
/// board, so a run that drew nothing at all still reported one board and
/// settled as Done (measured 2026-09-13: a 演示文稿 brief that never
/// reached the design pipeline showed 已完成 over an empty Frame). A page
/// that is still the untouched starter has produced nothing.
pub(crate) fn produced_board_count(state: &EditorState) -> usize {
    if op_editor_core::blank_starter::active_page_is_blank_starter(state) {
        return 0;
    }
    op_editor_core::preview_slideshow::active_page_boards(state).len()
}

/// Whether a queued send has yet to reach the launcher.
///
/// Between `begin_send` and the next `launch_if_pending` drain there is
/// no chat or design session, so every "is everything idle?" test says
/// yes even though the run has not started. The workspace must keep
/// showing 生成中 across that gap instead of settling a verdict on a run
/// that has not begun.
pub(crate) fn awaiting_launch(state: &EditorState) -> bool {
    state.chat.pending_send.is_some()
}

/// Whether any assistant message is still streaming in the active tab
/// — the "generating" half of the phase that `agents_running` alone
/// misses (single-agent turns never touch the running counters).
pub(crate) fn assistant_streaming(state: &EditorState) -> bool {
    state
        .chat
        .messages
        .iter()
        .any(|message| message.role == ChatRole::Assistant && message.streaming)
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::chat::message::ChatMessage;

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
        assert!(op_editor_core::blank_starter::active_page_is_blank_starter(
            &starter
        ));
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
        assert!(!op_editor_core::blank_starter::active_page_is_blank_starter(&drawn));
        assert_eq!(produced_board_count(&drawn), 2);
    }

    #[test]
    fn a_clean_completion_is_not_a_failure() {
        let mut message = assistant();
        message.completion = Some(op_editor_core::ChatCompletion {
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
        subtasks
            .failed_subtasks
            .push(op_editor_core::PendingSubtaskRetry {
                subtask_id: "s1".into(),
                subtask_json: "{}".into(),
                insert_after_sibling_id: None,
            });
        assert!(last_assistant_failed(&editor_with_last(subtasks)));

        let mut completion = assistant();
        completion.completion = Some(op_editor_core::ChatCompletion {
            succeeded: 1,
            failed: 2,
            nodes: 5,
        });
        assert!(last_assistant_failed(&editor_with_last(completion)));

        let mut activity = assistant();
        activity.activities.push(op_editor_core::ChatActivity {
            id: "a1".into(),
            title: "生成".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Error,
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
}
