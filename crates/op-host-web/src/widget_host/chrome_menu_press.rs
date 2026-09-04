//! TopBar chrome-overlay press dispatch for the web `WidgetHost` —
//! file-menu dropdown, export dialog, and Figma-import modal.
//! Mirrors the native host's dispatchers in
//! `widget_host/property_dispatch.rs`. File / export actions raise
//! the same `pending_file_action` flags the native host does; their
//! consumers are host-level services (file pickers, image encoders)
//! the web host doesn't ship yet.

use super::WidgetHost;

impl WidgetHost {
    /// File-menu press dispatcher.
    pub(in crate::widget_host) fn dispatch_file_menu_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
    ) {
        use op_editor_core::editor_ui_state::FileAction;
        use op_editor_ui::widgets::file_menu::{FileMenu, FileMenuChoice, MenuHit};
        self.refresh_layout_scene();
        let top_bar_rect = self.top_bar_rect(viewport_width);
        let anchor = self.top_bar().file_menu_rect_for(top_bar_rect);
        let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, self.wall_now_secs);
        let menu_rect = menu.rect_at(anchor);
        let point = op_editor_ui::Point2D::new(x, y);
        match menu.hit(menu_rect, point) {
            MenuHit::Row(row) => {
                let Some(choice) = menu.choice_for_row(row) else {
                    return;
                };
                // Opening the centre is not a file action: it shows a panel,
                // and the document is replaced later by whichever card the
                // user picks.
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
                    // Handled above — it opens a panel, not a file action.
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
    /// shared with the native host; only the blur / repaint tail is web's.
    ///
    /// Rows the browser cannot serve never reach here: their capability
    /// flags stay `false`, so `export_menu_rows` leaves them out of the
    /// menu exactly as it leaves them out of the File menu.
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
                // The file picker is a host-level service web lacks —
                // raise the same pending flag as native, for whichever
                // source the modal is showing.
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
                self.editor_state.editor_ui.pending_file_action =
                    Some(FileAction::FinishFigmaImport(
                        op_editor_core::FigmaImportSelection::Page(index),
                    ));
                self.editor_state.editor_ui.figma_import_open = false;
                self.editor_state.editor_ui.figma_import_hover = None;
            }
            FigmaImportHit::ImportAll => {
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
}
