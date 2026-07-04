//! Figma node converters — ports `converters/index.ts` and the
//! per-type converter modules. Walks the [`TreeNode`] tree and emits
//! canonical `PenNode`s.

use crate::common::{
    common_props, extract_position, lookup_icon_by_name, map_corner_radius, normalize_angle,
    resolve_height, resolve_width, round2, round3, ConversionContext, FigLayoutMode, IconStyle,
    SKIPPED_TYPES,
};
use crate::figma_types::FigVec2;
use crate::instance::{apply_instance_overrides_cached, merge_symbol_props};
use crate::kiwi::FigValue;
use crate::mappers::{
    fig_fill_color, map_figma_effects, map_figma_fills, map_figma_layout, map_figma_stroke,
    LayoutProps,
};
use crate::node_build::{
    ellipse_node, frame_node, group_node, line_node, path_node, rectangle_node, ref_node, text_node,
};
use crate::text_mapper::map_figma_text_props;
use crate::tree::{guid_to_string, TreeNode};
use crate::vector_decoder::{compute_svg_path_bounds, decode_figma_vector_path};
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::SizingBehavior;

/// Spread mapped [`LayoutProps`] onto a `ContainerProps`.
fn apply_layout(l: LayoutProps, c: &mut ContainerProps) {
    if l.layout.is_some() {
        c.layout = l.layout;
    }
    if let Some(g) = l.gap {
        c.gap = Some(NumberOrExpression::Number(g));
    }
    if l.padding.is_some() {
        c.padding = l.padding;
    }
    if l.justify_content.is_some() {
        c.justify_content = l.justify_content;
    }
    if l.align_items.is_some() {
        c.align_items = l.align_items;
    }
    if l.clip_content.is_some() {
        c.clip_content = l.clip_content;
    }
}

fn is_string_sizing(s: &SizingBehavior) -> bool {
    matches!(
        s,
        SizingBehavior::Keyword(_) | SizingBehavior::Expression(_)
    )
}

/// Convert a parent's children, skipping invisible / transparent
/// nodes. `parentStackMode` passed down is `None` in preserve mode.
pub fn convert_children(parent: &TreeNode, ctx: &mut ConversionContext) -> Vec<PenNode> {
    let parent_stack_mode: Option<String> = match ctx.layout_mode {
        FigLayoutMode::Preserve => None,
        FigLayoutMode::OpenPencil => parent.figma.get_str("stackMode").map(|s| s.to_string()),
    };
    let mut result = Vec::new();
    for child in &parent.children {
        if child.figma.get_bool("visible") == Some(false) {
            continue;
        }
        if child.figma.get_f64("opacity").map(|o| o <= 0.0) == Some(true) {
            continue;
        }
        if let Some(node) = convert_node(child, parent_stack_mode.as_deref(), ctx) {
            result.push(node);
        }
    }
    result
}

/// Dispatch one tree node to its type-specific converter.
pub fn convert_node(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> Option<PenNode> {
    let ty = tree.figma.get_str("type")?;
    if SKIPPED_TYPES.contains(&ty) {
        return None;
    }
    Some(match ty {
        "FRAME" | "SECTION" => convert_frame(tree, parent_stack_mode, ctx),
        "GROUP" => convert_group(tree, parent_stack_mode, ctx),
        "SYMBOL" => convert_component(tree, parent_stack_mode, ctx),
        "INSTANCE" => convert_instance(tree, parent_stack_mode, ctx),
        "RECTANGLE" | "ROUNDED_RECTANGLE" => convert_rectangle(tree, parent_stack_mode, ctx),
        "ELLIPSE" => convert_ellipse(tree, parent_stack_mode, ctx),
        "LINE" => convert_line(tree, ctx),
        "VECTOR" | "STAR" | "REGULAR_POLYGON" | "BOOLEAN_OPERATION" => {
            convert_vector(tree, parent_stack_mode, ctx)
        }
        "TEXT" => convert_text(tree, parent_stack_mode, ctx),
        _ => {
            if !tree.children.is_empty() {
                convert_frame(tree, parent_stack_mode, ctx)
            } else {
                ctx.warnings.push(format!(
                    "Skipped unsupported node type: {ty} ({})",
                    tree.figma.get_str("name").unwrap_or("")
                ));
                return None;
            }
        }
    })
}

/// Build the `{ width, height, layout?, cornerRadius, fill, stroke,
/// effects }` container shared by frame / component.
fn build_container(
    figma: &FigValue,
    parent_stack_mode: Option<&str>,
    has_auto_layout: bool,
    ctx: &ConversionContext,
) -> ContainerProps {
    let mut container = ContainerProps {
        width: Some(resolve_width(figma, parent_stack_mode, ctx)),
        height: Some(resolve_height(figma, parent_stack_mode, ctx)),
        ..ContainerProps::default()
    };
    let layout: Option<LayoutProps> = match ctx.layout_mode {
        FigLayoutMode::Preserve => {
            if has_auto_layout {
                Some(map_figma_layout(figma))
            } else if figma.get_bool("frameMaskDisabled") != Some(true) {
                Some(LayoutProps {
                    clip_content: Some(true),
                    ..LayoutProps::default()
                })
            } else {
                None
            }
        }
        FigLayoutMode::OpenPencil => Some(map_figma_layout(figma)),
    };
    if let Some(l) = layout {
        apply_layout(l, &mut container);
    }
    container.corner_radius = map_corner_radius(figma);
    container.fill = map_figma_fills(figma.get_array("fillPaints"))
        .or_else(|| map_figma_fills(figma.get_array("backgroundPaints")));
    container.stroke = map_figma_stroke(figma);
    container.effects = map_figma_effects(figma.get_array("effects"));
    container
}

/// Auto-layout flow order is ascending; the tree builder sorted
/// descending (z-order), so auto-layout containers reverse back to
/// flow order in BOTH modes — Preserve keeps authored geometry but
/// downstream flex re-solves still read array order, and OpenPencil
/// feeds the array straight to the layout engine.
fn order_children(children: Vec<PenNode>, has_auto_layout: bool) -> Vec<PenNode> {
    if has_auto_layout && children.len() > 1 {
        children.into_iter().rev().collect()
    } else {
        children
    }
}

fn convert_frame(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let id = ctx.generate_id();
    let children = convert_children(tree, ctx);
    let figma = &tree.figma;
    let has_auto_layout = figma
        .get_str("stackMode")
        .map(|s| s != "NONE")
        .unwrap_or(false);
    let container = build_container(figma, parent_stack_mode, has_auto_layout, ctx);
    let ordered = order_children(children, has_auto_layout);
    let base = common_props(figma, id);
    frame_node(
        base,
        container,
        if ordered.is_empty() {
            None
        } else {
            Some(ordered)
        },
        None,
    )
}

fn convert_component(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let figma_id = tree
        .figma
        .get("guid")
        .and_then(guid_to_string)
        .unwrap_or_default();
    let id = ctx
        .component_map
        .get(&figma_id)
        .cloned()
        .unwrap_or_else(|| ctx.generate_id());
    let children = convert_children(tree, ctx);
    let figma = &tree.figma;
    let has_auto_layout = figma
        .get_str("stackMode")
        .map(|s| s != "NONE")
        .unwrap_or(false);
    let container = build_container(figma, parent_stack_mode, has_auto_layout, ctx);
    let ordered = order_children(children, has_auto_layout);
    let base = common_props(figma, id);
    frame_node(
        base,
        container,
        if ordered.is_empty() {
            None
        } else {
            Some(ordered)
        },
        Some(true),
    )
}

fn convert_group(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let id = ctx.generate_id();
    let children = convert_children(tree, ctx);
    let figma = &tree.figma;
    let container = ContainerProps {
        width: Some(resolve_width(figma, parent_stack_mode, ctx)),
        height: Some(resolve_height(figma, parent_stack_mode, ctx)),
        ..ContainerProps::default()
    };
    let base = common_props(figma, id);
    group_node(
        base,
        container,
        if children.is_empty() {
            None
        } else {
            Some(children)
        },
    )
}

/// Whether a symbol instance carries visual (non-layout) overrides.
fn has_visual_overrides(figma: &FigValue) -> bool {
    figma
        .get("symbolData")
        .and_then(|s| s.get_array("symbolOverrides"))
        .map(|ovs| {
            ovs.iter().any(|ov| {
                ov.get_array("fillPaints").map(|p| !p.is_empty()) == Some(true)
                    || ov.get_array("strokePaints").map(|p| !p.is_empty()) == Some(true)
                    || ov.get("arcData").is_some()
                    || ov.get("textData").is_some()
                    || ov.get("fontSize").is_some()
            })
        })
        .unwrap_or(false)
}

fn convert_instance(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let figma = &tree.figma;
    let component_guid = figma
        .get("overriddenSymbolID")
        .or_else(|| figma.get("symbolData").and_then(|s| s.get("symbolID")))
        .and_then(guid_to_string);

    let inline =
        component_guid.is_some() && (tree.children.is_empty() || has_visual_overrides(figma));

    if inline {
        let key = component_guid.clone().unwrap();
        if let Some(symbol_node) = ctx.symbol_tree.get(&key).cloned() {
            if !symbol_node.children.is_empty() {
                let overrides = figma
                    .get("symbolData")
                    .and_then(|s| s.get_array("symbolOverrides"))
                    .map(|a| a.to_vec());
                let derived = figma.get_array("derivedSymbolData").map(|a| a.to_vec());
                let instance_size = figma.get("size").and_then(FigVec2::from_value);
                let children = apply_instance_overrides_cached(
                    &symbol_node,
                    overrides.as_deref(),
                    derived.as_deref(),
                    instance_size,
                    &mut ctx.instance_assignments,
                );
                let merged = merge_symbol_props(&tree.figma, &symbol_node.figma);
                let synthetic = TreeNode {
                    figma: merged,
                    children,
                };
                return convert_frame(&synthetic, parent_stack_mode, ctx);
            }
        }
    }

    // No inlining — emit a ref when the component is registered.
    if let Some(pen_id) = component_guid
        .as_ref()
        .and_then(|g| ctx.component_map.get(g))
        .cloned()
    {
        let id = ctx.generate_id();
        return ref_node(common_props(&tree.figma, id), pen_id);
    }

    convert_frame(tree, parent_stack_mode, ctx)
}

fn convert_rectangle(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let id = ctx.generate_id();
    let figma = &tree.figma;
    let container = ContainerProps {
        width: Some(resolve_width(figma, parent_stack_mode, ctx)),
        height: Some(resolve_height(figma, parent_stack_mode, ctx)),
        corner_radius: map_corner_radius(figma),
        fill: map_figma_fills(figma.get_array("fillPaints")),
        stroke: map_figma_stroke(figma),
        effects: map_figma_effects(figma.get_array("effects")),
        ..ContainerProps::default()
    };
    rectangle_node(common_props(figma, id), container)
}

/// Figma arc (radians, start+end) → Pen arc (degrees, start+sweep).
fn map_figma_arc_data(arc: &FigValue) -> (Option<f64>, Option<f64>, Option<f64>) {
    let start_rad = arc.get_f64("startingAngle").unwrap_or(0.0);
    let end_rad = arc
        .get_f64("endingAngle")
        .unwrap_or(std::f64::consts::PI * 2.0);
    let inner = arc.get_f64("innerRadius").unwrap_or(0.0);
    let (actual_start, sweep_rad) = if end_rad >= start_rad {
        (start_rad, end_rad - start_rad)
    } else {
        (end_rad, start_rad - end_rad)
    };
    let start_deg = actual_start * 180.0 / std::f64::consts::PI;
    let sweep_deg = sweep_rad * 180.0 / std::f64::consts::PI;
    (
        if start_deg.abs() > 0.1 {
            Some(round2(start_deg))
        } else {
            None
        },
        if (sweep_deg - 360.0).abs() > 0.1 {
            Some(round2(sweep_deg))
        } else {
            None
        },
        if inner > 0.001 {
            Some(round3(inner))
        } else {
            None
        },
    )
}

fn convert_ellipse(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let id = ctx.generate_id();
    let figma = &tree.figma;
    let width = resolve_width(figma, parent_stack_mode, ctx);
    let height = resolve_height(figma, parent_stack_mode, ctx);
    let mut base = common_props(figma, id);

    let (mut start_angle, mut sweep_angle, inner_radius) = figma
        .get("arcData")
        .map(map_figma_arc_data)
        .unwrap_or((None, None, None));

    // Absorb a flip into the arc angles (extract_flip sets at most one).
    if start_angle.is_some() || sweep_angle.is_some() || inner_radius.is_some() {
        let start = start_angle.unwrap_or(0.0);
        let sweep = sweep_angle.unwrap_or(360.0);
        if base.flip_x == Some(true) {
            start_angle = Some(normalize_angle(180.0 - start - sweep));
            sweep_angle = Some(sweep);
            base.flip_x = None;
        }
        if base.flip_y == Some(true) {
            start_angle = Some(normalize_angle(360.0 - start - sweep));
            sweep_angle = Some(sweep);
            base.flip_y = None;
        }
    }

    ellipse_node(
        base,
        width,
        height,
        map_figma_fills(figma.get_array("fillPaints")),
        map_figma_stroke(figma),
        map_figma_effects(figma.get_array("effects")),
        start_angle,
        sweep_angle,
        inner_radius,
    )
}

fn convert_line(tree: &TreeNode, ctx: &mut ConversionContext) -> PenNode {
    let id = ctx.generate_id();
    let figma = &tree.figma;
    let (x, y) = extract_position(figma);
    let w = figma
        .get("size")
        .and_then(|s| s.get_f64("x"))
        .unwrap_or(100.0);
    // Line builds its base manually — no flip / locked.
    let mut base = common_props(figma, id);
    base.flip_x = None;
    base.flip_y = None;
    base.locked = None;
    line_node(
        base,
        x + w,
        y,
        map_figma_stroke(figma),
        map_figma_effects(figma.get_array("effects")),
    )
}

fn convert_text(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let id = ctx.generate_id();
    let figma = &tree.figma;
    let mut props = map_figma_text_props(figma);
    let width = resolve_width(figma, parent_stack_mode, ctx);
    let height = resolve_height(figma, parent_stack_mode, ctx);

    // Reconcile textGrowth with the resolved width.
    use jian_ops_schema::node::text::TextGrowth;
    if props.text_growth.is_none() {
        if is_string_sizing(&width) || figma.get_str("textAutoResize").is_none() {
            props.text_growth = Some(TextGrowth::FixedWidth);
        }
    } else if props.text_growth == Some(TextGrowth::Auto) && is_string_sizing(&width) {
        props.text_growth = Some(TextGrowth::FixedWidth);
    }

    text_node(
        common_props(figma, id),
        width,
        height,
        props,
        map_figma_fills(figma.get_array("fillPaints")),
        map_figma_effects(figma.get_array("effects")),
    )
}

fn any_visible(paints: Option<&[FigValue]>) -> bool {
    paints
        .map(|p| p.iter().any(|x| x.get_bool("visible") != Some(false)))
        .unwrap_or(false)
}

fn convert_vector(
    tree: &TreeNode,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
) -> PenNode {
    let id = ctx.generate_id();
    let figma = &tree.figma;

    // Icon-lookup branch — match the node's name against the host's
    // icon registry. When set, the node converts to a Path carrying the
    // canonical 24×24 lucide `d` + `icon_id`, bypassing vector decode.
    // Mirrors TS `convertVector` lines 25-65.
    let name = figma.get_str("name").unwrap_or("");
    if let Some(icon) = lookup_icon_by_name(name) {
        return build_icon_path_node(figma, id, parent_stack_mode, ctx, icon);
    }

    let path_d = decode_figma_vector_path(figma, &ctx.blobs).unwrap_or_default();

    if !path_d.is_empty() {
        let mut base = common_props(figma, id);
        let mut width = resolve_width(figma, parent_stack_mode, ctx);
        let mut height = resolve_height(figma, parent_stack_mode, ctx);

        // Zero-size vectors: derive extent from the path bounds.
        let size_x = figma
            .get("size")
            .and_then(|s| s.get_f64("x"))
            .unwrap_or(0.0);
        let size_y = figma
            .get("size")
            .and_then(|s| s.get_f64("y"))
            .unwrap_or(0.0);
        if (size_x < 0.01 || size_y < 0.01)
            && matches!(width, SizingBehavior::Number(_))
            && matches!(height, SizingBehavior::Number(_))
        {
            if let Some(bounds) = compute_svg_path_bounds(&path_d) {
                let path_w = bounds.max_x - bounds.min_x;
                let path_h = bounds.max_y - bounds.min_y;
                if size_x < 0.01 && path_w > 0.01 {
                    width = SizingBehavior::Number(round2(path_w));
                    base.x = Some(round2(base.x.unwrap_or(0.0) + bounds.min_x));
                }
                if size_y < 0.01 && path_h > 0.01 {
                    height = SizingBehavior::Number(round2(path_h));
                    base.y = Some(round2(base.y.unwrap_or(0.0) + bounds.min_y));
                }
            }
        }

        // Stroke-only outline: Figma strokeGeometry is an expanded
        // outline, so treat the stroke paints as the fill.
        let has_visible_fills = any_visible(figma.get_array("fillPaints"));
        let has_visible_strokes = any_visible(figma.get_array("strokePaints"));
        let is_stroke_only = !has_visible_fills
            && has_visible_strokes
            && figma
                .get_array("fillGeometry")
                .map(|g| g.is_empty())
                .unwrap_or(true)
            && figma
                .get_array("strokeGeometry")
                .map(|g| !g.is_empty())
                .unwrap_or(false);

        if is_stroke_only {
            return path_node(
                base,
                Some(path_d),
                None,
                width,
                height,
                map_figma_fills(figma.get_array("strokePaints")),
                None,
                map_figma_effects(figma.get_array("effects")),
            );
        }
        return path_node(
            base,
            Some(path_d),
            None,
            width,
            height,
            map_figma_fills(figma.get_array("fillPaints")),
            map_figma_stroke(figma),
            map_figma_effects(figma.get_array("effects")),
        );
    }

    // Fallback — undecodable vector becomes a rectangle.
    ctx.warnings.push(format!(
        "Vector node \"{}\" converted as rectangle (path data not decodable)",
        figma.get_str("name").unwrap_or("")
    ));
    let container = ContainerProps {
        width: Some(resolve_width(figma, parent_stack_mode, ctx)),
        height: Some(resolve_height(figma, parent_stack_mode, ctx)),
        fill: map_figma_fills(figma.get_array("fillPaints")),
        stroke: map_figma_stroke(figma),
        effects: map_figma_effects(figma.get_array("effects")),
        ..ContainerProps::default()
    };
    rectangle_node(common_props(figma, id), container)
}

/// Build a `Path` PenNode for a host-resolved icon — ports
/// `path-converter.ts` lines 25-65. Stroke gets a sensible default
/// when the node has no stroke paint, and the stroke thickness scales
/// down proportionally for icons smaller than the lucide 24×24
/// reference box.
fn build_icon_path_node(
    figma: &FigValue,
    id: String,
    parent_stack_mode: Option<&str>,
    ctx: &mut ConversionContext,
    icon: crate::common::IconLookupResult,
) -> PenNode {
    use jian_ops_schema::style::{
        PenFill, PenStroke, SolidFillBody, StrokeCap, StrokeJoin, StrokeThickness,
    };

    let icon_w = resolve_width(figma, parent_stack_mode, ctx);
    let icon_h = resolve_height(figma, parent_stack_mode, ctx);

    // iconSize = min(width-if-numeric, height-if-numeric), defaulting to
    // 24 (the lucide reference box) when either axis is non-numeric.
    let w_num: f64 = match icon_w {
        SizingBehavior::Number(n) => n,
        _ => 24.0,
    };
    let h_num: f64 = match icon_h {
        SizingBehavior::Number(n) => n,
        _ => 24.0,
    };
    let icon_size = w_num.min(h_num);
    let icon_scale: f64 = icon_size / 24.0;

    let style = icon.style.unwrap_or(IconStyle::Stroke);
    let mapped_stroke = map_figma_stroke(figma);
    let mut stroke = match style {
        IconStyle::Stroke => Some(mapped_stroke.unwrap_or_else(|| PenStroke {
            thickness: StrokeThickness::Uniform(1.5),
            align: None,
            join: Some(StrokeJoin::Round),
            cap: Some(StrokeCap::Round),
            dash_pattern: None,
            dash_offset: None,
            fill: Some(vec![PenFill::Solid(SolidFillBody {
                color: fig_fill_color(figma).unwrap_or_else(|| "#000000".to_string()),
                explain: None,
                opacity: None,
                blend_mode: None,
            })]),
        })),
        IconStyle::Fill => mapped_stroke,
    };

    if let Some(s) = stroke.as_mut() {
        if icon_scale < 0.99 {
            if let StrokeThickness::Uniform(t) = s.thickness {
                let scaled = round2(t as f64 * icon_scale) as f32;
                s.thickness = StrokeThickness::Uniform(scaled);
            }
        }
    }

    let fill = match style {
        IconStyle::Fill => map_figma_fills(figma.get_array("fillPaints")),
        IconStyle::Stroke => None,
    };

    path_node(
        common_props(figma, id),
        Some(icon.d),
        icon.icon_id,
        icon_w,
        icon_h,
        fill,
        stroke,
        map_figma_effects(figma.get_array("effects")),
    )
}

#[cfg(test)]
mod tests;
