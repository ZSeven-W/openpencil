//! Bezier path-anchor bounds — the canonical handle-aware bounding
//! box for a `PenNode::Path`. Ported from
//! `packages/pen-core/src/path-anchors.ts::getPathBoundsFromAnchors`.
//!
//! This is the single source of truth: `pen::refit_path_bounds`
//! writes a path's `width`/`height` from it, and `op-pen-loader`'s
//! `absolutize_path_anchors` reads the same value as the native
//! geometry span — so the absolutize scale stays `1.0` for an
//! editor-authored path (no spurious rescale when a handle is added).

use jian_ops_schema::node::PenPathAnchor;

/// `(min_x, min_y, width, height)` of a path's anchors — endpoints
/// plus the cubic-Bezier extrema of every segment (handle vectors
/// are anchor-relative deltas in the schema).
pub fn path_bounds_from_anchors(anchors: &[PenPathAnchor], closed: bool) -> (f64, f64, f64, f64) {
    if anchors.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut include = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };
    if anchors.len() == 1 {
        include(anchors[0].x, anchors[0].y);
    }
    let mut segment = |from: &PenPathAnchor, to: &PenPathAnchor| {
        let p0 = (from.x, from.y);
        let p3 = (to.x, to.y);
        include(p0.0, p0.1);
        include(p3.0, p3.1);
        let p1 = (
            from.x + from.handle_out.as_ref().map(|h| h.x).unwrap_or(0.0),
            from.y + from.handle_out.as_ref().map(|h| h.y).unwrap_or(0.0),
        );
        let p2 = (
            to.x + to.handle_in.as_ref().map(|h| h.x).unwrap_or(0.0),
            to.y + to.handle_in.as_ref().map(|h| h.y).unwrap_or(0.0),
        );
        for t in cubic_derivative_roots(p0.0, p1.0, p2.0, p3.0) {
            include(
                eval_cubic(p0.0, p1.0, p2.0, p3.0, t),
                eval_cubic(p0.1, p1.1, p2.1, p3.1, t),
            );
        }
        for t in cubic_derivative_roots(p0.1, p1.1, p2.1, p3.1) {
            include(
                eval_cubic(p0.0, p1.0, p2.0, p3.0, t),
                eval_cubic(p0.1, p1.1, p2.1, p3.1, t),
            );
        }
    };
    for i in 1..anchors.len() {
        segment(&anchors[i - 1], &anchors[i]);
    }
    if closed && anchors.len() > 1 {
        segment(&anchors[anchors.len() - 1], &anchors[0]);
    }
    if !min_x.is_finite() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (
        min_x,
        min_y,
        (max_x - min_x).max(0.0),
        (max_y - min_y).max(0.0),
    )
}

/// Real roots of a cubic Bezier's derivative on the open interval
/// `(0, 1)` — the parameter values of the curve's axis extrema.
fn cubic_derivative_roots(p0: f64, p1: f64, p2: f64, p3: f64) -> Vec<f64> {
    const EPS: f64 = 1e-9;
    let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let b = 2.0 * (p0 - 2.0 * p1 + p2);
    let c = -p0 + p1;
    let in_unit = |t: f64| t > 0.0 && t < 1.0;
    if a.abs() <= EPS {
        if b.abs() <= EPS {
            return Vec::new();
        }
        let t = -c / b;
        return if in_unit(t) { vec![t] } else { Vec::new() };
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return Vec::new();
    }
    let s = disc.sqrt();
    let mut out = Vec::with_capacity(2);
    for t in [(-b + s) / (2.0 * a), (-b - s) / (2.0 * a)] {
        if in_unit(t) {
            out.push(t);
        }
    }
    out
}

/// Evaluate a cubic Bezier at `t`.
fn eval_cubic(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let mt = 1.0 - t;
    mt * mt * mt * p0 + 3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t * p3
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::{PenPathAnchor, PenPathHandle};

    fn anchor(x: f64, y: f64, hout: Option<(f64, f64)>) -> PenPathAnchor {
        PenPathAnchor {
            x,
            y,
            handle_in: None,
            handle_out: hout.map(|(x, y)| PenPathHandle { x, y }),
            point_type: None,
        }
    }

    #[test]
    fn endpoint_only_bounds_for_straight_path() {
        let anchors = [anchor(0.0, 0.0, None), anchor(100.0, 40.0, None)];
        assert_eq!(
            path_bounds_from_anchors(&anchors, false),
            (0.0, 0.0, 100.0, 40.0)
        );
    }

    #[test]
    fn handle_extends_bounds_past_endpoints() {
        // A handle bowing the curve up past the endpoints.
        let anchors = [
            anchor(0.0, 0.0, Some((0.0, -80.0))),
            anchor(100.0, 0.0, None),
        ];
        let (_, min_y, _, h) = path_bounds_from_anchors(&anchors, false);
        assert!(min_y < 0.0, "curve extrema reach above the endpoints");
        assert!(h > 0.0);
    }
}
