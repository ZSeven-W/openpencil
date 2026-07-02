//! `EditorState` → [`LayoutScene`] builder.
//!
//! Produces the paint-only, layout-resolved render scene that the
//! `CanvasViewport` painter walks.
//!
//! The flex layout pass is NOT re-implemented here. `EditorState.doc`
//! is a `PenDocument`; [`pen_document_to_payload`] runs each page-root
//! through jian-core's taffy `LayoutEngine` + `jian_skia::SkiaMeasure`
//! (see `adapter.rs`) and bakes the resolved absolute AABBs — plus
//! every paint field — into a layout-resolved [`DocPayload`]. This
//! builder reuses that resolved payload and re-shapes its `NodePayload`
//! tree into [`SceneNode`]s, dropping all editor state (selection /
//! chat / history / ui) and resolving variable `$ref` fills against
//! the editor's variables + active theme.
//!
//! So the resolved geometry a `LayoutScene` carries is bit-identical
//! to what `pen_document_to_payload` bakes — there is one layout pass
//! and one set of resolved rects. The builder no longer routes through
//! the shell-core `Document` model: `DocPayload` already carries the
//! resolved geometry + paint fields, and `apply_payload`'s only
//! transforms on them are lossless format conversions (colour array →
//! struct, kind / fill-type string → enum).

use jian_scene::layout_scene::NodeKind;
use jian_scene::layout_scene::{
    stable_image_source_id, DropShadow, Effect, LayoutScene, SceneFillType, SceneGradient,
    SceneGradientStop, SceneImageFit, SceneNode, ScenePage, SceneShader, SceneShaderUniform,
    SceneStroke, SceneTextAlign, SceneTextRun, SceneTextVerticalAlign, SceneWidget,
    SceneWidgetOption,
};
use op_editor_core::render_backend::Color;
use op_editor_core::scene_vars::VariableTable;

use crate::editor_state_var_table;
use crate::payload::{
    DocPayload, GradientPayload, GradientStopPayload, NodePayload, ShaderPayload, StrokePayload,
};

/// Build a paint-only [`LayoutScene`] from an editor state.
///
/// Runs the same jian `LayoutEngine` + `SkiaMeasure` flex pass that
/// the canonical `.op` loader uses (via [`pen_document_to_payload`]),
/// resolves variable `$ref` fills / strokes against the editor's
/// variables + active theme (via [`editor_state_var_table`]), and
/// re-shapes the resolved node tree into a render scene that carries
/// NO editor state.
pub fn editor_state_to_layout_scene(state: &op_editor_core::EditorState) -> LayoutScene {
    // Render-time document preparation (TS parity: the flattener
    // expands component instances, then `resolveNodeForCanvas` lands
    // every `$token` as a concrete value — fills / strokes / shadows
    // / opacity / gap / padding / font-weight / text content). Refs
    // expand FIRST so the instance subtrees resolve their tokens
    // too; loaded documents render without any transient editor
    // cache, and numeric tokens reach the flex solver as real
    // numbers instead of unparsed expressions. Each pass clones the
    // tree, so both early-out via cheap detector walks — the common
    // ref-free / token-free document pays no clone at all.
    let mut prepared = std::borrow::Cow::Borrowed(&state.doc);
    if op_editor_core::ref_resolve::document_has_refs(&prepared) {
        prepared = std::borrow::Cow::Owned(op_editor_core::ref_resolve::resolve_refs_for_canvas(
            &prepared,
        ));
    }
    if op_editor_core::variables_resolve::document_has_tokens(&prepared) {
        prepared = std::borrow::Cow::Owned(
            op_editor_core::variables_resolve::resolve_document_for_canvas(
                &prepared,
                &state.ui.variables.active_theme,
            ),
        );
    }
    // The layout-resolved payload — flex layout already baked into
    // every `NodePayload`'s AABB by jian-core's `LayoutEngine`. This
    // is the reusable layout-resolution core; it never touches the
    // shell-core `Document` model.
    let payload: DocPayload = if state.editor_ui.preserve_authored_geometry {
        crate::adapter::pen_document_to_payload_preserving_geometry(&prepared).payload
    } else {
        crate::adapter::pen_document_to_payload(&prepared).payload
    };
    // Variables + active theme + the `fill_refs` / `stroke_refs`
    // caches the editor holds. `editor_state_var_table` folds the
    // persisted definitions and the transient `EditorState.ui`
    // selection / caches together.
    let var_table: VariableTable = editor_state_var_table(state);

    LayoutScene {
        pages: payload
            .pages
            .iter()
            .map(|page| ScenePage {
                id: page.id.clone(),
                name: page.name.clone(),
                children: page
                    .children
                    .iter()
                    .map(|n| node_payload_to_scene(n, &var_table, 1.0))
                    .collect(),
            })
            .collect(),
        // Follow the editor's active page (`EditorState.ui`) so the
        // canvas switches when the user picks a page in the LayerPanel.
        // `pen_document_to_payload` hardcodes the payload's index to 0,
        // so the live editor state — not the payload — is the source
        // of truth here; clamp into range against the page count.
        active_page_index: state
            .ui
            .active_page_index
            .min(payload.pages.len().saturating_sub(1)),
    }
}

/// Build a [`LayoutScene`] from a bare [`PenDocument`] + active theme +
/// active page index — the document-only path (no [`EditorState`]).
///
/// Runs the SAME render-time ref + token resolution and flex layout as
/// [`editor_state_to_layout_scene`]; Canvas Preview uses this to render
/// its prepared + promoted runtime document. Tokens resolve against
/// `active_theme`; with no transient editor fill/stroke-ref caches the
/// var table comes straight from the document's `variables` / `themes`
/// — sufficient because the resolved doc already carries concrete
/// colours, so `node_payload_to_scene`'s `$ref` lookups all miss and
/// fall back to the node's own (now literal) fill.
///
/// [`EditorState`]: op_editor_core::EditorState
pub fn pen_document_to_layout_scene(
    doc: &jian_ops_schema::PenDocument,
    active_theme: &std::collections::BTreeMap<String, String>,
    active_page_index: usize,
) -> LayoutScene {
    let mut prepared = std::borrow::Cow::Borrowed(doc);
    if op_editor_core::ref_resolve::document_has_refs(&prepared) {
        prepared = std::borrow::Cow::Owned(op_editor_core::ref_resolve::resolve_refs_for_canvas(
            &prepared,
        ));
    }
    if op_editor_core::variables_resolve::document_has_tokens(&prepared) {
        prepared = std::borrow::Cow::Owned(
            op_editor_core::variables_resolve::resolve_document_for_canvas(&prepared, active_theme),
        );
    }
    let payload: DocPayload = crate::adapter::pen_document_to_payload(&prepared).payload;
    let mut var_table = crate::adapter::build_var_table(&prepared);
    var_table.active_theme = active_theme.clone();
    LayoutScene {
        pages: payload
            .pages
            .iter()
            .map(|page| ScenePage {
                id: page.id.clone(),
                name: page.name.clone(),
                children: page
                    .children
                    .iter()
                    .map(|n| node_payload_to_scene(n, &var_table, 1.0))
                    .collect(),
            })
            .collect(),
        active_page_index: active_page_index.min(payload.pages.len().saturating_sub(1)),
    }
}

/// Convert one resolved [`NodePayload`] into a [`SceneNode`].
///
/// Geometry is copied straight through — `pen_document_to_payload`
/// already resolved it. Variable `$ref` fills / strokes are resolved
/// here so the scene carries only concrete colours; a registered ref
/// wins over the node's authored colour, mirroring the canvas
/// painter's `var_table.fill_for(id).or(node.fill)`.
fn node_payload_to_scene(
    node: &NodePayload,
    var_table: &VariableTable,
    parent_opacity: f32,
) -> SceneNode {
    use op_editor_core::render_backend::{Point2D, Rect};
    let node_id = op_editor_core::NodeId::new(node.id.clone());
    // Node-level opacity composites multiplicatively down the tree
    // (a 0.5 frame dims its 0.5 child to 0.25). We bake it into the
    // resolved fill / stroke / gradient alpha rather than painting a
    // group layer: every shape here is leaf-painted, so per-shape
    // alpha matches a group-opacity layer except when a node's own
    // sub-shapes overlap — which the current scene model never emits.
    let cum_opacity = (parent_opacity * node.opacity).clamp(0.0, 1.0);
    let bounds = Rect {
        origin: Point2D::new(node.x, node.y),
        size: Point2D::new(node.w, node.h),
    };
    let children: Vec<SceneNode> = node
        .children
        .iter()
        .map(|c| node_payload_to_scene(c, var_table, cum_opacity))
        .collect();
    let aggregate_bounds_cache = SceneNode::compute_aggregate_bounds(bounds, &children);
    SceneNode {
        id: node.id.clone(),
        kind: str_to_kind(&node.kind),
        bounds,
        aggregate_bounds_cache,
        // Carried so the image painter can dim rasters (which have no
        // colour to bake opacity into); fill/stroke/gradient/shadow
        // already have `cum_opacity` folded into their alpha above.
        opacity: cum_opacity,
        rotation: node.rotation,
        flip_x: node.flip_x,
        flip_y: node.flip_y,
        corner_radius: node.corner_radius,
        clip_content: node.clip_content,
        // Paint-time `$ref` resolution: a registered fill ref wins,
        // else the node's own fill. Same precedence as the canvas
        // painter's `node_fill` helper.
        fill: var_table
            .fill_for(&node_id)
            .or_else(|| node.fill.map(array_to_color))
            .map(|c| mul_alpha(c, cum_opacity)),
        fill_type: str_to_scene_fill_type(&node.fill_type),
        gradient: node
            .gradient
            .as_ref()
            .map(|g| scale_gradient_opacity(payload_gradient_to_scene(g), cum_opacity)),
        shader: node
            .shader
            .as_ref()
            .map(|s| payload_shader_to_scene(s, cum_opacity)),
        stroke: if is_status_bar_shell_stroke(node) {
            // The scene path (editor canvas + render-shots) bypasses the
            // adapter's `legacy_payload_repair`, so an iPhone status-bar
            // shell ("Time"/"Levels") authored with a no-fill stroke would
            // paint a phantom black box around the clock / signal cluster —
            // invisible in Pencil. Drop the stroke the same way here.
            None
        } else {
            node.stroke.as_ref().map(|s| {
                let mut st = scene_stroke(s, &node_id, var_table);
                st.color = mul_alpha(st.color, cum_opacity);
                st
            })
        },
        text: node.text.clone(),
        text_runs: text_runs_to_scene(&node.text_runs, cum_opacity),
        font_family: node.font_family.clone(),
        font_size: node.font_size,
        font_weight: node.font_weight,
        italic: node.italic,
        underline: node.underline,
        strikethrough: node.strikethrough,
        line_height: node.line_height,
        letter_spacing: node.letter_spacing,
        text_align: text_align_to_scene(&node.text_align),
        text_vertical_align: text_vertical_align_to_scene(&node.text_vertical_align),
        text_wrap: node.text_wrap,
        points: node
            .points
            .iter()
            .map(|p| Point2D::new(p[0], p[1]))
            .collect(),
        path_anchors: node.path_anchors.iter().map(anchor_to_scene).collect(),
        path_closed: node.path_closed,
        svg_path: node.svg_path.clone(),
        arc_start_angle: node.arc_start_angle,
        arc_sweep_angle: node.arc_sweep_angle,
        arc_inner_radius: node.arc_inner_radius,
        polygon_sides: node.polygon_sides.clamp(3, 100),
        image_src: node.image_src.clone(),
        image_src_id: node
            .image_src
            .as_deref()
            .map(stable_image_source_id)
            .unwrap_or(0),
        image_fit: image_fit_to_scene(node.image_fit.as_deref()),
        image_adjustments: image_adjustments_to_scene(node.image_adjustments),
        effects: crate::effects::effects_from_payload_ref(&node.effects)
            .into_iter()
            .map(|e| scale_effect_opacity(e, cum_opacity))
            .collect(),
        hidden: node.hidden,
        locked: node.locked,
        // Composite-widget props pass through untouched (no `$ref`
        // resolution needed — they're already concrete after the
        // adapter harvested them from the schema). Drives the
        // design-surface static visual painter.
        widget: node.widget.as_ref().map(widget_payload_to_scene),
        children,
    }
}

/// Convert a payload [`WidgetPayload`] into the paint-only
/// [`SceneWidget`]. Plain field copy — option rows map 1:1.
fn widget_payload_to_scene(w: &crate::payload::WidgetPayload) -> SceneWidget {
    SceneWidget {
        kind: w.kind.clone(),
        checked: w.checked,
        value_num: w.value_num,
        value_str: w.value_str.clone(),
        placeholder: w.placeholder.clone(),
        leading_icon: w.leading_icon.clone(),
        trailing_icon: w.trailing_icon.clone(),
        label: w.label.clone(),
        min: w.min,
        max: w.max,
        step: w.step,
        options: w
            .options
            .iter()
            .map(|o| SceneWidgetOption {
                value: o.value.clone(),
                label: o.label.clone(),
            })
            .collect(),
    }
}

/// Map payload text runs onto scene runs: each segment's `text` length
/// becomes a byte range into the node's flattened string (the payload
/// flattens segments in order, so ranges are cumulative). Per-run fill
/// colours get the node's cumulative opacity folded into their alpha —
/// same treatment as the node-level fill.
fn text_runs_to_scene(
    runs: &[crate::payload::TextRunPayload],
    cum_opacity: f32,
) -> Vec<SceneTextRun> {
    let mut start = 0usize;
    runs.iter()
        .map(|run| {
            let end = start + run.text.len();
            let scene = SceneTextRun {
                start,
                end,
                font_size: run.font_size,
                font_weight: run.font_weight,
                fill: run
                    .fill
                    .map(array_to_color)
                    .map(|c| mul_alpha(c, cum_opacity)),
                italic: run.italic,
                underline: run.underline,
                strikethrough: run.strikethrough,
            };
            start = end;
            scene
        })
        .collect()
}

/// Fold node opacity into an effect's colour. A drop shadow is part
/// of the node's own paint, so node opacity dims it alongside the
/// fill (a 30%-opacity node casts a 30%-strength shadow).
fn scale_effect_opacity(e: Effect, k: f32) -> Effect {
    match e {
        Effect::DropShadow(s) => Effect::DropShadow(DropShadow {
            color: mul_alpha(s.color, k),
            ..s
        }),
    }
}

/// Multiply a colour's alpha by `k` (node-opacity folding).
fn mul_alpha(c: Color, k: f32) -> Color {
    Color {
        a: (c.a * k).clamp(0.0, 1.0),
        ..c
    }
}

/// Fold node opacity into a gradient by scaling its alpha multiplier
/// only. The backend folds `opacity` into every stop colour when it
/// builds the shader (`gradient_color_arrays`), so scaling the stops
/// here too would dim the gradient twice. Leave stops at their
/// authored alpha; the single `opacity` multiplier carries node
/// opacity.
fn scale_gradient_opacity(g: SceneGradient, k: f32) -> SceneGradient {
    match g {
        SceneGradient::Linear {
            angle_deg,
            opacity,
            stops,
        } => SceneGradient::Linear {
            angle_deg,
            opacity: (opacity * k).clamp(0.0, 1.0),
            stops,
        },
        SceneGradient::Radial {
            cx,
            cy,
            radius,
            opacity,
            stops,
        } => SceneGradient::Radial {
            cx,
            cy,
            radius,
            opacity: (opacity * k).clamp(0.0, 1.0),
            stops,
        },
        SceneGradient::Mesh {
            rows,
            cols,
            colors,
            opacity,
        } => SceneGradient::Mesh {
            rows,
            cols,
            colors,
            opacity: (opacity * k).clamp(0.0, 1.0),
        },
    }
}

fn text_align_to_scene(value: &str) -> SceneTextAlign {
    match value {
        "center" => SceneTextAlign::Center,
        "right" => SceneTextAlign::Right,
        "justify" => SceneTextAlign::Justify,
        _ => SceneTextAlign::Left,
    }
}

fn text_vertical_align_to_scene(value: &str) -> SceneTextVerticalAlign {
    match value {
        "middle" => SceneTextVerticalAlign::Middle,
        "bottom" => SceneTextVerticalAlign::Bottom,
        _ => SceneTextVerticalAlign::Top,
    }
}

fn image_fit_to_scene(value: Option<&str>) -> SceneImageFit {
    match value {
        Some("fit") => SceneImageFit::Fit,
        Some("crop") => SceneImageFit::Crop,
        Some("tile") => SceneImageFit::Tile,
        Some("stretch") => SceneImageFit::Stretch,
        _ => SceneImageFit::Fill,
    }
}

fn image_adjustments_to_scene(
    value: Option<crate::payload::ImageAdjustmentPayload>,
) -> op_editor_core::render_backend::ImageAdjustments {
    let Some(value) = value else {
        return op_editor_core::render_backend::ImageAdjustments::default();
    };
    op_editor_core::render_backend::ImageAdjustments {
        exposure: value.exposure,
        contrast: value.contrast,
        saturation: value.saturation,
        temperature: value.temperature,
        tint: value.tint,
        highlights: value.highlights,
        shadows: value.shadows,
    }
}

/// Convert a payload path anchor into a scene anchor.
fn anchor_to_scene(a: &crate::payload::AnchorPayload) -> jian_scene::layout_scene::SceneAnchor {
    use jian_scene::layout_scene::{SceneAnchor, ScenePointType};
    use op_editor_core::render_backend::Point2D;
    SceneAnchor {
        pos: Point2D::new(a.x, a.y),
        handle_in: a.handle_in.map(|h| Point2D::new(h[0], h[1])),
        handle_out: a.handle_out.map(|h| Point2D::new(h[0], h[1])),
        point_type: match a.point_type {
            1 => ScenePointType::Mirrored,
            2 => ScenePointType::Independent,
            _ => ScenePointType::Corner,
        },
    }
}

/// Resolve a payload stroke into a scene stroke. The `$ref` stroke
/// resolution parallels the fill path.
fn scene_stroke(
    s: &StrokePayload,
    node_id: &op_editor_core::NodeId,
    var_table: &VariableTable,
) -> SceneStroke {
    SceneStroke {
        color: var_table
            .stroke_color_for(node_id)
            .unwrap_or_else(|| array_to_color(s.color)),
        width: s.width,
        sides: s.sides,
    }
}

/// An iPhone status-bar layout shell ("Time" / "Levels") authored with a
/// stroke but no fill — Pencil paints nothing, so the no-fill stroke (which
/// resolves to opaque black) must not draw a phantom box. Mirrors
/// `legacy_payload_repair::is_legacy_status_bar_shell` on the resolved
/// `NodePayload` the scene path carries (no canonical `PenNode` here). Width
/// is intentionally unconstrained — the shells are `fill_container`, so a
/// wider status bar computes a larger width than a fixed-width sample.
fn is_status_bar_shell_stroke(node: &NodePayload) -> bool {
    // A thin status-bar row: the authored 22 px computes to ~22-23 here
    // (stroke / rounding), so match a small range rather than an exact 22.
    // The no-fill + stroke + non-empty-children gates already exclude the
    // inner "Time" text glyph node (which is filled, strokeless, leaf).
    node.stroke.is_some()
        && node.fill.is_none()
        && matches!(node.name.as_str(), "Time" | "Levels")
        && !node.children.is_empty()
        && (18.0..=28.0).contains(&node.h)
}

/// `[r, g, b, a]` payload colour → shell-core `Color`. Lossless;
/// the same conversion `apply_payload` runs on the `Document` path.
fn array_to_color(a: [f32; 4]) -> Color {
    Color {
        r: a[0],
        g: a[1],
        b: a[2],
        a: a[3],
    }
}

/// `NodePayload.kind` string → shell-core `NodeKind`. Mirrors
/// `payload::str_to_kind` so the scene's per-kind paint dispatch
/// matches the `Document` path exactly.
fn str_to_kind(s: &str) -> NodeKind {
    match s {
        "frame" => NodeKind::Frame,
        "group" => NodeKind::Group,
        "rect" => NodeKind::Rect,
        "ellipse" => NodeKind::Ellipse,
        "polygon" => NodeKind::Polygon,
        "line" => NodeKind::Line,
        "text" => NodeKind::Text,
        "path" => NodeKind::Path,
        other => NodeKind::Other(other.to_string()),
    }
}

/// Convert a [`GradientPayload`] into the paint-only
/// [`SceneGradient`]. Each stop's `[r,g,b,a]` is unpacked into a
/// [`Color`]; the body's opacity rides through unchanged so the
/// painter can fold it into the per-stop alpha at draw time.
fn payload_gradient_to_scene(g: &GradientPayload) -> SceneGradient {
    match g {
        GradientPayload::Linear {
            angle_deg,
            opacity,
            stops,
        } => SceneGradient::Linear {
            angle_deg: *angle_deg,
            opacity: *opacity,
            stops: stops.iter().map(stop_to_scene).collect(),
        },
        GradientPayload::Radial {
            cx,
            cy,
            radius,
            opacity,
            stops,
        } => SceneGradient::Radial {
            cx: *cx,
            cy: *cy,
            radius: *radius,
            opacity: *opacity,
            stops: stops.iter().map(stop_to_scene).collect(),
        },
        GradientPayload::Mesh {
            rows,
            cols,
            colors,
            opacity,
        } => SceneGradient::Mesh {
            rows: *rows,
            cols: *cols,
            colors: colors.iter().map(|c| array_to_color(*c)).collect(),
            opacity: *opacity,
        },
    }
}

fn stop_to_scene(s: &GradientStopPayload) -> SceneGradientStop {
    SceneGradientStop {
        offset: s.offset,
        color: array_to_color(s.color),
    }
}

/// Convert a [`ShaderPayload`] into the paint-only [`SceneShader`].
/// The SkSL source + pre-resolved uniforms ride through unchanged;
/// node opacity (`k`) folds into the shader's own opacity multiplier.
/// The `fallback` `[r,g,b,a]` becomes the visible solid colour for
/// backends that can't run the program.
fn payload_shader_to_scene(s: &ShaderPayload, k: f32) -> SceneShader {
    SceneShader {
        sksl: s.sksl.clone(),
        uniforms: s
            .uniforms
            .iter()
            .map(|u| SceneShaderUniform {
                name: u.name.clone(),
                values: u.values.clone(),
            })
            .collect(),
        opacity: (s.opacity * k).clamp(0.0, 1.0),
        fallback: array_to_color(s.fallback),
    }
}

/// `NodePayload.fill_type` string → scene `SceneFillType`. Mirrors
/// `payload::str_to_fill_type` followed by `fill_type_to_scene`.
fn str_to_scene_fill_type(s: &str) -> SceneFillType {
    match s {
        "linear" => SceneFillType::LinearGradient,
        "radial" => SceneFillType::RadialGradient,
        "mesh" => SceneFillType::MeshGradient,
        "shader" => SceneFillType::Shader,
        "image" => SceneFillType::Image,
        _ => SceneFillType::Solid,
    }
}

#[cfg(test)]
#[path = "layout_scene_tests.rs"]
mod tests;
