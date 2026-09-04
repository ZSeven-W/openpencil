//! Which export rows a document offers — the single source shared by the
//! File menu's export section (`file_menu.rs`) and the TopBar export quick
//! menu (`export_quick_menu.rs`).
//!
//! The two surfaces offer the same actions in a different order, so the
//! availability conditions live here rather than being spelled twice: a
//! host capability the File menu respects but the quick menu forgot would
//! paint a row whose action is a silent no-op.

use op_editor_core::editor_ui_state::EditorUiState;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::ExportQuickRow;

/// Whether the open document is tagged as a slide deck. Deck-only export
/// formats (PowerPoint, slideshow page, slide-per-page PDF) hang off this.
pub fn document_is_deck(ui: &EditorUiState) -> bool {
    ui.scenario == Some(TemplateScene::Slides)
}

/// Whether the batch frame export is offered — hosts without a directory
/// picker + offscreen exporter leave it out entirely rather than paint a
/// dead row.
pub fn batch_frame_export_available(ui: &EditorUiState) -> bool {
    ui.batch_frame_export_supported
}

/// Whether the deck-export rows (slideshow page + PowerPoint) are offered.
/// Two conditions, both necessary: the host can write a file at all, and
/// the document is a deck — exporting a dashboard as a slideshow would be
/// an offer with no meaning behind it.
///
/// The slideshow page and the PowerPoint deck stand or fall together. They
/// are the same capability (render the boards, write one file through a
/// save picker) offered in two formats, so a host that can do one can do
/// the other.
pub fn deck_export_available(ui: &EditorUiState) -> bool {
    ui.deck_html_export_supported && document_is_deck(ui)
}

/// The rows the export quick menu paints, top to bottom.
///
/// A deck leads with the formats a deck is actually delivered in; every
/// document ends with the image export and, where the host supports it,
/// the whole frame set. PDF needs no host capability flag — it commits
/// through the same save picker as any other image format.
pub fn quick_menu_rows(ui: &EditorUiState) -> Vec<ExportQuickRow> {
    let mut rows = Vec::with_capacity(ExportQuickRow::ALL.len());
    if deck_export_available(ui) {
        rows.push(ExportQuickRow::Pptx);
    }
    if document_is_deck(ui) {
        rows.push(ExportQuickRow::Pdf);
    }
    if deck_export_available(ui) {
        rows.push(ExportQuickRow::SlideshowHtml);
    }
    rows.push(ExportQuickRow::Image);
    if batch_frame_export_available(ui) {
        rows.push(ExportQuickRow::AllFrames);
    }
    rows
}

/// Resolve a `fileMenu.*` row label, dropping the trailing ellipsis the
/// catalogue carries (a Mac convention the Rust menus don't follow).
pub fn trimmed_label(ui: &EditorUiState, key: &'static str) -> &'static str {
    op_i18n::translate(ui.effective_locale(), key).trim_end_matches(['.', '…'])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck_ui() -> EditorUiState {
        EditorUiState {
            scenario: Some(TemplateScene::Slides),
            deck_html_export_supported: true,
            batch_frame_export_supported: true,
            ..EditorUiState::default()
        }
    }

    #[test]
    fn deck_desktop_leads_with_powerpoint_and_offers_five_rows() {
        let rows = quick_menu_rows(&deck_ui());
        assert_eq!(
            rows,
            vec![
                ExportQuickRow::Pptx,
                ExportQuickRow::Pdf,
                ExportQuickRow::SlideshowHtml,
                ExportQuickRow::Image,
                ExportQuickRow::AllFrames,
            ]
        );
    }

    #[test]
    fn non_deck_offers_only_image_and_frame_rows() {
        let ui = EditorUiState {
            batch_frame_export_supported: true,
            ..EditorUiState::default()
        };
        assert_eq!(
            quick_menu_rows(&ui),
            vec![ExportQuickRow::Image, ExportQuickRow::AllFrames]
        );
    }

    #[test]
    fn host_without_capabilities_offers_the_image_row_alone() {
        let ui = EditorUiState::default();
        assert_eq!(quick_menu_rows(&ui), vec![ExportQuickRow::Image]);
    }

    #[test]
    fn deck_without_html_capability_drops_both_deck_file_rows() {
        let ui = EditorUiState {
            scenario: Some(TemplateScene::Slides),
            ..EditorUiState::default()
        };
        // PDF survives: it rides the same save picker as any image format.
        assert_eq!(
            quick_menu_rows(&ui),
            vec![ExportQuickRow::Pdf, ExportQuickRow::Image]
        );
        assert!(!deck_export_available(&ui));
    }
}
