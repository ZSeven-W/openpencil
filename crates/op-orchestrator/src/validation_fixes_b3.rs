//! B3: `build_node_from_spec` and parent-snapshot helpers.
//!
//! Faithful port of `buildNodeFromSpec`
//! (`apps/web/src/services/ai/design-validation-fixes.ts:307-372`)
//! and the tiny snapshot helpers used by the `addChild` branch of
//! `apply_validation_fixes`.
//!
//! **Icon resolution note:** The TS implementation calls
//! `lookupIconByName(spec.name)` first (a compile-time bundle of SVG
//! paths), and falls back to a host HTTP endpoint
//! (`/api/ai/icon?name=`). Neither path is available in the Rust crate
//! today:
//! - `lookupIconByName` lives in the TS web app bundle.
//! - The HTTP endpoint is a Nitro server route.
//!
//! This port uses a **stub icon resolver** that always returns `None`,
//! so `path`-type nodes are built without a `d` or `iconId` field — a
//! placeholder that a future `IconResolver` trait can fill in.  The
//! node is still emitted (width/height/fill defaulting to icon
//! conventions) so the insertion is not silently dropped.  A `TODO`
//! marker documents the gap.

use jian_ops_schema::node::path::PathNode;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::node::{ContainerProps, FrameNode, RectangleNode};
use jian_ops_schema::node::{CornerRadius, FontWeight, PenNodeBase, TextContent, TextNode};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{PenFill, PenStroke, SolidFillBody, StrokeThickness};
use serde_json::Value;

// NOTE: `is_valid_hex_color` is private in `validation_fixes.rs` —
// we duplicate the tiny predicate here to avoid a visibility change.
fn is_valid_hex_color(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('#') else {
        return false;
    };
    let len = rest.len();
    if len != 3 && len != 4 && len != 6 && len != 8 {
        return false;
    }
    rest.chars().all(|c| c.is_ascii_hexdigit())
}

// ── Sizing from JSON value ─────────────────────────────────────────────────────

/// Parse a `width` / `height` spec field into a `SizingBehavior`.
/// Accepts numbers and keyword strings (`"fill_container"` / `"fit_content"`).
fn parse_sizing(v: &Value) -> Option<SizingBehavior> {
    if let Some(n) = v.as_f64() {
        return Some(SizingBehavior::Number(n));
    }
    if let Some(s) = v.as_str() {
        use jian_ops_schema::sizing::SizingKeyword;
        match s {
            "fill_container" => return Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
            "fit_content" => return Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
            _ => {}
        }
    }
    None
}

// ── Fill from hex string ───────────────────────────────────────────────────────

fn solid_fill_from_hex(hex: &str) -> Vec<PenFill> {
    vec![PenFill::Solid(SolidFillBody {
        color: hex.to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })]
}

// ── build_node_from_spec ───────────────────────────────────────────────────────

/// Build a single `PenNode` from the raw JSON spec produced by the vision LLM.
///
/// Faithful port of `buildNodeFromSpec`
/// (`design-validation-fixes.ts:307-372`).
///
/// The `_index` parameter is accepted for future insertion-order support
/// but is not used today (see `TODO` in `validation_fixes_apply.rs`).
///
/// Returns `None` when the spec has an unsupported or absent `type` field.
pub(crate) fn build_node_from_spec(spec: &Value, _index: Option<usize>) -> Option<PenNode> {
    let obj = spec.as_object()?;
    let node_type = obj.get("type")?.as_str()?;

    // Placeholder id — overwritten by InsertSubtree's id-remapping step.
    let placeholder_id = "b3_placeholder".to_string();

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // ── fillColor ─────────────────────────────────────────────────────────────
    let fill_color: Option<String> = obj
        .get("fillColor")
        .and_then(|v| v.as_str())
        .filter(|s| is_valid_hex_color(s))
        .map(|s| s.to_string());

    match node_type {
        "frame" | "group" | "rectangle" | "ellipse" => {
            build_container_node(node_type, placeholder_id, name, obj, fill_color)
        }
        "text" => build_text_node(placeholder_id, name, obj, fill_color),
        "path" => {
            // TODO: wire a real IconResolver trait when host-side icon lookup
            // is available (TS uses `lookupIconByName` + `/api/ai/icon?name=`).
            build_path_node(placeholder_id, name, obj, fill_color)
        }
        _ => None,
    }
}

// ── Container node builder (frame / group / rectangle / ellipse) ──────────────

fn build_container_node(
    node_type: &str,
    id: String,
    name: Option<String>,
    obj: &serde_json::Map<String, Value>,
    fill_color: Option<String>,
) -> Option<PenNode> {
    use jian_ops_schema::node::base::NumberOrExpression;
    use jian_ops_schema::node::container::{AlignItems, JustifyContent, LayoutMode, Padding};

    let base = PenNodeBase {
        id,
        name,
        ..Default::default()
    };

    // Width / height
    let width = obj.get("width").and_then(parse_sizing);
    let height = obj.get("height").and_then(parse_sizing);

    // cornerRadius
    let corner_radius = obj
        .get("cornerRadius")
        .and_then(|v| v.as_f64())
        .map(CornerRadius::Uniform);

    // layout
    let layout = obj
        .get("layout")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "none" => Some(LayoutMode::None),
            "vertical" => Some(LayoutMode::Vertical),
            "horizontal" => Some(LayoutMode::Horizontal),
            _ => None,
        });

    // gap
    let gap = obj
        .get("gap")
        .and_then(|v| v.as_f64())
        .map(NumberOrExpression::Number);

    // padding
    let padding = obj.get("padding").and_then(|v| {
        if let Some(n) = v.as_f64() {
            Some(Padding::Uniform(n))
        } else if let Some(arr) = v.as_array() {
            match arr.len() {
                2 => {
                    let a = arr[0].as_f64()?;
                    let b = arr[1].as_f64()?;
                    Some(Padding::XY([a, b]))
                }
                4 => {
                    let a = arr[0].as_f64()?;
                    let b = arr[1].as_f64()?;
                    let c = arr[2].as_f64()?;
                    let d = arr[3].as_f64()?;
                    Some(Padding::LtrB([a, b, c, d]))
                }
                _ => None,
            }
        } else {
            None
        }
    });

    // alignItems
    let align_items = obj
        .get("alignItems")
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "start" => Some(AlignItems::Start),
            "center" => Some(AlignItems::Center),
            "end" => Some(AlignItems::End),
            _ => None,
        });

    // justifyContent
    let justify_content =
        obj.get("justifyContent")
            .and_then(|v| v.as_str())
            .and_then(|s| match s {
                "start" => Some(JustifyContent::Start),
                "center" => Some(JustifyContent::Center),
                "end" => Some(JustifyContent::End),
                "space_between" => Some(JustifyContent::SpaceBetween),
                "space_around" => Some(JustifyContent::SpaceAround),
                _ => None,
            });

    let fill = fill_color.map(|hex| solid_fill_from_hex(&hex));

    let container = ContainerProps {
        width,
        height,
        layout,
        gap,
        padding,
        justify_content,
        align_items,
        clip_content: None,
        corner_radius,
        fill,
        stroke: None,
        effects: None,
    };

    // All four container types share the same ContainerProps —
    // map to the right PenNode variant.
    let node = match node_type {
        "group" => PenNode::Group(jian_ops_schema::node::GroupNode {
            base,
            container,
            children: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
        "rectangle" | "ellipse" => {
            // Ellipse is not a container in jian-ops-schema (no ContainerProps);
            // use Rectangle for both, which is the closest equivalent.
            PenNode::Rectangle(RectangleNode {
                base,
                container,
                children: None,
                state: None,
                bindings: None,
                events: None,
                lifecycle: None,
                semantics: None,
                gestures: None,
                route: None,
            })
        }
        _ => PenNode::Frame(FrameNode {
            base,
            container,
            children: None,
            reusable: None,
            slot: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        }),
    };
    Some(node)
}

// ── Text node builder ─────────────────────────────────────────────────────────

fn build_text_node(
    id: String,
    name: Option<String>,
    obj: &serde_json::Map<String, Value>,
    fill_color: Option<String>,
) -> Option<PenNode> {
    let base = PenNodeBase {
        id,
        name,
        ..Default::default()
    };

    let content = obj
        .get("content")
        .and_then(|v| v.as_str())
        .map(|s| TextContent::Plain(s.to_string()))
        .unwrap_or_else(|| TextContent::Plain(String::new()));

    let font_size = obj.get("fontSize").and_then(|v| v.as_f64());
    let font_weight = obj
        .get("fontWeight")
        .and_then(|v| v.as_u64())
        .map(|n| FontWeight::Number(n as u32));

    let width = obj.get("width").and_then(parse_sizing);
    let height = obj.get("height").and_then(parse_sizing);

    let fill = fill_color.map(|hex| solid_fill_from_hex(&hex));

    Some(PenNode::Text(TextNode {
        base,
        width,
        height,
        content,
        font_family: None,
        font_size,
        font_weight,
        font_style: None,
        letter_spacing: None,
        line_height: None,
        text_align: None,
        text_align_vertical: None,
        text_growth: None,
        underline: None,
        strikethrough: None,
        fill,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    }))
}

// ── Path / icon node builder ──────────────────────────────────────────────────

/// Build a `path`-type node (icon placeholder).
///
/// The TS implementation attempts to resolve the icon's SVG `d` string via
/// `lookupIconByName` and a host `/api/ai/icon?name=` HTTP call.  Neither is
/// available in this Rust crate today, so `d` and `iconId` are left `None`
/// (stub / placeholder).  The node IS emitted so the insertion is not silently
/// dropped — a caller with icon data can mutate `d` after insertion.
///
/// Default dimensions follow TS: `width = 18, height = 18` when not given.
fn build_path_node(
    id: String,
    name: Option<String>,
    obj: &serde_json::Map<String, Value>,
    fill_color: Option<String>,
) -> Option<PenNode> {
    let base = PenNodeBase {
        id,
        name,
        ..Default::default()
    };

    // Default icon dimensions (TS: `if (!spec.width) node.width = 18`).
    let width = obj
        .get("width")
        .and_then(parse_sizing)
        .or(Some(SizingBehavior::Number(18.0)));
    let height = obj
        .get("height")
        .and_then(parse_sizing)
        .or(Some(SizingBehavior::Number(18.0)));

    let color = fill_color
        .as_deref()
        .unwrap_or("#64748B") // TS default fallback color for icons
        .to_string();

    // TODO: resolve icon d-string via an `IconResolver` trait when available.
    // For now, emit a stub path node with fill but no d/iconId.
    // TS pattern:
    //   if (icon.style === 'stroke') { stroke = ...; fill = [] }
    //   else { fill = [solid(color)] }
    // Without icon resolution we default to stroke style.
    let stroke = Some(PenStroke {
        thickness: StrokeThickness::Uniform(2.0),
        align: None,
        join: None,
        cap: None,
        dash_pattern: None,
        dash_offset: None,
        fill: Some(solid_fill_from_hex(&color)),
    });

    Some(PenNode::Path(PathNode {
        base,
        icon_id: None, // TODO: icon resolver
        d: None,       // TODO: icon resolver
        anchors: None,
        closed: None,
        width,
        height,
        fill: Some(vec![]), // stroke-style: empty fill
        stroke,
        effects: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    }))
}

// ── Parent snapshot helpers ───────────────────────────────────────────────────

/// Extract the `justifyContent` string from a node before mutation.
pub(crate) fn snapshot_justify(node: &PenNode) -> Option<String> {
    use jian_ops_schema::node::container::JustifyContent;
    let jc = match node {
        PenNode::Frame(n) => n.container.justify_content.as_ref(),
        PenNode::Group(n) => n.container.justify_content.as_ref(),
        PenNode::Rectangle(n) => n.container.justify_content.as_ref(),
        _ => return None,
    };
    jc.map(|j| match j {
        JustifyContent::Start => "start".to_string(),
        JustifyContent::Center => "center".to_string(),
        JustifyContent::End => "end".to_string(),
        JustifyContent::SpaceBetween => "space_between".to_string(),
        JustifyContent::SpaceAround => "space_around".to_string(),
    })
}

/// Extract the `layout` mode string from a node before mutation.
pub(crate) fn snapshot_layout(node: &PenNode) -> Option<String> {
    use jian_ops_schema::node::container::LayoutMode;
    let lm = match node {
        PenNode::Frame(n) => n.container.layout.as_ref(),
        PenNode::Group(n) => n.container.layout.as_ref(),
        PenNode::Rectangle(n) => n.container.layout.as_ref(),
        _ => return None,
    };
    lm.map(|l| match l {
        LayoutMode::None => "none".to_string(),
        LayoutMode::Vertical => "vertical".to_string(),
        LayoutMode::Horizontal => "horizontal".to_string(),
    })
}

/// Return the canonical type string for a `PenNode` variant.
pub(crate) fn node_type_str(node: &PenNode) -> &'static str {
    match node {
        PenNode::Frame(_) => "frame",
        PenNode::Group(_) => "group",
        PenNode::Rectangle(_) => "rectangle",
        PenNode::Ellipse(_) => "ellipse",
        PenNode::Line(_) => "line",
        PenNode::Polygon(_) => "polygon",
        PenNode::Path(_) => "path",
        PenNode::Text(_) => "text",
        PenNode::TextInput(_) => "text_input",
        PenNode::Image(_) => "image",
        PenNode::IconFont(_) => "icon_font",
        PenNode::Ref(_) => "ref",
    }
}
