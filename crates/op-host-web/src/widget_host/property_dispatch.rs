//! Web `apply_property_action` — PropertyPanel button / dropdown
//! action dispatch. Split out of `press.rs` to keep that file under
//! the 800-line cap (mirrors the native host's `property_dispatch.rs`).

use super::WidgetHost;
use jian_ops_schema::sizing::SizingKeyword;
use jian_ops_schema::variable::VariableKind;
use op_editor_core::{EffectField, PropertyFocus};
use op_editor_ui::util::{
    color_to_hex, color_to_hex_with_alpha, format_panel_number, parse_hex_color,
};
use op_editor_ui::{widgets::PropertyPanel, Color};

fn effect_param_snapshot_value(
    state: &op_editor_core::EditorState,
    effect: usize,
    field: EffectField,
) -> Option<f32> {
    PropertyPanel::for_selection(state)?
        .snapshot
        .effects
        .get(effect)
        .map(|summary| summary.param_value(field))
}

fn format_effect_param_value(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

impl WidgetHost {
    /// Swap a synced document into the live editor state via the shared, tested
    /// `EditorState::replace_document`, then `mark_dirty()` so the next paint
    /// re-derives the layout scene from the NEW document. Without the
    /// `mark_dirty()` the web host's `refresh_layout_scene()` is a no-op (the
    /// dirty flag isn't set), so the repaint would present the STALE scene yet
    /// succeed — and `WebSyncClient::sync` would then commit the version against
    /// a stale paint. Used by the opt-in `live-sync` glue. Lives here (not
    /// `widget_host.rs`) to keep that spine under the 800-line cap.
    /// `undoable` = the sync client was already initialized, i.e. this
    /// is NOT the mount-time first pull (starter → daemon doc), so the
    /// external write (AI turn / MCP client) lands as one undo step.
    #[cfg(feature = "canvaskit")]
    pub(crate) fn replace_document_from_sync(
        &mut self,
        doc: op_editor_core::PenDocument,
        undoable: bool,
    ) {
        if undoable {
            self.editor_state.replace_document_with_undo(doc);
        } else {
            self.editor_state.replace_document(doc);
        }
        // A whole-document replacement can restart the revision at 0 / page 0,
        // aliasing the previous document's LayerPanel row-model-cache key — rotate
        // the owner so the next owned paint resolve rebuilds the rows.
        self.force_rotate_layer_panel_owner();
        self.mark_dirty();
    }

    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: op_editor_ui::widgets::PropertyPanelAction,
    ) {
        use op_editor_ui::widgets::PropertyPanelAction as A;
        match action {
            A::GoToComponent => {
                if let Some(jian_ops_schema::node::PenNode::Ref(reference)) =
                    self.editor_state.selected_node()
                {
                    let master = op_editor_core::NodeId::new(reference.target.clone());
                    if let Some(pages) = self.editor_state.doc.pages.as_ref() {
                        if let Some(page_index) = pages.iter().position(|page| {
                            op_editor_core::walkers::find_node(&page.children, &master).is_some()
                        }) {
                            if page_index != self.editor_state.ui.active_page_index {
                                let _ = self.editor_state.set_active_page(page_index);
                            }
                        }
                    }
                    self.editor_state.set_single_selection(master);
                }
                self.mark_dirty();
                return;
            }
            A::DetachComponent | A::DetachInstance => {
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    let _ = self.editor_state.detach_component(&id);
                }
                self.mark_dirty();
                return;
            }
            _ => {}
        }
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
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
            A::ToggleFillTypePicker(index) => {
                let ui = &mut self.editor_state.editor_ui;
                ui.toggle_fill_type_picker_for(index);
                ui.image_fill_popover_open = false;
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
                // Property-panel icon section → replace-selection
                // picker (mirrors the native host's arm).
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
            A::ToggleExportScalePicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_scale_picker_open = !ui.export_scale_picker_open;
                ui.export_format_picker_open = false;
                ui.export_picker_hover = None;
                ui.property_color_variable_picker_open = None;
            }
            A::ToggleExportFormatPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_format_picker_open = !ui.export_format_picker_open;
                ui.export_scale_picker_open = false;
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
            A::OpenColorPicker(target) => {
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
                let _ = self
                    .editor_state
                    .open_color_picker(color_target(target), 0.0);
            }
            A::OpenFillColorPicker(index) => {
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
                self.commit_property_focus_if_any();
                let value =
                    effect_param_snapshot_value(&self.editor_state, effect, field).unwrap_or(value);
                let initial = format_effect_param_value(value);
                let ui = &mut self.editor_state.ui;
                ui.property_input.set_text(initial.clone());
                ui.property_input.touch(self.now_ms);
                ui.property_input_draft = initial;
                ui.property_caret_pos = ui.property_input.caret();
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
                // Queue the browser file picker — `dom_io` drains
                // this flag after the event handler releases the host
                // borrow and writes the chosen image into the selected
                // node's primary fill.
                self.editor_state.editor_ui.pending_file_action =
                    Some(op_editor_core::editor_ui_state::FileAction::PickFillImage);
            }
            A::ToggleFontFamilyPicker => {
                // The web entry drains Local Font Access after this
                // press; until the browser resolves / rejects that
                // permission flow the picker paints the bundled group
                // plus the TS fallback system list.
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
                let family = {
                    let ui = &self.editor_state.editor_ui;
                    op_editor_ui::widgets::property_panel_typography::font_picker_entries(
                        &ui.imported_font_families,
                        &ui.system_font_families,
                        &ui.font_picker_search,
                    )
                    .get(index)
                    .map(|e| e.family.to_string())
                };
                if let Some(family) = family {
                    let id = self.editor_state.selection.anchor.clone();
                    if id.is_real() && !family.trim().is_empty() {
                        self.editor_state.commit_history();
                        let _ = self.editor_state.apply(
                            op_editor_core::EditorCommand::SetNodeLayoutProp {
                                node_id: id,
                                property: "fontFamily".to_string(),
                                value: op_editor_core::LayoutPropValue::Keyword(family),
                            },
                        );
                    }
                }
                self.editor_state.editor_ui.close_font_picker();
            }
            A::ImportFont => {
                // Raise a pending request; `web_fonts::drain_font_requests`
                // (post-press / mount) opens the hidden file input, reads the
                // chosen `.ttf` / `.otf`, registers it through the CanvasKit
                // family registry, and persists it to IndexedDB. Keep the
                // picker open so the new family appears once it lands.
                self.editor_state.editor_ui.pending_font_import = true;
            }
            A::RemoveImportedFont(index) => {
                // Resolve the family against the SAME entries list the picker
                // painted / hit-tested, then hand it to the drain to drop from
                // the CanvasKit registry + IndexedDB (mirrors the native arm).
                let family = {
                    let ui = &self.editor_state.editor_ui;
                    op_editor_ui::widgets::property_panel_typography::font_picker_entries(
                        &ui.imported_font_families,
                        &ui.system_font_families,
                        &ui.font_picker_search,
                    )
                    .get(index)
                    .map(|e| e.family.to_string())
                };
                if let Some(family) = family {
                    self.editor_state.editor_ui.pending_font_remove = Some(family);
                }
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
                let id = self.editor_state.selection.anchor.clone();
                if id.is_real() {
                    self.editor_state.commit_history();
                    let _ = self.editor_state.commit_property_edit(
                        op_editor_core::PropertyFocus::FontWeight,
                        choice.value() as f32,
                    );
                }
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
                // Scope the pin to the node it was set for (no leak into
                // the next selection).
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
            A::Codegen(codegen_action) => {
                self.apply_codegen_action(codegen_action);
            }
            _ => {}
        }
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
        }
        self.mark_dirty();
    }

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
        let before = self.editor_state.snapshot_for_history();
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
        match focus {
            PropertyFocus::FillHex(index) => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let hex = color_to_hex(color);
                        if index == 0 {
                            let _ = self.editor_state.set_selected_color(true, &hex);
                        } else {
                            let _ = self.editor_state.set_selected_fill_hex_at(index, &hex);
                        }
                    }
                }
            }
            PropertyFocus::StrokeHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self
                            .editor_state
                            .set_selected_color(false, &color_to_hex(color));
                    }
                }
            }
            PropertyFocus::GradientStopHex(index) => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let existing_alpha = self
                            .editor_state
                            .selected_node()
                            .and_then(|n| current_stop_alpha(n, index))
                            .unwrap_or(1.0);
                        let with_alpha = Color {
                            r: color.r,
                            g: color.g,
                            b: color.b,
                            a: existing_alpha,
                        };
                        let _ = self.editor_state.set_selected_gradient_stop_hex(
                            index,
                            &color_to_hex_with_alpha(with_alpha),
                        );
                    }
                }
            }
            PropertyFocus::WidgetPlaceholder => {
                let _ = self.editor_state.set_selected_widget_text(
                    op_editor_core::WidgetTextField::Placeholder,
                    draft.trim(),
                );
            }
            PropertyFocus::WidgetValue => {
                let _ = self
                    .editor_state
                    .set_selected_widget_text(op_editor_core::WidgetTextField::Value, draft.trim());
            }
            PropertyFocus::WidgetLabel => {
                let _ = self
                    .editor_state
                    .set_selected_widget_text(op_editor_core::WidgetTextField::Label, draft.trim());
            }
            PropertyFocus::WidgetLeadingIcon => {
                let _ = self.editor_state.set_selected_widget_text(
                    op_editor_core::WidgetTextField::LeadingIcon,
                    draft.trim(),
                );
            }
            PropertyFocus::WidgetTrailingIcon => {
                let _ = self.editor_state.set_selected_widget_text(
                    op_editor_core::WidgetTextField::TrailingIcon,
                    draft.trim(),
                );
            }
            PropertyFocus::WidgetBindKey => {
                let _ = self
                    .editor_state
                    .set_selected_widget_bind_value(draft.trim());
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

    /// Dispatch a Code-panel action. `SelectFramework` is pure
    /// `editor_state.codegen` state (works without the `codegen`
    /// feature); `Generate` / `Regenerate` / `Cancel` raise the pending
    /// flags the `lib.rs` mousedown drain turns into
    /// `codegen_web::drain_codegen_flags` work (the dispatch has no
    /// `Inner` / daemon base in scope — mirror of the desktop
    /// pending-flag + `launch_codegen_if_pending` /
    /// `drain_codegen_cancel_request` pattern); `Copy` / `Download` are
    /// browser IO via `web_clipboard` (Download produces a
    /// `component.zip` when the generation returned image assets —
    /// desktop `codegen_export` layout).
    fn apply_codegen_action(
        &mut self,
        action: op_editor_ui::widgets::property_panel_action::CodegenAction,
    ) {
        use op_editor_core::codegen::CodegenPhase;
        use op_editor_ui::widgets::property_panel_action::CodegenAction;
        let cg = &mut self.editor_state.codegen;
        match action {
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
                // Raise the cancel intent for the web runner — the drain
                // aborts the in-flight XHR + parks the run so it actually
                // stops instead of streaming on and resurrecting the panel
                // (TS: abort(); native dispatch parity).
                cg.pending_cancel = true;
                cg.phase = if cg.code.is_empty() {
                    CodegenPhase::Idle
                } else {
                    CodegenPhase::Complete
                };
            }
            CodegenAction::Copy => {
                cg.copied_at = Some(self.now_ms);
                #[cfg(feature = "canvaskit")]
                {
                    crate::web_clipboard::copy_text(&cg.code);
                }
            }
            CodegenAction::Download => {
                #[cfg(feature = "canvaskit")]
                {
                    crate::codegen_web::download_generated(&self.editor_state);
                }
            }
            CodegenAction::ExportBundle => {
                // Live structure bundle (TS code-panel.tsx
                // `handleDownloadStructureBundle` → `buildAIStructureBundle`):
                // built FRESH from the selection (or active page) at click
                // time — no completed generation required. Nothing to bundle
                // returns silently, like the TS handler.
                #[cfg(feature = "canvaskit")]
                {
                    if let Some(bytes) =
                        crate::codegen_bundle::build_live_bundle_zip(&self.editor_state)
                    {
                        let _ = crate::web_clipboard::download_bytes(
                            "bundle.zip",
                            "application/zip",
                            &bytes,
                        );
                    }
                }
            }
            CodegenAction::ScrollFrameworksLeft | CodegenAction::ScrollFrameworksRight => {
                let pw = self.editor_state.editor_ui.property_panel_width;
                let max = op_editor_ui::widgets::property_panel_code::framework_row_overflow(pw);
                let step = 100.0;
                let cg = &mut self.editor_state.codegen;
                let delta = if matches!(action, CodegenAction::ScrollFrameworksLeft) {
                    -step
                } else {
                    step
                };
                cg.framework_scroll.scroll_by(delta, max, 0.0);
            }
        }
    }
}

/// Public alias for [`color_target`] — used by the press dispatch
/// in `press.rs` so it can anchor the colour picker at the clicked
/// y instead of always passing `0.0`.
pub(in crate::widget_host) fn color_target_public(
    t: op_editor_core::ColorTarget,
) -> op_editor_core::ui_draft::ColorTarget {
    color_target(t)
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

pub(in crate::widget_host) fn property_focus_initial(
    focus: PropertyFocus,
    panel: &op_editor_ui::widgets::PropertyPanel,
) -> String {
    match focus {
        PropertyFocus::PositionX => panel.snapshot.x.to_string(),
        PropertyFocus::PositionY => panel.snapshot.y.to_string(),
        PropertyFocus::SizeW => panel.snapshot.width.to_string(),
        PropertyFocus::SizeH => panel.snapshot.height.to_string(),
        PropertyFocus::LayoutGap => format_panel_number(panel.snapshot.layout_gap),
        PropertyFocus::PaddingTop
        | PropertyFocus::PaddingRight
        | PropertyFocus::PaddingBottom
        | PropertyFocus::PaddingLeft => panel
            .snapshot
            .layout_padding
            .value_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::Rotation => (panel.snapshot.rotation_deg.round() as i32).to_string(),
        PropertyFocus::PositionR => (panel.snapshot.corner_radius.round() as i32).to_string(),
        PropertyFocus::Opacity => "100".to_string(),
        PropertyFocus::PolygonSides => panel.snapshot.polygon_sides.unwrap_or(3).to_string(),
        PropertyFocus::EllipseStart => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.start_deg)
                .unwrap_or(0.0),
        ),
        PropertyFocus::EllipseSweep => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.sweep_deg)
                .unwrap_or(360.0),
        ),
        PropertyFocus::EllipseInnerRadius => format_panel_number(
            panel
                .snapshot
                .ellipse_arc
                .map(|a| a.inner_percent)
                .unwrap_or(0.0),
        ),
        PropertyFocus::FontSize => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.font_size))
            .unwrap_or_else(|| "16".to_string()),
        PropertyFocus::FontWeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| t.font_weight.to_string())
            .unwrap_or_else(|| "400".to_string()),
        PropertyFocus::LineHeight => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.line_height_percent))
            .unwrap_or_else(|| "120".to_string()),
        PropertyFocus::LetterSpacing => panel
            .snapshot
            .text
            .as_ref()
            .map(|t| format_panel_number(t.letter_spacing))
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::FillOpacity(index) => {
            let opacity = panel
                .snapshot
                .fills
                .get(index)
                .map(|f| f.opacity)
                .unwrap_or(panel.snapshot.fill_opacity);
            ((opacity * 100.0).round() as i32).to_string()
        }
        PropertyFocus::FillHex(index) => panel
            .snapshot
            .fills
            .get(index)
            .map(|f| f.color)
            .or(panel.snapshot.fill)
            .map(color_to_hex)
            .unwrap_or_else(|| "#FFFFFF".to_string()),
        PropertyFocus::StrokeHex => color_to_hex(panel.snapshot.stroke_swatch_color()),
        // Seed the SAME width the inline input paints (0 when unset, and
        // un-rounded) so clicking in never changes the displayed value.
        PropertyFocus::StrokeWidth => {
            format_panel_number(panel.snapshot.stroke.map(|s| s.width).unwrap_or(0.0))
        }
        PropertyFocus::StrokeTopWidth
        | PropertyFocus::StrokeRightWidth
        | PropertyFocus::StrokeBottomWidth
        | PropertyFocus::StrokeLeftWidth => panel
            .snapshot
            .stroke_side_width_for(focus)
            .map(format_panel_number)
            .unwrap_or_else(|| "0".to_string()),
        PropertyFocus::GradientAngle => {
            let a = panel.snapshot.gradient_angle.unwrap_or(0.0);
            if a.fract() == 0.0 {
                format!("{}", a as i32)
            } else {
                format!("{a}")
            }
        }
        PropertyFocus::GradientStopHex(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| op_editor_ui::widgets::property_panel_fill::stop_hex_rgb_only(&s.hex))
            .unwrap_or_else(|| "#000000".to_string()),
        PropertyFocus::GradientStopOffset(i) => panel
            .snapshot
            .gradient_stops
            .get(i)
            .map(|s| ((s.offset * 100.0).round() as i32).to_string())
            .unwrap_or_else(|| "0".to_string()),
        // Widget-section fields — seed the input draft from the selected
        // widget's current value so editing starts from it (mirrors the
        // native `property_focus_initial`).
        PropertyFocus::WidgetPlaceholder => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.placeholder.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetValue => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.value.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetLabel => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.label.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetLeadingIcon => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.leading_icon.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetTrailingIcon => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.trailing_icon.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetBindKey => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.bind_key.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetMin => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.min.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetMax => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.max.clone())
            .unwrap_or_default(),
        PropertyFocus::WidgetStep => panel
            .snapshot
            .widget
            .as_ref()
            .map(|w| w.step.clone())
            .unwrap_or_default(),
    }
}

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
