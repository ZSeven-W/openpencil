//! Per-node attribute MCP write tools (rotation / text / corner
//! radius / font size / font weight). Carved off `component_tools.rs`
//! to stay under the 800-line cap.

use std::collections::BTreeMap;

use super::{McpCommand, McpTool, ToolErrorCode, ToolOutcome};

/// First-party `set_node_rotation` tool — set rotation in
/// degrees on a node by id.
pub struct SetNodeRotation;

impl McpTool for SetNodeRotation {
    fn name(&self) -> &str {
        "set_node_rotation"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(raw_deg) = args.get("degrees") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "degrees is required (finite f32)".into(),
            );
        };
        let degrees: f32 = match raw_deg.parse::<f32>() {
            Ok(d) if d.is_finite() => d,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("degrees must be a finite f32, got {raw_deg:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeRotation { node_id, degrees },
        )
    }
}

pub fn set_node_rotation_snapshot() -> SetNodeRotation {
    SetNodeRotation
}

/// First-party `set_node_text` tool — set text content on a
/// Text-kind node by id. Other kinds reject at apply time.
pub struct SetNodeText;

impl McpTool for SetNodeText {
    fn name(&self) -> &str {
        "set_node_text"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(text) = args.get("text") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "text is required".into(),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeText {
                node_id,
                text: text.clone(),
            },
        )
    }
}

pub fn set_node_text_snapshot() -> SetNodeText {
    SetNodeText
}

/// First-party `set_node_corner_radius` tool — write the
/// `corner_radius` (doc-px) field on a node. Rejects negative
/// values + nan/inf.
pub struct SetNodeCornerRadius;

impl McpTool for SetNodeCornerRadius {
    fn name(&self) -> &str {
        "set_node_corner_radius"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(raw_r) = args.get("radius") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "radius is required (non-negative f32 doc-px)".into(),
            );
        };
        let radius: f32 = match raw_r.parse::<f32>() {
            Ok(r) if r.is_finite() && r >= 0.0 => r,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("radius must be a non-negative finite f32, got {raw_r:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeCornerRadius { node_id, radius },
        )
    }
}

pub fn set_node_corner_radius_snapshot() -> SetNodeCornerRadius {
    SetNodeCornerRadius
}

/// First-party `set_node_font_size` tool — write `font_size`
/// (doc-px) on a Text-kind node. Rejects non-Text kinds, non-
/// positive sizes, NaN/Inf.
pub struct SetNodeFontSize;

impl McpTool for SetNodeFontSize {
    fn name(&self) -> &str {
        "set_node_font_size"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(raw_size) = args.get("font_size") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "font_size is required (positive finite f32 doc-px)".into(),
            );
        };
        let font_size: f32 = match raw_size.parse::<f32>() {
            Ok(s) if s.is_finite() && s > 0.0 => s,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("font_size must be a positive finite f32, got {raw_size:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeFontSize { node_id, font_size },
        )
    }
}

pub fn set_node_font_size_snapshot() -> SetNodeFontSize {
    SetNodeFontSize
}

/// First-party `set_node_font_weight` tool — write `font_weight`
/// (OpenType range 1..=1000) on a Text-kind node. Rejects
/// out-of-range weights so the per-codepoint typeface cache lookup
/// stays well-formed, and non-Text kinds at apply time.
pub struct SetNodeFontWeight;

impl McpTool for SetNodeFontWeight {
    fn name(&self) -> &str {
        "set_node_font_weight"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(raw_w) = args.get("font_weight") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "font_weight is required (u16 in 1..=1000)".into(),
            );
        };
        let font_weight: u16 = match raw_w.parse::<u16>() {
            Ok(w) if (1..=1000).contains(&w) => w,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("font_weight must be a u16 in 1..=1000, got {raw_w:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeFontWeight {
                node_id,
                font_weight,
            },
        )
    }
}

pub fn set_node_font_weight_snapshot() -> SetNodeFontWeight {
    SetNodeFontWeight
}

/// First-party `set_node_stroke_hex` tool — set the stroke color
/// on a node. Existing stroke gets its color overwritten; missing
/// stroke gets a fresh 1 doc-px stroke attached at the parsed
/// color so the change is immediately visible.
pub struct SetNodeStrokeHex;

impl McpTool for SetNodeStrokeHex {
    fn name(&self) -> &str {
        "set_node_stroke_hex"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(hex) = args.get("hex") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "hex is required (#rgb / #rrggbb / #rrggbbaa)".into(),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeStrokeHex {
                node_id,
                hex: hex.clone(),
            },
        )
    }
}

pub fn set_node_stroke_hex_snapshot() -> SetNodeStrokeHex {
    SetNodeStrokeHex
}

/// First-party `set_node_stroke_width` tool — set the stroke
/// width (doc-px) on a node. width == 0 clears the stroke; width
/// > 0 on a node without an existing stroke attaches a fresh
/// black-default stroke at that width.
pub struct SetNodeStrokeWidth;

impl McpTool for SetNodeStrokeWidth {
    fn name(&self) -> &str {
        "set_node_stroke_width"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(raw_w) = args.get("width") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "width is required (non-negative finite f32 doc-px)".into(),
            );
        };
        let width: f32 = match raw_w.parse::<f32>() {
            Ok(w) if w.is_finite() && w >= 0.0 => w,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("width must be a non-negative finite f32, got {raw_w:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeStrokeWidth { node_id, width },
        )
    }
}

pub fn set_node_stroke_width_snapshot() -> SetNodeStrokeWidth {
    SetNodeStrokeWidth
}

/// First-party `set_node_fill_hex` tool — set the fill color on a
/// node by id. Sister tool to `set_node_stroke_hex`; the existing
/// `update_node` continues to handle multi-field updates.
pub struct SetNodeFillHex;

impl McpTool for SetNodeFillHex {
    fn name(&self) -> &str {
        "set_node_fill_hex"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(hex) = args.get("hex") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "hex is required (#rgb / #rrggbb / #rrggbbaa)".into(),
            );
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeFillHex {
                node_id,
                hex: hex.clone(),
            },
        )
    }
}

pub fn set_node_fill_hex_snapshot() -> SetNodeFillHex {
    SetNodeFillHex
}

/// First-party `set_node_name` tool — rename a node by id. Empty
/// names (after trimming) are rejected at apply time so the
/// LayerPanel never shows a blank row.
pub struct SetNodeName;

impl McpTool for SetNodeName {
    fn name(&self) -> &str {
        "set_node_name"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let Some(raw_id) = args.get("node_id") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "node_id is required".into(),
            );
        };
        let node_id: u64 = match raw_id.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("node_id must be a positive u64, got {raw_id:?}"),
                );
            }
        };
        let Some(name) = args.get("name") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "name is required (non-empty after trim)".into(),
            );
        };
        if name.trim().is_empty() {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                "name must not be empty after trimming whitespace".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            McpCommand::SetNodeName {
                node_id,
                name: name.clone(),
            },
        )
    }
}

pub fn set_node_name_snapshot() -> SetNodeName {
    SetNodeName
}
