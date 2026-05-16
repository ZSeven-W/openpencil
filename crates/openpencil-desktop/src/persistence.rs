//! `.pen` / `.op` file Save / Open — desktop-side dialog flow.
//!
//! The payload DTOs + the canonical `PenDocument` → `Document`
//! conversion live in the shared `op-pen-loader` library crate so
//! library crates can reuse the loader. This file keeps only the
//! desktop-only UI: the `rfd` native Save / Open file dialogs, the
//! `FileAction` router, and the bilingual error dialog.

use std::path::PathBuf;

use op_pen_loader::{apply_payload, load_canonical, to_payload, DocPayload, NodePayload, PagePayload};
use openpencil_shell_core::document::Document;

/// Write the document to `path` without prompting. Used by Cmd+S
/// once the document already has a known path.
pub fn save_to_path(doc: &Document, path: &std::path::Path) -> Result<(), String> {
    let payload = to_payload(doc);
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    // Write through a sibling temp file so a crash mid-write
    // doesn't leave a half-written .pen on disk.
    let mut tmp = path.to_path_buf();
    tmp.set_extension(match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{}.tmp", ext),
        None => "tmp".to_string(),
    });
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Pop a Save dialog (rfd native) and write the current document
/// to the chosen path. Returns `Ok(Some(path))` on success,
/// `Ok(None)` on user cancel, `Err` on IO / encode failure.
pub fn save_as_dialog(doc: &Document) -> Result<Option<PathBuf>, String> {
    let path = rfd::FileDialog::new()
        .set_title("Save document")
        .add_filter("OpenPencil", &["pen", "op"])
        .set_file_name("untitled.pen")
        .save_file();
    let Some(path) = path else {
        return Ok(None);
    };
    save_to_path(doc, &path)?;
    Ok(Some(path))
}

/// Load a `.pen` / `.op` file at `path` into `doc`. Tries the
/// desktop's private `DocPayload` first (round-trip with its own
/// Save), then falls back to the canonical `.op` / `.pen` schema so
/// files from the TS editor, Jian apps, or anything else emitting
/// the format load through the shared parser.
pub fn load_from_path(doc: &mut Document, path: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // Side-channel: when the canonical path fires, also harvest
    // `variables` + `themes` into a VariableTable for assignment
    // after apply_payload (which resets Document state via Default).
    let mut canonical_var_table: Option<openpencil_shell_core::document::VariableTable> = None;
    let payload = match serde_json::from_slice::<DocPayload>(&bytes) {
        Ok(p) => p,
        Err(_) => {
            let src = std::str::from_utf8(&bytes)
                .map_err(|e| format!("file is not valid UTF-8: {e}"))?;
            let loaded = load_canonical(src).map_err(|e| e.to_string())?;
            for w in &loaded.warnings {
                eprintln!("[open] schema warning: {:?}", w);
            }
            canonical_var_table = Some(op_pen_loader::build_var_table(&loaded.value));
            let adapted = op_pen_loader::pen_document_to_payload(&loaded.value);
            adapted.payload
        }
    };
    let page_count = payload.pages.len();
    let node_count: usize = payload.pages.iter().map(|p| count_nodes(&p.children)).sum();
    apply_payload(doc, payload)?;
    if let Some(tbl) = canonical_var_table {
        doc.var_table = tbl;
    }
    let bb = active_page_bbox(doc);
    eprintln!(
        "[open] {} pages, {} nodes; content bbox {:?}; viewport pan=({:.1},{:.1}) zoom={:.2}",
        page_count, node_count, bb, doc.viewport.pan_x, doc.viewport.pan_y, doc.viewport.zoom
    );
    Ok(())
}

fn count_nodes(nodes: &[NodePayload]) -> usize {
    nodes
        .iter()
        .map(|n| 1 + count_nodes(&n.children))
        .sum()
}

fn active_page_bbox(doc: &Document) -> Option<(f32, f32, f32, f32)> {
    let page = doc.pages.get(doc.active_page_index)?;
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    fn visit(
        n: &openpencil_shell_core::document::Node,
        mn_x: &mut f32,
        mn_y: &mut f32,
        mx_x: &mut f32,
        mx_y: &mut f32,
    ) {
        let r = n.bounds;
        let x0 = r.origin.x.min(r.origin.x + r.size.x);
        let y0 = r.origin.y.min(r.origin.y + r.size.y);
        let x1 = x0 + r.size.x.abs();
        let y1 = y0 + r.size.y.abs();
        if r.size.x.abs() > 0.0 || r.size.y.abs() > 0.0 {
            *mn_x = mn_x.min(x0);
            *mn_y = mn_y.min(y0);
            *mx_x = mx_x.max(x1);
            *mx_y = mx_y.max(y1);
        }
        for c in &n.children {
            visit(c, mn_x, mn_y, mx_x, mx_y);
        }
    }
    for n in &page.children {
        visit(n, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
    }
    if !min_x.is_finite() {
        None
    } else {
        Some((min_x, min_y, max_x, max_y))
    }
}

/// Cmd+S — save to `current_path` if known, else fall through to
/// Save As. Updates `current_path` + window title on success.
pub fn handle_save(
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    if let Some(path) = current_path.clone() {
        if let Err(e) = save_to_path(host.document(), &path) {
            eprintln!("[save] {e}");
            show_error_dialog(host.document(), ErrorKind::Save, Some(&path), &e);
        } else {
            crate::settings_io::touch_recent(host.document_mut(), &path);
            set_display_name(host.document_mut(), Some(&path));
        }
        return false;
    }
    handle_save_as(host, current_path, window)
}

fn set_display_name(doc: &mut openpencil_shell_core::document::Document, path: Option<&std::path::Path>) {
    doc.ui.file_name_display = path
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
}

/// Cmd+Shift+S — always pop the Save dialog.
pub fn handle_save_as(
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    match save_as_dialog(host.document()) {
        Ok(Some(path)) => {
            crate::settings_io::touch_recent(host.document_mut(), &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("[save as] {e}");
            show_error_dialog(host.document(), ErrorKind::Save, None, &e);
            false
        }
    }
}

/// Cmd+O — pop the Open dialog and replace the current document.
pub fn handle_open(
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    // `open_dialog` borrows the document mutably to call
    // `load_from_path`; the user picks a path inside, so we can't
    // know it until after the call. Failed-parse dialogs include
    // the path the user just chose, which `open_dialog` doesn't
    // hand back — open the file dialog directly here so the path
    // is available for the error dialog.
    let path = match rfd::FileDialog::new()
        .set_title("Open document")
        .add_filter("OpenPencil", &["pen", "op"])
        .pick_file()
    {
        Some(p) => p,
        None => return false,
    };
    match load_from_path(host.document_mut(), &path) {
        Ok(()) => {
            crate::settings_io::touch_recent(host.document_mut(), &path);
            *current_path = Some(path);
            refresh_title(current_path, window);
            true
        }
        Err(e) => {
            eprintln!("[open] {e}");
            show_error_dialog(host.document(), ErrorKind::Open, Some(&path), &e);
            false
        }
    }
}

/// Route a `FileAction` raised by the file-menu dispatcher to the
/// matching dialog flow. Ignores `OpenRecent` / `ClearRecent` for
/// now — recents land in the next pass.
pub fn run_action(
    action: openpencil_shell_core::document::FileAction,
    host: &mut openpencil_shell_native::WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) {
    use openpencil_shell_core::document::FileAction;
    match action {
        FileAction::New => {
            // Reset the document tree to a fresh untitled page.
            let payload = DocPayload {
                version: 1,
                active_page_index: 0,
                pages: vec![PagePayload {
                    id: "n1".to_string(),
                    name: "Page 1".into(),
                    children: Vec::new(),
                }],
                // Fresh document — no design tokens.
                var_table: op_pen_loader::VarTablePayload::default(),
            };
            let _ = apply_payload(host.document_mut(), payload);
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
            // Should not normally hit run_action — main.rs intercepts
            // ExportImage to open the picker dialog instead. Keep as
            // a fallback so external callers still work.
            host.document_mut().ui.export_dialog_open = true;
        }
        FileAction::ExportImageConfirm => {
            use openpencil_shell_core::widgets::export_dialog::ExportFormat as Fmt;
            let fmt = host.document().ui.export_format;
            let scale = host.document().ui.export_scale;
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
                let result: Result<(), String> = match fmt {
                    Fmt::Png => crate::export::export_raster(
                        host.document(),
                        &path,
                        crate::export::RasterFormat::Png,
                        scale,
                    ),
                    Fmt::Jpeg => crate::export::export_raster(
                        host.document(),
                        &path,
                        crate::export::RasterFormat::Jpeg,
                        scale,
                    ),
                    Fmt::Webp => crate::export::export_raster(
                        host.document(),
                        &path,
                        crate::export::RasterFormat::Webp,
                        scale,
                    ),
                    Fmt::Svg => crate::export::export_svg(host.document(), &path),
                    Fmt::Pdf => crate::export_pdf::export_pdf(host.document(), &path),
                };
                if let Err(e) = result {
                    eprintln!("[export-image] {e}");
                    show_error_dialog(host.document(), ErrorKind::Export, Some(&path), &e);
                }
            }
        }
        FileAction::OpenRecent(i) => {
            let Some(entry) = host.document().ui.recent_files.get(i).cloned() else {
                return;
            };
            let path = std::path::PathBuf::from(&entry.path);
            match load_from_path(host.document_mut(), &path) {
                Ok(()) => {
                    crate::settings_io::touch_recent(host.document_mut(), &path);
                    *current_path = Some(path);
                    refresh_title(current_path, window);
                }
                Err(e) => {
                    // File missing / parse failure → tell the user and
                    // drop the entry from recents so a stale path
                    // doesn't keep haunting the menu.
                    eprintln!("[open-recent] {e}; pruning {}", entry.path);
                    show_error_dialog(host.document(), ErrorKind::Open, Some(&path), &e);
                    host.document_mut()
                        .ui
                        .recent_files
                        .retain(|r| r.path != entry.path);
                }
            }
        }
        FileAction::ClearRecent => {
            host.document_mut().ui.recent_files.clear();
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
/// underlying IO or parse step fails so the user gets feedback
/// instead of a silent no-op + stderr line they'll never see.
fn show_error_dialog(
    doc: &openpencil_shell_core::document::Document,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    use openpencil_shell_core::document::Locale;
    let zh = matches!(doc.ui.locale, Locale::ZhCn | Locale::ZhTw);
    let (title, lead) = match (kind, zh) {
        (ErrorKind::Open, true) => ("无法打开文件", "OpenPencil 无法解析该文件。"),
        (ErrorKind::Open, false) => ("Couldn't open file", "OpenPencil could not parse the file."),
        (ErrorKind::Save, true) => ("保存失败", "写入文件时出错。"),
        (ErrorKind::Save, false) => ("Save failed", "An error occurred while writing the file."),
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
