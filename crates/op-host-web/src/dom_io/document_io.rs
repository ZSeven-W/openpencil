use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::editor_ui_state::{ExportFormat, FileAction};

use crate::file_actions;
use crate::listener::now_unix_secs;
use crate::repaint_ctx::RepaintContext;

use super::import_generation::{begin_document_replacement, document_replacement_is_current};
use super::{
    console_error, console_warn, import_figma, import_html_file, import_image_or_svg,
    open_file_picker, pick_fill_image, read_file, relink_image, InnerRc, ReadMode,
};

type DaemonSaveLaunch = Box<dyn FnOnce()>;

thread_local! {
    /// The daemon has one bound file destination, so all mounted web editors
    /// share one serialized write lane. Pending launches contain only an
    /// identity and an `InnerRc`; the large JSON body is built only when that
    /// launch becomes active.
    static DAEMON_SAVE_QUEUE: RefCell<file_actions::LatestSaveQueue<DaemonSaveLaunch>> =
        RefCell::new(file_actions::LatestSaveQueue::default());
}

/// Consume a pending file action raised by a press dispatcher.
/// Mirrors the desktop's `persistence::run_action` routing; called
/// from the mousedown listener right after `codegen_web::drain_codegen_flags`,
/// once the press-time `inner` borrow is released. The flag is taken
/// FIRST so a failed handler can't re-fire on every later press.
pub(crate) fn drain_pending_file_action<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let action = inner
        .borrow_mut()
        .host_mut()
        .editor_state_mut()
        .editor_ui
        .pending_file_action
        .take();
    let Some(action) = action else {
        return;
    };
    match action {
        FileAction::Home => {
            let mut b = inner.borrow_mut();
            let ui = &mut b.host_mut().editor_state_mut().editor_ui;
            ui.entry_surface = op_editor_core::EntrySurface::Home;
            ui.home.visible = true;
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
        FileAction::New => new_document(inner),
        FileAction::Open => open_document(inner),
        FileAction::Save => save_document(inner, true),
        FileAction::SaveAs => save_document(inner, false),
        FileAction::ExportImage => {
            // Same fallback as the desktop run_action: open the
            // format/scale picker dialog; Export raises
            // `ExportImageConfirm` which lands below.
            let mut b = inner.borrow_mut();
            b.host_mut()
                .editor_state_mut()
                .editor_ui
                .open_export_dialog();
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
        FileAction::ExportImageConfirm => export_image(inner),
        FileAction::ExportDeckPdfSelection => {
            // Unlike the batch / slideshow / pptx rows below, this one IS
            // reachable in a browser: the slides rail paints its export
            // menu on every host, and the daemon already writes the
            // whole-deck PDF the sibling row asks for. Only the board
            // list differs.
            let b = inner.borrow();
            let state = b.host().editor_state();
            let boards = op_editor_core::preview_slideshow::selected_page_boards(state);
            request_pdf_download(state, Some(&boards));
        }
        FileAction::ExportAllFrames => {
            // Web leaves `batch_frame_export_supported` at `false`, so
            // the File menu never paints the row that raises this — a
            // browser has no directory to write a frame set into. Kept
            // as an explicit no-op branch so the shared action stays
            // exhaustive without inventing a web-only fan-out of
            // downloads.
        }
        FileAction::ExportSlideshowHtml => {
            // Same shape as the batch row above: web leaves
            // `deck_html_export_supported` at `false`, so the File menu
            // never paints the row that raises this. Kept as an explicit
            // no-op branch so the shared action stays exhaustive without
            // inventing a browser download path here.
        }
        FileAction::ExportPptx => {
            // Same shape as the two export rows above: the deck rows are
            // gated on `deck_html_export_supported`, which web leaves at
            // `false`, so this action never reaches a browser. Kept as an
            // explicit no-op branch so the shared action stays
            // exhaustive.
        }
        FileAction::ImportFigma => import_figma(inner),
        FileAction::FinishFigmaImport(_) => {
            // Desktop alone holds a PreparedFig between modal steps.
            // Keep the shared action exhaustive without introducing a
            // second web-side cache or changing web's all-page import.
            let mut b = inner.borrow_mut();
            let ui = &mut b.host_mut().editor_state_mut().editor_ui;
            ui.figma_import_pages.clear();
            ui.figma_import_page_select = Default::default();
            ui.figma_import_open = false;
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
        FileAction::ImportHtml => import_html_file(inner),
        FileAction::ImportImageOrSvg => import_image_or_svg(inner),
        FileAction::PickFillImage => pick_fill_image(inner),
        FileAction::RelinkImage => relink_image(inner),
        FileAction::OpenRecent(i) => open_recent_document(inner, i),
        FileAction::ClearRecent => {
            let mut b = inner.borrow_mut();
            b.host_mut()
                .editor_state_mut()
                .editor_ui
                .recent_files
                .clear();
            b.host_mut().mark_editor_state_dirty();
            let _ = b.repaint();
        }
    }
}

pub(super) fn open_recent_document<C: RepaintContext + 'static>(inner: &InnerRc<C>, index: usize) {
    let Some(path) = inner
        .borrow()
        .host()
        .editor_state()
        .editor_ui
        .recent_files
        .get(index)
        .map(|recent| recent.path.clone())
    else {
        return;
    };
    let body = serde_json::json!({ "path": path.clone() }).to_string();
    let generation = begin_document_replacement(inner);
    let base = crate::daemon_base::daemon_base();
    let inner_for_response = inner.clone();
    let path_for_response = path.clone();
    let on_response: Rc<dyn Fn(String)> = Rc::new(move |response: String| {
        if !document_replacement_is_current(&inner_for_response, generation) {
            return;
        }
        let mut b = inner_for_response.borrow_mut();
        if !file_actions::apply_open_recent_response(
            b.host_mut().editor_state_mut(),
            &path_for_response,
            &response,
            now_unix_secs(),
        ) {
            return;
        }
        b.host_mut().mark_editor_state_dirty();
        let _ = b.repaint();
    });
    if !crate::live_sync::post_json(
        &format!("{base}/api/file/open-recent"),
        &body,
        Some(on_response),
    ) {
        console_warn("open recent request could not start");
    }
}

/// File → New: fresh starter document, app preferences carried over,
/// viewport fit to the blank starter frame (desktop `FileAction::New`).
pub(super) fn new_document<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    begin_document_replacement(inner);
    let mut b = inner.borrow_mut();
    let mut state = op_editor_core::EditorState::starter();
    file_actions::preserve_app_preferences(b.host().editor_state(), &mut state);
    state.editor_ui.file_name_display = None;
    b.host_mut().replace_editor_state(state);
    let (w, h) = b.viewport_size();
    b.host_mut().fit_content_to_viewport(w, h);
    let _ = b.repaint();
}

/// File → Save / Save As. Pick the destination before serializing so the
/// common daemon path does not also build a pretty canonical download, and
/// Save As does not build a daemon body it will never send.
pub(super) fn save_document<C: RepaintContext + 'static>(inner: &InnerRc<C>, daemon_first: bool) {
    if daemon_first {
        save_to_daemon(inner);
    } else {
        save_to_browser(inner);
    }
}

fn save_to_daemon<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let (requested_epoch, requested_generation) = {
        let b = inner.borrow();
        let state = b.host().editor_state();
        (b.host().document_epoch(), state.document_generation())
    };
    let inner = inner.clone();
    enqueue_daemon_save(Box::new(move || {
        start_daemon_save(&inner, requested_epoch, requested_generation);
    }));
}

fn enqueue_daemon_save(launch: DaemonSaveLaunch) {
    let launch = DAEMON_SAVE_QUEUE.with(|queue| queue.borrow_mut().enqueue(launch));
    if let Some(launch) = launch {
        launch();
    }
}

fn finish_daemon_save() {
    let next = DAEMON_SAVE_QUEUE.with(|queue| queue.borrow_mut().finish());
    if let Some(next) = next {
        next();
    }
}

fn start_daemon_save<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    requested_epoch: u64,
    requested_generation: u64,
) {
    let (
        body,
        snap_epoch,
        snap_gen,
        snap_rev,
        snap_active_page_index,
        snap_preserve_authored_geometry,
    ) = {
        let b = inner.borrow();
        let state = b.host().editor_state();
        let snap_epoch = b.host().document_epoch();
        let snap_gen = state.document_generation();
        if !file_actions::save_ack_matches_document(
            snap_epoch,
            snap_gen,
            requested_epoch,
            requested_generation,
        ) {
            drop(b);
            finish_daemon_save();
            return;
        }
        let body =
            file_actions::serialize_save_payload(state, file_actions::SavePayloadTarget::Daemon);
        let snap_rev = state.document_revision();
        (
            body,
            snap_epoch,
            snap_gen,
            snap_rev,
            state.ui.active_page_index,
            state.editor_ui.preserve_authored_geometry,
        )
    };
    let body = match body {
        Ok(body) => body,
        Err(e) => {
            console_error(&format!("[save] {e}"));
            save_to_browser_if_snapshot_current(inner, snap_epoch, snap_gen, snap_rev);
            finish_daemon_save();
            return;
        }
    };

    let base = crate::daemon_base::daemon_base();
    let inner_for_response = inner.clone();
    let on_response: Rc<dyn Fn(String)> = Rc::new(move |response| {
        match file_actions::parse_save_response(&response) {
            Ok(saved) => {
                let mut b = inner_for_response.borrow_mut();
                if file_actions::save_ack_matches_document(
                    b.host().document_epoch(),
                    b.host().editor_state().document_generation(),
                    snap_epoch,
                    snap_gen,
                ) {
                    let acknowledged = b
                        .host_mut()
                        .editor_state_mut()
                        .mark_saved_revision_at(snap_gen, snap_rev);
                    if acknowledged {
                        if let Some(version) = saved.version {
                            crate::live_sync_glue::acknowledge_daemon_save(
                                version,
                                snap_gen,
                                snap_rev,
                                snap_active_page_index,
                                snap_preserve_authored_geometry,
                            );
                        }
                        b.host_mut().editor_state_mut().editor_ui.file_name_display =
                            Some(saved.file_name);
                        b.host_mut().mark_editor_state_dirty();
                        let _ = b.repaint();
                    }
                }
            }
            Err(e) => {
                console_warn(&format!("[save] daemon save unavailable: {e}"));
                // A delayed failure may belong to a document replaced by
                // Open/New. Build a fallback only while the exact snapshot
                // is still live; otherwise it would download and mark
                // saved an unrelated current document.
                save_to_browser_if_snapshot_current(
                    &inner_for_response,
                    snap_epoch,
                    snap_gen,
                    snap_rev,
                );
            }
        }
        finish_daemon_save();
    });
    let started =
        crate::live_sync::post_json(&format!("{base}/api/file/save"), &body, Some(on_response));
    if !started {
        // XHR did not capture the request. Release this whole-document string
        // before constructing the canonical browser fallback.
        drop(body);
        save_to_browser_if_snapshot_current(inner, snap_epoch, snap_gen, snap_rev);
        finish_daemon_save();
    }
}

fn save_to_browser_if_snapshot_current<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    snapshot_epoch: u64,
    snapshot_generation: u64,
    snapshot_revision: u64,
) {
    let current = {
        let b = inner.borrow();
        let state = b.host().editor_state();
        file_actions::save_snapshot_matches_document(
            b.host().document_epoch(),
            state.document_generation(),
            state.document_revision(),
            snapshot_epoch,
            snapshot_generation,
            snapshot_revision,
        )
    };
    if current {
        save_to_browser(inner);
    }
}

fn save_to_browser<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let (name, json) = {
        let b = inner.borrow();
        let state = b.host().editor_state();
        let name = file_actions::save_file_name(state);
        let json = file_actions::serialize_save_payload(
            state,
            file_actions::SavePayloadTarget::BrowserDownload,
        );
        (name, json)
    };
    match json {
        Ok(json) => download_saved_document(inner, &name, &json),
        Err(error) => console_error(&format!("[save] {error}")),
    }
}

fn download_saved_document<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    name: &str,
    json: &str,
) {
    if let Err(e) = crate::web_clipboard::download_bytes(name, "application/json", json.as_bytes())
    {
        web_sys::console::error_1(&e);
        return;
    }
    let mut b = inner.borrow_mut();
    file_actions::acknowledge_browser_download(b.host_mut().editor_state_mut(), name);
    b.host_mut().mark_editor_state_dirty();
    let _ = b.repaint();
}

/// Ask the local web-canvas daemon for a PDF and hand the bytes to the
/// browser as a download.
///
/// `boards` narrows a deck to those board ids — `None` is the whole
/// active page. Both PDF entry points (the export dialog and the slides
/// rail's "Export selected slides" row) come through here, so the route,
/// the response handling and the error prefix cannot drift apart.
pub(super) fn request_pdf_download(state: &op_editor_core::EditorState, boards: Option<&[String]>) {
    let body = match file_actions::export_pdf_request_body(state, boards) {
        Ok(body) => body,
        Err(e) => {
            console_error(&format!("[export-pdf] {e}"));
            return;
        }
    };
    // The export dialog's PDF is this document, so it is named after
    // it. A narrowed deck export is a different artifact — it keeps
    // the daemon's slide-deck name rather than claiming to be the
    // whole document.
    let download_name = boards
        .is_none()
        .then(|| file_actions::export_download_file_name(state));
    let base = crate::daemon_base::daemon_base();
    let on_response: Rc<dyn Fn(String)> =
        Rc::new(
            move |response| match file_actions::parse_pdf_download_response(&response) {
                Ok(pdf) => {
                    let name = download_name.as_deref().unwrap_or(&pdf.file_name);
                    if let Err(e) =
                        crate::web_clipboard::download_bytes(name, &pdf.mime, &pdf.bytes)
                    {
                        web_sys::console::error_1(&e);
                    }
                }
                Err(e) => console_error(&format!("[export-pdf] {e}")),
            },
        );
    if !crate::live_sync::post_json(&format!("{base}/api/export/pdf"), &body, Some(on_response)) {
        console_error("[export-pdf] request could not start");
    }
}

/// Export dialog → Export: SVG downloads vector markup from the
/// shared serializer; PDF asks the local web-canvas daemon to emit
/// the same Skia vector PDF as desktop; raster formats ask that same
/// daemon to run desktop's Skia offscreen exporter.
pub(super) fn export_image<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let b = inner.borrow();
    let fmt = b.host().editor_state().editor_ui.export_format;
    // `<document>-<node>.<ext>`, derived exactly as the desktop save
    // dialog derives its pre-filled name.
    let file_name = file_actions::export_download_file_name(b.host().editor_state());
    if fmt == ExportFormat::Svg {
        match file_actions::export_svg_document(b.host().editor_state()) {
            Ok(svg) => {
                if let Err(e) = crate::web_clipboard::download_bytes(
                    &file_name,
                    "image/svg+xml",
                    svg.as_bytes(),
                ) {
                    web_sys::console::error_1(&e);
                }
            }
            Err(e) => console_error(&format!("[export-svg] {e}")),
        }
        return;
    }
    if fmt == ExportFormat::Pdf {
        request_pdf_download(b.host().editor_state(), None);
        return;
    }
    let body = match file_actions::export_raster_request_body(b.host().editor_state()) {
        Ok(body) => body,
        Err(e) => {
            console_error(&format!("[export-raster] {e}"));
            return;
        }
    };
    let base = crate::daemon_base::daemon_base();
    let on_response: Rc<dyn Fn(String)> =
        Rc::new(
            move |response| match file_actions::parse_raster_download_response(&response) {
                Ok(raster) => {
                    if let Err(e) = crate::web_clipboard::download_bytes(
                        &file_name,
                        &raster.mime,
                        &raster.bytes,
                    ) {
                        web_sys::console::error_1(&e);
                    }
                }
                Err(e) => console_error(&format!("[export-raster] {e}")),
            },
        );
    if !crate::live_sync::post_json(
        &format!("{base}/api/export/raster"),
        &body,
        Some(on_response),
    ) {
        console_error("[export-raster] request could not start");
    }
}

/// File → Open: hidden `.op` / `.pen` picker → canonical ingest →
/// state swap → viewport fit.
pub(super) fn open_document<C: RepaintContext + 'static>(inner: &InnerRc<C>) {
    let inner = inner.clone();
    open_file_picker(
        ".op,.pen",
        Box::new(move |file| {
            read_open_document_file(&inner, file);
        }),
    );
}

/// Start a replacement-scoped read shared by the picker and file drop.
pub(super) fn read_open_document_file<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    file: web_sys::File,
) {
    let name = file.name();
    let generation = begin_document_replacement(inner);
    let inner = inner.clone();
    read_file(
        file,
        ReadMode::Text,
        Box::new(move |value| {
            if !document_replacement_is_current(&inner, generation) {
                return;
            }
            match value.as_string() {
                Some(src) => apply_opened_document(&inner, generation, &src, &name),
                None => console_error("[open] file read produced no text"),
            }
        }),
    );
}

/// Shared `.op` / `.pen` ingestion for the Open picker and drag-drop.
fn apply_opened_document<C: RepaintContext + 'static>(
    inner: &InnerRc<C>,
    generation: u64,
    src: &str,
    file_name: &str,
) {
    if !document_replacement_is_current(inner, generation) {
        return;
    }
    let mut b = inner.borrow_mut();
    match file_actions::ingest_op_source(src, b.host().editor_state()) {
        Ok(ingested) => {
            for w in &ingested.warnings {
                console_warn(&format!("[open] schema warning: {w}"));
            }
            let mut state = ingested.state;
            state.editor_ui.file_name_display = Some(file_name.to_string());
            // Use the shared ingestion path so an opened `.op` / `.pen` gets
            // the same cache invalidation and missing-font detection as Figma
            // and HTML document imports.
            b.host_mut().install_ingested_state(state);
            let (w, h) = b.viewport_size();
            b.host_mut().fit_content_to_viewport(w, h);
            let _ = b.repaint();
        }
        Err(e) => console_error(&format!("[open] {file_name}: {e}")),
    }
}
