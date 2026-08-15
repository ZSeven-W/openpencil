use super::{
    apply_json, parse_transient_builtin, parse_transient_builtin_for_discovery,
    validate_web_provider_base_url_with_allowlist, MAX_CREDENTIAL_BODY_BYTES,
};
use op_editor_core::{BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState};

struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn unset(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

const VALID_BODY: &str = r#"{
  "version":2,
  "builtin_agents":[{
    "id":"builtin-web-1","preset":"custom","display_name":"Private Model",
    "kind":"openai-compat","api_key":"sk-browser-only","model":"private-model",
    "base_url":"https://api.openai.com/v1","enabled":true
  }],
  "image_gen_profiles":[{
    "id":"igp-web-1","name":"Image","provider":"openai",
    "api_key":"image-browser-only","model":"gpt-image-1","base_url":null
  }],
  "active_image_gen_profile_id":"igp-web-1",
  "openverse_oauth":{"client_id":"client","client_secret":"openverse-browser-only"}
}"#;

fn state_with_operator_agent() -> EditorState {
    let mut state = EditorState::new();
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(BuiltinAgentConfig {
            id: "operator-agent".into(),
            preset: BuiltinAgentPresetKey::Custom,
            display_name: "Operator".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "operator-secret".into(),
            models: vec!["operator-model".into()],
            base_url: "https://operator.example/v1".into(),
            enabled: true,
        });
    state
}

#[test]
fn discovery_accepts_a_credential_before_its_first_model_is_selected() {
    let credential = serde_json::json!({
        "id": "builtin-web-1",
        "preset": "custom",
        "display_name": "Private Model",
        "kind": "openai-compat",
        "api_key": "sk-browser-only",
        "model": "",
        "base_url": "https://api.openai.com/v1",
        "enabled": true,
    });

    assert!(parse_transient_builtin(&credential).is_none());
    assert!(parse_transient_builtin_for_discovery(&credential).is_some());
}

#[test]
fn transient_credential_preserves_an_explicit_preset_across_model_order() {
    let credential = serde_json::json!({
        "id": "builtin-web-1",
        "preset": "ark-coding",
        "display_name": "Ark Coding",
        "kind": "anthropic",
        "api_key": "sk-browser-only",
        "model": "doubao-seed-2-0-pro-260215",
        "models": ["doubao-seed-2-0-pro-260215", "ark-code-latest"],
        "base_url": "https://ark.cn-beijing.volces.com/api/coding",
        "enabled": true,
    });

    let agent = parse_transient_builtin(&credential).expect("credential parses");

    assert_eq!(agent.preset, BuiltinAgentPresetKey::ArkCoding);
}

#[test]
fn credential_snapshot_accepts_multiple_saved_models() {
    let mut body: serde_json::Value = serde_json::from_str(VALID_BODY).expect("fixture JSON");
    body["builtin_agents"][0]["models"] =
        serde_json::json!(["private-model", "private-model-fast"]);
    let mut state = EditorState::new();

    apply_json(&mut state, &body.to_string()).expect("multi-model snapshot merges");

    let agent = state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.api_key == "sk-browser-only")
        .expect("browser credential");
    assert_eq!(agent.models, ["private-model", "private-model-fast"]);
}

#[test]
fn merge_preserves_unrelated_operator_credentials() {
    let mut state = state_with_operator_agent();

    apply_json(&mut state, VALID_BODY).expect("valid credential merge");

    let settings = &state.editor_ui.agent_settings;
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id == "operator-agent" && agent.api_key == "operator-secret"));
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id.ends_with(":builtin:builtin-web-1")
            && agent.api_key == "sk-browser-only"));
    assert!(settings
        .image_gen_profiles
        .iter()
        .any(|profile| profile.id.ends_with(":image:igp-web-1")
            && profile.api_key == "image-browser-only"));
    assert_eq!(
        settings
            .image_gen_profiles
            .iter()
            .filter(|profile| profile.id.starts_with("web-credential:image:"))
            .count(),
        1
    );
    assert_eq!(settings.openverse_client_secret, "openverse-browser-only");
    assert_eq!(
        settings.openverse_credential_owner.as_deref(),
        Some("browser")
    );
}

#[test]
fn credential_merge_keeps_document_storage_in_place() {
    let mut state = EditorState::starter();
    let document_children = state.doc.children.as_ptr();

    apply_json(&mut state, VALID_BODY).expect("valid credential merge");

    assert_eq!(state.doc.children.as_ptr(), document_children);
}

#[test]
fn realistic_id_collisions_do_not_overwrite_operator_credentials() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Operator",
        "operator-secret",
        "operator-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://operator.example/v1",
    );
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.image_gen_profiles[0].api_key = "operator-image".into();
    let body = VALID_BODY
        .replace("builtin-web-1", "builtin-1")
        .replace("igp-web-1", "igp-1");

    apply_json(&mut state, &body).expect("browser snapshot merges");

    let settings = &state.editor_ui.agent_settings;
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id == "builtin-1" && agent.api_key == "operator-secret"));
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id != "builtin-1" && agent.api_key == "sk-browser-only"));
    assert!(settings
        .image_gen_profiles
        .iter()
        .any(|profile| profile.id == "igp-1" && profile.api_key == "operator-image"));
    assert!(settings
        .image_gen_profiles
        .iter()
        .any(|profile| profile.id != "igp-1" && profile.api_key == "image-browser-only"));
}

#[test]
fn browser_snapshot_deletes_only_browser_owned_missing_entries() {
    let mut state = state_with_operator_agent();
    apply_json(&mut state, VALID_BODY).expect("initial browser snapshot");
    let empty = r#"{
      "version":2,
      "builtin_agents":[],
      "image_gen_profiles":[],
      "active_image_gen_profile_id":null,
      "openverse_oauth":null
    }"#;

    apply_json(&mut state, empty).expect("empty browser snapshot");

    let settings = &state.editor_ui.agent_settings;
    assert_eq!(settings.builtin_agents.len(), 1);
    assert_eq!(settings.builtin_agents[0].id, "operator-agent");
    assert!(settings.image_gen_profiles.is_empty());
    assert!(settings.openverse_client_secret.is_empty());
}

#[test]
fn operator_openverse_credentials_win_without_blocking_other_browser_credentials() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.openverse_client_id = "operator-client".into();
    state.editor_ui.agent_settings.openverse_client_secret = "operator-secret".into();

    apply_json(&mut state, VALID_BODY).expect("non-Openverse credentials still merge");

    let settings = &state.editor_ui.agent_settings;
    assert_eq!(settings.openverse_client_id, "operator-client");
    assert_eq!(settings.openverse_client_secret, "operator-secret");
    assert_eq!(settings.openverse_credential_owner, None);
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.api_key == "sk-browser-only"));
    assert!(settings
        .image_gen_profiles
        .iter()
        .any(|profile| profile.api_key == "image-browser-only"));
}

#[test]
fn newer_browser_snapshot_replaces_the_previous_browser_snapshot() {
    let mut state = state_with_operator_agent();
    apply_json(&mut state, VALID_BODY).expect("first browser snapshot");
    let second = VALID_BODY
        .replace("builtin-web-1", "builtin-web-2")
        .replace("igp-web-1", "igp-web-2")
        .replace("sk-browser-only", "sk-second-browser")
        .replace("image-browser-only", "image-second-browser")
        .replace("openverse-browser-only", "openverse-second-browser");

    apply_json(&mut state, &second).expect("replacement browser snapshot");

    let settings = &state.editor_ui.agent_settings;
    assert_eq!(settings.builtin_agents.len(), 2);
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.api_key == "operator-secret"));
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.api_key == "sk-second-browser"));
    assert!(!settings
        .builtin_agents
        .iter()
        .any(|agent| agent.api_key == "sk-browser-only"));
    assert_eq!(settings.image_gen_profiles.len(), 1);
    assert_eq!(
        settings.image_gen_profiles[0].api_key,
        "image-second-browser"
    );
    assert_eq!(settings.openverse_client_secret, "openverse-second-browser");
}

#[test]
fn aggregate_entry_limit_is_atomic() {
    let mut state = EditorState::new();
    for index in 0..super::MAX_TOTAL_ENTRIES {
        state
            .editor_ui
            .agent_settings
            .builtin_agents
            .push(BuiltinAgentConfig {
                id: format!("operator-{index}"),
                preset: BuiltinAgentPresetKey::Custom,
                display_name: format!("Operator {index}"),
                kind: BuiltinAgentKind::OpenAiCompat,
                api_key: format!("operator-secret-{index}"),
                models: vec!["operator-model".into()],
                base_url: "https://operator.example/v1".into(),
                enabled: true,
            });
    }
    let before = crate::settings_io::fingerprint(&state);

    assert!(apply_json(&mut state, VALID_BODY).is_err());
    assert_eq!(before, crate::settings_io::fingerprint(&state));
}

#[test]
fn invalid_late_entry_does_not_partially_mutate_state() {
    let mut state = state_with_operator_agent();
    let before = crate::settings_io::fingerprint(&state);
    let body = VALID_BODY.replace(r#""provider":"openai""#, r#""provider":"unknown""#);

    assert!(apply_json(&mut state, &body).is_err());
    assert_eq!(before, crate::settings_io::fingerprint(&state));
}

#[test]
fn oversized_payload_is_rejected_without_mutation() {
    let mut state = state_with_operator_agent();
    let before = crate::settings_io::fingerprint(&state);
    let body = "x".repeat(MAX_CREDENTIAL_BODY_BYTES + 1);

    assert!(apply_json(&mut state, &body).is_err());
    assert_eq!(before, crate::settings_io::fingerprint(&state));
}

#[test]
fn credential_snapshot_rejects_reserved_provider_endpoints_atomically() {
    let endpoints = [
        "https://user:password@api.openai.com/v1",
        "http://127.0.0.1:8080/v1",
        "http://10.0.0.7/v1",
        "http://169.254.169.254/latest/meta-data",
        "http://168.63.129.16/metadata/instance",
        "http://[::1]:8080/v1",
        "http://[fd00:ec2::254]/latest/meta-data",
        "http://metadata.google.internal/computeMetadata/v1",
        "https://provider.example.test/v1",
    ];

    for endpoint in endpoints {
        let mut state = state_with_operator_agent();
        let before = crate::settings_io::fingerprint(&state);
        let body = VALID_BODY.replace("https://api.openai.com/v1", endpoint);

        assert!(
            apply_json(&mut state, &body).is_err(),
            "reserved endpoint was accepted: {endpoint}"
        );
        assert_eq!(before, crate::settings_io::fingerprint(&state));
    }
}

#[test]
fn credential_snapshot_accepts_a_public_https_origin_without_allowlist() {
    let _guard = EnvVarGuard::unset(super::WEB_AI_ENDPOINT_ALLOWLIST_ENV);
    let body = VALID_BODY.replace(
        "https://api.openai.com/v1",
        "https://custom-gateway.example/v1",
    );
    let mut state = state_with_operator_agent();

    apply_json(&mut state, &body).expect("public HTTPS origin is accepted without allowlist");
    assert!(state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .any(|agent| agent.base_url == "https://custom-gateway.example/v1"));
}

#[test]
fn explicit_endpoint_allowlist_can_opt_into_an_internal_origin() {
    assert!(validate_web_provider_base_url_with_allowlist(
        "http://127.0.0.1:11434/v1",
        Some("https://inference.example.com,http://127.0.0.1:11434"),
    )
    .is_ok());
    assert!(validate_web_provider_base_url_with_allowlist(
        "http://127.0.0.1:11435/v1",
        Some("http://127.0.0.1:11434"),
    )
    .is_err());
}

#[test]
fn browser_credential_endpoint_rejects_every_acp_field_without_mutation() {
    for acp_agents in [
        serde_json::json!([]),
        serde_json::json!([{"id":"acp-web-7"}]),
    ] {
        let mut state = state_with_operator_agent();
        state.editor_ui.agent_settings.add_acp_agent_config(
            "Operator ACP",
            op_editor_core::AcpConnectionType::Local,
            "/usr/bin/operator-agent",
            vec!["--stdio".into()],
            std::collections::BTreeMap::new(),
            None,
            true,
        );
        let before = crate::settings_io::fingerprint(&state);
        let mut payload: serde_json::Value =
            serde_json::from_str(VALID_BODY).expect("valid fixture");
        payload["acp_agents"] = acp_agents;

        apply_json(&mut state, &payload.to_string())
            .expect_err("ACP is not part of the browser credential schema");

        assert_eq!(before, crate::settings_io::fingerprint(&state));
    }
}
