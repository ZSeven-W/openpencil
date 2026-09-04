//! `.op` Save / Save As for the mobile editor shells.
//!
//! Two destinations exist, and the shell decides which one it can offer.
//!
//! * **Shell-owned (picker) saves.** Every touch shell that declares
//!   [`op_editor_configure_save_picker`] routes Save / Save As through the
//!   platform file picker, so the user places the document somewhere their
//!   file manager can reach. The picker hands back a *handle*, not a path
//!   (Android: a SAF `content://` URI plus persisted permission; HarmonyOS:
//!   a `DocumentViewPicker` file URI; iOS: base64 security-scoped bookmark
//!   data), so the engine can never write it directly. The engine stays the
//!   only writer of canonical `.op` bytes: it streams them into an
//!   app-private staging file the shell names, and the shell copies that
//!   file into the picked destination. See [`crate::editor_document_shell`].
//! * **Engine-owned paths.** A shell that declares no picker keeps the
//!   original behaviour: the engine owns the destination directory —
//!   `documents_root` from `op_create` when the shell has a user-visible one
//!   (iOS `NSDocumentDirectory`, which the Files app surfaces under
//!   "On My iPhone ▸ OpenPencil"), else `documents/` under the private
//!   storage root — and the engine-painted name dialog names the file.
//!
//! `documents_root` still matters for a picker shell: it is where legacy
//! documents are migrated to (see [`migrate_legacy_documents`]), where the
//! iOS picker opens by default, and where the suspend shadow copy lands.
//!
//! Either way the bytes come from the exact canonical writer the desktop's
//! `doc_io::save_to_path` wraps —
//! `jian_ops_schema::image_table::write_document_with_extension` with
//! `EditorMeta::from_state` — so a file written here round-trips through
//! every other host.
//!
//! Flow: the More sheet's Save / Save As tile queues
//! `FileAction::Save`/`SaveAs`; `editor_auth::take_shell_action` routes it
//! to [`begin_save`]. On a picker shell that stages bytes and returns
//! `SHELL_ACTION_SAVE_DOCUMENT`; otherwise a known path saves in place and
//! anything else opens the shared save-name dialog, whose confirmation
//! [`drain_confirmed_save`] performs.

use crate::error::{FfiError, FfiResult};
use crate::lifecycle::Session;
use crate::OpStatus;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Longest accepted file-name stem, in bytes (UTF-8, cut on a char
/// boundary). Keeps names under every mobile filesystem's 255-byte cap
/// with room for the ` NNN.op` dedup suffix.
const STEM_BYTE_CAP: usize = 120;

/// Where the current document is bound, if it has been saved at all.
///
/// Modelled explicitly rather than as "a path that might be missing":
/// a picker destination is an opaque, shell-owned handle the engine can
/// neither open nor write, and pretending otherwise is exactly how a
/// silent Save would end up writing the wrong file.
#[derive(Default)]
pub(crate) enum DocumentBinding {
    /// Never saved, or the binding was dropped by New / Open.
    #[default]
    None,
    /// A filesystem path the engine rewrites itself.
    Path(PathBuf),
    /// A destination only the shell can write. See [`ShellBinding`].
    Shell(ShellBinding),
}

/// A destination the platform picker owns.
///
/// `handle` is opaque to the engine and round-trips verbatim: the shell is
/// the only party that knows how to turn it back into a writable stream
/// (`ContentResolver.openOutputStream`, `fs.openSync`, or a resolved
/// security-scoped bookmark). It must be *durable* — the shell has to keep
/// it valid across process restarts (Android
/// `takePersistableUriPermission`, iOS bookmark data) — because a plain
/// Save is expected to rewrite it without prompting again.
pub(crate) struct ShellBinding {
    pub(crate) handle: String,
    /// What the picker actually named the file, for the TopBar and for the
    /// next save's suggested name.
    pub(crate) display_name: String,
}

/// Save-flow state for one session: the current binding, the shell's
/// destination directory, whether the shell drives a picker, and any
/// picker round trip currently in flight.
#[derive(Default)]
pub(crate) struct DocumentSaveShellState {
    pub(crate) binding: DocumentBinding,
    /// User-visible documents directory from `OpCreateDesc.documents_root`.
    /// `None` keeps the private `<storage_root>/documents` fallback.
    pub(crate) root: Option<PathBuf>,
    /// The shell implements `SHELL_ACTION_SAVE_DOCUMENT`, so Save / Save As
    /// go through its file picker instead of the engine's name dialog.
    /// Declared once by `op_editor_configure_save_picker`.
    pub(crate) picker: bool,
    /// Staged save waiting for the shell to report its picker outcome.
    pub(crate) pending: Option<crate::editor_document_shell::PendingShellSave>,
    /// A suspend flush wrote the shadow copy of a shell-bound document.
    /// The next shell-action drain (i.e. once the app is foregrounded and
    /// the engine is resumed) re-emits a silent save so the user's picked
    /// destination catches up.
    pub(crate) resave_pending: bool,
}

impl DocumentSaveShellState {
    pub(crate) fn with_root(root: Option<PathBuf>) -> Self {
        Self {
            root,
            ..Self::default()
        }
    }

    /// The engine-writable path this document is bound to, if any. A
    /// shell-owned binding deliberately has none.
    pub(crate) fn bound_path(&self) -> Option<&Path> {
        match &self.binding {
            DocumentBinding::Path(path) => Some(path),
            _ => None,
        }
    }

    pub(crate) fn shell_binding(&self) -> Option<&ShellBinding> {
        match &self.binding {
            DocumentBinding::Shell(binding) => Some(binding),
            _ => None,
        }
    }
}

/// Shell-action tail for document lifecycle: drain a confirmed save-name
/// dialog first, then any queued file action. Called by
/// `editor_auth::take_shell_action` after the auth / window / one-shot
/// request drains.
pub(crate) fn drain_document_actions(session: &mut Session) -> FfiResult<i32> {
    // A suspend flush could not reach a shell-owned destination; now that the
    // engine is drained again (so the app is foregrounded), ask the shell to
    // rewrite it silently. Runs before anything else so a user-initiated save
    // is never queued behind it.
    if let Some(action) = drain_suspend_resave(session) {
        return Ok(action);
    }
    // A confirmed save-name dialog writes into the sandbox engine-side; no
    // shell action is involved.
    if drain_confirmed_save(session)? {
        return Ok(crate::editor_auth::SHELL_ACTION_NONE);
    }

    let pending = session
        .editor_mut()?
        .editor_state()
        .editor_ui
        .pending_file_action;
    match pending {
        Some(op_editor_core::FileAction::New) => {
            install_new_document(session)?;
            Ok(crate::editor_auth::SHELL_ACTION_NONE)
        }
        Some(op_editor_core::FileAction::Open) => {
            let host = session.editor_mut()?;
            host.editor_state_mut().editor_ui.pending_file_action = None;
            host.mark_editor_state_dirty();
            Ok(crate::editor_auth::SHELL_ACTION_OPEN_DOCUMENT)
        }
        Some(op_editor_core::FileAction::Save) => begin_save(session, false),
        Some(op_editor_core::FileAction::SaveAs) => begin_save(session, true),
        Some(op_editor_core::FileAction::ImportImageOrSvg) => {
            crate::editor_image_import::begin_import(session)
        }
        #[cfg(any(target_os = "ios", target_os = "android", target_env = "ohos", test))]
        Some(op_editor_core::FileAction::ExportImageConfirm)
        | Some(op_editor_core::FileAction::ExportDeckPdfSelection) => {
            crate::editor_export::stage_export(session, pending)
        }
        _ => Ok(crate::editor_auth::SHELL_ACTION_NONE),
    }
}

/// File ▸ New: atomically install the starter document.
fn install_new_document(session: &mut Session) -> FfiResult<()> {
    let starter_document = op_editor_core::EditorState::starter().doc;
    {
        let host = session.editor_mut()?;
        // Consume the one-shot request even when collaboration starts between
        // the press and this drain. A rejected replacement must not retry on
        // every later frame.
        host.editor_state_mut().editor_ui.pending_file_action = None;
        host.install_open_document(starter_document, None, None)
            .map_err(|_| {
                FfiError::new(
                    OpStatus::Busy,
                    "new document is blocked by the collaboration session",
                )
            })?;
    }

    session.selected = None;
    // The starter document has no sandbox binding; drop the outgoing one.
    forget_current_document(session);
    session.gesture.reset();
    session.user_interacted = false;
    session.fit_content_to_viewports();
    // Fitting mutates the host-owned viewport. Clone only afterwards so the
    // lightweight state used by page APIs remains identical to the live host.
    session.state = session
        .editor()
        .ok_or_else(|| FfiError::new(OpStatus::NotReady, "engine is not in editor mode"))?
        .editor_state()
        .clone();
    session.scene = op_pen_loader::editor_state_to_active_page_layout_scene(&session.state);
    session.request_redraw();
    Ok(())
}

/// Handle a queued `FileAction::Save` / `SaveAs`.
///
/// Save with a known sandbox path writes in place. A first save — and every
/// Save As — opens the engine-painted name dialog instead; the write happens
/// when [`drain_confirmed_save`] sees the confirmation.
pub(crate) fn begin_save(session: &mut Session, save_as: bool) -> FfiResult<i32> {
    {
        // Consume the one-shot request first so a failure below cannot
        // retry on every later frame.
        let host = session.editor_mut()?;
        host.editor_state_mut().editor_ui.pending_file_action = None;
    }
    if !save_as {
        // A plain Save never re-asks where the document lives.
        if let Some(path) = session.document_save.bound_path().map(Path::to_path_buf) {
            write_current_document(session, &path)?;
            finish_successful_save(session, path, false);
            return Ok(crate::editor_auth::SHELL_ACTION_NONE);
        }
        if let Some(binding) = session.document_save.shell_binding() {
            let (handle, name) = (binding.handle.clone(), binding.display_name.clone());
            return crate::editor_document_shell::begin_shell_save(session, Some(handle), name);
        }
    }
    if session.document_save.picker {
        // First save, or Save As: the platform picker owns both the name and
        // the destination, so the engine's name dialog stays closed.
        let name = suggested_file_name(session)?;
        return crate::editor_document_shell::begin_shell_save(session, None, name);
    }
    let now_ms = session.now_ms;
    let host = session.editor_mut()?;
    let seed = seed_name(host.editor_state());
    host.editor_state_mut()
        .editor_ui
        .save_name_dialog
        .open_with(&seed, save_as, now_ms);
    host.mark_editor_state_dirty();
    session.request_redraw();
    Ok(crate::editor_auth::SHELL_ACTION_NONE)
}

/// `<stem>.op` the picker opens pre-filled with, derived from the same
/// seed the engine's name dialog would have shown.
pub(crate) fn suggested_file_name(session: &mut Session) -> FfiResult<String> {
    let host = session.editor_mut()?;
    Ok(format!(
        "{}.op",
        sanitize_stem(&seed_name(host.editor_state()))
    ))
}

/// Re-emit the silent save a suspend flush could not perform itself.
///
/// Returns `None` (and leaves no state behind) whenever the reconcile is
/// unnecessary or impossible — a clean document, a binding that is no longer
/// shell-owned, or a picker round trip already in flight. The flag is always
/// consumed so this can never spin on every frame.
fn drain_suspend_resave(session: &mut Session) -> Option<i32> {
    if !session.document_save.resave_pending {
        return None;
    }
    session.document_save.resave_pending = false;
    if session.document_save.pending.is_some() {
        return None;
    }
    let dirty = session
        .editor
        .as_ref()
        .is_some_and(|host| host.editor_state().is_dirty());
    if !dirty {
        return None;
    }
    let binding = session.document_save.shell_binding()?;
    let (handle, name) = (binding.handle.clone(), binding.display_name.clone());
    crate::editor_document_shell::begin_shell_save(session, Some(handle), name).ok()
}

/// Perform the write for a confirmed save-name dialog. Returns `true` when
/// a confirmation was drained (whether or not the write succeeded — on
/// failure the dialog stays open with the typed name so the user can retry,
/// and the error propagates to the shell).
pub(crate) fn drain_confirmed_save(session: &mut Session) -> FfiResult<bool> {
    // Save-first-time and Save As behave identically at the write: a fresh
    // unique target (dedupe rather than clobber a same-named document).
    let name = {
        let host = session.editor_mut()?;
        let dialog = &mut host.editor_state_mut().editor_ui.save_name_dialog;
        let Some(name) = dialog.take_confirmed_name() else {
            return Ok(false);
        };
        name
    };
    let dir = documents_dir(&session.document_save)?;
    let target = unique_target_path(&dir, &sanitize_stem(&name))?;
    write_current_document(session, &target)?;
    finish_successful_save(session, target, true);
    Ok(true)
}

/// Backgrounding flush: persist unsaved changes to whatever the engine can
/// reach by itself.
///
/// * An engine-owned path is overwritten and the document is marked saved —
///   the user-visible file is now current.
/// * A shell-owned binding cannot be written from here: reaching a
///   `content://` URI or a security-scoped bookmark means calling back into
///   the shell, and backgrounding is not a moment to start a UI round trip.
///   The bytes go into a private shadow copy so the work survives even a
///   process kill, the document deliberately stays **dirty** (the file the
///   user picked really is stale), and [`drain_suspend_resave`] rewrites the
///   picked destination on the next drain after resume.
/// * A document that was never saved is left alone — silently inventing a
///   file (and a name) for it would surprise more than it protects.
pub(crate) fn flush_on_suspend(session: &mut Session) {
    let dirty = session
        .editor
        .as_ref()
        .is_some_and(|host| host.editor_state().is_dirty());
    if !dirty {
        return;
    }
    if let Some(path) = session.document_save.bound_path().map(Path::to_path_buf) {
        match write_current_document(session, &path) {
            Ok(()) => {
                if let Some(host) = session.editor.as_mut() {
                    host.editor_state_mut().mark_saved_revision();
                    host.mark_editor_state_dirty();
                }
            }
            Err(error) => {
                // Backgrounding cannot show UI; leave the document dirty so
                // the next foreground save retries, and surface a diagnostic.
                session.emit_runtime_error(2, &error.message, "op-engine-ffi/save");
            }
        }
        return;
    }
    let Some(shadow) = session
        .document_save
        .shell_binding()
        .map(|binding| binding.display_name.clone())
        .and_then(|name| shadow_path(&session.document_save, &name).ok())
    else {
        return;
    };
    match write_current_document(session, &shadow) {
        // Still dirty on purpose: the picked destination has not been
        // rewritten yet, and claiming "saved" here would be a lie the user
        // would only discover by losing the delta.
        Ok(()) => session.document_save.resave_pending = true,
        Err(error) => session.emit_runtime_error(2, &error.message, "op-engine-ffi/save"),
    }
}

/// Private shadow copy for a shell-bound document that was backgrounded
/// while dirty. Hidden (dot-prefixed) so it never shows up next to the
/// user's own documents, and stable per document name so repeated
/// backgrounding does not litter the directory.
fn shadow_path(state: &DocumentSaveShellState, display_name: &str) -> FfiResult<PathBuf> {
    let stem = sanitize_stem(display_name.trim_end_matches(".op"));
    Ok(documents_dir(state)?.join(format!(".{stem}.autosave.op")))
}

/// The current document is being replaced (New / platform Open): its
/// sandbox binding and any stale name prompt must not survive onto the
/// incoming document.
pub(crate) fn forget_current_document(session: &mut Session) {
    session.document_save.binding = DocumentBinding::None;
    session.document_save.pending = None;
    session.document_save.resave_pending = false;
    if let Some(host) = session.editor.as_mut() {
        if host.editor_state().editor_ui.save_name_dialog.open {
            host.editor_state_mut().editor_ui.save_name_dialog.close();
            host.mark_editor_state_dirty();
        }
    }
}

fn finish_successful_save(session: &mut Session, path: PathBuf, close_dialog: bool) {
    if let Ok(host) = session.editor_mut() {
        let state = host.editor_state_mut();
        if close_dialog {
            state.editor_ui.save_name_dialog.close();
        }
        state.editor_ui.file_name_display = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        state.mark_saved_revision();
        host.mark_editor_state_dirty();
    }
    session.document_save.binding = DocumentBinding::Path(path);
    session.request_redraw();
}

/// Stream the live editor state to `path` through the canonical writer,
/// via a sibling temp file so a mid-write crash never leaves a truncated
/// document at the destination.
pub(crate) fn write_current_document(session: &mut Session, path: &Path) -> FfiResult<()> {
    let host = session.editor_mut()?;
    let state = host.editor_state();
    let meta = op_pen_loader::EditorMeta::from_state(state);
    let thumbnails = jian_ops_schema::image_thumbs::capture_snapshot();

    let io_error = |stage: &str, error: std::io::Error| {
        FfiError::new(
            OpStatus::InvalidArg,
            format!("could not {stage} the document file: {error}"),
        )
    };
    let tmp = sibling_temp_path(path);
    let result = (|| {
        let file = std::fs::File::create(&tmp).map_err(|e| io_error("create", e))?;
        let mut writer = std::io::BufWriter::with_capacity(1024 * 1024, file);
        jian_ops_schema::image_table::write_document_with_extension(
            &mut writer,
            &state.doc,
            &thumbnails,
            "editorMeta",
            &meta,
        )
        .map_err(|error| {
            FfiError::new(
                OpStatus::InvalidArg,
                format!("could not encode the document: {error}"),
            )
        })?;
        writer.flush().map_err(|e| io_error("write", e))?;
        drop(writer);
        std::fs::rename(&tmp, path).map_err(|e| io_error("commit", e))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn sibling_temp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.op".to_owned());
    name.push_str(".tmp");
    path.with_file_name(format!(".{name}"))
}

/// Where saves land: the shell's user-visible documents root when it
/// provided one, else the private fallback.
fn documents_dir(state: &DocumentSaveShellState) -> FfiResult<PathBuf> {
    let dir = match state.root.as_ref() {
        Some(root) => root.clone(),
        None => legacy_documents_dir()?,
    };
    std::fs::create_dir_all(&dir).map_err(|error| {
        FfiError::new(
            OpStatus::NotReady,
            format!("could not create the documents directory: {error}"),
        )
    })?;
    Ok(dir)
}

/// `documents/` under the shell-provided private storage root — the
/// pre-`documents_root` destination, and still the destination for shells
/// that pass no visible directory. Test binaries have no
/// `op_create`-configured root, so they fall back to the process config
/// dir, which the harness redirects to a scratch directory. Never creates
/// the directory: migration must be able to tell "absent" from "empty".
fn legacy_documents_dir() -> FfiResult<PathBuf> {
    let root = match op_config_store::configured_user_root() {
        Some(root) => root,
        None => op_config_store::openpencil_dir().map_err(|error| {
            FfiError::new(
                OpStatus::NotReady,
                format!("no private storage root is available: {error}"),
            )
        })?,
    };
    Ok(root.join("documents"))
}

/// Move every `.op` file left in the private `<storage_root>/documents`
/// into the shell's user-visible documents root, so documents saved before
/// the shell had one do not vanish from the user's view.
///
/// Runs on every editor create and is idempotent: a successful pass drains
/// (and removes) the legacy directory, so later launches see nothing to do.
/// Name collisions reuse the Save As dedupe rule — a legacy `poster.op`
/// landing next to an existing `poster.op` becomes `poster 2.op` rather
/// than clobbering it. A rename across devices falls back to copy + delete;
/// a file that cannot be moved at all is left where it is (a later launch
/// retries) rather than failing startup.
///
/// Returns the number of documents moved.
pub(crate) fn migrate_legacy_documents(state: &DocumentSaveShellState) -> FfiResult<usize> {
    let Some(target) = state.root.as_ref() else {
        return Ok(0);
    };
    migrate_documents(&legacy_documents_dir()?, target)
}

/// [`migrate_legacy_documents`] with both directories named explicitly.
pub(crate) fn migrate_documents(legacy: &Path, target: &Path) -> FfiResult<usize> {
    if !legacy.is_dir() || same_directory(legacy, target) {
        return Ok(0);
    }
    let entries = match std::fs::read_dir(legacy) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(FfiError::new(
                OpStatus::NotReady,
                format!("could not read the legacy documents directory: {error}"),
            ))
        }
    };
    let mut moved = 0usize;
    let mut created_target = false;
    for entry in entries.flatten() {
        let source = entry.path();
        if !source.is_file() || !has_op_extension(&source) {
            continue;
        }
        let stem = source
            .file_stem()
            .map(|stem| sanitize_stem(&stem.to_string_lossy()))
            .unwrap_or_else(|| "untitled".to_owned());
        if !created_target {
            std::fs::create_dir_all(target).map_err(|error| {
                FfiError::new(
                    OpStatus::NotReady,
                    format!("could not create the documents directory: {error}"),
                )
            })?;
            created_target = true;
        }
        let Ok(destination) = unique_target_path(target, &stem) else {
            continue;
        };
        if move_file(&source, &destination) {
            moved += 1;
        }
    }
    // Best-effort tidy-up; a non-empty (or busy) directory simply stays.
    let _ = std::fs::remove_dir(legacy);
    Ok(moved)
}

/// Rename, falling back to copy + delete across filesystems. A failed copy
/// clears its partial destination and leaves the source untouched; a
/// successful copy whose delete fails leaves both files, which the next
/// launch dedupes rather than loses.
fn move_file(source: &Path, destination: &Path) -> bool {
    if std::fs::rename(source, destination).is_ok() {
        return true;
    }
    if std::fs::copy(source, destination).is_err() {
        let _ = std::fs::remove_file(destination);
        return false;
    }
    let _ = std::fs::remove_file(source);
    true
}

fn has_op_extension(path: &Path) -> bool {
    path.extension()
        .map(|extension| extension.eq_ignore_ascii_case("op"))
        .unwrap_or(false)
}

/// Same directory even when one side is un-canonicalizable (not yet
/// created): fall back to a plain path comparison.
fn same_directory(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// Seed for the name dialog: the display name minus the canonical
/// extension, else the localized "Untitled" (未命名 for the default zh-CN
/// locale).
fn seed_name(state: &op_editor_core::EditorState) -> String {
    if let Some(name) = state
        .editor_ui
        .file_name_display
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let lower = name.to_ascii_lowercase();
        let stem = if lower.ends_with(".op") || lower.ends_with(".pen") {
            &name[..name.rfind('.').expect("checked suffix")]
        } else {
            name
        };
        let cleaned = sanitize_stem(stem);
        // "untitled" is the sanitizer's empty-input fallback; only keep it
        // when the display name genuinely says so, otherwise prefer the
        // locale default below.
        if cleaned != "untitled" || stem.trim().eq_ignore_ascii_case("untitled") {
            return cleaned;
        }
    }
    sanitize_stem(op_i18n::translate(
        state.editor_ui.effective_locale(),
        "common.untitled",
    ))
}

/// Make a typed name safe as a file-name stem: strip path separators and
/// characters the mobile filesystems (or later export to other platforms)
/// reject, collapse leading/trailing dots and whitespace, and cap the
/// length. An empty result becomes `untitled`.
fn sanitize_stem(name: &str) -> String {
    let mut cleaned: String = name
        .chars()
        .map(|c| {
            if op_editor_core::save_name_keyboard::is_forbidden_file_name_char(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    cleaned = cleaned.trim().trim_matches('.').trim().to_owned();
    if cleaned.len() > STEM_BYTE_CAP {
        let mut cut = STEM_BYTE_CAP;
        while !cleaned.is_char_boundary(cut) {
            cut -= 1;
        }
        cleaned.truncate(cut);
        cleaned = cleaned.trim_end().to_owned();
    }
    if cleaned.is_empty() {
        "untitled".to_owned()
    } else {
        cleaned
    }
}

/// First free `<stem>.op`, `<stem> 2.op`, … path inside `dir`. Save As to
/// an already-used name never overwrites — it writes the numbered copy.
fn unique_target_path(dir: &Path, stem: &str) -> FfiResult<PathBuf> {
    let first = dir.join(format!("{stem}.op"));
    if !first.exists() {
        return Ok(first);
    }
    for counter in 2..1000 {
        let candidate = dir.join(format!("{stem} {counter}.op"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(FfiError::new(
        OpStatus::InvalidArg,
        format!("too many documents named \"{stem}\""),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_replaces_separators_and_never_returns_empty() {
        assert_eq!(sanitize_stem("my/design:v2"), "my design v2");
        assert_eq!(sanitize_stem("  ..  "), "untitled");
        assert_eq!(sanitize_stem("...hidden"), "hidden");
        assert_eq!(sanitize_stem("海报设计"), "海报设计");
        let long = "长".repeat(200);
        let capped = sanitize_stem(&long);
        assert!(capped.len() <= STEM_BYTE_CAP);
        assert!(capped.chars().all(|c| c == '长'));
    }

    #[test]
    fn unique_target_dedupes_with_numeric_suffixes() {
        let dir = std::env::temp_dir().join(format!(
            "openpencil-ffi-save-unique-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let first = unique_target_path(&dir, "poster").expect("first");
        assert_eq!(first, dir.join("poster.op"));
        std::fs::write(&first, b"x").expect("occupy first");
        let second = unique_target_path(&dir, "poster").expect("second");
        assert_eq!(second, dir.join("poster 2.op"));
        std::fs::write(&second, b"x").expect("occupy second");
        let third = unique_target_path(&dir, "poster").expect("third");
        assert_eq!(third, dir.join("poster 3.op"));
        std::fs::remove_dir_all(&dir).expect("clean temp dir");
    }
}
