//! Settings payload round-trips: the MCP CLI toggle arrays across their
//! historical lengths, and the fields a payload may and may not carry.
//!
//! Sibling of `settings_io_tests.rs`, split out at the 800-line cap the
//! collab security-boundary check enforces over this crate.

use super::*;

#[test]
fn eleven_cli_mcp_payload_keeps_its_toggles_and_leaves_zcode_off() {
    let payload: SettingsPayload = serde_json::from_str(
        r#"{"version":1,"mcp_cli_enabled":[true,false,true,false,true,false,true,true,false,true,false]}"#,
    )
    .unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.agent_settings.mcp_cli_enabled = [true; 14];

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.mcp_cli_enabled,
        [
            true, false, true, false, true, false, true, true, false, true, false, false, false,
            false
        ]
    );
}

#[test]
fn thirteen_cli_mcp_payload_keeps_every_toggle_and_leaves_cline_off() {
    // The layout written before Cline was appended: DeepSeek Harness is
    // the last (13th) slot and must keep its toggle.
    let payload: SettingsPayload = serde_json::from_str(
        r#"{"version":1,"mcp_cli_enabled":[true,false,true,false,true,false,true,true,false,true,false,true,true]}"#,
    )
    .unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.agent_settings.mcp_cli_enabled = [true; 14];

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.mcp_cli_enabled,
        [
            true, false, true, false, true, false, true, true, false, true, false, true, true,
            false
        ]
    );
}

#[test]
fn fourteen_cli_mcp_payload_round_trips_the_cline_toggle() {
    let mut flags = vec![false; 14];
    flags[13] = true;
    assert_eq!(migrate_mcp_cli_flags(flags).last(), Some(&true));
}

#[test]
fn eight_cli_mcp_payload_drops_gemini_without_shifting_later_clis() {
    let payload: SettingsPayload = serde_json::from_str(
        r#"{"version":1,"mcp_cli_enabled":[true,false,true,false,true,false,true,true]}"#,
    )
    .unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.mcp_cli_enabled,
        [
            true, false, false, true, false, true, true, false, false, false, false, false, false,
            false
        ]
    );
}

#[test]
fn seven_provider_connected_payload_is_the_current_layout_and_round_trips() {
    // The current layout has seven slots (DeepSeek Harness appended at
    // the tail). A 7-entry file must round-trip VERBATIM — treating it
    // as the legacy Gemini-era layout would silently shift every saved
    // connection on every load.
    let payload: SettingsPayload = serde_json::from_str(
        r#"{"version":1,"connected":[true,false,true,false,true,true,false]}"#,
    )
    .unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.connected,
        [true, false, true, false, true, true, false]
    );
}

#[test]
fn six_provider_connected_payload_keeps_flags_and_leaves_dsh_off() {
    // Files written since the Gemini retirement have six slots; the
    // appended DeepSeek Harness slot starts disconnected.
    let payload: SettingsPayload =
        serde_json::from_str(r#"{"version":1,"connected":[true,false,true,false,true,true]}"#)
            .unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.connected,
        [true, false, true, false, true, true, false]
    );
}

#[test]
fn five_provider_connected_payload_drops_retired_gemini() {
    let payload: SettingsPayload =
        serde_json::from_str(r#"{"version":1,"connected":[true,false,false,false,true]}"#).unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.agent_settings.connected = [false, false, false, false, true, true, true];
    apply_payload(&mut dst, payload);
    assert_eq!(
        dst.editor_ui.agent_settings.connected,
        [true, false, false, false, false, false, false]
    );
}

#[test]
fn legacy_settings_without_connected_field_default_to_disconnected() {
    // A settings.json written before the `connected` field
    // existed must still load — the missing field defaults to
    // all-disconnected rather than failing the parse.
    let legacy = r#"{"version":1,"theme":"dark","locale":"en-US"}"#;
    let payload: SettingsPayload = serde_json::from_str(legacy).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(dst.editor_ui.agent_settings.connected, [false; 7]);
}

#[test]
fn builtin_agents_round_trip_through_payload() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.add_builtin_agent_config(
        "MiniMax",
        "sk-test",
        "MiniMax-M2.7",
        BuiltinAgentKind::Anthropic,
        "https://api.minimaxi.com/anthropic",
    );

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].display_name,
        "MiniMax"
    );
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].api_key,
        "sk-test"
    );
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::MiniMax
    );
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].models,
        ["MiniMax-M2.7"]
    );
}

#[test]
fn builtin_agent_multiple_models_round_trip_with_legacy_first_mirror() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.add_builtin_agent_configs(
        "Private",
        "sk-test",
        ["model-a", "model-b", "model-c"],
        BuiltinAgentKind::OpenAiCompat,
        "https://example.com/v1",
    );

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let raw: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(raw["builtin_agents"][0]["model"], "model-a");
    assert_eq!(
        raw["builtin_agents"][0]["models"],
        serde_json::json!(["model-a", "model-b", "model-c"])
    );

    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].models,
        ["model-a", "model-b", "model-c"]
    );
}

#[test]
fn preferred_agent_team_size_round_trips_and_seeds_chat_on_load() {
    let mut src = EditorState::new();
    src.editor_ui.preferred_agent_team_size = 3;

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.preferred_agent_team_size, 3);
    // `load`'s whole point: tab 0's live chat setting is seeded from the
    // persisted preference, not left at `ChatState::default()`'s 1.
    assert_eq!(
        dst.chat.agent_team_size, 3,
        "tab 0 must be seeded from the persisted preference"
    );
}

#[test]
fn entry_surface_preference_round_trips_and_changes_fingerprint() {
    let mut src = EditorState::new();
    src.editor_ui.entry_surface = op_editor_core::EntrySurface::Canvas;
    let before = fingerprint(&src);
    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    assert!(json.contains("\"entry_surface\":\"canvas\""));

    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(
        dst.editor_ui.entry_surface,
        op_editor_core::EntrySurface::Canvas
    );
    assert_ne!(before, fingerprint(&EditorState::new()));
}

#[test]
fn legacy_settings_without_preferred_agent_team_size_default_to_one() {
    // A settings.json written before this field existed must still load —
    // `SettingsPayload::preferred_agent_team_size` deserializes to `None`
    // (`#[serde(default)]`), which is a no-op on the fresh `EditorState`'s
    // already-default value, landing on the same `1` a defaulted
    // `ChatState` starts with — never a parse failure, never some OTHER
    // fallback number.
    let legacy = r#"{"version":1,"theme":"dark","locale":"en-US"}"#;
    let payload: SettingsPayload = serde_json::from_str(legacy).unwrap();
    assert_eq!(payload.preferred_agent_team_size, None);

    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.preferred_agent_team_size, 1);
    assert_eq!(dst.chat.agent_team_size, 1);
}

#[test]
fn duplicate_builtin_agents_are_deduped_on_load() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-1",
                "display_name": "MINIMAX",
                "kind": "openai-compat",
                "api_key": "sk-test",
                "model": "MiniMax-M2.7",
                "base_url": "https://api.minimaxi.com/v1",
                "enabled": true
            },
            {
                "id": "builtin-2",
                "display_name": "MINIMAX",
                "kind": "openai-compat",
                "api_key": "sk-test",
                "model": "MiniMax-M2.7",
                "base_url": "https://api.minimaxi.com/v1",
                "enabled": true
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].id,
        "builtin-1"
    );
    assert_eq!(dst.editor_ui.agent_settings.next_builtin_agent_id, 2);
}

#[test]
fn best_effort_load_keeps_operator_and_browser_owned_provider_cards_separate() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-1",
                "preset": "openai",
                "display_name": "Operator",
                "kind": "openai-compat",
                "api_key": "same-key",
                "model": "model-a",
                "base_url": "https://api.openai.com/v1",
                "enabled": true
            },
            {
                "id": "web-credential:builtin:browser-1",
                "preset": "openai",
                "display_name": "Browser",
                "kind": "openai-compat",
                "api_key": "same-key",
                "model": "model-b",
                "base_url": "https://api.openai.com/v1",
                "enabled": true
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    let agents = &dst.editor_ui.agent_settings.builtin_agents;
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0].id, "builtin-1");
    assert_eq!(agents[0].models, ["model-a"]);
    assert_eq!(agents[1].id, "web-credential:builtin:browser-1");
    assert_eq!(agents[1].models, ["model-b"]);
}

#[test]
fn duplicate_auto_named_builtin_agents_are_deduped_on_load() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-5",
                "display_name": "Built-in Agent 5",
                "kind": "anthropic",
                "api_key": "sk-test",
                "model": "claude-sonnet-4-5",
                "base_url": "https://api.anthropic.com",
                "enabled": true
            },
            {
                "id": "builtin-6",
                "display_name": "Built-in Agent 6",
                "kind": "anthropic",
                "api_key": "sk-test",
                "model": "claude-sonnet-4-5",
                "base_url": "https://api.anthropic.com",
                "enabled": true
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].display_name,
        "Built-in Agent 5"
    );
    assert_eq!(dst.editor_ui.agent_settings.next_builtin_agent_id, 6);
}

#[test]
fn builtin_agent_payload_without_api_key_loads_as_empty_key() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-2",
                "preset": "bailian-coding",
                "display_name": "百炼CP",
                "kind": "openai-compat",
                "model": "qwen3-coder-plus",
                "base_url": "https://coding.dashscope.aliyuncs.com/v1",
                "enabled": false
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    let agent = &dst.editor_ui.agent_settings.builtin_agents[0];
    assert_eq!(agent.display_name, "百炼CP");
    assert!(agent.api_key.is_empty());
    assert!(!agent.enabled);
}

#[test]
fn acp_agents_round_trip_through_payload() {
    let mut src = EditorState::new();
    let mut env = std::collections::BTreeMap::new();
    env.insert("ACP_TOKEN".into(), "secret".into());
    src.editor_ui.agent_settings.add_acp_agent_config(
        "Design Agent",
        AcpConnectionType::Local,
        "/usr/local/bin/design-agent",
        vec!["--stdio".into()],
        env,
        None,
        true,
    );
    src.editor_ui.agent_settings.add_acp_agent_config(
        "Remote Agent",
        AcpConnectionType::Remote,
        "",
        Vec::new(),
        std::collections::BTreeMap::new(),
        Some("ws://localhost:8100".into()),
        false,
    );

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.acp_agents.len(), 2);
    let local = &dst.editor_ui.agent_settings.acp_agents[0];
    assert_eq!(local.display_name, "Design Agent");
    assert_eq!(local.connection_type, AcpConnectionType::Local);
    assert_eq!(local.command, "/usr/local/bin/design-agent");
    assert!(!local.connected);
    assert_eq!(local.args, vec!["--stdio"]);
    assert_eq!(
        local.env.get("ACP_TOKEN").map(String::as_str),
        Some("secret")
    );
    let remote = &dst.editor_ui.agent_settings.acp_agents[1];
    assert_eq!(remote.connection_type, AcpConnectionType::Remote);
    assert_eq!(remote.url.as_deref(), Some("ws://localhost:8100"));
    assert!(!remote.enabled);
    assert_eq!(dst.editor_ui.agent_settings.next_acp_agent_id, 3);
}

#[test]
fn image_generation_profiles_round_trip_through_payload() {
    let mut src = EditorState::new();
    let first = src.editor_ui.agent_settings.add_image_gen_profile();
    let second = src.editor_ui.agent_settings.add_image_gen_profile();
    let second_profile = &mut src.editor_ui.agent_settings.image_gen_profiles[1];
    second_profile.name = "Gemini Image".into();
    second_profile.provider = ImageGenProvider::Gemini;
    second_profile.api_key = "image-key".into();
    second_profile.model = "gemini-image".into();
    second_profile.base_url = Some("https://images.example/v1".into());
    assert!(src
        .editor_ui
        .agent_settings
        .set_active_image_gen_profile(&second));

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.image_gen_profiles.len(), 2);
    assert_eq!(
        dst.editor_ui
            .agent_settings
            .active_image_gen_profile_id
            .as_deref(),
        Some(second.as_str())
    );
    assert_eq!(dst.editor_ui.agent_settings.image_gen_profiles[0].id, first);
    assert_eq!(
        dst.editor_ui.agent_settings.image_gen_profiles[1].provider,
        ImageGenProvider::Gemini
    );
    assert_eq!(
        dst.editor_ui.agent_settings.image_gen_profiles[1].base_url,
        Some("https://images.example/v1".into())
    );
}

#[test]
fn atlas_image_generation_profile_round_trips_through_payload() {
    let profile = ImageGenProfile {
        id: "igp-atlas".into(),
        name: "Atlas Cloud".into(),
        provider: ImageGenProvider::Atlas,
        api_key: "image-key".into(),
        model: "google/nano-banana-2-lite/text-to-image".into(),
        base_url: None,
        test_status: ImageTestStatus::Idle,
    };

    let payload = crate::settings_payload::image_gen_profile_to_payload(&profile);
    assert_eq!(payload.provider, "atlas");
    let restored = crate::settings_payload::image_gen_profile_from_payload(payload)
        .expect("Atlas profile should deserialize");
    assert_eq!(restored.provider, ImageGenProvider::Atlas);
    assert_eq!(restored.model, profile.model);
}

#[test]
fn openverse_oauth_round_trips_through_payload() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.openverse_client_id = "client-id".into();
    src.editor_ui.agent_settings.openverse_client_secret = "client-secret".into();
    src.editor_ui.agent_settings.openverse_credential_owner = Some("browser".into());

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.openverse_client_id,
        "client-id"
    );
    assert_eq!(
        dst.editor_ui.agent_settings.openverse_client_secret,
        "client-secret"
    );
    assert_eq!(
        dst.editor_ui
            .agent_settings
            .openverse_credential_owner
            .as_deref(),
        Some("browser")
    );
}

#[test]
fn auto_update_preference_round_trips_through_payload() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.auto_update_enabled = false;

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert!(!dst.editor_ui.agent_settings.auto_update_enabled);
}

#[test]
fn explicit_saved_builtin_preset_is_preserved_during_load() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-3",
                "preset": "doubao",
                "display_name": "方舟CP",
                "kind": "anthropic",
                "api_key": "sk-test",
                "model": "ark-code-latest",
                "base_url": "https://ark.cn-beijing.volces.com/api/coding",
                "enabled": true
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::Doubao
    );
}
