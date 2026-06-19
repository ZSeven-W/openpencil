use super::WidgetHost;
use op_editor_core::{
    CloneField, CloneFormState, EditorState, GitBranchPickerMode, GitFileEntry, GitPanelAction,
    NodeId,
};
use op_editor_ui::{KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers};

fn seed_text_edit(host: &mut WidgetHost, content: &str) {
    let doc = jian_ops_schema::load_str(&format!(
        r##"{{"version":"0.8.0","children":[
          {{"type":"text","id":"t1","name":"Title","x":0,"y":0,"width":100,"height":24,
           "content":{content:?},"font_size":16,"fills":[{{"type":"solid","color":"#111827"}}]}}
        ]}}"##
    ))
    .expect("fixture JSON parses")
    .value;
    host.editor_state = EditorState::from_document(doc);
    host.editor_state.set_single_selection(NodeId::new("t1"));
    assert!(host.editor_state.start_text_edit(NodeId::new("t1")));
}

#[test]
fn apply_key_unhandled_event_reports_no_change() {
    let mut host = WidgetHost::new();
    host.editor_state_dirty = false;

    let event = KeyEvent {
        key: KeyValue::Char('a'),
        code: KeyCode::KeyA,
        location: KeyLocation::Standard,
        modifiers: Modifiers::empty(),
        state: KeyState::Pressed,
        repeat: false,
        is_composing: false,
    };

    assert!(!host.apply_key(&event));
    assert!(!host.editor_state_dirty);
}

#[test]
fn keydown_shortcut_cmd_shift_k_toggles_component_browser() {
    let mut host = WidgetHost::new();
    host.editor_state_dirty = false;

    assert!(host.apply_keydown_shortcut("K", true, true, false));
    assert!(host.editor_state.editor_ui.component_browser_open);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    host.editor_state
        .editor_ui
        .component_browser_kit_picker_open = true;

    assert!(host.apply_keydown_shortcut("k", true, true, false));
    assert!(!host.editor_state.editor_ui.component_browser_open);
    assert!(
        !host
            .editor_state
            .editor_ui
            .component_browser_kit_picker_open
    );
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(!host.apply_keydown_shortcut("K", true, true, true));
    assert!(!host.editor_state.editor_ui.component_browser_open);
    assert!(!host.editor_state_dirty);
}

#[test]
fn text_edit_horizontal_arrows_move_caret_and_consume_event() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hi");
    host.editor_state_dirty = false;

    assert!(host.apply_text_edit_caret(false));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 1);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_caret(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 2);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_caret(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 2);

    assert!(host.editor_state.text_edit_commit());
    assert!(!host.apply_text_edit_caret(true));
}

#[test]
fn text_edit_vertical_arrows_move_caret_and_consume_event() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hello\nworld");
    assert!(host.editor_state.text_edit_set_caret(11, false, 0));
    host.editor_state_dirty = false;

    assert!(host.apply_text_edit_vertical(false));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 5);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_vertical(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 11);
    assert!(host.editor_state_dirty);

    assert!(host.editor_state.text_edit_commit());
    assert!(!host.apply_text_edit_vertical(true));
}

#[test]
fn text_edit_line_edge_jumps_within_current_line() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hello\nworld");
    assert!(host.editor_state.text_edit_set_caret(8, false, 0));
    host.editor_state_dirty = false;

    assert!(host.apply_text_edit_line_edge(false));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 6);
    assert!(host.editor_state_dirty);

    host.editor_state_dirty = false;
    assert!(host.apply_text_edit_line_edge(true));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 11);
    assert!(host.editor_state_dirty);

    assert!(host.editor_state.text_edit_commit());
    assert!(!host.apply_text_edit_line_edge(true));
}

#[test]
fn text_edit_enter_inserts_newline_instead_of_committing() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "hello\nworld");
    host.editor_state_dirty = false;

    assert!(host.apply_send());
    assert_eq!(
        host.editor_state.ui.text_editing,
        Some(NodeId::new("t1")),
        "Enter must keep the text edit session open"
    );
    assert_eq!(
        host.editor_state.text_edit_content(),
        Some("hello\nworld\n")
    );
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 12);
    assert!(host.editor_state_dirty);

    assert!(host.apply_text('!'));
    assert_eq!(
        host.editor_state.text_edit_content(),
        Some("hello\nworld\n!")
    );
}

#[test]
fn text_edit_delete_removes_character_after_caret() {
    let mut host = WidgetHost::new();
    seed_text_edit(&mut host, "abcd");
    assert!(host.editor_state.text_edit_set_caret(1, false, 0));
    host.editor_state_dirty = false;

    assert!(host.apply_delete());

    assert_eq!(host.editor_state.text_edit_content(), Some("acd"));
    assert_eq!(host.editor_state.ui.text_edit_input.caret(), 1);
    assert!(host.editor_state_dirty);
}

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

#[test]
fn single_key_tool_shortcut_switches_tool_when_no_input_focused() {
    use op_editor_core::Tool;
    let mut host = WidgetHost::new();

    // Bare letters switch the active tool (native-host parity); shape variants
    // also sync the toolbar shape slot.
    assert!(host.apply_tool_shortcut("r"));
    assert_eq!(host.editor_state.tool, Tool::Rect);
    assert_eq!(host.editor_state.editor_ui.shape_tool, Tool::Rect);

    assert!(host.apply_tool_shortcut("t"));
    assert_eq!(host.editor_state.tool, Tool::Text);
    // Non-shape tool leaves the shape slot on its last shape (Rect).
    assert_eq!(host.editor_state.editor_ui.shape_tool, Tool::Rect);

    assert!(host.apply_tool_shortcut("v"));
    assert_eq!(host.editor_state.tool, Tool::Select);

    // Unmapped letters are not consumed, so they fall through to apply_text.
    assert!(!host.apply_tool_shortcut("q"));
    assert_eq!(host.editor_state.tool, Tool::Select);
}

#[test]
fn tool_shortcut_is_suppressed_while_an_input_owns_the_keyboard() {
    use op_editor_core::Tool;
    let mut host = WidgetHost::new();

    // While a text node is being edited, a bare "r" must type into the field,
    // not switch tools — apply_tool_shortcut declines so apply_text wins.
    seed_text_edit(&mut host, "hello");
    host.editor_state.tool = Tool::Select;
    assert!(host.input_active());
    assert!(!host.apply_tool_shortcut("r"));
    assert_eq!(host.editor_state.tool, Tool::Select);
}

#[test]
fn tool_shortcut_does_not_steal_keys_from_preset_name_input() {
    use op_editor_core::Tool;
    let mut host = WidgetHost::new();
    host.editor_state.tool = Tool::Select;

    // The variables preset dropdown's save-as-name input owns the keyboard.
    host.editor_state.editor_ui.variables_preset_menu_open = true;
    host.editor_state.editor_ui.variables_preset_name_focus = true;
    assert!(host.editor_state.editor_ui.preset_name_input_active());
    assert!(host.input_active());

    // A bare "r" must type into the preset name, not switch to the Rect tool.
    assert!(!host.apply_tool_shortcut("r"));
    assert_eq!(host.editor_state.tool, Tool::Select);
}

#[test]
fn preset_name_input_receives_typed_keys_backspace_and_commit() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.variables_preset_menu_open = true;
    host.editor_state.editor_ui.variables_preset_name_focus = true;
    host.editor_state.ui.property_input_draft.clear();
    assert!(host.editor_state.editor_ui.preset_name_input_active());

    // Typed keys land in the shared draft the preset menu widget paints.
    assert!(host.apply_text('M'));
    assert!(host.apply_text('i'));
    assert!(host.apply_text('d'));
    assert_eq!(host.editor_state.ui.property_input_draft, "Mid");

    // Backspace pops the last char.
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state.ui.property_input_draft, "Mi");

    // Enter commits the preset and defocuses the name input.
    assert!(host.apply_send());
    assert!(!host.editor_state.editor_ui.variables_preset_name_focus);
    assert!(host.editor_state.ui.property_input_draft.is_empty());
}

#[test]
fn preset_name_input_escape_cancels_without_saving() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.variables_preset_menu_open = true;
    host.editor_state.editor_ui.variables_preset_name_focus = true;
    host.editor_state.ui.property_input_draft = "Draft".to_string();

    // Escape closes just the name input; the dropdown stays open.
    assert!(host.apply_escape());
    assert!(!host.editor_state.editor_ui.variables_preset_name_focus);
    assert!(host.editor_state.editor_ui.variables_preset_menu_open);
    assert!(host.editor_state.ui.property_input_draft.is_empty());
}
