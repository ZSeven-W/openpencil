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
        use op_editor_ui::widgets::file_menu::{FileMenu, FileMenuChoice};
        use op_editor_ui::widgets::top_bar::TopBar;
        self.refresh_layout_scene();
        let top_bar_rect = op_editor_ui::Rect {
            origin: op_editor_ui::Point2D::new(0.0, 0.0),
            size: op_editor_ui::Point2D::new(viewport_width, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        };
        let anchor =
            TopBar::file_menu_rect(top_bar_rect, self.editor_state.editor_ui.window_fullscreen);
        // No wall-clock on wasm32 (`SystemTime::now` panics there) and
        // the web host has no recent-file list to age against — a `0`
        // clock paints the same menu rows.
        let menu = FileMenu::from_editor_ui(&self.editor_state.editor_ui, 0);
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
        } else {
            // Miss — the dismissing click is a blank press.
            self.blur_text_inputs_on_blank_press();
        }
        self.editor_state.editor_ui.file_menu_open = false;
        self.editor_state.editor_ui.file_menu_hover = None;
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
                // The .fig file picker is a host-level service web lacks
                // — raise the same pending flag as native.
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
}
