//! Position-aware colour sampling for contrast repairs.

use super::ResolvedRect;
use op_design_lint::node_util::{resolve_color_ref, Theme, Variables};

#[derive(Clone)]
pub(super) struct GradientSource {
    kind: GradientKind,
    stops: Vec<(f64, String)>,
}

#[derive(Clone, Copy)]
enum GradientKind {
    Linear { angle_deg: f64 },
    Radial { cx: f64, cy: f64 },
}

impl GradientSource {
    pub(super) fn linear(angle_deg: f64, stops: Vec<(f64, String)>) -> Self {
        Self {
            kind: GradientKind::Linear { angle_deg },
            stops,
        }
    }

    pub(super) fn radial(cx: f64, cy: f64, stops: Vec<(f64, String)>) -> Self {
        Self {
            kind: GradientKind::Radial {
                cx: cx.clamp(0.0, 1.0),
                cy: cy.clamp(0.0, 1.0),
            },
            stops,
        }
    }
}

#[derive(Clone)]
pub(super) struct ResolvedGradient {
    kind: GradientKind,
    stops: Vec<ResolvedStop>,
}

#[derive(Clone)]
struct ResolvedStop {
    offset: f64,
    color: String,
}

impl ResolvedGradient {
    pub(super) fn colors(&self) -> Vec<String> {
        self.stops.iter().map(|stop| stop.color.clone()).collect()
    }
}

pub(super) fn resolve_gradient(
    source: GradientSource,
    variables: &Variables,
    theme: &Theme,
) -> Option<ResolvedGradient> {
    let mut stops: Vec<ResolvedStop> = source
        .stops
        .iter()
        .filter_map(|(offset, raw)| {
            let offset = offset.is_finite().then_some(offset.clamp(0.0, 1.0))?;
            let color = resolve_color_ref(raw, variables, theme)?;
            let rgba = super::parse_color_rgba(&color)?;
            Some(ResolvedStop {
                offset,
                color: super::rgb_hex(rgba),
            })
        })
        .collect();
    if stops.is_empty() {
        return None;
    }
    stops.sort_by(|left, right| left.offset.total_cmp(&right.offset));
    Some(ResolvedGradient {
        kind: source.kind,
        stops,
    })
}

/// Sample a resolved gradient at the centre of a text node.
///
/// Linear gradients use the same ellipse endpoints as the native/web
/// renderer. The renderer's canonical convention is
/// `rad = (angle - 90°)`, with 0° bottom→top, 90° left→right, and 180°
/// top→bottom. The projected coordinate is
/// `t = dot(text_center - start, end - start) / |end - start|²`.
/// Radial gradients use the requested half-diagonal normalization:
/// `t = distance(text_center, gradient_center) / (sqrt(w² + h²) / 2)`.
pub(super) fn sample_at_text(
    gradient: &ResolvedGradient,
    owner: ResolvedRect,
    text: ResolvedRect,
) -> Option<String> {
    if !valid_rect(owner) || !valid_rect(text) {
        return None;
    }
    let text_center = (text.x + text.w / 2.0, text.y + text.h / 2.0);
    let t = match gradient.kind {
        GradientKind::Linear { angle_deg } => {
            let center = (owner.x + owner.w / 2.0, owner.y + owner.h / 2.0);
            let rad = (angle_deg - 90.0).to_radians();
            let direction = (rad.cos() * owner.w, rad.sin() * owner.h);
            let start = (center.0 - direction.0 / 2.0, center.1 - direction.1 / 2.0);
            let denominator = direction.0.mul_add(direction.0, direction.1 * direction.1);
            if denominator <= f64::EPSILON || !denominator.is_finite() {
                return None;
            }
            let delta = (text_center.0 - start.0, text_center.1 - start.1);
            delta.0.mul_add(direction.0, delta.1 * direction.1) / denominator
        }
        GradientKind::Radial { cx, cy } => {
            let center = (owner.x + owner.w * cx, owner.y + owner.h * cy);
            let distance = (text_center.0 - center.0).hypot(text_center.1 - center.1);
            let half_diagonal = owner.w.hypot(owner.h) / 2.0;
            if half_diagonal <= f64::EPSILON || !half_diagonal.is_finite() {
                return None;
            }
            distance / half_diagonal
        }
    };
    t.is_finite()
        .then(|| sample_stops(&gradient.stops, t.clamp(0.0, 1.0)))
        .flatten()
}

fn valid_rect(rect: ResolvedRect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.w.is_finite()
        && rect.h.is_finite()
        && rect.w > 0.0
        && rect.h > 0.0
}

fn sample_stops(stops: &[ResolvedStop], t: f64) -> Option<String> {
    let first = stops.first()?;
    if t <= first.offset {
        return Some(first.color.clone());
    }
    for pair in stops.windows(2) {
        let [left, right] = pair else {
            continue;
        };
        if t <= right.offset {
            let span = right.offset - left.offset;
            let amount = if span <= f64::EPSILON {
                1.0
            } else {
                (t - left.offset) / span
            };
            return interpolate_srgb(&left.color, &right.color, amount);
        }
    }
    stops.last().map(|stop| stop.color.clone())
}

fn interpolate_srgb(left: &str, right: &str, amount: f64) -> Option<String> {
    let left = super::parse_color_rgba(left)?;
    let right = super::parse_color_rgba(right)?;
    let amount = amount.clamp(0.0, 1.0);
    let channel = |left: u8, right: u8| {
        (f64::from(left) + (f64::from(right) - f64::from(left)) * amount)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Some(super::rgb_hex([
        channel(left[0], right[0]),
        channel(left[1], right[1]),
        channel(left[2], right[2]),
        u8::MAX,
    ]))
}
