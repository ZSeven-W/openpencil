//! PropertyPanel action + commit dispatch, split out of `input.rs`
//! to stay under the 800-line cap.
//!
//! Hit-tests run against `EditorState` (chrome / panels) + the
//! layout-resolved `LayoutScene` (canvas); results feed `EditorState`
//! mutators (the host's source of truth).

use super::helpers::parse_hex_color;
use super::WidgetHostNative;
use op_editor_core::ui_draft::PropertyFocus;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: op_editor_ui::widgets::PropertyPanelAction,
    ) {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        match action {
            A::SetFlexLayout(mode) => {
                self.editor_state.editor_ui.flex_layout = mode;
            }
            A::ToggleSizeFillWidth => {
                let v = &mut self.editor_state.editor_ui.size_fill_width;
                *v = !*v;
            }
            A::ToggleSizeFillHeight => {
                let v = &mut self.editor_state.editor_ui.size_fill_height;
                *v = !*v;
            }
            A::ToggleSizeHugWidth => {
                let v = &mut self.editor_state.editor_ui.size_hug_width;
                *v = !*v;
            }
            A::ToggleSizeHugHeight => {
                let v = &mut self.editor_state.editor_ui.size_hug_height;
                *v = !*v;
            }
            A::ToggleSizeClipContent => {
                let v = &mut self.editor_state.editor_ui.size_clip_content;
                *v = !*v;
            }
            A::ToggleFillTypePicker => {
                let v = &mut self.editor_state.editor_ui.fill_type_picker_open;
                *v = !*v;
            }
            A::SetFillType(t) => {
                self.editor_state.set_selected_fill_type(t);
                self.editor_state.editor_ui.fill_type_picker_open = false;
            }
            A::OpenColorPicker(target) => {
                // Fallback anchor when called outside the press path.
                let _ = self
                    .editor_state
                    .open_color_picker(color_target(target), 0.0);
            }
            A::OpenExportDialog => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::editor_ui_state::FileAction::ExportImage);
            }
            A::AddEffect => {
                self.editor_state.add_drop_shadow_to_selected();
            }
            A::RemoveEffect(index) => {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    self.editor_state.commit_history();
                    let _ =
                        self.editor_state
                            .apply(op_editor_core::EditorCommand::RemoveNodeEffect {
                                node_id: id,
                                index: index as u32,
                            });
                }
            }
            A::AdjustEffectParam {
                effect,
                field,
                new_value,
            } => {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    self.editor_state.commit_history();
                    let _ =
                        self.editor_state
                            .apply(op_editor_core::EditorCommand::SetEffectParam {
                                node_id: id,
                                index: effect as u32,
                                field,
                                value: new_value,
                            });
                }
            }
        }
        self.mark_dirty();
    }

    /// Export-dialog press dispatcher.
    pub(in crate::widget_host) fn dispatch_export_dialog_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        use op_editor_core::editor_ui_state::FileAction;
        use op_editor_ui::widgets::export_dialog::{
            scale_from_index, ExportDialog, ExportDialogHit,
        };
        let dlg = ExportDialog::centered(viewport_w, viewport_h);
        let point = op_editor_ui::Point2D::new(x, y);
        match dlg.hit_test(point) {
            Some(ExportDialogHit::Format(f)) => {
                self.editor_state.editor_ui.export_format =
                    op_editor_ui::widgets::editor_state_ext::export_format(f);
            }
            Some(ExportDialogHit::Scale(i)) => {
                self.editor_state.editor_ui.export_scale = scale_from_index(i);
            }
            Some(ExportDialogHit::Cancel) => {
                self.editor_state.editor_ui.export_dialog_open = false;
            }
            Some(ExportDialogHit::Export) => {
                self.editor_state.editor_ui.export_dialog_open = false;
                self.editor_state.editor_ui.pending_file_action =
                    Some(FileAction::ExportImageConfirm);
            }
            None => {
                if !dlg.contains(point) {
                    // Outside click — dismiss like Cancel.
                    self.editor_state.editor_ui.export_dialog_open = false;
                }
            }
        }
        self.mark_dirty();
    }

    /// Figma-import-modal press dispatcher.
    pub(in crate::widget_host) fn dispatch_figma_import_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        use op_editor_core::editor_ui_state::FileAction;
        use op_editor_ui::widgets::figma_import::{FigmaImportHit, FigmaImportModal};
        let modal = FigmaImportModal::for_editor(&self.editor_state);
        let panel_rect = modal.rect(viewport_w, viewport_h);
        match modal.hit_test(panel_rect, op_editor_ui::Point2D::new(x, y)) {
            FigmaImportHit::Close | FigmaImportHit::Outside => {
                self.editor_state.editor_ui.figma_import_open = false;
            }
            FigmaImportHit::DropZone => {
                self.editor_state.editor_ui.pending_file_action = Some(FileAction::ImportFigma);
                self.editor_state.editor_ui.figma_import_open = false;
            }
            FigmaImportHit::Inside => {}
        }
        self.mark_dirty();
    }

    /// File-menu press dispatcher.
    pub(in crate::widget_host) fn dispatch_file_menu_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
    ) {
        use op_editor_core::editor_ui_state::FileAction;
        use op_editor_ui::widgets::file_menu::{FileMenu, FileMenuChoice};
        use op_editor_ui::widgets::top_bar::TopBar;
        self.refresh_layout_scene();
        let top_bar_rect = op_editor_ui::Rect {
            origin: op_editor_ui::Point2D::new(0.0, 0.0),
            size: op_editor_ui::Point2D::new(viewport_width, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        };
        let anchor =
            TopBar::file_menu_rect(top_bar_rect, self.editor_state.editor_ui.window_fullscreen);
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, now_secs);
        let menu_rect = menu.rect_at(anchor);
        if let Some(choice) = menu.hit_test(menu_rect, op_editor_ui::Point2D::new(x, y)) {
            self.editor_state.editor_ui.pending_file_action = Some(match choice {
                FileMenuChoice::NewFile => FileAction::New,
                FileMenuChoice::OpenFile => FileAction::Open,
                FileMenuChoice::Save => FileAction::Save,
                FileMenuChoice::SaveAs => FileAction::SaveAs,
                FileMenuChoice::ExportImage => FileAction::ExportImage,
                FileMenuChoice::OpenRecent(i) => FileAction::OpenRecent(i),
                FileMenuChoice::ClearRecent => FileAction::ClearRecent,
            });
        }
        self.editor_state.editor_ui.file_menu_open = false;
        self.editor_state.editor_ui.file_menu_hover = None;
        self.mark_dirty();
    }

    /// Commit any focused settings-modal input (currently only the
    /// MCP port).
    pub(in crate::widget_host) fn commit_settings_focus_if_any(&mut self) {
        use op_editor_core::agent_settings::SettingsFocus;
        let Some(focus) = self.editor_state.editor_ui.agent_settings.focus.take() else {
            return;
        };
        let draft = std::mem::take(&mut self.editor_state.editor_ui.settings_input_draft);
        match focus {
            SettingsFocus::McpPort => {
                if let Ok(port) = draft.trim().parse::<u16>() {
                    self.editor_state.editor_ui.agent_settings.mcp_server.port = port.max(1024);
                }
            }
        }
        self.mark_dirty();
    }

    /// Commit any pending VariablesPanel row edit (Number / String).
    pub(in crate::widget_host) fn commit_variable_row_focus_if_any(&mut self) {
        use op_editor_core::editor_ui_state::VariableRowFocus;
        let Some(focus) = self.editor_state.editor_ui.variable_row_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = std::mem::take(&mut self.editor_state.ui.property_input_draft);
        // Resolve the row index → variable name off the editor-state
        // var-table (the same Vec the VariablesPanel widget walks).
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let snap = self.editor_state.snapshot_for_history();
        // Every path below has already cleared focus + drained the
        // draft, so each exit must finalize through `mark_dirty` or the
        // derived render scene stays stale after an invalid edit. An
        // inner closure makes the "did the value commit" branches
        // return into one place that always marks dirty.
        let committed = (|| -> bool {
            match focus {
                VariableRowFocus::Number(idx) => {
                    let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
                        return false;
                    };
                    let Ok(n) = draft.trim().parse::<f64>() else {
                        return false;
                    };
                    if !n.is_finite() {
                        return false;
                    }
                    self.editor_state.set_variable_number(&name, n)
                }
                VariableRowFocus::String(idx) => {
                    let Some(name) = var_table.variables.get(idx).map(|v| v.name.clone()) else {
                        return false;
                    };
                    self.editor_state.set_variable_string(&name, draft)
                }
            }
        })();
        if committed {
            self.editor_state.history_push_past(snap);
        }
        self.mark_dirty();
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        // Commit any pending variable-row edit first.
        self.commit_variable_row_focus_if_any();
        let Some(focus) = self.editor_state.ui.property_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = std::mem::take(&mut self.editor_state.ui.property_input_draft);
        match focus {
            PropertyFocus::FillHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self
                            .editor_state
                            .set_selected_color(true, &super::helpers::color_to_hex(color));
                    }
                }
            }
            PropertyFocus::StrokeHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self
                            .editor_state
                            .set_selected_color(false, &super::helpers::color_to_hex(color));
                    }
                }
            }
            _ => {
                if let Ok(value) = draft.trim().parse::<f32>() {
                    let _ = self.editor_state.commit_property_edit(focus, value);
                }
            }
        }
        self.mark_dirty();
    }

    /// VariablesPanel press dispatcher.
    pub(in crate::widget_host) fn dispatch_variables_panel_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        let has_variables = self
            .editor_state
            .doc
            .variables
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if !has_variables {
            return false;
        }
        use op_editor_ui::widgets::variables_panel::{VariablesPanel, VariablesPanelHit};
        use op_editor_ui::widgets::{STATUS_BAR_HEIGHT, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        let vars = VariablesPanel::for_editor(&self.editor_state);
        let intrinsic = vars.intrinsic_height();
        let top_y = if self.editor_state.property_panel_visible() {
            let bottom_pad = STATUS_BAR_HEIGHT + 16.0;
            (viewport_height - bottom_pad - intrinsic).max(TOP_BAR_HEIGHT + 8.0)
        } else {
            TOP_BAR_HEIGHT + 8.0
        };
        let vars_rect = Rect {
            origin: Point2D::new(
                viewport_width - self.editor_state.editor_ui.property_panel_width,
                top_y,
            ),
            size: Point2D::new(self.editor_state.editor_ui.property_panel_width, intrinsic),
        };
        let Some(hit) = vars.hit_test(vars_rect, Point2D::new(x, y)) else {
            return false;
        };
        match hit {
            VariablesPanelHit::Row(idx) => {
                // Resolve (name, kind) off the editor-state var-table.
                use op_editor_ui::scene_vars::VariableKind;
                let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
                let Some((name, kind)) = var_table
                    .variables
                    .get(idx)
                    .map(|v| (v.name.clone(), v.kind))
                else {
                    return true;
                };
                match kind {
                    VariableKind::Color => {
                        self.commit_property_focus_if_any();
                        let _ = self.editor_state.open_color_picker_for_variable(name, y);
                    }
                    VariableKind::Boolean => {
                        // Toggle the boolean value through the
                        // active-theme-aware setter.
                        let current = self
                            .editor_state
                            .resolve_variable(&name)
                            .and_then(|s| match s {
                                jian_ops_schema::variable::VariableScalar::Bool(b) => Some(*b),
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
                        use op_editor_core::editor_ui_state::VariableRowFocus;
                        self.commit_property_focus_if_any();
                        self.commit_variable_row_focus_if_any();
                        // Seed the draft from the resolved scalar.
                        use jian_ops_schema::variable::VariableScalar;
                        let resolved = self.editor_state.resolve_variable(&name).cloned();
                        self.editor_state.ui.property_input_draft = match (&kind, &resolved) {
                            (VariableKind::Number, Some(VariableScalar::Num(n))) => {
                                format!("{n}")
                            }
                            (VariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                            _ => String::new(),
                        };
                        self.editor_state.editor_ui.variable_row_focus = Some(match kind {
                            VariableKind::Number => VariableRowFocus::Number(idx),
                            VariableKind::String => VariableRowFocus::String(idx),
                            _ => return true,
                        });
                        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                    }
                }
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AxisChip(idx) => {
                // Look up the axis name from the active-theme map
                // (BTreeMap iteration order is stable, matching the
                // chip walk order in VariablesPanel).
                let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
                let axis = var_table.active_theme.keys().nth(idx).cloned();
                if let Some(name) = axis {
                    self.commit_property_focus_if_any();
                    if self.editor_state.editor_ui.axis_dropdown_open.as_deref()
                        == Some(name.as_str())
                    {
                        self.editor_state.editor_ui.axis_dropdown_open = None;
                    } else {
                        self.editor_state.editor_ui.axis_dropdown_open = Some(name);
                    }
                }
                self.mark_dirty();
                true
            }
            VariablesPanelHit::AxisDropdownItem { axis, value } => {
                self.commit_property_focus_if_any();
                let snap = self.editor_state.snapshot_for_history();
                if self.editor_state.set_active_axis_value(&axis, &value) {
                    self.editor_state.history_push_past(snap);
                }
                self.editor_state.editor_ui.axis_dropdown_open = None;
                self.mark_dirty();
                true
            }
        }
    }
}

/// Translate a shell-core `ColorTarget` into op-editor-core's.
fn color_target(t: op_editor_core::ColorTarget) -> op_editor_core::ui_draft::ColorTarget {
    match t {
        op_editor_core::ColorTarget::Fill => op_editor_core::ui_draft::ColorTarget::Fill,
        op_editor_core::ColorTarget::Stroke => op_editor_core::ui_draft::ColorTarget::Stroke,
    }
}
