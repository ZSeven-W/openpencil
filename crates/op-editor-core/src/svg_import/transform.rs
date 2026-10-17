//! The SVG `transform` attribute: parsing the transform list into one
//! affine matrix, composing it down the element tree, and telling the
//! two cases apart that the node builder handles differently — a
//! translate + uniform scale keeps a shape a shape (the attrs are
//! scaled in place, as the root viewBox scale always was), anything
//! with rotation, skew or non-uniform scale turns the shape into a
//! path whose points carry the full matrix.
//!
//! Matrix layout follows the SVG spec's `matrix(a b c d e f)`:
//!
//! ```text
//!   | a c e |   | x |
//!   | b d f | * | y |
//!   | 0 0 1 |   | 1 |
//! ```
//!
//! Hand-rolled like the rest of the importer (no glam here) so the
//! math stays in one `f64` representation with the path data.

/// One affine transform in SVG `matrix(a b c d e f)` layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Mat {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Mat {
    pub const IDENTITY: Mat = Mat {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn translate(tx: f64, ty: f64) -> Mat {
        Mat {
            e: tx,
            f: ty,
            ..Mat::IDENTITY
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Mat {
        Mat {
            a: sx,
            d: sy,
            ..Mat::IDENTITY
        }
    }

    /// Rotation by `deg` degrees, clockwise on screen (SVG's y-down
    /// convention), about the origin.
    pub fn rotate(deg: f64) -> Mat {
        let (s, c) = deg.to_radians().sin_cos();
        Mat {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn skew_x(deg: f64) -> Mat {
        Mat {
            c: deg.to_radians().tan(),
            ..Mat::IDENTITY
        }
    }

    pub fn skew_y(deg: f64) -> Mat {
        Mat {
            b: deg.to_radians().tan(),
            ..Mat::IDENTITY
        }
    }

    /// `self` applied after `rhs` — the SVG composition order, where
    /// `transform="A B"` means "apply B first, then A", and a parent's
    /// transform wraps its children's.
    pub fn then(self, rhs: Mat) -> Mat {
        Mat {
            a: self.a * rhs.a + self.c * rhs.b,
            b: self.b * rhs.a + self.d * rhs.b,
            c: self.a * rhs.c + self.c * rhs.d,
            d: self.b * rhs.c + self.d * rhs.d,
            e: self.a * rhs.e + self.c * rhs.f + self.e,
            f: self.b * rhs.e + self.d * rhs.f + self.f,
        }
    }

    #[cfg(test)]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// `Some((scale, tx, ty))` when the matrix is a translation plus
    /// one uniform positive scale — the only case in which a `<rect>`
    /// is still an axis-aligned rect after the transform, so the
    /// builder can scale its attrs instead of tracing it as a path.
    pub fn as_translate_uniform_scale(&self) -> Option<(f64, f64, f64)> {
        const EPS: f64 = 1e-9;
        if self.b.abs() > EPS || self.c.abs() > EPS {
            return None;
        }
        if (self.a - self.d).abs() > EPS || self.a <= 0.0 {
            return None;
        }
        Some((self.a, self.e, self.f))
    }

    /// The factor a stroke width scales by: the geometric mean of the
    /// axis scales, which is exact for a uniform scale and the usual
    /// compromise for anything else (SVG's own `non-scaling-stroke`
    /// aside, a stroke follows the shape's transform).
    pub fn stroke_scale(&self) -> f64 {
        (self.a * self.d - self.b * self.c).abs().sqrt().max(0.001)
    }
}

/// Parse a `transform` attribute value — a whitespace / comma
/// separated list of `translate(…)`, `scale(…)`, `rotate(…)`,
/// `skewX(…)`, `skewY(…)` and `matrix(…)` — into one matrix. An
/// unreadable function is skipped rather than failing the import, so
/// a shape still lands (untransformed) instead of vanishing.
pub(super) fn parse_transform(value: &str) -> Mat {
    let mut out = Mat::IDENTITY;
    let mut rest = value.trim();
    while !rest.is_empty() {
        let Some(open) = rest.find('(') else { break };
        let name = rest[..open].trim().trim_start_matches(',').trim();
        let Some(close_rel) = rest[open..].find(')') else {
            break;
        };
        let args: Vec<f64> = rest[open + 1..open + close_rel]
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f64>().ok())
            .collect();
        rest = rest[open + close_rel + 1..].trim_start();
        let m = match (name, args.as_slice()) {
            ("translate", [tx]) => Mat::translate(*tx, 0.0),
            ("translate", [tx, ty, ..]) => Mat::translate(*tx, *ty),
            ("scale", [s]) => Mat::scale(*s, *s),
            ("scale", [sx, sy, ..]) => Mat::scale(*sx, *sy),
            ("rotate", [deg]) => Mat::rotate(*deg),
            // rotate(a cx cy) = translate(cx cy) rotate(a) translate(-cx -cy)
            ("rotate", [deg, cx, cy, ..]) => Mat::translate(*cx, *cy)
                .then(Mat::rotate(*deg))
                .then(Mat::translate(-*cx, -*cy)),
            ("skewX", [deg]) => Mat::skew_x(*deg),
            ("skewY", [deg]) => Mat::skew_y(*deg),
            ("matrix", [a, b, c, d, e, f, ..]) => Mat {
                a: *a,
                b: *b,
                c: *c,
                d: *d,
                e: *e,
                f: *f,
            },
            _ => continue,
        };
        out = out.then(m);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(p: (f64, f64), q: (f64, f64)) -> bool {
        (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6
    }

    #[test]
    fn translate_scale_and_matrix_read_back() {
        assert!(close(
            parse_transform("translate(10, 5)").apply(1.0, 1.0),
            (11.0, 6.0)
        ));
        assert!(close(
            parse_transform("scale(2)").apply(3.0, 4.0),
            (6.0, 8.0)
        ));
        assert!(close(
            parse_transform("scale(2 3)").apply(3.0, 4.0),
            (6.0, 12.0)
        ));
        assert!(close(
            parse_transform("matrix(1 0 0 1 7 8)").apply(0.0, 0.0),
            (7.0, 8.0)
        ));
    }

    #[test]
    fn a_list_applies_right_to_left_like_the_spec() {
        // translate then scale: the point is scaled first, then moved.
        let m = parse_transform("translate(10 0) scale(2)");
        assert!(close(m.apply(1.0, 0.0), (12.0, 0.0)));
        let m = parse_transform("scale(2) translate(10 0)");
        assert!(close(m.apply(1.0, 0.0), (22.0, 0.0)));
    }

    #[test]
    fn rotate_is_clockwise_on_a_y_down_canvas_and_honours_a_pivot() {
        assert!(close(
            parse_transform("rotate(90)").apply(1.0, 0.0),
            (0.0, 1.0)
        ));
        assert!(close(
            parse_transform("rotate(90 10 10)").apply(11.0, 10.0),
            (10.0, 11.0)
        ));
    }

    #[test]
    fn only_translate_plus_uniform_scale_keeps_a_shape_a_shape() {
        assert_eq!(
            parse_transform("translate(3 4) scale(2)").as_translate_uniform_scale(),
            Some((2.0, 3.0, 4.0))
        );
        assert!(parse_transform("scale(2 1)")
            .as_translate_uniform_scale()
            .is_none());
        assert!(parse_transform("rotate(10)")
            .as_translate_uniform_scale()
            .is_none());
        assert!(parse_transform("skewX(10)")
            .as_translate_uniform_scale()
            .is_none());
        assert!(parse_transform("scale(-1)")
            .as_translate_uniform_scale()
            .is_none());
    }

    #[test]
    fn an_unreadable_function_is_skipped_not_fatal() {
        let m = parse_transform("frobnicate(1) translate(5 5)");
        assert!(close(m.apply(0.0, 0.0), (5.0, 5.0)));
        assert_eq!(parse_transform("translate("), Mat::IDENTITY);
    }
}
