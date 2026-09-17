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
            Segment::Arc(_, _, _, _, _, x, y) => {
                include(x, y);
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

/// Rewrite `d` with every coordinate carried through the affine
/// `m = [a, b, c, d, e, f]` (SVG `matrix()` layout), returning absolute
/// commands. This is how an element's composed `transform` reaches a
/// `<path>` that keeps its `d` verbatim.
///
/// Points, control points and the `S` / `T` reflections all survive an
/// affine map unchanged in meaning, so those commands stay what they
/// are. `H` / `V` become `L`: a horizontal line is only horizontal
/// before a rotation. An arc stays an arc when the matrix is a
/// similarity (uniform scale + rotation, mirrored or not) — its radii
/// scale, its x-axis rotation turns with the matrix, its sweep flips
/// under a mirror — and is otherwise traced as cubic Béziers, which an
/// arbitrary affine map does preserve.
pub(crate) fn transform_svg_path(d: &str, m: [f64; 6]) -> Option<String> {
    let segments = absolute_segments(d)?;
    let [a, b, c, dd, e, f] = m;
    let map = |x: f64, y: f64| (a * x + c * y + e, b * x + dd * y + f);
    let det = a * dd - b * c;
    // A similarity has orthogonal columns of equal length.
    let similarity = {
        let dot = a * c + b * dd;
        let len_a = (a * a + b * b).sqrt();
        let len_c = (c * c + dd * dd).sqrt();
        dot.abs() < 1e-9 * (1.0 + len_a * len_c) && (len_a - len_c).abs() < 1e-9 * (1.0 + len_a)
    };
    let mut out: Vec<Segment> = Vec::with_capacity(segments.len());
    let (mut cx, mut cy) = (0.0f64, 0.0f64);
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    for seg in &segments {
        match *seg {
            Segment::Move(x, y) => {
                let (tx, ty) = map(x, y);
                out.push(Segment::Move(tx, ty));
                cx = x;
                cy = y;
                sx = x;
                sy = y;
            }
            Segment::Line(x, y) => {
                let (tx, ty) = map(x, y);
                out.push(Segment::Line(tx, ty));
                cx = x;
                cy = y;
            }
            Segment::HLine(x) => {
                let (tx, ty) = map(x, cy);
                out.push(Segment::Line(tx, ty));
                cx = x;
            }
            Segment::VLine(y) => {
                let (tx, ty) = map(cx, y);
                out.push(Segment::Line(tx, ty));
                cy = y;
            }
            Segment::Cubic(x1, y1, x2, y2, x, y) => {
                let (tx1, ty1) = map(x1, y1);
                let (tx2, ty2) = map(x2, y2);
                let (tx, ty) = map(x, y);
                out.push(Segment::Cubic(tx1, ty1, tx2, ty2, tx, ty));
                cx = x;
                cy = y;
            }
            Segment::SmoothCubic(x2, y2, x, y) => {
                let (tx2, ty2) = map(x2, y2);
                let (tx, ty) = map(x, y);
                out.push(Segment::SmoothCubic(tx2, ty2, tx, ty));
                cx = x;
                cy = y;
            }
            Segment::Quad(x1, y1, x, y) => {
                let (tx1, ty1) = map(x1, y1);
                let (tx, ty) = map(x, y);
                out.push(Segment::Quad(tx1, ty1, tx, ty));
                cx = x;
                cy = y;
            }
            Segment::SmoothQuad(x, y) => {
                let (tx, ty) = map(x, y);
                out.push(Segment::SmoothQuad(tx, ty));
                cx = x;
                cy = y;
            }
            Segment::Arc(rx, ry, rot, large, sweep, x, y) => {
                if similarity {
                    let scale = det.abs().sqrt();
                    let turn = b.atan2(a).to_degrees();
                    let (tx, ty) = map(x, y);
                    let sweep = if det < 0.0 { 1.0 - sweep } else { sweep };
                    out.push(Segment::Arc(
                        rx * scale,
                        ry * scale,
                        rot + turn,
                        large,
                        sweep,
                        tx,
                        ty,
                    ));
                } else {
                    for [x1, y1, x2, y2, px, py] in
                        arc_to_cubics(cx, cy, rx, ry, rot, large != 0.0, sweep != 0.0, x, y)
                    {
                        let (tx1, ty1) = map(x1, y1);
                        let (tx2, ty2) = map(x2, y2);
                        let (tx, ty) = map(px, py);
                        out.push(Segment::Cubic(tx1, ty1, tx2, ty2, tx, ty));
                    }
                }
                cx = x;
                cy = y;
            }
            Segment::Close => {
                out.push(Segment::Close);
                cx = sx;
                cy = sy;
            }
        }
    }
    Some(serialize_local_segments(&out, 0.0, 0.0))
}

/// Trace one SVG elliptical arc as cubic Béziers, each spanning at most
/// a quarter turn — the endpoint-to-center conversion of the SVG
/// implementation notes (F.6.5), then the standard quarter-arc cubic.
/// Returns `[c1x, c1y, c2x, c2y, x, y]` per piece; a degenerate arc
/// (zero radius or coincident endpoints) becomes a single line-like
/// cubic to the endpoint, which is what the spec draws for it.
#[allow(clippy::too_many_arguments)]
fn arc_to_cubics(
    x0: f64,
    y0: f64,
    rx: f64,
    ry: f64,
    rot_deg: f64,
    large: bool,
    sweep: bool,
    x: f64,
    y: f64,
) -> Vec<[f64; 6]> {
    let straight = |x: f64, y: f64| vec![[x0, y0, x, y, x, y]];
    if (x0 - x).abs() < 1e-12 && (y0 - y).abs() < 1e-12 {
        return Vec::new();
    }
    let mut rx = rx.abs();
    let mut ry = ry.abs();
    if rx < 1e-12 || ry < 1e-12 {
        return straight(x, y);
    }
    let (sin_p, cos_p) = rot_deg.to_radians().sin_cos();
    // Step 1: to the ellipse's own frame, midpoint at the origin.
    let dx = (x0 - x) / 2.0;
    let dy = (y0 - y) / 2.0;
    let x1p = cos_p * dx + sin_p * dy;
    let y1p = -sin_p * dx + cos_p * dy;
    // Radii too small for the chord are scaled up (F.6.6).
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    // Step 2: the center in that frame.
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    let num = (rx2 * ry2 - rx2 * y1p * y1p - ry2 * x1p * x1p).max(0.0);
    let den = rx2 * y1p * y1p + ry2 * x1p * x1p;
    let coef = if den.abs() < 1e-12 {
        0.0
    } else {
        (num / den).sqrt()
    } * if large == sweep { -1.0 } else { 1.0 };
    let cxp = coef * (rx * y1p / ry);
    let cyp = coef * (-(ry * x1p) / rx);
    // Step 3: back to user space.
    let cx = cos_p * cxp - sin_p * cyp + (x0 + x) / 2.0;
    let cy = sin_p * cxp + cos_p * cyp + (y0 + y) / 2.0;
    // Step 4: start angle and sweep.
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| -> f64 {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let mut ang = (dot / len).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            ang = -ang;
        }
        ang
    };
    let theta1 = angle(1.0, 0.0, (x1p - cxp) / rx, (y1p - cyp) / ry);
    let mut dtheta = angle(
        (x1p - cxp) / rx,
        (y1p - cyp) / ry,
        (-x1p - cxp) / rx,
        (-y1p - cyp) / ry,
    );
    if !sweep && dtheta > 0.0 {
        dtheta -= std::f64::consts::TAU;
    } else if sweep && dtheta < 0.0 {
        dtheta += std::f64::consts::TAU;
    }
    // Step 5: split into quarter turns and emit each as a cubic.
    let pieces = ((dtheta.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = dtheta / pieces as f64;
    let point = |t: f64| -> (f64, f64) {
        let (st, ct) = t.sin_cos();
        (
            cx + cos_p * rx * ct - sin_p * ry * st,
            cy + sin_p * rx * ct + cos_p * ry * st,
        )
    };
    let tangent = |t: f64| -> (f64, f64) {
        let (st, ct) = t.sin_cos();
        (
            -cos_p * rx * st - sin_p * ry * ct,
            -sin_p * rx * st + cos_p * ry * ct,
        )
    };
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let mut out = Vec::with_capacity(pieces);
    let mut t0 = theta1;
    for i in 0..pieces {
        let t1 = t0 + step;
        let (p0x, p0y) = point(t0);
        let (p3x, p3y) = if i + 1 == pieces { (x, y) } else { point(t1) };
        let (d0x, d0y) = tangent(t0);
        let (d1x, d1y) = tangent(t1);
        out.push([
            p0x + k * d0x,
            p0y + k * d0y,
            p3x - k * d1x,
            p3y - k * d1y,
            p3x,
            p3y,
        ]);
        t0 = t1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{localize_svg_path, svg_path_bounds, transform_svg_path};

    #[test]
    fn preserves_arc_and_compound_moves() {
        let (d, bounds) = localize_svg_path("M10 50 A40 40 0 0 1 90 50 Z M5 5 H15").expect("path");
        assert!(d.contains('A'));
        assert_eq!(d.matches('M').count(), 2);
        assert_eq!(bounds.x, 5.0);
        assert_eq!(bounds.y, 5.0);
    }

    #[test]
    fn a_translation_moves_every_point_and_absolutises_relative_commands() {
        let d = transform_svg_path("m10 10 l5 0 h5 v5 z", [1.0, 0.0, 0.0, 1.0, 100.0, 200.0])
            .expect("path");
        assert_eq!(d, "M 110 210 L 115 210 L 120 210 L 120 215 Z");
    }

    #[test]
    fn a_rotation_turns_h_and_v_into_lines_and_keeps_an_arc_an_arc() {
        // rotate(90): (x, y) -> (-y, x)
        let d = transform_svg_path("M0 0 H10 A5 5 0 0 1 20 0", [0.0, 1.0, -1.0, 0.0, 0.0, 0.0])
            .expect("path");
        assert_eq!(d, "M 0 0 L 0 10 A 5 5 90 0 1 0 20");
    }

    #[test]
    fn a_mirror_flips_the_arc_sweep() {
        let d = transform_svg_path("M0 0 A5 5 0 0 1 10 0", [-1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
            .expect("path");
        assert_eq!(d, "M 0 0 A 5 5 180 0 0 -10 0");
    }

    #[test]
    fn a_non_uniform_scale_traces_the_arc_as_cubics_through_the_same_points() {
        // A half circle of radius 10 from (0,0) to (20,0), squashed to half height.
        let d = transform_svg_path("M0 0 A10 10 0 0 1 20 0", [1.0, 0.0, 0.0, 0.5, 0.0, 0.0])
            .expect("path");
        assert!(!d.contains('A'), "{d}");
        assert_eq!(d.matches('C').count(), 2, "{d}");
        assert!(d.ends_with(" 20 0"), "{d}");
        let b = svg_path_bounds(&d).expect("bounds");
        assert!(
            (b.x - 0.0).abs() < 1e-3 && (b.w - 20.0).abs() < 1e-3,
            "{b:?}"
        );
        // the arc bulges upward (negative y) to half the radius
        assert!(
            (b.y + 5.0).abs() < 0.05 && (b.h - 5.0).abs() < 0.05,
            "{b:?}"
        );
    }
}
