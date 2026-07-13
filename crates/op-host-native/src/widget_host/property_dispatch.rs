//! PropertyPanel action + commit dispatch, split out of `input.rs`
//! to stay under the 800-line cap.
//!
//! Hit-tests run against `EditorState` (chrome / panels) + the
//! layout-resolved `LayoutScene` (canvas); results feed `EditorState`
//! mutators (the host's source of truth).

#[path = "property_input_dispatch.rs"]
mod property_input_dispatch;

use super::WidgetHostNative;
use jian_ops_schema::sizing::SizingKeyword;
use jian_ops_schema::variable::VariableKind;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: op_editor_ui::widgets::PropertyPanelAction,
    ) {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        // A sizing keyword toggle may temporarily swap an instance's merged
        // display node into the document below. Capture the real canvas size
        // before that scope starts so turning Fill/Hug off freezes exactly
        // what the user sees, without rebuilding a scene from the temporary
        // instance-write representation.
        let resolved_sizing_fallback = match action {
            A::ToggleSizeFillWidth | A::ToggleSizeHugWidth => {
                self.resolved_selected_sizing_axis(true)
            }
            A::ToggleSizeFillHeight | A::ToggleSizeHugHeight => {
                self.resolved_selected_sizing_axis(false)
            }
            _ => None,
        };
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
                self.toggle_selected_sizing(
                    true,
                    SizingKeyword::FillContainer,
                    resolved_sizing_fallback,
                );
            }
            A::ToggleSizeFillHeight => {
                self.toggle_selected_sizing(
                    false,
                    SizingKeyword::FillContainer,
                    resolved_sizing_fallback,
                );
            }
            A::ToggleSizeHugWidth => {
                self.toggle_selected_sizing(
                    true,
                    SizingKeyword::FitContent,
                    resolved_sizing_fallback,
                );
            }
            A::ToggleSizeHugHeight => {
                self.toggle_selected_sizing(
                    false,
                    SizingKeyword::FitContent,
                    resolved_sizing_fallback,
                );
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
            A::ToggleFillTypePicker(index) => {
                let ui = &mut self.editor_state.editor_ui;
                ui.toggle_fill_type_picker_for(index);
                ui.image_fill_popover_open = false;
                ui.close_font_picker();
                ui.font_weight_picker_open = false;
                ui.property_color_variable_picker_open = None;
            }
            A::SetFillType { index, fill_type } => {
                self.editor_state
                    .set_selected_fill_type_at(index, fill_type);
                self.editor_state.editor_ui.close_fill_type_picker();
                self.editor_state.editor_ui.image_fill_popover_open = false;
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
            }
            A::AddFill => {
                let _ = self.editor_state.add_selected_fill();
            }
            A::MoveFill { from, to } => {
                let _ = self.editor_state.move_selected_fill(from, to);
            }
            A::RemoveFill(index) => {
                let _ = self.editor_state.remove_selected_fill(index);
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
            A::ImportFont => {
                // Raise a pending request; the desktop host drains it,
                // opens the native font-file dialog, and registers the
                // chosen file (FontStore lives desktop-side). Keep the
                // picker open so the newly imported family appears.
                self.editor_state.editor_ui.pending_font_import = true;
                self.mark_dirty();
            }
            A::RemoveImportedFont(index) => {
                // Resolve the family against the SAME entries list, then
                // hand it to the desktop host to drop from FontStore.
                if let Some(family) = self.font_picker_family_at(index) {
                    self.editor_state.editor_ui.pending_font_remove = Some(family);
                    self.mark_dirty();
                }
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
                ui.stroke_mode_popover_open = false;
                ui.stroke_mode_popover_hover = None;
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
            A::ToggleStrokeModePopover => {
                let ui = &mut self.editor_state.editor_ui;
                ui.stroke_mode_popover_open = !ui.stroke_mode_popover_open;
                ui.stroke_mode_popover_hover = None;
                ui.padding_mode_popover_open = false;
                ui.padding_mode_popover_hover = None;
                ui.font_weight_picker_open = false;
                ui.close_font_picker();
                ui.close_fill_type_picker();
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
            }
            A::SetStrokeMode(mode) => {
                let anchor = self.editor_state.selection.anchor.as_str().to_string();
                self.editor_state.editor_ui.stroke_edit_mode = Some(mode);
                self.editor_state.editor_ui.stroke_edit_mode_anchor = anchor;
                self.editor_state.editor_ui.stroke_mode_popover_open = false;
                self.editor_state.editor_ui.stroke_mode_popover_hover = None;
                self.editor_state.commit_history();
                let _ = self.editor_state.set_selected_stroke_mode_shape(mode);
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
            A::OpenFillColorPicker(index) => {
                // Fallback anchor when called outside the press path.
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
                let _ = self.editor_state.open_color_picker_for_fill(
                    op_editor_core::ui_draft::ColorTarget::Fill,
                    index,
                    0.0,
                );
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
                self.editor_state.editor_ui.toggle_effect_add_picker();
            }
            A::AddDropShadowEffect => {
                self.editor_state.add_drop_shadow_to_selected();
                self.editor_state.editor_ui.close_effect_add_picker();
            }
            A::AddLayerBlur => {
                self.editor_state.add_layer_blur_to_selected();
                self.editor_state.editor_ui.close_effect_add_picker();
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
            A::ToggleWidgetChecked(new_value) => {
                self.editor_state.commit_history();
                let _ = self.editor_state.set_selected_widget_checked(new_value);
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
