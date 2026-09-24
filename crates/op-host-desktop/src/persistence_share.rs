//! Desktop flow for File ▸ "Share…" and the Studio workspace's 分享.
//!
//! One save picker, then one self-contained `.html` share page holding
//! every board plus the `.op` (with its sanitized share recipe). The
//! render, the markup and the sanitization all live in
//! `op_host_services::export_share`; this file owns only the dialogs.

use op_host_native::WidgetHostNative;
use op_host_services::export::ExportError;
use op_host_services::export_share::{export_share_html, share_file_name, ShareOptions};
use op_i18n::Locale;
use std::path::Path;

/// Run the whole flow: save picker → export → report. `document_path`
/// is where the open document lives (relative image references resolve
/// against it); it is never written into the page.
pub fn handle_share(host: &mut WidgetHostNative, document_path: Option<&Path>) {
    let locale = host.editor_state().editor_ui.locale;
    let title = op_i18n::translate(locale, "share.dialogTitle");
    let Some(path) = rfd::FileDialog::new()
        .set_title(title)
        .add_filter("HTML", &["html"])
        .set_file_name(share_file_name(host.editor_state()))
        .save_file()
    else {
        return;
    };

    let options = ShareOptions::for_state(host.editor_state(), document_path);
    match export_share_html(host.editor_state(), &path, &options) {
        Ok(package) => {
            eprintln!(
                "[share] {} boards, {} bytes, {} raster fallbacks",
                package.boards, package.bytes, package.raster_fallbacks
            );
            crate::message_dialog::alert(
                title,
                &summary_body(locale, &path, package.boards),
                rfd::MessageLevel::Info,
            );
        }
        Err(error) => {
            eprintln!("[share] {error}");
            crate::message_dialog::alert(
                title,
                &failure_body(locale, &error),
                rfd::MessageLevel::Warning,
            );
        }
    }
}

/// Body of the success dialog: how many boards landed and where.
fn summary_body(locale: Locale, path: &Path, boards: usize) -> String {
    let mut body =
        op_i18n::translate(locale, "share.summary").replace("{{count}}", &boards.to_string());
    body.push_str("\n\n");
    body.push_str(&path.display().to_string());
    body
}

/// Body of the failure dialog: the empty case gets its own sentence.
fn failure_body(locale: Locale, error: &ExportError) -> String {
    if matches!(error, ExportError::NothingToExport) {
        return op_i18n::translate(locale, "share.empty").to_string();
    }
    let mut body = op_i18n::translate(locale, "dialog.exportErrorLead").to_string();
    body.push_str("\n\n");
    body.push_str(&error.to_string());
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clean_run_reports_the_count_and_the_file() {
        let body = summary_body(Locale::EnUs, Path::new("/tmp/share.html"), 4);
        assert!(
            body.starts_with("Saved a share page with 4 boards."),
            "{body}"
        );
        assert!(body.ends_with("/tmp/share.html"), "{body}");
    }

    #[test]
    fn the_summary_is_localised() {
        let body = summary_body(Locale::ZhCn, Path::new("/tmp/share.html"), 3);
        assert!(body.contains("3 个画板"), "{body}");
    }

    #[test]
    fn an_empty_document_gets_its_own_sentence() {
        let body = failure_body(Locale::EnUs, &ExportError::NothingToExport);
        assert_eq!(body, "There are no visible boards to share.");
    }
}
