// UNVERIFIED on wasm32: needs EMSDK build + browser; run tools/check-wasm-bundle.sh
//! Browser file-IO glue — the web consumer of
//! `editor_ui.pending_file_action` plus the DOM paste / drag-drop
//! listeners. The pure serialize / ingest logic lives in
//! `crate::file_actions`; this module owns only the `web_sys` side:
//! Blob downloads, hidden `<input type=file>` pickers, `FileReader`
//! reads, and the event routing.
//!
//! Closure lifetime pattern: long-lived listeners go through
//! `crate::listener::add_listener` (stored on the `WebShell` like
//! every mount() listener); the one-shot picker / reader callbacks
//! use the self-dropping `Rc<RefCell<Option<Closure>>>` slot idiom
//! from `raf_pump.rs` so each fires once and frees its own
//! wasm-bindgen closure slot. If the user cancels a file dialog the
//! `change` event never fires and that input + closure leak until
//! page unload — bounded (one hidden node per cancelled pick);
//! browsers expose no reliable cancel event across engines.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use op_editor_core::editor_ui_state::FileAction;

use crate::file_actions::{self, DropKind};
use crate::listener::{add_listener, Listener};

type InnerRc = Rc<RefCell<crate::Inner>>;

/// One-shot closure slot — the closure drops itself out of the slot
/// after firing (raf_pump idiom), freeing its wasm-bindgen slot.
type OnceSlot = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

fn console_error(msg: &str) {
    web_sys::console::error_1(&JsValue::from_str(msg));
}

fn console_warn(msg: &str) {
    web_sys::console::warn_1(&JsValue::from_str(msg));
}

// ---------------------------------------------------------------------
// pending_file_action drain (task item 1)
// ---------------------------------------------------------------------

/// Consume a pending file action raised by a press dispatcher.
/// Mirrors the desktop's `persistence::run_action` routing; called
/// from the mousedown listener right after `codegen_web::drain_codegen_flags`,
/// once the press-time `inner` borrow is released. The flag is taken
/// FIRST so a failed handler can't re-fire on every later press.
pub(crate) fn drain_pending_file_action(inner: &InnerRc) {
    let action = inner
        .borrow_mut()
        .host
        .editor_state_mut()
        .editor_ui
        .pending_file_action
        .take();
    let Some(action) = action else {
        return;
    };
    match action {
        FileAction::New => new_document(inner),
        FileAction::Open => open_document(inner),
        // No file paths on web — Save and Save As are both a download.
        FileAction::Save | FileAction::SaveAs => save_document(inner),
        FileAction::ExportImage => {
            // Same fallback as the desktop run_action: open the
            // format/scale picker dialog; Export raises
            // `ExportImageConfirm` which lands below.
            let mut b = inner.borrow_mut();
            b.host.editor_state_mut().editor_ui.export_dialog_open = true;
            b.host.mark_editor_state_dirty();
            let _ = b.repaint();
        }
        FileAction::ExportImageConfirm => export_image(inner),
        FileAction::ImportFigma => import_figma(inner),
        FileAction::ImportImageOrSvg => import_image_or_svg(inner),
        FileAction::PickFillImage => pick_fill_image(inner),
        FileAction::RelinkImage => relink_image(inner),
        // Recents have no storage on the web build (no file paths to
        // reopen) — consume as a no-op so the flag can't go stale.
        FileAction::OpenRecent(_) | FileAction::ClearRecent => {}
    }
}

/// File → New: fresh starter document, app preferences carried over,
/// viewport fit to the blank starter frame (desktop `FileAction::New`).
fn new_document(inner: &InnerRc) {
    let mut b = inner.borrow_mut();
    let mut state = op_editor_core::EditorState::starter();
    file_actions::preserve_app_preferences(b.host.editor_state(), &mut state);
    state.editor_ui.file_name_display = None;
    *b.host.editor_state_mut() = state;
    b.host.mark_editor_state_dirty();
    let (w, h) = (
        b.backend.canvas_width() as f32,
        b.backend.canvas_height() as f32,
    );
    b.host.fit_content_to_viewport(w, h);
    let _ = b.repaint();
}

/// File → Save / Save As: serialize the canonical document JSON and
/// hand it to the browser as a download (web's stand-in for a write
/// dialog — there is no re-writable file handle).
fn save_document(inner: &InnerRc) {
    let mut b = inner.borrow_mut();
    let json = match file_actions::serialize_document(b.host.editor_state()) {
        Ok(json) => json,
        Err(e) => {
            console_error(&format!("[save] {e}"));
            return;
        }
    };
    let name = file_actions::save_file_name(b.host.editor_state());
    if let Err(e) = crate::web_clipboard::download_bytes(&name, "application/json", json.as_bytes())
    {
        web_sys::console::error_1(&e);
        return;
    }
    // First save of an untitled document names it — matches the
    // desktop updating the display name after Save As.
    if b.host.editor_state().editor_ui.file_name_display.is_none() {
        b.host.editor_state_mut().editor_ui.file_name_display = Some(name);
        b.host.mark_editor_state_dirty();
        let _ = b.repaint();
    }
}

/// Export dialog → Export: encode the live canvas raster via
/// `HtmlCanvasElement::to_data_url` and download it. See
/// `WebBackend::canvas_data_url` for the documented trade-off (the
/// raster is the full editor frame at 1× — no per-node crop /
/// `export_scale`, and SVG / PDF degrade to PNG).
fn export_image(inner: &InnerRc) {
    let b = inner.borrow();
    let fmt = b.host.editor_state().editor_ui.export_format;
    let (mime, ext, supported) = file_actions::export_target(fmt);
    if !supported {
        console_warn(
            "[export-image] SVG / PDF export is not supported in the browser build yet; \
             downloading a PNG raster of the canvas instead",
        );
    }
    match b.backend.canvas_data_url(mime) {
        Ok(url) => {
            let name = format!("openpencil-export.{ext}");
            if let Err(e) = crate::web_clipboard::download_data_url(&name, &url) {
                web_sys::console::error_1(&e);
            }
        }
        Err(e) => web_sys::console::error_1(&e),
    }
}

/// File → Open: hidden `.op` / `.pen` picker → canonical ingest →
/// state swap → viewport fit.
fn open_document(inner: &InnerRc) {
    let inner = inner.clone();
    open_file_picker(
        ".op,.pen",
        Box::new(move |file| {
            let name = file.name();
            let inner2 = inner.clone();
            read_file(
                file,
                ReadMode::Text,
                Box::new(move |value| match value.as_string() {
                    Some(src) => apply_opened_document(&inner2, &src, &name),
                    None => console_error("[open] file read produced no text"),
                }),
            );
        }),
    );
}

/// Shared `.op` / `.pen` ingestion for the Open picker and drag-drop.
fn apply_opened_document(inner: &InnerRc, src: &str, file_name: &str) {
    let mut b = inner.borrow_mut();
    match file_actions::ingest_op_source(src, b.host.editor_state()) {
        Ok(ingested) => {
            for w in &ingested.warnings {
                console_warn(&format!("[open] schema warning: {w}"));
            }
            let mut state = ingested.state;
            state.editor_ui.file_name_display = Some(file_name.to_string());
            *b.host.editor_state_mut() = state;
            b.host.mark_editor_state_dirty();
            let (w, h) = (
                b.backend.canvas_width() as f32,
                b.backend.canvas_height() as f32,
            );
            b.host.fit_content_to_viewport(w, h);
            let _ = b.repaint();
        }
        Err(e) => console_error(&format!("[open] {file_name}: {e}")),
    }
}

/// Figma modal drop-zone → hidden `.fig` picker → binary parse →
/// state install (the parse runs on the main thread after the async
/// read; the in-progress overlay covers the stall).
fn import_figma(inner: &InnerRc) {
    let inner = inner.clone();
    open_file_picker(
        ".fig",
        Box::new(move |file| {
            ingest_figma_file(&inner, file);
        }),
    );
}

/// Shared `.fig` ingestion for the import picker and drag-drop.
fn ingest_figma_file(inner: &InnerRc, file: web_sys::File) {
    let name = file.name();
    {
        // Raise the parsing overlay before the async read so the next
        // frame shows feedback (mirrors `figma_import_session::spawn`).
        let mut b = inner.borrow_mut();
        b.host.editor_state_mut().editor_ui.figma_import_in_progress = true;
        b.host.mark_editor_state_dirty();
        let _ = b.repaint();
    }
    let inner2 = inner.clone();
    read_file(
        file,
        ReadMode::Bytes,
        Box::new(move |value| {
            let stem = file_actions::file_stem(&name).to_string();
            let Some(bytes) = js_bytes(&value) else {
                clear_figma_in_progress(&inner2);
                console_error(&format!(
                    "[import-figma] {name}: file read produced no bytes"
                ));
                return;
            };
            // Parse outside any `inner` borrow — it's the heavy step.
            match file_actions::ingest_figma_bytes(&bytes, &stem) {
                Ok(ingested) => {
                    for w in &ingested.warnings {
                        console_warn(&format!("[import-figma] warning: {w}"));
                    }
                    let mut b = inner2.borrow_mut();
                    b.host.install_ingested_state(ingested.state);
                    let (w, h) = (
                        b.backend.canvas_width() as f32,
                        b.backend.canvas_height() as f32,
                    );
                    b.host.fit_content_to_viewport(w, h);
                    let _ = b.repaint();
                }
                Err(e) => {
                    clear_figma_in_progress(&inner2);
                    console_error(&format!("[import-figma] {name}: {e}"));
                }
            }
        }),
    );
}

fn clear_figma_in_progress(inner: &InnerRc) {
    let mut b = inner.borrow_mut();
    b.host.editor_state_mut().editor_ui.figma_import_in_progress = false;
    b.host.mark_editor_state_dirty();
    let _ = b.repaint();
}

/// Shape picker → Import image or SVG: SVG parses into editable
/// nodes; rasters insert an Image node carrying the file as a data
/// URL (the browser's `readAsDataURL` builds it — no base64 dep).
fn import_image_or_svg(inner: &InnerRc) {
    let inner = inner.clone();
    open_file_picker(
        ".png,.jpg,.jpeg,.gif,.webp,.svg",
        Box::new(move |file| {
            let name = file.name();
            if file_actions::drop_kind(&name) == DropKind::Svg {
                let inner2 = inner.clone();
                read_file(
                    file,
                    ReadMode::Text,
                    Box::new(move |value| match value.as_string() {
                        Some(svg) => insert_svg(&inner2, &svg, &name),
                        None => console_error("[import-svg] file read produced no text"),
                    }),
                );
            } else {
                let inner2 = inner.clone();
                read_file(
                    file,
                    ReadMode::DataUrl,
                    Box::new(move |value| match value.as_string() {
                        Some(url) => insert_image(&inner2, &url, &name),
                        None => console_error("[import-image] file read produced no data URL"),
                    }),
                );
            }
        }),
    );
}

/// Fill section 图片 row → picker → write the data URL into the
/// selected node's primary fill (desktop `handle_pick_fill_image`).
fn pick_fill_image(inner: &InnerRc) {
    let inner = inner.clone();
    open_file_picker(
        ".png,.jpg,.jpeg,.gif,.webp,.svg",
        Box::new(move |file| {
            let inner2 = inner.clone();
            read_file(
                file,
                ReadMode::DataUrl,
                Box::new(move |value| {
                    let Some(url) = value.as_string() else {
                        console_error("[fill-image] file read produced no data URL");
                        return;
                    };
                    let mut b = inner2.borrow_mut();
                    let _ = b.host.editor_state_mut().set_selected_fill_image_url(&url);
                    b.host.mark_editor_state_dirty();
                    let _ = b.repaint();
                }),
            );
        }),
    );
}

/// Image-section warning row's Relink: pick a replacement image and
/// rewrite the selected image node's `src`. The desktop
/// (`persistence_image::handle_relink_image`) stores a document-relative
/// FILE PATH; the browser has no file paths, so the picked file becomes a
/// data URL — the same divergence-by-platform as `PickFillImage`. The
/// stale-asset check is dropped so the warning row clears on re-probe.
fn relink_image(inner: &InnerRc) {
    let inner = inner.clone();
    open_file_picker(
        ".png,.jpg,.jpeg,.gif,.webp,.svg",
        Box::new(move |file| {
            let inner2 = inner.clone();
            read_file(
                file,
                ReadMode::DataUrl,
                Box::new(move |value| {
                    let Some(url) = value.as_string() else {
                        console_error("[relink-image] file read produced no data URL");
                        return;
                    };
                    let mut b = inner2.borrow_mut();
                    let state = b.host.editor_state_mut();
                    let id = state.selection.anchor.clone();
                    if !id.is_real() {
                        return;
                    }
                    state.commit_history();
                    if let Some(jian_ops_schema::node::PenNode::Image(image)) =
                        op_editor_core::walkers::find_node_mut(state.active_children_mut(), &id)
                    {
                        image.src = url;
                    }
                    state.editor_ui.image_panel.asset_check = None;
                    b.host.mark_editor_state_dirty();
                    let _ = b.repaint();
                }),
            );
        }),
    );
}

/// Parse SVG source into editable nodes centred near the viewport —
/// mirrors `persistence_image::handle_import_image_or_svg`'s SVG arm.
fn insert_svg(inner: &InnerRc, svg: &str, file_name: &str) {
    let mut b = inner.borrow_mut();
    let pan_x = b.host.editor_state().viewport.pan_x as f64;
    let pan_y = b.host.editor_state().viewport.pan_y as f64;
    let zoom = (b.host.editor_state().viewport.zoom as f64).max(0.001);
    let centre_x = -pan_x / zoom;
    let centre_y = -pan_y / zoom;
    let mut next_id = 0u64;
    let stem = file_actions::file_stem(file_name);
    let count = b.host.editor_state_mut().import_svg_named(
        &mut next_id,
        svg,
        (centre_x - 200.0, centre_y - 150.0),
        Some(stem),
    );
    if count == 0 {
        console_warn(&format!("[import-svg] {file_name} yielded no nodes"));
    }
    b.host.mark_editor_state_dirty();
    let _ = b.repaint();
}

/// Insert a raster image (as a data URL) as an Image node centred on
/// the viewport — mirrors the desktop's raster import arm.
fn insert_image(inner: &InnerRc, url: &str, file_name: &str) {
    let mut b = inner.borrow_mut();
    let stem = file_actions::file_stem(file_name).to_string();
    let _ = b
        .host
        .editor_state_mut()
        .insert_image_node_at_viewport(&stem, url);
    b.host.mark_editor_state_dirty();
    let _ = b.repaint();
}

// ---------------------------------------------------------------------
// DOM paste + file drag-drop listeners (task items 3 + 4)
// ---------------------------------------------------------------------

/// Register the paste / dragover / dragleave / drop listeners.
/// Called from `mount()`'s registration closure; the closures are
/// stored on the `WebShell` like every other listener.
pub(crate) fn register_io_listeners(
    inner: &InnerRc,
    canvas: &web_sys::HtmlCanvasElement,
    win_target: &web_sys::EventTarget,
    listeners: &mut Vec<Listener>,
) -> Result<(), JsValue> {
    // Paste on the window — the event fires on whatever element owns
    // focus (usually the hidden IME textarea) and bubbles up.
    {
        let inner_p = inner.clone();
        add_listener::<web_sys::ClipboardEvent, _, _>(
            win_target,
            "paste",
            listeners,
            move |evt: web_sys::ClipboardEvent| {
                handle_paste_event(&inner_p, &evt);
            },
        )?;
    }
    let canvas_target: web_sys::EventTarget = canvas.clone().into();
    // dragover fires continuously while a file hovers the canvas;
    // preventDefault marks the canvas a valid drop target (without it
    // the browser never fires `drop`).
    {
        let inner_d = inner.clone();
        add_listener::<web_sys::DragEvent, _, _>(
            &canvas_target,
            "dragover",
            listeners,
            move |evt: web_sys::DragEvent| {
                evt.prevent_default();
                set_file_drop_active(&inner_d, true);
            },
        )?;
    }
    {
        let inner_d = inner.clone();
        add_listener::<web_sys::DragEvent, _, _>(
            &canvas_target,
            "dragleave",
            listeners,
            move |_evt: web_sys::DragEvent| {
                set_file_drop_active(&inner_d, false);
            },
        )?;
    }
    {
        let inner_d = inner.clone();
        add_listener::<web_sys::DragEvent, _, _>(
            &canvas_target,
            "drop",
            listeners,
            move |evt: web_sys::DragEvent| {
                evt.prevent_default();
                set_file_drop_active(&inner_d, false);
                handle_drop_event(&inner_d, &evt);
            },
        )?;
    }
    Ok(())
}

/// Flip the painted file-drop overlay (`paint.rs` already renders it
/// when `file_drop_active` is set); repaints only on change.
fn set_file_drop_active(inner: &InnerRc, active: bool) {
    let mut b = inner.borrow_mut();
    if b.host.editor_state().editor_ui.file_drop_active != active {
        b.host.editor_state_mut().editor_ui.file_drop_active = active;
        b.host.mark_editor_state_dirty();
        let _ = b.repaint();
    }
}

/// DOM paste routing — native Cmd+V priority order
/// (`keyboard_input.rs`): Figma clipboard HTML first, then the
/// focused text input, then the internal node clipboard.
fn handle_paste_event(inner: &InnerRc, evt: &web_sys::ClipboardEvent) {
    let Some(dt) = evt.clipboard_data() else {
        return;
    };
    let html = dt.get_data("text/html").unwrap_or_default();
    if !html.is_empty() && op_figma::is_figma_clipboard_html(&html) {
        evt.prevent_default();
        if let Some(data) = op_figma::extract_figma_clipboard_data(&html) {
            let result = op_figma::figma_clipboard_to_nodes(&data.buffer, Some(&html));
            if !result.nodes.is_empty() {
                let mut b = inner.borrow_mut();
                let (w, h) = (
                    b.backend.canvas_width() as f32,
                    b.backend.canvas_height() as f32,
                );
                if b.host.paste_figma_nodes(result.nodes, w, h) {
                    let _ = b.repaint();
                }
                return;
            }
        }
        // Recognized Figma HTML but nothing usable decoded — swallow
        // the paste rather than dumping raw HTML text (native parity).
        return;
    }
    let text = dt.get_data("text/plain").unwrap_or_default();
    if !text.is_empty() {
        let mut b = inner.borrow_mut();
        if b.host.apply_paste_text(&text) {
            evt.prevent_default();
            let _ = b.repaint();
            return;
        }
    }
    // Nothing textual consumed it — fall back to the internal node
    // clipboard (Cmd+C/V of canvas nodes; lowest priority).
    let mut b = inner.borrow_mut();
    if b.host.apply_paste() {
        let _ = b.repaint();
    }
}

/// Route every dropped file through the same ingestion the file menu
/// / shape picker use.
fn handle_drop_event(inner: &InnerRc, evt: &web_sys::DragEvent) {
    let Some(dt) = evt.data_transfer() else {
        return;
    };
    let Some(files) = dt.files() else {
        return;
    };
    for i in 0..files.length() {
        let Some(file) = files.get(i) else {
            continue;
        };
        route_dropped_file(inner, file);
    }
}

fn route_dropped_file(inner: &InnerRc, file: web_sys::File) {
    let name = file.name();
    match file_actions::drop_kind(&name) {
        DropKind::Document => {
            let inner2 = inner.clone();
            read_file(
                file,
                ReadMode::Text,
                Box::new(move |value| match value.as_string() {
                    Some(src) => apply_opened_document(&inner2, &src, &name),
                    None => console_error("[open] dropped file read produced no text"),
                }),
            );
        }
        DropKind::Figma => ingest_figma_file(inner, file),
        DropKind::Svg => {
            let inner2 = inner.clone();
            read_file(
                file,
                ReadMode::Text,
                Box::new(move |value| match value.as_string() {
                    Some(svg) => insert_svg(&inner2, &svg, &name),
                    None => console_error("[import-svg] dropped file read produced no text"),
                }),
            );
        }
        DropKind::Image => {
            let inner2 = inner.clone();
            read_file(
                file,
                ReadMode::DataUrl,
                Box::new(move |value| match value.as_string() {
                    Some(url) => insert_image(&inner2, &url, &name),
                    None => console_error("[import-image] dropped file read produced no data URL"),
                }),
            );
        }
        DropKind::Unsupported => {
            console_warn(&format!("[drop] unsupported file type: {name}"));
        }
    }
}

// ---------------------------------------------------------------------
// Hidden file input + FileReader plumbing
// ---------------------------------------------------------------------

/// Pop a hidden `<input type=file accept=…>` and invoke `on_file`
/// with the chosen file. One-shot; see the module docs for the
/// cancel-leak trade-off.
fn open_file_picker(accept: &str, on_file: Box<dyn FnOnce(web_sys::File)>) {
    let result = (|| -> Result<(), JsValue> {
        let window = web_sys::window()
            .ok_or_else(|| JsValue::from_str("file picker: window unavailable"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("file picker: document unavailable"))?;
        let input = document
            .create_element("input")?
            .dyn_into::<web_sys::HtmlInputElement>()
            .map_err(|_| JsValue::from_str("file picker: <input> cast failed"))?;
        input.set_type("file");
        input.set_accept(accept);
        input.set_attribute("style", "display:none")?;
        input.set_attribute("aria-hidden", "true")?;
        let body = document
            .body()
            .ok_or_else(|| JsValue::from_str("file picker: document.body unavailable"))?;
        body.append_child(&input)?;

        let slot: OnceSlot = Rc::new(RefCell::new(None));
        let slot2 = slot.clone();
        let input_cb = input.clone();
        let mut once = Some(on_file);
        *slot.borrow_mut() = Some(Closure::new(move || {
            let file = input_cb.files().and_then(|files| files.get(0));
            input_cb.remove();
            if let (Some(cb), Some(file)) = (once.take(), file) {
                cb(file);
            }
            // Self-drop (raf_pump idiom) — the change event has been
            // handed back to the JS side for this firing.
            let _ = slot2.borrow_mut().take();
        }));
        {
            let slot_ref = slot.borrow();
            let closure = slot_ref.as_ref().expect("closure just installed");
            input.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())?;
        }
        input.click();
        Ok(())
    })();
    if let Err(e) = result {
        web_sys::console::error_1(&e);
    }
}

/// How to read a `File` — maps onto the three `FileReader` modes the
/// ingestion paths need.
enum ReadMode {
    /// `readAsText` → `.op` / `.pen` / `.svg` sources.
    Text,
    /// `readAsArrayBuffer` → binary `.fig` payloads.
    Bytes,
    /// `readAsDataURL` → raster images (the browser builds the
    /// `data:` URL, so no base64 dependency is needed).
    DataUrl,
}

/// Read `file` asynchronously and invoke `on_done` with the raw
/// `FileReader.result` JsValue (string for Text / DataUrl, an
/// ArrayBuffer for Bytes; `JsValue::NULL` on a failed read).
fn read_file(file: web_sys::File, mode: ReadMode, on_done: Box<dyn FnOnce(JsValue)>) {
    let result = (|| -> Result<(), JsValue> {
        let reader = web_sys::FileReader::new()?;
        let slot: OnceSlot = Rc::new(RefCell::new(None));
        let slot2 = slot.clone();
        let reader_cb = reader.clone();
        let mut once = Some(on_done);
        *slot.borrow_mut() = Some(Closure::new(move || {
            let value = reader_cb.result().unwrap_or(JsValue::NULL);
            if let Some(cb) = once.take() {
                cb(value);
            }
            let _ = slot2.borrow_mut().take();
        }));
        {
            let slot_ref = slot.borrow();
            let closure = slot_ref.as_ref().expect("closure just installed");
            // `loadend` fires for success, error, and abort alike, so
            // the one-shot callback always resolves and frees itself.
            reader.set_onloadend(Some(closure.as_ref().unchecked_ref()));
        }
        match mode {
            ReadMode::Text => reader.read_as_text(&file)?,
            ReadMode::Bytes => reader.read_as_array_buffer(&file)?,
            ReadMode::DataUrl => reader.read_as_data_url(&file)?,
        }
        Ok(())
    })();
    if let Err(e) = result {
        web_sys::console::error_1(&e);
    }
}

/// Extract a byte vec from a `FileReader.result` ArrayBuffer.
fn js_bytes(value: &JsValue) -> Option<Vec<u8>> {
    let buf = value.clone().dyn_into::<js_sys::ArrayBuffer>().ok()?;
    Some(js_sys::Uint8Array::new(&buf).to_vec())
}
