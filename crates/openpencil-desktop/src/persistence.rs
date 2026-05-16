//! `.pen` / `.op` file Save / Open — desktop-side dialog flow.
//!
//! Since the native host's single source of truth is
//! `op_editor_core::EditorState` (built on the canonical
//! `jian_ops_schema::PenDocument`), Save serializes `editor_state.doc`
//! straight to the canonical `.op` JSON, and Open parses the
//! canonical schema and re-seeds `EditorState` via
//! `EditorState::from_document`. There is no `Document → PenDocument`
//! reverse path, so the desktop's old private `DocPayload` format is
//! no longer written — every save is canonical, round-tripping
//! through the same parser the TS editor / Jian apps use.
//!
//! ## Editor view-state sidecar (`.opmeta`)
//!
//! The canonical `PenDocument` schema has no field for editor
//! view-state — most of it (selection / viewport / tool) is
//! deliberately transient, but `active_page_index` is a small piece of
//! view-state the user expects to survive a save / load round-trip.
//! `jian_ops_schema` is a shared crate and must not grow editor-only
//! fields, so Save writes a tiny JSON companion file next to the `.op`
//! (`<path>.opmeta`) carrying `active_page_index`; Open reads it
//! best-effort (a missing / unreadable sidecar falls back to page 0).
//! The `.op` file itself stays strictly canonical so TS editor / Jian
//! apps load it unchanged.
//!
//! ## Legacy `DocPayload` files
//!
//! Pre-canonical desktop builds saved a private `DocPayload` JSON
//! (`{"version": 1, "active_page_index": …, "pages": […]}` — note the
//! integer `version`). There is no `DocPayload → PenDocument`
//! converter (that needs a full node-tree converter, out of scope for
//! this fix), so rather than fail silently with an opaque schema
//! error, [`load_editor_state`] detects the legacy shape and surfaces
//! an explicit "saved by an older version, must be re-saved" message.

use std::path::PathBuf;

use op_editor_core::EditorState;
use openpencil_shell_core::document::Document;
use openpencil_shell_native::WidgetHostNative;

/// Editor view-state persisted alongside the canonical `.op` file.
/// Kept intentionally minimal — `active_page_index` is the only piece
/// of view-state the user expects to survive a round-trip. Selection /
/// viewport / tool stay transient and are NOT persisted.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct EditorMeta {
    /// 0-based active page index at save time.
    #[serde(default)]
    active_page_index: usize,
}

/// Sidecar path for a given `.op` / `.pen` file — `<path>.opmeta`.
fn sidecar_path(path: &std::path::Path) -> PathBuf {
    let mut p = path.to_path_buf();
    let ext = match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{}.opmeta", ext),
        None => "opmeta".to_string(),
    };
    p.set_extension(ext);
    p
}

/// Serialize an `EditorState`'s canonical document to `path` without
/// prompting. Used by Cmd+S once the document already has a path.
/// Also writes the `.opmeta` view-state sidecar so `active_page_index`
/// survives the round-trip.
pub fn save_to_path(state: &EditorState, path: &std::path::Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&state.doc).map_err(|e| e.to_string())?;
    // Write through a sibling temp file so a crash mid-write doesn't
    // leave a half-written file on disk.
    let mut tmp = path.to_path_buf();
    tmp.set_extension(match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{}.tmp", ext),
        None => "tmp".to_string(),
    });
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    // View-state sidecar — best-effort. A failed sidecar write must
    // not fail the (already-committed) document save, so it only logs.
    let meta = EditorMeta {
        active_page_index: state.ui.active_page_index,
    };
    if let Ok(meta_json) = serde_json::to_string(&meta) {
        if let Err(e) = std::fs::write(sidecar_path(path), meta_json) {
            eprintln!("[save] view-state sidecar write failed: {e}");
        }
    }
    Ok(())
}

/// Pop a Save dialog (rfd native) and write the current document to
/// the chosen path. `Ok(Some(path))` on success, `Ok(None)` on user
/// cancel, `Err` on IO / encode failure.
pub fn save_as_dialog(state: &EditorState) -> Result<Option<PathBuf>, String> {
    let path = rfd::FileDialog::new()
        .set_title("Save document")
        .add_filter("OpenPencil", &["pen", "op"])
        .set_file_name("untitled.op")
        .save_file();
    let Some(path) = path else {
        return Ok(None);
    };
    save_to_path(state, &path)?;
    Ok(Some(path))
}

/// Detect the legacy private `DocPayload` JSON shape. A pre-canonical
/// desktop save has a top-level object with an *integer* `version`
/// (the canonical schema's `version` is always a string) and a
/// `pages` array. The canonical schema never emits an integer
/// `version`, so this check has no false positives against current
/// files.
fn looks_like_legacy_doc_payload(src: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(src) else {
        return false;
    };
    let Some(obj) = value.as_object() else {
        return false;
    };
    let version_is_integer = obj
        .get("version")
        .map(|v| v.is_u64() || v.is_i64())
        .unwrap_or(false);
    version_is_integer && obj.get("pages").map(|p| p.is_array()).unwrap_or(false)
}

/// Load a canonical `.pen` / `.op` file at `path` into a fresh
/// `EditorState`. Files from the TS editor, Jian apps, or anything
/// else emitting the canonical schema all load through the shared
/// parser. The `.opmeta` view-state sidecar (if present) restores
/// `active_page_index`.
///
/// A file in the legacy private `DocPayload` format is detected and
/// rejected with an explicit message: there is no
/// `DocPayload → PenDocument` converter (a full node-tree converter is
/// out of scope for this bounded fix), so the choice here is the
/// explicit-error variant — the alternative would be a silent,
/// confusing schema parse failure.
pub fn load_editor_state(path: &std::path::Path) -> Result<EditorState, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let src =
        std::str::from_utf8(&bytes).map_err(|e| format!("file is not valid UTF-8: {e}"))?;
    let loaded = match op_pen_loader::load_canonical(src) {
        Ok(loaded) => loaded,
        Err(e) => {
            // Distinguish "old format" from a genuinely corrupt file so
            // the user gets actionable guidance instead of a raw parse
            // error.
            if looks_like_legacy_doc_payload(src) {
                return Err(
                    "This file was saved by an older version of OpenPencil and \
                     uses a format this build can no longer read. Open it in the \
                     version that created it and re-save it in the current \
                     format."
                        .to_string(),
                );
            }
            return Err(e.to_string());
        }
    };
    for w in &loaded.warnings {
        eprintln!("[open] schema warning: {:?}", w);
    }
    let mut state = EditorState::from_document(loaded.value);
    // Restore editor view-state from the `.opmeta` sidecar — best
    // effort: a missing / unreadable / out-of-range sidecar leaves the
    // freshly loaded state on page 0.
    if let Ok(meta_src) = std::fs::read_to_string(sidecar_path(path)) {
        if let Ok(meta) = serde_json::from_str::<EditorMeta>(&meta_src) {
            // `pages == None` is the single-page fallback — one logical
            // page, so the only valid index is 0. Otherwise clamp the
            // saved index against the real page count.
            let page_count = state.doc.pages.as_ref().map(|p| p.len()).unwrap_or(1).max(1);
            state.ui.active_page_index = meta.active_page_index.min(page_count - 1);
        }
    }
    Ok(state)
}

/// Cmd+S — save to `current_path` if known, else fall through to
/// Save As. Updates `current_path` + window title on success.
pub fn handle_save(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    if let Some(path) = current_path.clone() {
        if let Err(e) = save_to_path(host.editor_state(), &path) {
            eprintln!("[save] {e}");
            show_error_dialog(host, ErrorKind::Save, Some(&path), &e);
        } else {
            crate::settings_io::touch_recent(host, &path);
            set_display_name(host, Some(&path));
        }
        return false;
    }
    handle_save_as(host, current_path, window)
}

fn set_display_name(host: &mut WidgetHostNative, path: Option<&std::path::Path>) {
    host.editor_state_mut().editor_ui.file_name_display = path
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
}

/// Cmd+Shift+S — always pop the Save dialog.
pub fn handle_save_as(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match save_as_dialog(host.editor_state()) {
        Ok(Some(path)) => {
            crate::settings_io::touch_recent(host, &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("[save as] {e}");
            show_error_dialog(host, ErrorKind::Save, None, &e);
            false
        }
    }
}

/// Replace the host's `EditorState` with one loaded from `path`.
fn load_into_host(host: &mut WidgetHostNative, path: &std::path::Path) -> Result<(), String> {
    let state = load_editor_state(path)?;
    let bb = active_page_bbox(&state);
    eprintln!(
        "[open] {} top-level nodes; content bbox {:?}",
        state.doc.children.len(),
        bb
    );
    *host.editor_state_mut() = state;
    host.mark_editor_state_dirty();
    Ok(())
}

fn active_page_bbox(state: &EditorState) -> Option<(f64, f64, f64, f64)> {
    use op_editor_core::geometry::own_bounds;
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for n in state.active_children() {
        let r = own_bounds(n);
        if r.w > 0.0 || r.h > 0.0 {
            min_x = min_x.min(r.x);
            min_y = min_y.min(r.y);
            max_x = max_x.max(r.x + r.w);
            max_y = max_y.max(r.y + r.h);
        }
    }
    if min_x.is_finite() {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

/// Cmd+O — pop the Open dialog and replace the current document.
pub fn handle_open(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    let path = match rfd::FileDialog::new()
        .set_title("Open document")
        .add_filter("OpenPencil", &["pen", "op"])
        .pick_file()
    {
        Some(p) => p,
        None => return false,
    };
    match load_into_host(host, &path) {
        Ok(()) => {
            crate::settings_io::touch_recent(host, &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Err(e) => {
            eprintln!("[open] {e}");
            show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
            false
        }
    }
}

/// Route a `FileAction` raised by the file-menu dispatcher to the
/// matching dialog flow.
pub fn run_action(
    action: op_editor_core::editor_ui_state::FileAction,
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) {
    use op_editor_core::editor_ui_state::FileAction;
    match action {
        FileAction::New => {
            // Reset the document to a fresh untitled state.
            *host.editor_state_mut() = EditorState::new();
            host.mark_editor_state_dirty();
            *current_path = None;
            refresh_title(current_path, window);
        }
        FileAction::Open => {
            handle_open(host, current_path, window);
        }
        FileAction::Save => {
            handle_save(host, current_path, window);
        }
        FileAction::SaveAs => {
            handle_save_as(host, current_path, window);
        }
        FileAction::ExportImage => {
            // main.rs intercepts ExportImage to open the picker; this
            // fallback keeps external callers working.
            host.editor_state_mut().editor_ui.export_dialog_open = true;
            host.mark_editor_state_dirty();
        }
        FileAction::ExportImageConfirm => {
            use op_editor_core::editor_ui_state::ExportFormat as Fmt;
            let fmt = host.editor_state().editor_ui.export_format;
            let scale = host.editor_state().editor_ui.export_scale;
            let (filter_label, filter_exts): (&str, &[&str]) = match fmt {
                Fmt::Png => ("PNG", &["png"]),
                Fmt::Jpeg => ("JPEG", &["jpg", "jpeg"]),
                Fmt::Webp => ("WEBP", &["webp"]),
                Fmt::Svg => ("SVG", &["svg"]),
                Fmt::Pdf => ("PDF", &["pdf"]),
            };
            let default_name = format!("openpencil-export.{}", fmt.extension());
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export image")
                .add_filter(filter_label, filter_exts)
                .set_file_name(&default_name)
                .save_file()
            {
                // The export renderers read a `&Document` — derive the
                // paint snapshot once and hand it over.
                let doc: &Document = host.document();
                let result: Result<(), String> = match fmt {
                    Fmt::Png => crate::export::export_raster(
                        doc,
                        &path,
                        crate::export::RasterFormat::Png,
                        scale,
                    ),
                    Fmt::Jpeg => crate::export::export_raster(
                        doc,
                        &path,
                        crate::export::RasterFormat::Jpeg,
                        scale,
                    ),
                    Fmt::Webp => crate::export::export_raster(
                        doc,
                        &path,
                        crate::export::RasterFormat::Webp,
                        scale,
                    ),
                    Fmt::Svg => crate::export::export_svg(doc, &path),
                    Fmt::Pdf => crate::export_pdf::export_pdf(doc, &path),
                };
                if let Err(e) = result {
                    eprintln!("[export-image] {e}");
                    show_error_dialog(host, ErrorKind::Export, Some(&path), &e);
                }
            }
        }
        FileAction::OpenRecent(i) => {
            let Some(entry) = host.editor_state().editor_ui.recent_files.get(i).cloned()
            else {
                return;
            };
            let path = std::path::PathBuf::from(&entry.path);
            match load_into_host(host, &path) {
                Ok(()) => {
                    crate::settings_io::touch_recent(host, &path);
                    *current_path = Some(path);
                    refresh_title(current_path, window);
                }
                Err(e) => {
                    // File missing / parse failure → tell the user and
                    // drop the stale entry from recents.
                    eprintln!("[open-recent] {e}; pruning {}", entry.path);
                    show_error_dialog(host, ErrorKind::Open, Some(&path), &e);
                    host.editor_state_mut()
                        .editor_ui
                        .recent_files
                        .retain(|r| r.path != entry.path);
                    host.mark_editor_state_dirty();
                }
            }
        }
        FileAction::ClearRecent => {
            host.editor_state_mut().editor_ui.recent_files.clear();
            host.mark_editor_state_dirty();
        }
        FileAction::ImportFigma => {
            eprintln!("[file-action] {action:?} — not yet wired (UI only)");
        }
    }
}

fn refresh_title(current_path: &Option<PathBuf>, window: Option<&winit::window::Window>) {
    let Some(window) = window else { return };
    let title = match current_path.as_ref().and_then(|p| p.file_name()) {
        Some(name) => format!("{} — OpenPencil", name.to_string_lossy()),
        None => "OpenPencil".to_string(),
    };
    window.set_title(&title);
}

/// Pop a native error dialog. Used by Open / Save / Export when the
/// underlying IO or parse step fails.
fn show_error_dialog(
    host: &WidgetHostNative,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    use op_editor_core::Locale;
    let zh = matches!(
        host.editor_state().editor_ui.locale,
        Locale::ZhCn | Locale::ZhTw
    );
    let (title, lead) = match (kind, zh) {
        (ErrorKind::Open, true) => ("无法打开文件", "OpenPencil 无法解析该文件。"),
        (ErrorKind::Open, false) => {
            ("Couldn't open file", "OpenPencil could not parse the file.")
        }
        (ErrorKind::Save, true) => ("保存失败", "写入文件时出错。"),
        (ErrorKind::Save, false) => {
            ("Save failed", "An error occurred while writing the file.")
        }
        (ErrorKind::Export, true) => ("导出失败", "渲染图像时出错。"),
        (ErrorKind::Export, false) => {
            ("Export failed", "An error occurred while rendering the image.")
        }
    };
    let mut body = lead.to_string();
    if let Some(p) = path {
        body.push_str("\n\n");
        body.push_str(&p.display().to_string());
    }
    body.push_str("\n\n");
    body.push_str(detail);
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(&body)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[derive(Debug, Clone, Copy)]
enum ErrorKind {
    Open,
    Save,
    Export,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique temp path under the OS temp dir for a round-trip test.
    fn temp_op_path(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!("openpencil-test-{tag}-{pid}-{nanos}.op"));
        p
    }

    #[test]
    fn active_page_index_survives_a_save_load_round_trip() {
        // Fix 5: editor view-state (`active_page_index`) is persisted in
        // the `.opmeta` sidecar so a save / load round-trip restores it
        // instead of reinitializing to page 0.
        let mut state = EditorState::new();
        // Two extra pages → three total; page index 2 is valid.
        state.add_page();
        state.add_page();
        state.ui.active_page_index = 2;

        let path = temp_op_path("page-roundtrip");
        save_to_path(&state, &path).expect("save succeeds");

        let reloaded = load_editor_state(&path).expect("load succeeds");
        assert_eq!(reloaded.ui.active_page_index, 2);

        // Cleanup.
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
    }

    #[test]
    fn active_page_index_is_clamped_against_the_real_page_count() {
        // A sidecar that names a page that no longer exists must not
        // leave the editor on an out-of-range index.
        let state = EditorState::new();
        let path = temp_op_path("page-clamp");
        save_to_path(&state, &path).expect("save succeeds");
        // Overwrite the sidecar with an absurd index.
        std::fs::write(sidecar_path(&path), r#"{"active_page_index":99}"#)
            .expect("sidecar overwrite");

        let reloaded = load_editor_state(&path).expect("load succeeds");
        // Single-page document → only index 0 is valid.
        assert_eq!(reloaded.ui.active_page_index, 0);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
    }

    #[test]
    fn legacy_doc_payload_is_detected() {
        // Fix 4: a pre-canonical private `DocPayload` JSON (integer
        // `version` + `pages` array) is recognised as the legacy
        // format.
        let legacy = r#"{"version":1,"active_page_index":0,"pages":[
            {"id":"n1","name":"Page 1","children":[]}
        ]}"#;
        assert!(looks_like_legacy_doc_payload(legacy));
    }

    #[test]
    fn canonical_document_is_not_mistaken_for_legacy() {
        // A canonical `.op` file has a *string* `version` — it must not
        // trip the legacy detector.
        let canonical = r#"{"version":"0.8.0","children":[]}"#;
        assert!(!looks_like_legacy_doc_payload(canonical));
    }

    #[test]
    fn loading_a_legacy_file_surfaces_an_explicit_error() {
        // The load path turns the legacy format into an actionable
        // user-facing message rather than an opaque schema error.
        let path = temp_op_path("legacy-load");
        std::fs::write(
            &path,
            r#"{"version":1,"active_page_index":0,"pages":[
                {"id":"n1","name":"Page 1","children":[]}
            ]}"#,
        )
        .expect("write legacy fixture");

        let err = load_editor_state(&path).expect_err("legacy file is rejected");
        assert!(
            err.contains("older version"),
            "error should mention the old version: {err}"
        );

        let _ = std::fs::remove_file(&path);
    }
}
