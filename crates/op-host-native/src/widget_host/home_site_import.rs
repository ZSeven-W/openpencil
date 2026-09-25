//! Studio Home's one-click website import, host arm.
//!
//! A link-only draft turns Send into "Import this site" (see
//! `HomeSendMode::ImportSite`). The press records a request on the shared
//! Home state; the shell that owns the network drains it
//! ([`WidgetHostNative::take_home_site_import_request`]), runs
//! `op_host_services::site_import` off the UI thread, and lands the result
//! through [`WidgetHostNative::finish_home_site_import`]. The imported
//! document goes through the same swap a Home brief uses
//! (`swap_in_document_for_home`: unsaved work is parked for the shell, the
//! transcript starts fresh), and the workspace opens on it with the
//! import's quality report and a transcript line saying what was done.

use super::WidgetHostNative;
use op_editor_core::{ChatMessage, HomeFamily, SiteImportResult, Tool};

impl WidgetHostNative {
    /// The Send press in `ImportSite` mode. Returns whether an import was
    /// queued.
    pub(in crate::widget_host) fn home_site_import_press(&mut self) -> bool {
        let home = &mut self.editor_state.editor_ui.home;
        let Some(url) = op_editor_core::site_import_url(&home.draft) else {
            return false;
        };
        let queued = home.site_import.press(&url, self.now_ms);
        if queued {
            self.mark_dirty();
        }
        queued
    }

    /// Shell: take the queued import `(generation, url)`.
    pub fn take_home_site_import_request(&mut self) -> Option<(u64, String)> {
        self.editor_state.editor_ui.home.site_import.take_request()
    }

    /// Shell: land a finished import. A stale / cancelled request is
    /// dropped; a failure shows the timed hint over Send; a success swaps
    /// the document in and opens the workspace on it. Returns whether
    /// anything changed.
    pub fn finish_home_site_import(
        &mut self,
        generation: u64,
        result: Result<SiteImportResult, String>,
    ) -> bool {
        if !self
            .editor_state
            .editor_ui
            .home
            .site_import
            .accept(generation)
        {
            return false;
        }
        let now_ms = self.now_ms;
        match result {
            Ok(import) => {
                if !self.open_imported_site(import) {
                    // Refused by the swap (e.g. a collaboration session
                    // that does not allow replacing the document).
                    self.editor_state
                        .editor_ui
                        .home
                        .site_import
                        .fail("document swap refused", now_ms);
                }
            }
            Err(detail) => self
                .editor_state
                .editor_ui
                .home
                .site_import
                .fail(detail, now_ms),
        }
        self.mark_dirty();
        true
    }

    /// Install an imported site as the open document and show it in the
    /// workspace, Done, with its report.
    pub(in crate::widget_host) fn open_imported_site(&mut self, import: SiteImportResult) -> bool {
        let SiteImportResult {
            document,
            summary,
            report,
        } = import;
        // Always parked, even an untouched starter: the shell must forget
        // the replaced file's path so ⌘S never writes the site over it (a
        // clean starter parks with nothing to rescue).
        if !self.swap_in_document_for_home(*document) {
            return false;
        }
        // Never saved anywhere: the import is unsaved work, so a later Home
        // brief parks it instead of dropping it.
        self.editor_state.mark_document_changed();

        let previous_tool = Some(self.editor_state.tool);
        let options = self
            .editor_state
            .editor_ui
            .home
            .draft_for(HomeFamily::Web)
            .clone();
        self.editor_state.editor_ui.open_workspace_for_generation(
            HomeFamily::Web,
            summary.source_url.clone(),
            options,
            0,
            self.now_ms,
            previous_tool,
        );
        let workspace = &mut self.editor_state.editor_ui.workspace;
        // No run is coming: the import IS the result.
        workspace.mark_done(0);
        workspace.quality = Some(report);
        self.editor_state.tool = Tool::Hand;
        self.editor_state.editor_ui.home.hide();

        let locale = self.editor_state.editor_ui.locale;
        let chat = &mut self.editor_state.chat;
        chat.title_from_prompt_if_untitled(&summary.host);
        chat.messages
            .push(ChatMessage::user(summary.source_url.clone()));
        chat.messages
            .push(ChatMessage::assistant(summary.transcript(locale)));
        chat.expand();

        let (viewport_w, viewport_h) = (self.last_viewport_w, self.last_viewport_h);
        if viewport_w > 0.0 && viewport_h > 0.0 {
            self.apply_workspace_fit(viewport_w, viewport_h);
        }
        self.mark_dirty();
        true
    }
}

#[cfg(test)]
#[path = "home_site_import_tests.rs"]
mod tests;
