//! Editor-state export dispatch shared by desktop and mobile shells.

use std::path::Path;

use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::{EditorState, ExportFormat};

use crate::pdf::{render_deck_pdf_boards_bytes, render_deck_pdf_bytes, render_pdf_bytes};
use crate::{
    render_node_on_page_raster_bytes, render_node_svg_bytes, render_page_raster_bytes,
    render_svg_bytes, ExportError, RasterFormat,
};

/// Scope captured when the shell drains an export action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorExportScope {
    /// Use the format, scale, selection, and scenario currently configured on
    /// the editor state.
    Configured,
    /// Export only these slide boards as a PDF. The ids are a filter; output
    /// order still follows authored deck order.
    DeckBoards(Vec<String>),
}

/// One complete file ready for a native save or share flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportArtifact {
    pub file_name: String,
    pub mime_type: &'static str,
    pub bytes: Vec<u8>,
}

/// Suggested portable name for `scope`.
pub fn default_export_file_name(state: &EditorState, scope: &EditorExportScope) -> String {
    match scope {
        EditorExportScope::Configured => {
            op_editor_core::export_name::default_export_file_name(state)
        }
        EditorExportScope::DeckBoards(_) => format!(
            "{}-slides.pdf",
            op_editor_core::export_name::document_export_stem(state)
        ),
    }
}

/// Render the current editor state into one native-shell-friendly artifact.
pub fn export_editor_state(
    state: &EditorState,
    scope: &EditorExportScope,
) -> Result<ExportArtifact, ExportError> {
    let (mime_type, bytes) = match scope {
        EditorExportScope::DeckBoards(boards) => (
            "application/pdf",
            render_deck_pdf_boards_bytes(state, boards)?,
        ),
        EditorExportScope::Configured => render_configured(state)?,
    };
    Ok(ExportArtifact {
        file_name: default_export_file_name(state, scope),
        mime_type,
        bytes,
    })
}

/// Render and write the configured artifact to `target`.
pub fn export_editor_state_to_path(
    state: &EditorState,
    scope: &EditorExportScope,
    target: &Path,
) -> Result<(), ExportError> {
    let artifact = export_editor_state(state, scope)?;
    std::fs::write(target, artifact.bytes).map_err(|e| ExportError::Write(e.to_string()))
}

fn render_configured(state: &EditorState) -> Result<(&'static str, Vec<u8>), ExportError> {
    let format = state.editor_ui.export_format;
    if format == ExportFormat::Webp && cfg!(any(target_os = "ios", target_os = "android")) {
        return Err(ExportError::UnsupportedFormat { format: "WEBP" });
    }
    if format == ExportFormat::Pdf {
        let bytes = if state.editor_ui.scenario == Some(TemplateScene::Slides) {
            render_deck_pdf_bytes(state)?
        } else {
            let scene = op_pen_loader::editor_state_to_layout_scene(state);
            render_pdf_bytes(&scene)?
        };
        return Ok(("application/pdf", bytes));
    }

    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let page = scene.active_page().ok_or(ExportError::NoActivePage)?;
    let single_node = (state.selection_count() == 1 && state.selection.anchor.is_real())
        .then(|| state.selection.anchor.as_str());
    let scale = state.editor_ui.export_scale;
    match format {
        ExportFormat::Png => Ok((
            "image/png",
            render_raster(page, single_node, RasterFormat::Png, scale)?,
        )),
        ExportFormat::Jpeg => Ok((
            "image/jpeg",
            render_raster(page, single_node, RasterFormat::Jpeg, scale)?,
        )),
        ExportFormat::Webp => Ok((
            "image/webp",
            render_raster(page, single_node, RasterFormat::Webp, scale)?,
        )),
        ExportFormat::Svg => {
            let bytes = match single_node {
                Some(id) => render_node_svg_bytes(&scene, id)?,
                None => render_svg_bytes(&scene)?,
            };
            Ok(("image/svg+xml", bytes))
        }
        ExportFormat::Pdf => unreachable!("PDF returned before active-page scene construction"),
    }
}

fn render_raster(
    page: &op_editor_ui::layout_scene::ScenePage,
    single_node: Option<&str>,
    format: RasterFormat,
    scale: f32,
) -> Result<Vec<u8>, ExportError> {
    match single_node {
        Some(id) => render_node_on_page_raster_bytes(page, id, format, scale),
        None => render_page_raster_bytes(page, format, scale),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EditorState {
        let doc = jian_ops_schema::load_str(
            r##"{"version":"1.0.0","children":[
                {"type":"rectangle","id":"r1","name":"Card","x":0,"y":0,
                 "width":40,"height":30,"fill":[{"type":"solid","color":"#3366ff"}]}
            ]}"##,
        )
        .expect("fixture parses")
        .value;
        let mut state = EditorState::from_document(doc);
        state.editor_ui.file_name_display = Some("mobile.op".into());
        state.editor_ui.export_scale = 1.0;
        state
    }

    fn artifact(format: ExportFormat) -> ExportArtifact {
        let mut state = state();
        state.editor_ui.export_format = format;
        export_editor_state(&state, &EditorExportScope::Configured).expect("export")
    }

    #[test]
    fn configured_formats_emit_their_file_magic_and_names() {
        let png = artifact(ExportFormat::Png);
        assert_eq!(&png.bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(png.file_name, "mobile.png");
        assert_eq!(png.mime_type, "image/png");

        let jpeg = artifact(ExportFormat::Jpeg);
        assert_eq!(&jpeg.bytes[..3], &[0xff, 0xd8, 0xff]);
        assert_eq!(jpeg.file_name, "mobile.jpg");

        let svg = artifact(ExportFormat::Svg);
        assert!(svg.bytes.starts_with(b"<svg "));
        assert_eq!(svg.file_name, "mobile.svg");

        let pdf = artifact(ExportFormat::Pdf);
        assert!(pdf.bytes.starts_with(b"%PDF-"));
        assert_eq!(pdf.file_name, "mobile.pdf");
    }

    #[cfg(any(target_os = "ios", target_os = "android"))]
    #[test]
    fn mobile_default_reports_webp_as_unsupported_before_rendering() {
        let mut state = state();
        state.editor_ui.export_format = ExportFormat::Webp;
        let err = export_editor_state(&state, &EditorExportScope::Configured).unwrap_err();
        assert_eq!(err, ExportError::UnsupportedFormat { format: "WEBP" });
        assert_eq!(
            err.to_string(),
            "WEBP export is not supported by this build"
        );
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    #[test]
    fn desktop_keeps_the_historical_webp_encoder_path() {
        let mut state = state();
        state.editor_ui.export_format = ExportFormat::Webp;
        let result = export_editor_state(&state, &EditorExportScope::Configured);
        assert!(
            !matches!(result, Err(ExportError::UnsupportedFormat { .. })),
            "desktop must continue asking Skia to encode WebP"
        );
    }
}
