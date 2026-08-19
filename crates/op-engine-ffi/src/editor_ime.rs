//! IME composition FFI: preedit updates, commit, and focus queries.
//!
//! Coordinates and text follow the same contract as `editor.rs`: UTF-8
//! input, logical viewport units elsewhere.

use crate::error::{FfiError, STRING_CAP};
use crate::lifecycle::call_session;
use crate::OpStatus;

/// IME preedit (composition) into the host's focused input. `sel_start` /
/// `sel_end` are byte offsets within the preedit text.
///
/// # Safety
///
/// `engine` must be live and `text` must cover `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_ime_preedit(
    engine: *mut crate::OpEngine,
    text_ptr: *const u8,
    text_len: usize,
    sel_start: usize,
    sel_end: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let text = crate::error::read_utf8(text_ptr, text_len, STRING_CAP, "preedit text")?;
            // The first composition update can replace an active canvas-text
            // selection and synchronise that deletion into the document.
            // Treat every update as an owned transaction; UI-only updates
            // naturally finish as NoChange.
            if session.with_collab_local_edit(|host| {
                host.apply_ime_preedit(&text, Some((sel_start, sel_end)))
            })? {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// IME commit — the composition text lands in the focused input.
///
/// # Safety
///
/// `engine` must be live and `text` must cover `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_ime_commit(
    engine: *mut crate::OpEngine,
    text_ptr: *const u8,
    text_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let text = crate::error::read_utf8(text_ptr, text_len, STRING_CAP, "ime text")?;
            if session.with_collab_local_edit(|host| host.apply_ime_commit(&text))? {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Whether a canvas text edit (or panel input) currently holds the IME —
/// the shells show/hide the system keyboard accordingly.
///
/// # Safety
///
/// `engine` must be live and `out` must be writable.
#[no_mangle]
pub unsafe extern "C" fn op_editor_ime_focused(
    engine: *mut crate::OpEngine,
    out: *mut bool,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if out.is_null() {
                return Err(FfiError::invalid("focus output pointer is null"));
            }
            let focused = session
                .editor()
                .map(|host| host.text_input_focus_active())
                .unwrap_or(false);
            out.write(focused);
            Ok(())
        })
    }
}
