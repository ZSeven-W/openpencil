//! Margin terms in the absolute/fixed positioning equations.

use crate::css::cascade::ComputedStyle;
use crate::import_warning::ImportWarning;
use crate::length::{parse_length, LengthCtx};

use super::MapCtx;

#[derive(Clone, Copy)]
pub(super) enum PositionedMargin {
    Auto,
    Number(f64),
}

impl PositionedMargin {
    pub(super) fn number(self) -> f64 {
        match self {
            Self::Auto => 0.0,
            Self::Number(value) => value,
        }
    }
}

pub(super) fn resolve(
    style: &ComputedStyle,
    name: &str,
    reference: f64,
    context: &mut MapCtx<'_>,
) -> PositionedMargin {
    let Some(value) = style.get(name) else {
        return PositionedMargin::Number(0.0);
    };
    if value.trim().eq_ignore_ascii_case("auto") {
        return PositionedMargin::Auto;
    }
    let Some(length) = parse_length(
        value,
        &LengthCtx {
            font_size: style.font_size,
            root_font_size: context.opts.base_font_size,
            viewport_w: context.opts.viewport_width,
            viewport_h: context.opts.viewport_height(),
        },
    ) else {
        context.warn_once(ImportWarning::MarginsOnVisualBoxIgnored);
        return PositionedMargin::Number(0.0);
    };
    let resolved = length.resolve(reference);
    if !resolved.is_finite() {
        context.warn_once(ImportWarning::MarginsOnVisualBoxIgnored);
        return PositionedMargin::Number(0.0);
    }
    PositionedMargin::Number(resolved)
}

pub(super) fn axis_origin(
    start: Option<f64>,
    end: Option<f64>,
    size: f64,
    reference: f64,
    margins: (PositionedMargin, PositionedMargin),
    auto_rules: (bool, bool),
) -> f64 {
    let (margin_start, margin_end) = margins;
    let (distribute_auto, negative_auto_to_start) = auto_rules;
    match (start, end) {
        (Some(start), Some(end)) => {
            let fixed = reference - start - end - size;
            let numeric = margin_start.number() + margin_end.number();
            let free = fixed - numeric;
            let resolved_start = if distribute_auto {
                match (margin_start, margin_end) {
                    (PositionedMargin::Auto, PositionedMargin::Auto) if free >= 0.0 => free / 2.0,
                    (PositionedMargin::Auto, PositionedMargin::Auto) if negative_auto_to_start => {
                        free
                    }
                    (PositionedMargin::Auto, PositionedMargin::Auto) => 0.0,
                    (PositionedMargin::Auto, PositionedMargin::Number(_)) => free,
                    _ => margin_start.number(),
                }
            } else {
                margin_start.number()
            };
            start + resolved_start
        }
        (Some(start), None) => start + margin_start.number(),
        (None, Some(end)) => reference - size - end - margin_end.number(),
        (None, None) => margin_start.number(),
    }
}
