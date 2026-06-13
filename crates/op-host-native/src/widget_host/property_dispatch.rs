//! PropertyPanel action + commit dispatch, split out of `input.rs`
//! to stay under the 800-line cap.
//!
//! Hit-tests run against `EditorState` (chrome / panels) + the
//! layout-resolved `LayoutScene` (canvas); results feed `EditorState`
//! mutators (the host's source of truth).

use super::helpers::parse_hex_color;
use super::WidgetHostNative;
use jian_ops_schema::sizing::SizingKeyword;
use jian_ops_schema::variable::VariableKind;
use op_editor_core::PropertyFocus;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: op_editor_ui::widgets::PropertyPanelAction,
    ) {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        // Instance / component lifecycle actions act on the REAL Ref
        // node, so they dispatch BEFORE the instance-write redirect
        // scope below swaps in the merged display node.
        match action {
            A::GoToComponent => {
                if let Some(jian_ops_schema::node::PenNode::Ref(r)) =
                    self.editor_state.selected_node()
                {
                    let master = op_editor_core::NodeId::new(r.target.clone());
                    // Cross-page master: selection resolves against the
                    // ACTIVE page only, so switch pages first.
                    if let Some(pages) = self.editor_state.doc.pages.as_ref() {
                        let target_page = pages.iter().position(|page| {
                            op_editor_core::walkers::find_node(&page.children, &master).is_some()
                        });
                        if let Some(idx) = target_page {
                            if idx != self.editor_state.ui.active_page_index {
                                let _ = self.editor_state.set_active_page(idx);
                            }
                        }
                    }
                    self.editor_state.set_single_selection(master);
                }
                self.mark_dirty();
                return;
            }
            A::DetachInstance | A::DetachComponent => {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    let _ = self.editor_state.detach_component(&id);
                }
                self.mark_dirty();
                return;
            }
            _ => {}
        }
        // CHOKE POINT (GAP #10): when the anchor is a Ref, swap in
        // the merged display node so every anchor-keyed mutator below
        // writes into it; `finish_instance_write` then routes the
        // diff onto the RefNode (direct props) / descendants[target]
        // (overrides). See op-editor-core/src/instance_override.rs.
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
        match action {
            // Dispatched in the pre-scope match above; unreachable here.
            A::GoToComponent | A::DetachInstance | A::DetachComponent => {}
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
                ui.toggle_fill_type_picker();
                ui.image_fill_popover_open = false;
                ui.close_font_picker();
                ui.font_weight_picker_open = false;
                ui.property_color_variable_picker_open = None;
            }
            A::SetFillType(t) => {
                self.editor_state.set_selected_fill_type(t);
                self.editor_state.editor_ui.close_fill_type_picker();
                self.editor_state.editor_ui.image_fill_popover_open = false;
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
            }
            A::AddFill => {
                let _ = self
                    .editor_state
                    .set_selected_fill_type(op_editor_core::FillType::Solid);
            }
            A::RemoveFill => {
                let _ = self.editor_state.clear_selected_fills();
                self.editor_state.editor_ui.close_fill_type_picker();
                self.editor_state.editor_ui.image_fill_popover_open = false;
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
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
                ui.close_fill_type_picker();
                ui.close_font_picker();
                ui.font_weight_picker_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
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
                ui.open_icon_picker(true);
                ui.close_fill_type_picker();
                ui.image_fill_popover_open = false;
                ui.close_font_picker();
                ui.font_weight_picker_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
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
                let opening = !self.editor_state.editor_ui.font_picker.open;
                if opening {
                    // Enumerate installed families on first open (TS
                    // requests Local Font Access inside the click
                    // gesture for the same reason).
                    self.ensure_system_fonts_loaded();
                }
                let ui = &mut self.editor_state.editor_ui;
                ui.toggle_font_picker();
                ui.font_weight_picker_open = false;
                ui.close_fill_type_picker();
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
            }
            A::SetFontFamilyIndex(index) => {
                if let Some(family) = self.font_picker_family_at(index) {
                    self.set_selected_text_font_family(&family);
                }
                self.close_font_picker();
            }
            A::ToggleImageSearchPopover => {
                self.toggle_image_search_popover();
            }
            A::ToggleImageGeneratePopover => {
                self.toggle_image_generate_popover();
            }
            A::RunImageSearch => {
                self.run_image_search();
            }
            A::SelectImageSearchResult(index) => {
                self.select_image_search_result(index);
            }
            A::RunImageGenerate => {
                self.run_image_generate();
            }
            A::ApplyGeneratedImage => {
                self.apply_generated_image();
            }
            A::RetryImageGenerate => {
                self.retry_image_generate();
            }
            A::OpenImageGenSettings => {
                self.open_image_gen_settings();
            }
            A::RelinkImage => {
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::editor_ui_state::FileAction::RelinkImage);
            }
            A::ToggleFontWeightPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.font_weight_picker_open = !ui.font_weight_picker_open;
                ui.font_weight_picker_hover = None;
                ui.close_font_picker();
                ui.close_fill_type_picker();
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
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
                ui.close_font_picker();
                ui.close_fill_type_picker();
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
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
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
                let _ = self
                    .editor_state
                    .open_color_picker(color_target(target), 0.0);
            }
            A::ToggleColorVariablePicker(target) => {
                let target = color_target(target);
                let ui = &mut self.editor_state.editor_ui;
                ui.property_color_variable_picker_open =
                    if ui.property_color_variable_picker_open == Some(target) {
                        None
                    } else {
                        Some(target)
                    };
                ui.close_fill_type_picker();
                ui.image_fill_popover_open = false;
                ui.close_font_picker();
                ui.font_weight_picker_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
            }
            A::BindColorVariable { target, index } => {
                if let Some(name) = color_variable_name_at(&self.editor_state, index) {
                    self.editor_state.commit_history();
                    let _ = self
                        .editor_state
                        .bind_selected_color_variable(color_target(target), &name);
                }
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
            }
            A::UnbindColorVariable(target) => {
                self.editor_state.commit_history();
                let _ = self
                    .editor_state
                    .unbind_selected_color_variable(color_target(target));
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
            }
            A::ToggleExportScalePicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_scale_picker_open = !ui.export_scale_picker_open;
                ui.export_format_picker_open = false;
                ui.close_font_picker();
                ui.font_weight_picker_open = false;
                ui.export_picker_hover = None;
                ui.property_color_variable_picker_open = None;
            }
            A::ToggleExportFormatPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_format_picker_open = !ui.export_format_picker_open;
                ui.export_scale_picker_open = false;
                ui.close_font_picker();
                ui.font_weight_picker_open = false;
                ui.export_picker_hover = None;
                ui.property_color_variable_picker_open = None;
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
                let initial = if value.fract() == 0.0 {
                    format!("{}", value as i64)
                } else {
                    format!("{value}")
                };
                ui.property_input.set_text(initial.clone());
                ui.property_input.touch(self.now_ms);
                ui.property_input_draft = initial;
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
            // Code panel actions. SelectFramework / Copy fully work;
            // Generate / Regenerate raise pending flags + flip the
            // phase (drained by the desktop codegen session); Cancel
            // flips the phase AND raises a pending-cancel intent that
            // aborts the in-flight worker; Download / ExportBundle raise
            // pending flags drained by the desktop codegen-export pass
            // (rfd save dialog + fs/zip write).
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
                        cg.code_scroll.offset = 0.0;
                        cg.code_selection = None;
                    }
                    CodegenAction::Regenerate => {
                        cg.pending_regenerate = true;
                        cg.phase = CodegenPhase::Generating;
                        cg.error = None;
                        cg.code_scroll.offset = 0.0;
                        cg.code_selection = None;
                    }
                    CodegenAction::Cancel => {
                        cg.pending_generate = false;
                        cg.pending_regenerate = false;
                        // Raise the cancel intent for the desktop runner —
                        // it aborts the in-flight worker (shared AtomicBool)
                        // so the run actually stops instead of streaming on
                        // and resurrecting the panel (TS: abort()).
                        cg.pending_cancel = true;
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
                        let delta = if matches!(codegen_action, CodegenAction::ScrollFrameworksLeft)
                        {
                            -step
                        } else {
                            step
                        };
                        cg.framework_scroll.scroll_by(delta, max, 0.0);
                    }
                }
            }
        }
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
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
                // No control hit — blank press (inside chrome or
                // outside the dialog): blur the chrome text inputs.
                self.blur_text_inputs_on_blank_press();
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
            FigmaImportHit::Close => {
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::Outside => {
                // Outside click — blank press: dismiss + blur inputs.
                self.blur_text_inputs_on_blank_press();
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::DropZone => {
                self.editor_state.editor_ui.pending_file_action = Some(FileAction::ImportFigma);
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::Inside => {
                // Blank press on modal chrome — blur chrome inputs.
                self.blur_text_inputs_on_blank_press();
            }
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
        use op_editor_ui::widgets::file_menu::{FileMenu, FileMenuChoice, MenuHit};
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
        let point = op_editor_ui::Point2D::new(x, y);
        match menu.hit(menu_rect, point) {
            MenuHit::Row(row) => {
                let Some(choice) = menu.choice_for_row(row) else {
                    return;
                };
                self.editor_state.editor_ui.pending_file_action = Some(match choice {
                    FileMenuChoice::NewFile => FileAction::New,
                    FileMenuChoice::OpenFile => FileAction::Open,
                    FileMenuChoice::Save => FileAction::Save,
                    FileMenuChoice::SaveAs => FileAction::SaveAs,
                    FileMenuChoice::ExportImage => FileAction::ExportImage,
                    FileMenuChoice::OpenRecent(i) => FileAction::OpenRecent(i),
                    FileMenuChoice::ClearRecent => FileAction::ClearRecent,
                });
                self.editor_state.editor_ui.file_menu_open = false;
                self.editor_state.editor_ui.file_menu.hover = None;
                self.mark_dirty();
            }
            MenuHit::Inside => {}
            MenuHit::Outside => {
                // Miss — the dismissing click is a blank press.
                self.blur_text_inputs_on_blank_press();
                self.editor_state.editor_ui.file_menu_open = false;
                self.editor_state.editor_ui.file_menu.hover = None;
                self.mark_dirty();
            }
        }
    }

    /// Commit a pending effect-parameter edit (Effects section's
    /// editable value box). Parses the shared draft and writes it
    /// via `SetEffectParam`; a non-numeric draft is discarded.
    pub(in crate::widget_host) fn commit_effect_param_focus_if_any(&mut self) {
        let Some(focus) = self.editor_state.editor_ui.effect_param_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = self.editor_state.ui.property_input.text().to_owned();
        self.editor_state.ui.property_input.set_text("");
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        if let Ok(value) = draft.trim().parse::<f32>() {
            if value.is_finite() {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    // Instance-write redirect (GAP #10) — see
                    // `apply_property_action` for the choke-point note.
                    let instance_scope = self.editor_state.begin_instance_write_for_anchor();
                    self.editor_state.commit_history();
                    let _ =
                        self.editor_state
                            .apply(op_editor_core::EditorCommand::SetEffectParam {
                                node_id: id,
                                index: focus.effect as u32,
                                field: focus.field,
                                value,
                            });
                    if let Some(scope) = instance_scope {
                        self.editor_state.finish_instance_write(scope);
                    }
                }
            }
        }
        self.mark_dirty();
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        // Commit any pending variable-row / effect-param edit first.
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.commit_effect_param_focus_if_any();
        let Some(focus) = self.editor_state.ui.property_focus.take() else {
            return;
        };
        self.editor_state.ui.property_draft_select_all = false;
        let draft = self.editor_state.ui.property_input.text().to_owned();
        self.editor_state.ui.property_input.set_text("");
        self.editor_state.ui.property_input_draft.clear();
        self.editor_state.ui.property_caret_pos = 0;
        // Instance-write redirect (GAP #10) — see `apply_property_action`
        // for the choke-point note.
        let before = self.editor_state.snapshot_for_history();
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
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
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
        }
        if self.editor_state.snapshot_for_history() != before {
            self.editor_state.history_push_past(before);
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

fn color_variable_name_at(state: &op_editor_core::EditorState, index: usize) -> Option<String> {
    state
        .doc
        .variables
        .as_ref()?
        .iter()
        .filter(|(_, def)| matches!(def.kind, VariableKind::Color))
        .nth(index)
        .map(|(name, _)| name.clone())
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
