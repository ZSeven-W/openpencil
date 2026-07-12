//! Native property/modal dispatchers extracted from the main action match.

use super::super::helpers::parse_hex_color;
use super::super::WidgetHostNative;
use super::current_stop_alpha;
use op_editor_core::PropertyFocus;

impl WidgetHostNative {
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
        let hit = dlg.hit_test(point);
        self.editor_state.editor_ui.pressed_button = hit
            .map(op_editor_ui::widgets::editor_state_ext::export_dialog_button)
            .map(op_editor_core::ButtonPressTarget::ExportDialog);
        match hit {
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
        let hit = modal.hit_test(panel_rect, op_editor_ui::Point2D::new(x, y));
        self.editor_state.editor_ui.pressed_button =
            op_editor_ui::widgets::editor_state_ext::figma_import_button(hit)
                .map(op_editor_core::ButtonPressTarget::FigmaImport);
        match hit {
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
            PropertyFocus::FillHex(index) => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let hex = super::super::helpers::color_to_hex(color);
                        // The primary fill (index 0) keeps `set_selected_color`
                        // (prepends a solid + colour-variable-aware); a
                        // non-primary row writes its own solid fill by index.
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
                            .set_selected_color(false, &super::super::helpers::color_to_hex(color));
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
                            &super::super::helpers::color_to_hex_with_alpha(with_alpha),
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
}
