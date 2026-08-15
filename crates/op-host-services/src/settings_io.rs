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
    AcpAgentConfig, AcpConnectionType, BuiltinAgentConfig, BuiltinAgentPresetKey, EditorState,
    ImageGenProfile, ThemeMode,
};
// Shared settings payload shapes + conversions — single-sourced in
// op-editor-host-core so the desktop `settings.json` and the browser
// `web_settings` snapshots cannot drift field-by-field.
use op_editor_host_core::settings_payload::{
    builtin_agent_from_payload, builtin_agent_to_payload, dedupe_builtin_agents,
    image_gen_profile_from_payload, image_gen_profile_to_payload, migrate_mcp_cli_flags,
    next_builtin_agent_id, next_image_gen_profile_id, openverse_oauth_to_payload, str_to_theme,
    theme_to_str, BuiltinAgentPayload, ImageGenProfilePayload, OpenverseOAuthPayload,
    RecentFilePayload,
};
use op_i18n::Locale;
use serde::{Deserialize, Serialize};

pub use crate::settings_io_error::SettingsIoError;

// The sibling test files reach these enums through `use super::*`.
#[cfg(test)]
use op_editor_core::{BuiltinAgentKind, ImageGenProvider};

#[path = "settings_io_checked.rs"]
mod settings_io_checked;

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

/// Cheap snapshot of every persisted field. Captured before each
/// dispatch; if it differs after, save the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    theme: ThemeMode,
    locale: Locale,
    port: u16,
    cli: [bool; 13],
    images_adv: bool,
    openverse_client_id: String,
    openverse_client_secret: String,
    openverse_credential_owner: Option<String>,
    auto_update_enabled: bool,
    experimental_features_enabled: bool,
    connected: [bool; 7],
    builtin_agents: Vec<BuiltinAgentConfig>,
    acp_agents: Vec<AcpAgentConfig>,
    image_gen_profiles: Vec<ImageGenProfile>,
    active_image_gen_profile_id: Option<String>,
    preferred_agent_team_size: u32,
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
        preferred_agent_team_size: eui.preferred_agent_team_size,
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
    mcp_cli_enabled: Option<Vec<bool>>,
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
    /// Per-provider connect state, indexed by `AgentProvider::ALL`.
    /// Restored on launch so the chat model picker survives a restart.
    #[serde(default)]
    connected: Option<Vec<bool>>,
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
    /// User's last-set ⚡Nx parallel-agents team size — seeds tab 0's
    /// `ChatState::agent_team_size` on load. Absent in settings files
    /// predating this field; `#[serde(default)]` + the `unwrap_or(1)` at
    /// the apply site both land on the same `ChatState::default()` value,
    /// so an old file is fully backward-compatible.
    #[serde(default)]
    preferred_agent_team_size: Option<u32>,
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
        locale: Some(eui.locale.code().into()),
        mcp_port: Some(eui.agent_settings.mcp_server.port),
        mcp_cli_enabled: Some(eui.agent_settings.mcp_cli_enabled.to_vec()),
        images_advanced_open: Some(eui.agent_settings.images_advanced_open),
        openverse_oauth: openverse_oauth_to_payload(&eui.agent_settings),
        openverse_credential_owner: eui.agent_settings.openverse_credential_owner.clone(),
        auto_update_enabled: Some(eui.agent_settings.auto_update_enabled),
        experimental_features_enabled: Some(eui.agent_settings.experimental_features_enabled),
        connected: Some(eui.agent_settings.connected.to_vec()),
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
        preferred_agent_team_size: Some(eui.preferred_agent_team_size),
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
        eui.agent_settings.mcp_cli_enabled = migrate_mcp_cli_flags(flags);
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
        eui.agent_settings.connected = migrate_connected_provider_flags(c);
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
    if let Some(size) = payload.preferred_agent_team_size {
        eui.preferred_agent_team_size = size.clamp(1, 6);
    }
    // Seed tab 0's ⚡Nx from the persisted preference — `load` runs before
    // any tab has been created beyond the default single tab, so this is
    // the ONE spot that reconnects "what the user last set" across a full
    // app restart (`ChatSessions::new_tab` handles the SAME continuity
    // within a running session, carrying the active tab's current value
    // forward). Captured into a local before the last `eui` use ends the
    // mutable borrow of `state.editor_ui`, so `state.chat` can be written
    // next.
    let preferred_agent_team_size = eui.preferred_agent_team_size;
    state.chat.agent_team_size = preferred_agent_team_size;
    // Restored connect state changes which providers the chat model
    // picker may list — re-derive it. `discovered_models` is still
    // empty this early, so this is a no-op until discovery lands and
    // `ModelProbe::poll_into` rebuilds again against the same mask.
    state.rebuild_chat_models();
}

/// Remove the retired Gemini CLI slot from positional v1 settings without
/// shifting the providers that followed it. Released settings used either
/// five slots (through Gemini) or seven slots (Gemini + Antigravity + Grok);
/// the current seven-slot layout omits Gemini and appends DeepSeek Harness
/// at the tail (see `chat::models::AgentProvider::ALL` — append-only, so
/// persisted indices never shift).
///
/// Length disambiguation: 7 is the CURRENT layout, so a 7-entry file is
/// copied verbatim (a round-trip must be lossless). The one casualty is a
/// legacy Gemini-era 7-entry file that was never opened since the
/// retirement — its index-4 Gemini flag reads as Antigravity — but the
/// startup reconnect replay re-probes every remembered provider, so an
/// uninstalled / unauthenticated CLI honestly degrades back to
/// disconnected on first launch. Treating 7 as legacy instead would
/// corrupt EVERY current file on every load, which is far worse.
fn migrate_connected_provider_flags(flags: Vec<bool>) -> [bool; 7] {
    let mut migrated = [false; 7];
    match flags.len() {
        // Current layout (or a longer one written by a newer build).
        7.. => migrated.copy_from_slice(&flags[..7]),
        6 => migrated[..6].copy_from_slice(&flags),
        5 => migrated[..4].copy_from_slice(&flags[..4]),
        _ => {
            let unchanged = flags.len().min(4);
            migrated[..unchanged].copy_from_slice(&flags[..unchanged]);
        }
    }
    migrated
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

fn next_acp_agent_id(agents: &[AcpAgentConfig]) -> u64 {
    agents
        .iter()
        .filter_map(|agent| agent.id.strip_prefix("acp-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn load_checked_from_path(state: &mut EditorState, path: &Path) -> Result<(), SettingsIoError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(SettingsIoError::Read {
                detail: error.to_string(),
            })
        }
    };
    let raw: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| SettingsIoError::Parse {
            detail: error.to_string(),
        })?;
    settings_io_checked::validate_payload_fields(&raw)?;
    let payload: SettingsPayload =
        serde_json::from_value(raw).map_err(|error| SettingsIoError::Parse {
            detail: error.to_string(),
        })?;
    if payload.version != SETTINGS_VERSION {
        return Err(SettingsIoError::UnsupportedVersion {
            found: payload.version,
            expected: SETTINGS_VERSION,
        });
    }
    settings_io_checked::validate_lossless_payload(&payload)?;
    apply_payload_with_options(state, payload, false);
    Ok(())
}

/// Strict load used by web startup. A missing file is a normal first-run state,
/// but an existing file must be readable and losslessly loadable so the daemon
/// cannot later overwrite unknown settings or miss browser-owned credentials.
/// External application configs are intentionally not imported here: the web
/// model catalog must reflect web/OpenPencil settings only, rather than expose
/// machine-local Zode providers that the browser settings UI cannot manage.
pub fn load_checked(state: &mut EditorState) -> Result<(), SettingsIoError> {
    seed_system_locale(state);
    let path = settings_path().ok_or(SettingsIoError::PathUnresolved)?;
    load_checked_from_path(state, &path)?;
    Ok(())
}

/// Best-effort OpenPencil settings load. Returns silently on missing file /
/// parse error. Host-specific imports belong to the host startup path rather
/// than this shared loader so the web daemon cannot inherit desktop-only
/// configuration sources.
pub fn load(state: &mut EditorState) {
    // Seed the locale from the OS BEFORE the settings file is read.
    // `apply_payload`'s persisted-locale arm overrides this when a
    // saved choice exists; first-run / missing-file lands the
    // detected locale instead of leaving the EnUs default.
    seed_system_locale(state);
    if let Some(path) = settings_path() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(payload) = serde_json::from_slice::<SettingsPayload>(&bytes) {
                apply_payload(state, payload);
            }
        }
    }
}

/// Seed the first-run locale through the shared i18n environment resolver.
///
/// `Locale::from_environment` owns POSIX precedence and parsing semantics, so
/// desktop and web-server startup cannot drift from the translation layer.
fn seed_system_locale(state: &mut EditorState) {
    if let Some(locale) = Locale::from_environment() {
        state.editor_ui.locale = locale;
    }
}

/// Persist settings and report any failure to the caller.
pub fn save_checked(state: &EditorState) -> Result<(), SettingsIoError> {
    let path = settings_path().ok_or(SettingsIoError::PathUnavailable)?;
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

fn create_unique_settings_temp(path: &Path) -> Result<PendingSettingsFile, SettingsIoError> {
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
                        .map_err(|error| SettingsIoError::SecureTemp {
                            detail: error.to_string(),
                        })?;
                }
                return Ok(pending);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(SettingsIoError::CreateTemp {
                    detail: error.to_string(),
                });
            }
        }
    }

    Err(SettingsIoError::TempAllocExhausted)
}

fn save_checked_to_path(state: &EditorState, path: &Path) -> Result<(), SettingsIoError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| SettingsIoError::CreateDir {
            detail: error.to_string(),
        })?;
    }
    let payload = to_payload(state);
    let json = serde_json::to_string_pretty(&payload).map_err(|error| SettingsIoError::Encode {
        detail: error.to_string(),
    })?;
    let mut tmp = create_unique_settings_temp(path)?;
    if let Err(error) = tmp.file_mut().write_all(json.as_bytes()) {
        return Err(SettingsIoError::WriteTemp {
            detail: error.to_string(),
        });
    }
    tmp.close();
    if let Err(error) = std::fs::rename(&tmp.path, path) {
        return Err(SettingsIoError::Replace {
            detail: error.to_string(),
        });
    }
    Ok(())
}

/// Best-effort save for existing callers that do not surface persistence
/// failures to a request boundary.
pub fn save(state: &EditorState) {
    let _ = save_checked(state);
}

fn resolve_persisted_locale(current: Locale, persisted: Option<&str>) -> Locale {
    persisted.and_then(Locale::from_tag).unwrap_or(current)
}

#[cfg(test)]
#[path = "settings_io_tests.rs"]
mod settings_io_tests;
