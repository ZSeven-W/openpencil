//! Per-node attribute MCP write tools (rotation / text / corner
//! radius / font size / font weight / stroke / fill / name).
//!
//! Ported off shell-core's `McpCommand` onto `op_editor_core::
//! EditorCommand`. Node ids are now canonical `.op` schema strings.

use std::collections::BTreeMap;

use super::write_tools::parse_node_id;
use super::{EditorCommand, McpTool, ToolErrorCode, ToolOutcome};

/// First-party `set_node_rotation` tool — set rotation in degrees.
pub struct SetNodeRotation;

impl McpTool for SetNodeRotation {
    fn name(&self) -> &str {
        "set_node_rotation"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
        ToolOutcome::OkWithCommand(out, EditorCommand::SetNodeRotation { node_id, degrees })
    }
}

pub fn set_node_rotation_snapshot() -> SetNodeRotation {
    SetNodeRotation
}

/// First-party `set_node_text` tool — set text content on a Text-kind
/// node. Other kinds reject at apply time.
pub struct SetNodeText;

impl McpTool for SetNodeText {
    fn name(&self) -> &str {
        "set_node_text"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let Some(text) = args.get("text") else {
            return ToolOutcome::Err(ToolErrorCode::MissingArgument, "text is required".into());
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetNodeText {
                node_id,
                text: text.clone(),
            },
        )
    }
}

pub fn set_node_text_snapshot() -> SetNodeText {
    SetNodeText
}

/// First-party `set_node_corner_radius` tool — write the corner radius
/// (doc-px). Rejects negative values + nan/inf.
pub struct SetNodeCornerRadius;

impl McpTool for SetNodeCornerRadius {
    fn name(&self) -> &str {
        "set_node_corner_radius"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
        ToolOutcome::OkWithCommand(out, EditorCommand::SetNodeCornerRadius { node_id, radius })
    }
}

pub fn set_node_corner_radius_snapshot() -> SetNodeCornerRadius {
    SetNodeCornerRadius
}

/// First-party `set_node_font_size` tool — write `font_size` (doc-px)
/// on a Text-kind node.
pub struct SetNodeFontSize;

impl McpTool for SetNodeFontSize {
    fn name(&self) -> &str {
        "set_node_font_size"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
        ToolOutcome::OkWithCommand(out, EditorCommand::SetNodeFontSize { node_id, font_size })
    }
}

pub fn set_node_font_size_snapshot() -> SetNodeFontSize {
    SetNodeFontSize
}

/// First-party `set_node_font_weight` tool — write `font_weight`
/// (OpenType range 1..=1000) on a Text-kind node.
pub struct SetNodeFontWeight;

impl McpTool for SetNodeFontWeight {
    fn name(&self) -> &str {
        "set_node_font_weight"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
            EditorCommand::SetNodeFontWeight {
                node_id,
                font_weight,
            },
        )
    }
}

pub fn set_node_font_weight_snapshot() -> SetNodeFontWeight {
    SetNodeFontWeight
}

/// First-party `set_node_stroke_hex` tool — set the stroke color.
pub struct SetNodeStrokeHex;

impl McpTool for SetNodeStrokeHex {
    fn name(&self) -> &str {
        "set_node_stroke_hex"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
            EditorCommand::SetNodeStrokeHex {
                node_id,
                hex: hex.clone(),
            },
        )
    }
}

pub fn set_node_stroke_hex_snapshot() -> SetNodeStrokeHex {
    SetNodeStrokeHex
}

/// First-party `set_node_stroke_width` tool — set the stroke width.
pub struct SetNodeStrokeWidth;

impl McpTool for SetNodeStrokeWidth {
    fn name(&self) -> &str {
        "set_node_stroke_width"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
        ToolOutcome::OkWithCommand(out, EditorCommand::SetNodeStrokeWidth { node_id, width })
    }
}

pub fn set_node_stroke_width_snapshot() -> SetNodeStrokeWidth {
    SetNodeStrokeWidth
}

/// First-party `set_node_stroke_side_width` tool — set ONE side's
/// stroke width (doc-px) on a node. Emits the per-side variant so the
/// uniform stroke isn't clobbered; the engine promotes a uniform
/// stroke to its sided form at apply time.
pub struct SetNodeStrokeSideWidth;

impl McpTool for SetNodeStrokeSideWidth {
    fn name(&self) -> &str {
        "set_node_stroke_side_width"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let Some(raw_side) = args.get("side") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "side is required (top / right / bottom / left)".into(),
            );
        };
        let side = match raw_side.trim().to_ascii_lowercase().as_str() {
            "top" => op_editor_core::StrokeSide::Top,
            "right" => op_editor_core::StrokeSide::Right,
            "bottom" => op_editor_core::StrokeSide::Bottom,
            "left" => op_editor_core::StrokeSide::Left,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("side must be one of top / right / bottom / left, got {raw_side:?}"),
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
            EditorCommand::SetNodeStrokeSideWidth {
                node_id,
                side,
                width,
            },
        )
    }
}

pub fn set_node_stroke_side_width_snapshot() -> SetNodeStrokeSideWidth {
    SetNodeStrokeSideWidth
}

/// First-party `set_node_fill_hex` tool — set the fill color.
pub struct SetNodeFillHex;

impl McpTool for SetNodeFillHex {
    fn name(&self) -> &str {
        "set_node_fill_hex"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
            EditorCommand::SetNodeFillHex {
                node_id,
                hex: hex.clone(),
            },
        )
    }
}

pub fn set_node_fill_hex_snapshot() -> SetNodeFillHex {
    SetNodeFillHex
}

/// First-party `set_node_name` tool — rename a node by id.
pub struct SetNodeName;

impl McpTool for SetNodeName {
    fn name(&self) -> &str {
        "set_node_name"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
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
            EditorCommand::SetNodeName {
                node_id,
                name: name.clone(),
            },
        )
    }
}

pub fn set_node_name_snapshot() -> SetNodeName {
    SetNodeName
}

/// Parse an optional `"true"` / `"false"` argument. `Ok(None)` when
/// the key is absent; `Err` on a non-boolean value.
// `ToolOutcome` is the shared MCP outcome type — boxing it broadly to
// shrink the `Err` variant would destabilize every tool signature.
#[allow(clippy::result_large_err)]
fn parse_opt_bool(args: &BTreeMap<String, String>, key: &str) -> Result<Option<bool>, ToolOutcome> {
    match args.get(key) {
        None => Ok(None),
        Some(v) => match v.as_str() {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            _ => Err(ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("{key} must be \"true\" or \"false\", got {v:?}"),
            )),
        },
    }
}

/// Parse an optional finite-`f64` argument. `Ok(None)` when the key is
/// absent; `Err` on a non-numeric / non-finite value.
// `ToolOutcome` is the shared MCP outcome type — see `parse_opt_bool`.
#[allow(clippy::result_large_err)]
fn parse_opt_f64(args: &BTreeMap<String, String>, key: &str) -> Result<Option<f64>, ToolOutcome> {
    match args.get(key) {
        None => Ok(None),
        Some(v) => match v.parse::<f64>() {
            Ok(n) if n.is_finite() => Ok(Some(n)),
            _ => Err(ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("{key} must be a finite number, got {v:?}"),
            )),
        },
    }
}

/// First-party `set_node_flip` tool — write the horizontal / vertical
/// mirror flags on a node. At least one axis must be supplied.
pub struct SetNodeFlip;

impl McpTool for SetNodeFlip {
    fn name(&self) -> &str {
        "set_node_flip"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let flip_x = match parse_opt_bool(args, "flip_x") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let flip_y = match parse_opt_bool(args, "flip_y") {
            Ok(v) => v,
            Err(e) => return e,
        };
        if flip_x.is_none() && flip_y.is_none() {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "at least one of flip_x / flip_y is required".into(),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetNodeFlip {
                node_id,
                flip_x,
                flip_y,
            },
        )
    }
}

pub fn set_node_flip_snapshot() -> SetNodeFlip {
    SetNodeFlip
}

/// First-party `set_ellipse_arc` tool — write arc geometry on an
/// Ellipse node. At least one of start_angle / sweep_angle /
/// inner_radius must be supplied; non-Ellipse kinds reject at apply
/// time.
pub struct SetEllipseArc;

impl McpTool for SetEllipseArc {
    fn name(&self) -> &str {
        "set_ellipse_arc"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let start_angle = match parse_opt_f64(args, "start_angle") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let sweep_angle = match parse_opt_f64(args, "sweep_angle") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let inner_radius = match parse_opt_f64(args, "inner_radius") {
            Ok(v) => v,
            Err(e) => return e,
        };
        if start_angle.is_none() && sweep_angle.is_none() && inner_radius.is_none() {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "at least one of start_angle / sweep_angle / inner_radius is required".into(),
            );
        }
        if let Some(r) = inner_radius {
            if !(0.0..=1.0).contains(&r) {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("inner_radius must be a 0.0..=1.0 fraction, got {r}"),
                );
            }
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::SetEllipseArc {
                node_id,
                start_angle,
                sweep_angle,
                inner_radius,
            },
        )
    }
}

pub fn set_ellipse_arc_snapshot() -> SetEllipseArc {
    SetEllipseArc
}

/// First-party `add_node_effect` tool — append a visual effect
/// (`shadow` / `blur` / `background_blur`) to a node with default
/// parameters.
pub struct AddNodeEffect;

impl McpTool for AddNodeEffect {
    fn name(&self) -> &str {
        "add_node_effect"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let Some(kind) = args.get("kind") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "kind is required (shadow / blur / background_blur)".into(),
            );
        };
        if !matches!(kind.as_str(), "shadow" | "blur" | "background_blur") {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("kind must be shadow / blur / background_blur, got {kind:?}"),
            );
        }
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(
            out,
            EditorCommand::AddNodeEffect {
                node_id,
                kind: kind.clone(),
            },
        )
    }
}

pub fn add_node_effect_snapshot() -> AddNodeEffect {
    AddNodeEffect
}

/// First-party `remove_node_effect` tool — drop the effect at `index`
/// from a node's effect list.
pub struct RemoveNodeEffect;

impl McpTool for RemoveNodeEffect {
    fn name(&self) -> &str {
        "remove_node_effect"
    }
    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let node_id = match parse_node_id(args, "node_id") {
            Ok(v) => v,
            Err(e) => return e,
        };
        let Some(raw) = args.get("index") else {
            return ToolOutcome::Err(
                ToolErrorCode::MissingArgument,
                "index is required (0-based u32 effect index)".into(),
            );
        };
        let index: u32 = match raw.parse::<u32>() {
            Ok(i) => i,
            Err(_) => {
                return ToolOutcome::Err(
                    ToolErrorCode::InvalidArgument,
                    format!("index must be a non-negative integer, got {raw:?}"),
                );
            }
        };
        let mut out = BTreeMap::new();
        out.insert("wrote".into(), "true".into());
        ToolOutcome::OkWithCommand(out, EditorCommand::RemoveNodeEffect { node_id, index })
    }
}

pub fn remove_node_effect_snapshot() -> RemoveNodeEffect {
    RemoveNodeEffect
}
