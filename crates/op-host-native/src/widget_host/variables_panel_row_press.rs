//! VariablesPanel row-cell presses on the native host — name rename,
//! per-kind value cells (number/string inline drafts, color picker /
//! inline hex, boolean variant toggle), the `⋯` overflow menu, and
//! the variant-targeted color swatch (#19). Split from
//! `variables_panel_press.rs` to honor the 800-line cap.

use super::WidgetHostNative;
use jian_ops_schema::variable::{VariableScalar, VariableValue};

impl WidgetHostNative {
    pub(in crate::widget_host) fn press_variable_name_cell(&mut self, idx: usize) -> bool {
        if !self.collab_allows_variables_mutation() {
            return true;
        }
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

        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
            self.mark_dirty();
            return true;
        };
        self.commit_property_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.editor_state
            .editor_ui
            .variable_row_input
            .set_text(name.clone());
        self.editor_state
            .editor_ui
            .variable_row_input
            .touch(self.now_ms);
        self.editor_state.ui.property_input_draft = name;
        self.editor_state.ui.property_caret_pos =
            self.editor_state.editor_ui.variable_row_input.caret();
        self.editor_state.ui.property_draft_select_all = false;
        self.editor_state.editor_ui.variable_row_focus = Some(VariableRowFocus::Name(idx));
        self.editor_state.editor_ui.last_variable_name_click = None;
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn press_variable_row(
        &mut self,
        idx: usize,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.collab_allows_variables_mutation() {
            return true;
        }
        use op_editor_ui::scene_vars::VariableKind as UiVariableKind;
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let Some((name, kind)) = var_table
            .variables
            .get(idx)
            .map(|v| (v.name.clone(), v.kind))
        else {
            return true;
        };
        match kind {
            UiVariableKind::Color => {
                self.commit_property_focus_if_any();
                let _ = self
                    .editor_state
                    .open_color_picker_for_variable_at(name, x, y);
            }
            UiVariableKind::Boolean => {
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
            UiVariableKind::Number | UiVariableKind::String => {
                use op_editor_core::editor_ui_state::VariableRowFocus;
                self.commit_property_focus_if_any();
                self.commit_variable_row_focus_if_any();
                let resolved = self.editor_state.resolve_variable(&name).cloned();
                let draft = match (&kind, &resolved) {
                    (UiVariableKind::Number, Some(VariableScalar::Num(n))) => format!("{n}"),
                    (UiVariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                    _ => String::new(),
                };
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .set_text(draft.clone());
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .touch(self.now_ms);
                self.editor_state.ui.property_input_draft = draft;
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.editor_ui.variable_row_input.caret();
                self.editor_state.editor_ui.variable_row_focus = Some(match kind {
                    UiVariableKind::Number => VariableRowFocus::Number(idx),
                    UiVariableKind::String => VariableRowFocus::String(idx),
                    _ => return true,
                });
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            }
        }
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    /// Variant-targeted swatch press (#19): the HSV picker opens
    /// seeded from THAT column's value and commits into that themed
    /// entry — clicking the Dark swatch edits Dark even while the
    /// canvas renders Light (TS `setValueForTheme`).
    pub(in crate::widget_host) fn press_variable_color_swatch(
        &mut self,
        idx: usize,
        variant: usize,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.collab_allows_variables_mutation() {
            return true;
        }
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
            return true;
        };
        self.commit_property_focus_if_any();
        self.commit_variable_row_focus_if_any();
        if let Some((axis, value)) = self.variable_axis_value_for_variant(variant) {
            let _ = self
                .editor_state
                .open_color_picker_for_variable_theme_at(name, axis, value, x, y);
        } else {
            // No declared theme axis — single implicit column; the
            // active-theme routing is equivalent.
            let _ = self
                .editor_state
                .open_color_picker_for_variable_at(name, x, y);
        }
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    /// Toggle a row's `⋯` overflow menu (TS `variable-row.tsx`
    /// showMenu), closing every other open panel menu.
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
        if !self.collab_allows_variables_mutation() {
            return true;
        }
        use op_editor_core::editor_ui_state::VariableRowFocus;
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
            self.close_variable_menus();
            return true;
        };
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.editor_state
            .editor_ui
            .variable_row_input
            .set_text(name.clone());
        self.editor_state.editor_ui.variable_row_input.select_all();
        self.editor_state
            .editor_ui
            .variable_row_input
            .touch(self.now_ms);
        self.editor_state.ui.property_caret_pos =
            self.editor_state.editor_ui.variable_row_input.caret();
        self.editor_state.ui.property_input_draft = name;
        self.editor_state.ui.property_draft_select_all = true;
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.editor_state.editor_ui.variable_row_focus = Some(VariableRowFocus::Name(idx));
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    /// Row-menu Delete — `delete_variable` resolves every `$name` ref
    /// in the tree to its concrete value first (TS `removeVariable`),
    /// under one history snapshot.
    pub(in crate::widget_host) fn delete_variable_row(&mut self, idx: usize) -> bool {
        if !self.collab_allows_variables_mutation() {
            return true;
        }
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
            self.close_variable_menus();
            return true;
        };
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        // Any cell focus indexes shift after a delete — drop the
        // focus instead of committing into the wrong row.
        self.editor_state.editor_ui.variable_row_focus = None;
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.delete_variable(&name) {
            self.editor_state.history_push_past(snap);
        }
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn press_variable_value_cell(
        &mut self,
        idx: usize,
        variant: usize,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.collab_allows_variables_mutation() {
            return true;
        }
        use op_editor_core::editor_ui_state::VariableRowFocus;
        use op_editor_ui::scene_vars::VariableKind as UiVariableKind;
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let Some((name, kind)) = var_table
            .variables
            .get(idx)
            .map(|v| (v.name.clone(), v.kind))
        else {
            return true;
        };
        match kind {
            // Color hex region → inline hex editing of exactly that
            // variant column (TS ColorCell's text input). The draft
            // seeds with the column's 6-char hex (TS `toHex7`).
            UiVariableKind::Color => {
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
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .set_text(hex.clone());
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .touch(self.now_ms);
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.editor_ui.variable_row_input.caret();
                self.editor_state.ui.property_input_draft = hex;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.editor_ui.variable_row_focus =
                    Some(VariableRowFocus::ColorCell { row: idx, variant });
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
            // Boolean cells toggle the clicked variant column
            // (rust-only — TS boolean rows are inert).
            UiVariableKind::Boolean => {
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
            UiVariableKind::Number | UiVariableKind::String => {
                self.commit_property_focus_if_any();
                self.commit_variable_row_focus_if_any();
                let scalar =
                    self.variable_axis_value_for_variant(variant)
                        .and_then(|(axis, value)| {
                            self.editor_state
                                .find_variable(&name)
                                .and_then(|def| scalar_for_axis_value(&def.value, &axis, &value))
                        });
                let scalar = scalar.or_else(|| self.editor_state.resolve_variable(&name).cloned());
                let draft = match (&kind, &scalar) {
                    (UiVariableKind::Number, Some(VariableScalar::Num(n))) => format!("{n}"),
                    (UiVariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                    _ => String::new(),
                };
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .set_text(draft.clone());
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .touch(self.now_ms);
                self.editor_state.ui.property_input_draft = draft;
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.editor_ui.variable_row_input.caret();
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.editor_ui.variable_row_focus = Some(match kind {
                    UiVariableKind::Number => VariableRowFocus::NumberCell { row: idx, variant },
                    UiVariableKind::String => VariableRowFocus::StringCell { row: idx, variant },
                    _ => return true,
                });
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
        }
    }
}

/// First 7 chars of a `#rrggbb(aa)` hex — TS `variable-row.tsx
/// toHex7` (alpha is dropped from the editable text; `#000000` for
/// anything unparseable).
fn hex_7(hex: &str) -> String {
    match hex.get(..7) {
        Some(prefix) if hex.starts_with('#') => prefix.to_string(),
        _ => "#000000".to_string(),
    }
}

fn scalar_for_axis_value(
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
