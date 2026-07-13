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
//! view-state — most of it (selection / viewport / tool) is
//! deliberately transient, but `active_page_index` is a small piece of
//! view-state the user expects to survive a save / load round-trip.
//! Save embeds that tiny editor-only blob in the `.op` file under a
//! top-level `editorMeta` extension. Older files that still have the
//! former `<path>.opmeta` sidecar continue to load best-effort; new
//! saves remove stale sidecars.
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

use std::{borrow::Cow, path::PathBuf};

use op_editor_core::EditorState;

/// Editor view-state embedded in the canonical `.op` file.
/// Kept intentionally minimal — `active_page_index` is the only piece
/// of view-state the user expects to survive a round-trip. Selection /
/// viewport / tool stay transient and are NOT persisted.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorMeta {
    /// 0-based active page index at save time.
    #[serde(default, alias = "active_page_index")]
    active_page_index: usize,
}

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
/// Editor view-state is embedded under the top-level `editorMeta`
/// extension so `active_page_index` survives the round-trip without a
/// separate sidecar.
pub fn save_to_path(state: &EditorState, path: &std::path::Path) -> Result<(), String> {
    let mut value = serde_json::to_value(&state.doc).map_err(|e| e.to_string())?;
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "editorMeta".into(),
            serde_json::to_value(EditorMeta {
                active_page_index: state.ui.active_page_index,
            })
            .map_err(|e| e.to_string())?,
        );
    }
    let json = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    // Write through a sibling temp file so a crash mid-write doesn't
    // leave a half-written file on disk.
    let mut tmp = path.to_path_buf();
    tmp.set_extension(match path.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{}.tmp", ext),
        None => "tmp".to_string(),
    });
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    // Old builds wrote `<path>.opmeta`. New saves are single-file, so
    // remove any stale sidecar after the document write has committed.
    match std::fs::remove_file(sidecar_path(path)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("[save] stale view-state sidecar cleanup failed: {e}");
        }
    }
    Ok(())
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
/// parser. Embedded `editorMeta` restores `active_page_index`; legacy
/// `.opmeta` sidecars remain a fallback for older files.
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
    let (embedded_meta, canonical_src) = split_editor_meta(src);
    let loaded = match op_pen_loader::load_canonical(canonical_src.as_ref()) {
        Ok(loaded) => loaded,
        Err(e) => {
            // Distinguish "old format" from a genuinely corrupt file so
            // the user gets actionable guidance instead of a raw parse
            // error.
            if looks_like_legacy_doc_payload(canonical_src.as_ref()) {
                return Err(op_i18n::translate(locale, "dialog.loadErrorOldVersion").to_string());
            }
            return Err(e.to_string());
        }
    };
    for w in &loaded.warnings {
        eprintln!("[open] schema warning: {:?}", w);
    }
    let mut state = EditorState::from_document(loaded.value);
    // Restore editor view-state from embedded metadata first, then from
    // the legacy `.opmeta` sidecar. Missing / unreadable /
    // out-of-range metadata leaves the freshly loaded state on page 0.
    if let Some(meta) = embedded_meta.or_else(|| legacy_sidecar_meta(path)) {
        let page_count = state
            .doc
            .pages
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(1)
            .max(1);
        state.ui.active_page_index = meta.active_page_index.min(page_count - 1);
    } else {
        state.ui.active_page_index = first_page_with_content(&state);
    }
    Ok(state)
}

/// Which page to land on when the file carries no editor metadata. Page 0 is
/// the obvious answer — unless it is EMPTY and a later page is not: a 16-page
/// document whose first page is a blank cover then opened onto plain canvas
/// with a "content bbox None" and looked broken (measured: zwiki-ui-states.op).
/// A file that is empty everywhere still lands on page 0.
fn first_page_with_content(state: &EditorState) -> usize {
    let Some(pages) = state.doc.pages.as_ref() else {
        return 0;
    };
    if pages.first().is_some_and(|page| !page.children.is_empty()) {
        return 0;
    }
    pages
        .iter()
        .position(|page| !page.children.is_empty())
        .unwrap_or(0)
}

fn split_editor_meta(src: &str) -> (Option<EditorMeta>, Cow<'_, str>) {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(src) else {
        return (None, Cow::Borrowed(src));
    };
    let Some(obj) = value.as_object_mut() else {
        return (None, Cow::Borrowed(src));
    };
    let meta = obj
        .remove("editorMeta")
        .and_then(|value| serde_json::from_value::<EditorMeta>(value).ok());
    let Ok(canonical_src) = serde_json::to_string(&value) else {
        return (meta, Cow::Borrowed(src));
    };
    (meta, Cow::Owned(canonical_src))
}

fn legacy_sidecar_meta(path: &std::path::Path) -> Option<EditorMeta> {
    let meta_src = std::fs::read_to_string(sidecar_path(path)).ok()?;
    serde_json::from_str::<EditorMeta>(&meta_src).ok()
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
    let bounds = op_pen_loader::editor_state_to_layout_scene(state).content_bounds()?;
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
/// `figma_import_session::spawn` rather than the `.op`-only
/// `open_path`. Case-insensitive (Figma's "Save Local Copy" emits
/// `.fig`; some macOS shares fold to `.FIG`).
pub fn is_supported_figma_import(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("fig"))
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
        assert!(
            !sidecar_path(&path).exists(),
            "new saves should not create .opmeta sidecars"
        );

        // Cleanup.
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(sidecar_path(&path));
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
              "version":"0.8.0",
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
