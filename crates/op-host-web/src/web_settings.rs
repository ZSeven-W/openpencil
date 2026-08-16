//! Browser-local app preferences for the Rust web host.
//!
//! Documents still save through the normal file/daemon paths. This module
//! persists app-level preferences that native stores in `settings.json`:
//! theme, locale, recent files, MCP toggles, agent configs, and image-gen
//! profiles.

use op_editor_core::editor_ui_state::{RecentFile, RECENT_FILE_CAP};
use op_editor_core::{
    BuiltinAgentConfig, BuiltinAgentPresetKey, EditorState, ImageGenProfile, Locale, ThemeMode,
};
// Shared settings payload shapes + conversions — single-sourced in
// op-editor-host-core so the browser snapshots and the desktop/daemon
// `settings.json` cannot drift field-by-field.
use op_editor_host_core::settings_payload::{
    builtin_agent_from_payload, builtin_agent_to_payload, dedupe_builtin_agents,
    image_gen_profile_from_payload, image_gen_profile_to_payload, migrate_mcp_cli_flags,
    next_builtin_agent_id, next_image_gen_profile_id, openverse_oauth_to_payload, str_to_theme,
    theme_to_str, BuiltinAgentPayload, ImageGenProfilePayload, OpenverseOAuthPayload,
    RecentFilePayload,
};
use serde::{Deserialize, Serialize};

// The sibling test files reach these enums through `use super::*`.
#[cfg(test)]
use op_editor_core::{BuiltinAgentKind, ImageGenProvider};

const SETTINGS_VERSION: u32 = 1;
const CREDENTIAL_PAYLOAD_VERSION: u32 = 2;
const STORAGE_KEY: &str = "openpencil-rust-web-settings";
const CREDENTIAL_STORAGE_KEY: &str = "openpencil-rust-web-credentials";

/// Parse a managed embedding host's locale bootstrap hint. URL decoding uses
/// the browser-compatible form codec; the accepted values remain the strict
/// `zh-CN | en-US` bridge contract.
pub(crate) fn host_locale_from_query(search: &str) -> Option<Locale> {
    let query = search.strip_prefix('?').unwrap_or(search);
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "locale")
        .and_then(|(_, value)| op_editor_core::bridge_protocol::locale_from_wire(value.as_ref()))
}

/// Restore every setting the partition snapshot owns to its default.
///
/// `apply_payload` only writes fields the stored blob actually carries, so an
/// EMPTY partition left the previous account's values in place: B inherited
/// A's locale, recent files, MCP port and provider profiles. Resetting first
/// makes "absent from the blob" mean "default" rather than "keep whatever was
/// there".
///
/// The field set is exactly what `apply_payload` writes, so the two cannot
/// drift into a field that is saved per-account but never reset.
///
/// `theme_mode` is deliberately NOT included. Theme is a **device**
/// preference — light or dark is a property of the screen you are sitting at,
/// not of who is signed in — so it lives in its own unpartitioned key (see
/// [`theme`]) and must survive an account switch untouched. Resetting it here
/// would flip the screen every time a different person signed in on the same
/// laptop, which is the behaviour this split exists to remove.
pub(super) fn reset_account_scoped_settings(state: &mut EditorState) {
    let defaults = op_editor_core::AgentSettings::default();
    let eui = &mut state.editor_ui;
    eui.agent_settings.clear_builtin_model_catalogs();
    eui.locale = op_editor_core::EditorUiState::default().locale;
    eui.recent_files.clear();
    eui.agent_settings.mcp_server.port = defaults.mcp_server.port;
    eui.agent_settings.mcp_cli_enabled = defaults.mcp_cli_enabled;
    eui.agent_settings.images_advanced_open = defaults.images_advanced_open;
    eui.agent_settings.openverse_client_id = defaults.openverse_client_id;
    eui.agent_settings.openverse_client_secret = defaults.openverse_client_secret;
    eui.agent_settings.auto_update_enabled = defaults.auto_update_enabled;
    eui.agent_settings.experimental_features_enabled = defaults.experimental_features_enabled;
    eui.agent_settings.builtin_agents = defaults.builtin_agents;
    eui.agent_settings.next_builtin_agent_id = defaults.next_builtin_agent_id;
    eui.agent_settings.image_gen_profiles = defaults.image_gen_profiles;
    eui.agent_settings.next_image_gen_profile_id = defaults.next_image_gen_profile_id;
    eui.agent_settings.active_image_gen_profile_id = defaults.active_image_gen_profile_id;
}

/// Re-read account-scoped storage after the tab's partition changed.
///
/// The shell loads settings and credentials at mount, before any
/// `/api/auth/status` answer has arrived — so it loads them from the `anon`
/// partition. Once the account is known the partition key changes, and what is
/// in memory belongs to the wrong one: without this the tab keeps showing (and
/// re-saving) anonymous settings under the account's key.
pub(crate) fn reload_for_active_partition<C: crate::repaint_ctx::RepaintContext>(
    inner: &std::rc::Rc<std::cell::RefCell<C>>,
) {
    let Ok(mut context) = inner.try_borrow_mut() else {
        return;
    };
    // Defaults first, then the target partition on top: without this an empty
    // partition silently inherits the previous account's settings.
    reset_account_scoped_settings(context.host_mut().editor_state_mut());
    let load = storage::load_into(context.host_mut().editor_state_mut());
    // The device's own theme goes back on top of whatever the new partition
    // just applied. This is the account-switch half of the split: without it
    // the screen would follow whoever signed in.
    theme::apply_after_load(context.host_mut().editor_state_mut(), load.payload_theme());
    // AFTER the load, not before: the baselines must measure against the state
    // this partition just produced. Rebuilding them from the previous
    // account's state would make the very next comparison report the whole
    // partition as a change and write it back under the wrong key.
    context.reset_persistence_baselines(&load);
    // Restart credential sync for the partition now in force.
    //
    // Needed on FirstIdentified too, which does NOT go through the identity
    // reset: a policy fetch issued at mount under the anonymous epoch is
    // refused by the epoch guard when it lands, and `policy_in_flight` would
    // stay set forever — after which every `changed()` returns `SyncAction::
    // None` and the account can never upload a credential again.
    crate::web_credential_sync::reset();
    crate::web_credential_sync::start();
    context.host_mut().mark_editor_state_dirty();
    let _ = context.repaint();
}

/// Per-account storage keys.
///
/// The base keys above are same-origin and carry no account dimension, so two
/// accounts using one browser shared one settings blob — and one account's
/// provider API keys were readable by the next. Every read and write goes
/// through these instead, partitioned by the signed-in subject
/// (`identity_epoch::current_subject`, `"anon"` before sign-in).
///
/// The old unpartitioned keys are deliberately NOT migrated. They may hold a
/// different account's credentials than the one now signed in, and there is no
/// way to tell whose they were — so adopting them is exactly the leak this
/// closes. They are left in place (untouched, unread) rather than deleted, so
/// a user who downgrades does not lose their settings.
pub(crate) fn settings_storage_key() -> String {
    partitioned(STORAGE_KEY)
}

pub(crate) fn credential_storage_key() -> String {
    partitioned(CREDENTIAL_STORAGE_KEY)
}

fn partitioned(base: &str) -> String {
    format!("{base}::{}", crate::identity_epoch::current_subject())
}

#[path = "web_settings_legacy.rs"]
mod legacy;
#[path = "web_settings_storage.rs"]
mod storage;
#[path = "web_settings_theme.rs"]
pub(crate) mod theme;
#[path = "web_settings_validation.rs"]
mod validation;

#[cfg(test)]
use storage::{
    apply_stored_snapshots, load_into_with, save_credentials_if_changed_with, save_if_changed_with,
    StoredCredentialSource,
};
pub(crate) use storage::{
    credential_migration_pending, load_into, save_credentials_if_changed, save_if_changed,
    CredentialLoad,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Fingerprint {
    theme: ThemeMode,
    locale: Locale,
    port: u16,
    cli: [bool; 13],
    images_adv: bool,
    auto_update_enabled: bool,
    experimental_features_enabled: bool,
    recent_files: Vec<RecentFile>,
}

impl CredentialFingerprint {
    /// The fail-closed flag, for tests that assert an unsupported snapshot
    /// keeps writes disabled.
    #[cfg(test)]
    pub(crate) fn write_disabled_for_test(&self) -> bool {
        self.write_disabled
    }
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
    mcp_cli_enabled: Option<Vec<bool>>,
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
fn apply_json(
    state: &mut EditorState,
    raw: &str,
) -> Result<(), validation::SettingsValidationError> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| validation::SettingsValidationError::Json(e.to_string()))?;
    let payload = validation::settings_payload(&value)?;
    apply_payload(state, payload);
    Ok(())
}

#[cfg(test)]
fn apply_credential_json(
    state: &mut EditorState,
    raw: &str,
) -> Result<(), validation::SettingsValidationError> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|error| validation::SettingsValidationError::Json(error.to_string()))?;
    let validated = validation::credential_payload(&value)?;
    apply_credential_payload(state, validated.payload);
    Ok(())
}

fn to_payload(state: &EditorState) -> SettingsPayload {
    let eui = &state.editor_ui;
    SettingsPayload {
        version: SETTINGS_VERSION,
        theme: Some(theme_to_str(eui.theme_mode).into()),
        locale: Some(locale_to_str(eui.locale).into()),
        mcp_port: Some(eui.agent_settings.mcp_server.port),
        mcp_cli_enabled: Some(eui.agent_settings.mcp_cli_enabled.to_vec()),
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

/// The theme a stored account blob carries, for the one-time device-theme
/// migration. `None` for an unreadable or version-mismatched blob, which is
/// the same answer as "carried no theme".
pub(crate) fn payload_theme_of(raw: Option<&str>) -> Option<ThemeMode> {
    let payload: SettingsPayload = serde_json::from_str(raw?).ok()?;
    if payload.version != SETTINGS_VERSION {
        return None;
    }
    payload.theme.as_deref().map(str_to_theme)
}

fn apply_payload(state: &mut EditorState, payload: SettingsPayload) {
    if payload.version != SETTINGS_VERSION {
        return;
    }
    let eui = &mut state.editor_ui;
    // `payload.theme` is deliberately NOT applied: theme is device-level, and
    // reading it here would let the incoming account's blob flip the screen.
    // The field is still WRITTEN (see `to_payload`) so an older build — or an
    // older tab still running — keeps finding a theme where it expects one.
    // Its one remaining read is the migration in `theme::apply_after_load`,
    // which reaches it through `payload_theme_of` rather than through here.
    if let Some(locale) = payload.locale.as_deref().and_then(str_to_locale) {
        eui.locale = locale;
    }
    if let Some(port) = payload.mcp_port {
        eui.agent_settings.mcp_server.port = port.max(1024);
    }
    if let Some(flags) = payload.mcp_cli_enabled {
        eui.agent_settings.mcp_cli_enabled = migrate_mcp_cli_flags(flags);
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
        eui.agent_settings.clear_builtin_model_catalogs();
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
    settings.clear_builtin_model_catalogs();
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

#[cfg(test)]
#[path = "web_settings_disabled_provider_tests.rs"]
mod disabled_provider_tests;
