//! `.pen` / `.op` payload DTO.
//!
//! [`DocPayload`] is the layout-resolved intermediate the canonical
//! loader produces: `adapter::pen_document_to_payload` runs each
//! page-root through jian-core's `LayoutEngine` and bakes the resolved
//! AABBs + paint fields into this DTO. The `LayoutScene` builder
//! (`layout_scene.rs`) then re-shapes it into a paint-only render
//! scene.
//!
//! Carved out of `openpencil-desktop/src/persistence.rs` so library
//! crates can reuse the canonical `.op` loader; the desktop binary's
//! `rfd` Save/Open dialogs + error dialogs stay desktop-side.

use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct DocPayload {
    pub version: u32,
    pub active_page_index: usize,
    pub pages: Vec<PagePayload>,
    /// Design-token table. `#[serde(default)]` so a payload built
    /// without variables still deserializes (empty table).
    #[serde(default)]
    pub var_table: crate::variables::VarTablePayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PagePayload {
    pub id: String,
    pub name: String,
    pub children: Vec<NodePayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodePayload {
    pub id: String,
    /// Original schema id (the string `id` in the `.op` file). Used
    /// to look up layout rects after jian-core's `LayoutEngine`
    /// computes them.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub schema_id: String,
    pub kind: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub fill: Option<[f32; 4]>,
    #[serde(default)]
    pub stroke: Option<StrokePayload>,
    #[serde(default)]
    pub text: Option<String>,
    /// CSS font-family stack. Text-only.
    #[serde(default)]
    pub font_family: String,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
    /// Node-level opacity (0.0..=1.0). Folded into resolved fill /
    /// stroke / text / gradient-stop alpha at scene-build time (see
    /// `layout_scene`), cumulative down the subtree. Defaults to 1.0.
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub corner_radius: f32,
    /// Ellipse arc start angle in degrees (`None` = full ellipse).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc_start_angle: Option<f32>,
    /// Ellipse arc sweep angle in degrees (`None` = full ellipse).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc_sweep_angle: Option<f32>,
    /// Ellipse donut-hole radius, 0.0..=1.0 fraction of the radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc_inner_radius: Option<f32>,
    /// Polygon side count. Defaults to the triangle parity value.
    #[serde(
        default = "default_polygon_sides",
        skip_serializing_if = "is_default_polygon_sides"
    )]
    pub polygon_sides: u32,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub fill_type: String,
    /// Resolved gradient body for the first fill when it is a
    /// `LinearGradient` / `RadialGradient`. `None` for solid /
    /// image fills; `fill` still carries the first-stop colour as
    /// a fallback for paint paths that don't grok gradients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gradient: Option<GradientPayload>,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    /// Path bezier anchors (absolute doc coords, handles resolved).
    /// Parallel to `points` for `Path` nodes; empty otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_anchors: Vec<AnchorPayload>,
    /// Whether a `Path` node's outline is closed (last anchor links
    /// back to the first).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub path_closed: bool,
    /// Preserved SVG path data for imported path nodes. Coordinates
    /// are local doc-px relative to the node origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub svg_path: Option<String>,
    /// Text size in doc-px. 0 = use the renderer's default 13 px.
    /// Text-only.
    #[serde(default)]
    pub font_size: f32,
    /// CSS-style font weight (100-900). 0 = default 400. Text-only.
    #[serde(default)]
    pub font_weight: u16,
    /// Line-height multiplier. 0 = renderer default. Text-only.
    #[serde(default)]
    pub line_height: f32,
    /// Extra letter spacing in doc-px. Text-only.
    #[serde(default)]
    pub letter_spacing: f32,
    /// Horizontal alignment keyword. Text-only.
    #[serde(default)]
    pub text_align: String,
    /// Vertical alignment keyword. Text-only.
    #[serde(default)]
    pub text_vertical_align: String,
    #[serde(default)]
    pub text_wrap: bool,
    /// Drop-shadow effects.
    #[serde(default)]
    pub effects: Vec<crate::effects::ShadowPayload>,
    /// Source URL (`data:image/...;base64,...` or file path) when the
    /// node is an `Image` — the canvas painter decodes the inline
    /// bytes and draws them with `RenderBackend::draw_image`. `None`
    /// for non-image nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_src: Option<String>,
    /// Image placement mode for `image_src` (`fill`, `fit`, `crop`,
    /// `tile`, `stretch`). `None` defaults to `fill`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_fit: Option<String>,
    /// Per-image adjustment values from image fills / image nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_adjustments: Option<ImageAdjustmentPayload>,
    #[serde(default)]
    pub children: Vec<NodePayload>,
}

fn default_polygon_sides() -> u32 {
    3
}

fn default_opacity() -> f32 {
    1.0
}

fn is_default_polygon_sides(value: &u32) -> bool {
    *value == 3
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrokePayload {
    pub color: [f32; 4],
    pub width: f32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ImageAdjustmentPayload {
    #[serde(default)]
    pub exposure: f32,
    #[serde(default)]
    pub contrast: f32,
    #[serde(default)]
    pub saturation: f32,
    #[serde(default)]
    pub temperature: f32,
    #[serde(default)]
    pub tint: f32,
    #[serde(default)]
    pub highlights: f32,
    #[serde(default)]
    pub shadows: f32,
}

/// One resolved gradient stop — offset 0.0..=1.0 + RGBA colour.
/// Mirrors `jian_ops_schema::GradientStop` but with the colour
/// hex pre-parsed into the same `[r,g,b,a]` array `NodePayload.fill`
/// uses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStopPayload {
    pub offset: f32,
    pub color: [f32; 4],
}

/// Layout-resolved gradient body for `NodePayload.gradient`.
/// Pre-parsed (colour hex → RGBA, opacity baked into the variant)
/// so the scene builder + canvas painter never re-walk the
/// canonical schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GradientPayload {
    Linear {
        /// Gradient angle in degrees, canonical `.op` convention:
        /// 0° = bottom→top, 90° = left→right, 180° = top→bottom
        /// (matches CSS `to-top`). The renderer subtracts 90° before
        /// projecting endpoints, so storing as authored keeps the
        /// scene wire-format equal to the file's `angle`.
        angle_deg: f32,
        opacity: f32,
        stops: Vec<GradientStopPayload>,
    },
    Radial {
        /// Centre x as a 0.0..=1.0 fraction of bounds width.
        cx: f32,
        /// Centre y as a 0.0..=1.0 fraction of bounds height.
        cy: f32,
        /// Outer radius as a 0.0..=1.0 fraction of `max(w, h)` —
        /// matches the TS renderer, so the same `.op` file paints at
        /// the same radial size on native + web + export.
        radius: f32,
        opacity: f32,
        stops: Vec<GradientStopPayload>,
    },
}

/// One path bezier anchor in absolute doc coords. `handle_in` /
/// `handle_out` are absolute control-point positions (already
/// resolved from the schema's anchor-relative deltas); `point_type`
/// is `0` corner / `1` mirrored / `2` independent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchorPayload {
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_in: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle_out: Option<[f32; 2]>,
    #[serde(default)]
    pub point_type: u8,
}

/// Wrapper around `jian_ops_schema::load_str` that retries with the
/// document's `version` field rewritten to "1.0" when the canonical
/// loader rejects on an unrecognised major (e.g. the TS app's
/// `version: "2.8"` for the legacy pre-Jian format). The schema is
/// intentionally strict on rejection so callers can opt in to
/// "best-effort parse"; the desktop's Open dialog is one of those
/// callers — better to surface partial geometry than refuse the
/// file outright.
pub fn load_canonical(
    src: &str,
) -> Result<
    jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>,
    jian_ops_schema::OpsSchemaError,
> {
    match load_str_or_deep(src) {
        Ok(loaded) => Ok(loaded),
        Err(jian_ops_schema::OpsSchemaError::UnsupportedFormatVersion { found, .. }) => {
            eprintln!(
                "[open] file version {} is outside the canonical schema's supported range; \
                 retrying as v1.0 (best-effort)",
                found
            );
            let mut value: serde_json::Value = serde_json::from_str(src)?;
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "version".to_string(),
                    serde_json::Value::String("1.0".to_string()),
                );
                obj.remove("formatVersion");
            }
            let patched = serde_json::to_string(&value)?;
            load_str_or_deep(&patched)
        }
        Err(other) => Err(other),
    }
}

fn load_str_or_deep(
    src: &str,
) -> Result<
    jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>,
    jian_ops_schema::OpsSchemaError,
> {
    if looks_deeply_nested(src) {
        return load_canonical_deep(src);
    }
    match jian_ops_schema::load_str(src) {
        Ok(loaded) => Ok(loaded),
        Err(jian_ops_schema::OpsSchemaError::Json(err)) if is_recursion_limit_error(&err) => {
            load_canonical_deep(src)
        }
        Err(other) => Err(other),
    }
}

fn looks_deeply_nested(src: &str) -> bool {
    let mut depth = 0usize;
    for byte in src.bytes() {
        match byte {
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > 120 {
                    return true;
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

fn is_recursion_limit_error(err: &serde_json::Error) -> bool {
    err.to_string().contains("recursion limit")
}

fn deserialize_deep<T: DeserializeOwned>(src: &str) -> Result<T, serde_json::Error> {
    let mut de = serde_json::Deserializer::from_str(src);
    de.disable_recursion_limit();
    let de = serde_stacker::Deserializer::new(&mut de);
    T::deserialize(de)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct DeepHeader {
    #[serde(default)]
    format_version: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    id: Option<IgnoredAny>,
    #[serde(default)]
    name: Option<IgnoredAny>,
    #[serde(default)]
    themes: Option<IgnoredAny>,
    #[serde(default)]
    variables: Option<IgnoredAny>,
    #[serde(default)]
    pages: Option<IgnoredAny>,
    #[serde(default)]
    children: Option<IgnoredAny>,
    #[serde(default)]
    app: Option<IgnoredAny>,
    #[serde(default)]
    routes: Option<IgnoredAny>,
    #[serde(default)]
    state: Option<IgnoredAny>,
    #[serde(default)]
    lifecycle: Option<IgnoredAny>,
    #[serde(default)]
    logic_modules: Option<IgnoredAny>,
    #[serde(flatten)]
    extra: BTreeMap<String, IgnoredAny>,
}

fn load_canonical_deep(
    src: &str,
) -> Result<
    jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>,
    jian_ops_schema::OpsSchemaError,
> {
    let header: DeepHeader = deserialize_deep(src)?;
    let version = header
        .format_version
        .as_deref()
        .or(header.version.as_deref());
    if !jian_ops_schema::version::supports(version) {
        return Err(jian_ops_schema::OpsSchemaError::UnsupportedFormatVersion {
            found: version.unwrap_or("<missing>").to_owned(),
            supported: jian_ops_schema::version::FORMAT_VERSION_CURRENT,
        });
    }

    let mut warnings = Vec::new();
    for field in header.extra.keys() {
        warnings.push(jian_ops_schema::LoadWarning::UnknownField {
            path: "$".to_owned(),
            field: field.to_owned(),
        });
    }
    if let Some(format_version) = header.format_version.as_deref() {
        let (major, _) = jian_ops_schema::version::parse(Some(format_version));
        if major > 1 {
            warnings.push(jian_ops_schema::LoadWarning::FutureFormatVersion {
                found: format_version.to_owned(),
                supported_max: jian_ops_schema::version::FORMAT_VERSION_CURRENT,
            });
        }
    }
    if header.logic_modules.is_some() {
        warnings.push(jian_ops_schema::LoadWarning::LogicModulesSkipped {
            reason: "Tier 3 WASM is not implemented in this build",
        });
    }

    let doc: jian_ops_schema::PenDocument = deserialize_deep(src)?;
    Ok(jian_ops_schema::LoadResult {
        value: doc,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::PenNode;

    fn nested_frame_doc(depth: usize) -> String {
        let mut src = String::from(r#"{"version":"0.8.0","children":["#);
        for i in 0..depth {
            src.push_str(&format!(
                r##"{{"type":"frame","id":"nest-{i:05}","name":"Nested Layer {i:05}","x":8,"y":6,"width":400,"height":220,"fill":[{{"type":"solid","color":"#ffffff20"}}],"stroke":{{"thickness":1,"fill":[{{"type":"solid","color":"#0088ff"}}]}},"children":["##
            ));
        }
        for _ in 0..depth {
            src.push_str("]}");
        }
        src.push_str("]}");
        src
    }

    fn children(node: &PenNode) -> Option<&[PenNode]> {
        match node {
            PenNode::Frame(n) => n.children.as_deref(),
            PenNode::Group(n) => n.children.as_deref(),
            PenNode::Rectangle(n) => n.children.as_deref(),
            _ => None,
        }
    }

    fn max_depth(nodes: &[PenNode]) -> usize {
        let mut max = 0;
        let mut stack: Vec<(&PenNode, usize)> = nodes.iter().map(|node| (node, 1)).collect();
        while let Some((node, depth)) = stack.pop() {
            max = max.max(depth);
            if let Some(children) = children(node) {
                stack.extend(children.iter().map(|child| (child, depth + 1)));
            }
        }
        max
    }

    #[test]
    fn load_canonical_accepts_5000_deep_visible_frame_tree() {
        std::thread::Builder::new()
            .name("deep-loader-test".into())
            .stack_size(512 * 1024 * 1024)
            .spawn(|| {
                let loaded = load_canonical(&nested_frame_doc(5_000)).expect("deep document loads");
                assert_eq!(max_depth(&loaded.value.children), 5_000);
                // `PenDocument` is a recursive enum; drop is recursive too.
                // This test is about the loader accepting the document.
                std::mem::forget(loaded);
            })
            .expect("spawn deep-loader-test")
            .join()
            .expect("deep-loader-test completed");
    }
}
