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

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

mod compatibility;
mod fast_load;
mod legacy_normalize;
mod paint_payloads;
mod video_alias;

// Re-exported so every `crate::payload::…` import path stays stable across
// the split.
pub use paint_payloads::{
    AnchorPayload, GradientPayload, GradientStopPayload, ImageAdjustmentPayload, ShaderPayload,
    ShaderUniformPayload, StrokePayload, TextRunPayload,
};
pub use video_alias::normalize_video_alias;

#[cfg(test)]
static THUMBNAIL_REGISTRY_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
fn lock_thumbnail_registry_for_test() -> std::sync::MutexGuard<'static, ()> {
    THUMBNAIL_REGISTRY_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub use compatibility::{
    write_normalized_source_with_current_schema, CanonicalLoad, CompatibilityReport,
    NormalizedWriteError,
};
use legacy_normalize::normalize_legacy_value;

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
    /// Canonical fill stack in paint order (first entry is topmost).
    /// The legacy single-fill fields below remain populated so older
    /// payload readers keep working; an absent stack means the payload
    /// predates multi-fill support and must use those legacy fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fill_layers: Vec<jian_ops_schema::style::PenFill>,
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
    /// Node-level blend operation. Unlike image/fill blend this composites the
    /// complete node output, including descendants.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blend_mode: Option<jian_ops_schema::style::BlendMode>,
    #[serde(default)]
    pub corner_radius: f32,
    /// Per-corner radii in top-left, top-right, bottom-right,
    /// bottom-left order. `corner_radius` remains the legacy maximum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corner_radii: Option<[f32; 4]>,
    /// Container clips its children to its bounds (canonical
    /// `clipContent`). The page builders also force this on for root
    /// frames, which clip like artboards (TS flattener parity).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clip_content: bool,
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
    /// Resolved native SkSL shader body for the first fill when it is a
    /// `Shader`. `None` for every other fill type. `fill` still carries
    /// the shader's fallback colour for paint paths that can't run the
    /// program.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shader: Option<ShaderPayload>,
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
    /// Legacy mask marker retained for payload compatibility. New payloads
    /// carry the exact operation in `mask_type`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_mask: bool,
    /// Pixel operation used to mask front-layer siblings. Unlike the legacy
    /// marker this is valid on every node kind, including container subtrees.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mask_type: Option<jian_ops_schema::node::MaskType>,
    /// Whether SVG path fills use the even-odd winding rule.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub even_odd_fill: bool,
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
    /// Node-level `fontStyle: italic`. Text-only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    /// Node-level underline decoration. Text-only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    /// Node-level strikethrough decoration. Text-only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strikethrough: bool,
    /// Per-segment style runs when the canonical text content is
    /// `TextContent::Styled`. Each run covers `text.len()` bytes of
    /// the flattened `text` in order; empty for plain text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_runs: Vec<TextRunPayload>,
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
    /// Gaussian layer-blur radius (doc px) when the node has a Figma
    /// "Layer blur" effect. Kept separate from `effects` (which is
    /// shadow-only) so the save format stays additive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_blur: Option<f32>,
    /// Gaussian backdrop-blur radius (doc px).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_blur: Option<f32>,
    /// Source URL (`data:image/...;base64,...` or file path) when the
    /// node is an `Image` — the canvas painter decodes the inline
    /// bytes and draws them with `RenderBackend::draw_image`. `None`
    /// for non-image nodes. Shared (`ImageSrc` = `Arc<str>`) with the
    /// canonical document so building this payload never copies the
    /// multi-MB data-URL bytes (serde still reads/writes a plain
    /// string).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_src: Option<jian_ops_schema::node::ImageSrc>,
    /// Video playback metadata carried by an image poster. The video source
    /// is never externalized into the image table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<jian_ops_schema::node::VideoMeta>,
    /// Image placement mode for `image_src` (`fill`, `fit`, `crop`,
    /// `tile`, `stretch`). `None` defaults to `fill`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_fit: Option<String>,
    /// Compositing mode for a canonical `ImageNode`. `None` is normal
    /// source-over painting and preserves older payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_blend_mode: Option<jian_ops_schema::style::BlendMode>,
    /// Figma image-fill affine transform in normalized UV coordinates.
    /// `[m00, m01, m02, m10, m11, m12]` maps a node-local unit point
    /// `(x, y)` to image UV as `(m00*x + m01*y + m02,
    /// m10*x + m11*y + m12)`. `None` keeps the placement-mode default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_transform: Option<[f32; 6]>,
    /// Authored source bitmap dimensions for Figma TILE paints. The renderer
    /// uses these instead of a memory-saving downsampled raster's dimensions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_original_size: Option<[f32; 2]>,
    /// Positive Figma TILE paint scale. `None` preserves the historical 1.0
    /// default and non-tile image modes ignore this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_tile_scale: Option<f32>,
    /// Per-image adjustment values from image fills / image nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_adjustments: Option<ImageAdjustmentPayload>,
    /// Composite-widget descriptor for the interactive node family
    /// (switch / checkbox / slider / progress / select / radio_group /
    /// text_input / text_area / number_input / tabs). The geometry +
    /// container style still land on the base `NodePayload` fields (the
    /// leaf widgets degrade to a `rect` / `text` `kind`), but this
    /// carries the extra props the design-surface painter needs to draw
    /// the recognizable static visual (track + knob, box + check, …).
    /// `None` for ordinary shapes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<WidgetPayload>,
    #[serde(default)]
    pub children: Vec<NodePayload>,
}

/// Static props for a composite widget node, harvested from the
/// canonical schema by `adapter.rs`. Sentinels: `None` numeric fields
/// fall back to the runtime defaults (`min=0`, `max=100`, `step=1`)
/// the painter mirrors from jian-core's `emit_widget_visual`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WidgetPayload {
    /// Widget family tag (`switch` / `checkbox` / `slider` / `progress`
    /// / `select` / `radio_group` / `text_input` / `text_area` /
    /// `number_input` / `tabs`).
    pub kind: String,
    /// On/off state for switch / checkbox.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// Numeric value for slider / progress / number_input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_num: Option<f32>,
    /// String value for select / radio_group / tabs / text inputs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_str: Option<String>,
    /// Placeholder text for inputs / select.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Leading / trailing lucide glyph for input widgets (Phase 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leading_icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trailing_icon: Option<String>,
    /// Adjacent label (checkbox).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Range minimum (slider / number_input). `None` = 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f32>,
    /// Range maximum (slider / progress / number_input). `None` = 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f32>,
    /// Range step. `None` = 1. Carried for completeness; the static
    /// visual doesn't quantize.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f32>,
    /// Progress uses a deterministic unknown-progress segment.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub indeterminate: bool,
    /// True when `cornerRadius` was authored, including explicit zero.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub corner_radius_authored: bool,
    /// `(value, label)` option rows for select / radio_group / tabs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<WidgetOption>,
}

/// One `value` + `label` option row for select / radio_group / tabs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WidgetOption {
    pub value: String,
    pub label: String,
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

/// Wrapper around `jian_ops_schema::load_str` that loads an unrecognised
/// legacy major (for example the TS app's `version: "2.8"`) as v1.0. The
/// schema stays strict for direct callers; this product loader deliberately
/// opts into best-effort compatibility and patches the schema-owned JSON DOM
/// before its version check, so a large legacy file is not parsed twice.
pub fn load_canonical(
    src: &str,
) -> Result<
    jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>,
    jian_ops_schema::OpsSchemaError,
> {
    // jian-ops-schema owns the one full JSON Value required for warnings,
    // image-table extraction, and typed conversion. Repair legacy fields in
    // that same tree: the old Value -> String -> Value path doubled parsing
    // and made large Figma conversions retain far more allocator pages.
    load_canonical_with_compatibility(src).map(|result| result.loaded)
}

/// Load a canonical document and report only explicit compatibility repairs
/// that make a user-approved current-schema rewrite safe.
pub fn load_canonical_with_compatibility(
    src: &str,
) -> Result<CanonicalLoad, jian_ops_schema::OpsSchemaError> {
    if let Some(loaded) = fast_load::try_load_preserve(src) {
        return Ok(CanonicalLoad {
            loaded,
            compatibility: CompatibilityReport::default(),
        });
    }
    if looks_deeply_nested(src) {
        return load_canonical_deep(src);
    }
    let mut best_effort_version = None;
    let mut normalized_legacy = false;
    let loaded = load_str_with_preprocess(src, |value| {
        normalized_legacy = normalize_legacy_value(value);
        best_effort_version = patch_unsupported_version(value);
    });
    if let Some(found) = best_effort_version.as_deref() {
        log_best_effort_version(found);
    }
    match loaded {
        Ok(loaded) => Ok(CanonicalLoad {
            loaded,
            compatibility: CompatibilityReport {
                normalized_legacy,
                patched_legacy_version: best_effort_version,
            },
        }),
        Err(jian_ops_schema::OpsSchemaError::Json(err)) if is_recursion_limit_error(&err) => {
            load_canonical_deep(src)
        }
        Err(other) => Err(other),
    }
}

fn load_str_with_preprocess(
    src: &str,
    preprocess: impl FnOnce(&mut serde_json::Value),
) -> Result<
    jian_ops_schema::LoadResult<jian_ops_schema::PenDocument>,
    jian_ops_schema::OpsSchemaError,
> {
    jian_ops_schema::compat::load_str_with_preprocess(
        src,
        jian_ops_schema::compat::LoadOptions::default(),
        preprocess,
    )
}

/// Patch an unsupported legacy major in the already-parsed DOM. The public
/// product loader has always retried these documents as v1.0; doing it before
/// the schema check preserves that behavior without parsing a large file twice.
fn patch_unsupported_version(value: &mut serde_json::Value) -> Option<String> {
    // `formatVersion` is the authoritative schema version. A future major
    // must remain unsupported: rewriting it through today's typed model could
    // silently discard fields. The historical best-effort rule applies only
    // to files that have no `formatVersion` and used the old product version
    // (for example `version: "2.8"`) as their sole version marker.
    if value
        .get("formatVersion")
        .and_then(serde_json::Value::as_str)
        .is_some()
    {
        return None;
    }
    let found = value.get("version").and_then(serde_json::Value::as_str);
    if jian_ops_schema::version::supports(found) {
        return None;
    }
    let found = found.unwrap_or("<missing>").to_owned();
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "version".to_string(),
            serde_json::Value::String("1.0".to_string()),
        );
        obj.remove("formatVersion");
    }
    Some(found)
}

fn log_best_effort_version(found: &str) {
    eprintln!(
        "[open] file version {} is outside the canonical schema's supported range; \
         loading as v1.0 (best-effort)",
        found
    );
}

fn looks_deeply_nested(src: &str) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in src.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
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
    let value = T::deserialize(serde_stacker::Deserializer::new(&mut de))?;
    de.end()?;
    Ok(value)
}

/// `serde_json::from_value` with the same stack protection as
/// [`deserialize_deep`] — the deep-document path parses to a `Value`
/// first (to inline the image table), and the typed parse of that
/// `Value` must survive the same nesting depth that routed us here.
fn deserialize_deep_value<T: DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, serde_json::Error> {
    use serde::de::IntoDeserializer;
    let de = serde_stacker::Deserializer::new(value.into_deserializer());
    T::deserialize(de)
}

fn deep_load_warnings(raw: &serde_json::Value) -> Vec<jian_ops_schema::LoadWarning> {
    use jian_ops_schema::LoadWarning;
    use serde_json::Value;

    let mut warnings = Vec::new();
    if let Value::Object(map) = raw {
        for field in map.keys() {
            if !is_known_top_level_field(field) {
                warnings.push(LoadWarning::UnknownField {
                    path: "$".to_owned(),
                    field: field.to_owned(),
                });
            }
        }
    }

    let format_version = raw.get("formatVersion").and_then(Value::as_str);
    let legacy_version = raw.get("version").and_then(Value::as_str);
    if let Some(found) = format_version {
        let current =
            jian_ops_schema::version::parse(Some(jian_ops_schema::version::FORMAT_VERSION_CURRENT));
        let declared = jian_ops_schema::version::parse(Some(found));
        if declared > current {
            warnings.push(LoadWarning::FutureFormatVersion {
                found: found.to_owned(),
                supported_max: jian_ops_schema::version::FORMAT_VERSION_CURRENT,
            });
        }
    }
    if raw.get("responsive").and_then(Value::as_bool) == Some(true) {
        let declared = format_version.or(legacy_version);
        if jian_ops_schema::version::parse(declared) < (1, 2) {
            warnings.push(LoadWarning::ResponsiveBelowMinor {
                declared: declared.unwrap_or_default().to_owned(),
            });
        }
        collect_viewport_writes_deep(raw, &mut warnings);
    }
    if raw.get("logicModules").is_some() {
        warnings.push(LoadWarning::LogicModulesSkipped {
            reason: "Tier 3 WASM is not implemented in this build",
        });
    }
    warnings
}

fn is_known_top_level_field(field: &str) -> bool {
    matches!(
        field,
        "formatVersion"
            | "responsive"
            | "version"
            | "id"
            | "name"
            | "themes"
            | "variables"
            | "pages"
            | "children"
            | "app"
            | "routes"
            | "state"
            | "lifecycle"
            | "logicModules"
            | "designMd"
            | "conversion"
            | "editorMeta"
            | "images"
            | "imageThumbs"
    )
}

fn collect_viewport_writes_deep(
    root: &serde_json::Value,
    warnings: &mut Vec<jian_ops_schema::LoadWarning>,
) {
    use serde_json::Value;

    let mut stack = vec![(root, "$".to_owned())];
    while let Some((value, path)) = stack.pop() {
        match value {
            Value::Object(map) => {
                for (key, child) in map.iter().rev() {
                    let child_path = format!("{path}.{key}");
                    if key.starts_with("$viewport") {
                        warnings.push(jian_ops_schema::LoadWarning::ViewportWrite {
                            path: child_path.clone(),
                        });
                    }
                    stack.push((child, child_path));
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate().rev() {
                    stack.push((child, format!("{path}[{index}]")));
                }
            }
            _ => {}
        }
    }
}

fn load_canonical_deep(src: &str) -> Result<CanonicalLoad, jian_ops_schema::OpsSchemaError> {
    // Always use one stack-protected DOM for the deep compatibility pipeline.
    // Legacy repair and version patching must not depend on the presence of an
    // image table, and consuming that same DOM avoids a second full parse.
    let mut raw: serde_json::Value = deserialize_deep(src)?;
    let normalized_legacy = normalize_legacy_value(&mut raw);
    reject_unsupported_format_version(&raw)?;
    let patched_legacy_version = patch_unsupported_version(&mut raw);
    if let Some(found) = patched_legacy_version.as_deref() {
        log_best_effort_version(found);
    }
    let warnings = deep_load_warnings(&raw);
    let pending_thumbs = jian_ops_schema::image_thumbs::take_pending_from_document(&mut raw);
    let table = jian_ops_schema::image_table::take_image_table(&mut raw);
    let mut doc = jian_ops_schema::node::image_src::intern::with_load_scope(table, || {
        deserialize_deep_value(raw)
    })?;
    jian_ops_schema::image_thumbs::attach_to_document(&mut doc, pending_thumbs);
    Ok(CanonicalLoad {
        loaded: jian_ops_schema::LoadResult {
            value: doc,
            warnings,
        },
        compatibility: CompatibilityReport {
            normalized_legacy,
            patched_legacy_version,
        },
    })
}

fn reject_unsupported_format_version(
    value: &serde_json::Value,
) -> Result<(), jian_ops_schema::OpsSchemaError> {
    let Some(found) = value
        .get("formatVersion")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(());
    };
    if jian_ops_schema::version::supports(Some(found)) {
        return Ok(());
    }
    Err(jian_ops_schema::OpsSchemaError::UnsupportedFormatVersion {
        found: found.to_owned(),
        supported: jian_ops_schema::version::FORMAT_VERSION_CURRENT,
    })
}

#[cfg(test)]
#[path = "payload/load_tests.rs"]
mod load_tests;

#[cfg(test)]
#[path = "payload/deep_image_thumb_tests.rs"]
mod deep_image_thumb_tests;
