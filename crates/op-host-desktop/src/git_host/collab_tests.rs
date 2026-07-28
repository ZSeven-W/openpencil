use crate::DesktopApp;

fn bind_as_editor(app: &mut DesktopApp) {
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

#[test]
fn worktree_rewrite_gate_rejects_an_active_collaboration() {
    let mut app = DesktopApp::new(None);
    assert!(
        app.collaboration_allows_git_worktree_rewrite(),
        "standalone Git behavior stays unchanged"
    );

    bind_as_editor(&mut app);
    assert!(!app.collaboration_allows_git_worktree_rewrite());
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .collab
            .notice
            .map(|notice| notice.kind),
        Some(op_editor_core::CollabNoticeKind::UnsupportedEdit(
            op_editor_core::CollabUnsupportedFeature::ReplaceDocument,
        ))
    );
}

#[test]
fn milestone_save_is_rejected_before_touching_the_repository() {
    let mut app = DesktopApp::new(None);
    bind_as_editor(&mut app);
    {
        let panel = &mut app.host.editor_state_mut().editor_ui.git_panel;
        panel.commit_input.set_text("Checkpoint");
        panel.pending_action = Some(op_editor_core::GitPanelAction::CommitMilestone);
    }

    app.drain_git_action();

    let state = app.host.editor_state();
    assert_eq!(state.editor_ui.git_panel.commit_input.text(), "Checkpoint");
    assert!(!state.editor_ui.git_panel.author_prompt);
    assert_eq!(
        state.editor_ui.collab.notice.map(|notice| notice.kind),
        Some(op_editor_core::CollabNoticeKind::Reject(
            op_editor_core::CollabRejectUiCode::ReadOnly,
        ))
    );
}

#[test]
fn transition_barrier_is_ready_when_no_pull_is_running() {
    let mut app = DesktopApp::new(None);
    assert!(app.settle_git_before_collaboration_transition());
}
