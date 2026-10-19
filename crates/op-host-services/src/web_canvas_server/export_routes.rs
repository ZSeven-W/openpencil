//! Export + save routes for the web-canvas daemon: `POST /api/export/raster`,
//! `POST /api/export/pdf`, and `POST /api/file/save`. Split out of
//! `web_canvas_server.rs` to keep the spine under the 800-line cap; the
//! spine re-exports these so every existing path keeps resolving.

use super::*;

pub(super) struct RasterDownload {
    file_name: &'static str,
    mime: &'static str,
    bytes: Vec<u8>,
}

pub(super) fn export_raster_download(body: &str, state: &WebCanvasState) -> WebReply {
    match build_raster_download(body, &state.editor) {
        Ok(download) => WebReply {
            status: "200 OK",
            body: serde_json::json!({
                "ok": true,
                "fileName": download.file_name,
                "mime": download.mime,
                "dataBase64": base64::engine::general_purpose::STANDARD.encode(download.bytes),
            })
            .to_string(),
        },
        Err(e) => WebReply {
            status: e.http_status(),
            body: crate::mcp_serve::rest_error_body(&format!("export raster failed: {e}")),
        },
    }
}

pub(super) fn build_raster_download(body: &str, fallback: &EditorState) -> Result<RasterDownload> {
    let parsed = parse_export_body(body)?;
    let mut editor = export_editor_from_value(parsed.as_ref(), fallback)?;
    let (format, file_name, mime, ext) = raster_format_from_export_body(parsed.as_ref())?;
    let scale = parsed
        .as_ref()
        .and_then(|body| body.get("scale"))
        .and_then(|scale| scale.as_f64())
        .map(|scale| scale as f32)
        .unwrap_or(editor.editor_ui.export_scale);
    let selected_node_id = selected_node_id_from_export_body(parsed.as_ref(), &editor);
    if let Some(id) = &selected_node_id {
        editor.selection.anchor = op_editor_core::NodeId::new(id.clone());
        editor.selection.set = vec![editor.selection.anchor.clone()];
    }
    let scene = op_pen_loader::editor_state_to_active_page_export_layout_scene(&editor);
    let tmp = tmp_export_path(ext);
    let result = match selected_node_id {
        Some(id) => crate::export::export_node_raster(&scene, &id, &tmp, format, scale),
        None => crate::export::export_raster(&scene, &tmp, format, scale),
    };
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    let bytes = std::fs::read(&tmp).map_err(|e| WebCanvasError::Io(e.to_string()))?;
    let _ = std::fs::remove_file(&tmp);
    Ok(RasterDownload {
        file_name,
        mime,
        bytes,
    })
}

pub(super) fn raster_format_from_export_body(
    body: Option<&serde_json::Value>,
) -> Result<(
    crate::export::RasterFormat,
    &'static str,
    &'static str,
    &'static str,
)> {
    let format = body
        .and_then(|body| body.get("format"))
        .and_then(|format| format.as_str())
        .unwrap_or("png");
    match format {
        "png" => Ok((
            crate::export::RasterFormat::Png,
            "openpencil-export.png",
            "image/png",
            "png",
        )),
        "jpeg" | "jpg" => Ok((
            crate::export::RasterFormat::Jpeg,
            "openpencil-export.jpg",
            "image/jpeg",
            "jpg",
        )),
        "webp" => Ok((
            crate::export::RasterFormat::Webp,
            "openpencil-export.webp",
            "image/webp",
            "webp",
        )),
        other => Err(WebCanvasError::BadRequest(format!(
            "unsupported raster format: {other}"
        ))),
    }
}

pub(super) fn selected_node_id_from_export_body(
    body: Option<&serde_json::Value>,
    editor: &EditorState,
) -> Option<String> {
    body.and_then(|body| body.get("selectedNodeId"))
        .and_then(|node_id| node_id.as_str())
        .filter(|node_id| !node_id.trim().is_empty())
        .map(|node_id| node_id.to_string())
        .or_else(|| {
            if editor.selection_count() == 1 && editor.selection.anchor.is_real() {
                Some(editor.selection.anchor.as_str().to_string())
            } else {
                None
            }
        })
}

/// The `boards` filter a PDF request may carry, or `None` when it asks for
/// the whole active page.
///
/// An empty array is deliberately NOT `None`: it means "the caller narrowed
/// this to nothing", which the exporter answers with `NothingToExport` — a
/// missing field, which means "no narrowing at all", must not collapse onto
/// the same reading and silently ship the entire deck.
fn boards_from_export_body(body: &str) -> Option<Vec<String>> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let boards = parsed.get("boards")?.as_array()?;
    Some(
        boards
            .iter()
            .filter_map(|id| id.as_str())
            .filter(|id| !id.trim().is_empty())
            .map(|id| id.to_string())
            .collect(),
    )
}

pub(super) fn export_pdf_download(body: &str, state: &WebCanvasState) -> WebReply {
    match build_pdf_download(body, &state.editor) {
        Ok(bytes) => WebReply {
            status: "200 OK",
            body: serde_json::json!({
                "ok": true,
                "fileName": "openpencil-export.pdf",
                "mime": "application/pdf",
                "dataBase64": base64::engine::general_purpose::STANDARD.encode(bytes),
            })
            .to_string(),
        },
        Err(e) => WebReply {
            status: e.http_status(),
            body: crate::mcp_serve::rest_error_body(&format!("export PDF failed: {e}")),
        },
    }
}

pub(super) fn build_pdf_download(body: &str, fallback: &EditorState) -> Result<Vec<u8>> {
    let editor = export_editor_from_body(body, fallback)?;
    let boards = boards_from_export_body(body);
    let tmp = tmp_export_path("pdf");
    // An explicit board list means the caller narrowed the export itself —
    // the slides rail's "Export selected slides" row. It wins over the
    // scenario branch below because the browser's SELECTION cannot survive
    // the document round-trip, so the ids are the only record of it that
    // reached this process.
    if let Some(boards) = boards {
        crate::export_pdf::export_deck_pdf_boards(&editor, &tmp, &boards)?;
    } else if editor.editor_ui.scenario
        == Some(op_editor_core::scene_template_catalog::TemplateScene::Slides)
        || (editor.editor_ui.workspace.visible
            && editor.editor_ui.workspace.family == op_editor_core::HomeFamily::AppUi
            && !op_editor_core::preview_slideshow::active_page_boards(&editor).is_empty())
    {
        // The scenario tag survives the round-trip through `editorMeta`, so a
        // deck posted from the browser must get the same slide-per-page file
        // the desktop writes rather than one sheet holding every board.
        crate::export_pdf::export_deck_pdf(&editor, &tmp)?;
    } else {
        let scene = op_pen_loader::editor_state_to_export_layout_scene(&editor);
        crate::export_pdf::export_pdf(&scene, &tmp)?;
    }
    let bytes = std::fs::read(&tmp).map_err(|e| WebCanvasError::Io(e.to_string()))?;
    let _ = std::fs::remove_file(&tmp);
    Ok(bytes)
}

pub(super) fn export_editor_from_body(body: &str, fallback: &EditorState) -> Result<EditorState> {
    let parsed = parse_export_body(body)?;
    export_editor_from_value(parsed.as_ref(), fallback)
}

pub(super) fn export_editor_from_value(
    body: Option<&serde_json::Value>,
    fallback: &EditorState,
) -> Result<EditorState> {
    let full_content = body
        .and_then(|body| body.get("fullContent"))
        .and_then(|value| value.as_bool());
    let Some(doc) = body.and_then(|body| body.get("document")) else {
        let mut editor = fallback.clone();
        if let Some(full_content) = full_content {
            editor.editor_ui.workspace.visible = full_content;
        }
        return Ok(editor);
    };
    if !doc.is_object() {
        return Err(WebCanvasError::BadRequest(
            "document must be an object".into(),
        ));
    }
    let src = serde_json::to_string(doc).map_err(|e| WebCanvasError::BadRequest(e.to_string()))?;
    let editor_meta = op_pen_loader::extract_editor_meta(&src);
    let loaded =
        op_pen_loader::load_canonical(&src).map_err(|e| WebCanvasError::Document(e.to_string()))?;
    let mut editor = EditorState::from_document(loaded.value);
    if let Some(meta) = editor_meta {
        op_pen_loader::apply_editor_meta(&mut editor, meta);
    }
    if let Some(index) = body
        .and_then(|body| body.get("activePageIndex"))
        .and_then(|index| index.as_u64())
        .map(|index| index as usize)
    {
        let page_count = editor
            .doc
            .pages
            .as_ref()
            .map(|pages| pages.len())
            .unwrap_or(1)
            .max(1);
        editor.ui.active_page_index = index.min(page_count - 1);
    }
    // Export posture travels with this request, never in the saved document.
    editor.editor_ui.workspace.visible = full_content.unwrap_or(false);
    if let Some(family) = editor.editor_ui.home.work_family {
        editor.editor_ui.workspace.family = family;
    }
    Ok(editor)
}

pub(super) fn parse_export_body(body: &str) -> Result<Option<serde_json::Value>> {
    if body.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(body)
        .map(Some)
        .map_err(|e| WebCanvasError::BadRequest(format!("parse request body: {e}")))
}

pub(super) fn tmp_export_path(ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "openpencil-web-export-{}-{:?}.{ext}",
        std::process::id(),
        std::thread::current().id()
    ))
}

pub(super) fn save_current_file(body: &str, state: &mut WebCanvasState) -> WebReply {
    let Some(path) = state.current_path.clone() else {
        return WebReply {
            status: "400 Bad Request",
            body: crate::mcp_serve::rest_error_body("No file path is bound to this web session"),
        };
    };
    match save_editor_from_body(body, &state.editor, &path) {
        Ok(next) => {
            state.editor = next;
            state.version += 1;
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "untitled.op".to_string());
            WebReply {
                status: "200 OK",
                body: serde_json::json!({
                    "ok": true,
                    "version": state.version,
                    "fileName": file_name,
                })
                .to_string(),
            }
        }
        Err(e) => WebReply {
            status: e.http_status(),
            body: crate::mcp_serve::rest_error_body(&format!("save failed: {e}")),
        },
    }
}

pub(super) fn save_editor_from_body(
    body: &str,
    previous: &EditorState,
    path: &std::path::Path,
) -> Result<EditorState> {
    let (doc, active_page_index, editor_meta) = document_and_active_page_from_body(body)?;
    let mut next = previous.clone();
    next.replace_document(doc);
    if let Some(meta) = editor_meta {
        op_pen_loader::apply_editor_meta(&mut next, meta);
    }
    if let Some(index) = active_page_index {
        let page_count = next
            .doc
            .pages
            .as_ref()
            .map(|pages| pages.len())
            .unwrap_or(1)
            .max(1);
        next.ui.active_page_index = index.min(page_count - 1);
    }
    set_file_name_display(&mut next, path);
    crate::doc_io::save_to_path(&next, path)?;
    Ok(next)
}

pub(super) fn document_and_active_page_from_body(
    body: &str,
) -> Result<(
    jian_ops_schema::PenDocument,
    Option<usize>,
    Option<op_pen_loader::EditorMeta>,
)> {
    let parsed = crate::mcp_serve::parse_borrowed_document_envelope(body)
        .map_err(|e| WebCanvasError::BadRequest(e.to_string()))?;
    let Some(doc_json) = parsed.document_json else {
        return Err(WebCanvasError::BadRequest("missing document".into()));
    };
    if !doc_json.trim_start().starts_with('{') {
        return Err(WebCanvasError::BadRequest(
            "document must be an object".into(),
        ));
    }
    let editor_meta = op_pen_loader::extract_editor_meta(doc_json);
    let loaded = op_pen_loader::load_canonical(doc_json)
        .map_err(|e| WebCanvasError::Document(e.to_string()))?;
    for w in &loaded.warnings {
        eprintln!("openpencil-desktop --serve-web: schema warning: {w:?}");
    }
    let active_page_index = parsed.active_page_index.map(|value| value as usize);
    Ok((loaded.value, active_page_index, editor_meta))
}
