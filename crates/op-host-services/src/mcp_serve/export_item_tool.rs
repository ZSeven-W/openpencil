use std::collections::BTreeMap;

use base64::Engine as _;
use op_editor_core::EditorState;
use op_editor_ui::layout_scene::LayoutScene;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

use crate::export::{render_node_on_page_raster_bytes, render_page_raster_bytes, RasterFormat};
use crate::export_pdf::{render_node_on_page_pdf_bytes, render_page_pdf_bytes};

pub struct ExportItem {
    scene: LayoutScene,
    selected_node_id: Option<String>,
}

impl McpTool for ExportItem {
    fn name(&self) -> &str {
        "export_item"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let format = match args.get("format").map(|value| value.trim()) {
            Some(format) if !format.is_empty() => format,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "format is required (png|jpeg|webp|pdf)".into(),
                );
            }
        };
        let explicit_item_id = args.get("itemId").map(|value| value.trim());
        if explicit_item_id == Some("") {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "itemId must not be empty".into(),
            );
        }
        let item_id = match explicit_item_id {
            Some(item_id) => item_id,
            None => match self.selected_node_id.as_deref() {
                Some(item_id) => item_id,
                None => {
                    return ToolOutcome::Err(
                        ToolErrorCode::InvalidArgument,
                        "no node is selected on the Live Canvas".into(),
                    );
                }
            },
        };
        let scale = args
            .get("scale")
            .and_then(|value| value.trim().parse::<f32>().ok())
            .unwrap_or(1.0);

        let (bytes, item_type) = if let Some(page) =
            self.scene.pages.iter().find(|page| page.id == item_id)
        {
            let result = match format {
                "png" => render_page_raster_bytes(page, RasterFormat::Png, scale),
                "jpeg" | "jpg" => render_page_raster_bytes(page, RasterFormat::Jpeg, scale),
                "webp" => render_page_raster_bytes(page, RasterFormat::Webp, scale),
                "pdf" => render_page_pdf_bytes(page),
                other => return unsupported_format(other),
            };
            (result, "page")
        } else if let Some(page) = self
            .scene
            .pages
            .iter()
            .find(|page| page.find(item_id).is_some())
        {
            let result = match format {
                "png" => render_node_on_page_raster_bytes(page, item_id, RasterFormat::Png, scale),
                "jpeg" | "jpg" => {
                    render_node_on_page_raster_bytes(page, item_id, RasterFormat::Jpeg, scale)
                }
                "webp" => {
                    render_node_on_page_raster_bytes(page, item_id, RasterFormat::Webp, scale)
                }
                "pdf" => render_node_on_page_pdf_bytes(page, item_id),
                other => return unsupported_format(other),
            };
            (result, "node")
        } else {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("item {item_id} is not a page or node in the document"),
            );
        };

        match bytes {
            Ok(bytes) => ToolOutcome::OkJson(
                serde_json::json!({
                    "itemId": item_id,
                    "itemType": item_type,
                    "format": canonical_format(format),
                    "bytes_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
                })
                .to_string(),
            ),
            Err(message) => ToolOutcome::Err(ToolErrorCode::ToolFailed, message),
        }
    }
}

fn canonical_format(format: &str) -> &str {
    if format == "jpg" {
        "jpeg"
    } else {
        format
    }
}

fn unsupported_format(format: &str) -> ToolOutcome {
    ToolOutcome::Err(
        ToolErrorCode::InvalidArgument,
        format!("unknown format {format:?}: must be one of png, jpeg, webp, pdf"),
    )
}

pub fn export_item_snapshot(state: &EditorState) -> ExportItem {
    ExportItem {
        scene: op_pen_loader::editor_state_to_layout_scene(state),
        selected_node_id: state
            .selection
            .anchor
            .is_real()
            .then(|| state.selection.anchor.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_ui::layout_scene::{LayoutScene, ScenePage};
    use op_editor_ui::Color;
    use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
    use std::collections::BTreeMap;

    use crate::export::test_support::filled_rect;

    fn two_page_scene() -> LayoutScene {
        LayoutScene {
            pages: vec![
                ScenePage {
                    id: "page-one".into(),
                    name: "Page One".into(),
                    children: vec![filled_rect(
                        "page-one-node",
                        0.0,
                        0.0,
                        20.0,
                        20.0,
                        Color::BLACK,
                    )],
                },
                ScenePage {
                    id: "page-two".into(),
                    name: "Page Two".into(),
                    children: vec![filled_rect(
                        "selected-node",
                        100.0,
                        120.0,
                        40.0,
                        30.0,
                        Color::BLACK,
                    )],
                },
            ],
            active_page_index: 0,
        }
    }

    fn tool_from_scene(scene: LayoutScene, selected_node_id: Option<String>) -> ExportItem {
        ExportItem {
            scene,
            selected_node_id,
        }
    }

    fn call(
        tool: &ExportItem,
        item_id: Option<&str>,
        format: Option<&str>,
        scale: Option<&str>,
    ) -> ToolOutcome {
        let mut args = BTreeMap::new();
        if let Some(item_id) = item_id {
            args.insert("itemId".into(), item_id.into());
        }
        if let Some(format) = format {
            args.insert("format".into(), format.into());
        }
        if let Some(scale) = scale {
            args.insert("scale".into(), scale.into());
        }
        tool.call(&args)
    }

    fn expect_ok_json(outcome: ToolOutcome) -> serde_json::Value {
        match outcome {
            ToolOutcome::OkJson(json) => serde_json::from_str(&json).expect("valid JSON"),
            other => panic!("expected OkJson, got {other:?}"),
        }
    }

    fn decoded_bytes(json: &serde_json::Value) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD
            .decode(json["bytes_base64"].as_str().expect("bytes_base64"))
            .expect("valid base64")
    }

    #[test]
    fn explicit_non_active_page_exports_png() {
        let tool = tool_from_scene(two_page_scene(), Some("selected-node".into()));
        let json = expect_ok_json(call(&tool, Some("page-two"), Some("png"), Some("1")));
        assert_eq!(json["itemId"], "page-two");
        assert_eq!(json["itemType"], "page");
        assert_eq!(
            &decoded_bytes(&json)[..8],
            &[0x89, b'P', b'N', b'G', 13, 10, 26, 10]
        );
    }

    #[test]
    fn explicit_node_on_non_active_page_exports_png() {
        let tool = tool_from_scene(two_page_scene(), None);
        let json = expect_ok_json(call(&tool, Some("selected-node"), Some("png"), None));
        assert_eq!(json["itemType"], "node");
        assert_eq!(
            &decoded_bytes(&json)[..8],
            &[0x89, b'P', b'N', b'G', 13, 10, 26, 10]
        );
    }

    #[test]
    fn omitted_item_exports_captured_selection() {
        let tool = tool_from_scene(two_page_scene(), Some("selected-node".into()));
        let json = expect_ok_json(call(&tool, None, Some("png"), None));
        assert_eq!(json["itemId"], "selected-node");
        assert_eq!(json["itemType"], "node");
    }

    #[test]
    fn omitted_item_without_selection_is_an_error() {
        let tool = tool_from_scene(two_page_scene(), None);
        match call(&tool, None, Some("png"), None) {
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, message) => {
                assert!(message.contains("no node is selected"), "{message}");
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_format_is_an_error() {
        let tool = tool_from_scene(two_page_scene(), None);
        match call(&tool, Some("page-two"), Some("tiff"), None) {
            ToolOutcome::Err(ToolErrorCode::InvalidArgument, message) => {
                assert!(message.contains("tiff"), "{message}");
            }
            other => panic!("expected InvalidArgument, got {other:?}"),
        }
    }

    #[test]
    fn missing_format_is_an_error() {
        let tool = tool_from_scene(two_page_scene(), None);
        assert!(matches!(
            call(&tool, Some("page-two"), None, None),
            ToolOutcome::Err(ToolErrorCode::MissingArgument, _)
        ));
    }
}
