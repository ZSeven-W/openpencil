//! Web `apply_property_action` — PropertyPanel button / dropdown
//! action dispatch. Split out of `press.rs` to keep that file under
//! the 800-line cap (mirrors the native host's `property_dispatch.rs`).

use super::WidgetHost;
use jian_ops_schema::variable::VariableKind;
use op_editor_core::{EffectField, PropertyFocus};
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
    #[cfg(feature = "live-sync")]
    pub(crate) fn replace_document(&mut self, doc: op_editor_core::PenDocument) {
        self.editor_state.replace_document(doc);
        self.mark_dirty();
    }

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
                ui.property_color_variable_picker_open = None;
            }
            A::SetFillType(t) => {
                self.editor_state.set_selected_fill_type(t);
                self.editor_state.editor_ui.fill_type_picker_open = false;
                self.editor_state.editor_ui.image_fill_popover_open = false;
                self.editor_state
                    .editor_ui
                    .property_color_variable_picker_open = None;
            }
            A::ToggleImageFillPopover => {
                let ui = &mut self.editor_state.editor_ui;
                ui.image_fill_popover_open = !ui.image_fill_popover_open;
                ui.fill_type_picker_open = false;
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
                ui.icon_picker_open = true;
                ui.icon_picker_replace_selection = true;
                ui.icon_picker_search.clear();
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.font_family_picker_open = false;
                ui.font_weight_picker_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
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
            A::ToggleColorVariablePicker(target) => {
                let target = color_target(target);
                let ui = &mut self.editor_state.editor_ui;
                ui.property_color_variable_picker_open =
                    if ui.property_color_variable_picker_open == Some(target) {
                        None
                    } else {
                        Some(target)
                    };
                ui.fill_type_picker_open = false;
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
            A::PickFillImage => {
                // Web file-picker path lands later; the wasm shell
                // has no rfd / native dialog so this is a no-op for
                // now. A future implementation would surface a
                // `<input type="file">` via the JS bridge.
            }
            A::ToggleFontFamilyPicker => {
                // The wasm host has no system-font enumeration (the
                // bundle ships embedded fonts only), so the picker
                // paints the bundled group + the TS
                // FALLBACK_SYSTEM_FONTS list — the same set the TS
                // app shows when `queryLocalFonts` is unavailable.
                let ui = &mut self.editor_state.editor_ui;
                ui.font_family_picker_open = !ui.font_family_picker_open;
                ui.font_picker_search.clear();
                ui.font_picker_scroll = 0.0;
                ui.font_picker_hover = None;
                ui.font_weight_picker_open = false;
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
                ui.property_color_variable_picker_open = None;
            }
            A::SetFontFamilyIndex(index) => {
                let family = {
                    let ui = &self.editor_state.editor_ui;
                    op_editor_ui::widgets::property_panel_typography::font_picker_entries(
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
                let ui = &mut self.editor_state.editor_ui;
                ui.font_family_picker_open = false;
                ui.font_picker_search.clear();
                ui.font_picker_scroll = 0.0;
                ui.font_picker_hover = None;
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
                ui.font_weight_picker_open = false;
                ui.fill_type_picker_open = false;
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
            A::Codegen(codegen_action) => {
                self.apply_codegen_action(codegen_action);
            }
            _ => {}
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
            PropertyFocus::FillHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self
                            .editor_state
                            .set_selected_color(true, &color_to_hex(color));
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
                #[cfg(feature = "codegen")]
                {
                    crate::web_clipboard::copy_text(&cg.code);
                }
            }
            CodegenAction::Download => {
                #[cfg(feature = "codegen")]
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
                #[cfg(feature = "codegen")]
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
                cg.framework_scroll = if matches!(action, CodegenAction::ScrollFrameworksLeft) {
                    (cg.framework_scroll - step).clamp(0.0, max)
                } else {
                    (cg.framework_scroll + step).clamp(0.0, max)
                };
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
        PropertyFocus::FillOpacity => {
            ((panel.snapshot.fill_opacity * 100.0).round() as i32).to_string()
        }
        PropertyFocus::FillHex => panel
            .snapshot
            .fill
            .map(color_to_hex)
            .unwrap_or_else(|| "#FFFFFF".to_string()),
        PropertyFocus::StrokeHex => color_to_hex(panel.snapshot.stroke_swatch_color()),
        PropertyFocus::StrokeWidth => panel
            .snapshot
            .stroke
            .map(|s| format!("{}", s.width.round() as i32))
            .unwrap_or_else(|| "1".to_string()),
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
    }
}

fn format_panel_number(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.2}")
    }
}

fn parse_hex_color(s: &str) -> Option<Color> {
    let trimmed = s.trim().trim_start_matches('#');
    if trimmed.is_empty() || trimmed.len() > 8 {
        return None;
    }
    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let canonical = match trimmed.len() {
        3 => {
            let mut out = String::with_capacity(6);
            for c in trimmed.chars() {
                out.push(c);
                out.push(c);
            }
            out
        }
        8 => trimmed.to_string(),
        7 => format!("{:0>8}", trimmed),
        _ => format!("{:0>6}", trimmed),
    };
    let r = u8::from_str_radix(&canonical[0..2], 16).ok()?;
    let g = u8::from_str_radix(&canonical[2..4], 16).ok()?;
    let b = u8::from_str_radix(&canonical[4..6], 16).ok()?;
    let a = if canonical.len() == 8 {
        u8::from_str_radix(&canonical[6..8], 16).ok()? as f32 / 255.0
    } else {
        1.0
    };
    Some(Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    })
}

fn color_to_hex(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn color_to_hex_with_alpha(c: Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (c.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
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
