//! Chat-attachment helpers — file picking, base64 encoding, and
//! temp-file spill for the CLI subprocess transports.
//!
//! The chat panel stages `ChatAttachment`s (raw bytes) on
//! `ChatState::pending_attachments`; this module bridges them onto
//! the wire each provider expects:
//!  - SDK providers (Claude) take inline base64 image blocks —
//!    [`attachment_to_base64`].
//!  - CLI subprocesses take file *paths*, so attachments spill to
//!    temp files that [`TempGuard`] removes once the turn ends.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use base64::Engine;
use op_ai::chat_provider::{ChatDelta, StopReason};
use op_editor_core::chat::{ChatAttachment, ThinkingMode, MAX_ATTACHMENT_BYTES};
use op_host_native::WidgetHostNative;

/// Per-process counter making each turn's temp directory unique.
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// Base64-encode an attachment's raw bytes (no `data:` URL prefix —
/// providers add their own wrapper).
pub fn attachment_to_base64(att: &ChatAttachment) -> String {
    base64::engine::general_purpose::STANDARD.encode(&att.data)
}

/// Best-effort MIME type from a file path's extension. Falls back to
/// `application/octet-stream` for an unknown / missing extension.
pub fn media_type_for_path(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Strip directory separators from a file name so a crafted
/// attachment name can't escape the temp directory.
fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c == '/' || c == '\\' { '_' } else { c })
        .collect();
    if cleaned.is_empty() {
        "attachment".to_string()
    } else {
        cleaned
    }
}

/// Owns the per-turn temp directory holding one turn's attachment
/// files, and removes the whole directory on drop so a CLI
/// subprocess's attachment paths don't leak past the turn. The
/// directory is unique per turn (not shared) and `0o700` on Unix.
pub struct TempGuard {
    dir: PathBuf,
    paths: Vec<PathBuf>,
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        // Removing the directory takes the attachment files with it.
        let _ = fs::remove_dir_all(&self.dir);
    }
}

impl TempGuard {
    /// The temp-file paths, in attachment order.
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

/// Spill each attachment to a file inside a fresh, private per-turn
/// temp directory. Returns a [`TempGuard`] that removes the directory
/// when dropped — hold it for the lifetime of the turn.
pub fn write_temp_attachments(attachments: &[ChatAttachment]) -> io::Result<TempGuard> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    // Unique directory per turn — no collisions between concurrent
    // turns, and no leftover files visible to other processes.
    let dir = std::env::temp_dir().join(format!("openpencil-chat-{stamp}-{seq}"));
    fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }
    // Build the guard up front so a write failure partway through
    // still drops it — `remove_dir_all` then cleans up the
    // already-written files.
    let mut guard = TempGuard {
        dir,
        paths: Vec::with_capacity(attachments.len()),
    };
    for (i, att) in attachments.iter().enumerate() {
        let name = sanitize_file_name(&att.name);
        let path = guard.dir.join(format!("{i}-{name}"));
        fs::write(&path, &att.data)?;
        guard.paths.push(path);
    }
    Ok(guard)
}

/// A one-line directive a text-only transport (CLI subprocess,
/// built-in agent) can prepend to convey the user's thinking-mode
/// choice. `Adaptive` returns `None` — the provider keeps its own
/// default behaviour.
pub fn thinking_directive(mode: ThinkingMode) -> Option<&'static str> {
    match mode {
        ThinkingMode::Enabled => Some("Think step by step and reason carefully before answering."),
        ThinkingMode::Disabled => {
            Some("Answer directly and concisely, without extended reasoning.")
        }
        ThinkingMode::Adaptive => None,
    }
}

/// Build the turn's prompt text, appending an `[attached …: <path>]`
/// line for each attachment so a path-based transport (CLI, Claude
/// Code's `query` String API) can read the files. Returns the prompt
/// and — when attachments were spilled — a [`TempGuard`] the caller
/// must hold for the lifetime of the turn.
pub fn prompt_with_attachments(
    user_message: &str,
    attachments: &[ChatAttachment],
) -> io::Result<(String, Option<TempGuard>)> {
    if attachments.is_empty() {
        return Ok((user_message.to_string(), None));
    }
    let guard = write_temp_attachments(attachments)?;
    let mut prompt = user_message.to_string();
    for (att, path) in attachments.iter().zip(guard.paths()) {
        let kind = if att.is_image() { "image" } else { "file" };
        prompt.push_str(&format!("\n\n[attached {kind}: {}]", path.display()));
    }
    Ok((prompt, Some(guard)))
}

/// Build a one-shot provider iterator that reports an attachment
/// staging failure and ends the turn. Providers call this when
/// `prompt_with_attachments` fails — surfacing the error beats
/// silently downgrading to a text-only prompt the user didn't ask
/// for.
pub fn attachment_error_turn(err: io::Error) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
    Box::new(
        vec![
            ChatDelta::Error(format!("failed to stage chat attachments: {err}")),
            ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            },
        ]
        .into_iter(),
    )
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn img(name: &str, bytes: &[u8]) -> ChatAttachment {
        ChatAttachment {
            name: name.to_string(),
            media_type: "image/png".to_string(),
            data: bytes.to_vec(),
        }
    }

    #[test]
    fn base64_round_trips() {
        let att = img("a.png", b"hello world");
        let encoded = attachment_to_base64(&att);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        assert_eq!(decoded, b"hello world");
    }

    #[test]
    fn media_type_maps_known_extensions() {
        assert_eq!(media_type_for_path(Path::new("x.png")), "image/png");
        assert_eq!(media_type_for_path(Path::new("x.JPG")), "image/jpeg");
        assert_eq!(media_type_for_path(Path::new("x.webp")), "image/webp");
        assert_eq!(
            media_type_for_path(Path::new("x.bin")),
            "application/octet-stream"
        );
    }

    #[test]
    fn sanitize_strips_separators() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), ".._.._etc_passwd");
        assert_eq!(sanitize_file_name(""), "attachment");
    }

    #[test]
    fn thinking_directive_table() {
        assert!(thinking_directive(ThinkingMode::Adaptive).is_none());
        assert!(thinking_directive(ThinkingMode::Enabled).is_some());
        assert!(thinking_directive(ThinkingMode::Disabled).is_some());
    }

    #[test]
    fn prompt_with_no_attachments_is_unchanged() {
        let (prompt, guard) = prompt_with_attachments("hello", &[]).unwrap();
        assert_eq!(prompt, "hello");
        assert!(guard.is_none());
    }

    #[test]
    fn prompt_with_attachments_appends_path_lines() {
        let atts = vec![img("ref.png", b"x")];
        let (prompt, guard) = prompt_with_attachments("design this", &atts).unwrap();
        assert!(prompt.starts_with("design this"));
        assert!(prompt.contains("[attached image:"));
        let guard = guard.expect("guard present when attachments staged");
        assert_eq!(guard.paths().len(), 1);
        // The appended path is the real temp file.
        assert!(guard.paths()[0].exists());
    }

    #[test]
    fn temp_files_written_then_removed_on_drop() {
        let atts = vec![img("one.png", b"1"), img("two.png", b"2")];
        let kept: Vec<PathBuf>;
        {
            let guard = write_temp_attachments(&atts).unwrap();
            assert_eq!(guard.paths().len(), 2);
            for p in guard.paths() {
                assert!(p.exists());
            }
            kept = guard.paths().to_vec();
        }
        // Guard dropped — every temp file is gone.
        for p in &kept {
            assert!(!p.exists());
        }
    }
}
