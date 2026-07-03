use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::test_support::{rect, state_with};

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn document_command_marks_state_dirty_until_saved() {
    let mut state = state_with(vec![rect("n1", "Card", 0.0, 0.0, 100.0, 80.0)]);
    state.mark_saved_revision();
    assert!(!state.is_dirty());

    assert!(state.apply(EditorCommand::SetNodeName {
        node_id: id("n1"),
        name: "Renamed Card".into(),
    }));

    assert!(state.is_dirty());
    assert!(state.document_revision() > state.saved_revision());

    state.mark_saved_revision();
    assert!(!state.is_dirty());
}

#[test]
fn selection_and_viewport_commands_do_not_mark_dirty() {
    let mut state = state_with(vec![rect("n1", "Card", 0.0, 0.0, 100.0, 80.0)]);
    state.mark_saved_revision();

    assert!(state.apply(EditorCommand::SetSelection { node_id: id("n1") }));
    assert!(state.apply(EditorCommand::SetViewport {
        pan_x: Some(12),
        pan_y: Some(34),
        zoom_percent: Some(125),
    }));

    assert!(!state.is_dirty());
    assert_eq!(state.document_revision(), state.saved_revision());
}

#[test]
fn selection_only_batch_does_not_mark_dirty() {
    let mut state = state_with(vec![rect("n1", "Card", 0.0, 0.0, 100.0, 80.0)]);
    state.mark_saved_revision();

    assert!(state.apply(EditorCommand::Batch {
        commands: vec![
            EditorCommand::SetSelection { node_id: id("n1") },
            EditorCommand::ClearSelection,
        ],
    }));

    assert!(!state.is_dirty());
    assert_eq!(state.document_revision(), state.saved_revision());
}

#[test]
fn undo_back_to_saved_revision_becomes_clean() {
    let mut state = state_with(vec![rect("n1", "Card", 0.0, 0.0, 100.0, 80.0)]);
    state.mark_saved_revision();
    assert!(state.apply(EditorCommand::SetSelection { node_id: id("n1") }));

    assert!(state.apply(EditorCommand::DeleteSelected));
    assert!(state.is_dirty());

    assert!(state.apply(EditorCommand::Undo));
    assert!(!state.is_dirty());
    assert_eq!(state.document_revision(), state.saved_revision());
}

#[test]
fn edit_after_undo_past_save_point_stays_dirty() {
    // save -> undo -> DIFFERENT edit: a naive `revision + 1` allocator
    // reuses the saved revision value here and reports a clean file over
    // divergent content. The monotonic counter must keep this dirty.
    let mut state = state_with(vec![
        rect("n1", "Card", 0.0, 0.0, 100.0, 80.0),
        rect("n2", "Chip", 0.0, 0.0, 40.0, 20.0),
    ]);
    assert!(state.apply(EditorCommand::SetSelection { node_id: id("n1") }));
    assert!(state.apply(EditorCommand::DeleteSelected));
    state.mark_saved_revision();

    assert!(state.apply(EditorCommand::Undo));
    assert!(state.is_dirty());

    assert!(state.apply(EditorCommand::SetSelection { node_id: id("n2") }));
    assert!(state.apply(EditorCommand::DeleteSelected));
    assert!(state.is_dirty());
    assert_ne!(state.document_revision(), state.saved_revision());
}
