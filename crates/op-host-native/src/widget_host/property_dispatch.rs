//! PropertyPanel action + commit dispatch, split out of `input.rs`
//! to stay under the 800-line cap.
//!
//! Hit-tests run against `EditorState` (chrome / panels) + the
//! layout-resolved `LayoutScene` (canvas); results feed `EditorState`
//! mutators (the host's source of truth).

use super::helpers::parse_hex_color;
use super::WidgetHostNative;
use jian_ops_schema::sizing::SizingKeyword;
use op_editor_core::PropertyFocus;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: op_editor_ui::widgets::PropertyPanelAction,
    ) {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        match action {
            A::SetPropertyTab(tab) => {
                self.editor_state.editor_ui.property_tab = tab;
            }
            A::SetFlexLayout(mode) => {
                self.set_selected_layout_mode(mode);
            }
            A::ToggleSizeFillWidth => {
                self.toggle_selected_sizing(true, SizingKeyword::FillContainer);
            }
            A::ToggleSizeFillHeight => {
                self.toggle_selected_sizing(false, SizingKeyword::FillContainer);
            }
            A::ToggleSizeHugWidth => {
                self.toggle_selected_sizing(true, SizingKeyword::FitContent);
            }
            A::ToggleSizeHugHeight => {
                self.toggle_selected_sizing(false, SizingKeyword::FitContent);
            }
            A::ToggleSizeClipContent => {
                self.toggle_selected_clip_content();
            }
            A::SetLayoutAlign(value) => {
                self.set_selected_layout_align(value);
            }
            A::SetLayoutJustify(value) => {
                self.set_selected_layout_justify(value);
            }
            A::SetLayoutAlignment { justify, align } => {
                self.set_selected_layout_justify(justify);
                self.set_selected_layout_align(align);
            }
            A::CreateComponent => {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    let _ = self.editor_state.create_component_from_node_name(&id);
                }
            }
            A::ToggleFillTypePicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.fill_type_picker_open = !ui.fill_type_picker_open;
                ui.image_fill_popover_open = false;
                ui.font_family_picker_open = false;
                ui.font_weight_picker_open = false;
            }
            A::SetFillType(t) => {
                self.editor_state.set_selected_fill_type(t);
                self.editor_state.editor_ui.fill_type_picker_open = false;
                self.editor_state.editor_ui.image_fill_popover_open = false;
            }
            A::AddFill => {
                let _ = self
                    .editor_state
                    .set_selected_fill_type(op_editor_core::FillType::Solid);
            }
            A::RemoveFill => {
                let _ = self.editor_state.clear_selected_fills();
                self.editor_state.editor_ui.fill_type_picker_open = false;
                self.editor_state.editor_ui.image_fill_popover_open = false;
            }
            A::AddGradientStop => {
                let _ = self.editor_state.add_selected_gradient_stop();
            }
            A::RemoveGradientStop(index) => {
                let _ = self.editor_state.remove_selected_gradient_stop(index);
            }
            A::ToggleImageFillPopover => {
                let ui = &mut self.editor_state.editor_ui;
                ui.image_fill_popover_open = !ui.image_fill_popover_open;
                ui.fill_type_picker_open = false;
                ui.font_family_picker_open = false;
                ui.font_weight_picker_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
            }
            A::CloseImageFillPopover => {
                self.editor_state.editor_ui.image_fill_popover_open = false;
            }
            A::SetImageFillMode(mode) => {
                let _ = self.editor_state.set_selected_image_fill_mode(mode);
            }
            A::SetImageAdjustment { field, value } => {
                self.image_adjustment_drag = Some(field);
                let _ = self
                    .editor_state
                    .set_selected_image_adjustment(field, value);
            }
            A::ResetImageAdjustments => {
                self.image_adjustment_drag = None;
                let _ = self.editor_state.reset_selected_image_adjustments();
            }
            A::OpenSelectedIconPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.icon_picker_open = true;
                ui.icon_picker_replace_selection = true;
                ui.icon_picker_search.clear();
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.font_family_picker_open = false;
                ui.font_weight_picker_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
            }
            A::SetTextAlign(value) => {
                self.set_selected_text_align(value);
            }
            A::SetTextVerticalAlign(value) => {
                self.set_selected_text_vertical_align(value);
            }
            A::SetTextGrowth(value) => {
                self.set_selected_text_growth(value);
            }
            A::ToggleFontFamilyPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.font_family_picker_open = !ui.font_family_picker_open;
                ui.font_weight_picker_open = false;
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
            }
            A::SetFontFamily(choice) => {
                self.set_selected_text_font_family(choice.family());
                self.editor_state.editor_ui.font_family_picker_open = false;
            }
            A::ToggleFontWeightPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.font_weight_picker_open = !ui.font_weight_picker_open;
                ui.font_weight_picker_hover = None;
                ui.font_family_picker_open = false;
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
            }
            A::SetFontWeight(choice) => {
                self.set_selected_font_weight(choice.value());
                self.editor_state.editor_ui.font_weight_picker_open = false;
                self.editor_state.editor_ui.font_weight_picker_hover = None;
            }
            A::TogglePaddingModePopover => {
                let ui = &mut self.editor_state.editor_ui;
                ui.padding_mode_popover_open = !ui.padding_mode_popover_open;
                ui.padding_mode_popover_hover = None;
                ui.font_weight_picker_open = false;
                ui.font_family_picker_open = false;
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
            }
            A::SetPaddingMode(mode) => {
                // Scope the pin to the node it was set for so it can't
                // leak into the next selection.
                let anchor = self.editor_state.selection.anchor.as_str().to_string();
                self.editor_state.editor_ui.padding_edit_mode = Some(mode);
                self.editor_state.editor_ui.padding_edit_mode_anchor = anchor;
                self.editor_state.editor_ui.padding_mode_popover_open = false;
                self.editor_state.editor_ui.padding_mode_popover_hover = None;
                self.editor_state.commit_history();
                let _ = self.editor_state.set_selected_padding_mode_shape(mode);
            }
            A::OpenColorPicker(target) => {
                // Fallback anchor when called outside the press path.
                let _ = self
                    .editor_state
                    .open_color_picker(color_target(target), 0.0);
            }
            A::ToggleExportScalePicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_scale_picker_open = !ui.export_scale_picker_open;
                ui.export_format_picker_open = false;
                ui.font_family_picker_open = false;
                ui.font_weight_picker_open = false;
                ui.export_picker_hover = None;
            }
            A::ToggleExportFormatPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_format_picker_open = !ui.export_format_picker_open;
                ui.export_scale_picker_open = false;
                ui.font_family_picker_open = false;
                ui.font_weight_picker_open = false;
                ui.export_picker_hover = None;
            }
            A::SetExportScale(scale) => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_scale = scale;
                ui.export_scale_picker_open = false;
                ui.export_picker_hover = None;
            }
            A::SetExportFormat(format) => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_format = format;
                ui.export_format_picker_open = false;
                ui.export_picker_hover = None;
            }
            A::ExportImageNow => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::editor_ui_state::FileAction::ExportImageConfirm);
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
            A::FocusEffectParam {
                effect,
                field,
                value,
            } => {
                // Any prior input was committed by the press path's
                // `commit_property_focus_if_any`; seed this param's
                // draft from its current value, caret at the end.
                let ui = &mut self.editor_state.ui;
                ui.property_input_draft = if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    format!("{value}")
                };
                ui.property_caret_pos = ui.property_input_draft.len();
                ui.property_caret_anchor_ms = self.now_ms;
                ui.property_draft_select_all = false;
                self.editor_state.editor_ui.effect_param_focus =
                    Some(op_editor_core::editor_ui_state::EffectParamFocus { effect, field });
            }
            A::OpenEffectColorPicker(index) => {
                let _ = self.editor_state.open_color_picker(
                    op_editor_core::ui_draft::ColorTarget::EffectColor(index),
                    0.0,
                );
            }
            A::PickFillImage => {
                // Queue the file dialog — the desktop runner pops it
                // on the next frame and writes the chosen image into
                // the selected node's primary fill.
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::editor_ui_state::FileAction::PickFillImage);
            }
            // Code panel actions. SelectFramework / Cancel / Copy fully
            // work; Generate / Regenerate raise pending flags + flip the
            // phase (the host codegen session that drains them is P3);
            // Download / ExportBundle raise pending flags drained by the
            // desktop codegen-export pass (rfd save dialog + fs/zip write).
            A::Codegen(codegen_action) => {
                use op_editor_core::codegen::CodegenPhase;
                use op_editor_ui::widgets::property_panel_action::CodegenAction;
                let cg = &mut self.editor_state.codegen;
                match codegen_action {
                    CodegenAction::SelectFramework(fw) => {
                        cg.framework = fw;
                    }
                    CodegenAction::Generate => {
                        cg.pending_generate = true;
                        cg.phase = CodegenPhase::Generating;
                        cg.error = None;
                        cg.code_scroll = 0.0;
                        cg.code_selection = None;
                    }
                    CodegenAction::Regenerate => {
                        cg.pending_regenerate = true;
                        cg.phase = CodegenPhase::Generating;
                        cg.error = None;
                        cg.code_scroll = 0.0;
                        cg.code_selection = None;
                    }
                    CodegenAction::Cancel => {
                        cg.pending_generate = false;
                        cg.pending_regenerate = false;
                        cg.phase = if cg.code.is_empty() {
                            CodegenPhase::Idle
                        } else {
                            CodegenPhase::Complete
                        };
                    }
                    CodegenAction::Copy => {
                        cg.copied_at = Some(self.now_ms);
                        // Push the generated code onto the system
                        // clipboard via the same queue the MCP-config
                        // copy uses; the desktop runner drains
                        // `chat.pending_copy_text` into the OS clipboard.
                        self.editor_state.chat.queue_copy_text(cg.code.clone());
                    }
                    CodegenAction::Download => {
                        cg.pending_download = true;
                    }
                    CodegenAction::ExportBundle => {
                        cg.pending_export_bundle = true;
                    }
                    CodegenAction::ScrollFrameworksLeft | CodegenAction::ScrollFrameworksRight => {
                        let pw = self.editor_state.editor_ui.property_panel_width;
                        let max =
                            op_editor_ui::widgets::property_panel_code::framework_row_overflow(pw);
                        let step = 100.0;
                        let cg = &mut self.editor_state.codegen;
                        cg.framework_scroll =
                            if matches!(codegen_action, CodegenAction::ScrollFrameworksLeft) {
                                (cg.framework_scroll - step).clamp(0.0, max)
                            } else {
                                (cg.framework_scroll + step).clamp(0.0, max)
                            };
                    }
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
                self.editor_state.editor_ui.export_dialog_hover = None;
            }
            Some(ExportDialogHit::Export) => {
                self.editor_state.editor_ui.export_dialog_open = false;
                self.editor_state.editor_ui.export_dialog_hover = None;
                self.editor_state.editor_ui.pending_file_action =
                    Some(FileAction::ExportImageConfirm);
            }
            None => {
                if !dlg.contains(point) {
                    // Outside click — dismiss like Cancel.
                    self.editor_state.editor_ui.export_dialog_open = false;
                    self.editor_state.editor_ui.export_dialog_hover = None;
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
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::DropZone => {
                self.editor_state.editor_ui.pending_file_action = Some(FileAction::ImportFigma);
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
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

    /// Commit a pending effect-parameter edit (Effects section's
    /// editable value box). Parses the shared draft and writes it
    /// via `SetEffectParam`; a non-numeric draft is discarded.
    pub(in crate::widget_host) fn commit_effect_param_focus_if_any(&mut self) {
        let Some(focus) = self.editor_state.editor_ui.effect_param_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = std::mem::take(&mut self.editor_state.ui.property_input_draft);
        if let Ok(value) = draft.trim().parse::<f32>() {
            if value.is_finite() {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    self.editor_state.commit_history();
                    let _ =
                        self.editor_state
                            .apply(op_editor_core::EditorCommand::SetEffectParam {
                                node_id: id,
                                index: focus.effect as u32,
                                field: focus.field,
                                value,
                            });
                }
            }
        }
        self.mark_dirty();
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        // Commit any pending variable-row / effect-param edit first.
        self.commit_variable_row_focus_if_any();
        self.commit_effect_param_focus_if_any();
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
            PropertyFocus::GradientStopHex(index) => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        // The input pill never paints alpha digits,
                        // so re-attach the stop's existing alpha here
                        // — a transparent stop must stay transparent
                        // after the user edits its RGB.
                        let existing_alpha = self
                            .editor_state
                            .selected_node()
                            .and_then(|n| current_stop_alpha(n, index))
                            .unwrap_or(1.0);
                        let with_alpha = op_editor_ui::Color {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                            a: existing_alpha,
                        };
                        let _ = self.editor_state.set_selected_gradient_stop_hex(
                            index,
                            &super::helpers::color_to_hex_with_alpha(with_alpha),
                        );
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
        _viewport_height: f32,
    ) -> bool {
        // Lockstep with paint: auto-shown variables yield to the
        // PropertyPanel. The toolbar-opened variables manager is a
        // floating modal handled by `dispatch_variables_modal_press`.
        if self.editor_state.property_panel_visible() {
            return false;
        }
        let has_variable_table = self
            .editor_state
            .doc
            .variables
            .as_ref()
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let show_variables = has_variable_table && !self.editor_state.property_panel_visible();
        if !show_variables {
            return false;
        }
        use op_editor_ui::widgets::variables_panel::VariablesPanel;
        use op_editor_ui::widgets::TOP_BAR_HEIGHT;
        use op_editor_ui::{Point2D, Rect};
        let vars = VariablesPanel::for_editor(&self.editor_state);
        let intrinsic = vars.intrinsic_height();
        let top_y = TOP_BAR_HEIGHT + 8.0;
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
        self.dispatch_variables_hit(hit, y)
    }

    /// Floating VariablesModal press dispatcher.
    pub(in crate::widget_host) fn dispatch_variables_modal_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.variables_panel_open {
            return false;
        }
        use op_editor_ui::widgets::{VariablesModal, VariablesModalHit};
        let modal = VariablesModal::for_editor(&self.editor_state);
        let rect = modal.rect(viewport_width, viewport_height);
        match modal.hit_test(rect, op_editor_ui::Point2D::new(x, y)) {
            VariablesModalHit::Close | VariablesModalHit::Outside => {
                let ui = &mut self.editor_state.editor_ui;
                ui.variables_panel_open = false;
                ui.variables_panel_hover = None;
                ui.axis_dropdown_open = None;
                self.mark_dirty();
                true
            }
            VariablesModalHit::AddVariable | VariablesModalHit::HeaderAdd => {
                self.create_default_variable_from_modal();
                true
            }
            VariablesModalHit::PresetMenu | VariablesModalHit::Inside => true,
            VariablesModalHit::Row(idx) => {
                self.dispatch_variable_row_hit(idx, y);
                true
            }
            VariablesModalHit::AxisChip(idx) => {
                self.dispatch_variable_axis_chip_hit(idx);
                true
            }
            VariablesModalHit::AxisDropdownItem { axis, value } => {
                self.dispatch_variable_axis_dropdown_hit(axis, value);
                true
            }
        }
    }

    fn dispatch_variables_hit(
        &mut self,
        hit: op_editor_ui::widgets::variables_panel::VariablesPanelHit,
        y: f32,
    ) -> bool {
        use op_editor_ui::widgets::variables_panel::VariablesPanelHit;
        match hit {
            VariablesPanelHit::Row(idx) => {
                self.dispatch_variable_row_hit(idx, y);
                true
            }
            VariablesPanelHit::AxisChip(idx) => {
                self.dispatch_variable_axis_chip_hit(idx);
                true
            }
            VariablesPanelHit::AxisDropdownItem { axis, value } => {
                self.dispatch_variable_axis_dropdown_hit(axis, value);
                true
            }
        }
    }

    fn dispatch_variable_row_hit(&mut self, idx: usize, y: f32) {
        // Resolve (name, kind) off the editor-state var-table.
        use op_editor_ui::scene_vars::VariableKind;
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let Some((name, kind)) = var_table
            .variables
            .get(idx)
            .map(|v| (v.name.clone(), v.kind))
        else {
            return;
        };
        match kind {
            VariableKind::Color => {
                self.commit_property_focus_if_any();
                let _ = self.editor_state.open_color_picker_for_variable(name, y);
            }
            VariableKind::Boolean => {
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
                use jian_ops_schema::variable::VariableScalar;
                use op_editor_core::editor_ui_state::VariableRowFocus;
                self.commit_property_focus_if_any();
                self.commit_variable_row_focus_if_any();
                let resolved = self.editor_state.resolve_variable(&name).cloned();
                self.editor_state.ui.property_input_draft = match (&kind, &resolved) {
                    (VariableKind::Number, Some(VariableScalar::Num(n))) => format!("{n}"),
                    (VariableKind::String, Some(VariableScalar::Str(s))) => s.clone(),
                    _ => String::new(),
                };
                self.editor_state.editor_ui.variable_row_focus = Some(match kind {
                    VariableKind::Number => VariableRowFocus::Number(idx),
                    VariableKind::String => VariableRowFocus::String(idx),
                    _ => return,
                });
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            }
        }
        self.mark_dirty();
    }

    fn dispatch_variable_axis_chip_hit(&mut self, idx: usize) {
        let var_table = op_pen_loader::editor_state_var_table(&self.editor_state);
        let axis = var_table.active_theme.keys().nth(idx).cloned();
        if let Some(name) = axis {
            self.commit_property_focus_if_any();
            if self.editor_state.editor_ui.axis_dropdown_open.as_deref() == Some(name.as_str()) {
                self.editor_state.editor_ui.axis_dropdown_open = None;
            } else {
                self.editor_state.editor_ui.axis_dropdown_open = Some(name);
            }
        }
        self.mark_dirty();
    }

    fn dispatch_variable_axis_dropdown_hit(&mut self, axis: String, value: String) {
        self.commit_property_focus_if_any();
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.set_active_axis_value(&axis, &value) {
            self.editor_state.history_push_past(snap);
        }
        self.editor_state.editor_ui.axis_dropdown_open = None;
        self.mark_dirty();
    }

    fn create_default_variable_from_modal(&mut self) {
        use jian_ops_schema::variable::{VariableKind, VariableScalar};
        let existing = self.editor_state.doc.variables.as_ref();
        let mut idx = 1;
        let name = loop {
            let candidate = format!("variable-{idx}");
            if existing.is_none_or(|vars| !vars.contains_key(&candidate)) {
                break candidate;
            }
            idx += 1;
        };
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.create_variable(
            &name,
            VariableKind::Color,
            VariableScalar::Str("#000000".into()),
        ) {
            self.editor_state.history_push_past(snap);
        }
        self.mark_dirty();
    }
}

/// Read the live alpha of gradient stop `index` on `node`, parsed
/// out of the canonical hex (8-char `#RRGGBBAA`). `None` when the
/// first fill isn't a gradient or the stop hex omits alpha — the
/// caller defaults to `1.0` in that case so opaque stops stay
/// opaque through an RGB edit.
fn current_stop_alpha(node: &jian_ops_schema::node::PenNode, index: usize) -> Option<f32> {
    use jian_ops_schema::style::PenFill;
    let first = op_editor_core::fills::node_fills(node).and_then(|f| f.first())?;
    let stops = match first {
        PenFill::LinearGradient(b) => &b.stops,
        PenFill::RadialGradient(b) => &b.stops,
        _ => return None,
    };
    let hex = &stops.get(index)?.color;
    Some(op_editor_core::parse_hex_alpha(hex))
}

/// Translate a shell-core `ColorTarget` into op-editor-core's.
fn color_target(t: op_editor_core::ColorTarget) -> op_editor_core::ui_draft::ColorTarget {
    match t {
        op_editor_core::ColorTarget::Fill => op_editor_core::ui_draft::ColorTarget::Fill,
        op_editor_core::ColorTarget::Stroke => op_editor_core::ui_draft::ColorTarget::Stroke,
        op_editor_core::ColorTarget::GradientStop(i) => {
            op_editor_core::ui_draft::ColorTarget::GradientStop(i)
        }
        op_editor_core::ColorTarget::EffectColor(i) => {
            op_editor_core::ui_draft::ColorTarget::EffectColor(i)
        }
    }
}
