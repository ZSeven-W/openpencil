//! MCP tool `export_nodes` — renders one or more nodes to base64-encoded
//! image bytes (PNG / JPEG / WEBP) or PDF bytes, returning one entry per
//! requested node.
//!
//! Input: `{ "nodeIds": [String], "format": "png"|"jpeg"|"webp"|"pdf",
//!           "scale"?: number }`
//!
//! Output: `{ "files": [ { "nodeId": String, "bytes_base64": String,
//!                          "format": String } ] }`
//!
//! Unknown node ids are skipped with a note in the response (consistent
//! with allowing partial success across a batch). This is documented in
//! the per-id `error` field instead of failing the whole call — callers
//! can inspect entries where `bytes_base64` is absent and `error` is set.
//!
//! Raster (png/jpeg/webp): delegates to `render_node_raster_bytes` (the
//! same shared render core that `get_screenshot` uses).
//! PDF: delegates to `render_nodes_pdf_bytes` (node-scoped, one page per
//! requested node, cropped to that node's painted bounds).

use std::collections::BTreeMap;

use base64::Engine as _;
use op_editor_core::EditorState;
use op_editor_ui::layout_scene::LayoutScene;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

use crate::export::{render_node_raster_bytes, RasterFormat};
use crate::export_pdf::render_nodes_pdf_bytes;

/// MCP tool struct — stores the layout-resolved scene snapshot derived
/// at registration time from the live `EditorState`.
pub struct ExportNodes {
    /// Layout-resolved scene snapshot.
    scene: LayoutScene,
}

impl McpTool for ExportNodes {
    fn name(&self) -> &str {
        "export_nodes"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        // --- parse nodeIds (required, JSON array encoded as flat string) ---
        // The MCP wire layer delivers structured array values as a JSON
        // string in the flat BTreeMap; callers pass either a JSON-encoded
        // array string or a comma-separated fallback. We accept both.
        let node_ids_raw = match args.get("nodeIds") {
            Some(v) if !v.trim().is_empty() => v.trim(),
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "nodeIds is required (JSON array of node id strings)".into(),
                )
            }
        };
        let node_ids: Vec<String> = parse_string_array(node_ids_raw);
        if node_ids.is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "nodeIds must be a non-empty array".into(),
            );
        }

        // --- parse format (required) ---
        let format_str = match args.get("format").map(|s| s.trim()) {
            Some(f) if !f.is_empty() => f,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "format is required (png|jpeg|webp|pdf)".into(),
                )
            }
        };

        // --- parse optional scale (default 1.0) ---
        let scale: f32 = args
            .get("scale")
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(1.0);

        // --- render ---
        match format_str {
            "pdf" => render_pdf(&self.scene, &node_ids, scale),
            fmt => {
                let raster_fmt = match fmt {
                    "png" => RasterFormat::Png,
                    "jpeg" | "jpg" => RasterFormat::Jpeg,
                    "webp" => RasterFormat::Webp,
                    other => {
                        return ToolOutcome::Err(
                            ToolErrorCode::InvalidArgument,
                            format!(
                                "unknown format {other:?}: must be one of png, jpeg, webp, pdf"
                            ),
                        )
                    }
                };
                render_raster(&self.scene, &node_ids, raster_fmt, scale)
            }
        }
    }
}

/// Snapshot constructor — derives the layout scene from the live editor
/// state at registration time (same pattern as `get_screenshot_snapshot`).
pub fn export_nodes_snapshot(state: &EditorState) -> ExportNodes {
    ExportNodes {
        scene: op_pen_loader::editor_state_to_layout_scene(state),
    }
}

/// Render each node as a raster image and assemble the `files` array.
/// Unknown node ids and nodes that paint nothing are skipped; each
/// skipped entry records an `error` field instead of `bytes_base64`.
fn render_raster(
    scene: &LayoutScene,
    node_ids: &[String],
    format: RasterFormat,
    scale: f32,
) -> ToolOutcome {
    let format_label = match format {
        RasterFormat::Png => "png",
        RasterFormat::Jpeg => "jpeg",
        RasterFormat::Webp => "webp",
    };
    let mut files: Vec<serde_json::Value> = Vec::new();
    for id in node_ids {
        match render_node_raster_bytes(scene, id, format, scale) {
            Ok(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                files.push(serde_json::json!({
                    "nodeId": id,
                    "bytes_base64": b64,
                    "format": format_label,
                }));
            }
            Err(e) => {
                // Skip with a note — partial success is more useful for
                // multi-node batches than an all-or-nothing failure.
                files.push(serde_json::json!({
                    "nodeId": id,
                    "error": e,
                    "format": format_label,
                }));
            }
        }
    }
    ToolOutcome::OkJson(serde_json::json!({ "files": files }).to_string())
}

/// Render each requested node as one PDF page and return a single base64
/// blob. Because a PDF is a multi-page document, all requested nodes
/// (that are found and paintable) land in one PDF file; each `files`
/// entry shares that same `bytes_base64` value but carries its own
/// `nodeId`.  Nodes that are unknown or paint nothing are reported with
/// an `error` field and no bytes.
fn render_pdf(scene: &LayoutScene, node_ids: &[String], scale: f32) -> ToolOutcome {
    // Separate valid vs invalid ids up front so we can report per-id errors.
    let page_opt = scene.active_page();

    // Build per-id result: if a node is unknown we record an error entry.
    // The PDF renderer skips unknowns internally, so we reconstruct the
    // correspondence by checking page.find() ourselves first.
    let page = match page_opt {
        Some(p) => p,
        None => {
            // No active page at all — every id fails with the same error.
            let files: Vec<serde_json::Value> = node_ids
                .iter()
                .map(|id| {
                    serde_json::json!({
                        "nodeId": id,
                        "error": "no active page",
                        "format": "pdf",
                    })
                })
                .collect();
            return ToolOutcome::OkJson(serde_json::json!({ "files": files }).to_string());
        }
    };

    let mut valid_ids: Vec<&str> = Vec::new();
    let mut id_errors: BTreeMap<&str, String> = BTreeMap::new();
    for id in node_ids {
        if page.find(id).is_some() {
            valid_ids.push(id.as_str());
        } else {
            id_errors.insert(
                id.as_str(),
                format!("node {id} not found on the active page"),
            );
        }
    }

    // Call the shared PDF renderer with valid ids only.
    let pdf_result = if valid_ids.is_empty() {
        Err("no valid node ids to render".into())
    } else {
        render_nodes_pdf_bytes(scene, &valid_ids, scale)
    };

    match pdf_result {
        Ok(bytes) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            let files: Vec<serde_json::Value> = node_ids
                .iter()
                .map(|id| {
                    if let Some(e) = id_errors.get(id.as_str()) {
                        serde_json::json!({
                            "nodeId": id,
                            "error": e,
                            "format": "pdf",
                        })
                    } else {
                        serde_json::json!({
                            "nodeId": id,
                            "bytes_base64": b64,
                            "format": "pdf",
                        })
                    }
                })
                .collect();
            ToolOutcome::OkJson(serde_json::json!({ "files": files }).to_string())
        }
        Err(e) => {
            // Rendering failed for all valid ids — surface as tool error.
            ToolOutcome::Err(ToolErrorCode::ToolFailed, e)
        }
    }
}

/// Parse a JSON array string like `["a","b"]` into `Vec<String>`.
/// Falls back to splitting on commas when the value is not a JSON array
/// (defensive: some older MCP wires pass comma-separated plain strings).
fn parse_string_array(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    if trimmed.starts_with('[') {
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str(trimmed) {
            return arr
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }
    // Comma-separated fallback.
    trimmed
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::test_support::{filled_rect, scene_with};
    use op_editor_ui::Color;

    const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8];
    const PDF_MAGIC: &[u8] = b"%PDF-";

    fn decode_base64(s: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .expect("valid base64")
    }

    /// Build a minimal `ExportNodes` tool directly from a `LayoutScene` to
    /// avoid the `EditorState` round-trip in unit tests.
    fn tool_from_scene(scene: op_editor_ui::layout_scene::LayoutScene) -> ExportNodes {
        ExportNodes { scene }
    }

    fn call_tool(tool: &ExportNodes, node_ids_json: &str, format: &str) -> ToolOutcome {
        let mut args = BTreeMap::new();
        args.insert("nodeIds".into(), node_ids_json.into());
        args.insert("format".into(), format.into());
        tool.call(&args)
    }

    fn call_tool_with_scale(
        tool: &ExportNodes,
        node_ids_json: &str,
        format: &str,
        scale: f32,
    ) -> ToolOutcome {
        let mut args = BTreeMap::new();
        args.insert("nodeIds".into(), node_ids_json.into());
        args.insert("format".into(), format.into());
        args.insert("scale".into(), scale.to_string());
        tool.call(&args)
    }

    /// Helper: make a 2-node scene with two distinct filled rects.
    fn two_node_scene() -> op_editor_ui::layout_scene::LayoutScene {
        scene_with(vec![
            filled_rect(
                "n1",
                0.0,
                0.0,
                80.0,
                40.0,
                Color {
                    r: 0.2,
                    g: 0.4,
                    b: 0.8,
                    a: 1.0,
                },
            ),
            filled_rect(
                "n2",
                100.0,
                0.0,
                60.0,
                60.0,
                Color {
                    r: 0.9,
                    g: 0.1,
                    b: 0.2,
                    a: 1.0,
                },
            ),
        ])
    }

    /// PNG: both nodes render and decode to valid PNG magic.
    #[test]
    fn export_two_nodes_png_returns_files_with_png_magic() {
        let tool = tool_from_scene(two_node_scene());
        match call_tool(&tool, r#"["n1","n2"]"#, "png") {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
                let files = v["files"].as_array().expect("files array");
                assert_eq!(files.len(), 2, "expected 2 file entries");
                for f in files {
                    let b64 = f["bytes_base64"].as_str().expect("bytes_base64 field");
                    let bytes = decode_base64(b64);
                    assert_eq!(
                        &bytes[..8],
                        PNG_MAGIC,
                        "expected PNG magic in node {} export",
                        f["nodeId"]
                    );
                    assert_eq!(f["format"], "png");
                }
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    /// JPEG: both nodes render and decode to valid JPEG magic.
    #[test]
    fn export_two_nodes_jpeg_returns_files_with_jpeg_magic() {
        let tool = tool_from_scene(two_node_scene());
        match call_tool(&tool, r#"["n1","n2"]"#, "jpeg") {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
                let files = v["files"].as_array().expect("files array");
                assert_eq!(files.len(), 2, "expected 2 file entries");
                for f in files {
                    let b64 = f["bytes_base64"].as_str().expect("bytes_base64 field");
                    let bytes = decode_base64(b64);
                    assert_eq!(
                        &bytes[..2],
                        JPEG_MAGIC,
                        "expected JPEG magic in node {} export",
                        f["nodeId"]
                    );
                    assert_eq!(f["format"], "jpeg");
                }
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    /// PDF: both nodes produce a shared PDF blob that starts with %PDF-.
    #[test]
    fn export_two_nodes_pdf_returns_files_with_pdf_magic() {
        let tool = tool_from_scene(two_node_scene());
        match call_tool(&tool, r#"["n1","n2"]"#, "pdf") {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
                let files = v["files"].as_array().expect("files array");
                assert_eq!(files.len(), 2, "expected 2 file entries");
                for f in files {
                    let b64 = f["bytes_base64"].as_str().expect("bytes_base64 field");
                    let bytes = decode_base64(b64);
                    assert_eq!(
                        &bytes[..5],
                        PDF_MAGIC,
                        "expected %PDF- magic in node {} export",
                        f["nodeId"]
                    );
                    assert_eq!(f["format"], "pdf");
                }
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    /// Unknown node id is skipped with an error note; the valid node renders.
    #[test]
    fn unknown_node_id_is_skipped_with_error_note() {
        let tool = tool_from_scene(two_node_scene());
        match call_tool(&tool, r#"["n1","ghost"]"#, "png") {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
                let files = v["files"].as_array().expect("files array");
                assert_eq!(
                    files.len(),
                    2,
                    "one entry per requested id (including skipped)"
                );
                // n1 should have bytes_base64; ghost should have error.
                let n1 = files
                    .iter()
                    .find(|f| f["nodeId"] == "n1")
                    .expect("n1 entry");
                let ghost = files
                    .iter()
                    .find(|f| f["nodeId"] == "ghost")
                    .expect("ghost entry");
                assert!(
                    n1.get("bytes_base64").is_some(),
                    "n1 must have bytes_base64"
                );
                assert!(
                    ghost.get("error").is_some(),
                    "ghost must have an error field"
                );
                assert!(
                    ghost.get("bytes_base64").is_none(),
                    "ghost must not have bytes_base64"
                );
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    /// Missing nodeIds argument returns MissingArgument.
    #[test]
    fn missing_node_ids_returns_missing_argument_error() {
        let tool = tool_from_scene(two_node_scene());
        let args: BTreeMap<String, String> = {
            let mut m = BTreeMap::new();
            m.insert("format".into(), "png".into());
            m
        };
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
            other => panic!("expected MissingArgument, got {other:?}"),
        }
    }

    /// Missing format argument returns MissingArgument.
    #[test]
    fn missing_format_returns_missing_argument_error() {
        let tool = tool_from_scene(two_node_scene());
        let args: BTreeMap<String, String> = {
            let mut m = BTreeMap::new();
            m.insert("nodeIds".into(), r#"["n1"]"#.into());
            m
        };
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
            other => panic!("expected MissingArgument, got {other:?}"),
        }
    }

    /// Unknown format returns InvalidArgument.
    #[test]
    fn unknown_format_returns_invalid_argument_error() {
        let tool = tool_from_scene(two_node_scene());
        match call_tool(&tool, r#"["n1"]"#, "tiff") {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::InvalidArgument);
                assert!(msg.contains("tiff"), "{msg}");
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    /// Scale parameter is accepted and passed through.
    #[test]
    fn scale_parameter_is_accepted() {
        let tool = tool_from_scene(two_node_scene());
        // scale=2.0 should succeed (produces a larger image, but still valid PNG).
        match call_tool_with_scale(&tool, r#"["n1"]"#, "png", 2.0) {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
                let files = v["files"].as_array().expect("files array");
                let bytes = decode_base64(
                    files[0]["bytes_base64"]
                        .as_str()
                        .expect("bytes_base64 field"),
                );
                assert_eq!(&bytes[..8], PNG_MAGIC, "must be PNG at 2x scale");
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }
}
