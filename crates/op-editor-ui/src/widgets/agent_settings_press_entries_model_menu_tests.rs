//! Model-dropdown press-arm tests for the built-in provider form.
//!
//! Split out of `agent_settings_press_entries.rs` to keep both files
//! under the repo's 800-line cap.

use crate::widgets::agent_settings_panel::AgentSettingsHit;
use crate::widgets::agent_settings_press_entries::apply_entry_hit;
use crate::widgets::agent_settings_press_flow::SettingsPressOutcome;
use op_editor_core::agent_settings::{
    BuiltinAgentField, BuiltinAgentPresetMenuTarget, SettingsFocus,
};
use op_editor_core::host_settings_commit::SettingsCommitScope;
use op_editor_core::EditorState;

#[test]
fn focusing_the_model_field_opens_the_menu_and_queues_due_discovery() {
    let mut state = EditorState::new();
    let id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "MiniMax",
        "sk-new",
        "MiniMax-M3",
        op_editor_core::BuiltinAgentKind::Anthropic,
        "https://api.minimaxi.com/anthropic",
    );

    let outcome = apply_entry_hit(
        &mut state,
        AgentSettingsHit::FocusBuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        },
        SettingsCommitScope::Browser,
        12,
    );

    assert_eq!(outcome, SettingsPressOutcome::handled());
    let settings = &mut state.editor_ui.agent_settings;
    assert_eq!(
        settings.builtin_model_menu_open,
        Some(op_editor_core::agent_settings::BuiltinModelMenuTarget::Agent(0))
    );
    assert_eq!(
        settings.focus,
        Some(SettingsFocus::BuiltinAgent {
            index: 0,
            field: BuiltinAgentField::Model,
        })
    );
    let request = settings
        .take_pending_builtin_model_catalog_refresh()
        .expect("opening the menu queues discovery for a configured agent");
    assert_eq!(
        request.target,
        op_editor_core::BuiltinModelCatalogTarget::Agent(id)
    );
}

#[test]
fn focusing_the_draft_model_field_queues_draft_discovery() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.begin_builtin_agent_draft();
    state
        .editor_ui
        .agent_settings
        .builtin_agent_draft
        .as_mut()
        .expect("draft")
        .api_key = "sk-new".into();

    apply_entry_hit(
        &mut state,
        AgentSettingsHit::FocusBuiltinAgentDraft(BuiltinAgentField::Model),
        SettingsCommitScope::Browser,
        5,
    );

    let settings = &mut state.editor_ui.agent_settings;
    assert_eq!(
        settings.builtin_model_menu_open,
        Some(op_editor_core::agent_settings::BuiltinModelMenuTarget::Draft)
    );
    let request = settings
        .take_pending_builtin_model_catalog_refresh()
        .expect("a credentialed draft is discovery-ready");
    assert_eq!(
        request.target,
        op_editor_core::BuiltinModelCatalogTarget::Draft
    );
}

#[test]
fn selecting_model_rows_toggles_multiple_values_and_keeps_the_menu_open() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.begin_builtin_agent_draft();
    {
        let draft = state
            .editor_ui
            .agent_settings
            .builtin_agent_draft
            .as_mut()
            .expect("draft");
        draft.api_key = "sk-new".into();
        draft.models.clear();
    }
    let request = state
        .editor_ui
        .agent_settings
        .begin_builtin_model_catalog_refresh(op_editor_core::BuiltinModelCatalogTarget::Draft, 0)
        .expect("discovery request");
    let expected = state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("resolvable");
    state
        .editor_ui
        .agent_settings
        .apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            op_editor_core::BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![
                    op_editor_core::BuiltinModelOption::new(
                        "claude-sonnet-4-6-20250916",
                        "Claude Sonnet 4.6",
                    ),
                    op_editor_core::BuiltinModelOption::new("claude-opus-4-1", "Claude Opus 4.1"),
                ],
            },
        );
    apply_entry_hit(
        &mut state,
        AgentSettingsHit::FocusBuiltinAgentDraft(BuiltinAgentField::Model),
        SettingsCommitScope::Browser,
        10,
    );

    let outcome = apply_entry_hit(
        &mut state,
        AgentSettingsHit::SelectBuiltinModel {
            index: None,
            row: 1,
        },
        SettingsCommitScope::Browser,
        11,
    );

    assert_eq!(outcome, SettingsPressOutcome::handled());
    let settings = &state.editor_ui.agent_settings;
    assert_eq!(
        settings.builtin_agent_draft.as_ref().expect("draft").models,
        vec!["claude-opus-4-1"]
    );
    assert_eq!(
        settings.builtin_model_menu_open,
        Some(op_editor_core::agent_settings::BuiltinModelMenuTarget::Draft)
    );
    assert_eq!(
        settings.focus,
        Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model))
    );

    apply_entry_hit(
        &mut state,
        AgentSettingsHit::SelectBuiltinModel {
            index: None,
            row: 0,
        },
        SettingsCommitScope::Browser,
        12,
    );
    assert_eq!(
        state
            .editor_ui
            .agent_settings
            .builtin_agent_draft
            .as_ref()
            .expect("draft")
            .models,
        vec!["claude-opus-4-1", "claude-sonnet-4-6-20250916"]
    );

    apply_entry_hit(
        &mut state,
        AgentSettingsHit::SelectBuiltinModel {
            index: None,
            row: 1,
        },
        SettingsCommitScope::Browser,
        13,
    );
    assert_eq!(
        state
            .editor_ui
            .agent_settings
            .builtin_agent_draft
            .as_ref()
            .expect("draft")
            .models,
        vec!["claude-sonnet-4-6-20250916"],
        "pressing a checked row removes only that model"
    );
}

#[test]
fn toggling_the_chevron_closes_an_open_model_menu() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.begin_builtin_agent_draft();
    state
        .editor_ui
        .agent_settings
        .builtin_agent_draft
        .as_mut()
        .expect("draft")
        .api_key = "sk-new".into();
    apply_entry_hit(
        &mut state,
        AgentSettingsHit::FocusBuiltinAgentDraft(BuiltinAgentField::Model),
        SettingsCommitScope::Browser,
        10,
    );
    assert_eq!(
        state.editor_ui.agent_settings.builtin_model_menu_open,
        Some(op_editor_core::agent_settings::BuiltinModelMenuTarget::Draft)
    );

    apply_entry_hit(
        &mut state,
        AgentSettingsHit::ToggleBuiltinModelMenu(None),
        SettingsCommitScope::Browser,
        11,
    );

    assert_eq!(state.editor_ui.agent_settings.builtin_model_menu_open, None);
    assert_eq!(
        state.editor_ui.agent_settings.focus,
        Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model))
    );
}

#[test]
fn provider_and_model_dropdowns_are_mutually_exclusive() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.begin_builtin_agent_draft();
    state.editor_ui.agent_settings.builtin_preset_menu_open =
        Some(BuiltinAgentPresetMenuTarget::Draft);

    apply_entry_hit(
        &mut state,
        AgentSettingsHit::FocusBuiltinAgentDraft(BuiltinAgentField::Model),
        SettingsCommitScope::Browser,
        10,
    );
    assert_eq!(
        state.editor_ui.agent_settings.builtin_preset_menu_open,
        None
    );
    assert!(state
        .editor_ui
        .agent_settings
        .builtin_model_menu_open
        .is_some());

    apply_entry_hit(
        &mut state,
        AgentSettingsHit::ToggleBuiltinAgentPresetMenu(None),
        SettingsCommitScope::Browser,
        11,
    );
    assert_eq!(state.editor_ui.agent_settings.builtin_model_menu_open, None);
    assert_eq!(
        state.editor_ui.agent_settings.builtin_preset_menu_open,
        Some(BuiltinAgentPresetMenuTarget::Draft)
    );
}
