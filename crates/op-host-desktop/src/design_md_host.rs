//! Design-MD panel host logic — drains the panel's import / export
//! requests, which need the native file dialog the widget layer
//! cannot reach.
//!
//! Split out of `main.rs` to keep that file under the repo's
//! 800-line-per-file cap.

use crate::DesktopApp;

impl DesktopApp {
    /// Run a queued Design-MD request — `design_md_request`, set by a
    /// panel click. A no-op when nothing is queued.
    pub(crate) fn drain_design_md_action(&mut self) {
        use op_editor_core::DesignMdRequest;
        let Some(request) = self
            .host
            .editor_state_mut()
            .editor_ui
            .design_md_request
            .take()
        else {
            return;
        };
        let locale = self.host.editor_state().editor_ui.locale;
        match request {
            DesignMdRequest::Import => self.import_design_md(locale),
            DesignMdRequest::Export => self.export_design_md(locale),
        }
    }

    /// Pick a `.md` file, parse it into a `DesignMdSpec`, and bind it
    /// to the open document (undoable).
    fn import_design_md(&mut self, locale: op_editor_core::Locale) {
        let picked = rfd::FileDialog::new()
            .set_title(op_i18n::translate(locale, "designMd.import"))
            .add_filter("Markdown", &["md", "markdown"])
            .pick_file();
        let Some(path) = picked else {
            return;
        };
        let markdown = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("openpencil-desktop: design.md import failed: {err}");
                return;
            }
        };
        let spec = op_editor_core::parse_design_md(&markdown);
        // Snapshot first so the import is a single undo step.
        let snap = self.host.editor_state().snapshot_for_history();
        self.host.editor_state_mut().doc.design_md = Some(spec);
        self.host.editor_state_mut().history_push_past(snap);
        self.host.mark_editor_state_dirty();
    }

    /// Write the open document's design.md to a `.md` file. The
    /// original markdown (`DesignMdSpec::raw`) round-trips verbatim.
    fn export_design_md(&mut self, locale: op_editor_core::Locale) {
        let Some(raw) = self
            .host
            .editor_state()
            .doc
            .design_md
            .as_ref()
            .map(|s| s.raw.clone())
        else {
            // Nothing to export — the panel's export button is only
            // meaningful once a brief exists.
            return;
        };
        let picked = rfd::FileDialog::new()
            .set_title(op_i18n::translate(locale, "designMd.export"))
            .add_filter("Markdown", &["md"])
            .set_file_name("design.md")
            .save_file();
        let Some(path) = picked else {
            return;
        };
        if let Err(err) = std::fs::write(&path, raw) {
            eprintln!("openpencil-desktop: design.md export failed: {err}");
        }
    }
}
