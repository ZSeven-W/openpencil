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
/// Public so the desktop residual's tests can clean it up after a
/// round-trip.
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

/// Content bounding box of the active page (min_x, min_y, max_x,
/// max_y) in document coordinates, or `None` for an empty page. Public
/// because both the carved load path and the desktop residual's
/// `load_into_host` log it (codex Issue 4 — shared helper).
pub fn active_page_bbox(state: &EditorState) -> Option<(f64, f64, f64, f64)> {
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
