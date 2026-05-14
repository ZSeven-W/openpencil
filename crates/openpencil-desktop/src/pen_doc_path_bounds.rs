//! Bezier path-anchor bounds + canvas-absolute mapping for the
//! canonical  schema. Ported from
//! `packages/pen-core/src/path-anchors.ts::getPathBoundsFromAnchors`.

/// Port of `getPathBoundsFromAnchors` (packages/pen-core/src/
/// path-anchors.ts). Returns `(min_x, min_y, width, height)`.
/// Includes endpoints AND cubic-Bezier extrema for each segment,
/// matching the TS renderer's `getBounds` shape. The handle vectors
/// are stored relative to their anchor in the canonical schema
/// (`{x, y}` is a delta from `anchor.{x, y}`).
pub(super) fn path_bounds_from_anchors(
    anchors: &[jian_ops_schema::node::PenPathAnchor],
    closed: bool,
) -> (f32, f32, f32, f32) {
    if anchors.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut include = |x: f32, y: f32| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };
    if anchors.len() == 1 {
        include(anchors[0].x as f32, anchors[0].y as f32);
    }
    let mut segment = |from: &jian_ops_schema::node::PenPathAnchor,
                       to: &jian_ops_schema::node::PenPathAnchor| {
        let p0 = (from.x as f32, from.y as f32);
        let p3 = (to.x as f32, to.y as f32);
        include(p0.0, p0.1);
        include(p3.0, p3.1);
        let hout = from.handle_out.as_ref();
        let hin = to.handle_in.as_ref();
        let p1 = (
            from.x as f32 + hout.map(|h| h.x as f32).unwrap_or(0.0),
            from.y as f32 + hout.map(|h| h.y as f32).unwrap_or(0.0),
        );
        let p2 = (
            to.x as f32 + hin.map(|h| h.x as f32).unwrap_or(0.0),
            to.y as f32 + hin.map(|h| h.y as f32).unwrap_or(0.0),
        );
        for t in cubic_derivative_roots(p0.0, p1.0, p2.0, p3.0) {
            include(eval_cubic(p0.0, p1.0, p2.0, p3.0, t), eval_cubic(p0.1, p1.1, p2.1, p3.1, t));
        }
        for t in cubic_derivative_roots(p0.1, p1.1, p2.1, p3.1) {
            include(eval_cubic(p0.0, p1.0, p2.0, p3.0, t), eval_cubic(p0.1, p1.1, p2.1, p3.1, t));
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
    (min_x, min_y, (max_x - min_x).max(0.0), (max_y - min_y).max(0.0))
}

/// Real roots of the derivative of a cubic Bezier on (0, 1).
/// Used by `path_bounds_from_anchors` to find curve extrema.
pub(super) fn cubic_derivative_roots(p0: f32, p1: f32, p2: f32, p3: f32) -> Vec<f32> {
    const EPS: f32 = 1e-6;
    let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let b = 2.0 * (p0 - 2.0 * p1 + p2);
    let c = -p0 + p1;
    let in_unit = |t: f32| t > 0.0 && t < 1.0;
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
    let t1 = (-b + s) / (2.0 * a);
    let t2 = (-b - s) / (2.0 * a);
    let mut out = Vec::with_capacity(2);
    if in_unit(t1) { out.push(t1); }
    if in_unit(t2) { out.push(t2); }
    out
}

pub(super) fn eval_cubic(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    mt * mt * mt * p0
        + 3.0 * mt * mt * t * p1
        + 3.0 * mt * t * t * p2
        + t * t * t * p3
}

