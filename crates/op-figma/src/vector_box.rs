//! Fit an imported vector's box to the path it draws.
//!
//! `.op` paints a path by fitting the path's own tight bounds to the node's
//! box. Figma sizes a vector to its network, so the two normally coincide,
//! but tools that write `.fig` files (open-pencil's `insert_icon`, for one)
//! give every glyph path the full icon box: a lucide chevron's 6×12 stroke
//! sits inside a 24×24 vector. Fitting that path to the box stretched the
//! chevron to twice its size, turned the `···` dots into box-sized circles
//! and thickened every glyph (issue #277).
//!
//! For a positioned vector this shrinks the box to the path's tight bounds
//! and moves the path to the box origin, so the canvas fit is the identity
//! and the glyph paints exactly where the file drew it. Flow children of an
//! auto-layout parent are left alone — their box is a layout input.

use jian_ops_schema::node::PenNodeBase;
use jian_ops_schema::sizing::SizingBehavior;

use crate::common::round2;
use crate::vector_decoder::compute_svg_path_bounds;

/// Below this extent an axis is degenerate (a straight line).
const MIN_EXTENT: f64 = 0.01;
/// Slack before a path counts as inset in, or overflowing, its box.
const SLACK: f64 = 0.5;

/// Adjust `base` / `width` / `height` / `d` in place for one decoded path.
/// `size` is the vector's authored Figma size.
pub(crate) fn fit_box_to_path(
    base: &mut PenNodeBase,
    width: &mut SizingBehavior,
    height: &mut SizingBehavior,
    d: &mut String,
    size: (f64, f64),
) {
    // Zero-size vectors: derive extent from the path bounds.
    let (size_x, size_y) = size;
    if (size_x < MIN_EXTENT || size_y < MIN_EXTENT)
        && matches!(width, SizingBehavior::Number(_))
        && matches!(height, SizingBehavior::Number(_))
    {
        if let Some(bounds) = compute_svg_path_bounds(d) {
            let path_w = bounds.max_x - bounds.min_x;
            let path_h = bounds.max_y - bounds.min_y;
            if size_x < MIN_EXTENT && path_w > MIN_EXTENT {
                *width = SizingBehavior::Number(round2(path_w));
                base.x = Some(round2(base.x.unwrap_or(0.0) + bounds.min_x));
            }
            if size_y < MIN_EXTENT && path_h > MIN_EXTENT {
                *height = SizingBehavior::Number(round2(path_h));
                base.y = Some(round2(base.y.unwrap_or(0.0) + bounds.min_y));
            }
        }
        return;
    }
    trim_inset_box(base, width, height, d);
}

fn trim_inset_box(
    base: &mut PenNodeBase,
    width: &mut SizingBehavior,
    height: &mut SizingBehavior,
    d: &mut String,
) {
    let (Some(x), Some(y)) = (base.x, base.y) else {
        return; // a flow child: its box belongs to the layout
    };
    let (&SizingBehavior::Number(w), &SizingBehavior::Number(h)) = (&*width, &*height) else {
        return;
    };
    let Some(path) = AbsPath::parse(d) else {
        return;
    };
    let Some([min_x, min_y, max_x, max_y]) = path.tight_bounds() else {
        return;
    };
    let inside = min_x >= -SLACK && min_y >= -SLACK && max_x <= w + SLACK && max_y <= h + SLACK;
    let inset = (max_x - min_x) < w - SLACK || (max_y - min_y) < h - SLACK;
    if !inside || !inset {
        return;
    }
    let trim_x = max_x - min_x > MIN_EXTENT;
    let trim_y = max_y - min_y > MIN_EXTENT;
    // A degenerate axis keeps its box, but still moves to the line: the
    // painter anchors a zero-extent axis at the box edge.
    let dx = min_x;
    let dy = min_y;
    if trim_x {
        *width = SizingBehavior::Number(round2(max_x - min_x));
    }
    if trim_y {
        *height = SizingBehavior::Number(round2(max_y - min_y));
    }
    base.x = Some(round2(x + dx));
    base.y = Some(round2(y + dy));
    *d = path.translated(-dx, -dy);
}

/// An absolute `M` / `L` / `Q` / `C` / `Z` path — the only commands the
/// vector decoder emits. Anything else is not parsed and not trimmed.
struct AbsPath {
    commands: Vec<(char, Vec<f64>)>,
}

impl AbsPath {
    fn parse(d: &str) -> Option<Self> {
        let mut commands: Vec<(char, Vec<f64>)> = Vec::new();
        let mut number = String::new();
        let flush = |number: &mut String, commands: &mut Vec<(char, Vec<f64>)>| -> Option<()> {
            if !number.is_empty() {
                commands.last_mut()?.1.push(number.parse().ok()?);
                number.clear();
            }
            Some(())
        };
        for c in d.chars() {
            match c {
                'M' | 'L' | 'Q' | 'C' | 'Z' => {
                    flush(&mut number, &mut commands)?;
                    commands.push((c, Vec::new()));
                }
                '-' if !number.is_empty() && !number.ends_with(['e', 'E']) => {
                    flush(&mut number, &mut commands)?;
                    number.push(c);
                }
                c if c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E') => {
                    number.push(c)
                }
                c if c.is_ascii_whitespace() || c == ',' => flush(&mut number, &mut commands)?,
                _ => return None,
            }
        }
        flush(&mut number, &mut commands)?;
        let arity_ok = commands.iter().all(|(cmd, args)| match cmd {
            'Z' => args.is_empty(),
            'M' | 'L' => !args.is_empty() && args.len() % 2 == 0,
            'Q' => !args.is_empty() && args.len() % 4 == 0,
            _ => !args.is_empty() && args.len() % 6 == 0,
        });
        (arity_ok && !commands.is_empty()).then_some(Self { commands })
    }

    /// Bounds over on-curve points and curve extrema (no control hull),
    /// matching skia's `compute_tight_bounds`, which the painter fits.
    fn tight_bounds(&self) -> Option<[f64; 4]> {
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        let mut add = |x: f64, y: f64| {
            b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
        };
        let mut cur = (0.0, 0.0);
        let mut start = (0.0, 0.0);
        for (cmd, args) in &self.commands {
            match cmd {
                'M' | 'L' => {
                    for (i, p) in args.chunks(2).enumerate() {
                        cur = (p[0], p[1]);
                        if *cmd == 'M' && i == 0 {
                            start = cur;
                        }
                        add(cur.0, cur.1);
                    }
                }
                'Q' => {
                    for p in args.chunks(4) {
                        let (c, e) = ((p[0], p[1]), (p[2], p[3]));
                        for t in quad_extrema(cur, c, e) {
                            let (x, y) = quad_at(cur, c, e, t);
                            add(x, y);
                        }
                        cur = e;
                        add(cur.0, cur.1);
                    }
                }
                'C' => {
                    for p in args.chunks(6) {
                        let (c1, c2, e) = ((p[0], p[1]), (p[2], p[3]), (p[4], p[5]));
                        for t in cubic_extrema(cur, c1, c2, e) {
                            let (x, y) = cubic_at(cur, c1, c2, e, t);
                            add(x, y);
                        }
                        cur = e;
                        add(cur.0, cur.1);
                    }
                }
                _ => cur = start,
            }
        }
        b.iter().all(|v| v.is_finite()).then_some(b)
    }

    fn translated(&self, dx: f64, dy: f64) -> String {
        let fmt = |v: f64| {
            let v = (v * 10_000.0).round() / 10_000.0;
            if v == 0.0 {
                "0".to_string()
            } else {
                format!("{v}")
            }
        };
        self.commands
            .iter()
            .map(|(cmd, args)| {
                let coords: Vec<String> = args
                    .iter()
                    .enumerate()
                    .map(|(i, v)| fmt(if i % 2 == 0 { v + dx } else { v + dy }))
                    .collect();
                if coords.is_empty() {
                    cmd.to_string()
                } else {
                    format!("{cmd}{}", coords.join(" "))
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

type Pt = (f64, f64);

fn quad_at(p0: Pt, p1: Pt, p2: Pt, t: f64) -> Pt {
    let u = 1.0 - t;
    (
        u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
        u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
    )
}

fn quad_extrema(p0: Pt, p1: Pt, p2: Pt) -> Vec<f64> {
    let root = |a: f64, b: f64, c: f64| {
        let den = a - 2.0 * b + c;
        (den.abs() > 1e-12).then(|| (a - b) / den)
    };
    [root(p0.0, p1.0, p2.0), root(p0.1, p1.1, p2.1)]
        .into_iter()
        .flatten()
        .filter(|t| *t > 0.0 && *t < 1.0)
        .collect()
}

fn cubic_at(p0: Pt, p1: Pt, p2: Pt, p3: Pt, t: f64) -> Pt {
    let u = 1.0 - t;
    let f = |a: f64, b: f64, c: f64, d: f64| {
        u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
    };
    (f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
}

fn cubic_extrema(p0: Pt, p1: Pt, p2: Pt, p3: Pt) -> Vec<f64> {
    let mut ts = Vec::new();
    for (a, b, c, d) in [(p0.0, p1.0, p2.0, p3.0), (p0.1, p1.1, p2.1, p3.1)] {
        // Derivative: 3[(−a+3b−3c+d)t² + 2(a−2b+c)t + (b−a)].
        let qa = -a + 3.0 * b - 3.0 * c + d;
        let qb = 2.0 * (a - 2.0 * b + c);
        let qc = b - a;
        if qa.abs() < 1e-12 {
            if qb.abs() > 1e-12 {
                ts.push(-qc / qb);
            }
            continue;
        }
        let disc = qb * qb - 4.0 * qa * qc;
        if disc >= 0.0 {
            let s = disc.sqrt();
            ts.push((-qb + s) / (2.0 * qa));
            ts.push((-qb - s) / (2.0 * qa));
        }
    }
    ts.retain(|t| *t > 0.0 && *t < 1.0);
    ts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positioned(x: f64, y: f64) -> PenNodeBase {
        PenNodeBase {
            x: Some(x),
            y: Some(y),
            ..PenNodeBase::default()
        }
    }

    #[test]
    fn inset_glyph_path_gets_a_tight_box_at_its_drawn_position() {
        // lucide chevron-left "m15 18-6-6 6-6" in a 24×24 vector at (4, 4).
        let mut base = positioned(4.0, 4.0);
        let (mut w, mut h) = (SizingBehavior::Number(24.0), SizingBehavior::Number(24.0));
        let mut d = "M15 18 L9 12 L15 6".to_string();
        fit_box_to_path(&mut base, &mut w, &mut h, &mut d, (24.0, 24.0));
        assert_eq!((base.x, base.y), (Some(13.0), Some(10.0)));
        assert_eq!(
            (w, h),
            (SizingBehavior::Number(6.0), SizingBehavior::Number(12.0))
        );
        assert_eq!(d, "M6 12 L0 6 L6 0");
    }

    #[test]
    fn a_small_dot_keeps_its_size_instead_of_filling_the_box() {
        // One of more-horizontal's r=1 dots, as two cubic halves at x=19.
        let mut base = positioned(0.0, 0.0);
        let (mut w, mut h) = (SizingBehavior::Number(24.0), SizingBehavior::Number(24.0));
        let k = 0.5523;
        let mut d =
            format!(
            "M20 12 C20 {} {} 13 19 13 C{} 13 18 {} 18 12 C18 {} {} 11 19 11 C{} 11 20 {} 20 12 Z",
            12.0 + k, 19.0 + k, 19.0 - k, 12.0 + k, 12.0 - k, 19.0 - k, 19.0 + k, 12.0 - k
        );
        fit_box_to_path(&mut base, &mut w, &mut h, &mut d, (24.0, 24.0));
        assert_eq!((base.x, base.y), (Some(18.0), Some(11.0)));
        assert_eq!(
            (w, h),
            (SizingBehavior::Number(2.0), SizingBehavior::Number(2.0))
        );
    }

    #[test]
    fn flow_children_and_full_box_paths_are_left_alone() {
        let mut flow = PenNodeBase::default();
        let (mut w, mut h) = (SizingBehavior::Number(24.0), SizingBehavior::Number(24.0));
        let mut d = "M15 18 L9 12 L15 6".to_string();
        fit_box_to_path(&mut flow, &mut w, &mut h, &mut d, (24.0, 24.0));
        assert_eq!(d, "M15 18 L9 12 L15 6");

        let mut base = positioned(0.0, 0.0);
        let mut full = "M0 0 L24 24".to_string();
        fit_box_to_path(&mut base, &mut w, &mut h, &mut full, (24.0, 24.0));
        assert_eq!(full, "M0 0 L24 24");
        assert_eq!(base.x, Some(0.0));
    }
}
