//! Canonical `.op` (`PenDocument`) → shell `Document` loader / adapter.
//!
//! Extracted from `openpencil-desktop` so library crates (notably
//! `openpencil-shell-native`) can reuse the conversion without
//! depending on a binary crate.
//!
//! Two layers:
//!
//! - [`adapter`] bridges `jian_ops_schema::PenDocument` → the private
//!   [`payload::DocPayload`], running `jian-core::LayoutEngine` +
//!   `jian_skia::SkiaMeasure` to bake flex layout into AABB rects.
//! - [`payload`] holds the serde DTOs + `apply_payload` (payload →
//!   `Document` builder) + [`pen_document_to_document`] — the clean
//!   public entry point.
//!
//! This crate is desktop-side and may depend on skia; it does NOT
//! need to be wasm-clean. The `rfd` Save/Open dialogs + error
//! dialogs stay in `openpencil-desktop/src/persistence.rs`.

mod adapter;
mod bridge_enums;
mod bridge_enums_rev;
mod bridge_ui;
mod editor_state_bridge;
mod effects;
mod path_bounds;

pub mod payload;
pub mod variables;

/// Canonical entry point — convert a parsed `PenDocument` straight
/// into a shell-core `Document`.
pub use payload::pen_document_to_document;

// The `EditorState` → paint-`Document` type bridge. `op-editor-core`
// stores chrome / chat / components state as a different set of types
// than a shell-core `Document` carries; these functions translate
// them so a host can derive a faithful paint snapshot per frame.
pub use editor_state_bridge::{
    apply_editor_state_ui, editor_chat_to_chat_state,
    editor_components_to_component_library,
};
pub use bridge_ui::editor_ui_to_ui_state;
// The ~16 enum translators — also part of the bridge surface. Some
// (e.g. `fill_type`) have no field on `UiState` today because the
// value moved onto `Node`, but the pair still needs an exhaustive
// translator so future drift is caught; re-exporting keeps them
// reachable for callers + suppresses dead-code noise.
pub use bridge_enums::{
    agent_provider, agent_settings, agent_settings_tab, align_action,
    export_format, file_menu_choice, fill_type, flex_layout, locale, mcp_server,
    property_focus, property_tab, settings_focus, shape_choice, theme_mode, tool,
    variable_row_focus,
};

// The reverse (SC→EC) enum translators — the host-migration flip
// (Task 6.1c-2) feeds shell-core widget hit-test results into
// `op-editor-core` mutators. Exposed under the `rev` namespace so the
// forward / reverse `fn`s (same names, opposite direction) don't
// collide at the crate root.
pub mod rev {
    pub use crate::bridge_enums_rev::{
        agent_provider, agent_settings_tab, align_action, export_format,
        file_menu_choice, fill_type, flex_layout, locale, node_id, property_focus,
        property_tab, settings_focus, shape_choice, theme_mode, tool,
        variable_row_focus,
    };
}

// Re-exports so `openpencil-desktop`'s existing call sites change
// minimally.
pub use adapter::{build_var_table, pen_document_to_payload, LoadedDoc};
pub use effects::{effects_from_payload, effects_to_payload, shadows_from_canonical, ShadowPayload};
pub use payload::{
    apply_payload, load_canonical, to_payload, DocPayload, NodePayload, PagePayload, StrokePayload,
};
pub use variables::{var_table_from_payload, var_table_to_payload, VarTablePayload};

/// Build a shell-core [`VariableTable`] that reflects an
/// [`op_editor_core::EditorState`]'s full variable/theme state — the
/// persisted definitions (`EditorState.doc.variables` / `.themes`) AND
/// the transient editor selection (`EditorState.ui.variables`: the
/// active-theme map + the `fill_refs` / `stroke_refs` resolution
/// caches).
///
/// This is the variable-aware companion to [`pen_document_to_document`]:
/// a host's `paint_document()` derives a paint-only `Document` from
/// `EditorState`, then assigns this `var_table` so `VariablesPanel`
/// and variable-aware fill resolution see the same variables + active
/// theme the editor holds. Without it the derived `Document` would
/// carry an empty table (`apply_payload` resets `var_table` to
/// `Default`).
///
/// Lives here, not in `op-editor-core`: it materializes a
/// `shell-core::VariableTable`, and `op-editor-core` must stay free of
/// any shell-core dependency to keep its `wasm32-unknown-unknown`
/// invariant.
///
/// [`VariableTable`]: openpencil_shell_core::document::VariableTable
pub fn editor_state_var_table(
    state: &op_editor_core::EditorState,
) -> openpencil_shell_core::document::VariableTable {
    use openpencil_shell_core::document::NodeId;
    // Persisted definitions + theme axes — `EditorState.doc` is a
    // `PenDocument`, so `build_var_table` harvests them directly.
    let mut table = build_var_table(&state.doc);
    // Transient selection / caches from `EditorState.ui.variables`.
    let ui = &state.ui.variables;
    table.active_theme = ui.active_theme.clone();
    for (node_id, var_name) in &ui.fill_refs {
        table
            .fill_refs
            .insert(NodeId::new(node_id.as_str()), var_name.clone());
    }
    for (node_id, var_name) in &ui.stroke_refs {
        table
            .stroke_refs
            .insert(NodeId::new(node_id.as_str()), var_name.clone());
    }
    table
}

#[cfg(test)]
mod editor_state_var_table_tests {
    use super::editor_state_var_table;
    use jian_ops_schema::variable::{VariableKind, VariableScalar};
    use op_editor_core::EditorState;
    use openpencil_shell_core::document::NodeId;
    use std::collections::BTreeMap;

    #[test]
    fn folds_persisted_variables_themes_and_transient_selection() {
        let mut state = EditorState::new();
        // Persisted: a Color variable + a theme axis.
        state.create_variable(
            "brand",
            VariableKind::Color,
            VariableScalar::Str("#ff8800".into()),
        );
        let mut themes = BTreeMap::new();
        themes.insert(
            "mode".to_string(),
            vec!["light".to_string(), "dark".to_string()],
        );
        state.doc.themes = Some(themes);
        // Transient: active-theme selection + a fill ref cache entry.
        state.set_active_axis_value("mode", "dark");
        state
            .ui
            .variables
            .fill_refs
            .insert(op_editor_core::NodeId::new("n7"), "brand".into());

        let table = editor_state_var_table(&state);
        // Persisted definitions came across.
        assert_eq!(table.variables.len(), 1);
        assert_eq!(table.variables[0].name, "brand");
        assert_eq!(table.themes.len(), 1);
        assert_eq!(table.themes[0].name, "mode");
        // Transient selection + caches came across.
        assert_eq!(table.active_theme.get("mode"), Some(&"dark".to_string()));
        assert_eq!(
            table.fill_refs.get(&NodeId::new("n7")),
            Some(&"brand".to_string())
        );
        // The table resolves the variable to its colour.
        assert!(table.resolve_color("brand").is_some());
    }

    #[test]
    fn empty_editor_state_yields_empty_table() {
        let state = EditorState::new();
        let table = editor_state_var_table(&state);
        assert!(table.variables.is_empty());
        assert!(table.themes.is_empty());
        assert!(table.active_theme.is_empty());
    }
}
