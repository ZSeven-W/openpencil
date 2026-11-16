//! MCP motion tools that require the native raster backend.

use std::collections::BTreeMap;

use base64::Engine as _;
use op_editor_core::EditorState;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};
use op_render_export::motion::{render_motion_contact_sheet, MAX_CONTACT_SHEET_FRAMES};
use serde_json::json;

use super::export_item_tool::{
    export_item_snapshot, parse_motion_format, parse_motion_trigger, ExportItem, MotionToolError,
};

#[derive(Clone, Copy)]
pub(crate) enum MotionRenderTool {
    Frames,
    Export,
}

impl MotionRenderTool {
    pub(crate) fn snapshot(self, state: &EditorState) -> impl McpTool {
        MotionRenderSnapshot {
            tool: self,
            source: export_item_snapshot(state),
        }
    }
}

struct MotionRenderSnapshot {
    tool: MotionRenderTool,
    source: ExportItem,
}

impl McpTool for MotionRenderSnapshot {
    fn name(&self) -> &str {
        match self.tool {
            MotionRenderTool::Frames => "get_motion_frames",
            MotionRenderTool::Export => "export_motion",
        }
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        self.execute(args)
            .unwrap_or_else(|(code, message)| ToolOutcome::Err(code, message))
    }
}

impl MotionRenderSnapshot {
    fn execute(&self, args: &BTreeMap<String, String>) -> Result<ToolOutcome, MotionToolError> {
        let _ = op_editor_ui::set_brand_catalog(crate::web_static::ICONIFY_BRANDS_JSON);
        let root = required(args, "rootId")?;
        if root.trim().is_empty() {
            return Err(invalid("rootId must not be empty"));
        }
        let scale = match args.get("scale") {
            Some(raw) => raw
                .parse::<f32>()
                .ok()
                .filter(|v| v.is_finite() && *v > 0.0)
                .ok_or_else(|| invalid("scale must be a positive finite number"))?,
            None => 1.0,
        };
        let clip = args.get("clipId").map(String::as_str);
        match self.tool {
            MotionRenderTool::Export => {
                let format_name = required(args, "format")?;
                let format = parse_motion_format(format_name)
                    .ok_or_else(|| invalid("format must be gif, apng, or png-seq"))?;
                let mut forwarded = args.clone();
                if let Some(clip) = clip {
                    forwarded.insert("clip".into(), clip.into());
                }
                Ok(self
                    .source
                    .motion_export_response(&forwarded, root, format_name, format, scale))
            }
            MotionRenderTool::Frames => {
                let times: Vec<u64> =
                    serde_json::from_str(required(args, "timesMs")?).map_err(|_| {
                        invalid("timesMs must be an array of nonnegative integer milliseconds")
                    })?;
                if times.is_empty() || times.len() > MAX_CONTACT_SHEET_FRAMES as usize {
                    return Err(invalid(format!(
                        "timesMs must contain 1–{MAX_CONTACT_SHEET_FRAMES} samples"
                    )));
                }
                let trigger = parse_motion_trigger(args.get("trigger").map(String::as_str), clip)?;
                let (node, scene) = self.source.motion_target(root)?;
                let rendered = render_motion_contact_sheet(&scene, node, &trigger, &times, scale)
                    .map_err(|err| (ToolErrorCode::ToolFailed, err.to_string()))?;
                let frames: Vec<_> = rendered
                    .frames
                    .iter()
                    .map(|cell| {
                        json!({
                            "timeMs": cell.time_ms, "x": cell.x, "y": cell.y,
                            "width": cell.width, "height": cell.height,
                        })
                    })
                    .collect();
                let png = base64::engine::general_purpose::STANDARD.encode(rendered.png);
                Ok(ToolOutcome::OkImageContent {
                    image_base64: png,
                    mime_type: "image/png".into(),
                    metadata_json: Some(json!({"rootId":root,"frames":frames}).to_string()),
                })
            }
        }
    }
}

fn invalid(message: impl Into<String>) -> MotionToolError {
    (ToolErrorCode::InvalidArgument, message.into())
}

fn required<'a>(args: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, MotionToolError> {
    args.get(key)
        .map(String::as_str)
        .ok_or_else(|| (ToolErrorCode::MissingArgument, format!("{key} is required")))
}
