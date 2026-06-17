// UNVERIFIED on wasm32: needs EMSDK build + browser; run tools/check-wasm-bundle.sh
//! Pure (DOM-free) document IO helpers behind the browser file-IO
//! glue (`dom_io`). Everything here is plain Rust over
//! `op_editor_core` / `op_pen_loader` / `op_figma`, so the unit tests
//! exercise the exact serialize / ingest paths the browser uses
//! without a DOM.
//!
//! Mirrors the desktop's `persistence.rs` split: Save serializes
//! `editor_state.doc` straight to the canonical `.op` JSON
//! (`serde_json::to_string_pretty`, byte-identical to the desktop
//! writer minus the `.opmeta` sidecar — the web has no sidecar
//! because there is no path to re-read it from), and Open parses the
//! canonical schema via the same `op_pen_loader::load_canonical`
//! wrapper, then re-seeds `EditorState` through
//! `EditorState::from_document`.

use op_editor_core::editor_ui_state::ExportFormat;
use op_editor_core::{uikit_io, EditorState, UIKit};

/// An ingested document plus the loader's best-effort schema
/// warnings (the desktop logs these to stderr; the web glue routes
/// them to `console.warn`).
pub struct IngestedDoc {
    pub state: EditorState,
    pub warnings: Vec<String>,
}

/// Serialize the canonical document to the `.op` JSON the desktop /
/// TS editors write. The web Save path downloads this as a Blob.
pub fn serialize_document(state: &EditorState) -> Result<String, String> {
    serde_json::to_string_pretty(&state.doc).map_err(|e| e.to_string())
}

/// Download file name for Save / Save As — the current display name
/// when one is set (a previously opened file), else `untitled.op`.
pub fn save_file_name(state: &EditorState) -> String {
    match state.editor_ui.file_name_display.as_deref() {
        Some(name) if !name.trim().is_empty() => name.to_string(),
        _ => "untitled.op".to_string(),
    }
}

/// Map an export-dialog format onto the browser encoder target:
/// `(mime, extension, natively_supported)`. The browser's
/// `HtmlCanvasElement::to_data_url` encodes PNG / JPEG / WEBP only;
/// SVG / PDF have no web encoder yet, so they degrade to a PNG
/// raster (`natively_supported == false` lets the caller log the
/// degradation).
pub fn export_target(format: ExportFormat) -> (&'static str, &'static str, bool) {
    match format {
        ExportFormat::Png => ("image/png", "png", true),
        ExportFormat::Jpeg => ("image/jpeg", "jpg", true),
        ExportFormat::Webp => ("image/webp", "webp", true),
        ExportFormat::Svg | ExportFormat::Pdf => ("image/png", "png", false),
    }
}

pub struct KitExport {
    pub file_name: String,
    pub json: String,
}

pub fn export_kit_document(state: &EditorState) -> Result<Option<KitExport>, String> {
    let kit_name = state
        .doc
        .name
        .clone()
        .unwrap_or_else(|| "My Kit".to_string());
    let Some(kit_doc) = uikit_io::build_kit_document(&state.doc, &[], &kit_name) else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&kit_doc).map_err(|e| e.to_string())?;
    Ok(Some(KitExport {
        file_name: uikit_io::kit_export_file_name(&kit_name),
        json,
    }))
}

pub fn import_kit_source(src: &str, kit_id: String) -> Result<Option<UIKit>, String> {
    let loaded = op_pen_loader::load_canonical(src).map_err(|e| e.to_string())?;
    Ok(uikit_io::import_kit_from_document(&loaded.value, kit_id))
}

/// Parse canonical `.op` / `.pen` source into a fresh `EditorState`,
/// carrying over the app-level preferences from `previous` (the
/// state being replaced). Mirrors the desktop's
/// `persistence::load_editor_state` + `preserve_app_preferences`
/// pair, minus the `.opmeta` sidecar (no filesystem on web — the
/// loaded document starts on page 0).
pub fn ingest_op_source(src: &str, previous: &EditorState) -> Result<IngestedDoc, String> {
    let loaded = op_pen_loader::load_canonical(src).map_err(|e| e.to_string())?;
    let warnings = loaded.warnings.iter().map(|w| format!("{w:?}")).collect();
    let mut state = EditorState::from_document(loaded.value);
    preserve_app_preferences(previous, &mut state);
    Ok(IngestedDoc { state, warnings })
}

/// Parse a binary Figma `.fig` export into a fresh `EditorState`.
/// Mirrors the desktop's `figma_import_session::parse_path` body
/// (Preserve layout mode + `preserve_authored_geometry`); the wasm32
/// build has no worker threads, so the caller runs this on the main
/// thread after the async `FileReader` read completes.
pub fn ingest_figma_bytes(bytes: &[u8], file_name: &str) -> Result<IngestedDoc, String> {
    let import = op_figma::parse_fig_binary(bytes, file_name, op_figma::FigLayoutMode::Preserve)
        .map_err(|e| e.to_string())?;
    let mut state = EditorState::from_document(import.document);
    state.editor_ui.preserve_authored_geometry = true;
    Ok(IngestedDoc {
        state,
        warnings: import.warnings,
    })
}

/// Carry app-level preferences from the state being replaced into a
/// freshly ingested one. Port of the desktop's
/// `persistence::preserve_app_preferences` (private there, so
/// duplicated rather than shared — the desktop crate doesn't compile
/// for wasm32).
pub fn preserve_app_preferences(previous: &EditorState, next: &mut EditorState) {
    let previous_selected_model = previous.chat.selected_model_entry().cloned();
    next.editor_ui.theme_mode = previous.editor_ui.theme_mode;
    next.editor_ui.locale = previous.editor_ui.locale;
    next.editor_ui.recent_files = previous.editor_ui.recent_files.clone();
    next.editor_ui.agent_settings = previous.editor_ui.agent_settings.clone();
    next.editor_ui.chat_selected_agent = previous.editor_ui.chat_selected_agent;
    next.chat.discovered_models = previous.chat.discovered_models.clone();
    next.rebuild_chat_models();
    if let Some(prev) = previous_selected_model {
        if let Some(idx) = next.chat.available_models.iter().position(|m| {
            m.provider == prev.provider
                && m.value == prev.value
                && m.builtin_provider_id == prev.builtin_provider_id
        }) {
            next.select_chat_model(idx);
        }
    }
}

/// What a picked / dropped file is, by extension (case-insensitive
/// — matches the desktop's `is_supported_document` /
/// `is_supported_figma_import` semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropKind {
    /// Canonical `.op` / `.pen` document — replaces the open document.
    Document,
    /// Binary Figma `.fig` export — imported via `op_figma`.
    Figma,
    /// SVG file — parsed into editable nodes via `import_svg_named`.
    Svg,
    /// Raster image — inserted as an Image node carrying a data URL.
    Image,
    /// Anything else — ignored with a console warning.
    Unsupported,
}

/// Classify a file name for the drop / picker router.
pub fn drop_kind(name: &str) -> DropKind {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".op") || lower.ends_with(".pen") {
        DropKind::Document
    } else if lower.ends_with(".fig") {
        DropKind::Figma
    } else if lower.ends_with(".svg") {
        DropKind::Svg
    } else if [".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
    {
        DropKind::Image
    } else {
        DropKind::Unsupported
    }
}

/// File name without its last extension — node naming for inserted
/// images / SVGs (mirrors the desktop's `Path::file_stem` usage; the
/// browser only hands us a flat name string).
pub fn file_stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(idx) if idx > 0 => &name[..idx],
        _ => name,
    }
}

/// Best-effort MIME type for a chat attachment picked in the browser.
/// Mirrors the desktop attachment helper's extension table.
pub fn attachment_media_type_for_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    match lower.rsplit('.').next() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Browser `File.name` is already pathless in normal cases, but keep
/// the same separator hardening as desktop temp-file staging.
pub fn attachment_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c == '/' || c == '\\' { '_' } else { c })
        .collect();
    if cleaned.trim().is_empty() {
        "attachment".to_string()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::PenNodeExt;

    #[test]
    fn attachment_media_type_matches_desktop_image_extensions() {
        assert_eq!(attachment_media_type_for_name("a.png"), "image/png");
        assert_eq!(attachment_media_type_for_name("a.JPG"), "image/jpeg");
        assert_eq!(attachment_media_type_for_name("a.jpeg"), "image/jpeg");
        assert_eq!(attachment_media_type_for_name("a.gif"), "image/gif");
        assert_eq!(attachment_media_type_for_name("a.webp"), "image/webp");
        assert_eq!(attachment_media_type_for_name("a.svg"), "image/svg+xml");
        assert_eq!(
            attachment_media_type_for_name("notes.txt"),
            "application/octet-stream"
        );
    }

    #[test]
    fn attachment_file_name_strips_path_separators() {
        assert_eq!(attachment_file_name("../a.png"), ".._a.png");
        assert_eq!(attachment_file_name("folder\\a.png"), "folder_a.png");
        assert_eq!(attachment_file_name(""), "attachment");
    }

    #[test]
    fn export_kit_document_builds_download_name_and_json() {
        let src = r#"{"version":"0.8.0","name":"My Kit!","children":[{"type":"frame","id":"button","name":"Primary Button","reusable":true,"x":0,"y":0,"width":120,"height":40,"children":[]}]}"#;
        let doc = op_pen_loader::load_canonical(src)
            .expect("canonical doc")
            .value;
        let state = EditorState::from_document(doc);

        let export = export_kit_document(&state)
            .expect("export encodes")
            .expect("document has reusable components");

        assert_eq!(export.file_name, "My Kit.op");
        let parsed: jian_ops_schema::PenDocument =
            serde_json::from_str(&export.json).expect("kit json");
        assert_eq!(parsed.name.as_deref(), Some("My Kit!"));
        assert_eq!(parsed.children.len(), 1);
        assert_eq!(parsed.children[0].base().id, "button");
    }

    #[test]
    fn import_kit_source_extracts_components_with_supplied_id() {
        let src = r#"{"version":"0.8.0","name":"Imported System","children":[{"type":"frame","id":"card","name":"Profile Card","reusable":true,"x":0,"y":0,"width":240,"height":120,"children":[]}]}"#;

        let kit = import_kit_source(src, "web-kit-1".to_string())
            .expect("import parses")
            .expect("source has reusable components");

        assert_eq!(kit.id, "web-kit-1");
        assert_eq!(kit.name, "Imported System");
        assert_eq!(kit.components.len(), 1);
        assert_eq!(kit.components[0].id, "card");
    }
}
