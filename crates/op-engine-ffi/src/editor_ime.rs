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

/// Paste `text` into whichever text input currently owns the keyboard —
/// the mobile shells call this from their long-press edit menus with the
/// platform clipboard's contents. Routing mirrors the desktop Cmd+V text
/// arm (`op-host-desktop`'s `handle_paste_payload`): non-chat inputs
/// first (settings / git / rename / canvas text edit, with each field's
/// own filtering), then the chat input. Without a focused input this is
/// a no-op — node paste stays on the `KEY_PASTE` path.
///
/// # Safety
///
/// `engine` must be live and `text` must cover `text_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_paste_text(
    engine: *mut crate::OpEngine,
    text_ptr: *const u8,
    text_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            let text = crate::error::read_utf8(text_ptr, text_len, STRING_CAP, "paste text")?;
            if text.is_empty() {
                return Ok(());
            }
            let changed = session.with_collab_local_edit(|host| {
                if host.non_chat_input_owns_keyboard_pub() {
                    host.apply_input_paste(&text)
                } else if host.chat_input_owns_keyboard_pub() {
                    host.chat_input_paste(&text)
                } else {
                    false
                }
            })?;
            if changed {
                session.request_redraw();
            }
            Ok(())
        })
    }
}

/// Drain the engine's pending copy-to-clipboard text — the OUTBOUND half
/// of the clipboard bridge. Engine copy actions (collab invite / share
/// address, MCP client config, chat and codegen copy buttons, Cmd+C
/// selections) queue one string into `chat.pending_copy_text`; the desktop
/// runner drains it into the OS clipboard, and the mobile shells poll this
/// after each frame and write the system pasteboard.
///
/// Two-phase contract like `op_editor_copy_login_url`: a NULL/0 probe
/// reports the required length WITHOUT consuming; a complete copy consumes.
/// `NotReady` (with `required = 0`) means no copy is pending — the common
/// per-frame case.
///
/// # Safety
///
/// `engine` must be live and called on its owner thread; `required` must be
/// writable and a non-null `buffer` must cover `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn op_editor_take_copy_text(
    engine: *mut crate::OpEngine,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            if required.is_null() {
                return Err(FfiError::invalid("copy-text required pointer is null"));
            }
            let host = session.editor_mut()?;
            let chat = &mut host.editor_state_mut().chat;
            let Some(text) = chat.pending_copy_text.as_deref() else {
                required.write(0);
                return Err(FfiError::new(OpStatus::NotReady, "no copy text is pending"));
            };
            let len = text.len();
            required.write(len);
            if len > STRING_CAP {
                return Err(FfiError::invalid(format!(
                    "copy text length exceeds {STRING_CAP} bytes"
                )));
            }
            if buffer.is_null() {
                if capacity == 0 {
                    return Ok(());
                }
                return Err(FfiError::invalid(
                    "copy-text buffer is null with nonzero capacity",
                ));
            }
            if capacity < len {
                return Err(FfiError::invalid(format!(
                    "copy-text buffer covers {capacity} bytes but {len} are required"
                )));
            }
            // The full payload fits — copy and consume atomically.
            let text = chat.pending_copy_text.take().expect("checked above");
            std::ptr::copy_nonoverlapping(text.as_ptr(), buffer, len);
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
