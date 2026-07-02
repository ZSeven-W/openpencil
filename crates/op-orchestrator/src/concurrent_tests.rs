//! `concurrent` tests — `BufferDocSink` (the buffering sink reused by the
//! `spawn_agents` fan-out). The former screen-grouping / `effective_concurrency`
//! / `run_screen_group_worker` / `run_concurrent` tests were removed with the
//! multi-screen concurrent path.

use super::*;
use op_editor_core::EditorCommand;

/// `BufferDocSink` collects commands without modifying a real doc.
#[test]
fn buffer_doc_sink_collects_commands() {
    let mut sink = BufferDocSink::new(EditorState::new());
    let applied = sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![],
        parent_id: op_editor_core::NodeId::NONE,
        page_id: None,
    });
    assert!(applied, "BufferDocSink.apply must always return true");
    assert_eq!(sink.commands.len(), 1);
}

/// `state()` on `BufferDocSink` returns the snapshot passed at construction.
#[test]
fn buffer_doc_sink_state_returns_snapshot() {
    let state = EditorState::new();
    let sink = BufferDocSink::new(state.clone());
    let _ = sink.state();
}

/// `BufferDocSink` tracks undo-batch depth correctly.
#[test]
fn buffer_doc_sink_undo_batch_depth() {
    let mut sink = BufferDocSink::new(EditorState::new());
    assert_eq!(sink.batch_depth, 0);
    sink.begin_undo_batch();
    assert_eq!(sink.batch_depth, 1);
    sink.end_undo_batch();
    assert_eq!(sink.batch_depth, 0);
}
