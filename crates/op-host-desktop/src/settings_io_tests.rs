//! Exercise recent-file persistence through real Save/Open boundaries in
//! separate processes, without sharing config state with other tests.

use super::*;
use op_editor_core::PenNodeExt;

const MODEL: &str = "builtin:recent-test-glm:glm-5.3-flash";
const CHILD_PHASE: &str = "OPENPENCIL_RECENT_PERSISTENCE_TEST_PHASE";
const CHILD_ROOT: &str = "OPENPENCIL_RECENT_PERSISTENCE_TEST_ROOT";

#[test]
fn desktop_touch_recent_survives_cold_load_without_losing_the_selected_model() {
    if let Ok(phase) = std::env::var(CHILD_PHASE) {
        let root = std::path::PathBuf::from(std::env::var_os(CHILD_ROOT).expect("child root"));
        op_config_store::configure_user_root(&root).expect("isolated config root");
        match phase.as_str() {
            "save" => save_work(&root),
            "load" => cold_open_work(&root),
            _ => panic!("unknown test phase"),
        }
        return;
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "openpencil-recent-cold-load-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("test directory");
    let root = root.canonicalize().unwrap();
    let full_name = concat!(
        module_path!(),
        "::desktop_touch_recent_survives_cold_load_without_losing_the_selected_model"
    );
    let test = full_name.split_once("::").unwrap().1;
    let mut outcomes = Vec::new();
    for phase in ["save", "load"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(CHILD_PHASE, phase)
            .env(CHILD_ROOT, &root)
            .output()
            .expect("isolated recent-files regression");
        let passed = output.status.success()
            && String::from_utf8_lossy(&output.stdout).contains("1 passed;");
        outcomes.push((phase, passed, output));
        if !passed {
            break;
        }
    }
    std::fs::remove_dir_all(&root).expect("remove test-only config and work");
    for (phase, passed, output) in outcomes {
        assert!(
            passed,
            "{phase} subprocess failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

fn save_work(root: &std::path::Path) {
    use op_editor_core::{
        BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState,
    };

    let source = r#"{"version":"1.0.0","children":[
        {"type":"frame","id":"saved-work","width":960,"height":540,"children":[]}
    ]}"#;
    let document = jian_ops_schema::load_str(source).unwrap().value;
    let mut host = WidgetHostNative::new();
    host.replace_editor_state(EditorState::from_document(document));
    let state = host.editor_state_mut();
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(BuiltinAgentConfig {
            id: "recent-test-glm".into(),
            preset: BuiltinAgentPresetKey::GlmCoding,
            display_name: "GLM test".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "test-only-key".into(),
            models: vec!["glm-5.3".into(), "glm-5.3-flash".into()],
            base_url: "https://example.invalid/v1".into(),
            enabled: true,
        });
    state.rebuild_chat_models();
    let model_index = state
        .chat
        .available_models
        .iter()
        .position(|model| model.value == MODEL)
        .unwrap();
    state.select_chat_model(model_index);
    let path = root.join("Saved work.op");
    op_host_services::doc_io::save_to_path(state, &path).expect("save actual work");

    // Deliberately no settings.save() and no event-dispatch wrapper: this is
    // the same standalone hook the asynchronous save completion calls.
    touch_recent(&mut host, &path);
    touch_recent(&mut host, &path);
    let persisted: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("settings.json")).expect("touch_recent must persist settings"),
    )
    .unwrap();
    assert_eq!(persisted["recent_files"].as_array().unwrap().len(), 1);
    assert_eq!(
        persisted["recent_files"][0]["path"],
        path.to_string_lossy().as_ref()
    );
    assert_eq!(persisted["chat_model"], MODEL);
}

fn cold_open_work(root: &std::path::Path) {
    let path = root.join("Saved work.op");
    let mut host = WidgetHostNative::new();
    op_host_services::settings_io::load_checked(host.editor_state_mut())
        .expect("cold-load settings");
    let state = host.editor_state();
    assert_eq!(state.editor_ui.recent_files.len(), 1);
    assert_eq!(state.editor_ui.recent_files[0].path, path.to_string_lossy());
    assert_eq!(state.chat.selected_model_entry().unwrap().value, MODEL);
    assert!(state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id == "recent-test-glm" && agent.api_key == "test-only-key"));

    host.editor_state_mut().editor_ui.home.visible = true;
    let mut current_path = None;
    let outcome = crate::persistence::run_action(
        op_editor_core::FileAction::OpenRecent(0),
        &mut host,
        &mut current_path,
        None,
    );
    assert_eq!(outcome, op_host_services::doc_io::ActionOutcome::Saved);
    assert_eq!(current_path, Some(path));
    assert!(host.workspace_visible());
    assert_eq!(
        host.editor_state().active_children()[0].id_str(),
        "saved-work"
    );
    assert_eq!(
        host.editor_state()
            .chat
            .selected_model_entry()
            .unwrap()
            .value,
        MODEL
    );
}
