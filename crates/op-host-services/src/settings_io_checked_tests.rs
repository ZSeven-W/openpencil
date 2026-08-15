use super::super::*;

fn checked_settings_test_path(case: &str) -> std::path::PathBuf {
    let sequence = SETTINGS_TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "openpencil-settings-checked-{}-{sequence}-{case}",
        std::process::id()
    ))
}

#[test]
fn checked_settings_load_rejects_a_malformed_existing_file_without_mutation() {
    let root = checked_settings_test_path("malformed");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    std::fs::write(&path, br#"{"version":1,"builtin_agents":["#).expect("write malformed settings");
    let mut state = EditorState::new();
    let before = fingerprint(&state);

    let error = load_checked_from_path(&mut state, &path)
        .expect_err("strict web settings load must reject malformed JSON");

    assert!(
        error.to_string().contains("parse settings"),
        "unexpected error: {error}"
    );
    assert_eq!(before, fingerprint(&state));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_rejects_a_schema_invalid_existing_file() {
    let root = checked_settings_test_path("invalid-schema");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    std::fs::write(&path, br#"{"version":"one"}"#).expect("write schema-invalid settings");

    let error = load_checked_from_path(&mut EditorState::new(), &path)
        .expect_err("strict web settings load must reject schema-invalid JSON");

    assert!(
        error.to_string().contains("parse settings"),
        "unexpected error: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_rejects_an_unsupported_settings_version() {
    let root = checked_settings_test_path("future-version");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    std::fs::write(&path, br#"{"version":2}"#).expect("write future settings");

    let error = load_checked_from_path(&mut EditorState::new(), &path)
        .expect_err("web startup must not overwrite an unsupported settings schema");

    assert!(
        error
            .to_string()
            .contains("unsupported settings file version"),
        "unexpected error: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_accepts_canonical_multi_model_agents() {
    let root = checked_settings_test_path("multi-model");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    std::fs::write(
        &path,
        br#"{"version":1,"builtin_agents":[{"id":"builtin-1","preset":"custom","display_name":"Private","kind":"openai-compat","api_key":"key","model":"model-a","models":["model-a","model-b"],"base_url":"https://example.com/v1","enabled":true}]}"#,
    )
    .expect("write multi-model settings");
    let mut state = EditorState::new();

    load_checked_from_path(&mut state, &path).expect("canonical models must load losslessly");

    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].models,
        ["model-a", "model-b"]
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_preserves_an_explicit_preset_for_multi_model_agents() {
    let root = checked_settings_test_path("explicit-preset");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    std::fs::write(
        &path,
        br#"{"version":1,"builtin_agents":[{"id":"builtin-1","preset":"ark-coding","display_name":"Ark","kind":"anthropic","api_key":"key","model":"doubao-seed-2-0-pro-260215","models":["doubao-seed-2-0-pro-260215","ark-code-latest"],"base_url":"https://ark.cn-beijing.volces.com/api/coding","enabled":true}]}"#,
    )
    .expect("write multi-model settings");
    let mut state = EditorState::new();

    load_checked_from_path(&mut state, &path).expect("explicit preset must load losslessly");

    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::ArkCoding
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_migrates_retired_gemini_cli_slots() {
    let root = checked_settings_test_path("retired-gemini-cli");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    std::fs::write(
        &path,
        br#"{"version":1,"connected":[true,false,true,false,true,true,false],"mcp_cli_enabled":[true,false,true,false,true,false,true,true]}"#,
    )
    .expect("write legacy settings");
    let mut state = EditorState::new();

    load_checked_from_path(&mut state, &path).expect("legacy v1 layout should migrate");

    assert_eq!(
        state.editor_ui.agent_settings.connected,
        [true, false, true, false, true, false]
    );
    assert_eq!(
        state.editor_ui.agent_settings.mcp_cli_enabled,
        [true, false, false, true, false, true, true, false, false, false, false, false]
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_rejects_unknown_fields_at_every_settings_nesting_level() {
    let cases = vec![
        (
            "top",
            serde_json::json!({"version":1,"future_setting":true}),
        ),
        (
            "recent",
            serde_json::json!({
                "version":1,
                "recent_files":[{"path":"/tmp/design.op","modified_at":1,"future":true}]
            }),
        ),
        (
            "builtin",
            serde_json::json!({
                "version":1,
                "builtin_agents":[{
                    "id":"builtin-1","preset":"custom","display_name":"Built-in",
                    "kind":"openai-compat","api_key":"key","model":"model",
                    "base_url":"https://api.openai.com/v1","enabled":true,"future":true
                }]
            }),
        ),
        (
            "acp",
            serde_json::json!({
                "version":1,
                "acp_agents":[{
                    "id":"acp-1","display_name":"Native ACP","connection_type":"local",
                    "command":"agent","args":[],"env":{"TOKEN":"secret"},"url":null,
                    "enabled":true,"future":true
                }]
            }),
        ),
        (
            "image",
            serde_json::json!({
                "version":1,
                "image_gen_profiles":[{
                    "id":"igp-1","name":"Image","provider":"openai","api_key":"key",
                    "model":"gpt-image-1","base_url":null,"future":true
                }]
            }),
        ),
        (
            "openverse",
            serde_json::json!({
                "version":1,
                "openverse_oauth":{"client_id":"client","client_secret":"secret","future":true}
            }),
        ),
    ];

    for (case, value) in cases {
        // The native best-effort schema remains additive-compatible.
        serde_json::from_value::<SettingsPayload>(value.clone())
            .expect("native settings parser keeps ignoring additive fields");

        let root = checked_settings_test_path(case);
        let path = root.join("settings.json");
        std::fs::create_dir_all(&root).expect("create settings test directory");
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode settings"))
            .expect("write settings");
        let mut state = EditorState::new();
        let before = fingerprint(&state);

        let error = load_checked_from_path(&mut state, &path)
            .expect_err("strict web load must reject unknown same-version fields");

        assert!(
            error.to_string().contains("unknown settings field"),
            "case={case}, {error}"
        );
        assert_eq!(before, fingerprint(&state), "case={case}");
        let _ = std::fs::remove_dir_all(root);
    }
}

#[test]
fn checked_settings_load_rejects_values_that_would_be_silently_normalized() {
    let too_many_recent = (0..=RECENT_FILE_CAP)
        .map(|index| serde_json::json!({"path":format!("/tmp/{index}.op"),"modified_at":index}))
        .collect::<Vec<_>>();
    let cases = vec![
        ("theme", serde_json::json!({"version":1,"theme":"sepia"})),
        (
            "locale-unknown",
            serde_json::json!({"version":1,"locale":"xx"}),
        ),
        (
            "locale-alias",
            serde_json::json!({"version":1,"locale":"en"}),
        ),
        ("port", serde_json::json!({"version":1,"mcp_port":80})),
        (
            "recent-cap",
            serde_json::json!({"version":1,"recent_files":too_many_recent}),
        ),
        (
            "active-image",
            serde_json::json!({
                "version":1,
                "image_gen_profiles":[{
                    "id":"igp-1","name":"Image","provider":"openai","api_key":"key",
                    "model":"gpt-image-1","base_url":null
                }],
                "active_image_gen_profile_id":"igp-missing"
            }),
        ),
        (
            "active-image-null",
            serde_json::json!({
                "version":1,
                "image_gen_profiles":[{
                    "id":"igp-1","name":"Image","provider":"openai","api_key":"key",
                    "model":"gpt-image-1","base_url":null
                }],
                "active_image_gen_profile_id":null
            }),
        ),
        (
            "builtin-models-normalized",
            serde_json::json!({
                "version":1,
                "builtin_agents":[{
                    "id":"builtin-1","preset":"custom","display_name":"Built-in",
                    "kind":"openai-compat","api_key":"key","model":"model-a",
                    "models":["model-a"," model-b ","model-a"],
                    "base_url":"https://example.com/v1","enabled":true
                }]
            }),
        ),
        (
            "builtin-models-conflict",
            serde_json::json!({
                "version":1,
                "builtin_agents":[{
                    "id":"builtin-1","preset":"custom","display_name":"Built-in",
                    "kind":"openai-compat","api_key":"key","model":"legacy-other",
                    "models":["model-a","model-b"],
                    "base_url":"https://example.com/v1","enabled":true
                }]
            }),
        ),
        (
            "builtin-preset",
            serde_json::json!({
                "version":1,
                "builtin_agents":[{
                    "id":"builtin-1","preset":"future-preset","display_name":"Built-in",
                    "kind":"openai-compat","api_key":"key","model":"model",
                    "base_url":"https://api.openai.com/v1","enabled":true
                }]
            }),
        ),
        (
            "builtin-missing-preset",
            serde_json::json!({
                "version":1,
                "builtin_agents":[{
                    "id":"builtin-1","display_name":"Built-in",
                    "kind":"openai-compat","api_key":"key","model":"model",
                    "base_url":"https://api.openai.com/v1","enabled":true
                }]
            }),
        ),
        (
            "builtin-kind-alias-openai",
            serde_json::json!({
                "version":1,
                "builtin_agents":[{
                    "id":"builtin-1","preset":"custom","display_name":"Built-in",
                    "kind":"openai","api_key":"key","model":"model",
                    "base_url":"https://api.openai.com/v1","enabled":true
                }]
            }),
        ),
        (
            "builtin-kind-alias-underscore",
            serde_json::json!({
                "version":1,
                "builtin_agents":[{
                    "id":"builtin-1","preset":"custom","display_name":"Built-in",
                    "kind":"openai_compat","api_key":"key","model":"model",
                    "base_url":"https://api.openai.com/v1","enabled":true
                }]
            }),
        ),
        (
            "openverse-whitespace",
            serde_json::json!({
                "version":1,
                "openverse_oauth":{"client_id":" client ","client_secret":" secret "}
            }),
        ),
        (
            "openverse-empty",
            serde_json::json!({
                "version":1,
                "openverse_oauth":{"client_id":"","client_secret":""}
            }),
        ),
    ];

    for (case, value) in cases {
        let root = checked_settings_test_path(case);
        let path = root.join("settings.json");
        std::fs::create_dir_all(&root).expect("create settings test directory");
        std::fs::write(&path, serde_json::to_vec(&value).expect("encode settings"))
            .expect("write settings");
        let mut state = EditorState::new();
        let before = fingerprint(&state);

        let error = load_checked_from_path(&mut state, &path)
            .expect_err("strict web load must reject a lossy settings normalization");

        assert!(
            error.to_string().contains("losslessly"),
            "case={case}, {error}"
        );
        assert_eq!(before, fingerprint(&state), "case={case}");
        let _ = std::fs::remove_dir_all(root);
    }
}

#[test]
fn checked_settings_load_rejects_semantically_invalid_credential_entries() {
    let cases = [
        (
            "builtin-kind",
            r#"{
                "version":1,
                "builtin_agents":[{
                    "id":"web-credential:builtin:future",
                    "display_name":"Future Builtin",
                    "kind":"future-provider",
                    "api_key":"browser-secret",
                    "model":"future-model",
                    "base_url":"https://future.example/v1",
                    "enabled":true
                }]
            }"#,
        ),
        (
            "acp-connection",
            r#"{
                "version":1,
                "acp_agents":[{
                    "id":"acp-future",
                    "display_name":"Future ACP",
                    "connection_type":"future-transport",
                    "command":"/opt/browser-agent",
                    "enabled":true
                }]
            }"#,
        ),
        (
            "image-provider",
            r#"{
                "version":1,
                "image_gen_profiles":[{
                    "id":"web-credential:image:future",
                    "name":"Future Image",
                    "provider":"future-provider",
                    "api_key":"browser-image-secret",
                    "model":"future-image"
                }]
            }"#,
        ),
    ];

    for (case, body) in cases {
        let root = checked_settings_test_path(case);
        let path = root.join("settings.json");
        std::fs::create_dir_all(&root).expect("create settings test directory");
        std::fs::write(&path, body).expect("write semantically invalid settings");
        let mut state = EditorState::new();
        let before = fingerprint(&state);

        let error = load_checked_from_path(&mut state, &path)
            .expect_err("web startup must not silently drop an unknown credential entry");

        assert!(
            error
                .to_string()
                .contains("unsupported settings credential entry"),
            "case={case}, error={error}"
        );
        assert_eq!(before, fingerprint(&state), "case={case}");
        let _ = std::fs::remove_dir_all(root);
    }
}

#[test]
fn checked_settings_load_preserves_app_generated_operator_and_browser_duplicates() {
    let root = checked_settings_test_path("valid-duplicate");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    let mut persisted = EditorState::new();
    persisted.editor_ui.agent_settings.add_builtin_agent_config(
        "Shared",
        "same-key",
        "same-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    let mut browser = persisted.editor_ui.agent_settings.builtin_agents[0].clone();
    browser.id = "web-credential:builtin:shared".into();
    persisted
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(browser);
    let body = serde_json::to_vec(&to_payload(&persisted)).expect("encode app settings");
    std::fs::write(&path, body).expect("write app-generated settings");
    let mut loaded = EditorState::new();

    load_checked_from_path(&mut loaded, &path)
        .expect("checked web load accepts app-generated duplicate configurations");

    assert_eq!(loaded.editor_ui.agent_settings.builtin_agents.len(), 2);
    assert!(loaded
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id == "web-credential:builtin:shared"));
    assert!(crate::web_credentials::remove_browser_owned_credentials(
        &mut loaded
    ));
    assert_eq!(loaded.editor_ui.agent_settings.builtin_agents.len(), 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_rejects_duplicate_builtin_provider_ids() {
    let root = checked_settings_test_path("duplicate-built-in-id");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    let mut persisted = EditorState::new();
    persisted.editor_ui.agent_settings.add_builtin_agent_config(
        "First",
        "sk-first",
        "first-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    persisted.editor_ui.agent_settings.add_builtin_agent_config(
        "Second",
        "sk-second",
        "second-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://openrouter.ai/api/v1",
    );
    let duplicate_id = persisted.editor_ui.agent_settings.builtin_agents[0]
        .id
        .clone();
    persisted.editor_ui.agent_settings.builtin_agents[1].id = duplicate_id;
    let body = serde_json::to_vec(&to_payload(&persisted)).expect("encode settings");
    std::fs::write(&path, body).expect("write settings");
    let mut loaded = EditorState::new();
    let before = fingerprint(&loaded);

    let error = load_checked_from_path(&mut loaded, &path)
        .expect_err("duplicate provider ids are not lossless");

    assert!(error.to_string().contains("losslessly"), "{error}");
    assert_eq!(fingerprint(&loaded), before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_rejects_noncanonical_builtin_provider_ids() {
    let root = checked_settings_test_path("noncanonical-built-in-id");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    let mut persisted = EditorState::new();
    persisted.editor_ui.agent_settings.add_builtin_agent_config(
        "Provider",
        "sk-provider",
        "model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    persisted.editor_ui.agent_settings.builtin_agents[0].id = " provider-id ".into();
    let body = serde_json::to_vec(&to_payload(&persisted)).expect("encode settings");
    std::fs::write(&path, body).expect("write settings");
    let mut loaded = EditorState::new();
    let before = fingerprint(&loaded);

    let error = load_checked_from_path(&mut loaded, &path)
        .expect_err("provider ids with surrounding whitespace are not lossless");

    assert!(error.to_string().contains("losslessly"), "{error}");
    assert_eq!(fingerprint(&loaded), before);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_keeps_the_existing_native_acp_schema_unchanged() {
    let root = checked_settings_test_path("native-acp");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&root).expect("create settings test directory");
    let mut persisted = EditorState::new();
    persisted.editor_ui.agent_settings.add_acp_agent_config(
        "Native ACP",
        AcpConnectionType::Local,
        "/usr/local/bin/native-agent",
        vec!["--stdio".into()],
        std::collections::BTreeMap::from([("TOKEN".into(), "native-secret".into())]),
        None,
        true,
    );
    let body = serde_json::to_vec(&to_payload(&persisted)).expect("encode native settings");
    let encoded: serde_json::Value = serde_json::from_slice(&body).expect("decode native settings");
    assert!(encoded["acp_agents"][0].get("credential_owner").is_none());
    std::fs::write(&path, body).expect("write native settings");
    let mut loaded = EditorState::new();

    load_checked_from_path(&mut loaded, &path).expect("native ACP settings remain valid");

    let agent = &loaded.editor_ui.agent_settings.acp_agents[0];
    assert_eq!(agent.command, "/usr/local/bin/native-agent");
    assert_eq!(agent.args, vec!["--stdio"]);
    assert_eq!(
        agent.env.get("TOKEN").map(String::as_str),
        Some("native-secret")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_rejects_an_unreadable_existing_path_without_mutation() {
    let root = checked_settings_test_path("unreadable");
    let path = root.join("settings.json");
    std::fs::create_dir_all(&path).expect("create directory at settings file path");
    let mut state = EditorState::new();
    let before = fingerprint(&state);

    let error = load_checked_from_path(&mut state, &path)
        .expect_err("strict web settings load must reject an unreadable settings path");

    assert!(
        error.to_string().contains("read settings"),
        "unexpected error: {error}"
    );
    assert_eq!(before, fingerprint(&state));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_settings_load_allows_a_missing_file() {
    let root = checked_settings_test_path("missing");
    let path = root.join("settings.json");
    let mut state = EditorState::new();

    load_checked_from_path(&mut state, &path).expect("missing settings is a first-run state");
}
