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
                ui.icon_picker_caret_anchor_ms = self.now_ms;
                ui.icon_picker_scroll = 0.0;
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.font_family_picker_open = false;
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
                let opening = !ui.font_family_picker_open;
                ui.font_family_picker_open = opening;
                if opening {
                    ui.font_family_picker_scroll = 0.0;
                }
                ui.fill_type_picker_open = false;
                ui.image_fill_popover_open = false;
                ui.export_scale_picker_open = false;
                ui.export_format_picker_open = false;
            }
            A::SetFontFamily(family) => {
                self.set_selected_text_font_family(&family);
                self.editor_state.editor_ui.font_family_picker_open = false;
                self.editor_state.editor_ui.font_family_picker_scroll = 0.0;
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
                ui.export_picker_hover = None;
            }
            A::ToggleExportFormatPicker => {
                let ui = &mut self.editor_state.editor_ui;
                ui.export_format_picker_open = !ui.export_format_picker_open;
                ui.export_scale_picker_open = false;
                ui.font_family_picker_open = false;
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
        }
        self.mark_dirty();
    }

    /// Image-fill popover outside-click dismiss. Returns `true`
    /// when the popover was open and the press was consumed.
    pub(in crate::widget_host) fn dismiss_image_fill_popover_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.image_fill_popover_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                self.apply_property_action(action);
                return true;
            }
            if panel.image_fill_popover_contains(property_rect, Point2D::new(x, y)) {
                return true;
            }
        }
        self.editor_state.editor_ui.image_fill_popover_open = false;
        self.mark_dirty();
        true
    }

    /// Outside-click dismiss for the Export section's inline scale /
    /// format select popups. Returns `true` when a picker was open
    /// and the press was consumed — an option / toggle was applied,
    /// or the press fell outside and dismissed the popup. The caller
    /// must stop dispatching the press in that case. `false` when no
    /// picker was open (press dispatch continues normally).
    pub(in crate::widget_host) fn dismiss_export_picker_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.export_scale_picker_open
            && !self.editor_state.editor_ui.export_format_picker_open
        {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                if matches!(
                    action,
                    A::SetExportScale(_)
                        | A::SetExportFormat(_)
                        | A::ToggleExportScalePicker
                        | A::ToggleExportFormatPicker
                ) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        let ui = &mut self.editor_state.editor_ui;
        ui.export_scale_picker_open = false;
        ui.export_format_picker_open = false;
        ui.export_picker_hover = None;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn dismiss_font_family_picker_on_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        use op_editor_ui::widgets::{PropertyPanel, PropertyPanelAction as A, TOP_BAR_HEIGHT};
        use op_editor_ui::{Point2D, Rect};
        if !self.editor_state.editor_ui.font_family_picker_open {
            return false;
        }
        self.refresh_layout_scene();
        if let Some(panel) = PropertyPanel::for_selection(&self.editor_state) {
            let property_rect = Rect {
                origin: Point2D::new(
                    viewport_width - self.editor_state.editor_ui.property_panel_width,
                    TOP_BAR_HEIGHT,
                ),
                size: Point2D::new(
                    self.editor_state.editor_ui.property_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            if let Some(action) = panel.hit_test_action(property_rect, Point2D::new(x, y)) {
                if matches!(action, A::SetFontFamily(_) | A::ToggleFontFamilyPicker) {
                    self.apply_property_action(action);
                    return true;
                }
            }
        }
        self.editor_state.editor_ui.font_family_picker_open = false;
        self.editor_state.editor_ui.font_family_picker_scroll = 0.0;
        self.mark_dirty();
        true
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
        self.commit_variables_panel_header_focus_if_any();
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
