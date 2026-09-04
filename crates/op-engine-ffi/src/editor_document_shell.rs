//! Picker-backed Save / Save As: the shell-owned half of the save flow.
//!
//! Android 11+ hides `Android/data` from the file manager and HarmonyOS's
//! public directories are not path-writable, so on those platforms there is
//! no path the engine can write that the user can later find. iOS has one
//! (`NSDocumentDirectory`), but a user who wants the document in iCloud
//! Drive or a provider folder still has to be asked. All three therefore
//! route Save / Save As through the platform file picker.
//!
//! A picker hands back a *handle*, never a path — a SAF `content://` URI, a
//! `DocumentViewPicker` file URI, or security-scoped bookmark data — so this
//! module keeps the engine as the only writer of canonical `.op` bytes while
//! the shell is the only writer of the destination:
//!
//! ```text
//!  engine  begin_shell_save  -> SHELL_ACTION_SAVE_DOCUMENT
//!  shell   op_editor_copy_save_file_name   suggested "<stem>.op"
//!  shell   op_editor_copy_save_target      bound handle, or empty = prompt
//!  shell   (prompt only) run the picker
//!  shell   op_editor_stage_save_to_path    engine writes the canonical bytes
//!  shell   copy staging -> destination through the platform stream API
//!  shell   op_editor_commit_save(handle, name)   or op_editor_cancel_save
//! ```
//!
//! The staging file is app-private and shell-owned end to end: the shell
//! creates its directory, names it, and removes it on every terminal path —
//! exactly the discipline `editor_export` already uses for PNG/PDF, so
//! multi-megabyte documents never cross the C ABI as a byte array.
//!
//! Only [`op_editor_commit_save`] marks the document saved, and it does so
//! only after the shell reports that the destination really received the
//! bytes. A cancelled or failed round trip leaves the document dirty and the
//! previous binding intact.

use crate::error::{read_utf8, FfiError, FfiResult};
use crate::lifecycle::call_session;
#[cfg(feature = "editor")]
use crate::lifecycle::Session;
use crate::OpStatus;
use std::path::{Path, PathBuf};

/// Save the current document through the platform file picker.
///
/// Appended after the window-control codes so every existing action keeps
/// its number; a shell that does not implement it never sees it, because the
/// engine only emits it after `op_editor_configure_save_picker`.
pub const SHELL_ACTION_SAVE_DOCUMENT: i32 = 11;

/// Byte caps for the strings that cross this bridge. The handle is the
/// generous one: iOS bookmark data is kilobytes of base64, while a SAF URI
/// is short.
const SAVE_FILE_NAME_CAP: usize = 4 * 1024;
const SAVE_TARGET_CAP: usize = 64 * 1024;
const SAVE_PATH_CAP: usize = 16 * 1024;

/// One picker round trip in flight.
pub(crate) struct PendingShellSave {
    /// `<stem>.op` the picker opens pre-filled with, or the bound
    /// destination's own name when this is a silent rewrite.
    pub(crate) file_name: String,
    /// The durable handle to rewrite without prompting. `None` means the
    /// shell must present the picker.
    pub(crate) target: Option<String>,
    /// Where the canonical bytes were written, once staged.
    pub(crate) staged: Option<PathBuf>,
}

/// Freeze a save and hand it to the shell.
///
/// `target` is the bound handle for a silent rewrite (`None` prompts), and
/// `file_name` is the name the picker should pre-fill. Returns the shell
/// action so the caller can propagate it straight out of the drain.
pub(crate) fn begin_shell_save(
    session: &mut Session,
    target: Option<String>,
    file_name: String,
) -> FfiResult<i32> {
    if session.document_save.pending.is_some() {
        return Err(FfiError::new(
            OpStatus::Busy,
            "the previous save is still waiting for the platform file picker",
        ));
    }
    session.document_save.pending = Some(PendingShellSave {
        file_name,
        target,
        staged: None,
    });
    Ok(SHELL_ACTION_SAVE_DOCUMENT)
}

/// Declare that this shell drives Save / Save As through its file picker.
///
/// Called once, right after `op_create`, by every shell that implements
/// `OpShellAction_SaveDocument`. A shell that never calls it keeps the
/// engine-owned destination directory and the engine-painted name dialog,
/// so the ABI stays backward compatible.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_configure_save_picker(
    engine: *mut crate::OpEngine,
    enabled: bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            session.document_save.picker = enabled;
            Ok(())
        })
    }
}

/// Peek or copy the pending save's suggested UTF-8 file name.
///
/// A null buffer with zero capacity reports the required byte length.
/// Reading the name never consumes the pending save.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. `required` must be
/// writable; a non-null `buffer` must cover `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_copy_save_file_name(
    engine: *mut crate::OpEngine,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let pending = pending(&session.document_save)?;
            copy_out(pending.file_name.as_bytes(), buffer, capacity, required)
        })
    }
}

/// Peek or copy the durable destination handle the pending save should
/// rewrite.
///
/// Writes `0` to `required` and returns `Ok` when the document has no
/// binding yet — that is the signal to present the picker rather than an
/// error. Reading the handle never consumes the pending save.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. `required` must be
/// writable; a non-null `buffer` must cover `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_copy_save_target(
    engine: *mut crate::OpEngine,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let pending = pending(&session.document_save)?;
            let Some(target) = pending.target.as_deref() else {
                if required.is_null() {
                    return Err(FfiError::invalid(
                        "save target required-length pointer is null",
                    ));
                }
                required.write(0);
                return Ok(());
            };
            copy_out(target.as_bytes(), buffer, capacity, required)
        })
    }
}

/// Stream the live document's canonical `.op` bytes into a new shell-owned
/// absolute staging path.
///
/// The target must not exist. Staging does NOT consume the pending save nor
/// mark the document saved — only [`op_editor_commit_save`] does, once the
/// shell has actually placed the bytes at the destination.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. A non-empty path
/// byte range must cover readable UTF-8 bytes for its declared length.
#[no_mangle]
pub unsafe extern "C" fn op_editor_stage_save_to_path(
    engine: *mut crate::OpEngine,
    path_ptr: *const u8,
    path_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let path = read_absolute_path(path_ptr, path_len)?;
            stage_pending_save(session, &path)
        })
    }
}

/// The shell placed the staged bytes at the destination.
///
/// `handle` is the durable token a later plain Save rewrites (Android: a URI
/// held by `takePersistableUriPermission`; iOS: base64 bookmark data;
/// HarmonyOS: the picked file URI). `display_name` is what the destination
/// is actually called, which the picker may have adjusted.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. Both byte ranges
/// must cover readable UTF-8 bytes for their declared lengths.
#[no_mangle]
pub unsafe extern "C" fn op_editor_commit_save(
    engine: *mut crate::OpEngine,
    handle_ptr: *const u8,
    handle_len: usize,
    name_ptr: *const u8,
    name_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let handle = read_utf8(handle_ptr, handle_len, SAVE_TARGET_CAP, "save target")?;
            let display_name =
                read_utf8(name_ptr, name_len, SAVE_FILE_NAME_CAP, "save display name")?;
            commit_pending_save(session, handle, display_name)
        })
    }
}

/// The picker was dismissed, or the shell could not write the destination.
///
/// Either way the document stays dirty and keeps whatever binding it had, so
/// the next Save retries rather than silently believing itself saved.
/// `failed` distinguishes a real write failure (which raises a runtime-error
/// diagnostic) from an ordinary user cancellation (which does not).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread.
#[no_mangle]
pub unsafe extern "C" fn op_editor_cancel_save(
    engine: *mut crate::OpEngine,
    failed: bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if session.document_save.pending.take().is_none() {
                return Err(FfiError::new(OpStatus::NotReady, "no save is pending"));
            }
            if failed {
                session.emit_runtime_error(
                    2,
                    "the shell could not write the picked save destination",
                    "op-engine-ffi/save",
                );
            }
            Ok(())
        })
    }
}

fn pending(state: &crate::editor_document::DocumentSaveShellState) -> FfiResult<&PendingShellSave> {
    state
        .pending
        .as_ref()
        .ok_or_else(|| FfiError::new(OpStatus::NotReady, "no save is pending"))
}

pub(crate) fn stage_pending_save(session: &mut Session, path: &Path) -> FfiResult<()> {
    let expected = {
        let pending = pending(&session.document_save)?;
        if pending.staged.is_some() {
            return Err(FfiError::new(
                OpStatus::Busy,
                "the pending save has already been staged",
            ));
        }
        pending.file_name.clone()
    };
    // Same guard as the export bridge: the shell may choose the directory,
    // never the name, so a staged document cannot end up claiming to be a
    // different file than the one the picker was told about.
    if path.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
        return Err(FfiError::invalid(
            "save staging file name does not match the pending save",
        ));
    }
    if path.exists() {
        return Err(FfiError::invalid("save staging path already exists"));
    }
    crate::editor_document::write_current_document(session, path)?;
    if let Some(pending) = session.document_save.pending.as_mut() {
        pending.staged = Some(path.to_path_buf());
    }
    Ok(())
}

pub(crate) fn commit_pending_save(
    session: &mut Session,
    handle: String,
    display_name: String,
) -> FfiResult<()> {
    {
        let pending = pending(&session.document_save)?;
        if pending.staged.is_none() {
            return Err(FfiError::new(
                OpStatus::NotReady,
                "the pending save was never staged",
            ));
        }
    }
    if handle.trim().is_empty() {
        return Err(FfiError::invalid("save target handle is empty"));
    }
    let display_name = validated_display_name(&display_name)?;
    session.document_save.pending = None;
    session.document_save.resave_pending = false;
    session.document_save.binding =
        crate::editor_document::DocumentBinding::Shell(crate::editor_document::ShellBinding {
            handle,
            display_name: display_name.clone(),
        });
    if let Ok(host) = session.editor_mut() {
        let state = host.editor_state_mut();
        state.editor_ui.file_name_display = Some(display_name);
        state.mark_saved_revision();
        host.mark_editor_state_dirty();
    }
    session.request_redraw();
    Ok(())
}

/// The picker's own name for the destination, rejected when it could not be
/// a file name at all. Kept permissive otherwise: the platform already
/// created the file, so second-guessing its name would only desynchronize
/// the TopBar from what the user sees in their file manager.
fn validated_display_name(name: &str) -> FfiResult<String> {
    let trimmed = name.trim();
    let invalid = trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_control);
    if invalid {
        return Err(FfiError::invalid("save display name is not a file name"));
    }
    Ok(trimmed.to_owned())
}

unsafe fn read_absolute_path(path_ptr: *const u8, path_len: usize) -> FfiResult<PathBuf> {
    let value = unsafe { read_utf8(path_ptr, path_len, SAVE_PATH_CAP, "save staging path")? };
    if value.is_empty() {
        return Err(FfiError::invalid("save staging path is empty"));
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(FfiError::invalid("save staging path must be absolute"));
    }
    Ok(path)
}

/// Shared "size query, then copy" ABI used by both string getters.
unsafe fn copy_out(
    bytes: &[u8],
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> FfiResult<()> {
    if required.is_null() {
        return Err(FfiError::invalid("save required-length pointer is null"));
    }
    unsafe { required.write(bytes.len()) };
    if bytes.is_empty() {
        return Err(FfiError::invalid("save string is empty"));
    }
    if buffer.is_null() {
        if capacity == 0 {
            return Ok(());
        }
        return Err(FfiError::invalid(
            "save string buffer is null with nonzero capacity",
        ));
    }
    if capacity < bytes.len() {
        return Err(FfiError::invalid(format!(
            "save string buffer covers {capacity} bytes but {} are required",
            bytes.len()
        )));
    }
    if capacity > isize::MAX as usize {
        return Err(FfiError::invalid("save string buffer capacity overflows"));
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len()) };
    Ok(())
}
