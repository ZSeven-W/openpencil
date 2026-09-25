//! Studio Home's one-click website import, browser arm.
//!
//! Web twin of native `home_site_import.rs`. A link-only draft turns Send
//! into "Import this site" once the daemon reports it serves the import
//! route (`site_import.available`, set from `GET /api/mcp/server`). The
//! press records a request on the shared Home state; the DOM layer
//! (`studio_web::drain_home_site_import`) POSTs it to the daemon, which runs
//! the same pipeline desktop runs, and lands the reply through
//! [`WidgetHost::finish_home_site_import`].
//!
//! Unlike desktop there is no rescue copy to park the replaced document in
//! (see `studio_home_send.rs`), so the press asks FIRST when the open
//! document has unsaved changes — and swaps nothing until the import has
//! actually succeeded, so a failed fetch leaves the page as it was.

use super::{HomeReplaceIntent, WidgetHost};
use op_editor_core::{ChatMessage, HomeFamily, SiteImportResult, Tool};

impl WidgetHost {
    /// The Send press in `ImportSite` mode. Returns whether an import was
    /// queued (or the discard confirm was raised for it).
    pub(in crate::widget_host) fn home_site_import_press(
        &mut self,
        discard_confirmed: bool,
    ) -> bool {
        let Some(url) = op_editor_core::site_import_url(&self.editor_state.editor_ui.home.draft)
        else {
            return false;
        };
        let blank = op_editor_core::blank_starter::active_page_is_blank_starter(&self.editor_state);
        if !discard_confirmed && !blank && self.editor_state.is_dirty() {
            self.home_replace_confirm = Some(HomeReplaceIntent::ImportSite);
            return false;
        }
        let queued = self
            .editor_state
            .editor_ui
            .home
            .site_import
            .press(&url, self.now_ms);
        if queued {
            self.mark_dirty();
        }
        queued
    }

    /// DOM drain: take the queued import `(generation, url)`.
    pub fn take_home_site_import_request(&mut self) -> Option<(u64, String)> {
        self.editor_state.editor_ui.home.site_import.take_request()
    }

    /// Land a finished import. A stale / cancelled request is dropped; a
    /// failure shows the timed hint over Send; a success swaps the document
    /// in and opens the workspace on it. Returns whether anything changed.
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
        match result {
            Ok(import) => self.open_imported_site(import),
            Err(detail) => {
                let now_ms = self.now_ms;
                self.editor_state
                    .editor_ui
                    .home
                    .site_import
                    .fail(detail, now_ms);
            }
        }
        self.mark_dirty();
        true
    }

    /// Install an imported site as the open document and show it in the
    /// workspace, Done, with its report.
    fn open_imported_site(&mut self, import: SiteImportResult) {
        let SiteImportResult {
            document,
            summary,
            report,
        } = import;
        // Unsaved work: the import has never been written anywhere.
        self.install_home_document(*document, false);
        // Saved as `editorMeta.importedFrom` and sent with every chat turn,
        // so a later AI edit keeps the site's authored design.
        self.editor_state.editor_ui.home.imported_from =
            op_editor_core::sanitize_import_origin(&summary.source_url);

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

        let locale = self.editor_state.editor_ui.effective_locale();
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
    }
}

#[cfg(test)]
#[path = "studio_home_site_import_tests.rs"]
mod tests;
