//! Auto-saved user settings (TS parity with `agent-settings-store`
//! localStorage).
//!
//! Distinct from `persistence.rs`:
//!  - `persistence.rs` saves the *document* (.pen / .op) to a path
//!    the user chose via the rfd Save dialog.
//!  - `settings_io.rs` saves the *preferences* (theme / locale /
//!    MCP port / MCP CLI toggles / Images advanced flag) to a
//!    fixed config dir so they survive app restarts.
//!
//! All preferences live on `EditorState.editor_ui` — the host's
//! single source of truth.

use std::path::PathBuf;

use op_editor_core::editor_ui_state::RecentFile;
use op_editor_core::{EditorState, Locale, ThemeMode};
use op_host_native::WidgetHostNative;
use serde::{Deserialize, Serialize};

const RECENT_CAP: usize = 10;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RecentFilePayload {
    path: String,
    modified_at: u64,
}

/// Cheap snapshot of every persisted field. Captured before each
/// dispatch; if it differs after, save the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    theme: ThemeMode,
    locale: Locale,
    port: u16,
    cli: [bool; 6],
    images_adv: bool,
    connected: [bool; 5],
}

pub fn fingerprint(state: &EditorState) -> Fingerprint {
    let eui = &state.editor_ui;
    Fingerprint {
        theme: eui.theme_mode,
        locale: eui.locale,
        port: eui.agent_settings.mcp_server.port,
        cli: eui.agent_settings.mcp_cli_enabled,
        images_adv: eui.agent_settings.images_advanced_open,
        connected: eui.agent_settings.connected,
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
    /// Per-provider connect state, indexed by `AgentProvider::ALL`
    /// (Claude / Codex / OpenCode / Copilot / Gemini). Restored on
    /// launch so the chat model picker survives a restart.
    #[serde(default)]
    connected: Option<[bool; 5]>,
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
        connected: Some(eui.agent_settings.connected),
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
    if let Some(c) = payload.connected {
        eui.agent_settings.connected = c;
    }
    if let Some(list) = payload.recent_files {
        eui.recent_files = list
            .into_iter()
            .take(RECENT_CAP)
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

/// Push `path` to the head of the recent-files list on the host's
/// `EditorState`, dedupe by path, cap at 10. Called by `persistence`
/// after every successful Save / Save As / Open.
pub fn touch_recent(host: &mut WidgetHostNative, path: &std::path::Path) {
    let path_s = path.to_string_lossy().into_owned();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let recents = &mut host.editor_state_mut().editor_ui.recent_files;
    recents.retain(|r| r.path != path_s);
    recents.insert(
        0,
        RecentFile {
            path: path_s,
            modified_at: now,
        },
    );
    recents.truncate(RECENT_CAP);
    host.mark_editor_state_dirty();
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
mod tests {
    use super::*;

    #[test]
    fn connected_state_round_trips_through_payload() {
        // Connect Claude (0) + Gemini (4), leave the rest off.
        let mut src = EditorState::new();
        src.editor_ui.agent_settings.connected = [true, false, false, false, true];
        // Serialize → JSON → deserialize, the real on-disk path.
        let json = serde_json::to_string(&to_payload(&src)).unwrap();
        let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
        let mut dst = EditorState::new();
        apply_payload(&mut dst, payload);
        assert_eq!(
            dst.editor_ui.agent_settings.connected,
            [true, false, false, false, true]
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
        assert_eq!(dst.editor_ui.agent_settings.connected, [false; 5]);
    }
}
