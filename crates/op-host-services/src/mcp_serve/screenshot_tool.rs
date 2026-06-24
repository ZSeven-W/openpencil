//! MCP tool `get_screenshot` — renders a node (or the active page's
//! first top-level node when `nodeId` is `"root"`) to a base64 PNG.
//!
//! Part of the Pencil-style agentic tool-loop (Phase 0.3 — purely
//! additive). A design agent calls this after each generation step to
//! visually verify the result before proceeding to the next iteration.
//!
//! Layering note: this tool lives in `op-host-services` (not `op-mcp`)
//! because rendering requires skia, which is only available here. The
//! snapshot constructor takes `&EditorState` and derives the
//! layout-resolved scene via `op_pen_loader::editor_state_to_layout_scene`
//! at registration time, matching the pattern used by other read tools in
//! this crate (e.g. `debug_screenshot`).

use std::collections::BTreeMap;

use base64::Engine as _;
use op_editor_core::EditorState;
use op_editor_ui::layout_scene::LayoutScene;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

use crate::export::{render_node_raster_bytes, RasterFormat};

/// MCP tool struct — stores the layout-resolved scene (derived at
/// registration time) and the first top-level node id on the active page
/// for resolving the `"root"` alias at call time.
pub struct GetScreenshot {
    /// Layout-resolved scene snapshot — same derive the live canvas uses.
    scene: LayoutScene,
    /// Id of the first top-level node on the active page, used to resolve
    /// the `"root"` alias. `None` when the active page is empty.
    root_node_id: Option<String>,
}

impl McpTool for GetScreenshot {
    fn name(&self) -> &str {
        "get_screenshot"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        // Parse nodeId argument.
        let raw_id = match args.get("nodeId").map(String::as_str) {
            Some(id) if !id.trim().is_empty() => id.trim(),
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "nodeId is required (pass a node id or \"root\")".into(),
                )
            }
        };

        // Resolve "root" alias to the first top-level node on the active page.
        let node_id: &str = if raw_id == "root" {
            match &self.root_node_id {
                Some(id) => id.as_str(),
                None => {
                    return ToolOutcome::Err(
                        ToolErrorCode::ToolFailed,
                        "active page is empty — no root node to render".into(),
                    )
                }
            }
        } else {
            raw_id
        };

        // Render to PNG bytes via the shared raster pipeline.
        let bytes = match render_node_raster_bytes(&self.scene, node_id, RasterFormat::Png, 1.0) {
            Ok(b) => b,
            Err(e) => return ToolOutcome::Err(ToolErrorCode::ToolFailed, e),
        };

        // Base64-encode using the same engine used elsewhere in this crate
        // (see `export/screenshot.rs` and `export/tests.rs`).
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let out = serde_json::json!({
            "image_base64": image_base64,
            "format": "png",
        });
        ToolOutcome::OkJson(out.to_string())
    }
}

/// Snapshot constructor — derives the layout scene and resolves the root
/// node id from the live editor state at registration time.
pub fn get_screenshot_snapshot(state: &EditorState) -> GetScreenshot {
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let root_node_id = scene
        .active_page()
        .and_then(|page| page.children.first())
        .map(|node| node.id.clone());
    GetScreenshot {
        scene,
        root_node_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::test_support::{filled_rect, scene_with};
    use op_editor_ui::Color;

    const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    fn decode_base64(s: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .expect("valid base64")
    }

    /// Build a minimal `GetScreenshot` directly from a hand-crafted
    /// `LayoutScene` to avoid the `EditorState` round-trip in unit tests.
    fn tool_from_scene(scene: LayoutScene) -> GetScreenshot {
        let root_node_id = scene
            .active_page()
            .and_then(|p| p.children.first())
            .map(|n| n.id.clone());
        GetScreenshot {
            scene,
            root_node_id,
        }
    }

    fn call(tool: &GetScreenshot, node_id: &str) -> ToolOutcome {
        let mut args = BTreeMap::new();
        args.insert("nodeId".into(), node_id.into());
        tool.call(&args)
    }

    /// "root" resolves to the first top-level node; returned base64 decodes
    /// to a valid PNG (magic bytes check).
    #[test]
    fn root_alias_renders_first_top_level_node_as_png() {
        let scene = scene_with(vec![filled_rect(
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
        )]);
        let tool = tool_from_scene(scene);
        match call(&tool, "root") {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
                let b64 = v["image_base64"].as_str().expect("image_base64 field");
                assert!(!b64.is_empty(), "image_base64 must not be empty");
                let bytes = decode_base64(b64);
                assert_eq!(&bytes[..8], PNG_MAGIC, "must be a PNG payload");
                assert_eq!(v["format"], "png");
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    /// Explicit node id works the same way as "root".
    #[test]
    fn explicit_node_id_renders_that_node_as_png() {
        let scene = scene_with(vec![
            filled_rect("r1", 0.0, 0.0, 60.0, 60.0, Color::BLACK),
            filled_rect(
                "r2",
                200.0,
                200.0,
                40.0,
                40.0,
                Color {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                },
            ),
        ]);
        let tool = tool_from_scene(scene);
        match call(&tool, "r2") {
            ToolOutcome::OkJson(json) => {
                let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
                let bytes = decode_base64(v["image_base64"].as_str().expect("field"));
                assert_eq!(&bytes[..8], PNG_MAGIC, "must be a PNG payload");
            }
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    /// Unknown node id returns a tool-level error (not a JSON-RPC transport error).
    #[test]
    fn unknown_node_id_returns_tool_failed_error() {
        let scene = scene_with(vec![filled_rect("n1", 0.0, 0.0, 10.0, 10.0, Color::BLACK)]);
        let tool = tool_from_scene(scene);
        match call(&tool, "ghost") {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::ToolFailed);
                assert!(msg.contains("not found"), "{msg}");
            }
            other => panic!("expected Err, got {other:?}"),
        }
    }

    /// "root" on an empty page returns a clear error.
    #[test]
    fn root_on_empty_page_returns_tool_failed_error() {
        let scene = scene_with(vec![]);
        let tool = tool_from_scene(scene);
        match call(&tool, "root") {
            ToolOutcome::Err(code, msg) => {
                assert_eq!(code, ToolErrorCode::ToolFailed);
                assert!(msg.contains("empty"), "{msg}");
            }
            other => panic!("expected Err, got {other:?}"),
        }
    }

    /// Missing nodeId argument returns MissingArgument.
    #[test]
    fn missing_node_id_returns_missing_argument_error() {
        let scene = scene_with(vec![filled_rect("n1", 0.0, 0.0, 10.0, 10.0, Color::BLACK)]);
        let tool = tool_from_scene(scene);
        let args: BTreeMap<String, String> = BTreeMap::new();
        match tool.call(&args) {
            ToolOutcome::Err(code, _) => assert_eq!(code, ToolErrorCode::MissingArgument),
            other => panic!("expected MissingArgument, got {other:?}"),
        }
    }
}
