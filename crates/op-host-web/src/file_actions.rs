// Browser boundary: these pure helpers are unit-tested; DOM picker/download
// behavior lives in dom_io and still needs browser smoke.
//! Pure (DOM-free) document IO helpers behind the browser file-IO
//! glue (`dom_io`). Everything here is plain Rust over
//! `op_editor_core` / `op_pen_loader` / `op_figma`, so the unit tests
//! exercise the exact serialize / ingest paths the browser uses
//! without a DOM.
//!
//! Mirrors the desktop's `persistence.rs` split: Save serializes
//! `editor_state.doc` straight to canonical `.op` JSON. When the
//! web-canvas daemon has a bound local path it performs the real
//! desktop write, including the `.opmeta` active-page sidecar; the
//! browser fallback downloads the same JSON as a Blob. Open parses
//! the canonical schema via `op_pen_loader::load_canonical`, then
//! re-seeds `EditorState` through `EditorState::from_document`.

use base64::Engine as _;
use op_editor_core::editor_ui_state::ExportFormat;
use op_editor_core::{uikit_io, EditorState, UIKit};

mod drop_plan;
mod export_error;
mod image_fill_upload;
mod ingest_error;
mod ingested_doc;
mod save_payload;
mod save_queue;

pub use drop_plan::{drop_batch_plan, drop_kind, DropBatchPlan, DropKind};
pub use export_error::DocumentExportError;
pub use image_fill_upload::apply_fill_image_data_url;
pub use ingest_error::DocumentIngestError;
pub use ingested_doc::IngestedDoc;
pub use save_payload::{
    acknowledge_browser_download, parse_save_response, save_ack_matches_document, save_file_name,
    save_snapshot_matches_document, serialize_save_payload, SavePayloadTarget,
};
#[cfg(test)]
pub use save_payload::{save_request_body, serialize_document};
pub use save_queue::LatestSaveQueue;

/// Name the browser download for the current export gets.
///
/// Delegates to the shared derivation the desktop save dialog
/// pre-fills, so the same document and selection produce the same
/// `<document>-<node>.<ext>` on both hosts. The daemon's response
/// still carries a `fileName`, but that is a generic fallback for
/// clients without an editor state — this host has one.
pub fn export_download_file_name(state: &EditorState) -> String {
    op_editor_core::export_name::default_export_file_name(state)
}

pub fn export_svg_document(state: &EditorState) -> Result<String, DocumentExportError> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    if state.selection_count() == 1 && state.selection.anchor.is_real() {
        Ok(op_editor_ui::svg_export::serialize_node_svg(
            &scene,
            state.selection.anchor.as_str(),
        )?)
    } else {
        Ok(op_editor_ui::svg_export::serialize_active_page_svg(&scene)?)
    }
}

#[derive(Debug)]
pub struct PdfDownload {
    /// The daemon's generic name. Read only by the deck-selection
    /// download, which narrows the export to a board list this host
    /// has no name for; the export dialog's PDF overrides it with
    /// [`export_download_file_name`].
    pub file_name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

/// A raster export the daemon rendered.
///
/// Deliberately carries no file name: the daemon's `fileName` is the
/// generic fallback for clients that post a document without holding
/// an editor state, and this host holds one — every raster download
/// is named by [`export_download_file_name`] from the live document
/// and selection. Parsing the field only to discard it would leave two
/// candidate names for one file, which is how the desktop and browser
/// names drifted apart in the first place.
#[derive(Debug)]
pub struct RasterDownload {
    pub mime: String,
    pub bytes: Vec<u8>,
}

fn document_value_with_editor_meta(
    state: &EditorState,
) -> Result<serde_json::Value, DocumentExportError> {
    let mut document = serde_json::to_value(&state.doc)
        .map_err(|e| DocumentExportError::SerializeDocument(e.to_string()))?;
    let Some(object) = document.as_object_mut() else {
        return Err(DocumentExportError::DocumentNotObject);
    };
    object.insert(
        "editorMeta".to_string(),
        serde_json::to_value(op_pen_loader::EditorMeta::from_state(state))
            .map_err(|e| DocumentExportError::SerializeEditorMeta(e.to_string()))?,
    );
    Ok(document)
}

/// Body for the daemon's PDF route.
///
/// `boards` narrows a deck export to those board ids; `None` means the
/// whole active page, which is what every entry point but the slides
/// rail's "Export selected slides" row asks for. It is sent explicitly
/// rather than re-derived daemon-side because the daemon rebuilds its
/// `EditorState` from the posted document and that round trip does not
/// carry the browser's selection — so the selection has to travel as
/// data or it does not travel at all.
pub fn export_pdf_request_body(
    state: &EditorState,
    boards: Option<&[String]>,
) -> Result<String, DocumentExportError> {
    let document = document_value_with_editor_meta(state)?;
    let mut body = serde_json::json!({
        "document": document,
        "activePageIndex": state.ui.active_page_index,
    });
    if let Some(boards) = boards {
        body["boards"] = serde_json::json!(boards);
    }
    serde_json::to_string(&body).map_err(|e| DocumentExportError::SerializeRequest(e.to_string()))
}

pub fn parse_pdf_download_response(response: &str) -> Result<PdfDownload, DocumentExportError> {
    let parsed: serde_json::Value = serde_json::from_str(response)
        .map_err(|e| DocumentExportError::ResponseParse(e.to_string()))?;
    if !parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let message = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("PDF export failed");
        return Err(DocumentExportError::Daemon(message.to_string()));
    }
    let file_name = parsed
        .get("fileName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("openpencil-export.pdf")
        .to_string();
    let mime = parsed
        .get("mime")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("application/pdf")
        .to_string();
    let data = parsed
        .get("dataBase64")
        .and_then(|v| v.as_str())
        .ok_or(DocumentExportError::PdfMissingData)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| DocumentExportError::PdfDecode(e.to_string()))?;
    Ok(PdfDownload {
        file_name,
        mime,
        bytes,
    })
}

pub fn export_raster_request_body(state: &EditorState) -> Result<String, DocumentExportError> {
    let format = match state.editor_ui.export_format {
        ExportFormat::Png => "png",
        ExportFormat::Jpeg => "jpeg",
        ExportFormat::Webp => "webp",
        ExportFormat::Svg | ExportFormat::Pdf => {
            return Err(DocumentExportError::RasterFormatUnsupported);
        }
    };
    let document = document_value_with_editor_meta(state)?;
    let mut body = serde_json::json!({
        "document": document,
        "activePageIndex": state.ui.active_page_index,
        "format": format,
        "scale": state.editor_ui.export_scale,
    });
    if state.selection_count() == 1 && state.selection.anchor.is_real() {
        body["selectedNodeId"] =
            serde_json::Value::String(state.selection.anchor.as_str().to_string());
    }
    serde_json::to_string(&body).map_err(|e| DocumentExportError::SerializeRequest(e.to_string()))
}

pub fn parse_raster_download_response(
    response: &str,
) -> Result<RasterDownload, DocumentExportError> {
    let parsed: serde_json::Value = serde_json::from_str(response)
        .map_err(|e| DocumentExportError::ResponseParse(e.to_string()))?;
    if !parsed.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let message = parsed
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Raster export failed");
        return Err(DocumentExportError::Daemon(message.to_string()));
    }
    // `fileName` is present in the response and intentionally not read
    // here — see [`RasterDownload`].
    let mime = parsed
        .get("mime")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("image/png")
        .to_string();
    let data = parsed
        .get("dataBase64")
        .and_then(|v| v.as_str())
        .ok_or(DocumentExportError::RasterMissingData)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| DocumentExportError::RasterDecode(e.to_string()))?;
    Ok(RasterDownload { mime, bytes })
}

pub fn apply_open_recent_response(
    state: &mut EditorState,
    path: &str,
    response: &str,
    now_secs: u64,
) -> bool {
    let parsed: Option<serde_json::Value> = serde_json::from_str(response).ok();
    let ok = parsed
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if ok {
        state.editor_ui.file_name_display = Some(path_file_name(path).to_string());
        state
            .editor_ui
            .touch_recent_file(path.to_string(), now_secs);
        return true;
    }
    let pruned = parsed
        .as_ref()
        .and_then(|v| v.get("pruned"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    pruned && state.editor_ui.remove_recent_file(path)
}

fn path_file_name(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or(path)
}

pub struct KitExport {
    pub file_name: String,
    pub json: String,
}

pub fn export_kit_document(state: &EditorState) -> Result<Option<KitExport>, DocumentExportError> {
    let kit_name = state
        .doc
        .name
        .clone()
        .unwrap_or_else(|| "My Kit".to_string());
    let Some(kit_doc) = uikit_io::build_kit_document(&state.doc, &[], &kit_name) else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(&kit_doc)
        .map_err(|e| DocumentExportError::SerializeKit(e.to_string()))?;
    Ok(Some(KitExport {
        file_name: uikit_io::kit_export_file_name(&kit_name),
        json,
    }))
}

pub fn import_kit_source(src: &str, kit_id: String) -> Result<Option<UIKit>, DocumentIngestError> {
    let loaded = op_pen_loader::load_canonical(src)
        .map_err(|e| DocumentIngestError::LoadCanonical(e.to_string()))?;
    Ok(uikit_io::import_kit_from_document(&loaded.value, kit_id))
}

/// Parse canonical `.op` / `.pen` source into a fresh `EditorState`,
/// carrying over the app-level preferences from `previous` (the
/// state being replaced). Mirrors the desktop's
/// `persistence::load_editor_state` + `preserve_app_preferences`
/// pair, minus the legacy `.opmeta` sidecar. Embedded `editorMeta`
/// restores the active page and Figma Preserve geometry mode; older files
/// without it open on their first non-empty page, matching desktop.
pub fn ingest_op_source(
    src: &str,
    previous: &EditorState,
) -> Result<IngestedDoc, DocumentIngestError> {
    let editor_meta = op_pen_loader::extract_editor_meta(src);
    let loaded = op_pen_loader::load_canonical(src)
        .map_err(|e| DocumentIngestError::LoadCanonical(e.to_string()))?;
    let warnings = loaded.warnings.iter().map(|w| format!("{w:?}")).collect();
    let mut state = EditorState::from_document(loaded.value);
    op_pen_loader::apply_editor_meta_or_legacy_fallback(&mut state, editor_meta);
    preserve_app_preferences(previous, &mut state);
    Ok(IngestedDoc::new(state, warnings))
}

/// Parse a binary Figma `.fig` export into a fresh `EditorState`.
/// Mirrors the desktop's `figma_import_session::parse_path` body
/// (Preserve layout mode + `preserve_authored_geometry`); the wasm32
/// build has no worker threads, so the caller runs this on the main
/// thread after the async `FileReader` read completes.
pub fn ingest_figma_bytes(
    bytes: &[u8],
    file_name: &str,
) -> Result<IngestedDoc, DocumentIngestError> {
    let import = op_figma::parse_fig_binary(bytes, file_name, op_figma::FigLayoutMode::Preserve)
        .map_err(|e| DocumentIngestError::FigmaParse(e.to_string()))?;
    let mut state = EditorState::from_document(import.document);
    state.editor_ui.preserve_authored_geometry = true;
    Ok(IngestedDoc::new(state, import.warnings))
}

/// Install payload returned by the isolated Figma import Worker.
///
/// `source` is a complete, ordinary canonical `.op` document (including the
/// shared `images` / `imageThumbs` tables produced in the Worker). Routing it
/// through the compatibility loader preserves the exact same old-schema and
/// image-interning behavior as File → Open; no paged placeholder document is
/// ever exposed to `EditorState` in this first phase.
pub fn ingest_figma_temp_source(
    source: &str,
    worker_warnings_json: &str,
) -> Result<IngestedDoc, DocumentIngestError> {
    let loaded = op_pen_loader::load_canonical(source)
        .map_err(|error| DocumentIngestError::LoadCanonical(error.to_string()))?;
    let mut warnings: Vec<String> = serde_json::from_str(worker_warnings_json)
        .map_err(|error| DocumentIngestError::WorkerWarningsParse(error.to_string()))?;
    warnings.extend(loaded.warnings.iter().map(|warning| format!("{warning:?}")));
    let mut state = EditorState::from_document(loaded.value);
    state.editor_ui.preserve_authored_geometry = true;
    Ok(IngestedDoc::new(state, warnings))
}

#[cfg(test)]
fn ingest_html_project(
    files: &[op_html::HtmlProjectFile],
) -> Result<IngestedDoc, DocumentIngestError> {
    let imported = op_html::import_html_project_document(files, &Default::default())
        .map_err(|error| DocumentIngestError::HtmlProject(error.to_string()))?;
    if imported.document.children.is_empty() {
        return Err(DocumentIngestError::HtmlProjectEmpty);
    }
    Ok(IngestedDoc::from_html(
        EditorState::from_document(imported.document),
        imported.warnings,
        &imported.diagnostics,
    ))
}

/// Carry app-level preferences from the state being replaced into a
/// freshly ingested one. Port of the desktop's
/// `persistence::preserve_app_preferences` (private there, so
/// duplicated rather than shared — the desktop crate doesn't compile
/// for wasm32).
pub fn preserve_app_preferences(previous: &EditorState, next: &mut EditorState) {
    let previous_selected_model = previous.chat.selected_model_entry().cloned();
    next.editor_ui.theme_mode = previous.editor_ui.theme_mode;
    next.editor_ui.host_theme_override = previous.editor_ui.host_theme_override;
    next.editor_ui.locale = previous.editor_ui.locale;
    next.editor_ui.host_locale_override = previous.editor_ui.host_locale_override;
    next.editor_ui.recent_files = previous.editor_ui.recent_files.clone();
    next.editor_ui.font_import_supported = previous.editor_ui.font_import_supported;
    next.editor_ui.scene_template_center.save_current_supported = previous
        .editor_ui
        .scene_template_center
        .save_current_supported;
    next.editor_ui.system_fonts_loaded = previous.editor_ui.system_fonts_loaded;
    next.editor_ui.system_font_families = previous.editor_ui.system_font_families.clone();
    next.editor_ui.bundled_font_families = previous.editor_ui.bundled_font_families.clone();
    next.editor_ui.imported_font_families = previous.editor_ui.imported_font_families.clone();
    next.editor_ui.prompt_center.custom_prompts =
        previous.editor_ui.prompt_center.custom_prompts.clone();
    next.editor_ui.prompt_center.custom_store_writable =
        previous.editor_ui.prompt_center.custom_store_writable;
    next.editor_ui.prompt_center.custom_store_dirty =
        previous.editor_ui.prompt_center.custom_store_dirty;
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
#[path = "file_actions_tests.rs"]
mod tests;
