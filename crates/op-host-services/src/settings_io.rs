//! Auto-saved user settings (TS parity with `agent-settings-store`
//! localStorage).
//!
//! All preferences live on `EditorState.editor_ui` — the host's
//! single source of truth.

use std::path::PathBuf;

use op_editor_core::editor_ui_state::{RecentFile, RECENT_FILE_CAP};
use op_editor_core::{
    AcpAgentConfig, AcpConnectionType, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey,
    EditorState, ImageGenProfile, ImageGenProvider, Locale, ThemeMode,
};
use serde::{Deserialize, Serialize};

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

/// Cheap snapshot of every persisted field. Captured before each
/// dispatch; if it differs after, save the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
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
}

pub fn fingerprint(state: &EditorState) -> Fingerprint {
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
    }
}

pub fn save_if_changed(state: &EditorState, before: Fingerprint) {
    if before != fingerprint(state) {
        save(state);
    }
}

const SETTINGS_VERSION: u32 = 1;
const APP_DIR: &str = "openpencil";
const FILE_NAME: &str = "settings.json";

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
    /// Per-provider connect state, indexed by `AgentProvider::ALL`
    /// (Claude / Codex / OpenCode / Copilot / Gemini). Restored on
    /// launch so the chat model picker survives a restart.
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

/// Resolve the platform-specific settings path. `None` when no
/// usable config base exists — load/save become silent no-ops.
fn settings_path() -> Option<PathBuf> {
    let base = dirs::config_dir()?;
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// Snapshot the live `EditorState` preferences into a serializable
/// payload.
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
    if let Some(s) = payload.theme.as_deref() {
        eui.theme_mode = str_to_theme(s);
    }
    // Locale precedence: persisted user choice > detected system
    // locale > the EditorState default (EnUs). Without this fallback
    // a fresh install on a Chinese system would still pop English
    // dialogs / chrome until the user manually picked a locale.
    if let Some(s) =
        payload
            .locale
            .as_deref()
            .and_then(|s| if s.is_empty() { None } else { str_to_locale(s) })
    {
        eui.locale = s;
    } else if let Some(detected) = detect_system_locale() {
        eui.locale = detected;
    }
    if let Some(port) = payload.mcp_port {
        eui.agent_settings.mcp_server.port = port.max(1024);
    }
    if let Some(flags) = payload.mcp_cli_enabled {
        eui.agent_settings.mcp_cli_enabled = flags;
    }
    if let Some(b) = payload.images_advanced_open {
        eui.agent_settings.images_advanced_open = b;
    }
    if let Some(oauth) = payload.openverse_oauth {
        eui.agent_settings.openverse_client_id = oauth.client_id;
        eui.agent_settings.openverse_client_secret = oauth.client_secret;
    }
    if let Some(b) = payload.auto_update_enabled {
        eui.agent_settings.auto_update_enabled = b;
    }
    if let Some(b) = payload.experimental_features_enabled {
        eui.agent_settings.experimental_features_enabled = b;
    }
    if let Some(c) = payload.connected {
        eui.agent_settings.connected = c;
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
    if let Some(list) = payload.recent_files {
        eui.recent_files = list
            .into_iter()
            .take(RECENT_FILE_CAP)
            .map(|r| RecentFile {
                path: r.path,
                modified_at: r.modified_at,
            })
            .collect();
    }
    // Restored connect state changes which providers the chat model
    // picker may list — re-derive it. `discovered_models` is still
    // empty this early, so this is a no-op until discovery lands and
    // `ModelProbe::poll_into` rebuilds again against the same mask.
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

fn next_builtin_agent_id(agents: &[BuiltinAgentConfig]) -> u64 {
    agents
        .iter()
        .filter_map(|agent| agent.id.strip_prefix("builtin-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
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

/// Best-effort load. Returns silently on missing file / parse error.
pub fn load(state: &mut EditorState) {
    // Seed the locale from the OS BEFORE the settings file is read.
    // `apply_payload`'s persisted-locale arm overrides this when a
    // saved choice exists; first-run / missing-file lands the
    // detected locale instead of leaving the EnUs default.
    if let Some(detected) = detect_system_locale() {
        state.editor_ui.locale = detected;
    }
    let Some(path) = settings_path() else { return };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let Ok(payload) = serde_json::from_slice::<SettingsPayload>(&bytes) else {
        return;
    };
    apply_payload(state, payload);
}

/// Read the host OS's preferred locale (env-var driven, no extra
/// crate dependency) and map it onto the supported [`Locale`] set.
/// Returns `None` when nothing resolves so the caller can keep its
/// fallback. Order matches POSIX precedence: `LC_ALL` overrides
/// `LANG` which overrides `LC_MESSAGES`.
fn detect_system_locale() -> Option<Locale> {
    for var in ["LC_ALL", "LANG", "LC_MESSAGES"] {
        let Ok(raw) = std::env::var(var) else {
            continue;
        };
        if let Some(loc) = locale_from_tag(&raw) {
            return Some(loc);
        }
    }
    None
}

/// Parse a POSIX / IETF locale tag (`zh_CN.UTF-8`, `zh-CN`,
/// `pt_BR`, `en`, …) onto the supported `Locale` set. Falls back to
/// the language subtag when the full tag is unknown so `pt_BR` still
/// lands `Locale::Pt` rather than rejecting.
fn locale_from_tag(raw: &str) -> Option<Locale> {
    let tag = raw.split('.').next().unwrap_or(raw).replace('_', "-");
    // Try the full tag first (handles `zh-CN` / `zh-TW`); fall back
    // to the language subtag (`zh-CN` → `zh`).
    if let Some(loc) = str_to_locale(&tag) {
        return Some(loc);
    }
    // Heuristic: zh-Hans → zh-CN, zh-Hant → zh-TW.
    let lower = tag.to_ascii_lowercase();
    if lower.starts_with("zh") {
        if lower.contains("hant") || lower.contains("tw") || lower.contains("hk") {
            return Some(Locale::ZhTw);
        }
        return Some(Locale::ZhCn);
    }
    let lang = tag.split('-').next().unwrap_or(&tag);
    str_to_locale(lang)
}

/// Best-effort save. Returns silently on IO failure.
pub fn save(state: &EditorState) {
    let Some(path) = settings_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let payload = to_payload(state);
    let Ok(json) = serde_json::to_string_pretty(&payload) else {
        return;
    };
    let mut tmp = path.clone();
    tmp.set_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
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
#[path = "settings_io_tests.rs"]
mod settings_io_tests;
