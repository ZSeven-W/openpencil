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
use op_host_native::WidgetHostNative;

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
        .set_title(op_i18n::translate(
            state.editor_ui.locale,
            "dialog.pickerSaveTitle",
        ))
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
pub fn load_editor_state(
    path: &std::path::Path,
    locale: op_editor_core::Locale,
) -> Result<EditorState, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let src = match std::str::from_utf8(&bytes) {
        Ok(src) => src,
        Err(e) => {
            return Err(op_i18n::translate(locale, "dialog.loadErrorInvalidUtf8")
                .replace("{{detail}}", &e.to_string()));
        }
    };
    let loaded = match op_pen_loader::load_canonical(src) {
        Ok(loaded) => loaded,
        Err(e) => {
            // Distinguish "old format" from a genuinely corrupt file so
            // the user gets actionable guidance instead of a raw parse
            // error.
            if looks_like_legacy_doc_payload(src) {
                return Err(op_i18n::translate(locale, "dialog.loadErrorOldVersion").to_string());
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
            let page_count = state
                .doc
                .pages
                .as_ref()
                .map(|p| p.len())
                .unwrap_or(1)
                .max(1);
            state.ui.active_page_index = meta.active_page_index.min(page_count - 1);
        }
    }
    Ok(state)
}

/// Cmd+S — save to `current_path` if known, else fall through to
/// Save As. Updates `current_path` + window title on success.
/// Returns `true` when the document was written to disk, `false` on
/// an IO error or a cancelled Save-As dialog — so the caller can
/// tell a real save from a no-op (e.g. the unsaved-changes prompt).
pub fn handle_save(
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
    if let Some(path) = current_path.clone() {
        match save_to_path(host.editor_state(), &path) {
            Err(e) => {
                eprintln!("[save] {e}");
                show_error_dialog(host, ErrorKind::Save, Some(&path), &e);
                return false;
            }
            Ok(()) => {
                crate::settings_io::touch_recent(host, &path);
                set_display_name(host, Some(&path));
                return true;
            }
        }
    }
    handle_save_as(host, current_path, window)
}

/// A cheap content fingerprint of the document — the hash of its
/// canonical JSON serialization. The desktop runner compares the
/// live fingerprint against the one captured at the last save /
/// open / new to decide whether there are unsaved changes worth a
/// close-time prompt. A serialization failure (not expected for a
/// valid document) yields a sentinel that simply reads as "changed".
pub fn document_fingerprint(state: &EditorState) -> u64 {
    use std::hash::{Hash, Hasher};
    match serde_json::to_vec(&state.doc) {
        Ok(bytes) => {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            hasher.finish()
        }
        Err(_) => 0,
    }
}

fn set_display_name(host: &mut WidgetHostNative, path: Option<&std::path::Path>) {
    set_file_name_display(host.editor_state_mut(), path);
}

fn set_file_name_display(state: &mut EditorState, path: Option<&std::path::Path>) {
    state.editor_ui.file_name_display = path
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
    let locale = host.editor_state().editor_ui.locale;
    let mut state = load_editor_state(path, locale)?;
    preserve_app_preferences(host.editor_state(), &mut state);
    set_file_name_display(&mut state, Some(path));
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

fn preserve_app_preferences(previous: &EditorState, next: &mut EditorState) {
    let previous_selected_model = previous.chat.selected_model_entry().cloned();
    next.editor_ui.theme_mode = previous.editor_ui.theme_mode;
    next.editor_ui.locale = previous.editor_ui.locale;
    next.editor_ui.recent_files = previous.editor_ui.recent_files.clone();
    // Imported UIKits are app-level (persisted in `uikits.json`), not
    // document state — `from_document` reset them to the built-ins.
    next.ui_kits = previous.ui_kits.clone();
    // #20: theme presets are app-level too (`theme-presets.json`).
    next.theme_presets = previous.theme_presets.clone();
    next.theme_presets_dirty = previous.theme_presets_dirty;
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
        .set_title(op_i18n::translate(
            host.editor_state().editor_ui.locale,
            "dialog.pickerOpenTitle",
        ))
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

/// Whether `path` carries a document extension this build opens
/// (`.op` / `.pen`). Used to filter drag-and-drop drops + the
/// file-association argv before attempting a load. The extension
/// match is case-insensitive so `MyDesign.OP` from a case-folding
/// filesystem / argv still opens.
pub fn is_supported_document(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("op") || ext.eq_ignore_ascii_case("pen"))
}

/// True for Figma `.fig` binary exports. The bundle declares `.fig`
/// as a `CFBundleDocumentTypes` extension (macOS / Windows / Linux),
/// so double-clicking one in Finder / dragging one onto the dock /
/// the running window all need to route through
/// `figma_import_session::spawn` rather than the `.op`-only
/// `open_path`. Case-insensitive (Figma's "Save Local Copy" emits
/// `.fig`; some macOS shares fold to `.FIG`).
pub fn is_supported_figma_import(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("fig"))
}

/// Open `path` directly — no dialog. Backs drag-and-drop drops and
/// the file-association launch path. Replaces the host's document,
/// records the file in recents and refreshes the window title.
/// Returns `true` when the document loaded (so the caller can
/// request a redraw); a load failure pops the error dialog and
/// leaves the current document untouched.
pub fn open_path(
    host: &mut WidgetHostNative,
    path: PathBuf,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> bool {
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

/// Outcome of [`run_action`] — tells the desktop runner which
/// post-action bookkeeping to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOutcome {
    /// The document now matches a file on disk (New / successful
    /// Open / Save / Save-As / Open-Recent). The runner refreshes the
    /// unsaved-changes baseline AND rebinds the Git session.
    Saved,
    /// User picked a `.fig` and the desktop runner should spawn the
    /// background parser (`figma_import_session::spawn`). The actual
    /// document swap happens later when `figma_import_session::pump`
    /// drains the worker's result + rebinds the Git session itself
    /// (the previously-open repo binding goes stale on import).
    FigmaImportStarted(PathBuf),
    /// Nothing to reconcile — export, recent-list edits, or a user
    /// cancel / error.
    Noop,
}

impl ActionOutcome {
    /// Map a save/open helper's `bool` (`true` = the document now
    /// matches a file on disk) onto an outcome.
    fn saved_or_noop(saved: bool) -> Self {
        if saved {
            ActionOutcome::Saved
        } else {
            ActionOutcome::Noop
        }
    }
}

/// Route a `FileAction` raised by the file-menu dispatcher to the
/// matching dialog flow. The returned [`ActionOutcome`] tells the
/// runner which post-action bookkeeping to run — see its variant
/// docs.
pub fn run_action(
    action: op_editor_core::editor_ui_state::FileAction,
    host: &mut WidgetHostNative,
    current_path: &mut Option<PathBuf>,
    window: Option<&winit::window::Window>,
) -> ActionOutcome {
    use op_editor_core::editor_ui_state::FileAction;
    match action {
        FileAction::New => {
            let mut state = EditorState::starter();
            preserve_app_preferences(host.editor_state(), &mut state);
            *host.editor_state_mut() = state;
            let (vw, vh) = window
                .map(|w| {
                    let size = w.inner_size();
                    let scale = w.scale_factor() as f32;
                    (size.width as f32 / scale, size.height as f32 / scale)
                })
                .unwrap_or((super::INITIAL_VIEWPORT_W, super::INITIAL_VIEWPORT_H));
            host.fit_content_to_viewport(vw, vh);
            host.mark_editor_state_dirty();
            *current_path = None;
            refresh_title(current_path, window);
            ActionOutcome::Saved
        }
        FileAction::Open => ActionOutcome::saved_or_noop(handle_open(host, current_path, window)),
        FileAction::Save => ActionOutcome::saved_or_noop(handle_save(host, current_path, window)),
        FileAction::SaveAs => {
            ActionOutcome::saved_or_noop(handle_save_as(host, current_path, window))
        }
        FileAction::ExportImage => {
            // main.rs intercepts ExportImage to open the picker; this
            // fallback keeps external callers working.
            host.editor_state_mut().editor_ui.export_dialog_open = true;
            host.mark_editor_state_dirty();
            ActionOutcome::Noop
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
                .set_title(op_i18n::translate(
                    host.editor_state().editor_ui.locale,
                    "dialog.pickerExportTitle",
                ))
                .add_filter(filter_label, filter_exts)
                .set_file_name(&default_name)
                .save_file()
            {
                // The export renderers consume a layout-resolved
                // `LayoutScene` — build one from the live editor state
                // (runs jian's flex pass + `$ref` fill resolution).
                let scene = op_pen_loader::editor_state_to_layout_scene(host.editor_state());
                let scene = &scene;
                // When exactly one node is selected, raster export
                // crops to that layer (TS parity: exportLayerToRaster);
                // otherwise the whole active page is exported. SVG /
                // PDF always stay page-level.
                let single_node: Option<String> = {
                    let st = host.editor_state();
                    if st.selection_count() == 1 && st.selection.anchor.is_real() {
                        Some(st.selection.anchor.as_str().to_string())
                    } else {
                        None
                    }
                };
                let raster = |rf: crate::export::RasterFormat| -> Result<(), String> {
                    match &single_node {
                        Some(id) => crate::export::export_node_raster(scene, id, &path, rf, scale),
                        None => crate::export::export_raster(scene, &path, rf, scale),
                    }
                };
                let result: Result<(), String> = match fmt {
                    Fmt::Png => raster(crate::export::RasterFormat::Png),
                    Fmt::Jpeg => raster(crate::export::RasterFormat::Jpeg),
                    Fmt::Webp => raster(crate::export::RasterFormat::Webp),
                    Fmt::Svg => crate::export::export_svg(scene, &path),
                    Fmt::Pdf => crate::export_pdf::export_pdf(scene, &path),
                };
                if let Err(e) = result {
                    eprintln!("[export-image] {e}");
                    show_error_dialog(host, ErrorKind::Export, Some(&path), &e);
                }
            }
            ActionOutcome::Noop
        }
        FileAction::OpenRecent(i) => {
            let Some(entry) = host.editor_state().editor_ui.recent_files.get(i).cloned() else {
                return ActionOutcome::Noop;
            };
            let path = std::path::PathBuf::from(&entry.path);
            match load_into_host(host, &path) {
                Ok(()) => {
                    crate::settings_io::touch_recent(host, &path);
                    *current_path = Some(path);
                    refresh_title(current_path, window);
                    ActionOutcome::Saved
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
                    ActionOutcome::Noop
                }
            }
        }
        FileAction::ClearRecent => {
            host.editor_state_mut().editor_ui.recent_files.clear();
            host.mark_editor_state_dirty();
            ActionOutcome::Noop
        }
        FileAction::ImportFigma => {
            let path = match rfd::FileDialog::new()
                .set_title(op_i18n::translate(
                    host.editor_state().editor_ui.locale,
                    "dialog.pickerOpenTitle",
                ))
                .add_filter("Figma", &["fig"])
                .pick_file()
            {
                Some(p) => p,
                None => return ActionOutcome::Noop,
            };
            // Spawn the parse on a worker thread so the UI keeps
            // repainting (a 2–3 MB .fig with hundreds of nodes takes
            // multiple seconds; running it on the main thread freezes
            // the window). The desktop runner picks up the session in
            // the next `RedrawRequested` pump and applies the result
            // when it lands.
            ActionOutcome::FigmaImportStarted(path)
        }
        FileAction::ImportImageOrSvg => {
            crate::persistence_image::handle_import_image_or_svg(host);
            ActionOutcome::Noop
        }
        FileAction::PickFillImage => {
            crate::persistence_image::handle_pick_fill_image(host);
            ActionOutcome::Noop
        }
        FileAction::RelinkImage => {
            crate::persistence_image::handle_relink_image(host, current_path.as_deref());
            ActionOutcome::Noop
        }
    }
}

// `import_figma_into_host` (synchronous parse) was retired in favour
// of `figma_import_session::spawn`, which moves the parse to a worker
// thread and pumps the result back through a channel each frame.

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
    let locale = host.editor_state().editor_ui.locale;
    let (title_key, lead_key) = match kind {
        ErrorKind::Open => ("dialog.openErrorTitle", "dialog.openErrorLead"),
        ErrorKind::Save => ("dialog.saveErrorTitle", "dialog.saveErrorLead"),
        ErrorKind::Export => ("dialog.exportErrorTitle", "dialog.exportErrorLead"),
    };
    let mut body = op_i18n::translate(locale, lead_key).to_string();
    if let Some(p) = path {
        body.push_str("\n\n");
        body.push_str(&p.display().to_string());
    }
    body.push_str("\n\n");
    body.push_str(detail);
    rfd::MessageDialog::new()
        .set_title(op_i18n::translate(locale, title_key))
        .set_description(&body)
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Open,
    Save,
    Export,
}

/// Public re-export of the native error dialog — used by the
/// background Figma import session (`figma_import_session::pump`) to
/// pop the same OS dialog the synchronous error path uses.
pub fn show_error_dialog_public(
    host: &WidgetHostNative,
    kind: ErrorKind,
    path: Option<&std::path::Path>,
    detail: &str,
) {
    show_error_dialog(host, kind, path, detail)
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

        let reloaded =
            load_editor_state(&path, op_editor_core::Locale::EnUs).expect("load succeeds");
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

        let reloaded =
            load_editor_state(&path, op_editor_core::Locale::EnUs).expect("load succeeds");
        // Single-page document → only index 0 is valid.
        assert_eq!(reloaded.ui.active_page_index, 0);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
    }

    #[test]
    fn document_fingerprint_is_stable_and_change_sensitive() {
        let state = EditorState::new();
        let fp = document_fingerprint(&state);
        assert_eq!(fp, document_fingerprint(&state), "stable for the same doc");

        let mut mutated = EditorState::new();
        mutated.add_page();
        assert_ne!(
            fp,
            document_fingerprint(&mutated),
            "a structural change moves the fingerprint"
        );
    }

    #[test]
    fn new_file_action_resets_to_starter_frame() {
        let mut host = WidgetHostNative::new();
        host.editor_state_mut().doc.children.clear();
        host.editor_state_mut().viewport.pan_x = -5000.0;
        host.editor_state_mut().viewport.pan_y = -5000.0;
        host.editor_state_mut().viewport.zoom = 0.2;
        let mut current_path = Some(PathBuf::from("/tmp/old.op"));

        let outcome = run_action(
            op_editor_core::editor_ui_state::FileAction::New,
            &mut host,
            &mut current_path,
            None,
        );

        assert_eq!(outcome, ActionOutcome::Saved);
        assert!(current_path.is_none());
        assert_eq!(host.editor_state().doc.children.len(), 1);
        assert_eq!(
            host.editor_state().selection.anchor,
            op_editor_core::NodeId::new("n10")
        );
        let frame = match &host.editor_state().doc.children[0] {
            jian_ops_schema::node::PenNode::Frame(frame) => frame,
            other => panic!(
                "new file should create the blank starter frame, got {:?}",
                other
            ),
        };
        assert_eq!(frame.base.x, Some(0.0));
        assert_eq!(frame.base.y, Some(0.0));
        assert!(matches!(
            frame.container.width,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(1200.0))
        ));
        assert!(matches!(
            frame.container.height,
            Some(jian_ops_schema::sizing::SizingBehavior::Number(800.0))
        ));
        let v = host.editor_state().viewport;
        assert!((v.zoom - 0.68).abs() < 1e-3, "zoom {}", v.zoom);
        assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
        assert!((v.pan_y - 158.0).abs() < 1e-2, "pan_y {}", v.pan_y);
    }

    #[test]
    fn new_file_action_preserves_builtin_agent_models() {
        let mut host = WidgetHostNative::new();
        let builtin_id = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .add_builtin_agent_config(
                "DS",
                "sk-test",
                "deepseek-v4-pro",
                op_editor_core::BuiltinAgentKind::OpenAiCompat,
                "https://api.deepseek.com/v1",
            );
        host.editor_state_mut().rebuild_chat_models();
        let mut current_path = Some(PathBuf::from("/tmp/old.op"));

        let outcome = run_action(
            op_editor_core::editor_ui_state::FileAction::New,
            &mut host,
            &mut current_path,
            None,
        );

        assert_eq!(outcome, ActionOutcome::Saved);
        assert_eq!(
            host.editor_state()
                .editor_ui
                .agent_settings
                .builtin_agents
                .len(),
            1
        );
        assert!(host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));
    }

    #[test]
    fn opening_document_preserves_builtin_agent_models() {
        let mut host = WidgetHostNative::new();
        let builtin_id = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .add_builtin_agent_config(
                "MINIMAX",
                "sk-test",
                "MiniMax-M2.7",
                op_editor_core::BuiltinAgentKind::OpenAiCompat,
                "https://api.minimaxi.com/v1",
            );
        host.editor_state_mut().rebuild_chat_models();
        assert!(host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));

        let state_to_open = EditorState::new();
        let path = temp_op_path("open-preserves-builtins");
        save_to_path(&state_to_open, &path).expect("save succeeds");
        let mut current_path = None;

        assert!(open_path(&mut host, path.clone(), &mut current_path, None));

        assert_eq!(
            host.editor_state()
                .editor_ui
                .agent_settings
                .builtin_agents
                .len(),
            1
        );
        assert!(host
            .editor_state()
            .chat
            .available_models
            .iter()
            .any(|m| m.builtin_provider_id.as_deref() == Some(builtin_id.as_str())));

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

        let err = load_editor_state(&path, op_editor_core::Locale::EnUs)
            .expect_err("legacy file is rejected");
        // The legacy detector surfaces the `dialog.loadErrorOldVersion`
        // localised message rather than an opaque schema error.
        assert_eq!(
            err,
            op_i18n::translate(op_editor_core::Locale::EnUs, "dialog.loadErrorOldVersion")
        );

        let _ = std::fs::remove_file(&path);
    }
}
