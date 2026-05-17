//! Cubic-Bezier path bounding-box math for SVG import — split out
//! of `svg_import.rs` to keep that file under the 800-line cap.
//!
//! Mirrors `op-pen-loader::path_bounds` (which is private to that
//! crate and not reachable from `op-editor-core`).

use jian_ops_schema::node::PenPathAnchor;

/// Bounding box of a path's anchors **including cubic-bezier curve
/// extrema**. A curve bulges past its anchor points toward the control
/// handles, so an anchor-only box would clip imported curves and give
/// the Path node a too-small frame. Mirrors `op-pen-loader::path_bounds
/// ::path_bounds_from_anchors`. Handles are anchor-relative deltas.
/// Returns `(min_x, min_y, max_x, max_y)`.
pub(crate) fn path_anchor_bounds(anchors: &[PenPathAnchor], closed: bool) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut acc = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };
    let n = anchors.len();
    if n == 1 {
        acc(anchors[0].x, anchors[0].y);
    }
    // `n` segments when closed (the last wraps to the first), else
    // `n - 1`.
    let seg_count = if closed && n > 1 {
        n
    } else {
        n.saturating_sub(1)
    };
    for i in 0..seg_count {
        let from = &anchors[i];
        let to = &anchors[(i + 1) % n];
        let (p0x, p0y) = (from.x, from.y);
        let (p3x, p3y) = (to.x, to.y);
        acc(p0x, p0y);
        acc(p3x, p3y);
        // Reconstruct the control points: handle is a delta from its
        // anchor; a missing handle collapses to the anchor (the
        // segment is then a straight line, contributing no extrema).
        let (p1x, p1y) = match &from.handle_out {
            Some(h) => (p0x + h.x, p0y + h.y),
            None => (p0x, p0y),
        };
        let (p2x, p2y) = match &to.handle_in {
            Some(h) => (p3x + h.x, p3y + h.y),
            None => (p3x, p3y),
        };
        for t in cubic_extrema_roots(p0x, p1x, p2x, p3x) {
            acc(
                eval_cubic(p0x, p1x, p2x, p3x, t),
                eval_cubic(p0y, p1y, p2y, p3y, t),
            );
        }
        for t in cubic_extrema_roots(p0y, p1y, p2y, p3y) {
            acc(
                eval_cubic(p0x, p1x, p2x, p3x, t),
                eval_cubic(p0y, p1y, p2y, p3y, t),
            );
        }
    }
    // `acc`'s captures (`&mut min_x` …) are released by NLL at its
    // last call above — the accumulators are free to read here.
    if !min_x.is_finite() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    (min_x, min_y, max_x, max_y)
}

/// Real roots of a cubic Bezier's derivative on the open interval
/// `(0, 1)` — the parameter values where the curve reaches an extremum
/// on one axis. Port of `op-pen-loader::path_bounds::
/// cubic_derivative_roots` in `f64`.
fn cubic_extrema_roots(p0: f64, p1: f64, p2: f64, p3: f64) -> Vec<f64> {
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

/// Evaluate one axis of a cubic Bezier at parameter `t`.
fn eval_cubic(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let mt = 1.0 - t;
    mt * mt * mt * p0 + 3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t * p3
}
