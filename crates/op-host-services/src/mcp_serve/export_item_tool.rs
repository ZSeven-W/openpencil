use std::collections::BTreeMap;
use std::io::Cursor;

use base64::Engine as _;
use jian_ops_schema::{node::PenNode, PenDocument};
use op_editor_core::{walkers::find_node, EditorState, NodeId};
use op_editor_ui::layout_scene::LayoutScene;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
use op_render_export::motion::{export_motion, ClipTrigger, MotionExportOptions, MotionFormat};

use crate::export::{render_node_on_page_raster_bytes, render_page_raster_bytes, RasterFormat};
use crate::export_pdf::{render_node_on_page_pdf_bytes, render_page_pdf_bytes};

pub struct ExportItem {
    scene: LayoutScene,
    document: Option<PenDocument>,
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
                    "format is required (png|jpeg|webp|pdf|gif|apng|png-seq|mp4)".into(),
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

        if let Some(motion) = parse_motion_format(format) {
            return self.motion_export_response(args, item_id, format, motion, scale);
        }
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
            Err(error) => ToolOutcome::Err(ToolErrorCode::ToolFailed, error.to_string()),
        }
    }
}

impl ExportItem {
    pub(super) fn motion_export_response(
        &self,
        args: &BTreeMap<String, String>,
        item_id: &str,
        format_name: &str,
        format: MotionFormat,
        scale: f32,
    ) -> ToolOutcome {
        match self.motion_bytes(args, item_id, format, scale) {
            Ok(bytes) => ToolOutcome::OkJson(
                serde_json::json!({
                    "itemId": item_id,
                    "itemType": "node",
                    "format": format_name,
                    "bytes_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
                })
                .to_string(),
            ),
            Err((kind, message)) => ToolOutcome::Err(kind, message),
        }
    }

    fn motion_bytes(
        &self,
        args: &BTreeMap<String, String>,
        item_id: &str,
        format: MotionFormat,
        scale: f32,
    ) -> Result<Vec<u8>, (ToolErrorCode, String)> {
        let (node, scene) = self.motion_target(item_id)?;
        let trigger = parse_motion_trigger(
            args.get("trigger").map(String::as_str),
            args.get("clip").map(String::as_str),
        )?;
        let fps = args
            .get("fps")
            .map(|value| value.parse::<u32>())
            .transpose()
            .map_err(|_| invalid("fps must be an integer"))?
            .unwrap_or(30);
        let duration_ms = args
            .get("durationMs")
            .map(|value| value.parse::<u64>())
            .transpose()
            .map_err(|_| invalid("durationMs must be a nonnegative integer"))?;
        let background = args
            .get("background")
            .map(|value| {
                serde_json::from_value(serde_json::Value::String(value.clone()))
                    .map_err(|_| invalid("background must be transparent or white"))
            })
            .transpose()?
            .unwrap_or_default();
        let reduced_motion = args
            .get("reducedMotion")
            .map(|value| value.parse::<bool>())
            .transpose()
            .map_err(|_| invalid("reducedMotion must be a boolean"))?;
        let reduced_motion = self.document.as_ref().is_some_and(|document| {
            op_editor_core::motion_export::export_reduced_motion(document, reduced_motion)
        });
        let options = MotionExportOptions {
            trigger,
            fps,
            scale,
            duration_ms,
            background,
            reduced_motion,
            ..Default::default()
        };
        let mut output = Cursor::new(Vec::new());
        export_motion(&mut output, &scene, node, format, &options)
            .map_err(|error| (ToolErrorCode::ToolFailed, error.to_string()))?;
        Ok(output.into_inner())
    }
    pub(super) fn motion_target(
        &self,
        item_id: &str,
    ) -> Result<(&PenNode, LayoutScene), MotionToolError> {
        let Some(document) = &self.document else {
            return Err(invalid("motion export needs an authored document"));
        };
        let node_id = NodeId::new(item_id);
        let Some(page) = self
            .scene
            .pages
            .iter()
            .find(|page| page.find(item_id).is_some())
        else {
            return Err(invalid(
                "motion export requires a node id on the Live Canvas",
            ));
        };
        let children = document
            .pages
            .as_ref()
            .into_iter()
            .flatten()
            .find(|candidate| candidate.id == page.id)
            .map_or(document.children.as_slice(), |page| &page.children);
        let Some(node) = find_node(children, &node_id) else {
            return Err(invalid("motion export requires an authored node"));
        };
        let scene = LayoutScene {
            pages: vec![page.clone()],
            active_page_index: 0,
        };
        Ok((node, scene))
    }
}

pub(super) type MotionToolError = (ToolErrorCode, String);

fn invalid(message: &str) -> MotionToolError {
    (ToolErrorCode::InvalidArgument, message.to_owned())
}

pub(super) fn parse_motion_trigger(
    trigger: Option<&str>,
    clip: Option<&str>,
) -> Result<ClipTrigger, MotionToolError> {
    match (trigger, clip) {
        (None | Some("manual"), Some(id)) if !id.trim().is_empty() => {
            Ok(ClipTrigger::Manual(id.into()))
        }
        (None | Some("mount"), None) => Ok(ClipTrigger::Mount),
        (Some("inView"), None) => Ok(ClipTrigger::InView),
        _ => Err(invalid(
            "trigger must be mount, inView, or manual with a clip id",
        )),
    }
}

pub(super) fn parse_motion_format(format: &str) -> Option<MotionFormat> {
    Some(match format {
        "gif" => MotionFormat::Gif,
        "apng" => MotionFormat::Apng,
        "png-seq" => MotionFormat::PngSequence,
        "mp4" => MotionFormat::Mp4,
        _ => return None,
    })
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
        format!(
            "unknown format {format:?}: must be one of png, jpeg, webp, pdf, gif, apng, png-seq, mp4"
        ),
    )
}

pub fn export_item_snapshot(state: &EditorState) -> ExportItem {
    ExportItem {
        scene: op_pen_loader::editor_state_to_layout_scene(state),
        document: Some(state.doc.clone()),
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

    #[test]
    fn exports_a_named_clip_through_the_live_canvas_tool() {
        let document = jian_ops_schema::load_str(
            r##"{"version":"1.1","children":[{"type":"rectangle","id":"box","width":20,"height":12,
            "fill":[{"type":"solid","color":"#cc5544"}],"animations":[{
              "trigger":"manual","id":"fade","durationMs":100,"easing":"linear",
              "keyframes":[{"offset":0,"values":{"opacity":0}},{"offset":1,"values":{"opacity":1}}]}]}]}"##,
        ).unwrap().value;
        let state = EditorState::from_document(document);
        let args = BTreeMap::from([
            ("itemId".into(), "box".into()),
            ("format".into(), "apng".into()),
            ("clip".into(), "fade".into()),
            ("fps".into(), "24".into()),
        ]);
        let outcome = export_item_snapshot(&state).call(&args);
        let ToolOutcome::OkJson(payload) = outcome else {
            panic!("motion export failed: {outcome:?}");
        };
        let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload["bytes_base64"].as_str().unwrap())
            .unwrap();
        assert_eq!(payload["format"], "apng");
        assert!(bytes.windows(4).any(|chunk| chunk == b"acTL"));
    }

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
            document: None,
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
