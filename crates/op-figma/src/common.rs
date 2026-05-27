//! Shared converter helpers — ports `converters/common.ts`. Position
//! / rotation / flip extraction from Figma transforms, corner-radius
//! mapping, base-prop assembly, the conversion context, and the
//! SYMBOL-subtree rescaler.

use crate::figma_types::{BlobOrString, FigMatrix};
use crate::kiwi::FigValue;
use crate::mappers::{map_height_sizing, map_width_sizing};
use crate::tree::TreeNode;
use jian_ops_schema::node::base::{NumberOrExpression, PenNodeBase};
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::sizing::SizingBehavior;
use std::collections::HashMap;
use std::sync::RwLock;

/// Stroke vs fill rendering for a host-resolved icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconStyle {
    Stroke,
    Fill,
}

/// Host-resolved icon match (ports TS `IconLookupResult`). `d` is the
/// canonical 24×24 lucide path; `icon_id` is the canonical lucide name
/// (kebab-case) so the renderer can re-resolve; `style` defaults to
/// stroke when unspecified.
#[derive(Debug, Clone)]
pub struct IconLookupResult {
    pub d: String,
    pub icon_id: Option<String>,
    pub style: Option<IconStyle>,
}

type IconLookupFn = dyn Fn(&str) -> Option<IconLookupResult> + Send + Sync;

static ICON_LOOKUP: RwLock<Option<Box<IconLookupFn>>> = RwLock::new(None);

/// Install (or replace) a host-provided icon-name resolver. The
/// converter consults it on every VECTOR node — if a node's name maps
/// to a registered icon, the converter emits a `Path` carrying the
/// lucide `d` + `icon_id` instead of decoding the raw vector geometry.
/// Matches TS `setIconLookup` in `converters/common.ts`.
pub fn set_icon_lookup<F>(f: F)
where
    F: Fn(&str) -> Option<IconLookupResult> + Send + Sync + 'static,
{
    if let Ok(mut slot) = ICON_LOOKUP.write() {
        *slot = Some(Box::new(f));
    }
}

/// Drop any previously-installed icon resolver.
pub fn clear_icon_lookup() {
    if let Ok(mut slot) = ICON_LOOKUP.write() {
        *slot = None;
    }
}

/// Resolve `name` through the installed lookup; `None` when no
/// resolver is set or when the resolver itself returns `None`.
pub fn lookup_icon_by_name(name: &str) -> Option<IconLookupResult> {
    let slot = ICON_LOOKUP.read().ok()?;
    let f = slot.as_ref()?;
    f(name)
}

/// Whether OpenPencil layout semantics or verbatim Figma geometry is
/// produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigLayoutMode {
    /// Keep Figma's fixed pixel sizes.
    Preserve,
    /// Resolve auto-layout into OpenPencil sizing keywords.
    OpenPencil,
}

/// Node types Figma emits that have no OpenPencil equivalent.
pub const SKIPPED_TYPES: &[&str] = &[
    "SLICE",
    "CONNECTOR",
    "SHAPE_WITH_TEXT",
    "STICKY",
    "STAMP",
    "HIGHLIGHT",
    "WASHI_TAPE",
    "CODE_BLOCK",
    "MEDIA",
    "WIDGET",
    "SECTION_OVERLAY",
    "NONE",
];

/// State threaded through the whole conversion pass.
pub struct ConversionContext {
    /// Figma SYMBOL guid → minted `fig_N` ref id.
    pub component_map: HashMap<String, String>,
    /// Figma SYMBOL guid → its subtree (for instance inlining).
    pub symbol_tree: HashMap<String, TreeNode>,
    /// Non-fatal conversion notes.
    pub warnings: Vec<String>,
    /// Sequential `fig_N` id allocator.
    pub id_counter: u32,
    /// Geometry / image blob pool.
    pub blobs: Vec<BlobOrString>,
    pub layout_mode: FigLayoutMode,
}

impl ConversionContext {
    /// Mint the next sequential `fig_N` id.
    pub fn generate_id(&mut self) -> String {
        let id = format!("fig_{}", self.id_counter);
        self.id_counter += 1;
        id
    }
}

// ── Rounding (JS `Math.round` = floor(x + 0.5)) ───────────────────

pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

pub fn round2(v: f64) -> f64 {
    js_round(v * 100.0) / 100.0
}

pub fn round3(v: f64) -> f64 {
    js_round(v * 1000.0) / 1000.0
}

/// Normalise an angle into `[0, 360)`, 2-decimal rounded.
pub fn normalize_angle(deg: f64) -> f64 {
    let mut a = deg % 360.0;
    if a < 0.0 {
        a += 360.0;
    }
    round2(a)
}

// ── Transform → geometry ──────────────────────────────────────────

/// Recover a node's pre-transform top-left in parent space. For
/// rotated / flipped nodes the (invariant) centre is used; for
/// axis-aligned nodes `(m02, m12)` is the top-left directly.
pub fn extract_position(figma: &FigValue) -> (f64, f64) {
    let Some(m) = figma.get("transform").and_then(FigMatrix::from_value) else {
        return (0.0, 0.0);
    };
    let has_rotation = m.m01.abs() > 0.001 || m.m10.abs() > 0.001;
    let has_flip = m.m00 < -0.001 || m.m11 < -0.001;
    if has_rotation || has_flip {
        if let Some(size) = figma.get("size") {
            let w = size.get_f64("x").unwrap_or(0.0);
            let h = size.get_f64("y").unwrap_or(0.0);
            let cx = m.m00 * w / 2.0 + m.m01 * h / 2.0 + m.m02;
            let cy = m.m10 * w / 2.0 + m.m11 * h / 2.0 + m.m12;
            return (round2(cx - w / 2.0), round2(cy - h / 2.0));
        }
    }
    (round2(m.m02), round2(m.m12))
}

/// Rotation in whole degrees (`abs(m00)` ignores horizontal flip).
/// `None` when zero.
pub fn extract_rotation(figma: &FigValue) -> Option<f64> {
    let m = figma.get("transform").and_then(FigMatrix::from_value)?;
    let angle = m.m10.atan2(m.m00.abs()) * (180.0 / std::f64::consts::PI);
    let rounded = js_round(angle);
    if rounded != 0.0 {
        Some(rounded)
    } else {
        None
    }
}

/// Single-axis flip recovered from the transform determinant. At most
/// one of `(flipX, flipY)` is ever set.
pub fn extract_flip(figma: &FigValue) -> (Option<bool>, Option<bool>) {
    let Some(m) = figma.get("transform").and_then(FigMatrix::from_value) else {
        return (None, None);
    };
    let det = m.m00 * m.m11 - m.m01 * m.m10;
    if det < -0.001 {
        if m.m00 < 0.0 {
            (Some(true), None)
        } else {
            (None, Some(true))
        }
    } else {
        (None, None)
    }
}

/// Corner radius — uniform number, per-corner `[tl, tr, br, bl]`, or
/// nothing.
pub fn map_corner_radius(figma: &FigValue) -> Option<CornerRadius> {
    if figma.get_bool("rectangleCornerRadiiIndependent") == Some(true) {
        let tl = figma.get_f64("rectangleTopLeftCornerRadius").unwrap_or(0.0);
        let tr = figma
            .get_f64("rectangleTopRightCornerRadius")
            .unwrap_or(0.0);
        let br = figma
            .get_f64("rectangleBottomRightCornerRadius")
            .unwrap_or(0.0);
        let bl = figma
            .get_f64("rectangleBottomLeftCornerRadius")
            .unwrap_or(0.0);
        if tl == tr && tr == br && br == bl {
            return if tl > 0.0 {
                Some(CornerRadius::Uniform(tl))
            } else {
                None
            };
        }
        return Some(CornerRadius::PerCorner([tl, tr, br, bl]));
    }
    match figma.get_f64("cornerRadius") {
        Some(cr) if cr > 0.0 => Some(CornerRadius::Uniform(cr)),
        _ => None,
    }
}

/// Assemble the shared base props spread into every PenNode.
pub fn common_props(figma: &FigValue, id: String) -> PenNodeBase {
    let (x, y) = extract_position(figma);
    let (flip_x, flip_y) = extract_flip(figma);
    PenNodeBase {
        id,
        name: figma
            .get_str("name")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        role: None,
        explain: None,
        x: Some(x),
        y: Some(y),
        rotation: extract_rotation(figma),
        opacity: figma
            .get_f64("opacity")
            .filter(|o| *o < 1.0)
            .map(NumberOrExpression::Number),
        enabled: None,
        visible: None,
        locked: if figma.get_bool("locked") == Some(true) {
            Some(true)
        } else {
            None
        },
        flip_x,
        flip_y,
        theme: None,
    }
}

/// Width as a `SizingBehavior` — fixed pixels in preserve mode, the
/// auto-layout resolution otherwise.
pub fn resolve_width(
    figma: &FigValue,
    parent_stack_mode: Option<&str>,
    ctx: &ConversionContext,
) -> SizingBehavior {
    match ctx.layout_mode {
        FigLayoutMode::Preserve => SizingBehavior::Number(
            figma
                .get("size")
                .and_then(|s| s.get_f64("x"))
                .unwrap_or(100.0),
        ),
        FigLayoutMode::OpenPencil => map_width_sizing(figma, parent_stack_mode),
    }
}

/// Height as a `SizingBehavior` — see [`resolve_width`].
pub fn resolve_height(
    figma: &FigValue,
    parent_stack_mode: Option<&str>,
    ctx: &ConversionContext,
) -> SizingBehavior {
    match ctx.layout_mode {
        FigLayoutMode::Preserve => SizingBehavior::Number(
            figma
                .get("size")
                .and_then(|s| s.get_f64("y"))
                .unwrap_or(100.0),
        ),
        FigLayoutMode::OpenPencil => map_height_sizing(figma, parent_stack_mode),
    }
}

/// Recursively rescale a SYMBOL subtree to fit a differently-sized
/// instance. Only translation (`m02`/`m12`) + size + stroke weight
/// scale; the rotation/scale matrix cells are untouched.
pub fn scale_tree_children(children: &[TreeNode], sx: f64, sy: f64) -> Vec<TreeNode> {
    if (sx - 1.0).abs() < 0.001 && (sy - 1.0).abs() < 0.001 {
        return children.to_vec();
    }
    let stroke_scale = sx.min(sy);
    children
        .iter()
        .map(|child| {
            let mut figma = child.figma.clone();
            if let Some(t) = figma.get("transform").cloned() {
                let mut t = t;
                let m02 = t.get_f64("m02").unwrap_or(0.0) * sx;
                let m12 = t.get_f64("m12").unwrap_or(0.0) * sy;
                t.set("m02", FigValue::Float(m02 as f32));
                t.set("m12", FigValue::Float(m12 as f32));
                figma.set("transform", t);
            }
            if let Some(size) = figma.get("size").cloned() {
                let mut size = size;
                let x = size.get_f64("x").unwrap_or(0.0) * sx;
                let y = size.get_f64("y").unwrap_or(0.0) * sy;
                size.set("x", FigValue::Float(x as f32));
                size.set("y", FigValue::Float(y as f32));
                figma.set("size", size);
            }
            if let Some(sw) = figma.get_f64("strokeWeight") {
                if stroke_scale < 0.99 {
                    figma.set(
                        "strokeWeight",
                        FigValue::Float(round2(sw * stroke_scale) as f32),
                    );
                }
            }
            TreeNode {
                figma,
                children: scale_tree_children(&child.children, sx, sy),
            }
        })
        .collect()
}

/// Detect which decoded blobs are images (by magic bytes), keyed by
/// blob index.
pub fn collect_image_blobs(blobs: &[BlobOrString]) -> HashMap<u32, Vec<u8>> {
    let mut map = HashMap::new();
    for (i, blob) in blobs.iter().enumerate() {
        if let BlobOrString::Bytes(b) = blob {
            if b.len() > 8 {
                let is_image = (b[0] == 0x89 && b[1] == 0x50)
                    || (b[0] == 0xFF && b[1] == 0xD8)
                    || (b[0] == 0x47 && b[1] == 0x49)
                    || (b[0] == 0x52 && b[1] == 0x49);
                if is_image {
                    map.insert(i as u32, b.clone());
                }
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: Vec<(&str, FigValue)>) -> FigValue {
        FigValue::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn matrix(m00: f32, m01: f32, m02: f32, m10: f32, m11: f32, m12: f32) -> FigValue {
        obj(vec![
            ("m00", FigValue::Float(m00)),
            ("m01", FigValue::Float(m01)),
            ("m02", FigValue::Float(m02)),
            ("m10", FigValue::Float(m10)),
            ("m11", FigValue::Float(m11)),
            ("m12", FigValue::Float(m12)),
        ])
    }

    #[test]
    fn axis_aligned_position_is_translation() {
        let n = obj(vec![("transform", matrix(1.0, 0.0, 30.0, 0.0, 1.0, 40.0))]);
        assert_eq!(extract_position(&n), (30.0, 40.0));
    }

    #[test]
    fn rotation_extracted_in_degrees() {
        // 90° rotation: m00=0, m10=1.
        let n = obj(vec![("transform", matrix(0.0, -1.0, 0.0, 1.0, 0.0, 0.0))]);
        assert_eq!(extract_rotation(&n), Some(90.0));
    }

    #[test]
    fn horizontal_flip_detected() {
        // m00 negative, det negative → flipX.
        let n = obj(vec![("transform", matrix(-1.0, 0.0, 0.0, 0.0, 1.0, 0.0))]);
        assert_eq!(extract_flip(&n), (Some(true), None));
    }

    #[test]
    fn corner_radius_uniform_collapse() {
        let n = obj(vec![("cornerRadius", FigValue::Float(8.0))]);
        assert!(matches!(
            map_corner_radius(&n),
            Some(CornerRadius::Uniform(8.0))
        ));
    }

    #[test]
    fn js_round_is_floor_half_up() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-0.5), 0.0);
        assert_eq!(js_round(-1.5), -1.0);
    }
}
