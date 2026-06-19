//! Chat-attachment file picker — the rfd-backed host half.
//!
//! The headless attachment helpers (base64, temp-file spill, prompt
//! building, the Claude image-Read flow) live in
//! [`op_web_daemon::chat_attachment`]; this module keeps only the
//! native file picker that stages a chosen image on the chat state.

use std::fs;

use op_editor_core::chat::{ChatAttachment, MAX_ATTACHMENT_BYTES};
use op_host_native::WidgetHostNative;
use op_web_daemon::chat_attachment::media_type_for_path;

/// Drain `chat.pending_attachment_pick` (raised by the attach
/// button): open a native image picker and stage the chosen file on
/// `chat.pending_attachments`. Returns true when an attachment was
/// added (the caller redraws).
pub fn drain_attachment_pick(host: &mut WidgetHostNative) -> bool {
    if !std::mem::take(&mut host.editor_state_mut().chat.pending_attachment_pick) {
        return false;
    }
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "svg"])
        .pick_file()
    else {
        return false;
    };
    // Reject an oversized file before reading it fully into memory.
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() as usize > MAX_ATTACHMENT_BYTES {
            return false;
        }
    }
    let Ok(data) = fs::read(&path) else {
        return false;
    };
    if data.len() > MAX_ATTACHMENT_BYTES {
        return false;
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment")
        .to_string();
    let media_type = media_type_for_path(&path);
    // `add_attachment` enforces the per-turn attachment-count cap.
    host.editor_state_mut().chat.add_attachment(ChatAttachment {
        name,
        media_type,
        data,
    })
}
