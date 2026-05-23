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

use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub rotation: f32,
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
    /// Text size in doc-px. 0 = use the renderer's default 13 px.
    /// Text-only.
    #[serde(default)]
    pub font_size: f32,
    /// CSS-style font weight (100-900). 0 = default 400. Text-only.
    #[serde(default)]
    pub font_weight: u16,
    #[serde(default)]
    pub text_wrap: bool,
    /// Drop-shadow effects.
    #[serde(default)]
    pub effects: Vec<crate::effects::ShadowPayload>,
    #[serde(default)]
    pub children: Vec<NodePayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrokePayload {
    pub color: [f32; 4],
    pub width: f32,
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
    match jian_ops_schema::load_str(src) {
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
            jian_ops_schema::load_str(&patched)
        }
        Err(other) => Err(other),
    }
}
