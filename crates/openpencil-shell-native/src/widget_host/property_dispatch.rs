//! PropertyPanel action + commit dispatch, split out of `input.rs`
//! to stay under the 800-line cap.

use super::helpers::parse_hex_color;
use super::WidgetHostNative;
use openpencil_shell_core::document::PropertyFocus;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_property_action(
        &mut self,
        action: openpencil_shell_core::widgets::PropertyPanelAction,
    ) {
        use openpencil_shell_core::widgets::PropertyPanelAction as A;
        match action {
            A::SetFlexLayout(mode) => self.document.ui.flex_layout = mode,
            A::ToggleSizeFillWidth => {
                self.document.ui.size_fill_width = !self.document.ui.size_fill_width;
            }
            A::ToggleSizeFillHeight => {
                self.document.ui.size_fill_height = !self.document.ui.size_fill_height;
            }
            A::ToggleSizeHugWidth => {
                self.document.ui.size_hug_width = !self.document.ui.size_hug_width;
            }
            A::ToggleSizeHugHeight => {
                self.document.ui.size_hug_height = !self.document.ui.size_hug_height;
            }
            A::ToggleSizeClipContent => {
                self.document.ui.size_clip_content = !self.document.ui.size_clip_content;
            }
            A::ToggleFillTypePicker => {
                self.document.ui.fill_type_picker_open = !self.document.ui.fill_type_picker_open;
            }
            A::SetFillType(t) => {
                self.document.set_selected_fill_type(t);
                self.document.ui.fill_type_picker_open = false;
            }
            A::OpenColorPicker(target) => {
                // Fallback anchor when called outside the press
                // path (no click y available); the press handler
                // calls `open_color_picker` directly with the real
                // click y so the picker centers on the swatch.
                let _ = self.document.open_color_picker(target, 0.0);
            }
            A::OpenExportDialog => {
                self.document.ui.pending_file_action =
                    Some(openpencil_shell_core::document::FileAction::ExportImage);
            }
        }
    }

    /// Export-dialog press dispatcher. Format / Scale pills mutate
    /// `Document.ui.export_format` / `export_scale`; Cancel + outside
    /// click close the dialog; Export closes the dialog AND queues
    /// `FileAction::ExportImage` so the desktop binary's save dialog
    /// fires with the chosen format + scale.
    pub(in crate::widget_host) fn dispatch_export_dialog_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        use openpencil_shell_core::document::FileAction;
        use openpencil_shell_core::widgets::export_dialog::{
            scale_from_index, ExportDialog, ExportDialogHit,
        };
        let dlg = ExportDialog::centered(viewport_w, viewport_h);
        let point = openpencil_shell_core::Point2D::new(x, y);
        match dlg.hit_test(point) {
            Some(ExportDialogHit::Format(f)) => {
                self.document.ui.export_format = f;
            }
            Some(ExportDialogHit::Scale(i)) => {
                self.document.ui.export_scale = scale_from_index(i);
            }
            Some(ExportDialogHit::Cancel) => {
                self.document.ui.export_dialog_open = false;
            }
            Some(ExportDialogHit::Export) => {
                self.document.ui.export_dialog_open = false;
                self.document.ui.pending_file_action = Some(FileAction::ExportImageConfirm);
            }
            None => {
                if !dlg.contains(point) {
                    // Outside click — dismiss like Cancel.
                    self.document.ui.export_dialog_open = false;
                }
                // In-dialog dead-space — swallow without dispatching.
            }
        }
    }

    /// Figma-import-modal press dispatcher. Routes Outside / Close
    /// to dismissal; DropZone hit pushes `FileAction::ImportFigma`
    /// so the desktop binary opens the rfd .fig picker.
    pub(in crate::widget_host) fn dispatch_figma_import_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) {
        use openpencil_shell_core::document::FileAction;
        use openpencil_shell_core::widgets::figma_import::{FigmaImportHit, FigmaImportModal};
        let modal = FigmaImportModal::for_document(&self.document);
        let panel_rect = modal.rect(viewport_w, viewport_h);
        match modal.hit_test(panel_rect, openpencil_shell_core::Point2D::new(x, y)) {
            FigmaImportHit::Close | FigmaImportHit::Outside => {
                self.document.ui.figma_import_open = false;
            }
            FigmaImportHit::DropZone => {
                self.document.ui.pending_file_action = Some(FileAction::ImportFigma);
                self.document.ui.figma_import_open = false;
            }
            FigmaImportHit::Inside => {}
        }
    }

    /// File-menu press dispatcher — extracted from press.rs to keep
    /// the spine under the 800-line cap. Maps a `FileMenuChoice` to
    /// `Document.ui.pending_file_action` so the desktop binary can
    /// run the rfd dialogs + persistence calls.
    pub(in crate::widget_host) fn dispatch_file_menu_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
    ) {
        use openpencil_shell_core::document::FileAction;
        use openpencil_shell_core::widgets::file_menu::{FileMenu, FileMenuChoice};
        use openpencil_shell_core::widgets::top_bar::TopBar;
        let top_bar_rect = openpencil_shell_core::Rect {
            origin: openpencil_shell_core::Point2D::new(0.0, 0.0),
            size: openpencil_shell_core::Point2D::new(viewport_width, openpencil_shell_core::widgets::TOP_BAR_HEIGHT),
        };
        let anchor = TopBar::file_menu_rect(top_bar_rect);
        // Recent rows participate in hit-test — pass real Unix secs
        // so the age column matches paint (it's only used by paint
        // but `from_document` builds the same RecentEntry list paint
        // reads, keeping geometry consistent).
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let menu = FileMenu::from_document(&self.document, now_secs);
        let menu_rect = menu.rect_at(anchor);
        if let Some(choice) = menu.hit_test(menu_rect, openpencil_shell_core::Point2D::new(x, y)) {
            self.document.ui.pending_file_action = Some(match choice {
                FileMenuChoice::NewFile => FileAction::New,
                FileMenuChoice::OpenFile => FileAction::Open,
                FileMenuChoice::Save => FileAction::Save,
                FileMenuChoice::SaveAs => FileAction::SaveAs,
                FileMenuChoice::ExportImage => FileAction::ExportImage,
                FileMenuChoice::OpenRecent(i) => FileAction::OpenRecent(i),
                FileMenuChoice::ClearRecent => FileAction::ClearRecent,
            });
        }
        self.document.ui.file_menu_open = false;
        self.document.ui.file_menu_hover = None;
    }

    /// Commit any focused settings-modal input (currently only the
    /// MCP port). Parses the draft, clamps to a valid port range,
    /// writes it back, and clears focus + draft. No-op when nothing
    /// is focused.
    pub(in crate::widget_host) fn commit_settings_focus_if_any(&mut self) {
        use openpencil_shell_core::document::SettingsFocus;
        let Some(focus) = self.document.ui.agent_settings.focus.take() else {
            return;
        };
        let draft = std::mem::take(&mut self.document.ui.settings_input_draft);
        match focus {
            SettingsFocus::McpPort => {
                if let Ok(port) = draft.trim().parse::<u16>() {
                    // Keep ports above 1024 to avoid root-only ranges;
                    // anything below silently falls back to 1024 so
                    // the user still gets a usable value.
                    self.document.ui.agent_settings.mcp_server.port = port.max(1024);
                }
            }
        }
    }

    pub(in crate::widget_host) fn commit_property_focus_if_any(&mut self) {
        let Some(focus) = self.document.ui.property_focus.take() else {
            return;
        };
        self.document.ui.property_draft_select_all = false;
        let draft = std::mem::take(&mut self.document.ui.property_input_draft);
        match focus {
            PropertyFocus::FillHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self.document.set_selected_color(true, color);
                    }
                }
            }
            PropertyFocus::StrokeHex => {
                let stripped = draft.trim().trim_start_matches('#');
                if !stripped.is_empty() {
                    if let Some(color) = parse_hex_color(draft.trim()) {
                        let _ = self.document.set_selected_color(false, color);
                    }
                }
            }
            _ => {
                if let Ok(value) = draft.trim().parse::<f32>() {
                    let _ = self.document.commit_property_edit(focus, value);
                }
            }
        }
    }
}
