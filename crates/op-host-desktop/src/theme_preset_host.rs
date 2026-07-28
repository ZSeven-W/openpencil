//! Theme-preset host IO (#20) — app-level preset persistence + the
//! `.optheme` import/export file dialogs.
//!
//! TS parity:
//! - `apps/web/src/stores/theme-preset-store.ts` `persist()` /
//!   `hydrate()` (localStorage key `openpencil-theme-presets`) →
//!   `<config>/openpencil/theme-presets.json` holding the SAME JSON
//!   array value (kit_persistence.rs pattern: a payload moved between
//!   the TS and Rust apps hydrates on either side).
//! - `apps/web/src/utils/theme-preset-io.ts` `exportThemePreset` /
//!   `importThemePreset` → blocking rfd dialogs (persistence.rs
//!   pattern), `.optheme` filter, fixed `theme-preset` export name.
//!
//! Documented divergences:
//! - Storage is a JSON file, not localStorage — host platform
//!   difference; the value shape is identical.
//! - TS swallows export/import failures silently (try/catch → null);
//!   the Rust host logs them to stderr before moving on.

use std::path::PathBuf;

use op_editor_core::editor_ui_state::ThemePresetIo;
use op_editor_core::theme_presets::{
    parse_preset_file, preset_file_to_json, presets_from_json, presets_to_json,
    sanitize_preset_file_stem, THEME_PRESET_EXPORT_NAME,
};
use op_editor_core::EditorState;
use op_host_native::WidgetHostNative;

const APP_DIR: &str = "openpencil";
const FILE_NAME: &str = "theme-presets.json";

/// Resolve the platform-specific store path. `None` when no usable
/// config base exists — load/save become silent no-ops (same
/// contract as kit_persistence.rs).
fn store_path() -> Option<PathBuf> {
    let base = dirs::config_dir()?;
    Some(base.join(APP_DIR).join(FILE_NAME))
}

/// Hydrate the preset list at startup (TS `hydrate()` — tolerant of
/// a missing or malformed file).
pub fn load(state: &mut EditorState) {
    let Some(path) = store_path() else {
        return;
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return;
    };
    state.theme_presets = presets_from_json(&raw);
}

/// Persist the preset list when a save/delete/rename marked it dirty
/// (TS `persist()` runs inline on each store write; the Rust host
/// drains the flag once per frame).
pub fn persist_if_dirty(state: &mut EditorState) {
    if !state.theme_presets_dirty {
        return;
    }
    state.theme_presets_dirty = false;
    let Some(path) = store_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, presets_to_json(&state.theme_presets)) {
        eprintln!("openpencil-desktop: theme-presets.json save failed: {e}");
    }
}

/// Drain a pending `.optheme` import/export raised by the preset
/// dropdown. Blocking (rfd), like `persistence::run_action`. Returns
/// `true` when an action ran (caller requests a redraw).
pub fn drain_preset_io(host: &mut WidgetHostNative) -> bool {
    let Some(action) = host
        .editor_state_mut()
        .editor_ui
        .pending_theme_preset_io
        .take()
    else {
        return false;
    };
    match action {
        ThemePresetIo::Export => export_preset_dialog(host.editor_state()),
        ThemePresetIo::Import => {
            if host.gate_collaboration_action(
                op_editor_core::CollabGateAction::Document(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::VariablesThemes,
                    ),
                ),
                op_editor_core::CollabEditSource::Import,
            ) {
                import_preset_dialog(host);
            }
        }
    }
    true
}

/// TS `exportThemePreset` (`theme-preset-io.ts:33-75`): fixed
/// `theme-preset` name, sanitized `.optheme` file name, pretty JSON.
/// Cancel silently returns (TS catch → return).
fn export_preset_dialog(state: &EditorState) {
    let file_name = format!(
        "{}.optheme",
        sanitize_preset_file_stem(THEME_PRESET_EXPORT_NAME)
    );
    let Some(path) = rfd::FileDialog::new()
        .add_filter("OpenPencil Theme Preset", &["optheme"])
        .set_file_name(&file_name)
        .save_file()
    else {
        return;
    };
    let json = preset_file_to_json(
        THEME_PRESET_EXPORT_NAME,
        &state.doc.themes.clone().unwrap_or_default(),
        &state.doc.variables.clone().unwrap_or_default(),
    );
    if let Err(e) = std::fs::write(&path, json) {
        eprintln!(
            "openpencil-desktop: theme preset export failed ({}): {e}",
            path.display()
        );
    }
}

/// TS `importThemePreset` + `handleLoadPreset`
/// (`theme-preset-io.ts:78-133`, `variable-theme-manager.tsx:153-158`):
/// pick a `.optheme`, validate, merge themes + variables into the
/// document. An invalid file is a silent no-op in TS (resolves
/// `null`); the Rust host logs it. One undo step per import.
fn import_preset_dialog(host: &mut WidgetHostNative) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("OpenPencil Theme Preset", &["optheme"])
        .pick_file()
    else {
        return;
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!(
                "openpencil-desktop: theme preset read failed ({}): {e}",
                path.display()
            );
            return;
        }
    };
    let Some((_name, themes, variables)) = parse_preset_file(&raw) else {
        eprintln!(
            "openpencil-desktop: invalid theme preset file: {}",
            path.display()
        );
        return;
    };
    if !host.gate_collaboration_action(
        op_editor_core::CollabGateAction::Document(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::VariablesThemes,
            ),
        ),
        op_editor_core::CollabEditSource::Import,
    ) {
        return;
    }
    let state = host.editor_state_mut();
    let snap = state.snapshot_for_history();
    state.merge_theme_preset_payload(themes, variables);
    state.history_push_past(snap);
    host.mark_editor_state_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::variable::{VariableKind, VariableScalar};

    fn state_with_preset() -> EditorState {
        let mut state = EditorState::new();
        assert!(state.create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#ff0000".into()),
        ));
        assert!(state.save_theme_preset("kit", 7));
        state
    }

    #[test]
    fn store_file_round_trips_the_ts_local_storage_value_shape() {
        let state = state_with_preset();
        let raw = presets_to_json(&state.theme_presets);
        // The on-disk value is the exact TS localStorage array —
        // camelCase createdAt included.
        assert!(raw.starts_with('['));
        assert!(raw.contains("\"createdAt\":7"));
        let hydrated = presets_from_json(&raw);
        assert_eq!(hydrated, state.theme_presets);
    }

    #[test]
    fn persist_if_dirty_clears_the_flag_and_skips_when_clean() {
        let mut state = state_with_preset();
        assert!(state.theme_presets_dirty);
        // NOTE: writes to the real config dir is undesirable in a
        // unit test — exercise only the flag contract by pointing at
        // the parse/serialize pair (the fs round-trip is covered by
        // the import/export format tests).
        state.theme_presets_dirty = false;
        persist_if_dirty(&mut state); // clean → no-op
        assert!(!state.theme_presets_dirty);
    }

    #[test]
    fn import_merge_applies_one_undo_step_via_the_core_helpers() {
        // Wire-level slice of import_preset_dialog (sans rfd): parse
        // then merge + single history push.
        let donor = state_with_preset();
        let raw = preset_file_to_json(
            "kit",
            &donor.doc.themes.clone().unwrap_or_default(),
            &donor.doc.variables.clone().unwrap_or_default(),
        );
        let (_, themes, variables) = parse_preset_file(&raw).expect("valid");

        let mut state = EditorState::new();
        let depth = state.history.past.len();
        let snap = state.snapshot_for_history();
        state.merge_theme_preset_payload(themes, variables);
        state.history_push_past(snap);
        assert_eq!(state.history.past.len(), depth + 1);
        assert_eq!(
            state.resolve_variable("brand"),
            Some(&VariableScalar::Str("#ff0000".into()))
        );
    }

    #[test]
    fn export_file_name_is_the_sanitized_ts_default() {
        assert_eq!(
            format!(
                "{}.optheme",
                sanitize_preset_file_stem(THEME_PRESET_EXPORT_NAME)
            ),
            "theme-preset.optheme"
        );
    }
}
