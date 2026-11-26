//! SVG path `d` helpers used by import.
//!
//! The editable path model stores anchors. Imported SVG paths need a
//! separate preserve-`d` path so arc commands and compound fill rules
//! keep the same visual result as the TS renderer.

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SvgPathBounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PathToken {
    Cmd(u8),
    Num(f64),
}

#[derive(Debug, Clone, PartialEq)]
enum Segment {
    Move(f64, f64),
    Line(f64, f64),
    HLine(f64),
    VLine(f64),
    Cubic(f64, f64, f64, f64, f64, f64),
    SmoothCubic(f64, f64, f64, f64),
    Quad(f64, f64, f64, f64),
    SmoothQuad(f64, f64),
    Arc(f64, f64, f64, f64, f64, f64, f64),
    Close,
}

/// Convert any supported relative commands to absolute commands,
/// compute a coarse path bbox, then shift absolute coordinates so
/// the path is local to its own bbox origin.
pub(crate) fn localize_svg_path(d: &str) -> Option<(String, SvgPathBounds)> {
    let segments = absolute_segments(d)?;
    let tight = segment_bounds(&segments)?;
    let bounds = SvgPathBounds {
        x: tight.x.floor(),
        y: tight.y.floor(),
        w: (tight.x + tight.w - tight.x.floor()).ceil().max(1.0),
        h: (tight.y + tight.h - tight.y.floor()).ceil().max(1.0),
    };
    let local = serialize_local_segments(&segments, bounds.x, bounds.y);
    Some((local, bounds))
}

pub(crate) fn svg_path_bounds(d: &str) -> Option<SvgPathBounds> {
    segment_bounds(&absolute_segments(d)?)
}

fn absolute_segments(d: &str) -> Option<Vec<Segment>> {
    let tokens = tokenize_path(d);
    if tokens.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    let mut ti = 0usize;
    let mut cmd = b' ';
    let (mut cx, mut cy) = (0.0f64, 0.0f64);
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    while ti < tokens.len() {
        if let PathToken::Cmd(c) = tokens[ti] {
            cmd = c;
            ti += 1;
        }
        let rel = cmd.is_ascii_lowercase();
        let up = cmd.to_ascii_uppercase();
        let need = match up {
            b'M' | b'L' | b'T' => 2,
            b'H' | b'V' => 1,
            b'C' => 6,
            b'S' | b'Q' => 4,
            b'A' => 7,
            b'Z' => 0,
            _ => return None,
        };
        if up == b'Z' {
            out.push(Segment::Close);
            cx = sx;
            cy = sy;
            cmd = b' ';
            continue;
        }
        let mut args = [0.0f64; 7];
        let mut got = 0usize;
        while got < need && ti < tokens.len() {
            if let PathToken::Num(n) = tokens[ti] {
                args[got] = n;
                got += 1;
                ti += 1;
            } else {
                break;
            }
        }
        if got < need {
            return None;
        }
        match up {
            b'M' => {
                let (x, y) = abs_pt(rel, cx, cy, args[0], args[1]);
                out.push(Segment::Move(x, y));
                cx = x;
                cy = y;
                sx = x;
                sy = y;
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => {
                let (x, y) = abs_pt(rel, cx, cy, args[0], args[1]);
                out.push(Segment::Line(x, y));
                cx = x;
                cy = y;
            }
            b'H' => {
                let x = if rel { cx + args[0] } else { args[0] };
                out.push(Segment::HLine(x));
                cx = x;
            }
            b'V' => {
                let y = if rel { cy + args[0] } else { args[0] };
                out.push(Segment::VLine(y));
                cy = y;
            }
            b'C' => {
                let (x1, y1) = abs_pt(rel, cx, cy, args[0], args[1]);
                let (x2, y2) = abs_pt(rel, cx, cy, args[2], args[3]);
                let (x, y) = abs_pt(rel, cx, cy, args[4], args[5]);
                out.push(Segment::Cubic(x1, y1, x2, y2, x, y));
                cx = x;
                cy = y;
            }
            b'S' => {
                let (x2, y2) = abs_pt(rel, cx, cy, args[0], args[1]);
                let (x, y) = abs_pt(rel, cx, cy, args[2], args[3]);
                out.push(Segment::SmoothCubic(x2, y2, x, y));
                cx = x;
                cy = y;
            }
            b'Q' => {
                let (x1, y1) = abs_pt(rel, cx, cy, args[0], args[1]);
                let (x, y) = abs_pt(rel, cx, cy, args[2], args[3]);
                out.push(Segment::Quad(x1, y1, x, y));
                cx = x;
                cy = y;
            }
            b'T' => {
                let (x, y) = abs_pt(rel, cx, cy, args[0], args[1]);
                out.push(Segment::SmoothQuad(x, y));
                cx = x;
                cy = y;
            }
            b'A' => {
                let (x, y) = abs_pt(rel, cx, cy, args[5], args[6]);
                out.push(Segment::Arc(
                    args[0], args[1], args[2], args[3], args[4], x, y,
                ));
                cx = x;
                cy = y;
            }
            _ => return None,
        }
    }
    Some(out)
}

fn segment_bounds(segments: &[Segment]) -> Option<SvgPathBounds> {
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
    let (mut cx, mut cy) = (0.0f64, 0.0f64);
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    let mut last_cubic_ctrl: Option<(f64, f64)> = None;
    let mut last_quad_ctrl: Option<(f64, f64)> = None;
    for seg in segments {
        match *seg {
            Segment::Move(x, y) => {
                include(x, y);
                cx = x;
                cy = y;
                sx = x;
                sy = y;
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            Segment::Line(x, y) => {
                include(x, y);
                cx = x;
                cy = y;
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            Segment::HLine(x) => {
                include(x, cy);
                cx = x;
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            Segment::VLine(y) => {
                include(cx, y);
                cy = y;
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            Segment::Cubic(x1, y1, x2, y2, x, y) => {
                include_cubic_bounds(&mut include, [(cx, cy), (x1, y1), (x2, y2), (x, y)]);
                cx = x;
                cy = y;
                last_cubic_ctrl = Some((x2, y2));
                last_quad_ctrl = None;
            }
            Segment::SmoothCubic(x2, y2, x, y) => {
                let (x1, y1) = last_cubic_ctrl
                    .map(|(px, py)| (2.0 * cx - px, 2.0 * cy - py))
                    .unwrap_or((cx, cy));
                include_cubic_bounds(&mut include, [(cx, cy), (x1, y1), (x2, y2), (x, y)]);
                cx = x;
                cy = y;
                last_cubic_ctrl = Some((x2, y2));
                last_quad_ctrl = None;
            }
            Segment::Quad(qx, qy, x, y) => {
                let (x1, y1, x2, y2) = quad_to_cubic(cx, cy, qx, qy, x, y);
                include_cubic_bounds(&mut include, [(cx, cy), (x1, y1), (x2, y2), (x, y)]);
                cx = x;
                cy = y;
                last_quad_ctrl = Some((qx, qy));
                last_cubic_ctrl = None;
            }
            Segment::SmoothQuad(x, y) => {
                let (qx, qy) = last_quad_ctrl
                    .map(|(px, py)| (2.0 * cx - px, 2.0 * cy - py))
                    .unwrap_or((cx, cy));
                let (x1, y1, x2, y2) = quad_to_cubic(cx, cy, qx, qy, x, y);
                include_cubic_bounds(&mut include, [(cx, cy), (x1, y1), (x2, y2), (x, y)]);
                cx = x;
                cy = y;
                last_quad_ctrl = Some((qx, qy));
                last_cubic_ctrl = None;
            }
            Segment::Arc(rx, ry, rot, large, sweep, x, y) => {
                crate::svg_path_arc::include_arc_bounds(
                    &mut include,
                    (cx, cy),
                    rx,
                    ry,
                    rot,
                    large,
                    sweep,
                    (x, y),
                );
                cx = x;
                cy = y;
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
            Segment::Close => {
                cx = sx;
                cy = sy;
                last_cubic_ctrl = None;
                last_quad_ctrl = None;
            }
        }
    }
    if !min_x.is_finite() {
        return None;
    }
    Some(SvgPathBounds {
        x: min_x,
        y: min_y,
        w: max_x - min_x,
        h: max_y - min_y,
    })
}

fn include_cubic_bounds(include: &mut impl FnMut(f64, f64), points: [(f64, f64); 4]) {
    let [(p0x, p0y), (p1x, p1y), (p2x, p2y), (p3x, p3y)] = points;
    include(p0x, p0y);
    include(p3x, p3y);
    for t in cubic_derivative_roots(p0x, p1x, p2x, p3x) {
        include(
            eval_cubic(p0x, p1x, p2x, p3x, t),
            eval_cubic(p0y, p1y, p2y, p3y, t),
        );
    }
    for t in cubic_derivative_roots(p0y, p1y, p2y, p3y) {
        include(
            eval_cubic(p0x, p1x, p2x, p3x, t),
            eval_cubic(p0y, p1y, p2y, p3y, t),
        );
    }
}

fn quad_to_cubic(x0: f64, y0: f64, qx: f64, qy: f64, x1: f64, y1: f64) -> (f64, f64, f64, f64) {
    (
        x0 + 2.0 / 3.0 * (qx - x0),
        y0 + 2.0 / 3.0 * (qy - y0),
        x1 + 2.0 / 3.0 * (qx - x1),
        y1 + 2.0 / 3.0 * (qy - y1),
    )
}

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

fn eval_cubic(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let mt = 1.0 - t;
    mt * mt * mt * p0 + 3.0 * mt * mt * t * p1 + 3.0 * mt * t * t * p2 + t * t * t * p3
}

fn serialize_local_segments(segments: &[Segment], ox: f64, oy: f64) -> String {
    let mut out = String::new();
    for seg in segments {
        if !out.is_empty() {
            out.push(' ');
        }
        match *seg {
            Segment::Move(x, y) => write_pair(&mut out, 'M', x - ox, y - oy),
            Segment::Line(x, y) => write_pair(&mut out, 'L', x - ox, y - oy),
            Segment::HLine(x) => write_one(&mut out, 'H', x - ox),
            Segment::VLine(y) => write_one(&mut out, 'V', y - oy),
            Segment::Cubic(x1, y1, x2, y2, x, y) => {
                out.push('C');
                push_num(&mut out, x1 - ox);
                push_num(&mut out, y1 - oy);
                push_num(&mut out, x2 - ox);
                push_num(&mut out, y2 - oy);
                push_num(&mut out, x - ox);
                push_num(&mut out, y - oy);
            }
            Segment::SmoothCubic(x2, y2, x, y) => {
                out.push('S');
                push_num(&mut out, x2 - ox);
                push_num(&mut out, y2 - oy);
                push_num(&mut out, x - ox);
                push_num(&mut out, y - oy);
            }
            Segment::Quad(x1, y1, x, y) => {
                out.push('Q');
                push_num(&mut out, x1 - ox);
                push_num(&mut out, y1 - oy);
                push_num(&mut out, x - ox);
                push_num(&mut out, y - oy);
            }
            Segment::SmoothQuad(x, y) => write_pair(&mut out, 'T', x - ox, y - oy),
            Segment::Arc(rx, ry, rot, large, sweep, x, y) => {
                out.push('A');
                push_num(&mut out, rx);
                push_num(&mut out, ry);
                push_num(&mut out, rot);
                push_num(&mut out, large);
                push_num(&mut out, sweep);
                push_num(&mut out, x - ox);
                push_num(&mut out, y - oy);
            }
            Segment::Close => out.push('Z'),
        }
    }
    out
}

fn write_pair(out: &mut String, cmd: char, x: f64, y: f64) {
    out.push(cmd);
    push_num(out, x);
    push_num(out, y);
}

fn write_one(out: &mut String, cmd: char, n: f64) {
    out.push(cmd);
    push_num(out, n);
}

fn push_num(out: &mut String, n: f64) {
    out.push(' ');
    let n = if n.abs() < 1e-9 { 0.0 } else { n };
    out.push_str(
        format!("{n:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.'),
    );
}

fn abs_pt(rel: bool, cx: f64, cy: f64, x: f64, y: f64) -> (f64, f64) {
    if rel {
        (cx + x, cy + y)
    } else {
        (x, y)
    }
}

fn tokenize_path(d: &str) -> Vec<PathToken> {
    let bytes = d.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() {
            out.push(PathToken::Cmd(c));
            i += 1;
        } else if c == b'-' || c == b'+' || c == b'.' || c.is_ascii_digit() {
            let start = i;
            i += 1;
            let mut seen_dot = c == b'.';
            let mut seen_exp = false;
            while i < bytes.len() {
                let d = bytes[i];
                if d.is_ascii_digit() {
                    i += 1;
                } else if d == b'.' && !seen_dot && !seen_exp {
                    seen_dot = true;
                    i += 1;
                } else if (d == b'e' || d == b'E') && !seen_exp {
                    seen_exp = true;
                    i += 1;
                    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            if let Ok(n) = d[start..i].parse::<f64>() {
                out.push(PathToken::Num(n));
            }
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{localize_svg_path, svg_path_bounds, SvgPathBounds};

    fn assert_bounds_within(actual: SvgPathBounds, expected: (f64, f64, f64, f64), eps: f64) {
        assert!(
            (actual.x - expected.0).abs() < eps
                && (actual.y - expected.1).abs() < eps
                && (actual.w - expected.2).abs() < eps
                && (actual.h - expected.3).abs() < eps,
            "expected bounds {expected:?} (±{eps}), got {actual:?}"
        );
    }

    fn assert_bounds_near(actual: SvgPathBounds, expected: (f64, f64, f64, f64)) {
        assert_bounds_within(actual, expected, 1e-6);
    }

    #[test]
    fn preserves_arc_and_compound_moves() {
        let (d, bounds) = localize_svg_path("M10 50 A40 40 0 0 1 90 50 Z M5 5 H15").expect("path");
        assert!(d.contains('A'));
        assert_eq!(d.matches('M').count(), 2);
        assert_eq!(bounds.x, 5.0);
        assert_eq!(bounds.y, 5.0);
    }

    #[test]
    fn arc_semicircle_both_sweep_directions() {
        let up = svg_path_bounds("M10 50 A40 40 0 0 1 90 50").expect("arc up");
        assert_bounds_near(up, (10.0, 10.0, 80.0, 40.0));

        let down = svg_path_bounds("M10 50 A40 40 0 0 0 90 50").expect("arc down");
        assert_bounds_near(down, (10.0, 50.0, 80.0, 40.0));
    }

    #[test]
    fn arc_full_circle_from_two_arcs() {
        // Endpoints sit on x = 50 so horizontal extent (10..90) comes strictly from arc extrema.
        let vertical_split =
            svg_path_bounds("M50 10 A40 40 0 1 1 50 90 A40 40 0 1 1 50 10 Z").expect("circle");
        assert_bounds_near(vertical_split, (10.0, 10.0, 80.0, 80.0));

        let horizontal_split =
            svg_path_bounds("M10 50 A40 40 0 0 1 90 50 A40 40 0 0 1 10 50").expect("circle");
        assert_bounds_near(horizontal_split, (10.0, 10.0, 80.0, 80.0));
    }

    #[test]
    fn arc_rotated_ellipse() {
        // Full ellipse centered at (50, 50) with rx=40, ry=20 rotated 45 deg:
        // half-width = half-height = sqrt((40^2 + 20^2) / 2) = sqrt(1000).
        let r = 40.0 / 2.0_f64.sqrt();
        let d = format!(
            "M {:.15} {:.15} A 40 20 45 1 1 {:.15} {:.15} A 40 20 45 1 1 {:.15} {:.15}",
            50.0 + r,
            50.0 + r,
            50.0 - r,
            50.0 - r,
            50.0 + r,
            50.0 + r
        );
        let bounds = svg_path_bounds(&d).expect("rotated ellipse");
        let half = 1000.0_f64.sqrt();
        assert_bounds_near(bounds, (50.0 - half, 50.0 - half, 2.0 * half, 2.0 * half));

        // Single partial rotated arc bulges outside its endpoint box.
        let partial = svg_path_bounds("M20 50 A40 20 30 0 1 80 50").expect("partial rotated arc");
        assert_bounds_within(partial, (16.399439, 26.849676, 63.600561, 23.150324), 1e-5);

        // Partial rotated major arc with sweep=0 hits all four axis-aligned extrema.
        let major_rev =
            svg_path_bounds("M30 40 A40 20 35 1 0 70 60").expect("rotated major sweep=0 arc");
        assert_bounds_within(
            major_rev,
            (13.910342, 39.968079, 69.432329, 56.383966),
            1e-5,
        );
    }

    #[test]
    fn arc_large_arc_flag() {
        let minor = svg_path_bounds("M50 10 A40 40 0 0 1 90 50").expect("minor arc");
        assert_bounds_near(minor, (50.0, 10.0, 40.0, 40.0));

        let major_cw = svg_path_bounds("M50 10 A40 40 0 1 1 90 50").expect("major cw arc");
        assert_bounds_near(major_cw, (50.0, -30.0, 80.0, 80.0));

        let major_ccw = svg_path_bounds("M50 10 A40 40 0 1 0 90 50").expect("major ccw arc");
        assert_bounds_near(major_ccw, (10.0, 10.0, 80.0, 80.0));
    }

    #[test]
    fn arc_relative_command() {
        let rel = svg_path_bounds("M10 50 a40 40 0 0 1 80 0").expect("relative arc");
        assert_bounds_near(rel, (10.0, 10.0, 80.0, 40.0));
    }

    #[test]
    fn arc_radii_too_small_are_scaled_up() {
        // Endpoints 80 apart, rx=20 ry=10 -> lambda=4 -> scaled to rx=40 ry=20.
        let scaled = svg_path_bounds("M10 50 A20 10 0 0 1 90 50").expect("scaled arc");
        assert_bounds_near(scaled, (10.0, 30.0, 80.0, 20.0));
    }

    #[test]
    fn arc_degenerate_cases() {
        // rx=0 -> straight line segment between endpoints.
        let rx_zero = svg_path_bounds("M10 50 A0 40 30 1 1 90 20").expect("rx=0");
        assert_bounds_near(rx_zero, (10.0, 20.0, 80.0, 30.0));

        // ry=0 -> straight line segment between endpoints.
        let ry_zero = svg_path_bounds("M10 50 A40 0 0 0 1 90 50").expect("ry=0");
        assert_bounds_near(ry_zero, (10.0, 50.0, 80.0, 0.0));

        // Negative radii -> absolute values used.
        let neg_r = svg_path_bounds("M10 50 A-40 -40 0 0 1 90 50").expect("neg radii");
        assert_bounds_near(neg_r, (10.0, 10.0, 80.0, 40.0));

        // Identical endpoints -> arc omitted.
        let same_pt = svg_path_bounds("M10 50 A40 40 0 0 1 10 50").expect("identical endpoints");
        assert_bounds_near(same_pt, (10.0, 50.0, 0.0, 0.0));
    }

    #[test]
    fn localize_svg_path_shifts_by_corrected_arc_origin() {
        let (local, bounds) = localize_svg_path("M10 50 A40 40 0 0 1 90 50").expect("localized");
        assert_eq!(
            bounds,
            SvgPathBounds {
                x: 10.0,
                y: 10.0,
                w: 80.0,
                h: 40.0,
            }
        );
        assert_eq!(local, "M 0 40 A 40 40 0 0 1 80 40");
        assert_eq!(
            crate::svg_path_data_bounds("M10 50 A40 40 0 0 1 90 50"),
            Some((10.0, 10.0, 80.0, 40.0))
        );
    }
}
