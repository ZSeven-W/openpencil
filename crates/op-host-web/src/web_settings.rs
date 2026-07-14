//! Browser-local app preferences for the Rust web host.
//!
//! Documents still save through the normal file/daemon paths. This module
//! persists app-level preferences that native stores in `settings.json`:
//! theme, locale, recent files, MCP toggles, agent configs, and image-gen
//! profiles.

use op_editor_core::editor_ui_state::{RecentFile, RECENT_FILE_CAP};
use op_editor_core::{
    BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState, ImageGenProfile,
    ImageGenProvider, Locale, ThemeMode,
};
use serde::{Deserialize, Serialize};

const SETTINGS_VERSION: u32 = 1;
const CREDENTIAL_PAYLOAD_VERSION: u32 = 2;
const STORAGE_KEY: &str = "openpencil-rust-web-settings";
const CREDENTIAL_STORAGE_KEY: &str = "openpencil-rust-web-credentials";

#[path = "web_settings_legacy.rs"]
mod legacy;
#[path = "web_settings_storage.rs"]
mod storage;
#[path = "web_settings_validation.rs"]
mod validation;

#[cfg(test)]
use storage::{
    apply_stored_snapshots, load_into_with, save_credentials_if_changed_with, save_if_changed_with,
    StoredCredentialSource,
};
pub(crate) use storage::{
    credential_migration_pending, load_into, save_credentials_if_changed, save_if_changed,
};

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
    auto_update_enabled: bool,
    experimental_features_enabled: bool,
    recent_files: Vec<RecentFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CredentialFingerprint {
    builtin_agents: Vec<BuiltinAgentConfig>,
    image_gen_profiles: Vec<ImageGenProfile>,
    active_image_gen_profile_id: Option<String>,
    openverse_client_id: String,
    openverse_client_secret: String,
    write_pending: bool,
    write_disabled: bool,
    pending_credential_json: Option<String>,
    pending_settings_json: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    openverse_oauth: Option<OpenverseOAuthPayload>,
    #[serde(default)]
    auto_update_enabled: Option<bool>,
    #[serde(default)]
    experimental_features_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    builtin_agents: Option<Vec<BuiltinAgentPayload>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image_gen_profiles: Option<Vec<ImageGenProfilePayload>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_image_gen_profile_id: Option<String>,
    #[serde(default)]
    recent_files: Option<Vec<RecentFilePayload>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CredentialPayload {
    version: u32,
    builtin_agents: Vec<BuiltinAgentPayload>,
    image_gen_profiles: Vec<ImageGenProfilePayload>,
    active_image_gen_profile_id: Option<String>,
    openverse_oauth: Option<OpenverseOAuthPayload>,
}

pub(crate) fn fingerprint(state: &EditorState) -> Fingerprint {
    let eui = &state.editor_ui;
    Fingerprint {
        theme: eui.theme_mode,
        locale: eui.locale,
        port: eui.agent_settings.mcp_server.port,
        cli: eui.agent_settings.mcp_cli_enabled,
        images_adv: eui.agent_settings.images_advanced_open,
        auto_update_enabled: eui.agent_settings.auto_update_enabled,
        experimental_features_enabled: eui.agent_settings.experimental_features_enabled,
        recent_files: eui.recent_files.clone(),
    }
}

pub(crate) fn credential_fingerprint(state: &EditorState) -> CredentialFingerprint {
    let settings = &state.editor_ui.agent_settings;
    CredentialFingerprint {
        builtin_agents: settings.builtin_agents.clone(),
        image_gen_profiles: settings.image_gen_profiles.clone(),
        active_image_gen_profile_id: settings.active_image_gen_profile_id.clone(),
        openverse_client_id: settings.openverse_client_id.clone(),
        openverse_client_secret: settings.openverse_client_secret.clone(),
        write_pending: false,
        write_disabled: false,
        pending_credential_json: None,
        pending_settings_json: None,
    }
}

pub(crate) fn credentials_json(state: &EditorState) -> Option<String> {
    serde_json::to_string(&credential_payload(state)).ok()
}

/// Snapshot sent to the daemon when server persistence is explicitly enabled.
/// The web credential schema contains built-in providers and image services,
/// never CLI or ACP agent configuration.
pub(crate) fn server_credentials_json(state: &EditorState) -> Option<String> {
    credentials_json(state)
}

fn credential_payload(state: &EditorState) -> CredentialPayload {
    let settings = &state.editor_ui.agent_settings;
    CredentialPayload {
        version: CREDENTIAL_PAYLOAD_VERSION,
        builtin_agents: settings
            .builtin_agents
            .iter()
            .map(builtin_agent_to_payload)
            .collect(),
        image_gen_profiles: settings
            .image_gen_profiles
            .iter()
            .map(image_gen_profile_to_payload)
            .collect(),
        active_image_gen_profile_id: settings.active_image_gen_profile_id.clone(),
        openverse_oauth: openverse_oauth_to_payload(settings),
    }
}

#[cfg(test)]
fn apply_json(state: &mut EditorState, raw: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let payload = validation::settings_payload(&value)?;
    apply_payload(state, payload);
    Ok(())
}

#[cfg(test)]
fn apply_credential_json(state: &mut EditorState, raw: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| error.to_string())?;
    let payload = validation::credential_payload(&value)?;
    apply_credential_payload(state, payload);
    Ok(())
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
        openverse_oauth: None,
        auto_update_enabled: Some(eui.agent_settings.auto_update_enabled),
        experimental_features_enabled: Some(eui.agent_settings.experimental_features_enabled),
        builtin_agents: None,
        image_gen_profiles: None,
        active_image_gen_profile_id: None,
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
    if let Some(agents) = payload.builtin_agents {
        let agents = agents
            .into_iter()
            .filter_map(builtin_agent_from_payload)
            .collect();
        eui.agent_settings.builtin_agents = dedupe_builtin_agents(agents);
        eui.agent_settings.next_builtin_agent_id =
            next_builtin_agent_id(&eui.agent_settings.builtin_agents);
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

fn apply_credential_payload(state: &mut EditorState, payload: CredentialPayload) {
    let settings = &mut state.editor_ui.agent_settings;
    settings.builtin_agents = dedupe_builtin_agents(
        payload
            .builtin_agents
            .into_iter()
            .filter_map(builtin_agent_from_payload)
            .collect(),
    );
    settings.next_builtin_agent_id = next_builtin_agent_id(&settings.builtin_agents);
    settings.image_gen_profiles = payload
        .image_gen_profiles
        .into_iter()
        .filter_map(image_gen_profile_from_payload)
        .collect();
    settings.next_image_gen_profile_id = next_image_gen_profile_id(&settings.image_gen_profiles);
    settings.active_image_gen_profile_id = payload
        .active_image_gen_profile_id
        .filter(|active| {
            settings
                .image_gen_profiles
                .iter()
                .any(|profile| profile.id == *active)
        })
        .or_else(|| {
            settings
                .image_gen_profiles
                .first()
                .map(|profile| profile.id.clone())
        });
    if let Some(oauth) = payload.openverse_oauth {
        settings.openverse_client_id = oauth.client_id;
        settings.openverse_client_secret = oauth.client_secret;
    } else {
        settings.openverse_client_id.clear();
        settings.openverse_client_secret.clear();
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

#[cfg(test)]
#[path = "web_settings_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "web_settings_acp_scrub_tests.rs"]
mod acp_scrub_tests;

#[cfg(test)]
#[path = "web_settings_lossless_tests.rs"]
mod lossless_tests;
