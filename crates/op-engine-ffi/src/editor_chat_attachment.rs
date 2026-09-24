//! Chat / Studio Home image attachments picked by the mobile shell.
//!
//! The Home 添加截图 (Add screenshot) button and the chat panel's attach
//! button only raise `chat.pending_attachment_pick`; desktop answers it
//! with an rfd picker (`op-host-desktop/src/chat_attachment.rs`), the web
//! host with a DOM file input. On mobile the platform owns the photo /
//! document picker, so the contract is split across the C ABI:
//!
//! 1. **Request** — [`op_editor_take_shell_action`](crate::op_editor_take_shell_action)
//!    returns [`SHELL_ACTION_PICK_CHAT_ATTACHMENT`] once per press. Draining
//!    consumes the request, so a cancelled picker is a silent no-op and
//!    never reopens on a later poll.
//! 2. **Result** — the shell hands the chosen image back through
//!    [`op_editor_attach_chat_image`]. The bytes are staged on
//!    `chat.pending_attachments` exactly as the desktop picker stages them
//!    (same `ChatAttachment`, same 5 MiB per-file and 4-per-turn caps), so
//!    the Home thumbnail strip, the chat input strip and the next send all
//!    see it with no mobile-specific path.
//!
//! The shell may also call [`op_editor_attach_chat_image`] without a
//! request (a paste or share-sheet import); the staging rules are the same.

use op_editor_core::chat::{ChatAttachment, MAX_ATTACHMENTS, MAX_ATTACHMENT_BYTES};

use crate::error::{FfiError, FfiResult};
use crate::lifecycle::{call_session, Session};
use crate::OpStatus;

/// Append-only shell action code: present the platform image picker for
/// a chat / Home attachment, then call [`op_editor_attach_chat_image`].
pub const SHELL_ACTION_PICK_CHAT_ATTACHMENT: i32 = 13;

const MEDIA_TYPE_CAP: usize = 128;
const FILE_NAME_CAP: usize = 4 * 1024;
const NAME_CHAR_CAP: usize = 120;

/// Consume a raised attachment request, if any. Called from the shell
/// action drain.
pub(crate) fn drain_attachment_pick(session: &mut Session) -> FfiResult<Option<i32>> {
    let host = session.editor_mut()?;
    if !std::mem::take(&mut host.editor_state_mut().chat.pending_attachment_pick) {
        return Ok(None);
    }
    host.mark_editor_state_dirty();
    Ok(Some(SHELL_ACTION_PICK_CHAT_ATTACHMENT))
}

/// Stage one image the platform picker returned as a chat attachment.
///
/// - `data` — the image bytes, 1 byte to 5 MiB. PNG, JPEG, GIF and WebP
///   are recognised by their magic bytes; SVG is accepted when declared.
/// - `media_type` — optional (`null` / `0` to let the engine sniff). A
///   declared type must be `image/*` and must not contradict the sniffed
///   raster type.
/// - `file_name` — optional display name (the picker URL's last path
///   component); path separators and control characters are rejected.
///
/// Returns `Ok` when staged, `InvalidArg` for an empty / oversized /
/// unrecognised image or a bad name, and `Busy` when the turn already
/// holds the maximum number of attachments (nothing is staged).
///
/// # Safety
///
/// `engine` must be live and called on its owner thread. Non-empty byte
/// ranges must cover readable memory for their declared lengths; nothing
/// is retained past the call.
#[no_mangle]
pub unsafe extern "C" fn op_editor_attach_chat_image(
    engine: *mut crate::OpEngine,
    data_ptr: *const u8,
    data_len: usize,
    media_type_ptr: *const u8,
    media_type_len: usize,
    file_name_ptr: *const u8,
    file_name_len: usize,
) -> OpStatus {
    unsafe {
        call_session(engine, |session| {
            validate_bytes(data_ptr, data_len)?;
            // SAFETY: validation above rejects null, overflowing and
            // over-cap ranges; the C contract keeps the borrow live for
            // this call and the bytes are copied before it returns.
            let bytes = std::slice::from_raw_parts(data_ptr, data_len);
            let declared = crate::error::read_utf8(
                media_type_ptr,
                media_type_len,
                MEDIA_TYPE_CAP,
                "attachment media type",
            )?;
            let file_name = crate::error::read_utf8(
                file_name_ptr,
                file_name_len,
                FILE_NAME_CAP,
                "attachment file name",
            )?;
            let media_type = resolve_media_type(bytes, declared.trim())?;
            let name = attachment_name(&file_name, &media_type)?;
            stage(session, name, media_type, bytes.to_vec())
        })
    }
}

fn stage(session: &mut Session, name: String, media_type: String, data: Vec<u8>) -> FfiResult<()> {
    let host = session.editor_mut()?;
    let chat = &mut host.editor_state_mut().chat;
    if chat.pending_attachments.len() >= MAX_ATTACHMENTS {
        return Err(FfiError::new(
            OpStatus::Busy,
            format!("a turn holds at most {MAX_ATTACHMENTS} attachments"),
        ));
    }
    if !chat.add_attachment(ChatAttachment {
        name,
        media_type,
        data,
    }) {
        return Err(FfiError::invalid("attachment was not accepted"));
    }
    host.mark_editor_state_dirty();
    session.request_redraw();
    Ok(())
}

fn validate_bytes(pointer: *const u8, length: usize) -> FfiResult<()> {
    if length == 0 {
        return Err(FfiError::invalid("attachment is empty"));
    }
    if length > MAX_ATTACHMENT_BYTES {
        return Err(FfiError::invalid(format!(
            "attachment exceeds {MAX_ATTACHMENT_BYTES} bytes"
        )));
    }
    if pointer.is_null() {
        return Err(FfiError::invalid(
            "attachment pointer is null with nonzero length",
        ));
    }
    Ok(())
}

/// The sniffed raster type wins; a declared type fills in only what
/// sniffing cannot see (SVG) and may never contradict it.
fn resolve_media_type(bytes: &[u8], declared: &str) -> FfiResult<String> {
    let declared = declared.to_ascii_lowercase();
    if !declared.is_empty() && !declared.starts_with("image/") {
        return Err(FfiError::invalid("attachment media type is not an image"));
    }
    if let Some(sniffed) = op_image_enrich::net::providers::sniff_image_mime(bytes) {
        let sniffed = normalise(sniffed);
        if !declared.is_empty() && normalise(&declared) != sniffed {
            return Err(FfiError::invalid(
                "attachment media type contradicts its bytes",
            ));
        }
        return Ok(sniffed);
    }
    if declared == "image/svg+xml" && std::str::from_utf8(bytes).is_ok() {
        return Ok(declared);
    }
    Err(FfiError::invalid(
        "attachment is not PNG, JPEG, GIF, WebP, or SVG",
    ))
}

fn normalise(media_type: &str) -> String {
    match media_type.to_ascii_lowercase().as_str() {
        "image/jpg" => "image/jpeg".to_string(),
        other => other.to_string(),
    }
}

fn attachment_name(file_name: &str, media_type: &str) -> FfiResult<String> {
    if file_name.chars().any(char::is_control)
        || file_name.contains('/')
        || file_name.contains('\\')
    {
        return Err(FfiError::invalid("attachment file name is invalid"));
    }
    let name: String = file_name.trim().chars().take(NAME_CHAR_CAP).collect();
    if !name.is_empty() {
        return Ok(name);
    }
    let extension = match media_type {
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        _ => "png",
    };
    Ok(format!("attachment.{extension}"))
}

#[cfg(test)]
#[path = "editor_chat_attachment_tests.rs"]
mod tests;
