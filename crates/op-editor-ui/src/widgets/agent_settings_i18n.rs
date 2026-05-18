//! Settings-modal string lookup — thin wrapper over op-i18n.
//!
//! This used to be a hand-maintained EN/ZH table for the
//! agent-settings modal, kept separate because the `settings.*`
//! strings had not been added to the shared locale tables. Those
//! strings now live in the canonical 15-locale `op-i18n` tables
//! (`crates/op-i18n/src/i18n/*.rs`), so this file is only the
//! `t_settings(ui, key)` calling-convention adapter: it maps the
//! widget layer's `&EditorUiState` onto `op_i18n::translate`'s
//! `Locale` argument.
//!
//! `op_i18n::translate` falls through to English and then to the
//! key itself for an unknown key, so a missing string stays
//! visible in dev builds.

use op_editor_core::editor_ui_state::EditorUiState;

/// Translate a settings-modal `settings.*` key for the editor's
/// active locale.
pub fn t(ui: &EditorUiState, key: &'static str) -> &'static str {
    op_i18n::translate(ui.locale, key)
}
