use super::*;
use op_editor_core::agent_settings::{BuiltinAgentField, SettingsFocus};
use op_editor_core::host_settings_commit::{commit_settings_focus, SettingsCommitScope};

const PROBE_ROOT: &str = "OPENPENCIL_ZODE_SETTINGS_PROBE_ROOT";
const PROBE_STAGE: &str = "OPENPENCIL_ZODE_SETTINGS_PROBE_STAGE";

#[test]
fn edited_imported_model_survives_settings_save_and_cold_startup_import() {
    let root = std::env::temp_dir().join(format!(
        "openpencil-zode-settings-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("zode.json"),
        r#"{"providers":{
          "anthropic":{"type":"anthropic","apiKey":"fake-source-anthropic","models":{"default":{}}},
          "glm-coding-plan":{"type":"openai","apiKey":"fake-source-glm","baseUrl":"https://open.bigmodel.cn/api/coding/paas/v4","models":{"glm-5.2":{}}}
        }}"#,
    )
    .unwrap();

    // Separate processes exercise both writing and cold startup without
    // changing this parallel test process's global config root or real keys.
    for stage in ["save", "load"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "zode_import::persistence_tests::imported_provider_persistence_probe",
                "--ignored",
            ])
            .env(PROBE_ROOT, &root)
            .env(PROBE_STAGE, stage)
            .output()
            .expect("settings probe process starts");
        assert!(
            output.status.success(),
            "stage={stage}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "run in isolated processes by edited_imported_model_survives_settings_save_and_cold_startup_import"]
fn imported_provider_persistence_probe() {
    let Some(root) = std::env::var_os(PROBE_ROOT).map(PathBuf::from) else {
        return;
    };
    op_config_store::configure_user_root(&root).unwrap();
    let mut state = EditorState::new();
    if std::env::var(PROBE_STAGE).unwrap() == "save" {
        import_zode_builtin_agents_from_path(&mut state, &root.join("zode.json"));
        let index = state
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .position(|agent| agent.display_name == "glm-coding-plan")
            .unwrap();
        let id = state.editor_ui.agent_settings.builtin_agents[index]
            .id
            .clone();
        let row = state
            .chat
            .available_models
            .iter()
            .position(|entry| entry.builtin_provider_id.as_deref() == Some(&id))
            .unwrap();
        state.select_chat_model(row);

        // Focus and commit without changing a value must keep source-only
        // credentials excluded from OpenPencil's settings file.
        commit_model(&mut state, index, "glm-5.2");
        assert!(state
            .editor_ui
            .agent_settings
            .imported_agent_ids
            .contains(&id));
        crate::settings_io::save_checked(&state).unwrap();
        let saved = std::fs::read_to_string(root.join("settings.json")).unwrap();
        assert!(!saved.contains("fake-source-glm"));
        assert!(!saved.contains("fake-source-anthropic"));

        let before = crate::settings_io::fingerprint(&state);
        commit_model(&mut state, index, "glm-5.3-flash");
        assert!(!state
            .editor_ui
            .agent_settings
            .imported_agent_ids
            .contains(&id));
        crate::settings_io::save_if_changed(&state, before);
        let saved = std::fs::read_to_string(root.join("settings.json")).unwrap();
        assert!(saved.contains("glm-5.3-flash"));
        assert!(!saved.contains("fake-source-anthropic"));
    } else {
        // DesktopApp::new performs these operations in this exact order.
        crate::settings_io::load(&mut state);
        import_zode_builtin_agents_from_path(&mut state, &root.join("zode.json"));
        let providers: Vec<_> = state
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .filter(|agent| agent.display_name == "glm-coding-plan")
            .collect();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].models, ["glm-5.3-flash"]);
        assert!(!state
            .editor_ui
            .agent_settings
            .imported_agent_ids
            .contains(&providers[0].id));
    }
    assert_eq!(
        state
            .chat
            .selected_model_entry()
            .and_then(|entry| entry.builtin_model_id()),
        Some("glm-5.3-flash")
    );
}

fn commit_model(state: &mut EditorState, index: usize, model: &str) {
    state.editor_ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgent {
        index,
        field: BuiltinAgentField::Model,
    });
    state.editor_ui.settings_input.set_text(model);
    assert!(commit_settings_focus(
        state,
        SettingsCommitScope::Operator,
        0
    ));
}
