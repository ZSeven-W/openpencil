use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use jian_ops_schema::node::{ImageFitMode, ImageNode, ImageSrc, PenNode};
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};
use jian_ops_schema::style::BlendMode;

use crate::css::cascade::{compute_style_for_viewport, ComputedStyle};
use crate::dom::{DomElement, DomNode};
use crate::import_warning::ImportWarning;
use crate::length::{parse_length, LengthCtx};
use crate::mapper::MapCtx;

use super::{base_with_sizing, finish, numeric_attr, visual_props};

#[derive(Clone, Copy, Default)]
struct ParentContentBox {
    width: Option<f64>,
    height: Option<f64>,
    height_is_definite: bool,
}

pub(super) fn map_image(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    element: &DomElement,
    style: &ComputedStyle,
) -> PenNode {
    let source = crate::resources::normalize_image_source(crate::srcset::resolve_image_source(
        context, path, element,
    ));
    let visual = visual_props(context, style);
    let object_fit = match style.get("object-fit") {
        Some("cover") => Some(ImageFitMode::Crop),
        Some("contain") => Some(ImageFitMode::Fit),
        Some("fill") => Some(ImageFitMode::Fill),
        Some("scale-down") => {
            context.warn_once(ImportWarning::ObjectFitScaleDown);
            Some(ImageFitMode::Fit)
        }
        Some("none") => {
            context.warn_once(ImportWarning::ObjectFitNoneIgnored);
            None
        }
        _ => None,
    };
    if style
        .get("object-position")
        .is_some_and(|value| !is_default_object_position(value))
    {
        context.warn_once(ImportWarning::ObjectPositionIgnored);
    }
    // HTML width/height are presentation hints. Any cascaded CSS declaration,
    // including `auto`, wins; an absent CSS axis may still use the hint after
    // absolute opposing-inset stretch is undone below.
    let attribute_width = style
        .get("width")
        .is_none()
        .then(|| numeric_attr(element, "width"))
        .flatten();
    let attribute_height = style
        .get("height")
        .is_none()
        .then(|| numeric_attr(element, "height"))
        .flatten();
    let mut width = visual.width.or_else(|| attribute_width.clone());
    let mut height = visual.height.or_else(|| attribute_height.clone());
    let intrinsic = crate::resources::element_intrinsic_metadata(element);
    if intrinsic.is_some() {
        restore_replaced_auto_inset_axes(
            &mut width,
            &mut height,
            attribute_width,
            attribute_height,
            style,
        );
    }
    if !(intrinsic.is_some_and(|metadata| metadata.preferred_ratio.is_some())
        && aspect_ratio_prefers_intrinsic(style))
    {
        crate::mapper::apply_aspect_ratio_axes(
            &mut width,
            &mut height,
            &visual.limits,
            style,
            context,
        );
    }
    if let Some(metadata) = intrinsic {
        let needs_parent_box = metadata.preferred_ratio.is_some()
            && matches!(
                (width.as_ref(), height.as_ref()),
                (
                    Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
                    None
                ) | (
                    None,
                    Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
                )
            );
        let parent_box = if needs_parent_box {
            parent_content_box(context, path)
        } else {
            ParentContentBox::default()
        };
        apply_intrinsic_axes(
            &mut width,
            &mut height,
            &visual.limits,
            metadata,
            (parent_box.width, parent_box.height),
            parent_box.height_is_definite,
            context,
        );
    }
    // Positioning, transform scaling, and reserved flow boxes must see the
    // intrinsic axes before the leaf reaches its parent.
    let mut image_base = base_with_sizing(
        context,
        "img",
        style,
        &mut width,
        &mut height,
        visual.limits,
    );
    image_base.blend_mode = image_blend_mode(context, style);
    finish(
        context,
        PenNode::Image(ImageNode {
            base: image_base,
            src: ImageSrc::from(source),
            object_fit,
            width,
            height,
            corner_radius: visual.corner_radius,
            effects: visual.effects,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
            image_prompt: None,
            image_search_query: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
            limits: visual.limits,
        }),
    )
}

pub(super) fn map_svg(
    context: &mut MapCtx<'_>,
    element: &DomElement,
    style: &ComputedStyle,
) -> PenNode {
    context.warn_once(ImportWarning::InlineSvgPlaceholder);
    let source = serialize_element(element);
    let src = format!("data:image/svg+xml;base64,{}", STANDARD.encode(&source));
    let visual = visual_props(context, style);
    let mut width = visual.width.or_else(|| numeric_attr(element, "width"));
    let mut height = visual.height.or_else(|| numeric_attr(element, "height"));
    let intrinsic = crate::resources::browser_image_metadata(source.as_bytes());
    if !(intrinsic.is_some_and(|metadata| metadata.preferred_ratio.is_some())
        && aspect_ratio_prefers_intrinsic(style))
    {
        crate::mapper::apply_aspect_ratio_axes(
            &mut width,
            &mut height,
            &visual.limits,
            style,
            context,
        );
    }
    if let Some(metadata) = intrinsic {
        apply_intrinsic_axes(
            &mut width,
            &mut height,
            &visual.limits,
            metadata,
            (None, None),
            false,
            context,
        );
    }
    let mut image_base = base_with_sizing(
        context,
        "svg",
        style,
        &mut width,
        &mut height,
        visual.limits,
    );
    image_base.blend_mode = image_blend_mode(context, style);
    finish(
        context,
        PenNode::Image(ImageNode {
            base: image_base,
            src: ImageSrc::from(src),
            object_fit: None,
            width,
            height,
            corner_radius: visual.corner_radius,
            effects: visual.effects,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
            image_prompt: None,
            image_search_query: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
            limits: visual.limits,
        }),
    )
}

fn aspect_ratio_prefers_intrinsic(style: &ComputedStyle) -> bool {
    style.get("aspect-ratio").is_some_and(|value| {
        value
            .split_ascii_whitespace()
            .any(|token| token.eq_ignore_ascii_case("auto"))
    })
}

pub(crate) fn apply_intrinsic_axes(
    width: &mut Option<SizingBehavior>,
    height: &mut Option<SizingBehavior>,
    limits: &SizeLimits,
    metadata: crate::resources::BrowserImageMetadata,
    containing_content_box: (Option<f64>, Option<f64>),
    containing_height_is_definite: bool,
    context: &mut MapCtx<'_>,
) {
    let (intrinsic_width, intrinsic_height) = metadata.dimensions;
    let preferred_ratio = metadata.preferred_ratio;
    match (width.as_ref(), height.as_ref()) {
        (None, None) => {
            let (resolved_width, resolved_height) = if preferred_ratio.is_some() {
                constrained_intrinsic_size(intrinsic_width, intrinsic_height, 1.0, limits)
            } else {
                (
                    clamp_axis(intrinsic_width, limits.min_width, limits.max_width),
                    clamp_axis(intrinsic_height, limits.min_height, limits.max_height),
                )
            };
            *width = Some(SizingBehavior::Number(resolved_width));
            *height = Some(SizingBehavior::Number(resolved_height));
        }
        (Some(SizingBehavior::Number(value)), None) if value.is_finite() && *value >= 0.0 => {
            let (resolved_width, resolved_height) = if preferred_ratio.is_some() {
                constrained_intrinsic_size(
                    intrinsic_width,
                    intrinsic_height,
                    *value / intrinsic_width,
                    limits,
                )
            } else {
                (
                    clamp_axis(*value, limits.min_width, limits.max_width),
                    clamp_axis(intrinsic_height, limits.min_height, limits.max_height),
                )
            };
            *width = Some(SizingBehavior::Number(resolved_width));
            *height = Some(SizingBehavior::Number(resolved_height));
        }
        (None, Some(SizingBehavior::Number(value))) if value.is_finite() && *value >= 0.0 => {
            let (resolved_width, resolved_height) = if preferred_ratio.is_some() {
                constrained_intrinsic_size(
                    intrinsic_width,
                    intrinsic_height,
                    *value / intrinsic_height,
                    limits,
                )
            } else {
                (
                    clamp_axis(intrinsic_width, limits.min_width, limits.max_width),
                    clamp_axis(*value, limits.min_height, limits.max_height),
                )
            };
            *width = Some(SizingBehavior::Number(resolved_width));
            *height = Some(SizingBehavior::Number(resolved_height));
        }
        (Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)), None)
            if preferred_ratio.is_none() =>
        {
            *height = Some(SizingBehavior::Number(clamp_axis(
                intrinsic_height,
                limits.min_height,
                limits.max_height,
            )));
        }
        (Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)), None)
            if context.containing_width_is_definite && containing_content_box.0.is_some() =>
        {
            let anchor = containing_content_box.0.unwrap_or(0.0);
            let (resolved_width, resolved_height) = constrained_intrinsic_size(
                intrinsic_width,
                intrinsic_height,
                anchor / intrinsic_width,
                limits,
            );
            if (resolved_width - anchor).abs() > 1.0e-6 {
                *width = Some(SizingBehavior::Number(resolved_width));
            }
            *height = Some(SizingBehavior::Number(resolved_height));
        }
        (None, Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)))
            if preferred_ratio.is_none() =>
        {
            *width = Some(SizingBehavior::Number(clamp_axis(
                intrinsic_width,
                limits.min_width,
                limits.max_width,
            )));
        }
        (None, Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)))
            if containing_height_is_definite && containing_content_box.1.is_some() =>
        {
            let anchor = containing_content_box.1.unwrap_or(0.0);
            let (resolved_width, resolved_height) = constrained_intrinsic_size(
                intrinsic_width,
                intrinsic_height,
                anchor / intrinsic_height,
                limits,
            );
            *width = Some(SizingBehavior::Number(resolved_width));
            if (resolved_height - anchor).abs() > 1.0e-6 {
                *height = Some(SizingBehavior::Number(resolved_height));
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            context.warn_once(ImportWarning::ImageIntrinsicAxisUnresolved);
        }
        _ => {}
    }
}

fn parent_content_box(context: &MapCtx<'_>, path: &[&DomElement]) -> ParentContentBox {
    let Some(parent_index) = path.len().checked_sub(2) else {
        return ParentContentBox::default();
    };
    let initial_root_font_size = path
        .first()
        .and_then(|element| element.attr(crate::dom::INITIAL_ROOT_FONT_SIZE_ATTR))
        .and_then(|value| value.parse().ok())
        .filter(|value: &f64| value.is_finite() && *value > 0.0)
        .unwrap_or(context.opts.base_font_size);
    let mut styles = Vec::with_capacity(parent_index + 1);
    let mut computed_root_font_size = initial_root_font_size;
    for index in 0..=parent_index {
        let style = compute_style_for_viewport(
            &path[..=index],
            context.rules,
            styles.last(),
            if index == 0 {
                initial_root_font_size
            } else {
                computed_root_font_size
            },
            context.opts.viewport_width,
            context.opts.viewport_height(),
        );
        if index == 0 {
            computed_root_font_size = style.font_size;
        }
        styles.push(style);
    }
    let mut available_width = Some(context.opts.viewport_width);
    let mut available_height = Some(context.opts.viewport_height());
    let mut height_is_definite = true;
    for index in 0..=parent_index {
        let element = path[index];
        let style = &styles[index];
        let parent_style = index.checked_sub(1).map(|parent| &styles[parent]);
        let width_reference = available_width.unwrap_or(context.containing_width);
        let height_reference = available_height.unwrap_or(context.opts.viewport_height());
        let mut probe = MapCtx {
            opts: context.opts,
            rules: context.rules,
            warnings: Vec::new(),
            warned: Default::default(),
            next_id: 0,
            node_count: 0,
            containing_width: width_reference,
            containing_height: height_reference,
            containing_width_is_definite: true,
            positioned_width: width_reference,
            positioned_height: height_reference,
            auto_margin_handled_by_parent: false,
            pending_base_outcome: Default::default(),
        };
        let props = crate::mapper::container_props_from(style, &mut probe);
        let (mut padding_width, mut padding_height) = padding_axes(props.padding.as_ref());
        let (margin_width, margin_height) = margin_axes(style, available_width, context);
        if available_width.is_none() && box_edges_depend_on_reference(style, context) {
            padding_width = None;
            padding_height = None;
        }
        let root = matches!(element.tag.as_str(), "html" | "body");
        let fill_width = root || inferred_block_fill(element, style, parent_style);
        let outer_width = match props.width.as_ref() {
            Some(SizingBehavior::Number(value))
                if value.is_finite()
                    && (available_width.is_some()
                        || !style_axis_depends_on_reference(style, "width", context)) =>
            {
                Some(clamp_axis(
                    *value,
                    props.limits.min_width,
                    props.limits.max_width,
                ))
            }
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => {
                available_width.zip(margin_width).map(|(width, margin)| {
                    clamp_axis(
                        (width - margin).max(0.0),
                        props.limits.min_width,
                        props.limits.max_width,
                    )
                })
            }
            None if fill_width => available_width.zip(margin_width).map(|(width, margin)| {
                clamp_axis(
                    (width - margin).max(0.0),
                    props.limits.min_width,
                    props.limits.max_width,
                )
            }),
            _ => None,
        };
        available_width = outer_width
            .zip(padding_width)
            .map(|(width, padding)| (width - padding).max(0.0));

        let outer_height = match props.height.as_ref() {
            Some(SizingBehavior::Number(value)) if value.is_finite() => {
                height_is_definite = true;
                Some(clamp_axis(
                    *value,
                    props.limits.min_height,
                    props.limits.max_height,
                ))
            }
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) if height_is_definite => {
                available_height.zip(margin_height).map(|(height, margin)| {
                    clamp_axis(
                        (height - margin).max(0.0),
                        props.limits.min_height,
                        props.limits.max_height,
                    )
                })
            }
            _ => {
                height_is_definite = false;
                None
            }
        };
        available_height = outer_height
            .zip(padding_height)
            .map(|(height, padding)| (height - padding).max(0.0));
        if available_height.is_none() {
            height_is_definite = false;
        }
    }
    ParentContentBox {
        width: available_width,
        height: available_height,
        height_is_definite,
    }
}

fn parent_length(
    value: &str,
    style: &ComputedStyle,
    context: &MapCtx<'_>,
) -> Option<crate::length::CssLength> {
    parse_length(
        value,
        &LengthCtx {
            font_size: style.font_size,
            root_font_size: context.opts.base_font_size,
            viewport_w: context.opts.viewport_width,
            viewport_h: context.opts.viewport_height(),
        },
    )
}

fn padding_axes(
    padding: Option<&jian_ops_schema::node::container::Padding>,
) -> (Option<f64>, Option<f64>) {
    use jian_ops_schema::node::container::Padding;
    match padding {
        None => (Some(0.0), Some(0.0)),
        Some(Padding::Uniform(value)) => (Some(value * 2.0), Some(value * 2.0)),
        Some(Padding::XY([vertical, horizontal])) => (Some(horizontal * 2.0), Some(vertical * 2.0)),
        Some(Padding::LtrB([top, right, bottom, left])) => (Some(left + right), Some(top + bottom)),
        Some(Padding::Expression(_)) => (None, None),
    }
}

fn margin_axes(
    style: &ComputedStyle,
    reference: Option<f64>,
    context: &MapCtx<'_>,
) -> (Option<f64>, Option<f64>) {
    let margin = |name: &str| {
        let Some(value) = style
            .get(name)
            .filter(|value| !value.trim().eq_ignore_ascii_case("auto"))
        else {
            return Some(0.0);
        };
        let length = parent_length(value, style, context)?;
        if length.depends_on_reference() && reference.is_none() {
            return None;
        }
        let value = length.resolve(reference.unwrap_or(0.0));
        value.is_finite().then_some(value)
    };
    (
        margin("margin-left")
            .zip(margin("margin-right"))
            .map(|(left, right)| left + right),
        margin("margin-top")
            .zip(margin("margin-bottom"))
            .map(|(top, bottom)| top + bottom),
    )
}

fn box_edges_depend_on_reference(style: &ComputedStyle, context: &MapCtx<'_>) -> bool {
    [
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
    ]
    .into_iter()
    .any(|name| style_axis_depends_on_reference(style, name, context))
}

fn style_axis_depends_on_reference(
    style: &ComputedStyle,
    name: &str,
    context: &MapCtx<'_>,
) -> bool {
    style
        .get(name)
        .and_then(|value| parent_length(value, style, context))
        .is_some_and(|length| length.depends_on_reference())
}

fn inferred_block_fill(
    element: &DomElement,
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
) -> bool {
    if parent_style.is_some_and(|parent| {
        matches!(parent.get("display"), Some("flex" | "inline-flex"))
            && !matches!(
                parent.get("flex-direction"),
                Some("column" | "column-reverse")
            )
    }) {
        return false;
    }
    match style.get("display").map(str::trim) {
        Some("block" | "flow-root" | "list-item" | "table" | "flex" | "grid") => true,
        Some(_) => false,
        None => !crate::text::is_inline_tag(&element.tag),
    }
}

fn clamp_axis(value: f64, minimum: Option<f64>, maximum: Option<f64>) -> f64 {
    let mut value = value;
    if let Some(maximum) = maximum {
        value = value.min(maximum);
    }
    if let Some(minimum) = minimum {
        value = value.max(minimum);
    }
    value.max(0.0)
}

fn restore_replaced_auto_inset_axes(
    width: &mut Option<SizingBehavior>,
    height: &mut Option<SizingBehavior>,
    attribute_width: Option<SizingBehavior>,
    attribute_height: Option<SizingBehavior>,
    style: &ComputedStyle,
) {
    if !matches!(style.get("position"), Some("absolute" | "fixed")) {
        return;
    }
    let automatic = |axis: &str| {
        style
            .get(axis)
            .is_none_or(|value| value.trim().eq_ignore_ascii_case("auto"))
    };
    let pinned = |start: &str, end: &str| {
        [start, end].into_iter().all(|side| {
            style
                .get(side)
                .is_some_and(|value| !value.trim().eq_ignore_ascii_case("auto"))
        })
    };
    if automatic("width") && pinned("left", "right") {
        *width = attribute_width;
    }
    if automatic("height") && pinned("top", "bottom") {
        *height = attribute_height;
    }
}

fn constrained_intrinsic_size(
    width: f64,
    height: f64,
    desired_scale: f64,
    limits: &SizeLimits,
) -> (f64, f64) {
    let mut lower = 0.0_f64;
    if let Some(minimum) = limits.min_width {
        lower = lower.max(minimum / width);
    }
    if let Some(minimum) = limits.min_height {
        lower = lower.max(minimum / height);
    }
    let mut upper = f64::INFINITY;
    if let Some(maximum) = limits.max_width {
        upper = upper.min(maximum / width);
    }
    if let Some(maximum) = limits.max_height {
        upper = upper.min(maximum / height);
    }
    let desired_scale = if desired_scale.is_finite() {
        desired_scale.max(0.0)
    } else if upper.is_finite() {
        upper.max(0.0)
    } else {
        lower.max(1.0)
    };
    let scale = if lower > upper {
        lower
    } else {
        desired_scale.clamp(lower, upper)
    };
    let scale = if scale.is_finite() {
        scale.max(0.0)
    } else {
        1.0
    };
    (width * scale, height * scale)
}

fn is_default_object_position(value: &str) -> bool {
    let parts: Vec<_> = value
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect();
    match parts.as_slice() {
        [value] => matches!(value.as_str(), "center" | "50%"),
        [first, second] => {
            matches!(first.as_str(), "center" | "50%")
                && matches!(second.as_str(), "center" | "50%")
        }
        _ => false,
    }
}

fn image_blend_mode(context: &mut MapCtx<'_>, style: &ComputedStyle) -> Option<BlendMode> {
    let value = style.get("mix-blend-mode")?;
    match crate::mapper::map_blend_mode(value) {
        Some(BlendMode::Normal) => None,
        Some(mode) => Some(mode),
        None => {
            context.warn_once(ImportWarning::ImageMixBlendModeUnsupported);
            None
        }
    }
}

fn serialize_element(element: &DomElement) -> String {
    let mut source = format!("<{}", element.tag);
    for (name, value) in &element.attrs {
        source.push(' ');
        source.push_str(name);
        source.push_str("=\"");
        source.push_str(&escape_xml(value));
        source.push('"');
    }
    source.push('>');
    for child in &element.children {
        match child {
            DomNode::Text(text) => source.push_str(&escape_xml(text)),
            DomNode::Element(child) => source.push_str(&serialize_element(child)),
        }
    }
    source.push_str("</");
    source.push_str(&element.tag);
    source.push('>');
    source
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
