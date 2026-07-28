use crate::test_support::{rect, state_with};
use crate::{EditOrigin, EditorState, LocalEditError, LocalEditOutcome, NodeId, PropertyFocus};

#[test]
fn local_edit_noop_removes_legacy_history_and_revision_noise() {
    let mut state = state_with(vec![rect("n1", "A", 0.0, 0.0, 10.0, 10.0)]);
    let capture = state.begin_local_edit();

    state.commit_history();
    state.set_single_selection(NodeId::new("n1"));
    assert_eq!(state.document_revision(), 1);

    assert!(matches!(
        state.end_local_edit(capture).unwrap(),
        LocalEditOutcome::NoChange
    ));
    assert!(!state.history.can_undo());
    assert_eq!(state.document_revision(), 0);
    assert_eq!(state.revision_counter, 1);
    assert_eq!(state.selection.anchor, NodeId::new("n1"));
}

#[test]
fn local_edit_reports_exact_before_and_after_documents() {
    let mut state = EditorState::new();
    state.doc.name = Some("before".into());
    let capture = state.begin_local_edit();
    state.doc.name = Some("after".into());

    let LocalEditOutcome::Changed(completed) = state.end_local_edit(capture).unwrap() else {
        panic!("document mutation must produce a completed edit");
    };
    assert_eq!(completed.before().name.as_deref(), Some("before"));
    assert_eq!(completed.after().name.as_deref(), Some("after"));
    assert_eq!(state.document_revision(), 1);
    completed.accept();
}

#[test]
fn rollback_restores_document_selection_refs_and_history_but_not_revision_counter() {
    let mut state = state_with(vec![rect("n1", "A", 0.0, 0.0, 10.0, 10.0)]);
    state.doc.name = Some("before".into());
    state.set_single_selection(NodeId::new("n1"));
    state
        .ui
        .variables
        .fill_refs
        .insert(NodeId::new("n1"), "accent".into());
    state.commit_history();
    let history_len = state.history.past.len();
    let before_revision = state.document_revision();
    let capture = state.begin_local_edit();

    state.commit_history();
    state.doc.name = Some("unsupported".into());
    state.clear_selection();
    state.ui.property_focus = Some(PropertyFocus::PositionX);
    state.ui.property_input.set_text("stale");
    state.ui.pen_in_progress = Some(NodeId::new("n1"));
    state.editor_ui.hovered_layer_id = Some(NodeId::new("n1"));
    let LocalEditOutcome::Changed(completed) = state.end_local_edit(capture).unwrap() else {
        panic!("document mutation must produce a completed edit");
    };
    let allocated_revision = state.revision_counter;

    state.rollback_local_edit(completed).unwrap();

    assert_eq!(state.doc.name.as_deref(), Some("before"));
    assert_eq!(state.selection.anchor, NodeId::new("n1"));
    assert_eq!(
        state
            .ui
            .variables
            .fill_refs
            .get(&NodeId::new("n1"))
            .map(String::as_str),
        Some("accent")
    );
    assert_eq!(state.history.past.len(), history_len);
    assert_eq!(state.document_revision(), before_revision);
    assert_eq!(state.revision_counter, allocated_revision);
    assert!(state.ui.property_focus.is_none());
    assert!(state.ui.property_input.text().is_empty());
    assert!(state.ui.pen_in_progress.is_none());
    assert!(state.editor_ui.hovered_layer_id.is_none());

    state.mark_document_changed();
    assert!(state.document_revision() > allocated_revision);
}

#[test]
fn ending_an_edit_after_document_replacement_is_rejected() {
    let mut state = EditorState::new();
    let capture = state.begin_local_edit();
    state.replace_document(EditorState::new().doc);

    assert_eq!(
        state.end_local_edit(capture).unwrap_err(),
        LocalEditError::DocumentGenerationChanged {
            expected: 0,
            actual: 1,
        }
    );
}

#[test]
fn rollback_refuses_to_overwrite_a_later_document_change() {
    let mut state = EditorState::new();
    let capture = state.begin_local_edit();
    state.doc.name = Some("edit".into());
    let LocalEditOutcome::Changed(completed) = state.end_local_edit(capture).unwrap() else {
        panic!("document mutation must produce a completed edit");
    };
    state.doc.name = Some("later".into());

    assert_eq!(
        state.rollback_local_edit(completed),
        Err(LocalEditError::DocumentChangedAfterEdit)
    );
    assert_eq!(state.doc.name.as_deref(), Some("later"));
}

#[test]
fn remote_install_does_not_participate_in_local_capture() {
    let mut state = EditorState::new();
    let mut remote = state.doc.clone();
    remote.name = Some("remote".into());

    state
        .install_verified_document(remote, EditOrigin::RemoteCommit)
        .unwrap();

    assert_eq!(state.doc.name.as_deref(), Some("remote"));
    assert!(!state.history.can_undo());
}
