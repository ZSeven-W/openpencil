use crate::import_warning::ImportWarning;
use jian_ops_schema::node::base::{NumberOrExpression, PenNodeBase};
use jian_ops_schema::node::container::{AlignItems, ContainerProps, JustifyContent, LayoutMode};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};

use crate::css::cascade::{compute_style_for_viewport, ComputedStyle, StyleRule};
use crate::dom::DomElement;
use crate::length::{parse_length, CssLength, LengthCtx};
use crate::HtmlImportOptions;

#[path = "layout_heuristics.rs"]
mod layout_heuristics;
pub(crate) use layout_heuristics::apply_aspect_ratio_axes;
pub(crate) use layout_heuristics::infer_child_alignment;
pub use layout_heuristics::infer_gap_from_margins;

#[path = "mapper_visual.rs"]
mod visual;
pub(crate) use visual::map_blend_mode;
pub(crate) use visual::map_text_shadow;
pub(crate) use visual::warn_segment_text_shadow;
pub(crate) use visual::{fill_glyph_color, text_paint_color};

#[path = "mapper_frame.rs"]
mod mapper_frame;
use mapper_frame::frame;

#[path = "mapper_text_scope.rs"]
pub(crate) mod text_scope;
pub(crate) use text_scope::map_container_children;
pub use text_scope::{container_props_from, map_element};

#[path = "mapper_grid.rs"]
mod grid;

#[path = "mapper_pseudo.rs"]
pub(crate) mod pseudo;

#[path = "mapper_stack.rs"]
mod stack;

#[path = "mapper_box.rs"]
mod box_model;

#[path = "mapper_node_access.rs"]
pub(crate) mod node_access;

#[path = "mapper_offset.rs"]
mod offset;

#[path = "mapper_margin.rs"]
mod margin;
pub(crate) use margin::apply_root_margins;
#[cfg(test)]
pub(crate) use margin::authored_node as unwrap_margin_node;

#[path = "mapper_item.rs"]
mod item;

#[path = "mapper_intrinsic.rs"]
pub(crate) mod intrinsic;

#[path = "mapper_wrap.rs"]
mod wrap;
pub(crate) use wrap::apply_flex_wrap;

#[path = "mapper_grid_place.rs"]
mod grid_place;

#[path = "table.rs"]
mod table;

#[cfg(test)]
#[path = "mapper_position_tests.rs"]
mod position_tests;

#[cfg(test)]
#[path = "mapper_margin_tests.rs"]
mod margin_tests;

pub struct MapCtx<'a> {
    pub opts: &'a HtmlImportOptions,
    pub rules: &'a [StyleRule],
    pub warnings: Vec<ImportWarning>,
    /// Rendered text of every warning in `warnings` (mirrors
    /// `SheetParser::warned`). Only `warn_once` maintains it; a direct
    /// `warnings.push` bypasses the de-duplication as it always did.
    pub warned: std::collections::BTreeSet<String>,
    pub next_id: u32,
    pub node_count: usize,
    /// Current CSS containing block used to resolve percentages.
    pub containing_width: f64,
    pub containing_height: f64,
    pub containing_width_is_definite: bool,
    /// Nearest non-static ancestor used by absolutely positioned descendants.
    pub positioned_width: f64,
    pub positioned_height: f64,
    /// Set by a parent that already centred every child through
    /// `align-items`, so the per-child auto-margin emulation stays out of the
    /// way instead of wrapping each child a second time.
    pub auto_margin_handled_by_parent: bool,
    /// Result of the most recent [`apply_base_style`] call. `map_special`
    /// applies the base style deep inside `special.rs`, where the outcome has
    /// no way back to `map_element`; the replaced-element path drains it here
    /// so it still gets the offset wrapper an ordinary element would get.
    pub pending_base_outcome: BaseStyleOutcome,
}

/// What `apply_base_style_with_box` could not write onto the node itself.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BaseStyleOutcome {
    /// In-flow offset (CSS px) that would pull the node out of auto layout if
    /// written onto `base.x` / `base.y`. The caller re-parents the node
    /// through `mapper_offset::wrap_offset` instead.
    pub flow_offset: (f64, f64),
    /// The element's border-box size BEFORE `transform: scale()` was baked
    /// into it, when both axes were definite. The offset wrapper reserves this
    /// box in flow so a scaled element does not push its siblings.
    pub reserved_box: Option<(f64, f64)>,
}

impl MapCtx<'_> {
    pub fn generate_id(&mut self) -> String {
        let id = format!("html_{}", self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// Record `warning` unless an identical message was already recorded.
    ///
    /// De-duplication compares the rendered text, so two variants that print
    /// the same sentence still collapse into one entry exactly as the old
    /// string-keyed set did. The incoming warning is rendered ONCE and probed
    /// against [`MapCtx::warned`]; re-rendering the stored list per call was
    /// quadratic on a page with many distinct degradations.
    pub fn warn_once(&mut self, warning: ImportWarning) {
        if self.warned.insert(warning.to_string()) {
            self.warnings.push(warning);
        }
    }
}

pub(crate) fn map_element_scoped(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    inheritance_parent_style: Option<&ComputedStyle>,
    layout_parent_style: Option<&ComputedStyle>,
    text_fill_override: Option<&str>,
) -> Option<PenNode> {
    if context.node_count >= crate::MAX_OUTPUT_NODES {
        context.warn_once(ImportWarning::NodeLimitMapping);
        return None;
    }
    let element = *path.last()?;
    let style = compute_style_for_viewport(
        path,
        context.rules,
        inheritance_parent_style,
        context.opts.base_font_size,
        context.opts.viewport_width,
        context.opts.viewport_height(),
    );
    if style.get("display") == Some("none") {
        return None;
    }
    let previous_containing = (context.containing_width, context.containing_height);
    let previous_width_is_definite = context.containing_width_is_definite;
    let previous_positioned = (context.positioned_width, context.positioned_height);
    match style.get("position") {
        Some("fixed") => {
            context.containing_width = context.opts.viewport_width;
            context.containing_height = context.opts.viewport_height();
            context.containing_width_is_definite = true;
        }
        Some("absolute") => {
            context.containing_width = context.positioned_width;
            context.containing_height = context.positioned_height;
            context.containing_width_is_definite = true;
        }
        _ => {}
    }
    layout_heuristics::warn_for_degradations(
        &style,
        matches!(element.tag.as_str(), "img" | "svg"),
        context,
    );
    context.pending_base_outcome = BaseStyleOutcome::default();
    if let Some(mapped) = crate::special::map_radio_group(context, path, &style)
        .or_else(|| crate::special::map_special(context, path, &style))
    {
        // Replaced elements are leaves, but they still live in their parent's
        // flow: grid placement, in-flow offsets and auto-margin alignment all
        // apply to them exactly as they do to a mapped frame.
        let outcome = std::mem::take(&mut context.pending_base_outcome);
        let node = mapped.map(|node| {
            item::finish_leaf(
                context,
                node,
                &element.tag,
                &style,
                layout_parent_style,
                outcome,
            )
        });
        context.containing_width = previous_containing.0;
        context.containing_height = previous_containing.1;
        context.containing_width_is_definite = previous_width_is_definite;
        return node;
    }
    // Reserve this frame before generated content and descendants consume the
    // remaining budget, so no successfully mapped child becomes orphaned.
    context.node_count += 1;
    let can_transfer_text_clip =
        text_scope::subtree_allows_text_clip_transfer(context, path, &style, &element.children);
    let (mut container, local_text_fill) =
        text_scope::container_props_with_text_scope(&style, context, can_transfer_text_clip);
    let text_fill_override = local_text_fill.as_deref().or(text_fill_override);
    let mut children_centered_by_auto_margins = false;
    if container.align_items.is_none() {
        container.align_items = infer_child_alignment(context, path, &style, &element.children);
        children_centered_by_auto_margins = container.align_items.is_some();
    }
    // Native buttons center their anonymous content box even when the author
    // does not opt into flex/grid. A plain vertical Jian frame otherwise puts
    // labels such as the 31×31 product-card `+` at the top-left. Preserve any
    // explicit author alignment and only supply the browser control defaults.
    if element.tag == "button" {
        if container.justify_content.is_none() {
            container.justify_content = Some(JustifyContent::Center);
        }
        if container.align_items.is_none() {
            container.align_items = Some(AlignItems::Center);
        }
    }
    layout_heuristics::apply_sizing_defaults(
        &mut container,
        &style,
        layout_parent_style,
        context.containing_width_is_definite,
        layout_heuristics::is_inline_level(&style)
            || (style.get("display").is_none()
                && crate::text::is_inline_tag(&element.tag)
                && !layout_parent_style.is_some_and(|parent| {
                    matches!(
                        parent.get("display"),
                        Some("flex" | "inline-flex" | "grid" | "inline-grid")
                    )
                })),
    );
    layout_heuristics::apply_aspect_ratio(&mut container, &style, context);
    layout_heuristics::apply_legacy_size_limits(
        &mut container,
        context.containing_width,
        context.containing_height,
    );
    let parent_reference = (context.containing_width, context.containing_height);
    let mut base = PenNodeBase {
        id: context.generate_id(),
        name: Some(element.tag.clone()),
        role: ((element.tag == "button") || element.attr("role") == Some("button"))
            .then_some("button".to_string()),
        ..Default::default()
    };
    let outcome = apply_base_style_with_box(&mut base, &style, context, Some(&mut container));
    item::record_parent_hints(&mut base, &style, layout_parent_style, context);
    table::record_cell_span(
        &mut base,
        element,
        path,
        &style,
        layout_parent_style,
        context,
    );
    context.containing_width_is_definite = layout_heuristics::width_is_definite(
        container.width.as_ref(),
        context.containing_width_is_definite,
    );
    context.containing_width = layout_heuristics::resolved_axis(
        container.width.as_ref(),
        container.limits.min_width,
        container.limits.max_width,
        parent_reference.0,
    );
    context.containing_height = layout_heuristics::resolved_axis(
        container.height.as_ref(),
        container.limits.min_height,
        container.limits.max_height,
        parent_reference.1,
    );
    if layout_heuristics::establishes_positioning_context(&style) {
        context.positioned_width = context.containing_width;
        context.positioned_height = context.containing_height;
    }
    let previous_auto_margin = context.auto_margin_handled_by_parent;
    context.auto_margin_handled_by_parent = children_centered_by_auto_margins;
    let mapped_children = map_container_children(
        context,
        path,
        &style,
        layout_parent_style,
        &element.children,
        text_fill_override,
    );
    let children = mapped_children.nodes;
    let children = wrap::apply_flex_wrap(context, &style, &mut container, children);
    // Runs before the containing block is restored: the column widths are
    // measured against the table's own used width. That width is only real
    // when the author pinned one — a missing `width` is inferred as a fill of
    // the parent further up, which for a table nested in a cell is the cell's
    // own containing block rather than the width the post-pass will give it.
    let table_width_is_definite = context.containing_width_is_definite
        && style
            .get("width")
            .and_then(|value| map_sizing(value, style.font_size, context.opts, parent_reference.0))
            .is_some();
    let children = table::finish_table(context, path, &style, table_width_is_definite, children);
    intrinsic::promote_single_image_size(&mut container, &children);
    context.auto_margin_handled_by_parent = previous_auto_margin;
    context.containing_width = previous_containing.0;
    context.containing_height = previous_containing.1;
    context.containing_width_is_definite = previous_width_is_definite;
    context.positioned_width = previous_positioned.0;
    context.positioned_height = previous_positioned.1;
    let node = offset::wrap_offset(
        context,
        frame(base, container, children),
        outcome.flow_offset,
        outcome.reserved_box,
    );
    let node = item::align_by_auto_margins(
        context,
        node,
        &style,
        layout_parent_style,
        previous_auto_margin,
    );
    Some(margin::wrap_margins(
        context,
        node,
        &element.tag,
        &style,
        layout_parent_style,
        mapped_children.collapsed,
    ))
}

pub(crate) fn layer_positioned_children(children: Vec<PenNode>) -> Vec<PenNode> {
    stack::layer_absolute_children(children)
}

fn container_props_from_impl(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    text_clip_will_transfer: bool,
) -> ContainerProps {
    let layout = layout_heuristics::layout_for(style);
    let gap_property = if layout == LayoutMode::Horizontal {
        "column-gap"
    } else {
        "row-gap"
    };
    let gap = if matches!(style.get("display"), Some("grid" | "inline-grid")) {
        grid::grid_row_gap(style, context)
    } else {
        style
            .get(gap_property)
            .or_else(|| style.get("gap"))
            .and_then(|value| {
                length_px(
                    value,
                    style.font_size,
                    context.opts,
                    context.containing_width,
                )
            })
    }
    // CSS `border-spacing` is the table box tree's own gap property.
    .or_else(|| table::spacing_gap(style, context))
    .map(NumberOrExpression::Number);
    let border_widths = box_model::border_widths(style, context);
    let justify_content = style
        .get("justify-content")
        .or_else(|| style.get("justify-items"))
        .and_then(map_justify);
    let align_items = style.get("align-items").and_then(AlignItems::from_css);
    let mut width = style.get("width").and_then(|value| {
        map_sizing(
            value,
            style.font_size,
            context.opts,
            context.containing_width,
        )
    });
    let mut height = style.get("height").and_then(|value| {
        map_sizing(
            value,
            style.font_size,
            context.opts,
            context.containing_height,
        )
    });
    let absolute = matches!(style.get("position"), Some("absolute" | "fixed"));
    let infer_stretched_width = absolute
        && width.is_none()
        && layout_heuristics::has_non_auto(style, "left")
        && layout_heuristics::has_non_auto(style, "right");
    let infer_stretched_height = absolute
        && height.is_none()
        && layout_heuristics::has_non_auto(style, "top")
        && layout_heuristics::has_non_auto(style, "bottom");
    // A FillContainer leaf is represented as a percentage in Jian/Taffy. An
    // absolutely positioned percentage has no flex track to fill, so the
    // legacy loader can resolve it to zero. Bake the selected viewport's
    // containing block into an explicit size instead. This is also the exact
    // static-canvas meaning of CSS `width/height:100%` at import time.
    if absolute {
        layout_heuristics::resolve_absolute_fill(&mut width, context.containing_width);
        layout_heuristics::resolve_absolute_fill(&mut height, context.containing_height);
    }
    let mut limits = map_size_limits(style, context);
    box_model::apply_box_sizing(
        style,
        context,
        border_widths,
        &mut width,
        &mut height,
        &mut limits,
    );
    // Auto size plus opposing insets uses the remaining containing-block
    // space. Apply this after content-box expansion: unlike an authored
    // width, the inferred value already denotes the outer box.
    if infer_stretched_width {
        width = Some(SizingBehavior::Number(
            layout_heuristics::stretched_absolute_axis(
                style,
                context,
                "left",
                "right",
                context.containing_width,
            ),
        ));
    }
    if infer_stretched_height {
        height = Some(SizingBehavior::Number(
            layout_heuristics::stretched_absolute_axis(
                style,
                context,
                "top",
                "bottom",
                context.containing_height,
            ),
        ));
    }
    let own_width =
        layout_heuristics::resolved_axis(width.as_ref(), None, None, context.containing_width);
    let own_height =
        layout_heuristics::resolved_axis(height.as_ref(), None, None, context.containing_height);
    // Which axes of that box are the element's own used size rather than the
    // containing block substituted for a hug / fill / missing axis — the same
    // discipline `box_is_definite` applies to the transform bake. A background
    // crop transform built from a fabricated axis lands off the node entirely.
    let box_definite = (
        matches!(width.as_ref(), Some(SizingBehavior::Number(value)) if value.is_finite()),
        matches!(height.as_ref(), Some(SizingBehavior::Number(value)) if value.is_finite()),
    );
    let corner_radius = visual::map_corner_radius(style, context, own_width, own_height);
    // The visual pass runs after sizing on purpose: `background-size` and
    // `background-position` are resolved against the element's used box, which
    // only exists once width / height / limits have been settled.
    let fill = visual::map_fill(
        style,
        context,
        (own_width, own_height),
        box_definite,
        text_clip_will_transfer,
    );
    let stroke = visual::map_stroke(style, context);
    let effects = visual::map_effects(style, context);
    let padding = box_model::map_padding(style, context, border_widths);
    let mut container = ContainerProps {
        width,
        height,
        layout: Some(layout),
        gap,
        padding,
        justify_content,
        align_items,
        clip_content: box_model::clips_content(style, context).then_some(true),
        sticky_children: None,
        corner_radius,
        fill,
        stroke,
        effects,
        // No responsive-schema (jian formatVersion 1.2) source in HTML
        // import — this pipeline only ever emits non-responsive documents.
        limits,
    };
    layout_heuristics::apply_legacy_size_limits(
        &mut container,
        context.containing_width,
        context.containing_height,
    );
    container
}

/// Base-style pass for callers that have no `ContainerProps` to hand (replaced
/// elements, pseudo elements, the document root). The outcome is also parked on
/// `MapCtx::pending_base_outcome` so `map_special`'s deep call chain can hand it
/// back to `map_element` without threading a return value through `special.rs`.
pub(crate) fn apply_base_style(
    base: &mut PenNodeBase,
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
) -> BaseStyleOutcome {
    let outcome = apply_base_style_with_box(base, style, context, None);
    context.pending_base_outcome = outcome;
    outcome
}

/// Report an in-flow offset the caller has no way to apply. `map_element` and
/// `finish_leaf` both build an offset wrapper; the pseudo-element and document
/// root paths cannot, so the shift is silently lost without this.
pub(crate) fn warn_dropped_flow_offset(context: &mut MapCtx<'_>, outcome: BaseStyleOutcome) {
    if outcome.flow_offset.0 != 0.0 || outcome.flow_offset.1 != 0.0 {
        context.warn_once(ImportWarning::FlowOffsetNoWrapper);
    }
}

/// Shared base-style pass. `container` is the element's already-sized
/// `ContainerProps` when the caller has one; it lets the transform bake read
/// the real used size and write a scaled size back.
pub(crate) fn apply_base_style_with_box(
    base: &mut PenNodeBase,
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    mut container: Option<&mut ContainerProps>,
) -> BaseStyleOutcome {
    base.opacity = style
        .get("opacity")
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| NumberOrExpression::Number(value.clamp(0.0, 1.0)));
    if matches!(style.get("visibility"), Some("hidden" | "collapse")) {
        base.visible = Some(false);
    }
    let position = style.get("position");
    let explicit_z_index = style
        .get("z-index")
        .filter(|value| !value.eq_ignore_ascii_case("auto"))
        .and_then(|value| value.parse::<i32>().ok());
    // Relative/sticky elements with z-index:auto remain ordinary flow items.
    // Marking them as a layer would reorder auto-layout children and change
    // their geometry even though CSS preserves their source-order position.
    if matches!(position, Some("absolute" | "fixed")) || explicit_z_index.is_some() {
        let z_index = explicit_z_index.unwrap_or(0);
        base.explain = Some(format!("{}{z_index}", stack::Z_INDEX_HINT));
    }
    let (own_width, own_height) = used_box(style, context, container.as_deref());
    let definite = box_is_definite(style, container.as_deref());
    let reserved_box =
        (definite == (true, true)).then_some((own_width.max(0.0), own_height.max(0.0)));
    let bake = crate::transform::bake_transform(style, context, own_width, own_height, definite);
    if let Some(bake) = &bake {
        if bake.rotation_degrees.abs() > 1e-9 {
            base.rotation = Some(bake.rotation_degrees);
        }
    }
    layout_heuristics::apply_position(
        base,
        style,
        context,
        (
            definite.0.then_some(own_width),
            definite.1.then_some(own_height),
        ),
    );
    let out_of_flow = matches!(style.get("position"), Some("absolute" | "fixed"));
    let mut flow_offset = layout_heuristics::relative_offset(style, context);
    if let Some(bake) = &bake {
        // The scale runs first so the translation knows which axes actually
        // grew: the centre-derived pull-back is half of the SCALED box only
        // where the scale landed, and half of the original box elsewhere.
        let applied = if bake.resizes_node() {
            bake_scale(context, container.take(), bake.scale)
        } else {
            (true, true)
        };
        let translate = bake.translate_for_applied_scale(applied);
        if translate.0.abs() > 1e-6 || translate.1.abs() > 1e-6 {
            if out_of_flow {
                base.x = Some(base.x.unwrap_or(0.0) + translate.0);
                base.y = Some(base.y.unwrap_or(0.0) + translate.1);
            } else {
                flow_offset.0 += translate.0;
                flow_offset.1 += translate.1;
            }
        }
    }
    if out_of_flow {
        // Out-of-flow nodes carry their offset directly on x/y.
        base.x = Some(base.x.unwrap_or(0.0) + flow_offset.0);
        base.y = Some(base.y.unwrap_or(0.0) + flow_offset.1);
        return BaseStyleOutcome {
            flow_offset: (0.0, 0.0),
            reserved_box,
        };
    }
    BaseStyleOutcome {
        flow_offset,
        reserved_box,
    }
}

/// The element's used border-box size, preferring the resolved container.
fn used_box(
    style: &ComputedStyle,
    context: &MapCtx<'_>,
    container: Option<&ContainerProps>,
) -> (f64, f64) {
    match container {
        Some(container) => (
            layout_heuristics::resolved_axis(
                container.width.as_ref(),
                container.limits.min_width,
                container.limits.max_width,
                context.containing_width,
            ),
            layout_heuristics::resolved_axis(
                container.height.as_ref(),
                container.limits.min_height,
                container.limits.max_height,
                context.containing_height,
            ),
        ),
        None => (
            layout_heuristics::style_axis_size(style, context, "width", context.containing_width),
            layout_heuristics::style_axis_size(style, context, "height", context.containing_height),
        ),
    }
}

/// Whether each axis of [`used_box`] is the element's real used size rather
/// than the containing block substituted for a hug / fill / missing axis.
fn box_is_definite(style: &ComputedStyle, container: Option<&ContainerProps>) -> (bool, bool) {
    match container {
        Some(container) => (
            matches!(container.width.as_ref(), Some(SizingBehavior::Number(value)) if value.is_finite()),
            matches!(container.height.as_ref(), Some(SizingBehavior::Number(value)) if value.is_finite()),
        ),
        // No resolved container: `style_axis_size` reads the declaration
        // directly and yields 0 when there is none, so only an authored,
        // non-`auto` value counts as a real size.
        None => (
            layout_heuristics::has_non_auto(style, "width"),
            layout_heuristics::has_non_auto(style, "height"),
        ),
    }
}

/// `transform: scale()` is baked into the node's own size. Font sizes and the
/// fixed sizes of descendants are deliberately left alone: the schema scales
/// text through `font_size` and each node through its own width / height, and
/// rewriting a whole subtree at import time would fight the cascade far more
/// than a slightly-small label costs.
///
/// Runs after `apply_legacy_size_limits`, so the scaled result can exceed the
/// min / max box just enforced. That is the CSS reading — `transform` is a
/// paint-time operation that `min-width` / `max-width` never constrain — but it
/// does mean the serialized size no longer satisfies `limits`.
///
/// Returns which axes actually took the factor.
fn bake_scale(
    context: &mut MapCtx<'_>,
    container: Option<&mut ContainerProps>,
    scale: (f64, f64),
) -> (bool, bool) {
    let Some(container) = container else {
        context.warn_once(ImportWarning::TransformScaleNotBaked);
        return (false, false);
    };
    let width = layout_heuristics::scale_axis(&mut container.width, scale.0);
    let height = layout_heuristics::scale_axis(&mut container.height, scale.1);
    if width || height {
        context.warn_once(ImportWarning::TransformScaleBaked);
    } else {
        context.warn_once(ImportWarning::TransformScaleAutoSizeIgnored);
    }
    (width, height)
}

fn length_context(font_size: f64, options: &HtmlImportOptions) -> LengthCtx {
    LengthCtx {
        font_size,
        root_font_size: options.base_font_size,
        viewport_w: options.viewport_width,
        viewport_h: options.viewport_height(),
    }
}

fn length_px(
    value: &str,
    font_size: f64,
    options: &HtmlImportOptions,
    reference: f64,
) -> Option<f64> {
    Some(parse_length(value, &length_context(font_size, options))?.resolve(reference))
}

fn map_sizing(
    value: &str,
    font_size: f64,
    options: &HtmlImportOptions,
    reference: f64,
) -> Option<SizingBehavior> {
    let length = parse_length(value, &length_context(font_size, options))?;
    match &length {
        CssLength::Px(value) => Some(SizingBehavior::Number(*value)),
        CssLength::Percent(100.0) => Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
        _ => Some(SizingBehavior::Number(length.resolve(reference))),
    }
}

fn map_size_limits(style: &ComputedStyle, context: &MapCtx<'_>) -> SizeLimits {
    let resolve = |name: &str, reference: f64| {
        style
            .get(name)
            .and_then(|value| parse_length(value, &length_context(style.font_size, context.opts)))
            .map(|length| length.resolve(reference))
            .filter(|value| value.is_finite() && *value >= 0.0)
    };
    SizeLimits {
        min_width: resolve("min-width", context.containing_width),
        max_width: resolve("max-width", context.containing_width),
        min_height: resolve("min-height", context.containing_height),
        max_height: resolve("max-height", context.containing_height),
    }
}

fn map_justify(value: &str) -> Option<JustifyContent> {
    match value.trim().to_ascii_lowercase().as_str() {
        "flex-start" | "start" | "left" => Some(JustifyContent::Start),
        "center" => Some(JustifyContent::Center),
        "flex-end" | "end" | "right" => Some(JustifyContent::End),
        "space-between" => Some(JustifyContent::SpaceBetween),
        "space-around" | "space-evenly" => Some(JustifyContent::SpaceAround),
        _ => None,
    }
}

#[cfg(test)]
#[path = "mapper_tests.rs"]
mod tests;
