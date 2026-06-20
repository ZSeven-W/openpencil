//! Browser-local app preferences for the Rust web host.
//!
//! Documents still save through the normal file/daemon paths. This module
//! persists app-level preferences that native stores in `settings.json`:
//! theme, locale, recent files, MCP toggles, agent configs, and image-gen
//! profiles.

use op_editor_core::editor_ui_state::{RecentFile, RECENT_FILE_CAP};
use op_editor_core::{
    AcpAgentConfig, AcpConnectionType, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey,
    EditorState, ImageGenProfile, ImageGenProvider, Locale, ThemeMode,
};
use serde::{Deserialize, Serialize};

const SETTINGS_VERSION: u32 = 1;
const STORAGE_KEY: &str = "openpencil-rust-web-settings";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RecentFilePayload {
    path: String,
    modified_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BuiltinAgentPayload {
    id: String,
    #[serde(default)]
    preset: Option<String>,
    display_name: String,
    kind: String,
    #[serde(default)]
    api_key: String,
    model: String,
    base_url: String,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AcpAgentPayload {
    id: String,
    display_name: String,
    connection_type: String,
    #[serde(default)]
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    url: Option<String>,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ImageGenProfilePayload {
    id: String,
    name: String,
    provider: String,
    api_key: String,
    model: String,
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenverseOAuthPayload {
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    theme: ThemeMode,
    locale: Locale,
    port: u16,
    cli: [bool; 6],
    images_adv: bool,
    openverse_client_id: String,
    openverse_client_secret: String,
    auto_update_enabled: bool,
    experimental_features_enabled: bool,
    connected: [bool; 5],
    builtin_agents: Vec<BuiltinAgentConfig>,
    acp_agents: Vec<AcpAgentConfig>,
    image_gen_profiles: Vec<ImageGenProfile>,
    active_image_gen_profile_id: Option<String>,
    recent_files: Vec<RecentFile>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SettingsPayload {
    version: u32,
    #[serde(default)]
    theme: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    mcp_port: Option<u16>,
    #[serde(default)]
    mcp_cli_enabled: Option<[bool; 6]>,
    #[serde(default)]
    images_advanced_open: Option<bool>,
    #[serde(default)]
    openverse_oauth: Option<OpenverseOAuthPayload>,
    #[serde(default)]
    auto_update_enabled: Option<bool>,
    #[serde(default)]
    experimental_features_enabled: Option<bool>,
    #[serde(default)]
    connected: Option<[bool; 5]>,
    #[serde(default)]
    builtin_agents: Option<Vec<BuiltinAgentPayload>>,
    #[serde(default)]
    acp_agents: Option<Vec<AcpAgentPayload>>,
    #[serde(default)]
    image_gen_profiles: Option<Vec<ImageGenProfilePayload>>,
    #[serde(default)]
    active_image_gen_profile_id: Option<String>,
    #[serde(default)]
    recent_files: Option<Vec<RecentFilePayload>>,
}

pub(crate) fn fingerprint(state: &EditorState) -> Fingerprint {
    let eui = &state.editor_ui;
    Fingerprint {
        theme: eui.theme_mode,
        locale: eui.locale,
        port: eui.agent_settings.mcp_server.port,
        cli: eui.agent_settings.mcp_cli_enabled,
        images_adv: eui.agent_settings.images_advanced_open,
        openverse_client_id: eui.agent_settings.openverse_client_id.clone(),
        openverse_client_secret: eui.agent_settings.openverse_client_secret.clone(),
        auto_update_enabled: eui.agent_settings.auto_update_enabled,
        experimental_features_enabled: eui.agent_settings.experimental_features_enabled,
        connected: eui.agent_settings.connected,
        builtin_agents: eui.agent_settings.builtin_agents.clone(),
        acp_agents: eui.agent_settings.acp_agents.clone(),
        image_gen_profiles: eui.agent_settings.image_gen_profiles.clone(),
        active_image_gen_profile_id: eui.agent_settings.active_image_gen_profile_id.clone(),
        recent_files: eui.recent_files.clone(),
    }
}

pub(crate) fn load_into(state: &mut EditorState) {
    let Some(raw) = storage_get(STORAGE_KEY) else {
        return;
    };
    let _ = apply_json(state, &raw);
}

pub(crate) fn save_if_changed(state: &EditorState, before: &mut Fingerprint) {
    let next = fingerprint(state);
    if next != *before {
        save(state);
        *before = next;
    }
}

fn apply_json(state: &mut EditorState, raw: &str) -> Result<(), String> {
    let payload: SettingsPayload = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    apply_payload(state, payload);
    Ok(())
}

fn save(state: &EditorState) {
    let Ok(json) = serde_json::to_string(&to_payload(state)) else {
        return;
    };
    storage_set(STORAGE_KEY, &json);
}

fn to_payload(state: &EditorState) -> SettingsPayload {
    let eui = &state.editor_ui;
    SettingsPayload {
        version: SETTINGS_VERSION,
        theme: Some(theme_to_str(eui.theme_mode).into()),
        locale: Some(locale_to_str(eui.locale).into()),
        mcp_port: Some(eui.agent_settings.mcp_server.port),
        mcp_cli_enabled: Some(eui.agent_settings.mcp_cli_enabled),
        images_advanced_open: Some(eui.agent_settings.images_advanced_open),
        openverse_oauth: openverse_oauth_to_payload(&eui.agent_settings),
        auto_update_enabled: Some(eui.agent_settings.auto_update_enabled),
        experimental_features_enabled: Some(eui.agent_settings.experimental_features_enabled),
        connected: Some(eui.agent_settings.connected),
        builtin_agents: Some(
            eui.agent_settings
                .builtin_agents
                .iter()
                .map(builtin_agent_to_payload)
                .collect(),
        ),
        acp_agents: Some(
            eui.agent_settings
                .acp_agents
                .iter()
                .map(acp_agent_to_payload)
                .collect(),
        ),
        image_gen_profiles: Some(
            eui.agent_settings
                .image_gen_profiles
                .iter()
                .map(image_gen_profile_to_payload)
                .collect(),
        ),
        active_image_gen_profile_id: eui.agent_settings.active_image_gen_profile_id.clone(),
        recent_files: Some(
            eui.recent_files
                .iter()
                .map(|r| RecentFilePayload {
                    path: r.path.clone(),
                    modified_at: r.modified_at,
                })
                .collect(),
        ),
    }
}

fn apply_payload(state: &mut EditorState, payload: SettingsPayload) {
    if payload.version != SETTINGS_VERSION {
        return;
    }
    let eui = &mut state.editor_ui;
    if let Some(theme) = payload.theme.as_deref() {
        eui.theme_mode = str_to_theme(theme);
    }
    if let Some(locale) = payload.locale.as_deref().and_then(str_to_locale) {
        eui.locale = locale;
    }
    if let Some(port) = payload.mcp_port {
        eui.agent_settings.mcp_server.port = port.max(1024);
    }
    if let Some(flags) = payload.mcp_cli_enabled {
        eui.agent_settings.mcp_cli_enabled = flags;
    }
    if let Some(open) = payload.images_advanced_open {
        eui.agent_settings.images_advanced_open = open;
    }
    if let Some(oauth) = payload.openverse_oauth {
        eui.agent_settings.openverse_client_id = oauth.client_id;
        eui.agent_settings.openverse_client_secret = oauth.client_secret;
    }
    if let Some(enabled) = payload.auto_update_enabled {
        eui.agent_settings.auto_update_enabled = enabled;
    }
    if let Some(enabled) = payload.experimental_features_enabled {
        eui.agent_settings.experimental_features_enabled = enabled;
    }
    if let Some(connected) = payload.connected {
        eui.agent_settings.connected = connected;
    }
    if let Some(agents) = payload.builtin_agents {
        let agents = agents
            .into_iter()
            .filter_map(builtin_agent_from_payload)
            .collect();
        eui.agent_settings.builtin_agents = dedupe_builtin_agents(agents);
        eui.agent_settings.next_builtin_agent_id =
            next_builtin_agent_id(&eui.agent_settings.builtin_agents);
    }
    if let Some(agents) = payload.acp_agents {
        eui.agent_settings.acp_agents = agents
            .into_iter()
            .filter_map(acp_agent_from_payload)
            .collect();
        eui.agent_settings.next_acp_agent_id = next_acp_agent_id(&eui.agent_settings.acp_agents);
    }
    if let Some(profiles) = payload.image_gen_profiles {
        eui.agent_settings.image_gen_profiles = profiles
            .into_iter()
            .filter_map(image_gen_profile_from_payload)
            .collect();
        eui.agent_settings.next_image_gen_profile_id =
            next_image_gen_profile_id(&eui.agent_settings.image_gen_profiles);
    }
    if let Some(active) = payload.active_image_gen_profile_id {
        if eui
            .agent_settings
            .image_gen_profiles
            .iter()
            .any(|profile| profile.id == active)
        {
            eui.agent_settings.active_image_gen_profile_id = Some(active);
        } else {
            eui.agent_settings.active_image_gen_profile_id = eui
                .agent_settings
                .image_gen_profiles
                .first()
                .map(|profile| profile.id.clone());
        }
    }
    if eui.agent_settings.active_image_gen_profile_id.is_none() {
        eui.agent_settings.active_image_gen_profile_id = eui
            .agent_settings
            .image_gen_profiles
            .first()
            .map(|profile| profile.id.clone());
    }
    if let Some(recent) = payload.recent_files {
        eui.recent_files = recent
            .into_iter()
            .take(RECENT_FILE_CAP)
            .map(|r| RecentFile {
                path: r.path,
                modified_at: r.modified_at,
            })
            .collect();
    }
    state.rebuild_chat_models();
}

fn builtin_agent_to_payload(agent: &BuiltinAgentConfig) -> BuiltinAgentPayload {
    BuiltinAgentPayload {
        id: agent.id.clone(),
        preset: Some(agent.preset.as_str().into()),
        display_name: agent.display_name.clone(),
        kind: match agent.kind {
            BuiltinAgentKind::Anthropic => "anthropic",
            BuiltinAgentKind::OpenAiCompat => "openai-compat",
        }
        .into(),
        api_key: agent.api_key.clone(),
        model: agent.model.clone(),
        base_url: agent.base_url.clone(),
        enabled: agent.enabled,
    }
}

fn builtin_agent_from_payload(payload: BuiltinAgentPayload) -> Option<BuiltinAgentConfig> {
    let kind = match payload.kind.as_str() {
        "anthropic" => BuiltinAgentKind::Anthropic,
        "openai" | "openai-compat" | "openai_compat" => BuiltinAgentKind::OpenAiCompat,
        _ => return None,
    };
    Some(BuiltinAgentConfig {
        id: payload.id,
        preset: payload
            .preset
            .as_deref()
            .and_then(BuiltinAgentPresetKey::from_str)
            .map(|saved| {
                op_editor_core::normalize_builtin_agent_preset(
                    saved,
                    kind,
                    &payload.base_url,
                    &payload.model,
                )
            })
            .unwrap_or_else(|| {
                op_editor_core::infer_builtin_agent_preset(kind, &payload.base_url, &payload.model)
            }),
        display_name: payload.display_name,
        kind,
        api_key: payload.api_key,
        model: payload.model,
        base_url: payload.base_url,
        enabled: payload.enabled,
    })
}

fn acp_agent_to_payload(agent: &AcpAgentConfig) -> AcpAgentPayload {
    AcpAgentPayload {
        id: agent.id.clone(),
        display_name: agent.display_name.clone(),
        connection_type: match agent.connection_type {
            AcpConnectionType::Local => "local",
            AcpConnectionType::Remote => "remote",
        }
        .into(),
        command: agent.command.clone(),
        args: agent.args.clone(),
        env: agent.env.clone(),
        url: agent.url.clone(),
        enabled: agent.enabled,
    }
}

fn acp_agent_from_payload(payload: AcpAgentPayload) -> Option<AcpAgentConfig> {
    let connection_type = match payload.connection_type.as_str() {
        "local" => AcpConnectionType::Local,
        "remote" => AcpConnectionType::Remote,
        _ => return None,
    };
    Some(AcpAgentConfig {
        id: payload.id,
        display_name: payload.display_name,
        connection_type,
        command: payload.command,
        args: payload.args,
        env: payload.env,
        url: payload.url,
        enabled: payload.enabled,
        connected: false,
    })
}

fn image_gen_profile_to_payload(profile: &ImageGenProfile) -> ImageGenProfilePayload {
    ImageGenProfilePayload {
        id: profile.id.clone(),
        name: profile.name.clone(),
        provider: match profile.provider {
            ImageGenProvider::OpenAi => "openai",
            ImageGenProvider::Gemini => "gemini",
            ImageGenProvider::Replicate => "replicate",
            ImageGenProvider::Custom => "custom",
        }
        .into(),
        api_key: profile.api_key.clone(),
        model: profile.model.clone(),
        base_url: profile.base_url.clone(),
    }
}

fn image_gen_profile_from_payload(payload: ImageGenProfilePayload) -> Option<ImageGenProfile> {
    let provider = match payload.provider.as_str() {
        "openai" => ImageGenProvider::OpenAi,
        "gemini" => ImageGenProvider::Gemini,
        "replicate" => ImageGenProvider::Replicate,
        "custom" => ImageGenProvider::Custom,
        _ => return None,
    };
    Some(ImageGenProfile {
        id: payload.id,
        name: payload.name,
        provider,
        api_key: payload.api_key,
        model: payload.model,
        base_url: payload.base_url,
        test_status: op_editor_core::agent_settings::ImageTestStatus::Idle,
    })
}

fn openverse_oauth_to_payload(
    settings: &op_editor_core::agent_settings::AgentSettings,
) -> Option<OpenverseOAuthPayload> {
    let client_id = settings.openverse_client_id.trim();
    let client_secret = settings.openverse_client_secret.trim();
    if client_id.is_empty() && client_secret.is_empty() {
        None
    } else {
        Some(OpenverseOAuthPayload {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        })
    }
}

fn dedupe_builtin_agents(agents: Vec<BuiltinAgentConfig>) -> Vec<BuiltinAgentConfig> {
    let mut deduped: Vec<BuiltinAgentConfig> = Vec::new();
    for agent in agents {
        let is_duplicate = deduped.iter().any(|existing| {
            existing.matches_add_candidate(
                &agent.display_name,
                &agent.api_key,
                &agent.model,
                agent.kind,
                &agent.base_url,
            )
        });
        if !is_duplicate {
            deduped.push(agent);
        }
    }
    deduped
}

fn next_builtin_agent_id(agents: &[BuiltinAgentConfig]) -> u64 {
    agents
        .iter()
        .filter_map(|agent| agent.id.strip_prefix("builtin-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn next_acp_agent_id(agents: &[AcpAgentConfig]) -> u64 {
    agents
        .iter()
        .filter_map(|agent| agent.id.strip_prefix("acp-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn next_image_gen_profile_id(profiles: &[ImageGenProfile]) -> u64 {
    profiles
        .iter()
        .filter_map(|profile| profile.id.strip_prefix("igp-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn theme_to_str(t: ThemeMode) -> &'static str {
    match t {
        ThemeMode::Dark => "dark",
        ThemeMode::Light => "light",
    }
}

fn str_to_theme(s: &str) -> ThemeMode {
    match s {
        "light" => ThemeMode::Light,
        _ => ThemeMode::Dark,
    }
}

fn locale_to_str(l: Locale) -> &'static str {
    match l {
        Locale::EnUs => "en-US",
        Locale::ZhCn => "zh-CN",
        Locale::ZhTw => "zh-TW",
        Locale::Ja => "ja",
        Locale::Ko => "ko",
        Locale::Fr => "fr",
        Locale::Es => "es",
        Locale::De => "de",
        Locale::Pt => "pt",
        Locale::Ru => "ru",
        Locale::Hi => "hi",
        Locale::Tr => "tr",
        Locale::Th => "th",
        Locale::Vi => "vi",
        Locale::Id => "id",
    }
}

fn str_to_locale(s: &str) -> Option<Locale> {
    Some(match s {
        "en-US" | "en" => Locale::EnUs,
        "zh-CN" | "zh" => Locale::ZhCn,
        "zh-TW" => Locale::ZhTw,
        "ja" => Locale::Ja,
        "ko" => Locale::Ko,
        "fr" => Locale::Fr,
        "es" => Locale::Es,
        "de" => Locale::De,
        "pt" => Locale::Pt,
        "ru" => Locale::Ru,
        "hi" => Locale::Hi,
        "tr" => Locale::Tr,
        "th" => Locale::Th,
        "vi" => Locale::Vi,
        "id" => Locale::Id,
        _ => return None,
    })
}

#[cfg(target_arch = "wasm32")]
fn storage_get(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

#[cfg(not(target_arch = "wasm32"))]
fn storage_get(_key: &str) -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn storage_set(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn storage_set(_key: &str, _value: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{EditorState, Locale, ThemeMode};

    #[test]
    fn settings_payload_restores_theme_and_locale() {
        let payload = r#"{"version":1,"theme":"light","locale":"en-US"}"#;
        let mut state = EditorState::new();

        apply_json(&mut state, payload).expect("settings payload should parse");

        assert_eq!(state.editor_ui.theme_mode, ThemeMode::Light);
        assert_eq!(state.editor_ui.locale, Locale::EnUs);
    }

    #[test]
    fn fingerprint_changes_when_theme_changes() {
        let mut state = EditorState::new();
        let before = fingerprint(&state);

        state.editor_ui.theme_mode = ThemeMode::Light;

        assert_ne!(before, fingerprint(&state));
    }

    #[test]
    fn settings_payload_round_trips_recent_files_and_mcp_preferences() {
        let mut src = EditorState::new();
        src.editor_ui.theme_mode = ThemeMode::Light;
        src.editor_ui.locale = Locale::Ja;
        src.editor_ui.agent_settings.mcp_server.port = 4321;
        src.editor_ui.agent_settings.mcp_cli_enabled[1] = true;
        src.editor_ui.agent_settings.images_advanced_open = true;
        src.editor_ui.agent_settings.openverse_client_id = "client".into();
        src.editor_ui.agent_settings.openverse_client_secret = "secret".into();
        src.editor_ui.agent_settings.auto_update_enabled = false;
        src.editor_ui.agent_settings.experimental_features_enabled = true;
        src.editor_ui.agent_settings.connected = [true, false, false, true, false];
        src.editor_ui.recent_files = vec![
            RecentFile {
                path: "/tmp/a.op".into(),
                modified_at: 1,
            },
            RecentFile {
                path: "/tmp/b.op".into(),
                modified_at: 2,
            },
        ];
        let json = serde_json::to_string(&to_payload(&src)).expect("settings serialize");
        let mut dst = EditorState::new();

        apply_json(&mut dst, &json).expect("settings payload parses");

        assert_eq!(dst.editor_ui.theme_mode, ThemeMode::Light);
        assert_eq!(dst.editor_ui.locale, Locale::Ja);
        assert_eq!(dst.editor_ui.agent_settings.mcp_server.port, 4321);
        assert!(dst.editor_ui.agent_settings.mcp_cli_enabled[1]);
        assert!(dst.editor_ui.agent_settings.images_advanced_open);
        assert_eq!(dst.editor_ui.agent_settings.openverse_client_id, "client");
        assert_eq!(
            dst.editor_ui.agent_settings.openverse_client_secret,
            "secret"
        );
        assert!(!dst.editor_ui.agent_settings.auto_update_enabled);
        assert!(dst.editor_ui.agent_settings.experimental_features_enabled);
        assert_eq!(
            dst.editor_ui.agent_settings.connected,
            [true, false, false, true, false]
        );
        assert_eq!(dst.editor_ui.recent_files.len(), 2);
        assert_eq!(dst.editor_ui.recent_files[0].path, "/tmp/a.op");
    }

    #[test]
    fn settings_payload_round_trips_agent_and_image_profiles() {
        let mut src = EditorState::new();
        src.editor_ui.agent_settings.add_builtin_agent_config(
            "MiniMax",
            "sk-test",
            "MiniMax-M2.7",
            BuiltinAgentKind::Anthropic,
            "https://api.minimaxi.com/anthropic",
        );
        src.editor_ui.agent_settings.add_acp_agent_config(
            "Design Agent",
            AcpConnectionType::Local,
            "/usr/local/bin/design-agent",
            vec!["--stdio".into()],
            std::collections::BTreeMap::new(),
            None,
            true,
        );
        let image_profile_id = src.editor_ui.agent_settings.add_image_gen_profile();
        let profile = &mut src.editor_ui.agent_settings.image_gen_profiles[0];
        profile.name = "Gemini Image".into();
        profile.provider = ImageGenProvider::Gemini;
        profile.api_key = "image-key".into();
        profile.model = "gemini-image".into();
        assert!(src
            .editor_ui
            .agent_settings
            .set_active_image_gen_profile(&image_profile_id));
        let json = serde_json::to_string(&to_payload(&src)).expect("settings serialize");
        let mut dst = EditorState::new();

        apply_json(&mut dst, &json).expect("settings payload parses");

        assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
        assert_eq!(
            dst.editor_ui.agent_settings.builtin_agents[0].display_name,
            "MiniMax"
        );
        assert_eq!(
            dst.editor_ui.agent_settings.builtin_agents[0].preset,
            BuiltinAgentPresetKey::MiniMax
        );
        assert_eq!(dst.editor_ui.agent_settings.acp_agents.len(), 1);
        assert_eq!(
            dst.editor_ui.agent_settings.acp_agents[0].command,
            "/usr/local/bin/design-agent"
        );
        assert_eq!(dst.editor_ui.agent_settings.image_gen_profiles.len(), 1);
        assert_eq!(
            dst.editor_ui.agent_settings.image_gen_profiles[0].provider,
            ImageGenProvider::Gemini
        );
        assert_eq!(
            dst.editor_ui
                .agent_settings
                .active_image_gen_profile_id
                .as_deref(),
            Some(image_profile_id.as_str())
        );
    }
}
