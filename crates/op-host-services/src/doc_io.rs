//! Headless `.pen` / `.op` document load / save — the host-free core
//! carved out of `op-host-desktop`'s `persistence.rs` (Phase 2, Task
//! 2.3). The rfd / winit dialog flow (`handle_open` / `handle_save` /
//! `run_action` / `show_error_dialog`) stays desktop-side and imports
//! these functions back; the web daemon and any other headless caller
//! reach the same serializer the GUI uses.
//!
//! Save serializes `EditorState::doc` straight to canonical `.op` JSON
//! (the same schema the TS editor / Jian apps emit); Open parses the
//! canonical schema and re-seeds `EditorState` via
//! `EditorState::from_document`. There is no `Document → PenDocument`
//! reverse path, so the desktop's old private `DocPayload` format is no
//! longer written — every save is canonical.
//!
//! ## Embedded editor view-state (`editorMeta`)
//!
//! The canonical `PenDocument` schema has no field for editor
//! view-state — most of it (selection / viewport / tool) is deliberately
//! transient. `active_page_index` and the document-derived
//! `preserve_authored_geometry` layout latch must survive a save / load
//! round-trip, so Save embeds them under a top-level `editorMeta` extension.
//! Older files that still have the former `<path>.opmeta` sidecar continue to
//! load best-effort; new saves remove stale sidecars.
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

pub mod atomic_file;
mod canonical_save;
mod clean_copy;
#[cfg(test)]
mod editor_meta_roundtrip_tests;
mod error;
mod load;
mod load_report;
#[cfg(test)]
mod load_report_tests;

use atomic_file::{create_sibling_temp, replace_file};
pub use canonical_save::{write_canonical_document, CanonicalSaveSnapshot, StreamingSaveStats};
pub use clean_copy::{
    copy_clean_document_with_editor_meta_to_path, copy_document_to_current_schema_path,
};
pub use error::DocIoError;
#[cfg(test)]
use load::looks_like_legacy_doc_payload;
pub use load::{load_editor_state, load_editor_state_from_source, load_editor_state_with_report};
pub use load_report::{DocumentLoadReport, LoadedEditorState};

/// Legacy sidecar path for a given `.op` / `.pen` file —
/// `<path>.opmeta`. Public so compatibility tests and stale-sidecar
/// cleanup use the same path convention older builds wrote.
pub fn sidecar_path(path: &std::path::Path) -> PathBuf {
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
/// Editor metadata is embedded under the top-level `editorMeta` extension so
/// the active page and Preserve-import geometry mode survive the round-trip
/// without a separate sidecar.
pub fn save_to_path(state: &EditorState, path: &std::path::Path) -> Result<(), DocIoError> {
    let thumbnails = jian_ops_schema::image_thumbs::capture_snapshot();
    save_document_with_thumbnails_to_path(
        &state.doc,
        state.ui.active_page_index,
        state.editor_ui.preserve_authored_geometry,
        &thumbnails,
        path,
    )
}

/// Save a borrowed canonical document without first cloning it. Synchronous
/// callers use this path; background desktop jobs should capture a
/// [`CanonicalSaveSnapshot`] on the UI thread and call
/// [`save_snapshot_to_path`] in their worker instead.
pub fn save_document_to_path(
    document: &jian_ops_schema::PenDocument,
    active_page_index: usize,
    path: &std::path::Path,
) -> Result<(), DocIoError> {
    let thumbnails = jian_ops_schema::image_thumbs::capture_snapshot();
    // This document-only compatibility entry point predates the authored-
    // geometry latch. Without an EditorState there is no truthful value to
    // capture, so retain the legacy normal-layout default.
    save_document_with_thumbnails_to_path(document, active_page_index, false, &thumbnails, path)
}

/// Save a self-contained snapshot captured for a background job.
pub fn save_snapshot_to_path(
    snapshot: &CanonicalSaveSnapshot,
    path: &std::path::Path,
) -> Result<(), DocIoError> {
    save_serializable_document_with_thumbnails_to_path(
        snapshot.document(),
        snapshot.active_page_index(),
        snapshot.preserve_authored_geometry(),
        snapshot.image_thumbnails(),
        path,
    )
}

fn save_document_with_thumbnails_to_path(
    document: &jian_ops_schema::PenDocument,
    active_page_index: usize,
    preserve_authored_geometry: bool,
    thumbnails: &jian_ops_schema::image_thumbs::ImageThumbSnapshot,
    path: &std::path::Path,
) -> Result<(), DocIoError> {
    save_serializable_document_with_thumbnails_to_path(
        document,
        active_page_index,
        preserve_authored_geometry,
        thumbnails,
        path,
    )
}

fn save_serializable_document_with_thumbnails_to_path<
    D: serde::Serialize + jian_ops_schema::image_table::SaveImageOrder + ?Sized,
>(
    document: &D,
    active_page_index: usize,
    preserve_authored_geometry: bool,
    thumbnails: &jian_ops_schema::image_thumbs::ImageThumbSnapshot,
    path: &std::path::Path,
) -> Result<(), DocIoError> {
    // Write through a sibling temp file so a crash mid-write doesn't
    // leave a half-written file on disk.
    let (tmp, file) = create_sibling_temp(path)?;
    let write_result = (|| {
        let mut writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
        canonical_save::write_serializable_document_with_thumbnails(
            &mut writer,
            document,
            active_page_index,
            preserve_authored_geometry,
            thumbnails,
        )?;
        std::io::Write::flush(&mut writer).map_err(|e| DocIoError::Io(e.to_string()))?;
        drop(writer);
        commit_staged_document(&tmp, path)
    })();
    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    write_result
}

/// Atomically install a completed sibling `.op` file at its fixed destination.
///
/// Figma import writes the large document to a hidden sibling first so it can
/// still observe cancellation before publication. Reusing the save path's
/// replace primitive keeps re-imports crash-safe while allowing the existing
/// `<source-stem>.op` to be replaced instead of creating numbered copies.
pub fn commit_staged_document(
    staging_path: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), DocIoError> {
    replace_file(staging_path, destination)?;
    // Old builds wrote `<path>.opmeta`. New saves are single-file, so
    // remove any stale sidecar after the document write has committed.
    match std::fs::remove_file(sidecar_path(destination)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("[save] stale view-state sidecar cleanup failed: {e}");
        }
    }
    Ok(())
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

/// Set `editor_ui.file_name_display` from a path's file name (or clear
/// it for an unsaved document). Public because both the carved load
/// path and the desktop residual's `set_display_name` host wrapper
/// call it (codex Issue 4 — shared helper).
pub fn set_file_name_display(state: &mut EditorState, path: Option<&std::path::Path>) {
    state.editor_ui.file_name_display = path
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned());
}

/// Carry app-level preferences from the previously open document onto a
/// freshly loaded / reset one (theme / locale / recents / agent config
/// / UIKits / theme presets / chat model selection). `from_document`
/// resets these to built-ins, so the New / Open paths re-apply them.
/// Public — called by both the carved load path and the desktop
/// residual's `run_action` New branch (codex Issue 4 — shared helper).
pub fn preserve_app_preferences(previous: &EditorState, next: &mut EditorState) {
    let previous_selected_model = previous.chat.selected_model_entry().cloned();
    next.editor_ui.theme_mode = previous.editor_ui.theme_mode;
    next.editor_ui.locale = previous.editor_ui.locale;
    next.editor_ui.recent_files = previous.editor_ui.recent_files.clone();
    next.editor_ui.font_import_supported = previous.editor_ui.font_import_supported;
    next.editor_ui.batch_frame_export_supported = previous.editor_ui.batch_frame_export_supported;
    next.editor_ui.system_fonts_loaded = previous.editor_ui.system_fonts_loaded;
    next.editor_ui.system_font_families = previous.editor_ui.system_font_families.clone();
    next.editor_ui.bundled_font_families = previous.editor_ui.bundled_font_families.clone();
    next.editor_ui.imported_font_families = previous.editor_ui.imported_font_families.clone();
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

/// Layout-resolved content bounding box of the active page (min_x, min_y,
/// max_x, max_y) in document coordinates, or `None` for an empty page.
///
/// The canonical document can store keyword sizing such as `fit_content`, so
/// raw authored bounds are insufficient here: a content-sized root has a raw
/// height of zero even though the canvas lays it out to its children's height.
/// Keep open diagnostics on the same resolved scene geometry used by canvas
/// framing and raster export.
pub fn active_page_bbox(state: &EditorState) -> Option<(f64, f64, f64, f64)> {
    let bounds = op_pen_loader::editor_state_to_active_page_layout_scene(state).content_bounds()?;
    let min_x = f64::from(bounds.origin.x);
    let min_y = f64::from(bounds.origin.y);
    Some((
        min_x,
        min_y,
        min_x + f64::from(bounds.size.x),
        min_y + f64::from(bounds.size.y),
    ))
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
/// `figma_import_session::spawn_approved` rather than the `.op`-only
/// `open_path`. Case-insensitive (Figma's "Save Local Copy" emits
/// `.fig`; some macOS shares fold to `.FIG`).
pub fn is_supported_figma_import(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("fig"))
}

/// True for HTML pages (`.html` / `.htm`) and packaged HTML projects
/// (`.zip`) — routed through the desktop `html_import_session`
/// (op-html structured import) rather than the `.op`-only `open_path`.
/// Case-insensitive like the other extension filters.
pub fn is_supported_html_import(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("html")
                || ext.eq_ignore_ascii_case("htm")
                || ext.eq_ignore_ascii_case("zip")
        })
}

/// Outcome of the desktop residual's `run_action` — tells the desktop
/// runner which post-action bookkeeping to run. Lives here (not on the
/// desktop side) so the headless daemon can name it too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionOutcome {
    /// The document now matches a file on disk (New / successful
    /// Open / Save / Save-As / Open-Recent). The runner refreshes the
    /// unsaved-changes baseline AND rebinds the Git session.
    Saved,
    /// User picked a `.fig` and the desktop runner should spawn the
    /// background parser (`figma_import_session::spawn_approved`). The actual
    /// document swap happens later when `figma_import_session::pump`
    /// drains the worker's result + rebinds the Git session itself
    /// (the previously-open repo binding goes stale on import).
    FigmaImportStarted(PathBuf),
    /// User chose a page, all pages, or cancel in the prepared Figma
    /// page selector. The desktop runner owns and consumes the session.
    FigmaImportSelection(op_editor_core::FigmaImportSelection),
    /// User picked a saved web page or ZIP project and the desktop
    /// runner should spawn the background HTML import worker
    /// (`html_import_session::spawn`), which applies the parsed document
    /// when it lands.
    HtmlImportStarted(PathBuf),
    /// A legacy synchronous Save-As entry reached a collaboration-bound
    /// document. The desktop runner must schedule its background Save-As fork
    /// flow; synchronous persistence has no authority to detach the session.
    SaveAsForkRequired,
    /// Nothing to reconcile — export, recent-list edits, or a user
    /// cancel / error.
    Noop,
}

impl ActionOutcome {
    /// Map a save/open helper's `bool` (`true` = the document now
    /// matches a file on disk) onto an outcome. Public — the desktop
    /// residual's `run_action` calls it (codex Issue 4 — shared helper).
    pub fn saved_or_noop(saved: bool) -> Self {
        if saved {
            ActionOutcome::Saved
        } else {
            ActionOutcome::Noop
        }
    }
}

/// Which file operation produced an error — picks the error dialog's
/// title / lead copy. Lives here so the headless callers and the
/// desktop `show_error_dialog` agree on the variant set.
#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Open,
    Save,
    Export,
}

#[cfg(test)]
mod tests {
    #[test]
    fn html_import_extensions() {
        use std::path::Path;
        assert!(is_supported_html_import(Path::new("a.html")));
        assert!(is_supported_html_import(Path::new("A.HTM")));
        assert!(is_supported_html_import(Path::new("site.zip")));
        assert!(is_supported_html_import(Path::new("SITE.ZIP")));
        assert!(!is_supported_html_import(Path::new("a.svg")));
        assert!(!is_supported_html_import(Path::new("html")));
    }

    #[test]
    fn app_preferences_preserve_runtime_font_availability() {
        let mut previous = EditorState::new();
        previous.editor_ui.font_import_supported = true;
        previous.editor_ui.system_fonts_loaded = true;
        previous.editor_ui.system_font_families = std::sync::Arc::new(vec!["PingFang SC".into()]);
        previous.editor_ui.bundled_font_families = std::sync::Arc::new(vec!["Inter".into()]);
        previous.editor_ui.imported_font_families = std::sync::Arc::new(vec!["Brand Sans".into()]);
        let mut next = EditorState::new();

        preserve_app_preferences(&previous, &mut next);

        assert!(next.editor_ui.font_import_supported);
        assert!(next.editor_ui.system_fonts_loaded);
        assert_eq!(&*next.editor_ui.system_font_families, &["PingFang SC"]);
        assert_eq!(&*next.editor_ui.bundled_font_families, &["Inter"]);
        assert_eq!(&*next.editor_ui.imported_font_families, &["Brand Sans"]);
    }

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
        // Editor view-state (`active_page_index`) is persisted inside
        // the `.op` file so a save / load round-trip restores it
        // without a separate `.opmeta` sidecar.
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
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("saved document"))
                .expect("saved document json");
        assert_eq!(saved["editorMeta"]["activePageIndex"], 2);
        assert_eq!(saved["editorMeta"]["preserveAuthoredGeometry"], false);
        assert!(
            !sidecar_path(&path).exists(),
            "new saves should not create .opmeta sidecars"
        );

        // Cleanup.
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
    }

    #[test]
    fn sibling_temp_files_are_create_new_and_unique() {
        let target = temp_op_path("unique-save-temp");
        let (first_path, first_file) = create_sibling_temp(&target).expect("first temp");
        let (second_path, second_file) = create_sibling_temp(&target).expect("second temp");
        assert_ne!(first_path, second_path);
        assert_eq!(first_path.parent(), target.parent());
        assert_eq!(second_path.parent(), target.parent());
        drop(first_file);
        drop(second_file);
        let _ = std::fs::remove_file(first_path);
        let _ = std::fs::remove_file(second_path);
    }

    #[test]
    fn overlapping_saves_use_independent_temps_and_leave_valid_json() {
        let target = temp_op_path("overlapping-saves");
        let mut workers = Vec::new();
        for index in 0..4 {
            let target = target.clone();
            workers.push(std::thread::spawn(move || {
                let mut state = EditorState::new();
                state.doc.name = Some(format!("save-{index}"));
                save_to_path(&state, &target)
            }));
        }
        for worker in workers {
            worker.join().expect("save worker panicked").expect("save");
        }
        let saved: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&target).expect("one complete destination remains"),
        )
        .expect("destination is valid JSON");
        assert!(saved["name"]
            .as_str()
            .is_some_and(|name| name.starts_with("save-")));
        assert!(saved.get("editorMeta").is_some());
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn legacy_sidecar_active_page_index_is_still_loaded_and_clamped() {
        // A legacy sidecar that names a page that no longer exists must
        // not leave the editor on an out-of-range index.
        let state = EditorState::new();
        let path = temp_op_path("page-clamp");
        save_to_path(&state, &path).expect("save succeeds");
        // Legacy sidecar from older versions.
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
    fn embedded_editor_meta_takes_precedence_over_legacy_sidecar() {
        let path = temp_op_path("embedded-meta");
        std::fs::write(
            &path,
            r#"{"version":"1.0.0","editorMeta":{"activePageIndex":1},"children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[]}]}"#,
        )
        .expect("write merged document");
        std::fs::write(sidecar_path(&path), r#"{"active_page_index":0}"#)
            .expect("write stale sidecar");

        let reloaded =
            load_editor_state(&path, op_editor_core::Locale::EnUs).expect("load succeeds");
        assert_eq!(reloaded.ui.active_page_index, 1);

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
    fn active_page_bbox_resolves_fit_content_root_height() {
        let doc = jian_ops_schema::load_str(
            r#"{
              "version":"1.0.0",
              "children":[{
                "type":"frame", "id":"root", "name":"Explore",
                "x":12, "y":24, "width":390, "height":"fit_content",
                "layout":"vertical",
                "children":[
                  {"type":"frame", "id":"header", "width":"fill_container", "height":62},
                  {"type":"frame", "id":"content", "width":"fill_container", "height":616},
                  {"type":"frame", "id":"tabs", "width":"fill_container", "height":72}
                ]
              }]
            }"#,
        )
        .expect("fixture parses")
        .value;
        let state = EditorState::from_document(doc);

        let (min_x, min_y, max_x, max_y) =
            active_page_bbox(&state).expect("fit-content root has resolved bounds");

        assert!((min_x - 12.0).abs() < 0.01, "min_x={min_x}");
        assert!((min_y - 24.0).abs() < 0.01, "min_y={min_y}");
        assert!((max_x - 402.0).abs() < 0.01, "max_x={max_x}");
        assert!((max_y - 774.0).abs() < 0.01, "max_y={max_y}");
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
        let canonical = r#"{"version":"1.0.0","children":[]}"#;
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
            err.to_string(),
            op_i18n::translate(op_editor_core::Locale::EnUs, "dialog.loadErrorOldVersion")
        );

        let _ = std::fs::remove_file(&path);
    }

    /// A multi-page document whose FIRST page is an empty cover opened onto
    /// plain canvas ("content bbox None") and read as a broken file — the
    /// content was on page 2 (measured: zwiki-ui-states.op, 16 pages).
    #[test]
    fn a_file_with_an_empty_first_page_opens_on_the_first_page_with_content() {
        let path = temp_op_path("empty-first-page");
        std::fs::write(
            &path,
            r##"{ "version": "1.0", "children": [], "pages": [
                { "id": "p1", "name": "Cover", "children": [] },
                { "id": "p2", "name": "States", "children": [
                    { "type": "frame", "id": "n1", "name": "Screen",
                      "width": 390, "height": 844 }
                ]}
            ]}"##,
        )
        .expect("write");
        let state = load_editor_state(&path, op_editor_core::Locale::EnUs).expect("loads");
        assert_eq!(
            state.ui.active_page_index, 1,
            "the blank cover is skipped — the file opens where the design is"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_whose_first_page_has_content_still_opens_on_it() {
        let path = temp_op_path("first-page-content");
        std::fs::write(
            &path,
            r##"{ "version": "1.0", "children": [], "pages": [
                { "id": "p1", "name": "Home", "children": [
                    { "type": "frame", "id": "n1", "width": 390, "height": 844 }
                ]},
                { "id": "p2", "name": "Detail", "children": [
                    { "type": "frame", "id": "n2", "width": 390, "height": 844 }
                ]}
            ]}"##,
        )
        .expect("write");
        let state = load_editor_state(&path, op_editor_core::Locale::EnUs).expect("loads");
        assert_eq!(state.ui.active_page_index, 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_entirely_empty_file_still_lands_on_page_zero() {
        let path = temp_op_path("all-empty");
        std::fs::write(
            &path,
            r##"{ "version": "1.0", "children": [], "pages": [
                { "id": "p1", "name": "A", "children": [] },
                { "id": "p2", "name": "B", "children": [] }
            ]}"##,
        )
        .expect("write");
        let state = load_editor_state(&path, op_editor_core::Locale::EnUs).expect("loads");
        assert_eq!(state.ui.active_page_index, 0);
        let _ = std::fs::remove_file(&path);
    }
}
