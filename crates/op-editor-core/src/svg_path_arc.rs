//! Elliptical arc bounding-box math (SVG 1.1 / SVG 2 Implementation Notes F.6.5 & F.6.6).

use std::f64::consts::{PI, TAU};

/// Include the full extent of an SVG elliptical arc segment from `start` to `end`
/// in a bounding-box accumulator.
///
/// Converts SVG endpoint parameterization `(rx, ry, rot_deg, large_arc, sweep, end)`
/// to center parameterization `(cx, cy, rx, ry, theta1, dtheta)` per SVG 1.1 F.6.5/F.6.6,
/// then evaluates the endpoints and any axis-aligned extrema inside the swept angle range.
#[allow(clippy::too_many_arguments)]
pub(crate) fn include_arc_bounds(
    include: &mut impl FnMut(f64, f64),
    start: (f64, f64),
    rx: f64,
    ry: f64,
    rot_deg: f64,
    large_arc: f64,
    sweep: f64,
    end: (f64, f64),
) {
    let (x1, y1) = start;
    let (x2, y2) = end;
    if x1.is_finite() && y1.is_finite() {
        include(x1, y1);
    }
    if x2.is_finite() && y2.is_finite() {
        include(x2, y2);
    }
    if ![x1, y1, rx, ry, rot_deg, large_arc, sweep, x2, y2]
        .into_iter()
        .all(f64::is_finite)
    {
        return;
    }

    // SVG F.6.2: identical endpoints omit the arc; zero radii fall back to a straight line.
    if (x1 == x2 && y1 == y2) || rx == 0.0 || ry == 0.0 {
        return;
    }

    // SVG F.6.6.1: ensure radii are positive.
    let mut rx = rx.abs();
    let mut ry = ry.abs();

    let phi = rot_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    // SVG F.6.5.1: midpoint-centered rotated coordinates (x1', y1').
    let dx2 = (x1 - x2) * 0.5;
    let dy2 = (y1 - y2) * 0.5;
    let x1p = cos_phi * dx2 + sin_phi * dy2;
    let y1p = -sin_phi * dx2 + cos_phi * dy2;

    // SVG F.6.6.2-3: scale radii up when too small to connect the endpoints.
    let lambda = (x1p / rx).powi(2) + (y1p / ry).powi(2);
    if !lambda.is_finite() || lambda <= 0.0 {
        return;
    }
    let large_flag = large_arc != 0.0;
    let sweep_flag = sweep != 0.0;
    let (cxp, cyp) = if lambda > 1.0 {
        let scale = lambda.sqrt();
        rx *= scale;
        ry *= scale;
        if !rx.is_finite() || !ry.is_finite() {
            return;
        }
        (0.0, 0.0)
    } else {
        // SVG F.6.5.2: (rx^2 ry^2 - rx^2 y1'^2 - ry^2 x1'^2) / (rx^2 y1'^2 + ry^2 x1'^2)
        // simplifies to (1 - lambda) / lambda without fourth-power overflow.
        let sign = if large_flag == sweep_flag { -1.0 } else { 1.0 };
        let coef = sign * ((1.0 - lambda) / lambda).max(0.0).sqrt();
        (coef * (rx * y1p / ry), -coef * (ry * x1p / rx))
    };

    // SVG F.6.5.3: center (cx, cy) in user space.
    let mid_x = (x1 + x2) * 0.5;
    let mid_y = (y1 + y2) * 0.5;
    let cx = cos_phi * cxp - sin_phi * cyp + mid_x;
    let cy = sin_phi * cxp + cos_phi * cyp + mid_y;

    // SVG F.6.5.4-6: start angle theta1 and sweep angle dtheta.
    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;

    let theta1 = uy.atan2(ux);
    let mut dtheta = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    if !sweep_flag && dtheta > 0.0 {
        dtheta -= TAU;
    } else if sweep_flag && dtheta < 0.0 {
        dtheta += TAU;
    }

    // Analytical axis-aligned extrema of:
    //   x(t) = cx + rx * cos(phi) * cos(t) - ry * sin(phi) * sin(t)
    //   y(t) = cy + rx * sin(phi) * cos(t) + ry * cos(phi) * sin(t)
    let ax = (-ry * sin_phi).atan2(rx * cos_phi);
    let ay = (ry * cos_phi).atan2(rx * sin_phi);
    for alpha in [ax, ax + PI, ay, ay + PI] {
        if angle_in_arc(alpha, theta1, dtheta) {
            let (sin_t, cos_t) = alpha.sin_cos();
            let px = cx + rx * cos_phi * cos_t - ry * sin_phi * sin_t;
            let py = cy + rx * sin_phi * cos_t + ry * cos_phi * sin_t;
            if px.is_finite() && py.is_finite() {
                include(px, py);
            }
        }
    }
}

fn angle_in_arc(alpha: f64, theta1: f64, dtheta: f64) -> bool {
    // Skip endpoints (0 and |dtheta|): they are already included directly, and
    // re-evaluating them via trig can introduce ~1e-15 drift that trips ceil/floor.
    const EPS: f64 = 1e-12;
    let span = dtheta.abs();
    if span <= EPS {
        return false;
    }
    let offset = if dtheta > 0.0 {
        (alpha - theta1).rem_euclid(TAU)
    } else {
        (theta1 - alpha).rem_euclid(TAU)
    };
    offset > EPS && offset + EPS < span
}
