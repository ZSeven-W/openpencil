//! VariablesPanel row-cell presses on the web host — name rename,
//! per-kind value cells (number/string inline drafts, color picker /
//! inline hex, boolean toggle), the `⋯` overflow menu, and the
//! variant-targeted color swatch (#19). Mirrors the native
//! `variables_panel_press.rs` row helpers.

use super::WidgetHost;
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};

impl WidgetHost {
    /// `(name, kind)` of the row at UNFILTERED index `idx`, straight
    /// from the `doc.variables` BTreeMap (the same order the panel's
    /// rows and the native host's var-table use).
    fn variable_at(&self, idx: usize) -> Option<(String, VariableKind)> {
        self.editor_state
            .doc
            .variables
            .as_ref()?
            .iter()
            .nth(idx)
            .map(|(name, def)| (name.clone(), def.kind.clone()))
    }

    /// Variant-targeted swatch press (#19) — the HSV picker opens
    /// seeded from THAT column's value and commits into that themed
    /// entry (TS `setValueForTheme`).
    pub(in crate::widget_host) fn press_variable_color_swatch(
        &mut self,
        idx: usize,
        variant: usize,
        x: f32,
        y: f32,
    ) -> bool {
        let Some((name, _)) = self.variable_at(idx) else {
            return true;
        };
        self.commit_property_focus_if_any();
        self.commit_variable_row_focus_if_any();
        if let Some((axis, value)) = self.variable_axis_value_for_variant(variant) {
            let _ = self
                .editor_state
                .open_color_picker_for_variable_theme_at(name, axis, value, x, y);
        } else {
            let _ = self
                .editor_state
                .open_color_picker_for_variable_at(name, x, y);
        }
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    /// Toggle a row's `⋯` overflow menu, closing every other menu.
    pub(in crate::widget_host) fn toggle_variable_row_menu(&mut self, idx: usize) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        let was_open = self.editor_state.editor_ui.variables_row_menu == Some(idx);
        self.close_variable_menus();
        self.editor_state.editor_ui.variables_row_menu = if was_open { None } else { Some(idx) };
        self.mark_dirty();
        true
    }

    /// Row-menu Rename — seeds the name draft with select-all (TS
    /// focuses the input and `.select()`s it).
    pub(in crate::widget_host) fn start_variable_row_rename(&mut self, idx: usize) -> bool {
        use op_editor_core::editor_ui_state::VariableRowFocus;
        let Some((name, _)) = self.variable_at(idx) else {
            self.close_variable_menus();
            return true;
        };
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.editor_state
            .editor_ui
            .variable_row_input
            .set_text(name);
        self.editor_state.editor_ui.variable_row_input.select_all();
        self.editor_state
            .editor_ui
            .variable_row_input
            .touch(self.now_ms);
        self.sync_variable_row_input_legacy(true);
        self.editor_state.editor_ui.variable_row_focus = Some(VariableRowFocus::Name(idx));
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    /// Row-menu Delete — `delete_variable` resolves every `$name`
    /// ref in the tree to its concrete value first, under one
    /// history snapshot.
    pub(in crate::widget_host) fn delete_variable_row(&mut self, idx: usize) -> bool {
        let Some((name, _)) = self.variable_at(idx) else {
            self.close_variable_menus();
            return true;
        };
        self.commit_variables_panel_header_focus_if_any();
        self.editor_state.editor_ui.variable_row_focus = None;
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.delete_variable(&name) {
            self.editor_state.history_push_past(snap);
        }
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    /// Name pill: 400 ms double-press starts the inline rename.
    pub(in crate::widget_host) fn press_variable_name_cell(&mut self, idx: usize) -> bool {
        use op_editor_core::editor_ui_state::VariableRowFocus;
        let is_double = matches!(
            self.editor_state.editor_ui.last_variable_name_click,
            Some((prev, t)) if prev == idx && self.now_ms.saturating_sub(t) < 400
        );
        self.editor_state.editor_ui.last_variable_name_click = Some((idx, self.now_ms));
        self.close_variable_menus();
        if !is_double {
            self.mark_dirty();
            return true;
        }
        let Some((name, _)) = self.variable_at(idx) else {
            self.mark_dirty();
            return true;
        };
        self.commit_property_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.editor_state
            .editor_ui
            .variable_row_input
            .set_text(name);
        self.editor_state
            .editor_ui
            .variable_row_input
            .touch(self.now_ms);
        self.sync_variable_row_input_legacy(false);
        self.editor_state.editor_ui.variable_row_focus = Some(VariableRowFocus::Name(idx));
        self.editor_state.editor_ui.last_variable_name_click = None;
        self.mark_dirty();
        true
    }

    /// Row-body press: Color opens the (active-theme) picker,
    /// Boolean toggles, Number/String start the row draft.
    pub(in crate::widget_host) fn press_variable_row(
        &mut self,
        idx: usize,
        x: f32,
        y: f32,
    ) -> bool {
        use op_editor_core::editor_ui_state::VariableRowFocus;
        let Some((name, kind)) = self.variable_at(idx) else {
            return true;
        };
        match kind {
            VariableKind::Color => {
                self.commit_property_focus_if_any();
                let _ = self
                    .editor_state
                    .open_color_picker_for_variable_at(name, x, y);
            }
            VariableKind::Boolean => {
                let current = self
                    .editor_state
                    .resolve_variable(&name)
                    .and_then(|s| match s {
                        VariableScalar::Bool(b) => Some(*b),
                        _ => None,
                    })
                    .unwrap_or(false);
                self.commit_property_focus_if_any();
                let snap = self.editor_state.snapshot_for_history();
                if self.editor_state.set_variable_boolean(&name, !current) {
                    self.editor_state.history_push_past(snap);
                }
            }
            VariableKind::Number | VariableKind::String => {
                self.commit_property_focus_if_any();
                self.commit_variable_row_focus_if_any();
                let resolved = self.editor_state.resolve_variable(&name).cloned();
                let draft = match (&kind, &resolved) {
                    (VariableKind::Number, Some(VariableScalar::Num(n))) => format!("{n}"),
                    (VariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                    _ => String::new(),
                };
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .set_text(draft);
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .touch(self.now_ms);
                self.sync_variable_row_input_legacy(false);
                self.editor_state.editor_ui.variable_row_focus = Some(match kind {
                    VariableKind::Number => VariableRowFocus::Number(idx),
                    VariableKind::String => VariableRowFocus::String(idx),
                    _ => return true,
                });
            }
        }
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    /// Value-cell press routed per kind. Color hex region → inline
    /// hex editing of exactly that variant column; Boolean toggles
    /// the clicked column; Number/String start a variant-cell draft.
    pub(in crate::widget_host) fn press_variable_value_cell(
        &mut self,
        idx: usize,
        variant: usize,
        x: f32,
        y: f32,
    ) -> bool {
        use op_editor_core::editor_ui_state::VariableRowFocus;
        let Some((name, kind)) = self.variable_at(idx) else {
            return true;
        };
        match kind {
            VariableKind::Color => {
                self.commit_property_focus_if_any();
                self.commit_variable_row_focus_if_any();
                let scalar = self
                    .variable_axis_value_for_variant(variant)
                    .and_then(|(axis, value)| {
                        self.editor_state
                            .find_variable(&name)
                            .and_then(|def| scalar_for_axis_value(&def.value, &axis, &value))
                    })
                    .or_else(|| self.editor_state.resolve_variable(&name).cloned());
                let hex = match scalar {
                    Some(VariableScalar::Str(hex)) => hex_7(&hex),
                    _ => "#000000".to_string(),
                };
                self.editor_state.editor_ui.variable_row_input.set_text(hex);
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .touch(self.now_ms);
                self.sync_variable_row_input_legacy(false);
                self.editor_state.editor_ui.variable_row_focus =
                    Some(VariableRowFocus::ColorCell { row: idx, variant });
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
            VariableKind::Boolean => {
                if let Some((axis, value)) = self.variable_axis_value_for_variant(variant) {
                    let current = self
                        .editor_state
                        .find_variable(&name)
                        .and_then(|def| scalar_for_axis_value(&def.value, &axis, &value))
                        .and_then(|s| match s {
                            VariableScalar::Bool(b) => Some(b),
                            _ => None,
                        })
                        .unwrap_or(false);
                    self.commit_property_focus_if_any();
                    let snap = self.editor_state.snapshot_for_history();
                    if self
                        .editor_state
                        .set_variable_boolean_for_theme(&name, &axis, &value, !current)
                    {
                        self.editor_state.history_push_past(snap);
                    }
                    self.close_variable_menus();
                    self.mark_dirty();
                    return true;
                }
                self.press_variable_row(idx, x, y)
            }
            VariableKind::Number | VariableKind::String => {
                self.commit_property_focus_if_any();
                self.commit_variable_row_focus_if_any();
                let scalar = self
                    .variable_axis_value_for_variant(variant)
                    .and_then(|(axis, value)| {
                        self.editor_state
                            .find_variable(&name)
                            .and_then(|def| scalar_for_axis_value(&def.value, &axis, &value))
                    })
                    .or_else(|| self.editor_state.resolve_variable(&name).cloned());
                let draft = match (&kind, &scalar) {
                    (VariableKind::Number, Some(VariableScalar::Num(n))) => format!("{n}"),
                    (VariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                    _ => String::new(),
                };
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .set_text(draft);
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .touch(self.now_ms);
                self.sync_variable_row_input_legacy(false);
                self.editor_state.editor_ui.variable_row_focus = Some(match kind {
                    VariableKind::Number => VariableRowFocus::NumberCell { row: idx, variant },
                    VariableKind::String => VariableRowFocus::StringCell { row: idx, variant },
                    _ => return true,
                });
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
        }
    }
}

/// Scalar shown in one variant column: exact `(axis, value)` match →
/// untagged entry → first entry (TS `getValueForTheme`).
pub(in crate::widget_host) fn scalar_for_axis_value(
    value: &VariableValue,
    axis: &str,
    theme_value: &str,
) -> Option<VariableScalar> {
    match value {
        VariableValue::Scalar(scalar) => Some(scalar.clone()),
        VariableValue::Themed(entries) => entries
            .iter()
            .find(|entry| {
                entry
                    .theme
                    .as_ref()
                    .and_then(|theme| theme.get(axis))
                    .is_some_and(|value| value == theme_value)
            })
            .or_else(|| entries.iter().find(|entry| entry.theme.is_none()))
            .or_else(|| entries.first())
            .map(|entry| entry.value.clone()),
    }
}

/// First 7 chars of a `#rrggbb(aa)` hex — TS `toHex7` (alpha is
/// dropped from the editable text; `#000000` for anything else).
pub(in crate::widget_host) fn hex_7(hex: &str) -> String {
    match hex.get(..7) {
        Some(prefix) if hex.starts_with('#') => prefix.to_string(),
        _ => "#000000".to_string(),
    }
}
