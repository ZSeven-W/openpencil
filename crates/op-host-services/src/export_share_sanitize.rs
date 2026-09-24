//! What the share page may carry out of this machine.
//!
//! The embedded `.op` is the author's document, so its design content —
//! every text, colour and image — travels as authored. What must not
//! travel is where it came from: a string whose whole value is a local
//! file reference (an image linked from disk, a relinked asset) names a
//! path, and usually the user's account with it. Such a reference is
//! inlined as a `data:` URL when it is a readable image, and emptied
//! otherwise. Remote URLs that carry a credential in their query are
//! emptied too. The recipe itself is sanitized separately (see
//! `op_editor_core::sanitize_share_text`), with the terms collected here.

use base64::Engine as _;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Largest local image inlined into a share page. Anything bigger is
/// dropped rather than turning one share page into a photo archive.
const MAX_INLINE_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

/// Terms the host knows are identifying: the OS account name(s) and the
/// home directory's last component. Every term is ≥ 3 chars (shorter
/// ones are ignored by the redactor anyway).
pub fn host_redact_terms() -> Vec<String> {
    let mut terms: Vec<String> = ["USER", "USERNAME", "LOGNAME"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .collect();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        if let Some(name) = Path::new(&home).file_name().and_then(|name| name.to_str()) {
            terms.push(name.to_string());
        }
    }
    terms.retain(|term| term.trim().chars().count() >= 3);
    terms.sort();
    terms.dedup();
    terms
}

/// Rewrite every local-file reference in a serialized document. Relative
/// references resolve against `document_dir` when there is one.
pub fn sanitize_document_value(value: &mut Value, document_dir: Option<&Path>) {
    match value {
        Value::String(text) => {
            if let Some(clean) = sanitize_reference(text, document_dir) {
                *text = clean;
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize_document_value(item, document_dir);
            }
        }
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                if let Value::String(text) = item {
                    if is_reference_key(key) && is_relative_reference(text) {
                        *text = inline_image(document_dir.map(|dir| dir.join(text.as_str())))
                            .unwrap_or_default();
                        continue;
                    }
                }
                sanitize_document_value(item, document_dir);
            }
        }
        _ => {}
    }
}

/// The replacement for one string, or `None` to keep it.
fn sanitize_reference(text: &str, document_dir: Option<&Path>) -> Option<String> {
    let trimmed = text.trim();
    if let Some(path) = local_path(trimmed) {
        let resolved = if path.is_absolute() {
            Some(path)
        } else {
            document_dir.map(|dir| dir.join(path))
        };
        return Some(inline_image(resolved).unwrap_or_default());
    }
    if is_credentialed_url(trimmed) {
        return Some(String::new());
    }
    None
}

/// A string whose WHOLE value is a local path, as a path.
fn local_path(text: &str) -> Option<PathBuf> {
    if text.contains('\n') || text.chars().count() > 4096 {
        return None;
    }
    if let Some(rest) = text.strip_prefix("file://") {
        return Some(PathBuf::from(rest));
    }
    if let Some(rest) = text.strip_prefix("~/") {
        let home = std::env::var_os("HOME").map(PathBuf::from)?;
        return Some(home.join(rest));
    }
    let bytes = text.as_bytes();
    let windows_drive = bytes.len() > 2
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    // `/x/y` — two segments at least, so a lone "/" or "1/2" is not one.
    let unix = text.starts_with('/') && !text.starts_with("//") && text.matches('/').count() >= 2;
    (windows_drive || unix || text.starts_with("\\\\")).then(|| PathBuf::from(text))
}

/// Keys whose values are asset references by schema (`ImageNode.src`,
/// image-fill `url`) — the only place a RELATIVE path is recognised,
/// because a relative-looking string anywhere else is ordinary text.
fn is_reference_key(key: &str) -> bool {
    matches!(key, "src" | "url")
}

fn is_relative_reference(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    !lower.is_empty()
        && !lower.contains("://")
        && !lower.starts_with("data:")
        && !lower.starts_with("op-image:")
        && !lower.starts_with('#')
        && has_image_extension(&lower)
}

fn is_credentialed_url(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return false;
    }
    let Some((_, query)) = lower.split_once('?') else {
        return false;
    };
    query.split('&').any(|pair| {
        let name = pair.split('=').next().unwrap_or_default();
        matches!(
            name,
            "key" | "api_key" | "apikey" | "token" | "access_token" | "sig" | "signature"
        ) || name.starts_with("x-amz-")
    })
}

fn has_image_extension(lower: &str) -> bool {
    mime_for(lower).is_some()
}

fn mime_for(lower_path: &str) -> Option<&'static str> {
    let ext = lower_path.rsplit('.').next()?;
    Some(match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        _ => return None,
    })
}

/// A readable local image as a `data:` URL.
fn inline_image(path: Option<PathBuf>) -> Option<String> {
    let path = path?;
    let lower = path.to_string_lossy().to_ascii_lowercase();
    let mime = mime_for(&lower)?;
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > MAX_INLINE_IMAGE_BYTES {
        return None;
    }
    let bytes = std::fs::read(&path).ok()?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn local_references_are_inlined_or_emptied_and_text_is_left_alone() {
        let dir = std::env::temp_dir().join(format!("op-share-sanitize-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let png = dir.join("dot.png");
        std::fs::write(&png, [0x89, b'P', b'N', b'G']).expect("png");
        let mut doc = json!({
            "children": [
                {"type": "image", "src": png.to_string_lossy()},
                {"type": "image", "src": "dot.png"},
                {"type": "image", "src": "/Users/nobody/missing.png"},
                {"type": "text", "content": "Save to /Users/demo/Desktop in step 2"},
                {"type": "rectangle", "fill": [{"type": "image", "url": "file:///tmp/none.jpg"}]},
                {"type": "image", "src": "https://cdn.example.com/a.png?sig=abc"},
                {"type": "image", "src": "https://cdn.example.com/b.png"}
            ]
        });
        sanitize_document_value(&mut doc, Some(&dir));
        let children = doc["children"].as_array().expect("children");
        assert!(children[0]["src"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert!(children[1]["src"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
        assert_eq!(children[2]["src"], "");
        // Design text is content, not a reference.
        assert_eq!(
            children[3]["content"],
            "Save to /Users/demo/Desktop in step 2"
        );
        assert_eq!(children[4]["fill"][0]["url"], "");
        assert_eq!(children[5]["src"], "");
        assert_eq!(children[6]["src"], "https://cdn.example.com/b.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_terms_are_long_enough_to_be_safe() {
        assert!(host_redact_terms()
            .iter()
            .all(|term| term.chars().count() >= 3));
    }
}
