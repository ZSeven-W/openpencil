use crate::import_warning::ImportWarning;
use std::cmp::Ordering;

use jian_ops_schema::constraints::{Constraints, HConstraint, VConstraint};
use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::container::{AlignItems, ContainerProps, LayoutMode};
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};

use crate::css::cascade::{compute_style_for_viewport, ComputedStyle};
use crate::dom::{DomElement, DomNode};
use crate::length::{parse_length, LengthCtx};

use super::MapCtx;

#[path = "layout_positioned_margin.rs"]
mod positioned_margin;

pub(crate) fn infer_child_alignment(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    parent_style: &ComputedStyle,
    children: &[DomNode],
) -> Option<AlignItems> {
    let styles: Vec<_> = children
        .iter()
        .filter_map(|child| match child {
            DomNode::Element(element) => {
                let mut child_path = path.to_vec();
                child_path.push(element);
                let style = compute_style_for_viewport(
                    &child_path,
                    context.rules,
                    Some(parent_style),
                    context.opts.base_font_size,
                    context.opts.viewport_width,
                    context.opts.viewport_height(),
                );
                (style.get("display") != Some("none")
                    && !matches!(style.get("position"), Some("absolute" | "fixed")))
                .then_some(style)
            }
            DomNode::Text(text) if text.trim().is_empty() => None,
            DomNode::Text(_) => None,
        })
        .collect();
    let is_auto = |style: &ComputedStyle, name: &str| {
        style.get(name).is_some_and(|value| value.trim() == "auto")
    };
    (!styles.is_empty()
        && styles
            .iter()
            .all(|style| is_auto(style, "margin-left") && is_auto(style, "margin-right")))
    .then_some(AlignItems::Center)
}

pub fn infer_gap_from_margins(
    child_styles: &[&ComputedStyle],
    context_font: f64,
) -> (Option<f64>, bool) {
    if child_styles.len() < 2 {
        return (None, false);
    }
    let context = LengthCtx {
        font_size: context_font,
        root_font_size: context_font,
        viewport_w: 0.0,
        viewport_h: 0.0,
    };
    let margin = |style: &ComputedStyle, name: &str| {
        style
            .get(name)
            .and_then(|value| parse_length(value, &context))
            .map(|length| length.resolve(0.0))
            .unwrap_or(0.0)
    };
    let mut gaps: Vec<_> = child_styles
        .windows(2)
        .map(|pair| margin(pair[0], "margin-bottom") + margin(pair[1], "margin-top"))
        .collect();
    gaps.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let mut modes: Vec<(f64, usize)> = Vec::new();
    for gap in &gaps {
        if let Some((_, count)) = modes.iter_mut().find(|(value, _)| *value == *gap) {
            *count += 1;
        } else {
            modes.push((*gap, 1));
        }
    }
    let mode = modes
        .into_iter()
        .max_by(|(left_value, left_count), (right_value, right_count)| {
            left_count.cmp(right_count).then_with(|| {
                right_value
                    .partial_cmp(left_value)
                    .unwrap_or(Ordering::Equal)
            })
        })
        .map(|(value, _)| value);
    let deviated = mode.is_some_and(|mode| gaps.iter().any(|gap| *gap != mode));
    (mode, deviated)
}

pub(super) fn apply_sizing_defaults(
    container: &mut ContainerProps,
    style: &ComputedStyle,
    parent: Option<&ComputedStyle>,
    parent_width_is_definite: bool,
    inline_level: bool,
) {
    let parent_layout = parent.map(layout_for).unwrap_or(LayoutMode::Vertical);
    if container.width.is_none() && parent.is_some() {
        let shrink_to_fit =
            parent_layout == LayoutMode::Horizontal || inline_level || !parent_width_is_definite;
        container.width = Some(SizingBehavior::Keyword(if shrink_to_fit {
            SizingKeyword::FitContent
        } else {
            SizingKeyword::FillContainer
        }));
    }
    if container.height.is_none() {
        container.height = Some(SizingBehavior::Keyword(SizingKeyword::FitContent));
    }
    if style
        .get("flex-grow")
        .and_then(|value| value.parse::<f64>().ok())
        .is_some_and(|value| value > 0.0)
    {
        let fill = Some(SizingBehavior::Keyword(SizingKeyword::FillContainer));
        if parent_layout == LayoutMode::Horizontal {
            container.width = fill;
        } else {
            container.height = fill;
        }
    }
}

pub(super) fn width_is_definite(
    width: Option<&SizingBehavior>,
    parent_width_is_definite: bool,
) -> bool {
    match width {
        Some(SizingBehavior::Number(value)) => value.is_finite(),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => parent_width_is_definite,
        None => parent_width_is_definite,
        _ => false,
    }
}

pub(super) fn resolved_axis(
    sizing: Option<&SizingBehavior>,
    minimum: Option<f64>,
    maximum: Option<f64>,
    fallback: f64,
) -> f64 {
    let mut value = match sizing {
        Some(SizingBehavior::Number(value)) if value.is_finite() => *value,
        _ => fallback,
    };
    if let Some(maximum) = maximum {
        value = value.min(maximum);
    }
    if let Some(minimum) = minimum {
        value = value.max(minimum);
    }
    value.max(0.0)
}

pub(super) fn has_non_auto(style: &ComputedStyle, property: &str) -> bool {
    style
        .get(property)
        .is_some_and(|value| !value.trim().eq_ignore_ascii_case("auto"))
}

pub(super) fn establishes_positioning_context(style: &ComputedStyle) -> bool {
    !matches!(style.get("position"), None | Some("static"))
        || style.get("transform").is_some_and(|value| value != "none")
}

/// Non-responsive Jian documents do not consume `SizeLimits` during layout.
/// Preserve the limits for serialization, but also bake any constraint that
/// changes the imported size at the selected viewport into the legacy axis.
pub(super) fn apply_legacy_size_limits(
    container: &mut ContainerProps,
    available_width: f64,
    available_height: f64,
) {
    constrain_legacy_axis(
        &mut container.width,
        container.limits.min_width,
        container.limits.max_width,
        available_width,
    );
    constrain_legacy_axis(
        &mut container.height,
        container.limits.min_height,
        container.limits.max_height,
        available_height,
    );
}

fn constrain_legacy_axis(
    sizing: &mut Option<SizingBehavior>,
    minimum: Option<f64>,
    maximum: Option<f64>,
    available: f64,
) {
    let clamp = |mut value: f64| {
        if let Some(maximum) = maximum {
            value = value.min(maximum);
        }
        if let Some(minimum) = minimum {
            value = value.max(minimum);
        }
        value.max(0.0)
    };
    match sizing {
        Some(SizingBehavior::Number(value)) => *value = clamp(*value),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)) => {
            let constrained = clamp(available);
            if (constrained - available).abs() > f64::EPSILON {
                *sizing = Some(SizingBehavior::Number(constrained));
            }
        }
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) | None => {
            if let Some(minimum) = minimum {
                *sizing = Some(SizingBehavior::Number(clamp(minimum)));
            }
        }
        Some(SizingBehavior::Expression(_)) => {
            if minimum.is_some() || maximum.is_some() {
                *sizing = Some(SizingBehavior::Number(clamp(available)));
            }
        }
    }
}

pub(super) fn is_inline_level(style: &ComputedStyle) -> bool {
    matches!(
        style.get("display").map(str::trim),
        Some(
            "inline"
                | "inline flow"
                | "inline-block"
                | "inline flow-root"
                | "inline-flex"
                | "inline flex"
                | "inline-grid"
                | "inline grid"
        )
    )
}

/// The single funnel every caller uses to pick a Jian layout axis. A table row
/// lays its cells out inline, which is exactly a horizontal frame; everything
/// else that is not a row-direction flex container stacks vertically.
pub(crate) fn layout_for(style: &ComputedStyle) -> LayoutMode {
    match (style.get("display"), style.get("flex-direction")) {
        (Some("flex" | "inline-flex"), Some("column" | "column-reverse")) => LayoutMode::Vertical,
        (Some("flex" | "inline-flex"), _) => LayoutMode::Horizontal,
        (Some("table-row"), _) => LayoutMode::Horizontal,
        _ => LayoutMode::Vertical,
    }
}

pub(super) fn apply_position(
    base: &mut PenNodeBase,
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    used_size: (Option<f64>, Option<f64>),
) {
    if !matches!(style.get("position"), Some("absolute" | "fixed")) {
        return;
    }
    let (left, left_percent) = position_value(
        style.get("left"),
        style.font_size,
        context.containing_width,
        context,
    );
    let (top, top_percent) = position_value(
        style.get("top"),
        style.font_size,
        context.containing_height,
        context,
    );
    // Jian treats a child as absolute as soon as either coordinate is
    // authored. CSS `position:absolute` without offsets therefore maps to
    // the static origin instead of accidentally entering auto layout.
    let (right, right_percent) = position_value(
        style.get("right"),
        style.font_size,
        context.containing_width,
        context,
    );
    let (bottom, bottom_percent) = position_value(
        style.get("bottom"),
        style.font_size,
        context.containing_height,
        context,
    );
    let distribute_horizontal_auto = used_size.0.is_some();
    let distribute_vertical_auto = used_size.1.is_some();
    let own_width = used_size.0.unwrap_or_else(|| {
        own_size(
            style.get("width"),
            style.font_size,
            context.containing_width,
            context,
        )
    });
    let own_height = used_size.1.unwrap_or_else(|| {
        own_size(
            style.get("height"),
            style.font_size,
            context.containing_height,
            context,
        )
    });
    let margin_reference = context.containing_width;
    let margin_left = positioned_margin::resolve(style, "margin-left", margin_reference, context);
    let margin_right = positioned_margin::resolve(style, "margin-right", margin_reference, context);
    let margin_top = positioned_margin::resolve(style, "margin-top", margin_reference, context);
    let margin_bottom =
        positioned_margin::resolve(style, "margin-bottom", margin_reference, context);
    let x = positioned_margin::axis_origin(
        left,
        right,
        own_width,
        context.containing_width,
        (margin_left, margin_right),
        (
            distribute_horizontal_auto,
            style.get("direction") == Some("rtl"),
        ),
    );
    let y = positioned_margin::axis_origin(
        top,
        bottom,
        own_height,
        context.containing_height,
        (margin_top, margin_bottom),
        (distribute_vertical_auto, false),
    );
    base.x = Some(x);
    base.y = Some(y);
    let has_left = has_offset(style, "left");
    let has_right = has_offset(style, "right");
    let has_top = has_offset(style, "top");
    let has_bottom = has_offset(style, "bottom");
    base.constraints = Some(Constraints {
        h: match (has_left, has_right) {
            (true, true) => HConstraint::LeftRight,
            (false, true) => HConstraint::Right,
            _ => HConstraint::Left,
        },
        v: match (has_top, has_bottom) {
            (true, true) => VConstraint::TopBottom,
            (false, true) => VConstraint::Bottom,
            _ => VConstraint::Top,
        },
    });
    if left_percent || top_percent || right_percent || bottom_percent {
        context.warn_once(ImportWarning::PercentageAbsoluteOffsetInferred);
    }
}

/// The element's own used size on one axis derived from its computed style
/// alone. Used where the resolved `ContainerProps` is not available (pseudo
/// elements, replaced elements, the document root).
pub(super) fn style_axis_size(
    style: &ComputedStyle,
    context: &MapCtx<'_>,
    property: &str,
    reference: f64,
) -> f64 {
    own_size(style.get(property), style.font_size, reference, context)
}

/// CSS `position: relative` offsets. Jian has no "shift without affecting
/// flow" semantics, so the caller re-parents the node inside a fixed-size
/// wrapper and offsets it there; the surrounding flow keeps the original box.
///
/// The approximation itself is reported by `mapper_offset::wrap_offset`, at
/// the point the wrapper is actually built (or refused) — describing a wrapper
/// here would be wrong for every caller that never builds one.
pub(super) fn relative_offset(style: &ComputedStyle, context: &mut MapCtx<'_>) -> (f64, f64) {
    if style.get("position") != Some("relative") {
        return (0.0, 0.0);
    }
    let axis = |start: &str, end: &str, reference: f64, context: &MapCtx<'_>| {
        let (value, percent) =
            position_value(style.get(start), style.font_size, reference, context);
        let (value, percent) = match value {
            Some(value) => (Some(value), percent),
            None => {
                let (value, percent) =
                    position_value(style.get(end), style.font_size, reference, context);
                (value.map(|value| -value), percent)
            }
        };
        (
            value.filter(|value| value.is_finite()).unwrap_or(0.0),
            percent,
        )
    };
    let (x, x_percent) = axis("left", "right", context.containing_width, context);
    let (y, y_percent) = axis("top", "bottom", context.containing_height, context);
    if (x != 0.0 && x_percent) || (y != 0.0 && y_percent) {
        context.warn_once(ImportWarning::PercentageRelativeOffsetInferred);
    }
    (x, y)
}

/// Bake `aspect-ratio` into the missing axis. Runs after the sizing defaults
/// so a Tailwind `aspect-video` box whose width became `FillContainer` still
/// resolves a concrete height.
pub(super) fn apply_aspect_ratio(
    container: &mut ContainerProps,
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
) {
    let limits = container.limits;
    apply_aspect_ratio_axes(
        &mut container.width,
        &mut container.height,
        &limits,
        style,
        context,
    );
}

/// Axis form, shared with the replaced-element path in `special.rs` where the
/// two axes do not live inside a `ContainerProps`.
pub(crate) fn apply_aspect_ratio_axes(
    width: &mut Option<SizingBehavior>,
    height: &mut Option<SizingBehavior>,
    limits: &SizeLimits,
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
) {
    let Some(ratio) = parse_aspect_ratio(style.get("aspect-ratio")) else {
        return;
    };
    let width_is_auto = axis_is_auto(width.as_ref());
    let height_is_auto = axis_is_auto(height.as_ref());
    if width_is_auto == height_is_auto {
        // Both axes auto (no anchor to derive from) or both authored
        // (the author's explicit sizes win over the ratio).
        if width_is_auto {
            context.warn_once(ImportWarning::AspectRatioNoDefiniteAxis);
        }
        return;
    }
    if height_is_auto {
        // A non-`Number` anchor (a `FillContainer` `width:100%`) only resolves
        // through the containing block. When that block is itself indefinite —
        // a shrink-to-fit ancestor, say — `resolved_axis` would hand back the
        // viewport and bake a hard pixel height from a number CSS never had.
        let anchor_is_number =
            matches!(width.as_ref(), Some(SizingBehavior::Number(value)) if value.is_finite());
        if !anchor_is_number && !context.containing_width_is_definite {
            context.warn_once(ImportWarning::AspectRatioIndefiniteContainer);
            return;
        }
        let resolved = resolved_axis(
            width.as_ref(),
            limits.min_width,
            limits.max_width,
            context.containing_width,
        );
        if resolved > 0.0 {
            *height = Some(SizingBehavior::Number(resolved / ratio));
        }
    } else {
        // The block-axis anchor has no `containing_height_is_definite` twin to
        // check; heights are indefinite far more often than widths, so a
        // `height:100%` anchor here is already best-effort.
        let resolved = resolved_axis(
            height.as_ref(),
            limits.min_height,
            limits.max_height,
            context.containing_height,
        );
        if resolved > 0.0 {
            *width = Some(SizingBehavior::Number(resolved * ratio));
        }
    }
}

/// `FitContent` is what the sizing defaults leave behind for an auto axis,
/// so it counts as "not authored" for aspect-ratio purposes.
fn axis_is_auto(sizing: Option<&SizingBehavior>) -> bool {
    matches!(
        sizing,
        None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    )
}

/// `aspect-ratio: <w> [/ <h>]`. `auto` and the `auto <ratio>` pair form both
/// resolve to the plain ratio when one is present.
pub(super) fn parse_aspect_ratio(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("auto") {
        return None;
    }
    let mut width = None;
    let mut height = None;
    for token in value.split('/') {
        for part in token.split_whitespace() {
            if part.eq_ignore_ascii_case("auto") {
                continue;
            }
            let number = part.parse::<f64>().ok()?;
            if width.is_none() {
                width = Some(number);
            } else if height.is_none() {
                height = Some(number);
            } else {
                return None;
            }
        }
    }
    let ratio = width? / height.unwrap_or(1.0);
    (ratio.is_finite() && ratio > 0.0).then_some(ratio)
}

/// Multiply a numeric axis by a `transform: scale()` factor. Auto axes are
/// left alone: replacing `FitContent` with a guessed number would break
/// content hugging far worse than losing the scale.
pub(super) fn scale_axis(sizing: &mut Option<SizingBehavior>, factor: f64) -> bool {
    match sizing {
        Some(SizingBehavior::Number(value)) => {
            *value *= factor;
            true
        }
        _ => false,
    }
}

fn own_size(value: Option<&str>, font_size: f64, reference: f64, context: &MapCtx<'_>) -> f64 {
    value
        .and_then(|value| {
            parse_length(
                value,
                &LengthCtx {
                    font_size,
                    root_font_size: context.opts.base_font_size,
                    viewport_w: context.opts.viewport_width,
                    viewport_h: context.opts.viewport_height(),
                },
            )
        })
        .map(|length| length.resolve(reference))
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
}

fn has_offset(style: &ComputedStyle, name: &str) -> bool {
    style
        .get(name)
        .is_some_and(|value| !value.trim().eq_ignore_ascii_case("auto"))
}

fn position_value(
    value: Option<&str>,
    font_size: f64,
    reference: f64,
    context: &MapCtx<'_>,
) -> (Option<f64>, bool) {
    let Some(value) = value else {
        return (None, false);
    };
    let length = parse_length(
        value,
        &LengthCtx {
            font_size,
            root_font_size: context.opts.base_font_size,
            viewport_w: context.opts.viewport_width,
            viewport_h: context.opts.viewport_height(),
        },
    );
    length.map_or((None, false), |length| {
        let depends_on_reference = length.depends_on_reference();
        (Some(length.resolve(reference)), depends_on_reference)
    })
}

pub(super) fn resolve_absolute_fill(sizing: &mut Option<SizingBehavior>, reference: f64) {
    if matches!(
        sizing,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ) {
        *sizing = Some(SizingBehavior::Number(reference.max(0.0)));
    }
}

pub(super) fn stretched_absolute_axis(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    start: &str,
    end: &str,
    reference: f64,
) -> f64 {
    let inset = |name: &str| {
        position_value(style.get(name), style.font_size, reference, context)
            .0
            .unwrap_or(0.0)
    };
    let start_inset = inset(start);
    let end_inset = inset(end);
    let (margin_start, margin_end) = match start {
        "left" => ("margin-left", "margin-right"),
        "top" => ("margin-top", "margin-bottom"),
        _ => return (reference - start_inset - end_inset).max(0.0),
    };
    let margin_reference = context.containing_width;
    let margin_start =
        positioned_margin::resolve(style, margin_start, margin_reference, context).number();
    let margin_end =
        positioned_margin::resolve(style, margin_end, margin_reference, context).number();
    (reference - start_inset - end_inset - margin_start - margin_end).max(0.0)
}

pub(super) fn warn_for_degradations(
    style: &ComputedStyle,
    supports_image_blend: bool,
    context: &mut MapCtx<'_>,
) {
    if style.get("position") == Some("sticky") {
        context.warn_once(ImportWarning::PositionStickyIgnored);
    }
    if matches!(style.get("display"), Some("grid" | "inline-grid")) {
        if let Some(columns) = style.get("grid-template-columns") {
            if super::grid::grid_column_count(style).is_none()
                && !columns.contains("auto-fit")
                && !columns.contains("auto-fill")
            {
                context.warn_once(ImportWarning::GridTracksApproximated);
            }
        }
    }
    if style
        .get("float")
        .is_some_and(|value| value != "none" && value != "initial")
    {
        context.warn_once(ImportWarning::FloatIgnored);
    }
    if !supports_image_blend
        && style
            .get("mix-blend-mode")
            .is_some_and(|value| value != "normal")
    {
        context.warn_once(ImportWarning::MixBlendModeNoNodeEquivalent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn style(pairs: &[(&str, &str)]) -> ComputedStyle {
        ComputedStyle {
            props: pairs
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<BTreeMap<_, _>>(),
            font_size: 16.0,
        }
    }

    fn context(options: &crate::HtmlImportOptions) -> MapCtx<'_> {
        MapCtx {
            opts: options,
            rules: &[],
            warnings: Vec::new(),
            warned: Default::default(),
            next_id: 0,
            node_count: 0,
            containing_width: options.viewport_width,
            containing_height: options.viewport_height(),
            containing_width_is_definite: true,
            positioned_width: options.viewport_width,
            positioned_height: options.viewport_height(),
            auto_margin_handled_by_parent: false,
            pending_base_outcome: Default::default(),
        }
    }

    #[test]
    fn parses_the_ratio_forms_tailwind_emits() {
        assert_eq!(parse_aspect_ratio(Some("16 / 9")), Some(16.0 / 9.0));
        assert_eq!(parse_aspect_ratio(Some("1/1")), Some(1.0));
        assert_eq!(parse_aspect_ratio(Some("1.5")), Some(1.5));
        assert_eq!(parse_aspect_ratio(Some("auto")), None);
        assert_eq!(parse_aspect_ratio(Some("auto 4 / 3")), Some(4.0 / 3.0));
        assert_eq!(parse_aspect_ratio(Some("0 / 5")), None);
        assert_eq!(parse_aspect_ratio(None), None);
    }

    #[test]
    fn fills_whichever_axis_the_author_left_auto() {
        let options = crate::HtmlImportOptions::default();
        let mut context = context(&options);
        let style = style(&[("aspect-ratio", "16 / 9")]);

        // Definite width, auto height (the Tailwind `aspect-video` case).
        let mut width = Some(SizingBehavior::Number(640.0));
        let mut height = Some(SizingBehavior::Keyword(SizingKeyword::FitContent));
        apply_aspect_ratio_axes(
            &mut width,
            &mut height,
            &SizeLimits::default(),
            &style,
            &mut context,
        );
        assert_eq!(height, Some(SizingBehavior::Number(360.0)));

        // Definite height, auto width.
        let mut width = None;
        let mut height = Some(SizingBehavior::Number(180.0));
        apply_aspect_ratio_axes(
            &mut width,
            &mut height,
            &SizeLimits::default(),
            &style,
            &mut context,
        );
        assert_eq!(width, Some(SizingBehavior::Number(320.0)));

        // Both authored: the explicit sizes win, silently.
        let mut width = Some(SizingBehavior::Number(100.0));
        let mut height = Some(SizingBehavior::Number(100.0));
        apply_aspect_ratio_axes(
            &mut width,
            &mut height,
            &SizeLimits::default(),
            &style,
            &mut context,
        );
        assert_eq!(width, Some(SizingBehavior::Number(100.0)));
        assert_eq!(height, Some(SizingBehavior::Number(100.0)));
        assert!(context.warnings.is_empty(), "{:?}", context.warnings);

        // Neither axis definite: nothing to anchor on, so warn.
        let mut width = Some(SizingBehavior::Keyword(SizingKeyword::FitContent));
        let mut height = None;
        apply_aspect_ratio_axes(
            &mut width,
            &mut height,
            &SizeLimits::default(),
            &style,
            &mut context,
        );
        assert!(height.is_none());
        assert_eq!(context.warnings.len(), 1);
    }

    #[test]
    fn a_fill_container_width_resolves_against_the_containing_block() {
        let options = crate::HtmlImportOptions {
            viewport_width: 1000.0,
            ..Default::default()
        };
        let mut context = context(&options);
        let mut width = Some(SizingBehavior::Keyword(SizingKeyword::FillContainer));
        let mut height = Some(SizingBehavior::Keyword(SizingKeyword::FitContent));
        apply_aspect_ratio_axes(
            &mut width,
            &mut height,
            &SizeLimits::default(),
            &style(&[("aspect-ratio", "2")]),
            &mut context,
        );
        assert_eq!(height, Some(SizingBehavior::Number(500.0)));
    }
}
