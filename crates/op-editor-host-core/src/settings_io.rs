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
// this crate so the desktop `settings.json` and the browser
// `web_settings` snapshots cannot drift field-by-field.
use crate::settings_payload::{
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
    image_gen_enabled: bool,
    preferred_agent_team_size: u32,
    entry_surface: op_editor_core::EntrySurface,
    chat_agent: String,
    chat_model: String,
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
        image_gen_enabled: eui.agent_settings.image_gen_enabled,
        preferred_agent_team_size: eui.preferred_agent_team_size,
        entry_surface: eui.entry_surface,
        chat_agent: selected_chat_agent_name(eui),
        chat_model: selected_chat_model_value(state),
    }
}

/// The persisted identity of the selected chat agent — its stable
/// provider NAME, never its index: `AgentProvider::ALL` is append-only
/// but new entries still shift nothing, whereas any future re-ordering
/// would silently re-map every persisted index. An out-of-range
/// selection falls back to the first provider's name.
fn selected_chat_agent_name(eui: &op_editor_core::EditorUiState) -> String {
    op_editor_core::AgentProvider::ALL
        .get(eui.chat_selected_agent)
        .map(|provider| provider.name())
        .unwrap_or(op_editor_core::AgentProvider::ALL[0].name())
        .to_string()
}

/// Resolve a persisted agent name back to its `AgentProvider::ALL`
/// index; unknown names (a provider renamed or removed by a newer
/// build) fall back to index 0 rather than dangling.
/// The picker row the user last chose, by the catalog entry's wire
/// `value` (`builtin:<agent id>:<model>` for API-key agents, the model id
/// for CLI providers, the agent id for ACP agents). `chat_agent` alone
/// cannot carry this: a built-in or ACP choice never moves
/// `chat_selected_agent`, so without this field every relaunch fell back
/// to the first catalog row.
fn selected_chat_model_value(state: &EditorState) -> String {
    state
        .chat
        .selected_model_entry()
        .map(|entry| entry.value.clone())
        .unwrap_or_default()
}

fn chat_agent_index_for_name(name: &str) -> usize {
    op_editor_core::AgentProvider::ALL
        .iter()
        .position(|provider| provider.name() == name)
        .unwrap_or(0)
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
/// Settings paths whose file existed but did not parse at startup. Every
/// save to one of these is refused for the rest of the process: the
/// alternative — writing this process's defaults over a file we could not
/// read — is exactly how a user loses every API key after a build mismatch
/// or a truncated write.
static REJECTED_SETTINGS_PATHS: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

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
    /// Chat-panel image-generation toggle; older files default to off.
    #[serde(default)]
    image_gen_enabled: Option<bool>,
    #[serde(default)]
    recent_files: Option<Vec<RecentFilePayload>>,
    /// User's last-set ⚡Nx parallel-agents team size — seeds tab 0's
    /// `ChatState::agent_team_size` on load. Absent in settings files
    /// predating this field; `#[serde(default)]` + the `unwrap_or(1)` at
    /// the apply site both land on the same `ChatState::default()` value,
    /// so an old file is fully backward-compatible.
    #[serde(default)]
    preferred_agent_team_size: Option<u32>,
    /// First-launch surface; older settings default to Home.
    #[serde(default)]
    entry_surface: Option<String>,
    /// The chat agent's stable provider name (see
    /// `selected_chat_agent_name`); older settings keep index 0.
    #[serde(default)]
    chat_agent: Option<String>,
    /// The last chosen picker row by catalog `value` (see
    /// `selected_chat_model_value`); resolved after the catalog is rebuilt.
    #[serde(default)]
    chat_model: Option<String>,
}

/// Resolve the platform-specific settings path. `None` when no
/// usable config base exists — load/save become silent no-ops.
///
/// An embedded shell (the mobile FFI hosts) selects its private
/// app-sandbox directory through `op_config_store::configure_user_root`
/// before engine construction; `settings.json` then lives next to the
/// other per-user config files in that root. Desktop never configures an
/// explicit root and keeps the platform config directory, so existing
/// installs do not move.
fn settings_path() -> Option<PathBuf> {
    if let Some(root) = op_config_store::configured_user_root() {
        return Some(root.join(FILE_NAME));
    }
    if let Some(sandbox) = test_process_settings_dir() {
        return Some(sandbox.join(FILE_NAME));
    }
    let base = dirs::config_dir()?;
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// A cargo test binary (`target/…/deps/<crate>-<hash>`) never touches the
/// user's real settings file. A desktop test once called the public
/// `save` on a fixture state and silently replaced the founder's
/// settings.json — API keys included — on every `cargo test`; every test
/// process now writes under its own temp directory instead. Explicit
/// config roots (`op_config_store::configured_user_root`) still win, so
/// tests that stage their own root keep working.
fn test_process_settings_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let under_deps = exe
        .components()
        .any(|component| component.as_os_str() == "deps");
    let under_target = exe
        .components()
        .any(|component| component.as_os_str() == "target");
    (under_deps && under_target).then(|| {
        std::env::temp_dir().join(format!("openpencil-test-settings-{}", std::process::id()))
    })
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
        image_gen_enabled: Some(eui.agent_settings.image_gen_enabled),
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
        entry_surface: Some(eui.entry_surface.as_str().into()),
        chat_agent: Some(selected_chat_agent_name(eui)),
        chat_model: Some(selected_chat_model_value(state)),
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
    let chat_model = payload.chat_model.clone();
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
    if let Some(enabled) = payload.image_gen_enabled {
        eui.agent_settings.image_gen_enabled = enabled;
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
    if let Some(surface) = payload.entry_surface.as_deref() {
        eui.entry_surface = op_editor_core::EntrySurface::from_str(surface);
    }
    // Restore the chat agent by name — `rebuild_chat_models` further
    // down re-derives the model catalog against this selection.
    if let Some(name) = payload.chat_agent.as_deref() {
        eui.chat_selected_agent = chat_agent_index_for_name(name);
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
    // Now that the catalog exists, land on the row the user last chose.
    // Later rebuilds (model discovery) preserve the selection by the same
    // provider + value + built-in id triple.
    if let Some(value) = chat_model.as_deref().filter(|v| !v.is_empty()) {
        if let Some(index) = state
            .chat
            .available_models
            .iter()
            .position(|entry| entry.value == value)
        {
            state.chat.selected_model = index;
        }
    }
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
        load_lenient_from_path(state, &path);
    }
}

/// The lenient startup load behind [`load`]. A missing file is a normal
/// first run. A file that exists but does not parse is NOT silently
/// replaced by defaults: its bytes are copied to a `settings.json.corrupt-…`
/// sibling and the path is pinned so every later save is refused (see
/// [`SettingsIoError::RejectedLoad`]). Returns `false` when the file was
/// rejected.
pub fn load_lenient_from_path(state: &mut EditorState, path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return true;
    };
    match serde_json::from_slice::<SettingsPayload>(&bytes) {
        Ok(payload) => {
            apply_payload(state, payload);
            true
        }
        Err(_) => {
            let _ = std::fs::write(corrupt_backup_path(path), &bytes);
            if let Ok(mut rejected) = REJECTED_SETTINGS_PATHS.lock() {
                rejected.push(path.to_path_buf());
            }
            false
        }
    }
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(FILE_NAME);
    path.with_file_name(format!("{name}.corrupt-{stamp}"))
}

fn save_refused_for(path: &Path) -> bool {
    REJECTED_SETTINGS_PATHS
        .lock()
        .map(|rejected| rejected.iter().any(|p| p == path))
        .unwrap_or(false)
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
    if save_refused_for(path) {
        return Err(SettingsIoError::RejectedLoad);
    }
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

#[cfg(test)]
#[path = "settings_io_chat_agent_tests.rs"]
mod settings_io_chat_agent_tests;

#[cfg(test)]
#[path = "settings_io_guard_tests.rs"]
mod settings_io_guard_tests;

#[cfg(test)]
#[path = "settings_io_chat_model_tests.rs"]
mod settings_io_chat_model_tests;
