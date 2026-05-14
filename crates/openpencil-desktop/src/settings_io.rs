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
//! Skips fields that look like state-of-the-world rather than
//! preferences (current tab, `connected[]` since there's no real
//! auth, hover state, draft buffers, in-progress drags).

use std::path::PathBuf;

use openpencil_shell_core::document::{Document, Locale, RecentFile, ThemeMode};
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
}

pub fn fingerprint(doc: &Document) -> Fingerprint {
    Fingerprint {
        theme: doc.ui.theme_mode,
        locale: doc.ui.locale,
        port: doc.ui.agent_settings.mcp_server.port,
        cli: doc.ui.agent_settings.mcp_cli_enabled,
        images_adv: doc.ui.agent_settings.images_advanced_open,
    }
}

pub fn save_if_changed(doc: &Document, before: Fingerprint) {
    if before != fingerprint(doc) {
        save(doc);
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
    recent_files: Option<Vec<RecentFilePayload>>,
}

/// Resolve the platform-specific settings path. `None` when neither
/// `dirs::config_dir` nor `dirs::home_dir` returns a usable base —
/// in that case load/save become no-ops (we never panic on missing
/// HOME, just silently skip persistence).
fn settings_path() -> Option<PathBuf> {
    let base = dirs::config_dir()?;
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// Snapshot the live `Document` preferences into a serializable
/// payload. Skips every field whose value is session state rather
/// than a user preference.
fn to_payload(doc: &Document) -> SettingsPayload {
    SettingsPayload {
        version: SETTINGS_VERSION,
        theme: Some(theme_to_str(doc.ui.theme_mode).into()),
        locale: Some(locale_to_str(doc.ui.locale).into()),
        mcp_port: Some(doc.ui.agent_settings.mcp_server.port),
        mcp_cli_enabled: Some(doc.ui.agent_settings.mcp_cli_enabled),
        images_advanced_open: Some(doc.ui.agent_settings.images_advanced_open),
        recent_files: Some(
            doc.ui
                .recent_files
                .iter()
                .map(|r| RecentFilePayload {
                    path: r.path.clone(),
                    modified_at: r.modified_at,
                })
                .collect(),
        ),
    }
}

fn apply_payload(doc: &mut Document, payload: SettingsPayload) {
    if payload.version != SETTINGS_VERSION {
        return;
    }
    if let Some(s) = payload.theme.as_deref() {
        doc.ui.theme_mode = str_to_theme(s);
    }
    if let Some(s) = payload.locale.as_deref() {
        if let Some(loc) = str_to_locale(s) {
            doc.ui.locale = loc;
        }
    }
    if let Some(port) = payload.mcp_port {
        doc.ui.agent_settings.mcp_server.port = port.max(1024);
    }
    if let Some(flags) = payload.mcp_cli_enabled {
        doc.ui.agent_settings.mcp_cli_enabled = flags;
    }
    if let Some(b) = payload.images_advanced_open {
        doc.ui.agent_settings.images_advanced_open = b;
    }
    if let Some(list) = payload.recent_files {
        doc.ui.recent_files = list
            .into_iter()
            .take(RECENT_CAP)
            .map(|r| RecentFile {
                path: r.path,
                modified_at: r.modified_at,
            })
            .collect();
    }
}

/// Push `path` to the head of `doc.ui.recent_files`, dedupe by
/// path, cap at 10. Stamps `modified_at` from the system clock so
/// the file_menu can format an "ago" string. Called by
/// `persistence` after every successful Save / Save As / Open.
pub fn touch_recent(doc: &mut Document, path: &std::path::Path) {
    let path_s = path.to_string_lossy().into_owned();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    doc.ui.recent_files.retain(|r| r.path != path_s);
    doc.ui.recent_files.insert(
        0,
        RecentFile {
            path: path_s,
            modified_at: now,
        },
    );
    doc.ui.recent_files.truncate(RECENT_CAP);
}

/// Best-effort load. Returns silently on missing file / parse error
/// so a corrupted settings file never blocks startup.
pub fn load(doc: &mut Document) {
    let Some(path) = settings_path() else { return };
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let Ok(payload) = serde_json::from_slice::<SettingsPayload>(&bytes) else {
        return;
    };
    apply_payload(doc, payload);
}

/// Best-effort save. Returns silently on IO failure — settings are
/// not a hard correctness contract and we already log error paths
/// elsewhere when something goes truly wrong.
pub fn save(doc: &Document) {
    let Some(path) = settings_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let payload = to_payload(doc);
    let Ok(json) = serde_json::to_string_pretty(&payload) else {
        return;
    };
    // Sibling temp + rename so a crash mid-write never leaves a
    // half-written settings file behind.
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
