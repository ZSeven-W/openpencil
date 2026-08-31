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
    stable_image_source_id, DropShadow, Effect, LayoutScene, SceneFillLayer, SceneFillType,
    SceneGradient, SceneGradientStop, SceneImageFit, SceneNode, ScenePage, SceneShader,
    SceneShaderUniform, SceneTextAlign, SceneTextRun, SceneTextVerticalAlign, SceneWidget,
    SceneWidgetOption,
};
use op_editor_core::render_backend::{Color, ImageBlendMode};
use op_editor_core::scene_vars::VariableTable;

use crate::payload::{
    DocPayload, GradientPayload, GradientStopPayload, NodePayload, ShaderPayload,
};

// Keep the legacy test module's historical local name; implementation lives in `editor_scene`.
#[cfg(test)]
use crate::editor_scene::editor_state_to_layout_scene;

mod stroke;
use stroke::{is_status_bar_shell_stroke, is_unpainted_widget_stroke, scene_stroke};

/// Build a [`LayoutScene`] from a bare [`PenDocument`], theme, and active page index.
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
    pen_document_to_layout_scene_with_geometry_mode(doc, active_theme, active_page_index, false)
}

/// Build a [`LayoutScene`] from a bare [`PenDocument`] while selecting how
/// node geometry is resolved.
///
/// `preserve_authored_geometry` is intended for Preserve-mode Figma imports:
/// those documents already carry numeric parent-local positions and sizes, so
/// running flex layout again would both waste work and move overlapping
/// children away from their authored coordinates. Passing `false` retains the
/// historical behavior of [`pen_document_to_layout_scene`].
pub fn pen_document_to_layout_scene_with_geometry_mode(
    doc: &jian_ops_schema::PenDocument,
    active_theme: &std::collections::BTreeMap<String, String>,
    active_page_index: usize,
    preserve_authored_geometry: bool,
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
    let payload: DocPayload = if preserve_authored_geometry {
        crate::adapter::pen_document_to_payload_preserving_geometry(&prepared).payload
    } else {
        crate::adapter::pen_document_to_payload(&prepared).payload
    };
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

/// Build the Canvas Preview [`LayoutScene`]: paint tree from the
/// PROMOTED document (so legacy `role=input` frames render as live
/// widgets), geometry from the UNPROMOTED document laid out exactly as
/// the design canvas lays it out — honoring
/// `preserve_authored_geometry` for Figma Preserve imports. Node
/// positions in the returned scene therefore match the design canvas
/// by construction; only the paint representation differs.
///
/// Both documents must already be ref/token-resolved (the preview
/// session prepares them before promotion), so no resolution passes
/// run here; the var table still folds the document's variables +
/// `active_theme` for any `$ref` fill lookups.
pub fn pen_document_to_layout_scene_for_preview(
    paint_doc: &jian_ops_schema::PenDocument,
    layout_doc: &jian_ops_schema::PenDocument,
    preserve_authored_geometry: bool,
    active_theme: &std::collections::BTreeMap<String, String>,
    active_page_index: usize,
) -> LayoutScene {
    let payload: DocPayload = crate::adapter::pen_documents_to_payload_for_preview(
        paint_doc,
        layout_doc,
        preserve_authored_geometry,
    )
    .payload;
    let mut var_table = crate::adapter::build_var_table(paint_doc);
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
pub(crate) fn node_payload_to_scene(
    node: &NodePayload,
    var_table: &VariableTable,
    inherited_paint_opacity: f32,
) -> SceneNode {
    use op_editor_core::render_backend::{Point2D, Rect};
    let node_id = op_editor_core::NodeId::new(node.id.clone());
    let mask_type = node.mask_type.or_else(|| {
        node.is_mask
            .then_some(jian_ops_schema::node::MaskType::Alpha)
    });
    // A mask source is composited into siblings that already carry the common
    // ancestor opacity. Baking that ancestor into the mask as well would
    // multiply it a second time through DstIn. Reset only at the mask root;
    // Its own local opacity remains represented on either the direct paint or
    // the source's isolation layer.
    let inherited_paint_opacity = if mask_type.is_some() {
        1.0
    } else {
        inherited_paint_opacity
    };
    let local_opacity = node.opacity.clamp(0.0, 1.0);
    let blend_mode = blend_mode_to_scene(node.blend_mode.as_ref());
    // A translucent subtree must apply its local alpha after its own paint and
    // children have assembled; otherwise overlapping children accumulate alpha
    // independently. Non-Normal node blending already requires the same
    // isolation layer, so it carries local opacity there even for leaves. Keep
    // ordinary leaves on the direct-paint path to avoid allocating a layer.
    let isolates_output =
        blend_mode != ImageBlendMode::Normal || (!node.children.is_empty() && local_opacity < 1.0);
    let paint_opacity = if isolates_output {
        inherited_paint_opacity
    } else {
        (inherited_paint_opacity * local_opacity).clamp(0.0, 1.0)
    };
    let composite_opacity = if isolates_output { local_opacity } else { 1.0 };
    let bounds = Rect {
        origin: Point2D::new(node.x, node.y),
        size: Point2D::new(node.w, node.h),
    };
    let children: Vec<SceneNode> = node
        .children
        .iter()
        .map(|c| node_payload_to_scene(c, var_table, paint_opacity))
        .collect();
    let aggregate_bounds_cache = SceneNode::compute_aggregate_bounds(bounds, &children);
    let variable_fill = var_table.fill_for(&node_id);
    SceneNode {
        id: node.id.clone(),
        kind: str_to_kind(&node.kind),
        bounds,
        aggregate_bounds_cache,
        // Carried so the image and canonical fill-stack painters can apply
        // direct paint opacity; legacy fill/stroke/gradient/shadow fields
        // already have `paint_opacity` folded into their alpha below.
        opacity: paint_opacity,
        composite_opacity,
        blend_mode,
        rotation: node.rotation,
        flip_x: node.flip_x,
        flip_y: node.flip_y,
        corner_radius: node.corner_radius,
        corner_radii: node.corner_radii,
        clip_content: node.clip_content,
        // Paint-time `$ref` resolution: a registered fill ref wins,
        // else the node's own fill. Same precedence as the canvas
        // painter's `node_fill` helper.
        fill: variable_fill
            .or_else(|| node.fill.map(array_to_color))
            .map(|c| mul_alpha(c, paint_opacity)),
        fill_layers: fill_layers_to_scene(&node.fill_layers, variable_fill, [node.w, node.h]),
        fill_type: str_to_scene_fill_type(&node.fill_type),
        gradient: node
            .gradient
            .as_ref()
            .map(|g| scale_gradient_opacity(payload_gradient_to_scene(g), paint_opacity)),
        shader: node
            .shader
            .as_ref()
            .map(|s| payload_shader_to_scene(s, paint_opacity, [node.w, node.h])),
        stroke: if is_unpainted_widget_stroke(node, &node_id, var_table) {
            // A first-class widget's stroke is its *inactive track / border*
            // role paint, not a literal outline. When the author declared a
            // stroke with no resolvable colour, handing the painter a
            // fabricated opaque black turned every such control into a black
            // track / border; dropping it lets
            // `resolve_authored_widget_visual` fall back to its role defaults
            // (derived-from-fill or the legacy #D1D5DB track, and a
            // borderless select) exactly as an unstroked control does.
            None
        } else if is_status_bar_shell_stroke(node) {
            // The scene path (editor canvas + render-shots) bypasses the
            // adapter's `legacy_payload_repair`, so an iPhone status-bar
            // shell ("Time"/"Levels") authored with a no-fill stroke would
            // paint a phantom black box around the clock / signal cluster —
            // invisible in Pencil. Drop the stroke the same way here.
            None
        } else {
            node.stroke.as_ref().map(|s| {
                let mut st = scene_stroke(s, &node_id, var_table);
                st.color = mul_alpha(st.color, paint_opacity);
                st
            })
        },
        text: node.text.clone(),
        text_runs: text_runs_to_scene(&node.text_runs, paint_opacity),
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
        is_mask: node.is_mask || node.mask_type.is_some(),
        mask_type,
        even_odd_fill: node.even_odd_fill,
        svg_path: node.svg_path.clone(),
        arc_start_angle: node.arc_start_angle,
        arc_sweep_angle: node.arc_sweep_angle,
        arc_inner_radius: node.arc_inner_radius,
        polygon_sides: node.polygon_sides.clamp(3, 100),
        image_src: node.image_src.as_ref().map(|s| s.as_arc()),
        image_src_id: node
            .image_src
            .as_deref()
            .map(stable_image_source_id)
            .unwrap_or(0),
        image_fit: image_fit_to_scene(node.image_fit.as_deref()),
        image_blend_mode: blend_mode_to_scene(node.image_blend_mode.as_ref()),
        image_transform: node.image_transform,
        image_original_size: image_original_size_to_scene(node.image_original_size),
        image_tile_scale: image_tile_scale_to_scene(node.image_tile_scale),
        image_adjustments: image_adjustments_to_scene(node.image_adjustments),
        effects: crate::effects::effects_from_payload_ref(&node.effects)
            .into_iter()
            .chain(crate::effects::blur_effect_from_payload(node.layer_blur))
            .chain(crate::effects::background_blur_effect_from_payload(
                node.background_blur,
            ))
            .map(|e| scale_effect_opacity(e, paint_opacity))
            .collect(),
        hidden: node.hidden,
        locked: node.locked,
        // Widget props are already concrete after the adapter harvested them;
        // no `$ref` resolution is needed here.
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
        indeterminate: w.indeterminate,
        corner_radius_authored: w.corner_radius_authored,
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
/// colours get the node's direct paint opacity folded into their alpha — the
/// same treatment as the node-level fill. Isolated group alpha is applied by
/// the painter after all runs and descendants have assembled.
fn text_runs_to_scene(
    runs: &[crate::payload::TextRunPayload],
    paint_opacity: f32,
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
                    .map(|c| mul_alpha(c, paint_opacity)),
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
        // Blur (layer or backdrop) has no colour to fold node opacity into.
        Effect::Blur(b) => Effect::Blur(b),
        Effect::BackgroundBlur { radius } => Effect::BackgroundBlur { radius },
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

/// Resolve every canonical fill without collapsing the stack. Canonical arrays
/// are front-to-back; the scene preserves that order and painters reverse it.
/// Layer alpha stays authored; `SceneNode::opacity` owns direct paint opacity
/// and any isolated local group alpha lives on `SceneNode::composite_opacity`.
fn fill_layers_to_scene(
    fills: &[jian_ops_schema::style::PenFill],
    variable_fill: Option<Color>,
    size: [f32; 2],
) -> Vec<SceneFillLayer> {
    use jian_ops_schema::style::PenFill;

    if fills.is_empty() {
        return variable_fill
            .map(|color| SceneFillLayer::Solid {
                color,
                blend_mode: ImageBlendMode::Normal,
            })
            .into_iter()
            .collect();
    }

    fills
        .iter()
        .enumerate()
        .filter_map(|(index, fill)| {
            let blend_mode = fill_blend_mode_to_scene(fill);
            // Override only the primary paint; retain its compositing metadata
            // and every layer behind it.
            if index == 0 {
                if let Some(color) = variable_fill {
                    return Some(SceneFillLayer::Solid {
                        color: mul_alpha(color, fill_opacity(fill)),
                        blend_mode,
                    });
                }
            }
            let fallback = || {
                crate::style_payload::fill_fallback_color(fill)
                    .map(array_to_color)
                    .map(|color| SceneFillLayer::Solid { color, blend_mode })
            };
            match fill {
                PenFill::Solid(_) => fallback(),
                PenFill::LinearGradient(_)
                | PenFill::RadialGradient(_)
                | PenFill::MeshGradient(_) => crate::style_payload::gradient_payload(fill)
                    .map(|gradient| payload_gradient_to_scene(&gradient))
                    .map(|gradient| SceneFillLayer::Gradient {
                        gradient,
                        blend_mode,
                    })
                    .or_else(fallback),
                PenFill::Shader(_) => crate::style_payload::shader_payload(fill)
                    .map(|shader| payload_shader_to_scene(&shader, 1.0, size))
                    .map(|shader| SceneFillLayer::Shader { shader, blend_mode })
                    .or_else(fallback),
                PenFill::Image(body) if !body.url.trim().is_empty() => {
                    let fit = crate::style_payload::image_fill_mode_to_payload(body.mode.as_ref());
                    Some(SceneFillLayer::Image {
                        src: body.url.as_arc(),
                        src_id: stable_image_source_id(body.url.as_ref()),
                        fit: image_fit_to_scene(Some(&fit)),
                        transform: body
                            .transform
                            .as_ref()
                            .map(|m| [m.m00, m.m01, m.m02, m.m10, m.m11, m.m12]),
                        original_size: image_original_size_to_scene(
                            body.original_size
                                .as_ref()
                                .map(|size| [size.width, size.height]),
                        ),
                        tile_scale: image_tile_scale_to_scene(body.tile_scale),
                        adjustments: image_adjustments_to_scene(
                            crate::style_payload::image_fill_adjustments(body),
                        ),
                        opacity: body.opacity.unwrap_or(1.0).clamp(0.0, 1.0),
                        blend_mode,
                    })
                }
                PenFill::Image(_) => None,
            }
        })
        .collect()
}

fn image_original_size_to_scene(value: Option<[f32; 2]>) -> Option<[f32; 2]> {
    value.filter(|[width, height]| {
        width.is_finite() && height.is_finite() && *width > 0.0 && *height > 0.0
    })
}

fn image_tile_scale_to_scene(value: Option<f32>) -> f32 {
    value
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .unwrap_or(1.0)
}

fn fill_opacity(fill: &jian_ops_schema::style::PenFill) -> f32 {
    use jian_ops_schema::style::PenFill;
    let opacity = match fill {
        PenFill::Solid(body) => body.opacity,
        PenFill::LinearGradient(body) => body.opacity,
        PenFill::RadialGradient(body) => body.opacity,
        PenFill::MeshGradient(body) => body.opacity,
        PenFill::Shader(body) => body.opacity,
        PenFill::Image(body) => body.opacity,
    };
    opacity.unwrap_or(1.0).clamp(0.0, 1.0)
}

fn fill_blend_mode_to_scene(fill: &jian_ops_schema::style::PenFill) -> ImageBlendMode {
    use jian_ops_schema::style::PenFill;
    let blend = match fill {
        PenFill::Solid(body) => body.blend_mode.as_ref(),
        PenFill::LinearGradient(body) => body.blend_mode.as_ref(),
        PenFill::RadialGradient(body) => body.blend_mode.as_ref(),
        PenFill::MeshGradient(body) => body.blend_mode.as_ref(),
        PenFill::Shader(body) => body.blend_mode.as_ref(),
        PenFill::Image(body) => body.blend_mode.as_ref(),
    };
    blend_mode_to_scene(blend)
}

fn blend_mode_to_scene(value: Option<&jian_ops_schema::style::BlendMode>) -> ImageBlendMode {
    use jian_ops_schema::style::BlendMode;
    match value {
        Some(BlendMode::Darken) => ImageBlendMode::Darken,
        Some(BlendMode::Multiply) => ImageBlendMode::Multiply,
        Some(BlendMode::Screen) => ImageBlendMode::Screen,
        Some(BlendMode::Overlay) => ImageBlendMode::Overlay,
        Some(BlendMode::Lighten) => ImageBlendMode::Lighten,
        Some(BlendMode::Difference) => ImageBlendMode::Difference,
        Some(BlendMode::Hue) => ImageBlendMode::Hue,
        Some(BlendMode::Saturation) => ImageBlendMode::Saturation,
        Some(BlendMode::Color) => ImageBlendMode::Color,
        Some(BlendMode::Luminosity) => ImageBlendMode::Luminosity,
        Some(BlendMode::SoftLight) => ImageBlendMode::SoftLight,
        Some(BlendMode::ColorDodge) => ImageBlendMode::ColorDodge,
        Some(BlendMode::ColorBurn) => ImageBlendMode::ColorBurn,
        Some(BlendMode::HardLight) => ImageBlendMode::HardLight,
        Some(BlendMode::Exclusion) => ImageBlendMode::Exclusion,
        Some(BlendMode::Normal) | None => ImageBlendMode::Normal,
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
/// The SkSL source + pre-resolved uniforms ride through unchanged, except an
/// exact `size` uniform is overwritten with the resolved node dimensions;
/// node opacity (`k`) folds into the shader's own opacity multiplier.
/// The `fallback` `[r,g,b,a]` becomes the visible solid colour for
/// backends that can't run the program.
fn payload_shader_to_scene(s: &ShaderPayload, k: f32, size: [f32; 2]) -> SceneShader {
    let mut uniforms: Vec<SceneShaderUniform> = s
        .uniforms
        .iter()
        .map(|uniform| SceneShaderUniform {
            name: uniform.name.clone(),
            values: uniform.values.clone(),
        })
        .collect();
    if let Some(uniform) = uniforms.iter_mut().find(|uniform| uniform.name == "size") {
        uniform.values = size.to_vec();
    }
    SceneShader {
        sksl: s.sksl.clone(),
        uniforms,
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
