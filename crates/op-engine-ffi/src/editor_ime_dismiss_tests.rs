use super::op_editor_ime_dismiss;
use crate::desc::{Callbacks, CreateOptions};
use crate::lifecycle::{OpEngine, Session};
use crate::{op_editor_back, OpStatus};
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};

fn engine() -> OpEngine {
    OpEngine::new(Session::new(CreateOptions {
        document: r#"{"version":"1.0.0","children":[{"type":"frame","id":"board","width":390,"height":600}]}"#.into(),
        width: 390.0, height: 844.0, dpr: 1.0,
        callbacks: Callbacks::default(), asset_base: None,
        editor_mode: true, documents_root: None,
    }).unwrap())
}

fn dismiss(engine: &mut OpEngine) -> bool {
    let mut changed = false;
    assert_eq!(
        unsafe { op_editor_ime_dismiss(engine, &mut changed) },
        OpStatus::Ok
    );
    changed
}

#[test]
fn keyboard_dismiss_keeps_home_draft_and_connection_card() {
    let mut engine = engine();
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    let before = host.editor_state().doc.clone();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.home.visible = true;
    ui.home.composer_focused = true;
    ui.home.connect_card_open = true;
    ui.home.set_draft("keep my unfinished idea");
    assert!(dismiss(&mut engine));
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    assert!(!host.text_input_focus_active());
    assert!(host.editor_state().editor_ui.home.connect_card_open);
    assert_eq!(
        host.editor_state().editor_ui.home.input.text(),
        "keep my unfinished idea"
    );
    assert_eq!(host.editor_state().doc, before);
    assert!(!dismiss(&mut engine));
}

#[test]
fn keyboard_dismiss_keeps_settings_draft_and_next_back_closes_settings() {
    let mut engine = engine();
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    let before = host.editor_state().doc.clone();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.home.visible = true;
    ui.agent_settings_open = true;
    ui.agent_settings.begin_builtin_agent_draft();
    ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::ApiKey));
    ui.settings_input.set_text("test-only-key");
    assert!(dismiss(&mut engine));
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    let ui = &host.editor_state().editor_ui;
    assert!(ui.agent_settings_open);
    assert!(ui.agent_settings.focus.is_none());
    assert_eq!(
        ui.agent_settings
            .builtin_agent_draft
            .as_ref()
            .unwrap()
            .api_key,
        "test-only-key"
    );
    assert!(ui.agent_settings.builtin_agents.is_empty());
    assert!(!host.text_input_focus_active());
    assert_eq!(host.editor_state().doc, before);
    let mut consumed = false;
    assert_eq!(
        unsafe { op_editor_back(&mut engine, &mut consumed) },
        OpStatus::Ok
    );
    assert!(consumed);
    assert!(
        !engine
            .session_mut_for_test()
            .editor_mut()
            .unwrap()
            .editor_state()
            .editor_ui
            .agent_settings_open
    );
}

#[test]
fn keyboard_dismiss_does_not_cancel_a_save_dialog_or_its_name() {
    let mut engine = engine();
    let host = engine.session_mut_for_test().editor_mut().unwrap();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.home.visible = true;
    ui.save_name_dialog.open = true;
    ui.save_name_dialog.input.set_text("preserve this name");
    assert!(!dismiss(&mut engine));
    let ui = &engine
        .session_mut_for_test()
        .editor_mut()
        .unwrap()
        .editor_state()
        .editor_ui;
    assert!(ui.save_name_dialog.open);
    assert_eq!(ui.save_name_dialog.input.text(), "preserve this name");
}
