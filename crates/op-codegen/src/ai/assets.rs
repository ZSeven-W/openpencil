//! Embedded-image extraction. Port of
//! `apps/web/src/services/ai/codegen-assets.ts` (`extractCodegenAssets` +
//! `collectChunkAssetHints`). Walks the node JSON, pulls embedded image
//! data-URLs out into concrete asset files, and rewrites each `src` /
//! `fill.url` to a stable `./assets/<file>` path so prompts ship paths
//! instead of multi-KB base64 blobs.
//!
//! PARITY TODO: consumer-view enrichment (enrichNodeLocallyForAIConsumerView)
//! not yet ported — see structure-bundle/codegen-assets.

use base64::Engine;
use serde_json::Value;

use crate::ai::types::AssetFile;

const DATA_URL_PREFIX: &str = "data:";
const ASSETS_RELATIVE_PREFIX: &str = "./assets/";

/// Map a MIME type to a file extension, mirroring TS
/// `inferExtensionFromMimeType` (jpeg→jpg, svg+xml→svg, strip non-alnum,
/// default `bin`).
fn infer_extension_from_mime(mime: &str) -> String {
    let mapped = mime.split('/').nth(1).unwrap_or("bin").to_ascii_lowercase();
    if mapped == "jpeg" {
        return "jpg".to_string();
    }
    if mapped == "svg+xml" {
        return "svg".to_string();
    }
    let cleaned: String = mapped
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if cleaned.is_empty() {
        "bin".to_string()
    } else {
        cleaned
    }
}

/// Parse a `data:<mime>;base64,<payload>` URL into (mime, decoded bytes).
/// Returns `None` for non-data-URLs or anything not base64-encoded — same
/// shape as the TS `parseDataUrl` regex `^data:([^;,]+);base64,([\s\S]+)$`.
fn parse_data_url(url: &str) -> Option<(String, Vec<u8>)> {
    if !url.starts_with(DATA_URL_PREFIX) {
        return None;
    }
    let rest = &url[DATA_URL_PREFIX.len()..];
    // mime runs up to the first ';' or ',' (matches `[^;,]+`).
    let mime_end = rest.find([';', ','])?;
    let mime = &rest[..mime_end];
    if mime.is_empty() {
        return None;
    }
    let after_mime = &rest[mime_end..];
    let payload = after_mime.strip_prefix(";base64,")?;
    if payload.is_empty() {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()?;
    Some((mime.to_string(), bytes))
}

/// Mutable extraction state: dedup map (data-URL → relative path) + the
/// first-seen-ordered asset list + a stable id counter.
struct Extractor {
    assets: Vec<AssetFile>,
    seen: std::collections::HashMap<String, String>,
    next_index: usize,
}

impl Extractor {
    fn new() -> Self {
        Self {
            assets: Vec::new(),
            seen: std::collections::HashMap::new(),
            next_index: 1,
        }
    }

    /// Materialize an asset for `source_url` (or reuse the deduped one) and
    /// return the `./assets/<file>` relative path to substitute back in.
    fn materialize(&mut self, source_url: &str, owner_id: Option<&str>) -> Option<String> {
        if let Some(existing) = self.seen.get(source_url) {
            return Some(existing.clone());
        }
        let (mime, bytes) = parse_data_url(source_url)?;
        let ext = infer_extension_from_mime(&mime);
        let id = format!("image-{}", self.next_index);
        self.next_index += 1;
        let file_name = format!("{id}.{ext}");
        let relative_path = format!("{ASSETS_RELATIVE_PREFIX}{file_name}");
        let asset = AssetFile {
            id,
            relative_path: relative_path.clone(),
            zip_path: format!("assets/{file_name}"),
            mime_type: mime,
            bytes,
            source_node_id: owner_id.unwrap_or_default().to_string(),
        };
        self.assets.push(asset);
        self.seen
            .insert(source_url.to_string(), relative_path.clone());
        Some(relative_path)
    }

    /// Rewrite a single string field in place if it holds a data-URL.
    fn rewrite_field(&mut self, field: &mut Value, owner_id: Option<&str>) {
        if let Some(s) = field.as_str() {
            if s.starts_with(DATA_URL_PREFIX) {
                if let Some(path) = self.materialize(s, owner_id) {
                    *field = Value::String(path);
                }
            }
        }
    }

    /// Rewrite image-fill `url` data-URLs inside a `fill`/`fills` field. A
    /// fill matches when it's an object with `"type":"image"` and a string
    /// `"url"` starting with `data:` (mirrors the TS `fill.type === 'image'`
    /// gate). Both single-object and array shapes are handled.
    fn rewrite_fills(&mut self, fills: &mut Value, owner_id: Option<&str>) {
        match fills {
            Value::Array(items) => {
                for item in items.iter_mut() {
                    self.rewrite_image_fill(item, owner_id);
                }
            }
            Value::Object(_) => self.rewrite_image_fill(fills, owner_id),
            _ => {}
        }
    }

    fn rewrite_image_fill(&mut self, fill: &mut Value, owner_id: Option<&str>) {
        let is_image = fill.get("type").and_then(Value::as_str) == Some("image");
        if !is_image {
            return;
        }
        if let Some(url) = fill.get_mut("url") {
            self.rewrite_field(url, owner_id);
        }
    }

    /// Recurse over a node value: arrays map element-wise, objects detect
    /// image `src` / image fills then descend into `children`.
    fn visit(&mut self, value: &mut Value) {
        match value {
            Value::Array(items) => {
                for item in items.iter_mut() {
                    self.visit(item);
                }
            }
            Value::Object(map) => {
                let owner_id = map.get("id").and_then(Value::as_str).map(|s| s.to_string());
                let owner_ref = owner_id.as_deref();

                // (a) image node with a data-URL `src`.
                let is_image_node = map.get("type").and_then(Value::as_str) == Some("image");
                if is_image_node {
                    if let Some(src) = map.get_mut("src") {
                        self.rewrite_field(src, owner_ref);
                    }
                }

                // (b) image fills under `fill` / `fills`.
                if let Some(fill) = map.get_mut("fill") {
                    self.rewrite_fills(fill, owner_ref);
                }
                if let Some(fills) = map.get_mut("fills") {
                    self.rewrite_fills(fills, owner_ref);
                }

                // Recurse into children.
                if let Some(children) = map.get_mut("children") {
                    self.visit(children);
                }
            }
            _ => {}
        }
    }
}

/// Walk the node JSON, find embedded images (image nodes with a data-URL
/// `src`, and image fills with a data-URL `url`), replace each data-URL
/// with `./assets/<file>`, and return the sanitized JSON + the asset files.
/// Port of `extractCodegenAssets`.
///
/// Identical data-URLs collapse to a single `AssetFile`; assets are returned
/// in first-seen order. Invalid JSON (or non-data-URL `src`/`url` values)
/// passes through unchanged with an empty asset list.
pub fn extract_codegen_assets(nodes_json: &str) -> (String, Vec<AssetFile>) {
    let mut value: Value = match serde_json::from_str(nodes_json) {
        Ok(v) => v,
        Err(_) => return (nodes_json.to_string(), Vec::new()),
    };
    let mut extractor = Extractor::new();
    extractor.visit(&mut value);
    let sanitized = serde_json::to_string(&value).unwrap_or_else(|_| nodes_json.to_string());
    (sanitized, extractor.assets)
}

/// Given a chunk's node JSON + the full asset list, return human hints
/// (`"<relative_path> (<mime>)"`) for each asset whose `relative_path`
/// appears in the chunk JSON, de-duplicated and in asset order. Port of
/// `collectChunkAssetHints`.
pub fn collect_chunk_asset_hints(chunk_nodes_json: &str, assets: &[AssetFile]) -> Vec<String> {
    let mut hints = Vec::new();
    for asset in assets {
        if chunk_nodes_json.contains(&asset.relative_path) {
            hints.push(format!("{} ({})", asset.relative_path, asset.mime_type));
        }
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_PX_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    #[test]
    fn extracts_image_node_data_url_into_asset_file() {
        let nodes = format!("[{{\"type\":\"image\",\"id\":\"n1\",\"src\":\"{ONE_PX_PNG}\"}}]");
        let (sanitized, assets) = extract_codegen_assets(&nodes);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].mime_type, "image/png");
        assert!(assets[0].relative_path.starts_with("./assets/"));
        assert!(!assets[0].bytes.is_empty());
        assert!(!sanitized.contains("data:image"));
        assert!(sanitized.contains(&assets[0].relative_path));
    }

    #[test]
    fn dedupes_identical_data_urls() {
        let nodes = format!("[{{\"type\":\"image\",\"id\":\"a\",\"src\":\"{ONE_PX_PNG}\"}},{{\"type\":\"image\",\"id\":\"b\",\"src\":\"{ONE_PX_PNG}\"}}]");
        let (_s, assets) = extract_codegen_assets(&nodes);
        assert_eq!(assets.len(), 1);
    }

    #[test]
    fn no_assets_returns_input_unchanged_shape() {
        let nodes = "[{\"type\":\"frame\",\"children\":[]}]";
        let (sanitized, assets) = extract_codegen_assets(nodes);
        assert!(assets.is_empty());
        assert!(sanitized.contains("frame"));
    }

    #[test]
    fn extracts_image_fill_url_and_records_source_node() {
        let nodes = format!(
            "[{{\"type\":\"frame\",\"id\":\"f1\",\"fill\":[{{\"type\":\"image\",\"url\":\"{ONE_PX_PNG}\"}}]}}]"
        );
        let (sanitized, assets) = extract_codegen_assets(&nodes);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].source_node_id, "f1");
        assert!(!sanitized.contains("data:image"));
        assert!(sanitized.contains(&assets[0].relative_path));
    }

    #[test]
    fn recurses_into_children() {
        let nodes = format!(
            "[{{\"type\":\"frame\",\"id\":\"root\",\"children\":[{{\"type\":\"image\",\"id\":\"deep\",\"src\":\"{ONE_PX_PNG}\"}}]}}]"
        );
        let (_s, assets) = extract_codegen_assets(&nodes);
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].source_node_id, "deep");
    }

    #[test]
    fn collects_chunk_hints_for_referenced_assets() {
        let nodes = format!("[{{\"type\":\"image\",\"id\":\"n1\",\"src\":\"{ONE_PX_PNG}\"}}]");
        let (sanitized, assets) = extract_codegen_assets(&nodes);
        let hints = collect_chunk_asset_hints(&sanitized, &assets);
        assert_eq!(hints.len(), 1);
        assert!(hints[0].contains(&assets[0].relative_path));
        assert!(hints[0].contains("image/png"));
        // An unrelated chunk references no assets.
        let empty = collect_chunk_asset_hints("[{\"type\":\"frame\"}]", &assets);
        assert!(empty.is_empty());
    }
}
