//! Theme is a **device** preference, not an account one.
//!
//! Everything else in `web_settings` is partitioned by signed-in subject: an
//! MCP port, a recent-file list and a provider profile belong to the person,
//! and account B must never see account A's. Theme is the odd one out. Light
//! or dark is a property of the screen you are sitting at — the room's light,
//! the display, the time of day — and it does not change because someone else
//! signed in on the same laptop. That is now the product decision, so theme
//! moves out of the partitioned `SettingsPayload` semantics and into its own
//! **unpartitioned** key.
//!
//! ## Why a separate key rather than an unpartitioned payload
//!
//! `SettingsPayload` is account-scoped by construction. Splitting one field
//! into a tiny key of its own makes the device/account boundary visible in
//! storage itself, rather than encoded in which fields of a shared blob a
//! reader is allowed to trust.
//!
//! ## Compatibility: the payload keeps its `theme` field, write-only
//!
//! The per-account payload still *writes* `theme` and no longer *reads* it
//! (except once, as the migration source below). Dropping the write would be
//! cheaper but costs a real user something: an older build reads theme only
//! from the payload, so a downgrade — or a second tab still running the old
//! bundle — would silently snap back to the default. One redundant string per
//! account blob is a good trade for that.
//!
//! ## Migration
//!
//! First run after the upgrade has no device key. The theme in the partition
//! that just loaded is then adopted as the device theme and written out, so
//! the user's existing choice carries over instead of resetting to default.

use op_editor_core::{EditorState, ThemeMode};
use op_editor_host_core::settings_payload::{str_to_theme, theme_to_str};

use crate::web_storage::storage_get;

/// Device-level theme key. Deliberately carries **no** `::<subject>` suffix —
/// that suffix is what makes a key account-scoped.
const DEVICE_THEME_KEY: &str = "openpencil-rust-web-theme";

/// Why a device-theme write did not land.
///
/// Typed rather than a bool because the two cases are different operational
/// stories: storage that refuses everything (private mode, quota) is the
/// user's browser, while an unwritable value would be ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeviceThemeError {
    /// `localStorage` rejected the write — unavailable, full, or blocked.
    StorageUnavailable,
}

impl std::fmt::Display for DeviceThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageUnavailable => write!(f, "browser storage refused the device theme"),
        }
    }
}

thread_local! {
    /// Last value successfully written to the device key, so the per-frame
    /// check is a comparison rather than a `localStorage` write.
    ///
    /// `None` means "nothing written this session yet", which makes the first
    /// check after mount always attempt a write — that is what performs the
    /// migration when the key was absent.
    static LAST_WRITTEN: std::cell::Cell<Option<ThemeMode>> = const {
        std::cell::Cell::new(None)
    };
}

/// Parse the host theme bootstrap hint from a page query. Query decoding is
/// delegated to the URL form codec, while the value contract remains the same
/// strict `light | dark` contract as `op-bridge/theme`.
pub(crate) fn host_theme_from_query(search: &str) -> Option<ThemeMode> {
    let query = search.strip_prefix('?').unwrap_or(search);
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == "theme")
        .and_then(|(_, value)| {
            op_editor_core::bridge_protocol::color_scheme_from_wire(value.as_ref())
        })
}

/// Apply or update an embedding host's page-lifetime paint theme. The user's
/// `theme_mode` preference remains untouched and is therefore all persistence
/// paths continue to observe.
pub(crate) fn set_host_override(state: &mut EditorState, active: ThemeMode) {
    state.editor_ui.set_host_theme_override(Some(active));
}

/// The device theme, if this browser has one.
pub(crate) fn stored_device_theme() -> Option<ThemeMode> {
    parse_device_theme(storage_get(DEVICE_THEME_KEY).as_deref())
}

/// Parse a stored device-theme value. Split out so the state machine is
/// testable without a DOM.
pub(crate) fn parse_device_theme(raw: Option<&str>) -> Option<ThemeMode> {
    let raw = raw?.trim();
    // `str_to_theme` is total — it answers `Light` for anything it does not
    // recognise — so an unknown or empty value would masquerade as a real
    // stored preference and defeat the migration fallback. Only the two
    // values this module writes count as "the device has a theme".
    match raw {
        "light" | "dark" => Some(str_to_theme(raw)),
        _ => None,
    }
}

/// Resolve the theme for a freshly loaded partition.
///
/// `payload_theme` is whatever the account blob carried, which is used ONLY
/// when this device has no theme of its own — the one-time adoption of
/// pre-split data. Once the device key exists the account blob's copy is
/// ignored on every subsequent read.
///
/// Returns the theme now in force.
pub(crate) fn resolve(
    device_theme: Option<ThemeMode>,
    payload_theme: Option<ThemeMode>,
    default_theme: ThemeMode,
) -> ThemeMode {
    device_theme.or(payload_theme).unwrap_or(default_theme)
}

/// Apply the device theme to `state` after a partition load.
///
/// Called on mount and on every account switch. The switch case is the point
/// of the whole exercise: the partition reload resets and re-applies every
/// account-scoped field, and this puts the device's theme back on top so the
/// screen does not flip when a different person signs in.
pub(crate) fn apply_after_load(state: &mut EditorState, payload_theme: Option<ThemeMode>) {
    let default_theme = op_editor_core::EditorUiState::default().theme_mode;
    let resolved = resolve(stored_device_theme(), payload_theme, default_theme);
    state.editor_ui.theme_mode = resolved;
}

/// Persist the theme if it differs from what this session last wrote.
///
/// Deliberately NOT behind the settings fingerprint. That fingerprint is
/// `None` whenever the partition blob is unwritable (an unsupported stored
/// version fails closed), and a device preference must not be collateral
/// damage of an account blob the tab refuses to touch — that is exactly the
/// shape of the "reset and then never saves again" regression this codebase
/// has already paid for once.
pub(crate) fn save_if_changed(state: &EditorState) -> Result<bool, DeviceThemeError> {
    save_if_changed_with(state, |key, value| {
        crate::web_storage::storage_set_checked(key, value)
    })
}

/// [`save_if_changed`] against an injected writer, for tests.
pub(crate) fn save_if_changed_with<F>(
    state: &EditorState,
    mut persist: F,
) -> Result<bool, DeviceThemeError>
where
    F: FnMut(&str, &str) -> bool,
{
    let theme = state.editor_ui.theme_mode;
    if LAST_WRITTEN.with(std::cell::Cell::get) == Some(theme) {
        return Ok(false);
    }
    if !persist(DEVICE_THEME_KEY, theme_to_str(theme)) {
        return Err(DeviceThemeError::StorageUnavailable);
    }
    LAST_WRITTEN.with(|slot| slot.set(Some(theme)));
    Ok(true)
}

/// Forget what this session wrote, so the next save re-persists.
///
/// Only tests need it: nothing in the product invalidates a device preference,
/// which is the entire difference between it and an account-scoped one.
#[cfg(test)]
pub(crate) fn reset_last_written_for_test() {
    LAST_WRITTEN.with(|slot| slot.set(None));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(theme: ThemeMode) -> EditorState {
        let mut state = EditorState::new();
        state.editor_ui.theme_mode = theme;
        state
    }

    #[test]
    fn host_query_accepts_only_exact_light_or_dark_values() {
        assert_eq!(
            host_theme_from_query("?theme=light"),
            Some(ThemeMode::Light)
        );
        assert_eq!(
            host_theme_from_query("?embed=vscode&theme=dark"),
            Some(ThemeMode::Dark)
        );
        for query in [
            "",
            "?theme=Light",
            "?theme=system",
            "?theme=",
            "?colorScheme=dark",
        ] {
            assert_eq!(host_theme_from_query(query), None, "{query}");
        }
    }

    #[test]
    fn host_override_repaints_without_replacing_the_user_preference() {
        let mut state = state_with(ThemeMode::Dark);
        set_host_override(&mut state, ThemeMode::Light);
        assert_eq!(state.editor_ui.effective_theme_mode(), ThemeMode::Light);
        assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);

        set_host_override(&mut state, ThemeMode::Dark);
        assert_eq!(state.editor_ui.effective_theme_mode(), ThemeMode::Dark);
        assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);
    }

    #[test]
    fn host_override_is_never_written_as_the_device_preference() {
        reset_last_written_for_test();
        let mut state = state_with(ThemeMode::Light);
        set_host_override(&mut state, ThemeMode::Dark);
        let mut written = None;
        assert_eq!(
            save_if_changed_with(&state, |_, value| {
                written = Some(value.to_string());
                true
            }),
            Ok(true)
        );
        assert_eq!(written.as_deref(), Some("light"));
    }

    #[test]
    fn the_device_key_carries_no_account_partition() {
        // The `::<subject>` suffix is what makes a key account-scoped; a
        // device preference must not have one, or it is not device-level.
        assert!(!DEVICE_THEME_KEY.contains("::"));
        assert_ne!(DEVICE_THEME_KEY, super::super::settings_storage_key());
    }

    #[test]
    fn only_the_two_written_values_count_as_a_stored_preference() {
        assert_eq!(parse_device_theme(Some("dark")), Some(ThemeMode::Dark));
        assert_eq!(parse_device_theme(Some("light")), Some(ThemeMode::Light));
        assert_eq!(parse_device_theme(Some(" dark ")), Some(ThemeMode::Dark));
        // `str_to_theme` is total, so without the explicit match these would
        // masquerade as a real preference and suppress the migration.
        assert_eq!(parse_device_theme(Some("")), None);
        assert_eq!(parse_device_theme(Some("purple")), None);
        assert_eq!(parse_device_theme(None), None);
    }

    #[test]
    fn a_device_theme_wins_over_whatever_the_account_blob_says() {
        assert_eq!(
            resolve(
                Some(ThemeMode::Dark),
                Some(ThemeMode::Light),
                ThemeMode::Light
            ),
            ThemeMode::Dark
        );
    }

    #[test]
    fn existing_account_data_is_adopted_when_the_device_has_no_theme_yet() {
        // The migration: first run after the upgrade has no device key, so the
        // user's existing choice carries over instead of resetting.
        assert_eq!(
            resolve(None, Some(ThemeMode::Dark), ThemeMode::Light),
            ThemeMode::Dark
        );
    }

    #[test]
    fn a_fresh_browser_with_no_data_anywhere_gets_the_default() {
        assert_eq!(resolve(None, None, ThemeMode::Light), ThemeMode::Light);
        assert_eq!(resolve(None, None, ThemeMode::Dark), ThemeMode::Dark);
    }

    #[test]
    fn the_first_save_of_a_session_always_writes_so_the_migration_lands() {
        reset_last_written_for_test();
        let mut written = Vec::new();
        let saved = save_if_changed_with(&state_with(ThemeMode::Dark), |key, value| {
            written.push((key.to_string(), value.to_string()));
            true
        });
        assert_eq!(saved, Ok(true));
        assert_eq!(written, vec![(DEVICE_THEME_KEY.to_string(), "dark".into())]);
    }

    #[test]
    fn an_unchanged_theme_does_not_touch_storage_every_frame() {
        reset_last_written_for_test();
        let state = state_with(ThemeMode::Dark);
        assert_eq!(save_if_changed_with(&state, |_, _| true), Ok(true));

        let mut writes = 0usize;
        for _ in 0..10 {
            assert_eq!(
                save_if_changed_with(&state, |_, _| {
                    writes += 1;
                    true
                }),
                Ok(false)
            );
        }
        assert_eq!(writes, 0, "the per-frame check must be a comparison");
    }

    #[test]
    fn a_changed_theme_is_persisted_again() {
        reset_last_written_for_test();
        assert_eq!(
            save_if_changed_with(&state_with(ThemeMode::Dark), |_, _| true),
            Ok(true)
        );
        let mut value = String::new();
        assert_eq!(
            save_if_changed_with(&state_with(ThemeMode::Light), |_, v| {
                value = v.to_string();
                true
            }),
            Ok(true)
        );
        assert_eq!(value, "light");
    }

    #[test]
    fn a_refused_write_reports_and_stays_retryable() {
        // Storage can refuse (private mode, quota). The theme must keep
        // trying rather than latching as "written" — otherwise one refusal
        // during start-up loses the preference for the whole session.
        reset_last_written_for_test();
        let state = state_with(ThemeMode::Dark);
        assert_eq!(
            save_if_changed_with(&state, |_, _| false),
            Err(DeviceThemeError::StorageUnavailable)
        );
        assert_eq!(
            save_if_changed_with(&state, |_, _| true),
            Ok(true),
            "a refusal must not be recorded as a successful write"
        );
    }
}
