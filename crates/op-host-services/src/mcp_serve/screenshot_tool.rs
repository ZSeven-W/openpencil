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
        // The browser loads the shipped simple-icons catalog from the daemon,
        // but this exact PNG is rendered server-side. Install the same catalog
        // before painting so brand icon_font nodes do not degrade to the
        // honest fallback dot during agent visual QA.
        let _ = op_editor_ui::set_brand_catalog(crate::web_static::ICONIFY_BRANDS_JSON);

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
            Err(e) => return ToolOutcome::Err(ToolErrorCode::ToolFailed, e.to_string()),
        };

        // Base64-encode using the same engine used elsewhere in this crate
        // (see `export/screenshot.rs` and `export/tests.rs`).
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let metadata = serde_json::json!({
            "nodeId": node_id,
            "format": "png",
            // include image_base64 in metadata for the in-app chat agent path —
            // the MCP path uses the `image` field on ToolResponse::Ok separately
            "image_base64": image_base64,
        });
        ToolOutcome::OkImageContent {
            image_base64,
            mime_type: "image/png".into(),
            metadata_json: Some(metadata.to_string()),
        }
    }
}

/// Snapshot constructor — derives the layout scene and resolves the root
/// node id from the live editor state at registration time.
pub fn get_screenshot_snapshot(state: &EditorState) -> GetScreenshot {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
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
    use op_editor_ui::layout_scene::{NodeKind, SceneNode};
    use op_editor_ui::{Color, Rect};

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
            ToolOutcome::OkImageContent {
                image_base64,
                mime_type,
                metadata_json,
            } => {
                assert_eq!(mime_type, "image/png");
                assert!(!image_base64.is_empty(), "image_base64 must not be empty");
                let bytes = decode_base64(&image_base64);
                assert_eq!(&bytes[..8], PNG_MAGIC, "must be a PNG payload");
                // Metadata carries nodeId + format + image_base64 (for chat agent path).
                let meta: serde_json::Value =
                    serde_json::from_str(&metadata_json.expect("metadata")).expect("valid JSON");
                assert_eq!(meta["format"], "png");
                assert_eq!(meta["nodeId"], "n1");
                assert!(!meta["image_base64"].as_str().unwrap_or("").is_empty());
            }
            other => panic!("expected OkImageContent, got {other:?}"),
        }
    }

    #[test]
    fn exact_screenshot_loads_the_shipped_brand_catalog_before_painting() {
        let mut icon = SceneNode::leaf("wechat", NodeKind::Other("icon_font".into()));
        icon.bounds = Rect::xywh(0.0, 0.0, 32.0, 32.0);
        icon.text = Some("wechat".into());
        icon.font_family = "simple-icons".into();
        icon.fill = Some(Color::BLACK);
        let tool = tool_from_scene(scene_with(vec![icon]));

        let ToolOutcome::OkImageContent { image_base64, .. } = call(&tool, "root") else {
            panic!("expected exact brand screenshot");
        };
        let bytes = decode_base64(&image_base64);
        let image = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(&bytes))
            .expect("decode screenshot PNG");
        let info = skia_safe::ImageInfo::new(
            (image.width(), image.height()),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Unpremul,
            None,
        );
        let stride = image.width() as usize * 4;
        let mut pixels = vec![0u8; stride * image.height() as usize];
        assert!(image.read_pixels(
            &info,
            pixels.as_mut_slice(),
            stride,
            (0, 0),
            skia_safe::image::CachingHint::Allow,
        ));
        let painted = pixels.chunks_exact(4).filter(|rgba| rgba[3] > 0).count();
        assert!(
            painted > 120,
            "expected the WeChat silhouette rather than the small fallback dot, got {painted} pixels"
        );
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
            ToolOutcome::OkImageContent {
                image_base64,
                mime_type,
                ..
            } => {
                assert_eq!(mime_type, "image/png");
                let bytes = decode_base64(&image_base64);
                assert_eq!(&bytes[..8], PNG_MAGIC, "must be a PNG payload");
            }
            other => panic!("expected OkImageContent, got {other:?}"),
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
