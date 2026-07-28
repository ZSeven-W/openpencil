//! VariablesPanel draft commits shared by the native and web widget
//! hosts — theme/variant header renames and the per-row cell drafts
//! (Name / Number / String / inline Color hex).
//!
//! Their `widget_host/variables_panel_commit.rs` twins were a verbatim
//! pair apart from two equivalent-but-costlier readings on the native
//! side (an `op_pen_loader` variable table built just to name the
//! `idx`-th variable — `doc.variables` is the very order that table
//! emits) and a merged early-return guard. Everything here is
//! `EditorState` mutation; the hosts keep only their `mark_dirty()` tail.

use std::collections::BTreeMap;

use crate::editor_ui_state::VariableRowFocus;
use crate::host_variables_transitions::ensure_variable_axis;
use crate::EditorState;

enum VariableHeaderFocus {
    Theme(String),
    Variant(String),
}

/// Exactly `#` + six hex digits (TS ColorCell's commit gate).
fn is_full_hex6(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Name of the `idx`-th variable row. Rows paint in `doc.variables`
/// (BTreeMap) order, so the map's key order is the row order.
fn variable_name_at(state: &EditorState, idx: usize) -> Option<String> {
    state.doc.variables.as_ref()?.keys().nth(idx).cloned()
}

/// Commit any pending VariablesPanel theme/variant header rename.
/// Returns `true` when a rename draft was pending (the host's repaint
/// trigger), regardless of whether it validated.
pub fn commit_header_focus(state: &mut EditorState) -> bool {
    let theme_axis = state.editor_ui.variables_theme_rename_axis.take();
    let variant_value = state.editor_ui.variables_variant_rename_value.take();
    let Some(focus) = theme_axis
        .map(VariableHeaderFocus::Theme)
        .or_else(|| variant_value.map(VariableHeaderFocus::Variant))
    else {
        return false;
    };
    state.ui.property_draft_select_all = false;
    let draft = state.editor_ui.variables_header_input.text().to_owned();
    let snap = state.snapshot_for_history();
    let committed = match focus {
        VariableHeaderFocus::Theme(old_axis) => rename_theme_axis(state, &old_axis, &draft),
        VariableHeaderFocus::Variant(old_value) => rename_variant_value(state, &old_value, &draft),
    };
    if committed {
        state.history_push_past(snap);
    }
    true
}

/// Discard an in-flight theme/variant rename without touching the document.
pub fn discard_header_focus(state: &mut EditorState) -> bool {
    let had_theme = state.editor_ui.variables_theme_rename_axis.take().is_some();
    let had_variant = state
        .editor_ui
        .variables_variant_rename_value
        .take()
        .is_some();
    let had_focus = had_theme || had_variant;
    if had_focus {
        state.ui.property_draft_select_all = false;
    }
    had_focus
}

/// Rename a theme axis, keeping the active-theme selection and the
/// current-axis pointer attached. Any rejection (blank / unchanged /
/// duplicate / missing) restores the old text in the header input.
fn rename_theme_axis(state: &mut EditorState, old_axis: &str, new_axis: &str) -> bool {
    let new_axis = new_axis.trim();
    let rejected = new_axis.is_empty()
        || old_axis == new_axis
        || match state.doc.themes.as_ref() {
            Some(themes) => themes.contains_key(new_axis),
            None => true,
        };
    if rejected {
        state.editor_ui.variables_header_input.set_text(old_axis);
        return false;
    }
    let mut found = false;
    if let Some(themes) = state.doc.themes.as_mut() {
        let mut updated = BTreeMap::new();
        for (axis, values) in std::mem::take(themes) {
            if axis == old_axis {
                updated.insert(new_axis.to_string(), values);
                found = true;
            } else {
                updated.insert(axis, values);
            }
        }
        *themes = updated;
    }
    if !found {
        state.editor_ui.variables_header_input.set_text(old_axis);
        return false;
    }
    if let Some(value) = state.ui.variables.active_theme.remove(old_axis) {
        state
            .ui
            .variables
            .active_theme
            .insert(new_axis.to_string(), value);
    }
    if state.editor_ui.variables_current_axis.as_deref() == Some(old_axis) {
        state.editor_ui.variables_current_axis = Some(new_axis.to_string());
    }
    state.editor_ui.variables_header_input.set_text(new_axis);
    true
}

/// Rename one variant value of the current axis, re-pointing the active
/// value when it was the renamed one.
fn rename_variant_value(state: &mut EditorState, old_value: &str, new_value: &str) -> bool {
    let new_value = new_value.trim();
    if new_value.is_empty() || old_value == new_value {
        state.editor_ui.variables_header_input.set_text(old_value);
        return false;
    }
    let axis = ensure_variable_axis(state);
    let Some(values) = state
        .doc
        .themes
        .as_mut()
        .and_then(|themes| themes.get_mut(&axis))
    else {
        state.editor_ui.variables_header_input.set_text(old_value);
        return false;
    };
    if values.iter().any(|v| v == new_value) {
        state.editor_ui.variables_header_input.set_text(old_value);
        return false;
    }
    let mut found = false;
    for value in values.iter_mut() {
        if value == old_value {
            *value = new_value.to_string();
            found = true;
        }
    }
    if found
        && state
            .ui
            .variables
            .active_theme
            .get(&axis)
            .is_some_and(|active| active == old_value)
    {
        state
            .ui
            .variables
            .active_theme
            .insert(axis, new_value.to_string());
    }
    if found {
        state.editor_ui.variables_header_input.set_text(new_value);
    } else {
        state.editor_ui.variables_header_input.set_text(old_value);
    }
    found
}

/// The axis the panel's variant columns belong to: the current one when
/// it still exists, else the first declared axis. Read-only (unlike
/// [`ensure_variable_axis`], it never mints one).
pub fn active_variable_axis(state: &EditorState) -> Option<String> {
    state
        .editor_ui
        .variables_current_axis
        .as_ref()
        .filter(|axis| {
            state
                .doc
                .themes
                .as_ref()
                .is_some_and(|themes| themes.contains_key(*axis))
        })
        .cloned()
        .or_else(|| {
            state
                .doc
                .themes
                .as_ref()
                .and_then(|themes| themes.keys().next().cloned())
        })
}

/// `(axis, value)` addressed by a variant column index.
pub fn variable_axis_value_for_variant(
    state: &EditorState,
    variant: usize,
) -> Option<(String, String)> {
    let axis = active_variable_axis(state)?;
    let value = state.doc.themes.as_ref()?.get(&axis)?.get(variant)?.clone();
    Some((axis, value))
}

/// Commit any pending VariablesPanel row edit (Name / Number / String /
/// inline Color hex). Returns `true` when a row draft was pending.
pub fn commit_row_focus(state: &mut EditorState) -> bool {
    let Some(focus) = state.editor_ui.variable_row_focus.take() else {
        return false;
    };
    state.ui.property_draft_select_all = false;
    let draft = state.editor_ui.variable_row_input.text().to_owned();
    let snap = state.snapshot_for_history();
    let committed = commit_row_draft(state, focus, draft);
    if committed {
        state.history_push_past(snap);
    }
    true
}

/// Discard an in-flight variable row draft without touching the document.
pub fn discard_row_focus(state: &mut EditorState) -> bool {
    let had_focus = state.editor_ui.variable_row_focus.take().is_some();
    if had_focus {
        state.ui.property_draft_select_all = false;
    }
    had_focus
}

fn commit_row_draft(state: &mut EditorState, focus: VariableRowFocus, draft: String) -> bool {
    match focus {
        VariableRowFocus::Name(idx) => {
            let Some(name) = variable_name_at(state, idx) else {
                return false;
            };
            let next = draft.trim();
            if next.is_empty() || next == name {
                state.editor_ui.variable_row_input.set_text(name);
                return false;
            }
            if state.rename_variable(&name, next) {
                state.editor_ui.variable_row_input.set_text(next);
                true
            } else {
                state.editor_ui.variable_row_input.set_text(name);
                false
            }
        }
        VariableRowFocus::Number(idx) | VariableRowFocus::NumberCell { row: idx, .. } => {
            let Some(name) = variable_name_at(state, idx) else {
                return false;
            };
            let Ok(n) = draft.trim().parse::<f64>() else {
                return false;
            };
            if !n.is_finite() {
                return false;
            }
            if let VariableRowFocus::NumberCell { variant, .. } = focus {
                if let Some((axis, value)) = variable_axis_value_for_variant(state, variant) {
                    return state.set_variable_number_for_theme(&name, &axis, &value, n);
                }
            }
            state.set_variable_number(&name, n)
        }
        VariableRowFocus::String(idx) | VariableRowFocus::StringCell { row: idx, .. } => {
            let Some(name) = variable_name_at(state, idx) else {
                return false;
            };
            if let VariableRowFocus::StringCell { variant, .. } = focus {
                if let Some((axis, value)) = variable_axis_value_for_variant(state, variant) {
                    return state.set_variable_string_for_theme(&name, &axis, &value, draft);
                }
            }
            state.set_variable_string(&name, draft)
        }
        // Inline color-cell hex — commits only a full `#rrggbb` (TS
        // `/^#[0-9a-fA-F]{6}$/`); anything else reverts (TS blur restores
        // the previous hex). DIVERGENCE: TS also live-commits each valid
        // keystroke; Rust follows the panel-wide commit-on-Enter/blur
        // discipline so one edit = one history entry.
        VariableRowFocus::ColorCell { row: idx, variant } => {
            let Some(name) = variable_name_at(state, idx) else {
                return false;
            };
            let hex = draft.trim();
            if !is_full_hex6(hex) {
                return false;
            }
            if let Some((axis, value)) = variable_axis_value_for_variant(state, variant) {
                return state.set_variable_color_for_theme(&name, &axis, &value, hex);
            }
            state.set_variable_color(&name, hex)
        }
    }
}
