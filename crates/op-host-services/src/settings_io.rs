//! Auto-saved user settings (TS parity with `agent-settings-store`
//! localStorage).
//!
//! All preferences live on `EditorState.editor_ui` — the host's
//! single source of truth.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use op_editor_core::editor_ui_state::{RecentFile, RECENT_FILE_CAP};
use op_editor_core::{
    AcpAgentConfig, AcpConnectionType, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey,
    EditorState, ImageGenProfile, ImageGenProvider, Locale, ThemeMode,
};
use serde::{Deserialize, Serialize};

#[path = "settings_io_checked.rs"]
mod settings_io_checked;

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
    openverse_credential_owner: Option<String>,
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
        openverse_credential_owner: eui.agent_settings.openverse_credential_owner.clone(),
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
static SETTINGS_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    openverse_credential_owner: Option<String>,
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
        openverse_credential_owner: eui.agent_settings.openverse_credential_owner.clone(),
        auto_update_enabled: Some(eui.agent_settings.auto_update_enabled),
        experimental_features_enabled: Some(eui.agent_settings.experimental_features_enabled),
        connected: Some(eui.agent_settings.connected),
        // Skip auto-imported (e.g. Zode) agents: their source file is the
        // single source of truth and they're re-imported every launch, so
        // persisting them would silently duplicate the source's API keys
        // into this settings.json.
        builtin_agents: Some(
            eui.agent_settings
                .builtin_agents
                .iter()
                .filter(|agent| !eui.agent_settings.imported_agent_ids.contains(&agent.id))
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
    apply_payload_with_options(state, payload, true);
}

fn apply_payload_with_options(
    state: &mut EditorState,
    payload: SettingsPayload,
    dedupe_builtins: bool,
) {
    if payload.version != SETTINGS_VERSION {
        return;
    }
    let eui = &mut state.editor_ui;
    if let Some(s) = payload.theme.as_deref() {
        eui.theme_mode = str_to_theme(s);
    }
    // `load` seeds the current locale from the OS; only a valid
    // persisted user choice may override that seed.
    eui.locale = resolve_persisted_locale(eui.locale, payload.locale.as_deref());
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
    eui.agent_settings.openverse_credential_owner = payload.openverse_credential_owner;
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
        eui.agent_settings.builtin_agents = if dedupe_builtins {
            dedupe_builtin_agents(agents)
        } else {
            agents
        };
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

fn load_checked_from_path(state: &mut EditorState, path: &Path) -> Result<(), String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("failed to read settings file: {error}")),
    };
    let raw: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse settings file: {error}"))?;
    settings_io_checked::validate_payload_fields(&raw)?;
    let payload: SettingsPayload = serde_json::from_value(raw)
        .map_err(|error| format!("failed to parse settings file: {error}"))?;
    if payload.version != SETTINGS_VERSION {
        return Err(format!(
            "unsupported settings file version {}; expected {SETTINGS_VERSION}",
            payload.version
        ));
    }
    settings_io_checked::validate_lossless_payload(&payload)?;
    apply_payload_with_options(state, payload, false);
    Ok(())
}

/// Strict load used by web startup. A missing file is a normal first-run state,
/// but an existing file must be readable and losslessly loadable so the daemon
/// cannot later overwrite unknown settings or miss browser-owned credentials.
pub fn load_checked(state: &mut EditorState) -> Result<(), String> {
    if let Some(detected) = detect_system_locale() {
        state.editor_ui.locale = detected;
    }
    let path = settings_path().ok_or_else(|| "failed to resolve settings file path".to_string())?;
    load_checked_from_path(state, &path)?;
    crate::zode_import::import_zode_builtin_agents(state);
    Ok(())
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
    if let Some(path) = settings_path() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(payload) = serde_json::from_slice::<SettingsPayload>(&bytes) {
                apply_payload(state, payload);
            }
        }
    }
    // Merge any Zode CLI providers (`~/.zode/config.json`) as built-in
    // custom models. Runs AFTER the persisted agents load so the
    // backend-dedupe sees them; best-effort and a no-op when Zode isn't
    // configured. Must not be skipped by a missing OpenPencil settings
    // file, so it sits outside the load above.
    crate::zode_import::import_zode_builtin_agents(state);
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

/// Persist settings and report any failure to the caller.
pub fn save_checked(state: &EditorState) -> Result<(), String> {
    let path = settings_path().ok_or_else(|| "settings path is unavailable".to_string())?;
    save_checked_to_path(state, &path)
}

struct PendingSettingsFile {
    path: PathBuf,
    file: Option<std::fs::File>,
}

impl PendingSettingsFile {
    fn file_mut(&mut self) -> &mut std::fs::File {
        self.file
            .as_mut()
            .expect("pending settings file must stay open until replacement")
    }

    fn close(&mut self) {
        drop(self.file.take());
    }
}

impl Drop for PendingSettingsFile {
    fn drop(&mut self) {
        self.close();
        let _ = std::fs::remove_file(&self.path);
    }
}

fn create_unique_settings_temp(path: &Path) -> Result<PendingSettingsFile, String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "settings.json".into());

    for _ in 0..128 {
        let sequence = SETTINGS_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let tmp_path = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }

        match options.open(&tmp_path) {
            Ok(file) => {
                let pending = PendingSettingsFile {
                    path: tmp_path,
                    file: Some(file),
                };
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    pending
                        .file
                        .as_ref()
                        .expect("new settings file must be open")
                        .set_permissions(std::fs::Permissions::from_mode(0o600))
                        .map_err(|error| {
                            format!("failed to secure temporary settings file: {error}")
                        })?;
                }
                return Ok(pending);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!("failed to create temporary settings file: {error}"));
            }
        }
    }

    Err("failed to allocate a unique temporary settings file".to_string())
}

fn save_checked_to_path(state: &EditorState, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create settings directory: {error}"))?;
    }
    let payload = to_payload(state);
    let json = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("failed to encode settings: {error}"))?;
    let mut tmp = create_unique_settings_temp(path)?;
    if let Err(error) = tmp.file_mut().write_all(json.as_bytes()) {
        return Err(format!("failed to write temporary settings: {error}"));
    }
    tmp.close();
    if let Err(error) = std::fs::rename(&tmp.path, path) {
        return Err(format!("failed to replace settings file: {error}"));
    }
    Ok(())
}

/// Best-effort save for existing callers that do not surface persistence
/// failures to a request boundary.
pub fn save(state: &EditorState) {
    let _ = save_checked(state);
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

fn resolve_persisted_locale(current: Locale, persisted: Option<&str>) -> Locale {
    persisted.and_then(str_to_locale).unwrap_or(current)
}

#[cfg(test)]
#[path = "settings_io_tests.rs"]
mod settings_io_tests;
