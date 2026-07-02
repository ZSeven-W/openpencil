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

#[derive(Debug, Serialize, Deserialize)]
pub struct StrokePayload {
    pub color: [f32; 4],
    pub width: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sides: Option<[f32; 4]>,
}

/// One styled text segment, flattened in document order. Sentinels
/// mirror the node-level fields: `0.0` font size / `0` weight / `None`
/// fill = inherit the node's value. `italic` / `underline` /
/// `strikethrough` are RESOLVED against the node level already (a
/// segment without an override inherits the node's flag).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRunPayload {
    pub text: String,
    #[serde(default)]
    pub font_size: f32,
    #[serde(default)]
    pub font_weight: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strikethrough: bool,
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
    /// Uniform-grid mesh gradient (v1). `colors` is a row-major
    /// `rows`×`cols` lattice of pre-resolved RGBA values (length ==
    /// `rows * cols`); vertex `(r, c)` lives at `colors[r * cols + c]`.
    /// Opacity is carried separately and folded by the painter (parity
    /// with how the Linear / Radial variants thread `opacity`).
    Mesh {
        rows: u32,
        cols: u32,
        colors: Vec<[f32; 4]>,
        opacity: f32,
    },
}

/// One resolved SkSL shader uniform — name plus a concrete float vector
/// (length 1 = float, 2/3/4 = vec*). A `color` uniform is pre-expanded
/// into a 4-float premultiplied-RGBA `vec4` here so the scene builder +
/// painter never re-walk the canonical schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderUniformPayload {
    pub name: String,
    pub values: Vec<f32>,
}

/// Layout-resolved native SkSL shader body for `NodePayload.shader`.
/// `sksl` is the RAW (untrusted) source; uniforms are pre-resolved.
/// `fallback` is the `[r,g,b,a]` solid colour painted when a host can't
/// compile the program (first `color` uniform, else mid-gray) — kept
/// alongside `NodePayload.fill` so the degradation path always has a
/// visible colour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderPayload {
    pub sksl: String,
    #[serde(default)]
    pub uniforms: Vec<ShaderUniformPayload>,
    pub opacity: f32,
    pub fallback: [f32; 4],
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
    // Two TS-vs-Rust leniency gaps repaired on load: (1) LLM-authored image
    // nodes carry `imageSearchQuery` / `imagePrompt` but no `src`, yet jian's
    // `ImageNode.src` is a required `String`; (2) older TS files write a
    // node's `fill` as a bare color string (`"#1A1D2E"`) where jian expects
    // `Vec<PenFill>`. TS's runtime `JSON.parse` enforced neither, so these
    // files opened in the web app — repair both so the strict canonical
    // loader accepts them. A document past serde's default recursion limit
    // fails the `Value` parse and falls through unchanged to the deep loader.
    let normalized = normalize_legacy_doc(src);
    let src = normalized.as_deref().unwrap_or(src);
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

/// Repair the two known TS-vs-Rust leniency gaps (image node missing `src`,
/// PenNode `fill` written as a bare color string) by walking the parsed
/// JSON. Returns the patched JSON only when something changed, so untouched
/// files skip re-serialization; a parse failure (e.g. a document past
/// serde's default recursion limit) returns `None` so the caller loads the
/// original through the deep path.
fn normalize_legacy_doc(src: &str) -> Option<String> {
    let mut value: serde_json::Value = serde_json::from_str(src).ok()?;
    let mut changed = false;
    // Pencil design-kit `.pen` files reference `$--radius-*` number variables
    // from `cornerRadius`; the canonical `CornerRadius` enum can only hold a
    // number / `[f64; 4]`, so collect the document's number-variable table up
    // front to resolve those refs to concrete radii during the walk.
    let radius_vars = collect_number_variables(&value);
    normalize_node_value(&mut value, &radius_vars, &mut changed);
    if changed {
        serde_json::to_string(&value).ok()
    } else {
        None
    }
}

/// Harvest `variables.<name> = { type: "number", value: N | [{value, theme}] }`
/// into a `$<name>` → first-concrete-number map. Used to resolve
/// `cornerRadius: "$--radius-pill"` legacy refs (Pencil design kits) into the
/// number the canonical `CornerRadius` enum can actually hold. Theme-keyed
/// arrays collapse to their first entry's value — corner radius is rarely
/// theme-varied, and a constant fallback beats refusing the whole file.
fn collect_number_variables(root: &serde_json::Value) -> std::collections::HashMap<String, f64> {
    use serde_json::Value;
    let mut out = std::collections::HashMap::new();
    let Some(vars) = root.get("variables").and_then(Value::as_object) else {
        return out;
    };
    for (name, def) in vars {
        if def.get("type").and_then(Value::as_str) != Some("number") {
            continue;
        }
        let resolved = match def.get("value") {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::Array(entries)) => entries
                .first()
                .and_then(|e| e.get("value"))
                .and_then(Value::as_f64),
            _ => None,
        };
        if let Some(v) = resolved {
            out.insert(format!("${name}"), v);
        }
    }
    out
}

/// Iterative (explicit stack, not recursion — deep trees must not blow the
/// stack) walk that repairs known legacy `.pen` / `.op` shapes the strict
/// canonical schema rejects but the TS runtime tolerated:
///
/// - PenNode `fill` written as a bare color/`$ref` string → wrap into the
///   `Vec<PenFill>` jian expects.
/// - Image node missing the required `src`.
/// - Pencil legacy `type: "icon"` → canonical `icon_font`.
/// - A `stroke` written as a bare color string → wrap into a `PenStroke`.
/// - A stroke object's `fill` string (no `type` key on the stroke object) →
///   wrap into `Vec<PenFill>` (same shape as node fill).
/// - `cornerRadius` written as a `$ref` string (or array of them) → resolved
///   to the concrete number(s) the `CornerRadius` enum can hold.
///
/// Type-less objects whose `fill` is legitimately a `String`
/// (`StyledTextSegment`) are left untouched — the wrap is gated on the object
/// being a PenNode (`type` key) or a stroke (`thickness` key).
fn normalize_node_value(
    root: &mut serde_json::Value,
    number_vars: &std::collections::HashMap<String, f64>,
    changed: &mut bool,
) {
    use serde_json::Value;
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        match node {
            Value::Object(map) => {
                let is_pen_node = map.contains_key("type");
                if is_pen_node {
                    // Copy the kind decisions to owned bools so the mutating
                    // inserts below don't conflict with the borrow on `map`.
                    let kind = map.get("type").and_then(Value::as_str);
                    let is_image = kind == Some("image");
                    let is_icon = kind == Some("icon");
                    let is_prompt = kind == Some("prompt");
                    let is_path = kind == Some("path");
                    if is_image && !map.contains_key("src") {
                        map.insert("src".to_string(), Value::String(String::new()));
                        *changed = true;
                    }
                    // Pencil's `prompt` node is an AI-annotation card with no
                    // canonical equivalent → degrade to a `text` node. Its
                    // `content` field already matches `TextNode.content`; just
                    // drop the editor-only `model` field.
                    if is_prompt {
                        map.insert("type".to_string(), Value::String("text".to_string()));
                        map.remove("model");
                        *changed = true;
                    }
                    // Pencil's legacy `icon` node is jian's `icon_font`: the
                    // glyph lives under `icon` (→ `iconFontName`) and the font
                    // family under `library` (→ `iconFontFamily`).
                    if is_icon {
                        map.insert("type".to_string(), Value::String("icon_font".to_string()));
                        if let Some(glyph) = map.remove("icon") {
                            map.insert("iconFontName".to_string(), glyph);
                        }
                        if let Some(family) = map.remove("library") {
                            map.insert("iconFontFamily".to_string(), family);
                        }
                        *changed = true;
                    }
                    // Pencil stores a `path` node's flattened outline as an SVG
                    // string under `geometry`; jian's `PathNode` reads it from
                    // `d`. Without this remap the geometry is dropped and the
                    // path renders empty — status-bar icons, logos, and any
                    // vector art authored as a path vanish.
                    if is_path {
                        if let Some(geom) = map.get("geometry").cloned() {
                            if geom.is_string() && !map.contains_key("d") {
                                map.insert("d".to_string(), geom);
                                *changed = true;
                            }
                        }
                    }
                    // A PenNode `fill` written as a bare string / single object /
                    // legacy `type:"color"` array → canonical `Vec<PenFill>`.
                    normalize_fill(map, changed);
                    // `cornerRadius` as a `$ref` string / array of them →
                    // resolve to the number(s) the `CornerRadius` enum holds.
                    normalize_corner_radius(map, number_vars, changed);
                    // A `stroke` written as a bare color string → wrap into a
                    // minimal `PenStroke { thickness: 1, fill: [...] }`.
                    if let Some(color) = map.get("stroke").and_then(|s| match s {
                        Value::String(s) => Some(s.clone()),
                        _ => None,
                    }) {
                        map.insert(
                            "stroke".to_string(),
                            serde_json::json!({
                                "thickness": 1,
                                "fill": [{ "type": "solid", "color": color }],
                            }),
                        );
                        *changed = true;
                    }
                } else if map.contains_key("thickness") {
                    // Stroke object (has `thickness`, no `type`): its `fill`
                    // is the same normalization target as a node's.
                    normalize_fill(map, changed);
                }
                // Spacing fields (`padding` / `gap` / `margin`) may be numeric
                // arrays with an embedded `$ref` string (`[8, "$spacing/3"]`).
                // The `Padding` enum's array arms are all-number, so resolve any
                // ref element to its number. A bare-string padding (`"$x"`) is
                // left alone — the `Expression(String)` arm accepts it.
                for key in ["padding", "gap", "margin"] {
                    resolve_spacing_ref_array(map, key, number_vars, changed);
                }
                stack.extend(map.values_mut());
            }
            Value::Array(items) => stack.extend(items.iter_mut()),
            _ => {}
        }
    }
}

/// Normalize a `fill` into the canonical `Vec<PenFill>` jian expects. Handles
/// the legacy Pencil shapes the strict schema rejects:
/// - bare color/`$ref` string (`"#1A1D2E"` / `"$--sidebar-border"`)
/// - a single fill *object* (`{ "type": "color", "color": "...", "enabled": false }`)
///   rather than a one-element array
/// - the `type: "color"` discriminant (jian's solid arm is `"solid"`)
/// - an `enabled: false` flag → the fill is dropped (disabled in the source)
///
/// No-op when `fill` is absent or already a clean canonical array.
fn normalize_fill(map: &mut serde_json::Map<String, serde_json::Value>, changed: &mut bool) {
    use serde_json::Value;
    let Some(fill) = map.get("fill") else {
        return;
    };
    match fill {
        Value::String(s) => {
            let color = s.clone();
            map.insert(
                "fill".to_string(),
                serde_json::json!([{ "type": "solid", "color": color }]),
            );
            *changed = true;
        }
        Value::Object(_) => {
            // A single fill written as an object → array of (maybe) one,
            // after legacy-discriminant + enabled normalization.
            let mut one = fill.clone();
            let mut dummy = false;
            let arr: Vec<Value> = match normalize_fill_entry(&mut one, &mut dummy) {
                Some(v) => vec![v],
                None => vec![],
            };
            map.insert("fill".to_string(), Value::Array(arr));
            *changed = true;
        }
        Value::Array(items) => {
            // Already an array — but its elements may still carry the legacy
            // `type: "color"` discriminant or an `enabled: false` flag.
            let mut local_changed = false;
            let mut out = Vec::with_capacity(items.len());
            for item in items.clone() {
                let mut entry = item;
                if let Some(v) = normalize_fill_entry(&mut entry, &mut local_changed) {
                    out.push(v);
                }
            }
            if local_changed {
                map.insert("fill".to_string(), Value::Array(out));
                *changed = true;
            }
        }
        _ => {}
    }
}

/// Normalize one PenFill value. Returns `None` when the fill is explicitly
/// `enabled: false` (a disabled fill is dropped from the array). Rewrites the
/// legacy `type: "color"` discriminant to `"solid"` and strips the non-schema
/// `enabled` key. Sets `changed` when it touches anything.
fn normalize_fill_entry(
    entry: &mut serde_json::Value,
    changed: &mut bool,
) -> Option<serde_json::Value> {
    use serde_json::Value;
    let Value::Object(obj) = entry else {
        return Some(entry.clone());
    };
    if obj.get("enabled") == Some(&Value::Bool(false)) {
        *changed = true;
        return None;
    }
    if obj.remove("enabled").is_some() {
        *changed = true;
    }
    match obj.get("type").and_then(Value::as_str) {
        Some("color") => {
            obj.insert("type".to_string(), Value::String("solid".to_string()));
            *changed = true;
        }
        Some("gradient") => {
            normalize_gradient_fill(obj, changed);
        }
        _ => {}
    }
    Some(Value::Object(obj.clone()))
}

/// Rewrite Pencil's generic `type: "gradient"` fill into the canonical
/// `linear_gradient` / `radial_gradient` PenFill. Source shape:
/// `{ type: "gradient", gradientType: "linear"|"radial", rotation, colors: [{ color, position }], size }`.
/// Canonical shape: `{ type: "linear_gradient", angle, stops: [{ offset, color }] }`.
fn normalize_gradient_fill(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    changed: &mut bool,
) {
    use serde_json::Value;
    let radial = obj.get("gradientType").and_then(Value::as_str) == Some("radial");
    obj.insert(
        "type".to_string(),
        Value::String(
            if radial {
                "radial_gradient"
            } else {
                "linear_gradient"
            }
            .to_string(),
        ),
    );
    obj.remove("gradientType");
    // `rotation` (degrees) → `angle`; only for the linear arm (radial ignores it).
    if let Some(rot) = obj.remove("rotation") {
        if !radial {
            obj.insert("angle".to_string(), rot);
        }
    }
    obj.remove("size");
    // `colors: [{ color, position }]` → `stops: [{ offset, color }]`.
    if let Some(Value::Array(colors)) = obj.remove("colors") {
        let stops: Vec<Value> = colors
            .into_iter()
            .filter_map(|c| {
                let o = c.as_object()?;
                let color = o.get("color").and_then(Value::as_str)?.to_string();
                let offset = o.get("position").and_then(Value::as_f64).unwrap_or(0.0);
                Some(serde_json::json!({ "offset": offset, "color": color }))
            })
            .collect();
        obj.insert("stops".to_string(), Value::Array(stops));
    } else if !obj.contains_key("stops") {
        // A gradient with no stops is illegal; seed an empty array so the
        // canonical loader gets a valid (if degenerate) gradient body.
        obj.insert("stops".to_string(), Value::Array(vec![]));
    }
    *changed = true;
}

/// Resolve `$ref` strings embedded in a numeric spacing array
/// (`padding`/`gap`/`margin`: `[8, "$spacing/3"]`) to their concrete numbers.
/// No-op unless the value is an array containing at least one `$ref` string —
/// a bare-string spacing is left for the schema's `Expression` arm.
fn resolve_spacing_ref_array(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    number_vars: &std::collections::HashMap<String, f64>,
    changed: &mut bool,
) {
    use serde_json::Value;
    let Some(Value::Array(items)) = map.get(key) else {
        return;
    };
    if !items.iter().any(Value::is_string) {
        return;
    }
    let resolved: Vec<Value> = items
        .iter()
        .map(|v| match v {
            Value::String(s) => {
                serde_json::json!(number_vars.get(s).copied().unwrap_or(0.0))
            }
            other => other.clone(),
        })
        .collect();
    map.insert(key.to_string(), Value::Array(resolved));
    *changed = true;
}

/// Resolve a `cornerRadius` that is a `$ref` string, or an array of
/// `$ref`/number entries, into the number / `[f64; 4]` the canonical
/// `CornerRadius` enum accepts. A ref with no matching number variable falls
/// back to `0.0` so the file still loads (an unresolved radius is cosmetic).
fn normalize_corner_radius(
    map: &mut serde_json::Map<String, serde_json::Value>,
    number_vars: &std::collections::HashMap<String, f64>,
    changed: &mut bool,
) {
    use serde_json::Value;
    let resolve_scalar = |v: &Value| -> Option<f64> {
        match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => Some(number_vars.get(s).copied().unwrap_or(0.0)),
            _ => None,
        }
    };
    match map.get("cornerRadius") {
        Some(Value::String(s)) => {
            let n = number_vars.get(s).copied().unwrap_or(0.0);
            map.insert("cornerRadius".to_string(), serde_json::json!(n));
            *changed = true;
        }
        Some(Value::Array(entries)) => {
            // Only rewrite if at least one entry is a `$ref` string (a plain
            // numeric `[f64; 4]` is already valid; leave it alone).
            if entries.iter().any(|e| e.is_string()) {
                let nums: Vec<f64> = entries.iter().filter_map(&resolve_scalar).collect();
                // The enum's array arm is exactly `[f64; 4]`. Pad/truncate so a
                // 1- or 2-value Pencil shorthand still lands in a legal shape.
                let four = match nums.len() {
                    0 => [0.0; 4],
                    1 => [nums[0]; 4],
                    n if n >= 4 => [nums[0], nums[1], nums[2], nums[3]],
                    _ => {
                        let mut a = [0.0; 4];
                        for (i, v) in nums.iter().enumerate() {
                            a[i] = *v;
                        }
                        a
                    }
                };
                map.insert("cornerRadius".to_string(), serde_json::json!(four));
                *changed = true;
            }
        }
        _ => {}
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

    #[test]
    fn image_node_without_src_loads_via_injected_empty_src() {
        // LLM-authored image node: `imageSearchQuery` / `imagePrompt` but no
        // `src`. jian's `ImageNode.src` is required, so this must be repaired
        // on load rather than rejected (the web app opened these files fine).
        let doc = r#"{"version":"0.8.0","children":[{"id":"hero","type":"image","name":"Hero","width":120,"height":80,"imageSearchQuery":"pizza","imagePrompt":"a hot pizza"}]}"#;
        let loaded = load_canonical(doc).expect("image node without src must load");
        assert_eq!(loaded.value.children.len(), 1, "the image node should load");
    }

    #[test]
    fn node_fill_as_color_string_is_wrapped_into_solid_fill() {
        // Older TS files write a PenNode `fill` as a bare color string where
        // jian expects `Vec<PenFill>`. Must be wrapped, not rejected. The
        // type-less `StyledTextSegment.fill` (also a string) stays a string.
        let doc = r##"{"version":"0.8.0","children":[{"id":"bg","type":"frame","name":"BG","x":0,"y":0,"width":100,"height":100,"fill":"#1A1D2E","children":[]}]}"##;
        let loaded = load_canonical(doc).expect("string fill must load");
        assert_eq!(loaded.value.children.len(), 1, "the frame should load");
    }
}
