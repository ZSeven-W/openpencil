use super::helpers::rect_contains;
use super::WidgetHostNative;
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use op_editor_ui::widgets::variables_panel::{VariablesPanel, VariablesPanelHit};
use op_editor_ui::Point2D;
use std::collections::BTreeMap;

impl WidgetHostNative {
    pub(in crate::widget_host) fn dispatch_variables_panel_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.variables_panel_open {
            return false;
        }
        let Some(vars_rect) = self.variables_panel_rect(viewport_width, viewport_height) else {
            return false;
        };
        let point = Point2D::new(x, y);
        if !rect_contains(vars_rect, point) {
            return false;
        }
        let vars = VariablesPanel::for_editor(&self.editor_state);
        let Some(hit) = vars.hit_test(vars_rect, point) else {
            self.close_variable_menus();
            self.mark_dirty();
            return true;
        };
        match hit {
            VariablesPanelHit::Close => {
                self.commit_variables_panel_header_focus_if_any();
                self.commit_variable_row_focus_if_any();
                self.editor_state.editor_ui.variables_panel_open = false;
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
            VariablesPanelHit::ThemeTab(axis) => self.select_variable_axis(axis),
            VariablesPanelHit::ToggleThemeMenu(axis) => self.toggle_theme_menu(axis),
            VariablesPanelHit::ThemeMenuRename(axis) => self.start_theme_rename(axis),
            VariablesPanelHit::ThemeMenuDelete(axis) => self.delete_theme_axis(axis),
            VariablesPanelHit::AddTheme => self.add_variable_theme(),
            VariablesPanelHit::TogglePresetMenu => {
                let ui = &mut self.editor_state.editor_ui;
                ui.variables_preset_menu_open = !ui.variables_preset_menu_open;
                ui.variables_add_menu_open = false;
                ui.axis_dropdown_open = None;
                ui.variables_theme_menu_axis = None;
                ui.variables_variant_menu_value = None;
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AddVariant => self.add_variable_variant(),
            VariablesPanelHit::ToggleVariantMenu(value) => self.toggle_variant_menu(value),
            VariablesPanelHit::VariantMenuRename(value) => self.start_variant_rename(value),
            VariablesPanelHit::VariantMenuDelete(value) => self.delete_variant_value(value),
            VariablesPanelHit::ToggleAddVariableMenu => {
                let ui = &mut self.editor_state.editor_ui;
                ui.variables_add_menu_open = !ui.variables_add_menu_open;
                ui.variables_preset_menu_open = false;
                ui.axis_dropdown_open = None;
                ui.variables_theme_menu_axis = None;
                ui.variables_variant_menu_value = None;
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AddVariableColor => self.add_variable(
                "color",
                VariableKind::Color,
                VariableScalar::Str("#000000".into()),
            ),
            VariablesPanelHit::AddVariableNumber => {
                self.add_variable("number", VariableKind::Number, VariableScalar::Num(0.0))
            }
            VariablesPanelHit::AddVariableString => self.add_variable(
                "string",
                VariableKind::String,
                VariableScalar::Str("string".into()),
            ),
            VariablesPanelHit::NameCell(idx) => self.press_variable_name_cell(idx),
            VariablesPanelHit::ValueCell { row, variant } => {
                self.press_variable_value_cell(row, variant, x, y)
            }
            VariablesPanelHit::Row(idx) => self.press_variable_row(idx, x, y),
            VariablesPanelHit::AxisChip(idx) => self.toggle_variable_axis(idx),
            VariablesPanelHit::AxisDropdownItem { axis, value } => {
                self.commit_property_focus_if_any();
                let snap = self.editor_state.snapshot_for_history();
                if self.editor_state.set_active_axis_value(&axis, &value) {
                    self.editor_state.history_push_past(snap);
                }
                self.close_variable_menus();
                self.mark_dirty();
                true
            }
        }
    }

    fn toggle_theme_menu(&mut self, axis: String) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        let ui = &mut self.editor_state.editor_ui;
        ui.variables_theme_menu_axis = if ui.variables_theme_menu_axis.as_deref() == Some(&axis) {
            None
        } else {
            Some(axis)
        };
        ui.variables_variant_menu_value = None;
        ui.variables_add_menu_open = false;
        ui.variables_preset_menu_open = false;
        ui.axis_dropdown_open = None;
        self.mark_dirty();
        true
    }

    fn start_theme_rename(&mut self, axis: String) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.editor_state.ui.property_input_draft = axis.clone();
        self.editor_state.ui.property_caret_pos = axis.len();
        self.editor_state.ui.property_draft_select_all = false;
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        let ui = &mut self.editor_state.editor_ui;
        ui.variables_theme_rename_axis = Some(axis);
        ui.variables_theme_menu_axis = None;
        ui.variables_variant_menu_value = None;
        self.mark_dirty();
        true
    }

    fn delete_theme_axis(&mut self, axis: String) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        let Some(themes) = self.editor_state.doc.themes.as_ref() else {
            return true;
        };
        if themes.len() <= 1 || !themes.contains_key(&axis) {
            self.close_variable_menus();
            self.mark_dirty();
            return true;
        }
        let snap = self.editor_state.snapshot_for_history();
        if let Some(themes) = self.editor_state.doc.themes.as_mut() {
            themes.remove(&axis);
        }
        self.editor_state.ui.variables.active_theme.remove(&axis);
        if self
            .editor_state
            .editor_ui
            .variables_current_axis
            .as_deref()
            == Some(axis.as_str())
        {
            self.editor_state.editor_ui.variables_current_axis =
                self.editor_state.doc.themes.as_ref().and_then(|themes| {
                    themes.iter().next().map(|(next_axis, values)| {
                        self.editor_state.ui.variables.active_theme.insert(
                            next_axis.clone(),
                            values.first().cloned().unwrap_or_else(|| "Default".into()),
                        );
                        next_axis.clone()
                    })
                });
        }
        self.editor_state.history_push_past(snap);
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    fn toggle_variant_menu(&mut self, value: String) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        let ui = &mut self.editor_state.editor_ui;
        ui.variables_variant_menu_value =
            if ui.variables_variant_menu_value.as_deref() == Some(value.as_str()) {
                None
            } else {
                Some(value)
            };
        ui.variables_theme_menu_axis = None;
        ui.variables_add_menu_open = false;
        ui.variables_preset_menu_open = false;
        ui.axis_dropdown_open = None;
        self.mark_dirty();
        true
    }

    fn start_variant_rename(&mut self, value: String) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.editor_state.ui.property_input_draft = value.clone();
        self.editor_state.ui.property_caret_pos = value.len();
        self.editor_state.ui.property_draft_select_all = false;
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        let ui = &mut self.editor_state.editor_ui;
        ui.variables_variant_rename_value = Some(value);
        ui.variables_variant_menu_value = None;
        ui.variables_theme_menu_axis = None;
        self.mark_dirty();
        true
    }

    fn delete_variant_value(&mut self, value: String) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        let axis = self.ensure_variable_axis();
        let Some(values) = self
            .editor_state
            .doc
            .themes
            .as_ref()
            .and_then(|themes| themes.get(&axis))
        else {
            return true;
        };
        if values.len() <= 1 || !values.iter().any(|v| v == &value) {
            self.close_variable_menus();
            self.mark_dirty();
            return true;
        }
        let snap = self.editor_state.snapshot_for_history();
        if let Some(values) = self
            .editor_state
            .doc
            .themes
            .as_mut()
            .and_then(|themes| themes.get_mut(&axis))
        {
            values.retain(|v| v != &value);
            if self
                .editor_state
                .ui
                .variables
                .active_theme
                .get(&axis)
                .is_some_and(|active| active == &value)
            {
                if let Some(next) = values.first().cloned() {
                    self.editor_state
                        .ui
                        .variables
                        .active_theme
                        .insert(axis, next);
                }
            }
        }
        self.editor_state.history_push_past(snap);
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    fn press_variable_name_cell(&mut self, idx: usize) -> bool {
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
        self.editor_state.ui.property_input_draft = name;
        self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input_draft.len();
        self.editor_state.ui.property_draft_select_all = false;
        self.editor_state.editor_ui.variable_row_focus = Some(VariableRowFocus::Name(idx));
        self.editor_state.editor_ui.last_variable_name_click = None;
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    fn press_variable_row(&mut self, idx: usize, x: f32, y: f32) -> bool {
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
                self.editor_state.ui.property_input_draft = match (&kind, &resolved) {
                    (UiVariableKind::Number, Some(VariableScalar::Num(n))) => format!("{n}"),
                    (UiVariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                    _ => String::new(),
                };
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

    fn press_variable_value_cell(&mut self, idx: usize, variant: usize, x: f32, y: f32) -> bool {
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
            UiVariableKind::Color | UiVariableKind::Boolean => self.press_variable_row(idx, x, y),
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
                self.editor_state.ui.property_input_draft = match (&kind, &scalar) {
                    (UiVariableKind::Number, Some(VariableScalar::Num(n))) => format!("{n}"),
                    (UiVariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                    _ => String::new(),
                };
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.ui.property_input_draft.len();
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

    fn toggle_variable_axis(&mut self, idx: usize) -> bool {
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let axis = var_table.active_theme.keys().nth(idx).cloned();
        if let Some(name) = axis {
            self.commit_property_focus_if_any();
            let ui = &mut self.editor_state.editor_ui;
            ui.axis_dropdown_open = if ui.axis_dropdown_open.as_deref() == Some(name.as_str()) {
                None
            } else {
                Some(name)
            };
            ui.variables_add_menu_open = false;
            ui.variables_preset_menu_open = false;
        }
        self.mark_dirty();
        true
    }

    fn add_variable_theme(&mut self) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        let snap = self.editor_state.snapshot_for_history();
        let themes = self
            .editor_state
            .doc
            .themes
            .get_or_insert_with(BTreeMap::new);
        let name = unique_numbered("Theme", |candidate| themes.contains_key(candidate));
        themes.insert(name.clone(), vec!["Default".into()]);
        let current_axis = name.clone();
        self.editor_state
            .ui
            .variables
            .active_theme
            .insert(name, "Default".into());
        self.editor_state.editor_ui.variables_current_axis = Some(current_axis);
        self.editor_state.history_push_past(snap);
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    fn select_variable_axis(&mut self, axis: String) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        let Some(values) = self
            .editor_state
            .doc
            .themes
            .as_ref()
            .and_then(|themes| themes.get(&axis))
        else {
            return true;
        };
        let fallback = values.first().cloned().unwrap_or_else(|| "Default".into());
        self.editor_state
            .ui
            .variables
            .active_theme
            .entry(axis.clone())
            .or_insert(fallback);
        self.editor_state.editor_ui.variables_current_axis = Some(axis);
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    fn add_variable_variant(&mut self) -> bool {
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        let snap = self.editor_state.snapshot_for_history();
        let axis = self.ensure_variable_axis();
        let Some(values) = self
            .editor_state
            .doc
            .themes
            .as_mut()
            .and_then(|themes| themes.get_mut(&axis))
        else {
            return true;
        };
        let variant = unique_numbered("Variant", |candidate| values.iter().any(|v| v == candidate));
        values.push(variant);
        self.editor_state.history_push_past(snap);
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    fn add_variable(&mut self, base: &str, kind: VariableKind, default: VariableScalar) -> bool {
        use op_editor_core::editor_ui_state::VariableRowFocus;
        self.commit_property_focus_if_any();
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        let snap = self.editor_state.snapshot_for_history();
        self.ensure_variable_axis();
        let name = unique_numbered(base, |candidate| {
            self.editor_state
                .doc
                .variables
                .as_ref()
                .is_some_and(|vars| vars.contains_key(candidate))
        });
        let default_focus = match (&kind, &default) {
            (VariableKind::Number, VariableScalar::Num(value)) => Some((
                VariableRowFocus::NumberCell { row: 0, variant: 0 },
                format!("{value}"),
                true,
            )),
            (VariableKind::String, VariableScalar::Str(value)) => Some((
                VariableRowFocus::StringCell { row: 0, variant: 0 },
                value.clone(),
                !value.is_empty(),
            )),
            _ => None,
        };
        if self.editor_state.create_variable(&name, kind, default) {
            self.editor_state.history_push_past(snap);
            if let Some((focus, draft, select_all)) = default_focus {
                let row = op_pen_loader::editor_state_var_table(&self.editor_state)
                    .variables
                    .iter()
                    .position(|var| var.name == name)
                    .unwrap_or(0);
                self.editor_state.ui.property_input_draft = draft;
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.ui.property_input_draft.len();
                self.editor_state.ui.property_draft_select_all = select_all;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.editor_state.editor_ui.variable_row_focus = Some(match focus {
                    VariableRowFocus::NumberCell { variant, .. } => {
                        VariableRowFocus::NumberCell { row, variant }
                    }
                    VariableRowFocus::StringCell { variant, .. } => {
                        VariableRowFocus::StringCell { row, variant }
                    }
                    other => other,
                });
            }
        }
        self.close_variable_menus();
        self.mark_dirty();
        true
    }

    fn ensure_variable_axis(&mut self) -> String {
        if let Some(axis) = self
            .editor_state
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
        {
            return axis;
        }
        if let Some(axis) = self
            .editor_state
            .ui
            .variables
            .active_theme
            .keys()
            .next()
            .cloned()
        {
            return axis;
        }
        if let Some((axis, values)) = self
            .editor_state
            .doc
            .themes
            .as_ref()
            .and_then(|themes| themes.iter().next())
        {
            let value = values.first().cloned().unwrap_or_else(|| "Default".into());
            self.editor_state
                .ui
                .variables
                .active_theme
                .insert(axis.clone(), value);
            self.editor_state.editor_ui.variables_current_axis = Some(axis.clone());
            return axis.clone();
        }
        let themes = self
            .editor_state
            .doc
            .themes
            .get_or_insert_with(BTreeMap::new);
        let axis = unique_numbered("Theme", |candidate| themes.contains_key(candidate));
        themes.insert(axis.clone(), vec!["Default".into()]);
        self.editor_state
            .ui
            .variables
            .active_theme
            .insert(axis.clone(), "Default".into());
        self.editor_state.editor_ui.variables_current_axis = Some(axis.clone());
        axis
    }

    fn close_variable_menus(&mut self) {
        let ui = &mut self.editor_state.editor_ui;
        ui.axis_dropdown_open = None;
        ui.variables_add_menu_open = false;
        ui.variables_preset_menu_open = false;
        ui.variables_theme_menu_axis = None;
        ui.variables_variant_menu_value = None;
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

fn unique_numbered(base: &str, exists: impl Fn(&str) -> bool) -> String {
    for idx in 1.. {
        let candidate = format!("{base}-{idx}");
        if !exists(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}
