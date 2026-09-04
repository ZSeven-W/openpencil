//! Native property/modal dispatchers extracted from the main action match.

use super::super::WidgetHostNative;
use op_editor_core::PropertyFocus;
use op_editor_ui::widgets::property_panel_commit as commit;

impl WidgetHostNative {
    /// Commit the floating image-fill editor's numeric draft before an action
    /// hides or replaces that editor. Keeping this guard here gives every
    /// close path the same focus/draft cleanup without disturbing unrelated
    /// property inputs.
    pub(in crate::widget_host) fn commit_image_tile_scale_focus_if_any(&mut self) -> bool {
        if self.editor_state.ui.property_focus != Some(PropertyFocus::ImageTileScale) {
            return false;
        }
        self.commit_property_focus_if_any();
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
                if modal.page_selection_active() {
                    self.editor_state.editor_ui.pending_file_action = Some(
                        FileAction::FinishFigmaImport(op_editor_core::FigmaImportSelection::Cancel),
                    );
                }
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::Outside => {
                // Outside click — blank press: dismiss + blur inputs.
                self.blur_text_inputs_on_blank_press();
                if modal.page_selection_active() {
                    self.editor_state.editor_ui.pending_file_action = Some(
                        FileAction::FinishFigmaImport(op_editor_core::FigmaImportSelection::Cancel),
                    );
                }
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::DropZone => {
                if !self.collab_allows_document_mutation_from(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::ExternalAssets,
                    ),
                    op_editor_core::CollabEditSource::Import,
                ) {
                    return;
                }
                use op_editor_core::figma_import_state::ImportSource;
                self.editor_state.editor_ui.pending_file_action =
                    Some(match self.editor_state.editor_ui.import_source {
                        ImportSource::Figma => FileAction::ImportFigma,
                        ImportSource::Html => FileAction::ImportHtml,
                    });
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::Page(index) => {
                if !self.collab_allows_document_mutation_from(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::ExternalAssets,
                    ),
                    op_editor_core::CollabEditSource::Import,
                ) {
                    return;
                }
                self.editor_state.editor_ui.pending_file_action =
                    Some(FileAction::FinishFigmaImport(
                        op_editor_core::FigmaImportSelection::Page(index),
                    ));
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::ImportAll => {
                if !self.collab_allows_document_mutation_from(
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::ExternalAssets,
                    ),
                    op_editor_core::CollabEditSource::Import,
                ) {
                    return;
                }
                self.editor_state.editor_ui.pending_file_action = Some(
                    FileAction::FinishFigmaImport(op_editor_core::FigmaImportSelection::All),
                );
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
                let gate_action = match choice {
                    FileMenuChoice::NewFile
                    | FileMenuChoice::OpenFile
                    | FileMenuChoice::OpenRecent(_) => {
                        op_editor_core::CollabGateAction::ReplaceDocument
                    }
                    FileMenuChoice::Save => op_editor_core::CollabGateAction::SaveShared,
                    FileMenuChoice::SaveAs => op_editor_core::CollabGateAction::SaveFork,
                    FileMenuChoice::SaveAsTemplate => op_editor_core::CollabGateAction::LocalUi,
                    FileMenuChoice::ExportImage
                    | FileMenuChoice::ExportAllFrames
                    | FileMenuChoice::ExportSlideshowHtml
                    | FileMenuChoice::ExportPptx
                    | FileMenuChoice::ClearRecent
                    // Opening the centre only shows a panel; the document is
                    // replaced later, and that step re-runs the gate itself.
                    | FileMenuChoice::NewFromTemplate => op_editor_core::CollabGateAction::LocalUi,
                };
                if !self.collab_allows_user_action(gate_action) {
                    self.editor_state.editor_ui.file_menu_open = false;
                    self.editor_state.editor_ui.file_menu.hover = None;
                    return;
                }
                if choice == FileMenuChoice::NewFromTemplate {
                    self.editor_state
                        .editor_ui
                        .open_scene_template_center(self.now_ms);
                    self.editor_state.editor_ui.file_menu_open = false;
                    self.editor_state.editor_ui.file_menu.hover = None;
                    self.mark_dirty();
                    return;
                }
                if choice == FileMenuChoice::SaveAsTemplate {
                    self.editor_state
                        .editor_ui
                        .scene_template_center
                        .request_save_current();
                    self.editor_state.editor_ui.file_menu_open = false;
                    self.editor_state.editor_ui.file_menu.hover = None;
                    self.mark_dirty();
                    return;
                }
                self.editor_state.editor_ui.pending_file_action = Some(match choice {
                    FileMenuChoice::NewFile => FileAction::New,
                    FileMenuChoice::OpenFile => FileAction::Open,
                    FileMenuChoice::Save => FileAction::Save,
                    FileMenuChoice::SaveAs => FileAction::SaveAs,
                    FileMenuChoice::ExportImage => FileAction::ExportImage,
                    FileMenuChoice::ExportAllFrames => FileAction::ExportAllFrames,
                    FileMenuChoice::ExportSlideshowHtml => FileAction::ExportSlideshowHtml,
                    FileMenuChoice::ExportPptx => FileAction::ExportPptx,
                    FileMenuChoice::OpenRecent(i) => FileAction::OpenRecent(i),
                    FileMenuChoice::ClearRecent => FileAction::ClearRecent,
                    // Handled above — it opens a panel rather than queuing a
                    // file action.
                    FileMenuChoice::NewFromTemplate | FileMenuChoice::SaveAsTemplate => return,
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

    /// Export quick-menu press dispatcher. The row → `FileAction` walk is
    /// shared with the web host; only the blur / repaint tail is native.
    ///
    /// No collaboration gate: every row is a `CollabGateAction::LocalUi`
    /// export, exactly as the file-menu dispatcher classifies the same
    /// choices, and that action is admitted unconditionally.
    pub(in crate::widget_host) fn dispatch_export_quick_menu_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
    ) {
        use op_editor_ui::widgets::press_flow::{self, ExportQuickMenuPress};
        self.refresh_layout_scene();
        let panel_rect = self.export_quick_menu_rect(viewport_width);
        match press_flow::press_export_quick_menu(
            &mut self.editor_state,
            panel_rect,
            op_editor_ui::Point2D::new(x, y),
        ) {
            ExportQuickMenuPress::Swallow => {}
            ExportQuickMenuPress::Applied => self.mark_dirty(),
            ExportQuickMenuPress::Outside => {
                // Silent outside-close is a blank press — blur inputs too.
                self.blur_text_inputs_on_blank_press();
                op_editor_core::host_press_transitions::close_export_quick_menu(
                    &mut self.editor_state.editor_ui,
                );
                self.mark_dirty();
            }
        }
    }

    /// Commit a pending effect-parameter edit (Effects section's
    /// editable value box). Parses the shared draft and writes it
    /// via `SetEffectParam`; a non-numeric draft is discarded.
    pub(in crate::widget_host) fn commit_effect_param_focus_if_any(&mut self) {
        if self.editor_state.editor_ui.effect_param_focus.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::Effects,
                ),
            )
        {
            if commit::discard_effect_param_focus(&mut self.editor_state) {
                self.mark_dirty();
            }
            return;
        }
        if commit::commit_effect_param_focus(&mut self.editor_state) {
            self.mark_dirty();
        }
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        // Commit any pending variable-row / effect-param edit first.
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.commit_effect_param_focus_if_any();
        if let Some(focus) = self.editor_state.ui.property_focus {
            if !self.collab_allows_document_mutation(focus.collab_document_mutation()) {
                if commit::discard_property_focus(&mut self.editor_state) {
                    self.mark_dirty();
                }
                return;
            }
        }
        if commit::commit_property_focus(&mut self.editor_state) {
            self.mark_dirty();
        }
    }
}
