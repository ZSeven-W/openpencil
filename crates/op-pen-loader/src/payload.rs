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
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
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
