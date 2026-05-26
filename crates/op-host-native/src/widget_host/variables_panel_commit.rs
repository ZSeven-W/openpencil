use super::WidgetHostNative;
use std::collections::BTreeMap;

enum VariableHeaderFocus {
    Theme(String),
    Variant(String),
}

impl WidgetHostNative {
    /// Commit any pending VariablesPanel theme/variant header rename.
    pub(in crate::widget_host) fn commit_variables_panel_header_focus_if_any(&mut self) {
        let theme_axis = self
            .editor_state
            .editor_ui
            .variables_theme_rename_axis
            .take();
        let variant_value = self
            .editor_state
            .editor_ui
            .variables_variant_rename_value
            .take();
        let Some(focus) = theme_axis
            .map(VariableHeaderFocus::Theme)
            .or_else(|| variant_value.map(VariableHeaderFocus::Variant))
        else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = self.editor_state.ui.property_input_draft.clone();
        let snap = self.editor_state.snapshot_for_history();
        let committed = match focus {
            VariableHeaderFocus::Theme(old_axis) => {
                self.rename_variable_theme_axis(&old_axis, &draft)
            }
            VariableHeaderFocus::Variant(old_value) => {
                self.rename_variable_variant_value(&old_value, &draft)
            }
        };
        if committed {
            self.editor_state.history_push_past(snap);
        }
        self.mark_dirty();
    }

    fn rename_variable_theme_axis(&mut self, old_axis: &str, new_axis: &str) -> bool {
        let new_axis = new_axis.trim();
        if new_axis.is_empty() {
            self.editor_state.ui.property_input_draft = old_axis.to_string();
            return false;
        }
        let Some(themes) = self.editor_state.doc.themes.as_mut() else {
            self.editor_state.ui.property_input_draft = old_axis.to_string();
            return false;
        };
        if old_axis == new_axis {
            self.editor_state.ui.property_input_draft = old_axis.to_string();
            return false;
        }
        if themes.contains_key(new_axis) {
            self.editor_state.ui.property_input_draft = old_axis.to_string();
            return false;
        }
        let mut updated = BTreeMap::new();
        let mut found = false;
        for (axis, values) in std::mem::take(themes) {
            if axis == old_axis {
                updated.insert(new_axis.to_string(), values);
                found = true;
            } else {
                updated.insert(axis, values);
            }
        }
        *themes = updated;
        if !found {
            self.editor_state.ui.property_input_draft = old_axis.to_string();
            return false;
        }
        if let Some(value) = self.editor_state.ui.variables.active_theme.remove(old_axis) {
            self.editor_state
                .ui
                .variables
                .active_theme
                .insert(new_axis.to_string(), value);
        }
        if self
            .editor_state
            .editor_ui
            .variables_current_axis
            .as_deref()
            == Some(old_axis)
        {
            self.editor_state.editor_ui.variables_current_axis = Some(new_axis.to_string());
        }
        self.editor_state.ui.property_input_draft = new_axis.to_string();
        true
    }

    fn rename_variable_variant_value(&mut self, old_value: &str, new_value: &str) -> bool {
        let new_value = new_value.trim();
        if new_value.is_empty() {
            self.editor_state.ui.property_input_draft = old_value.to_string();
            return false;
        }
        let Some(axis) = self.active_variable_axis() else {
            self.editor_state.ui.property_input_draft = old_value.to_string();
            return false;
        };
        let Some(values) = self
            .editor_state
            .doc
            .themes
            .as_mut()
            .and_then(|themes| themes.get_mut(&axis))
        else {
            self.editor_state.ui.property_input_draft = old_value.to_string();
            return false;
        };
        if old_value == new_value {
            self.editor_state.ui.property_input_draft = old_value.to_string();
            return false;
        }
        if values.iter().any(|v| v == new_value) {
            self.editor_state.ui.property_input_draft = old_value.to_string();
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
            && self
                .editor_state
                .ui
                .variables
                .active_theme
                .get(&axis)
                .is_some_and(|active| active == old_value)
        {
            self.editor_state
                .ui
                .variables
                .active_theme
                .insert(axis, new_value.to_string());
        }
        if found {
            self.editor_state.ui.property_input_draft = new_value.to_string();
        } else {
            self.editor_state.ui.property_input_draft = old_value.to_string();
        }
        found
    }

    pub(in crate::widget_host) fn active_variable_axis(&self) -> Option<String> {
        self.editor_state
            .editor_ui
            .variables_current_axis
            .as_ref()
            .filter(|axis| {
                self.editor_state
                    .doc
                    .themes
                    .as_ref()
                    .is_some_and(|themes| themes.contains_key(*axis))
            })
            .cloned()
            .or_else(|| {
                self.editor_state
                    .doc
                    .themes
                    .as_ref()
                    .and_then(|themes| themes.keys().next().cloned())
            })
    }

    pub(in crate::widget_host) fn variable_axis_value_for_variant(
        &self,
        variant: usize,
    ) -> Option<(String, String)> {
        let axis = self.active_variable_axis()?;
        let value = self
            .editor_state
            .doc
            .themes
            .as_ref()?
            .get(&axis)?
            .get(variant)?
            .clone();
        Some((axis, value))
    }

    /// Commit any pending VariablesPanel row edit (Number / String).
    pub(in crate::widget_host) fn commit_variable_row_focus_if_any(&mut self) {
        use op_editor_core::editor_ui_state::VariableRowFocus;
        let Some(focus) = self.editor_state.editor_ui.variable_row_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = self.editor_state.ui.property_input_draft.clone();
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let snap = self.editor_state.snapshot_for_history();
        let committed = (|| -> bool {
            match focus {
                VariableRowFocus::Name(idx) => {
                    let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
                        return false;
                    };
                    let next = draft.trim();
                    if next.is_empty() || next == name {
                        self.editor_state.ui.property_input_draft = name;
                        return false;
                    }
                    if self.editor_state.rename_variable(&name, next) {
                        self.editor_state.ui.property_input_draft = next.to_string();
                        true
                    } else {
                        self.editor_state.ui.property_input_draft = name;
                        false
                    }
                }
                VariableRowFocus::Number(idx) | VariableRowFocus::NumberCell { row: idx, .. } => {
                    let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
                        return false;
                    };
                    let Ok(n) = draft.trim().parse::<f64>() else {
                        return false;
                    };
                    if !n.is_finite() {
                        return false;
                    }
                    if let VariableRowFocus::NumberCell { variant, .. } = focus {
                        if let Some((axis, value)) = self.variable_axis_value_for_variant(variant) {
                            return self
                                .editor_state
                                .set_variable_number_for_theme(&name, &axis, &value, n);
                        }
                    }
                    self.editor_state.set_variable_number(&name, n)
                }
                VariableRowFocus::String(idx) | VariableRowFocus::StringCell { row: idx, .. } => {
                    let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
                        return false;
                    };
                    if let VariableRowFocus::StringCell { variant, .. } = focus {
                        if let Some((axis, value)) = self.variable_axis_value_for_variant(variant) {
                            return self
                                .editor_state
                                .set_variable_string_for_theme(&name, &axis, &value, draft);
                        }
                    }
                    self.editor_state.set_variable_string(&name, draft)
                }
            }
        })();
        if committed {
            self.editor_state.history_push_past(snap);
        }
        self.mark_dirty();
    }
}
