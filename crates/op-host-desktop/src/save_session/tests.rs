use super::*;

fn temp_op_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "openpencil-save-session-{tag}-{}-{}.op",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ))
}

fn bind_active_guest(app: &mut crate::DesktopApp) {
    assert!(app
        .host
        .editor_state_mut()
        .editor_ui
        .collab
        .set_authenticated_session(
            op_editor_core::CollabConnectionPhase::Active,
            op_editor_core::AuthenticatedCollabSession {
                session_name: "Shared design".into(),
                role: op_editor_core::CollabUiRole::Editor,
                share_endpoint: None,
            },
            Vec::new(),
        ));
}

fn collab_fork_identity(app: &crate::DesktopApp, path: PathBuf) -> (u64, u64, u64, PathBuf) {
    (
        app.host.document_epoch(),
        app.host.editor_state().document_generation(),
        app.host.editor_state().document_revision(),
        path,
    )
}

#[test]
fn worker_saves_the_captured_revision_not_later_edits() {
    let path = temp_op_path("snapshot");
    let mut state = EditorState::new();
    state.doc.name = Some("captured".into());
    state.mark_document_changed();
    let revision = state.document_revision();
    let mut session = SaveSession::new();
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), true, None),
        EnqueueOutcome::Started
    );
    state.doc.name = Some("edited-after-save".into());
    state.mark_document_changed();

    let completion = session.wait_next().expect("save completion");
    assert!(completion.result.is_ok());
    assert_eq!(completion.revision, revision);
    let loaded = op_host_services::doc_io::load_editor_state(&path, op_editor_core::Locale::EnUs)
        .expect("load saved snapshot");
    assert_eq!(loaded.doc.name.as_deref(), Some("captured"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn identical_in_flight_request_is_not_duplicated() {
    let path = temp_op_path("dedupe");
    let state = EditorState::new();
    let mut session = SaveSession::new();
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), false, None),
        EnqueueOutcome::Started
    );
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), false, None),
        EnqueueOutcome::AlreadyPending
    );
    assert!(session.wait_next().expect("save completion").result.is_ok());
    assert!(!session.is_active());
    let _ = std::fs::remove_file(path);
}

#[test]
fn pending_identity_distinguishes_save_from_save_as() {
    let path = temp_op_path("operation-identity");
    let state = EditorState::new();
    let mut session = SaveSession::new();
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), false, None),
        EnqueueOutcome::Started
    );
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), true, None),
        EnqueueOutcome::Queued,
        "a shared save must not impersonate an exact Save-As request"
    );

    let shared = session.wait_next().expect("shared save completion");
    assert!(shared.result.is_ok());
    assert!(!shared.set_current_path);
    let save_as = session.wait_next().expect("Save-As completion");
    assert!(save_as.result.is_ok());
    assert!(save_as.set_current_path);
    let _ = std::fs::remove_file(path);
}

#[test]
fn pending_target_is_scoped_to_the_document_epoch() {
    let path = temp_op_path("target-epoch");
    let state = EditorState::new();
    let mut session = SaveSession::new();
    assert_eq!(
        session.enqueue(&state, 7, path.clone(), true, None),
        EnqueueOutcome::Started
    );
    assert_eq!(session.latest_target(7), Some(path.as_path()));
    assert_eq!(session.latest_target(8), None);
    assert!(session.wait_next().expect("save completion").result.is_ok());
    let _ = std::fs::remove_file(path);
}

#[test]
fn unchanged_bound_document_skips_snapshot_only_while_the_file_exists() {
    let path = temp_op_path("unchanged-skip");
    std::fs::write(&path, b"existing OP").expect("write bound file marker");
    let mut state = EditorState::new();
    state.mark_saved_revision();

    assert!(can_skip_unchanged_current_save(
        &state,
        Some(&path),
        &path,
        false
    ));
    assert!(!can_skip_unchanged_current_save(
        &state,
        Some(&path),
        &path,
        true
    ));
    state.mark_document_changed();
    assert!(!can_skip_unchanged_current_save(
        &state,
        Some(&path),
        &path,
        false
    ));
    std::fs::remove_file(&path).expect("remove bound file marker");
    state.mark_saved_revision();
    assert!(!can_skip_unchanged_current_save(
        &state,
        Some(&path),
        &path,
        false
    ));
}

#[test]
fn collaboration_fork_path_comparison_normalizes_equivalent_paths() {
    let source = temp_op_path("path-identity");
    std::fs::write(&source, b"existing OP").expect("write source");
    let dotted_alias = source
        .parent()
        .expect("source parent")
        .join(".")
        .join(source.file_name().expect("source file name"));
    assert!(!collaboration_fork_target_is_safe(
        Some(&source),
        &dotted_alias
    ));

    let distinct = temp_op_path("path-identity-distinct");
    assert!(collaboration_fork_target_is_safe(Some(&source), &distinct));
    assert!(collaboration_fork_target_is_safe(None, &source));

    let unresolved = temp_op_path("missing-path-parent")
        .with_extension("")
        .join("fork.op");
    assert!(
        !collaboration_fork_target_is_safe(Some(&source), &unresolved),
        "an unresolved target must fail closed"
    );
    let _ = std::fs::remove_file(source);
}

#[cfg(unix)]
#[test]
fn collaboration_fork_path_comparison_rejects_symlink_alias() {
    let source = temp_op_path("path-symlink-source");
    let alias = temp_op_path("path-symlink-alias");
    std::fs::write(&source, b"existing OP").expect("write source");
    std::os::unix::fs::symlink(&source, &alias).expect("create source alias");

    assert!(!collaboration_fork_target_is_safe(Some(&source), &alias));

    let _ = std::fs::remove_file(alias);
    let _ = std::fs::remove_file(source);
}

#[test]
fn next_capture_reuses_only_the_live_in_flight_snapshot() {
    let path = temp_op_path("capture-anchor");
    let mut state = EditorState::new();
    let mut session = SaveSession::new();
    assert_eq!(
        session.enqueue(&state, 7, path.clone(), false, None),
        EnqueueOutcome::Started
    );

    let first = session.running.as_ref().expect("running snapshot");
    assert!(std::ptr::eq(
        session
            .capture_anchor(7, state.document_generation())
            .unwrap(),
        first.snapshot.as_ref()
    ));
    assert!(session
        .capture_anchor(8, state.document_generation())
        .is_none());

    state.doc.name = Some("new revision".into());
    state.mark_document_changed();
    assert_eq!(
        session.enqueue(&state, 7, path.clone(), false, None),
        EnqueueOutcome::Queued
    );
    let queued = session.queued.as_ref().expect("queued snapshot");
    assert!(std::ptr::eq(
        session
            .capture_anchor(7, state.document_generation())
            .unwrap(),
        queued.snapshot.as_ref()
    ));

    while let Some(completion) = session.wait_next() {
        assert!(completion.result.is_ok());
    }
    let _ = std::fs::remove_file(path);
}

#[test]
fn queued_requests_coalesce_to_the_latest_revision_and_commit_in_order() {
    let path = temp_op_path("coalesce");
    let mut state = EditorState::new();
    state.doc.name = Some("first".into());
    state.mark_document_changed();
    let first_revision = state.document_revision();
    let mut session = SaveSession::new();
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), false, None),
        EnqueueOutcome::Started
    );

    state.doc.name = Some("superseded".into());
    state.mark_document_changed();
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), false, None),
        EnqueueOutcome::Queued
    );
    state.doc.name = Some("latest".into());
    state.mark_document_changed();
    let latest_revision = state.document_revision();
    assert_eq!(
        session.enqueue(&state, 0, path.clone(), false, None),
        EnqueueOutcome::Queued
    );

    let first = session.wait_next().expect("first save completion");
    assert!(first.result.is_ok());
    assert_eq!(first.revision, first_revision);
    let latest = session.wait_next().expect("latest save completion");
    assert!(latest.result.is_ok());
    assert_eq!(latest.revision, latest_revision);
    assert!(!session.is_active());

    let loaded = op_host_services::doc_io::load_editor_state(&path, op_editor_core::Locale::EnUs)
        .expect("load final saved snapshot");
    assert_eq!(loaded.doc.name.as_deref(), Some("latest"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn stale_epoch_ack_cannot_rebind_save_as_to_a_replaced_document() {
    let mut app = crate::DesktopApp::new(None);
    let old_epoch = app.host.document_epoch();
    let old_generation = app.host.editor_state().document_generation();
    let old_revision = app.host.editor_state().document_revision();
    app.host.replace_editor_state(EditorState::new());
    assert_ne!(app.host.document_epoch(), old_epoch);

    let applied = app.apply_save_completion(SaveCompletion {
        path: PathBuf::from("stale-save-as.op"),
        set_current_path: true,
        document_epoch: old_epoch,
        generation: old_generation,
        revision: old_revision,
        result: Ok(()),
    });

    assert!(applied);
    assert!(app.current_path.is_none());
    assert!(app
        .host
        .editor_state()
        .editor_ui
        .file_name_display
        .is_none());
}

#[test]
fn registered_stale_fork_completion_cannot_leave_the_live_collaboration() {
    let mut app = crate::DesktopApp::new(None);
    let old_epoch = app.host.document_epoch();
    let old_generation = app.host.editor_state().document_generation();
    let old_revision = app.host.editor_state().document_revision();
    let path = PathBuf::from("stale-collaboration-fork.op");
    app.collab_fork_saves
        .push((old_epoch, old_generation, old_revision, path.clone()));

    app.host.replace_editor_state(EditorState::new());
    bind_active_guest(&mut app);
    assert_ne!(app.host.document_epoch(), old_epoch);
    assert!(app.apply_save_completion(SaveCompletion {
        path,
        set_current_path: true,
        document_epoch: old_epoch,
        generation: old_generation,
        revision: old_revision,
        result: Ok(()),
    }));

    assert!(app.collab_fork_saves.is_empty());
    assert_eq!(
        app.host.editor_state().editor_ui.collab.phase,
        op_editor_core::CollabConnectionPhase::Active
    );
    assert!(app.current_path.is_none());
}

#[test]
fn already_pending_without_exact_fork_registration_cannot_gain_leave_authority() {
    let mut app = crate::DesktopApp::new(None);
    bind_active_guest(&mut app);
    let identity = collab_fork_identity(&app, temp_op_path("unregistered-pending"));

    assert!(
        !app.confirm_collaboration_fork_enqueue(identity.clone(), EnqueueOutcome::AlreadyPending)
    );
    assert!(app.collab_fork_saves.is_empty());

    assert!(app.confirm_collaboration_fork_enqueue(identity.clone(), EnqueueOutcome::Started));
    assert_eq!(app.collab_fork_saves, vec![identity.clone()]);
    assert!(app.confirm_collaboration_fork_enqueue(identity, EnqueueOutcome::AlreadyPending));
    assert_eq!(app.collab_fork_saves.len(), 1);
}

#[test]
fn successful_guest_fork_leaves_only_after_worker_completion_is_applied() {
    let source = temp_op_path("collab-source");
    let target = temp_op_path("collab-fork");
    let mut app = crate::DesktopApp::new(None);
    app.host.editor_state_mut().doc.name = Some("Shared source".into());
    app.host.editor_state_mut().mark_document_changed();
    op_host_services::doc_io::save_to_path(app.host.editor_state(), &source)
        .expect("write shared source");
    app.host.editor_state_mut().mark_saved_revision();
    app.current_path = Some(source.clone());
    let source_bytes = std::fs::read(&source).expect("read shared source");

    bind_active_guest(&mut app);
    app.host.editor_state_mut().doc.name = Some("Guest local fork".into());
    app.host.editor_state_mut().mark_document_changed();
    assert!(app.host.gate_collaboration_action(
        op_editor_core::CollabGateAction::SaveFork,
        op_editor_core::CollabEditSource::User,
    ));
    let identity = collab_fork_identity(&app, target.clone());
    let outcome = app.enqueue_background_save(target.clone(), true);
    assert!(app.confirm_collaboration_fork_enqueue(identity, outcome));
    assert_eq!(app.collab_fork_saves.len(), 1);

    let completion = app
        .save_session
        .wait_next()
        .expect("background Save As completion");
    assert!(completion.result.is_ok(), "{:?}", completion.result);
    assert_eq!(
        app.host.editor_state().editor_ui.collab.phase,
        op_editor_core::CollabConnectionPhase::Active,
        "a completed worker has no leave authority before its UI-thread acknowledgement"
    );
    assert_eq!(app.current_path.as_deref(), Some(source.as_path()));
    assert_eq!(
        std::fs::read(&source).expect("re-read shared source"),
        source_bytes,
        "Save As must not overwrite the collaboration source path"
    );

    assert!(app.apply_save_completion(completion));
    assert_eq!(
        app.host.editor_state().editor_ui.collab.phase,
        op_editor_core::CollabConnectionPhase::Idle
    );
    assert!(app.collab_fork_saves.is_empty());
    assert_eq!(app.current_path.as_deref(), Some(target.as_path()));
    let fork = op_host_services::doc_io::load_editor_state(&target, op_editor_core::Locale::EnUs)
        .expect("load local fork");
    assert_eq!(fork.doc.name.as_deref(), Some("Guest local fork"));
    let shared = op_host_services::doc_io::load_editor_state(&source, op_editor_core::Locale::EnUs)
        .expect("reload shared source");
    assert_eq!(shared.doc.name.as_deref(), Some("Shared source"));

    let _ = std::fs::remove_file(source);
    let _ = std::fs::remove_file(target);
}

#[test]
fn failed_guest_fork_worker_does_not_touch_or_leave_the_shared_document() {
    let source = temp_op_path("failed-collab-source");
    let missing_parent = temp_op_path("missing-parent").with_extension("");
    let target = missing_parent.join("fork.op");
    let mut app = crate::DesktopApp::new(None);
    app.host.editor_state_mut().doc.name = Some("Shared source".into());
    app.host.editor_state_mut().mark_document_changed();
    op_host_services::doc_io::save_to_path(app.host.editor_state(), &source)
        .expect("write shared source");
    app.host.editor_state_mut().mark_saved_revision();
    app.current_path = Some(source.clone());
    let source_bytes = std::fs::read(&source).expect("read shared source");

    bind_active_guest(&mut app);
    app.host.editor_state_mut().doc.name = Some("Unsaved guest fork".into());
    app.host.editor_state_mut().mark_document_changed();
    let identity = collab_fork_identity(&app, target.clone());
    let outcome = app.enqueue_background_save(target.clone(), true);
    assert!(app.confirm_collaboration_fork_enqueue(identity, outcome));
    let completion = app
        .save_session
        .wait_next()
        .expect("failed background Save As completion");

    assert!(completion.result.is_err());
    assert_eq!(app.collab_fork_saves.len(), 1);
    assert_eq!(
        app.host.editor_state().editor_ui.collab.phase,
        op_editor_core::CollabConnectionPhase::Active
    );
    assert_eq!(app.current_path.as_deref(), Some(source.as_path()));
    assert!(!target.exists());
    assert_eq!(
        std::fs::read(&source).expect("re-read shared source"),
        source_bytes
    );

    let _ = std::fs::remove_file(source);
}

#[test]
fn unregistered_save_completion_cannot_leave_an_active_guest() {
    let target = temp_op_path("non-fork-completion");
    let mut app = crate::DesktopApp::new(None);
    bind_active_guest(&mut app);
    assert!(app.collab_fork_saves.is_empty());
    assert_eq!(
        app.save_session.enqueue(
            app.host.editor_state(),
            app.host.document_epoch(),
            target.clone(),
            false,
            None,
        ),
        EnqueueOutcome::Started
    );
    let completion = app
        .save_session
        .wait_next()
        .expect("background save completion");
    assert!(completion.result.is_ok(), "{:?}", completion.result);

    assert!(app.apply_save_completion(completion));
    assert_eq!(
        app.host.editor_state().editor_ui.collab.phase,
        op_editor_core::CollabConnectionPhase::Active,
        "a cancelled Save As registers no fork identity, so another completion cannot detach it"
    );

    let _ = std::fs::remove_file(target);
}
