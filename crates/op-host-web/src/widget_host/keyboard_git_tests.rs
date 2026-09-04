//! Git-panel keyboard ownership: the panel's text inputs, the clone wizard,
//! and the Escape ladder inside it.
//!
//! Split out of `keyboard_tests.rs` (pure code motion) to keep that file
//! under the repo's 800-line cap; the git tests own their own fixture
//! (`host_with_git_panel_open`) and share nothing with the ones left behind.

use super::WidgetHost;
use op_editor_core::{
    CloneField, CloneFormState, GitBranchPickerMode, GitFileEntry, GitPanelAction,
};

fn host_with_git_panel_open() -> WidgetHost {
    let mut host = WidgetHost::new();
    let panel = &mut host.editor_state_mut().editor_ui.git_panel;
    panel.open = true;
    panel.loading = false;
    panel.in_repo = false;
    panel.has_saved_file = false;
    host
}

#[test]
fn git_commit_input_uses_text_input_state_for_editing() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        panel.commit_focused = true;
        panel.commit_input.set_text("design");
        panel.commit_no_changes = true;
        panel.changed_files = vec![GitFileEntry {
            path: "design.op".into(),
            staged: true,
            status: 'M',
        }];
    }

    assert!(host.input_active());
    assert!(host.apply_select_all());
    assert!(host
        .editor_state()
        .editor_ui
        .git_panel
        .commit_input
        .is_select_all());

    assert!(host.apply_text('改'));
    {
        let panel = &host.editor_state().editor_ui.git_panel;
        assert_eq!(panel.commit_input.text(), "改");
        assert!(!panel.commit_no_changes);
    }
    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.commit_input.text(),
        ""
    );

    assert!(host.apply_text('好'));
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::Commit)
    );
}

#[test]
fn git_remote_inputs_use_text_input_state_for_editing() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        panel.remote_focused = true;
        panel.remote_input.set_text("https://old.example/repo.git");
    }

    assert!(host.apply_select_all());
    for c in "https://new.example/repo.git".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::SetRemote(
            "https://new.example/repo.git".to_string()
        ))
    );

    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.pending_action = None;
        panel.remote_focused = false;
        panel.https_focused = true;
        panel.https_input.set_text("user:old-token");
    }
    assert!(host.apply_select_all());
    for c in "user:new-token".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::SetHttpsAuth("user:new-token".to_string()))
    );
}

#[test]
fn git_branch_create_input_uses_text_input_state_for_editing() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.branch = Some("main".to_string());
        panel.branch_picker_open = true;
        panel.branch_picker_mode = GitBranchPickerMode::Create;
        panel.branch_create_focused = true;
        panel.branch_create_input.set_text("old");
    }

    assert!(host.apply_select_all());
    for c in "feature/web".chars() {
        assert!(host.apply_text(c));
    }
    assert!(host.apply_send());
    let panel = &host.editor_state().editor_ui.git_panel;
    assert_eq!(
        panel.pending_action,
        Some(GitPanelAction::CreateBranch("feature/web".to_string()))
    );
    assert_eq!(panel.branch_picker_mode, GitBranchPickerMode::List);
    assert!(panel.branch_create_input.text().is_empty());
    assert!(!panel.branch_create_focused);
    assert!(!panel.branch_picker_open);
}

#[test]
fn clone_wizard_owns_keyboard_and_enter() {
    let mut host = host_with_git_panel_open();
    host.editor_state_mut().editor_ui.git_panel.clone_form = Some(CloneFormState {
        focus: Some(CloneField::Url),
        ..Default::default()
    });

    assert!(host.input_active());
    for c in "http".chars() {
        assert!(host.apply_text(c));
    }
    assert_eq!(
        host.editor_state()
            .editor_ui
            .git_panel
            .clone_form
            .as_ref()
            .unwrap()
            .url_input
            .text(),
        "http"
    );
    assert!(host.apply_send());
    assert_eq!(
        host.editor_state().editor_ui.git_panel.pending_action,
        Some(GitPanelAction::SubmitClone)
    );

    host.editor_state_mut().editor_ui.git_panel.pending_action = None;
    host.editor_state_mut()
        .editor_ui
        .git_panel
        .clone_form
        .as_mut()
        .unwrap()
        .focus = None;
    assert!(host.apply_send());
    assert_eq!(host.editor_state().editor_ui.git_panel.pending_action, None);
}

#[test]
fn git_escape_defocuses_then_closes_clone_wizard() {
    let mut host = host_with_git_panel_open();
    host.editor_state_mut().editor_ui.git_panel.clone_form = Some(CloneFormState {
        focus: Some(CloneField::Dest),
        ..Default::default()
    });

    assert!(host.apply_escape());
    {
        let form = host
            .editor_state()
            .editor_ui
            .git_panel
            .clone_form
            .as_ref()
            .unwrap();
        assert_eq!(form.focus, None);
    }

    assert!(host.apply_escape());
    assert!(host.editor_state().editor_ui.git_panel.clone_form.is_none());
}

#[test]
fn git_escape_resets_branch_picker_submode() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.branch_picker_open = true;
        panel.branch_picker_mode = GitBranchPickerMode::Create;
        panel.branch_picker_menu.hover = Some(1);
        panel.branch_create_focused = true;
        panel.branch_create_input.set_text("feature/temp");
    }

    assert!(host.apply_escape());
    let panel = &host.editor_state().editor_ui.git_panel;
    assert_eq!(panel.branch_picker_mode, GitBranchPickerMode::List);
    assert!(panel.branch_picker_open);
    assert_eq!(panel.branch_picker_menu.hover, None);
    assert!(panel.branch_create_input.text().is_empty());
    assert!(!panel.branch_create_focused);
}

#[test]
fn git_escape_dismisses_author_prompt() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.author_prompt = true;
        panel.author_name_focused = true;
        panel.author_email_focused = true;
        panel.author_name_input.set_text("Ada");
        panel.author_email_input.set_text("ada@example.com");
    }

    assert!(host.apply_escape());
    let panel = &host.editor_state().editor_ui.git_panel;
    assert!(!panel.author_prompt);
    assert!(!panel.author_name_focused);
    assert!(!panel.author_email_focused);
    assert_eq!(panel.author_name_input.text(), "Ada");
    assert_eq!(panel.author_email_input.text(), "ada@example.com");
}

#[test]
fn git_escape_defocuses_ready_inputs() {
    let mut host = host_with_git_panel_open();
    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.in_repo = true;
        panel.commit_focused = true;
        panel.commit_input.set_text("message");
    }
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.git_panel.commit_focused);
    assert_eq!(
        host.editor_state().editor_ui.git_panel.commit_input.text(),
        "message"
    );

    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.remote_focused = true;
        panel.remote_input.set_text("https://example.com/repo.git");
    }
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.git_panel.remote_focused);
    assert_eq!(
        host.editor_state().editor_ui.git_panel.remote_input.text(),
        "https://example.com/repo.git"
    );

    {
        let panel = &mut host.editor_state_mut().editor_ui.git_panel;
        panel.https_focused = true;
        panel.https_input.set_text("user:token");
    }
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.git_panel.https_focused);
    assert_eq!(
        host.editor_state().editor_ui.git_panel.https_input.text(),
        "user:token"
    );
}
