//! Image-blob resolution — ports `figma-image-resolver.ts`. Walks the
//! converted document and replaces `__blob:N` / `__hash:HEX` image-
//! fill placeholders with `data:` URLs.

use base64::Engine;
use jian_ops_schema::document::PenDocument;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use std::collections::HashMap;

/// Encode raw image bytes as a `data:` URL, sniffing the MIME type
/// from the leading magic bytes (PNG fallback).
fn blob_to_data_url(bytes: &[u8]) -> String {
    let mime = match bytes {
        [0xFF, 0xD8, ..] => "image/jpeg",
        [0x47, 0x49, ..] => "image/gif",
        [0x52, 0x49, ..] => "image/webp",
        _ => "image/png",
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{b64}")
}

/// Resolve a `__blob:` / `__hash:` placeholder to a data URL.
fn resolve_ref(
    src: &str,
    data_urls: &HashMap<u32, String>,
    hash_urls: &HashMap<String, String>,
) -> Option<String> {
    if let Some(rest) = src.strip_prefix("__blob:") {
        let index: u32 = rest.parse().ok()?;
        return data_urls.get(&index).cloned();
    }
    if let Some(hash) = src.strip_prefix("__hash:") {
        return hash_urls.get(hash).cloned();
    }
    None
}

fn patch_fills(
    fills: &mut Option<Vec<PenFill>>,
    data_urls: &HashMap<u32, String>,
    hash_urls: &HashMap<String, String>,
) -> usize {
    let mut count = 0;
    if let Some(fills) = fills {
        for fill in fills {
            if let PenFill::Image(img) = fill {
                if img.url.starts_with("__blob:") || img.url.starts_with("__hash:") {
                    if let Some(url) = resolve_ref(&img.url, data_urls, hash_urls) {
                        img.url = url.into();
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// Patch one node's image fills, then recurse into its children.
fn patch_node(
    node: &mut PenNode,
    data_urls: &HashMap<u32, String>,
    hash_urls: &HashMap<String, String>,
) -> usize {
    let mut count = 0;
    let children: Option<&mut Vec<PenNode>> = match node {
        PenNode::Frame(f) => {
            count += patch_fills(&mut f.container.fill, data_urls, hash_urls);
            f.children.as_mut()
        }
        PenNode::Group(g) => {
            count += patch_fills(&mut g.container.fill, data_urls, hash_urls);
            g.children.as_mut()
        }
        PenNode::Rectangle(r) => {
            count += patch_fills(&mut r.container.fill, data_urls, hash_urls);
            r.children.as_mut()
        }
        PenNode::Ellipse(e) => {
            count += patch_fills(&mut e.fill, data_urls, hash_urls);
            None
        }
        PenNode::Polygon(p) => {
            count += patch_fills(&mut p.fill, data_urls, hash_urls);
            None
        }
        PenNode::Path(p) => {
            count += patch_fills(&mut p.fill, data_urls, hash_urls);
            None
        }
        PenNode::Text(t) => {
            count += patch_fills(&mut t.fill, data_urls, hash_urls);
            None
        }
        PenNode::Ref(r) => r.children.as_mut(),
        _ => None,
    };
    if let Some(children) = children {
        for child in children {
            count += patch_node(child, data_urls, hash_urls);
        }
    }
    count
}

/// Replace every image-fill placeholder in `doc` with a data URL.
/// Returns the number of replacements made.
pub fn resolve_image_blobs(
    doc: &mut PenDocument,
    image_blobs: &HashMap<u32, Vec<u8>>,
    image_files: &HashMap<String, Vec<u8>>,
) -> usize {
    if image_blobs.is_empty() && image_files.is_empty() {
        return 0;
    }
    let data_urls: HashMap<u32, String> = image_blobs
        .iter()
        .map(|(k, v)| (*k, blob_to_data_url(v)))
        .collect();
    let hash_urls: HashMap<String, String> = image_files
        .iter()
        .map(|(k, v)| (k.clone(), blob_to_data_url(v)))
        .collect();

    let mut count = 0;
    if let Some(pages) = &mut doc.pages {
        for page in pages {
            for child in &mut page.children {
                count += patch_node(child, &data_urls, &hash_urls);
            }
        }
    }
    for child in &mut doc.children {
        count += patch_node(child, &data_urls, &hash_urls);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_mime_is_default() {
        assert!(blob_to_data_url(&[0x89, 0x50, 0x4e, 0x47]).starts_with("data:image/png;base64,"));
    }

    #[test]
    fn jpeg_mime_detected() {
        assert!(blob_to_data_url(&[0xFF, 0xD8, 0xFF, 0xE0]).starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn resolve_blob_ref() {
        let mut data_urls = HashMap::new();
        data_urls.insert(3u32, "data:image/png;base64,AAA".to_string());
        assert_eq!(
            resolve_ref("__blob:3", &data_urls, &HashMap::new()).as_deref(),
            Some("data:image/png;base64,AAA")
        );
        assert!(resolve_ref("__blob:9", &data_urls, &HashMap::new()).is_none());
    }

    #[test]
    fn resolve_hash_ref() {
        let mut hash_urls = HashMap::new();
        hash_urls.insert("abcd".to_string(), "data:image/png;base64,ZZ".to_string());
        assert_eq!(
            resolve_ref("__hash:abcd", &HashMap::new(), &hash_urls).as_deref(),
            Some("data:image/png;base64,ZZ")
        );
    }
}
