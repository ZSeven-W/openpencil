use std::fmt;

use crate::{
    path, prelude::*, scalar, Matrix, Path, PathDirection, PathFillType, PathVerb, Point, RRect,
    Rect, Vector,
};
use skia_bindings::{self as sb, SkPathBuilder};

pub type ArcSize = sb::SkPathBuilder_ArcSize;
variant_name!(ArcSize::Large);

// PathBuilder can't be a Handle<>, because SkPathBuilder contains several STArrays with interior
// pointers.
//
// See <https://github.com/rust-skia/rust-skia/pull/1195>.
pub type PathBuilder = RefHandle<SkPathBuilder>;
unsafe_send_sync!(PathBuilder);

impl NativeDrop for SkPathBuilder {
    fn drop(&mut self) {
        unsafe { sb::C_SkPathBuilder_delete(self) }
    }
}

impl NativePartialEq for SkPathBuilder {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkPathBuilder_equals(self, rhs) }
    }
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for PathBuilder {
    fn clone(&self) -> Self {
        Self::from_ptr(unsafe { sb::C_SkPathBuilder_clone(self.native()) }).unwrap()
    }
}

impl fmt::Debug for PathBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathBuilder")
            .field("fill_type", &self.fill_type())
            .finish()
    }
}

impl From<PathBuilder> for Path {
    fn from(mut value: PathBuilder) -> Self {
        value.detach()
    }
}

impl PathBuilder {
    /// Constructs an empty [`PathBuilder`]. By default, [`PathBuilder`] has no verbs, no [`Point`], and
    /// no weights. [`PathFillType`] is set to [`PathFillType::Winding`].
    ///
    /// # Returns
    /// empty [`PathBuilder`]
    pub fn new() -> Self {
        Self::from_ptr(unsafe { sb::C_SkPathBuilder_new() }).unwrap()
    }

    /// Constructs an empty [`PathBuilder`] with the given [`PathFillType`]. By default, [`PathBuilder`] has no
    /// verbs, no [`Point`], and no weights.
    ///
    /// - `fill_type`: [`PathFillType`] to set on the [`PathBuilder`].
    ///
    /// # Returns
    /// empty [`PathBuilder`]
    pub fn new_with_fill_type(fill_type: PathFillType) -> Self {
        Self::from_ptr(unsafe { sb::C_SkPathBuilder_newWithFillType(fill_type) }).unwrap()
    }

    /// Constructs a [`PathBuilder`] that is a copy of an existing [`Path`].
    /// Copies the [`PathFillType`] and replays all of the verbs from the [`Path`] into the [`PathBuilder`].
    ///
    /// - `path`: [`Path`] to copy
    ///
    /// # Returns
    /// [`PathBuilder`]
    pub fn new_path(path: &Path) -> Self {
        Self::from_ptr(unsafe { sb::C_SkPathBuilder_newFromPath(path.native()) }).unwrap()
    }

    /// Returns [`PathFillType`], the rule used to fill [`Path`].
    ///
    /// # Returns
    /// current [`PathFillType`] setting
    pub fn fill_type(&self) -> PathFillType {
        self.native().fFillType
    }

    /// Returns minimum and maximum axes values of [`Point`] array.
    /// Returns `None` if [`PathBuilder`] contains no points.
    ///
    /// [`Rect`] returned includes all [`Point`] added to [`PathBuilder`], including [`Point`] associated
    /// with [`PathVerb::Move`] that define empty contours.
    ///
    /// If any of the points are non-finite, returns `None`.
    ///
    /// # Returns
    /// Bounds of all [`Point`] in [`Point`] array, or `None`.
    pub fn compute_finite_bounds(&self) -> Option<Rect> {
        let mut rect = Rect::default();
        unsafe { sb::C_SkPathBuilder_computeFiniteBounds(self.native(), rect.native_mut()) }
            .then_some(rect)
    }

    /// Like [`Self::compute_finite_bounds()`] but returns a 'tight' bounds, meaning when there are curve
    /// segments, this computes the X/Y limits of the curve itself, not the curve's control
    /// point(s). For a polygon, this returns the same as [`Self::compute_finite_bounds()`].
    pub fn compute_tight_bounds(&self) -> Option<Rect> {
        let mut rect = Rect::default();
        unsafe { sb::C_SkPathBuilder_computeTightBounds(self.native(), rect.native_mut()) }
            .then_some(rect)
    }

    /// Returns minimum and maximum axes values of [`Point`] array.
    ///
    /// # Returns
    /// Bounds of all [`Point`] in [`Point`] array, or an empty [`Rect`] if the bounds are non-finite.
    ///
    /// # Deprecated
    /// Use [`Self::compute_finite_bounds()`] instead, which returns `None` when the bounds are non-finite.
    #[deprecated(since = "0.91.0", note = "Use compute_finite_bounds() instead")]
    pub fn compute_bounds(&self) -> Rect {
        self.compute_finite_bounds().unwrap_or_else(Rect::new_empty)
    }

    /// Returns a [`Path`] representing the current state of the [`PathBuilder`]. The builder is
    /// unchanged after returning the path.
    ///
    /// # Returns
    /// [`Path`] representing the current state of the builder.
    pub fn snapshot(&self) -> Path {
        self.snapshot_and_transform(None)
    }

    /// Returns a [`Path`] representing the current state of the [`PathBuilder`]. The builder is
    /// unchanged after returning the path.
    ///
    /// - `mx`: if present, applied to the points after they are copied into the resulting path.
    ///
    /// # Returns
    /// [`Path`] representing the current state of the builder.
    pub fn snapshot_and_transform<'m>(&self, mx: impl Into<Option<&'m Matrix>>) -> Path {
        let mut p = Path::default();
        unsafe {
            sb::C_SkPathBuilder_snapshot(
                self.native(),
                mx.into().native_ptr_or_null(),
                p.native_mut(),
            )
        };
        p
    }

    /// Returns a [`Path`] representing the current state of the [`PathBuilder`]. The builder is
    /// reset to empty after returning the path.
    ///
    /// # Returns
    /// [`Path`] representing the current state of the builder.
    pub fn detach(&mut self) -> Path {
        self.detach_and_transform(None)
    }

    /// Returns a [`Path`] representing the current state of the [`PathBuilder`]. The builder is
    /// reset to empty after returning the path.
    ///
    /// - `mx`: if present, applied to the points after they are copied into the resulting path.
    ///
    /// # Returns
    /// [`Path`] representing the current state of the builder.
    pub fn detach_and_transform<'m>(&mut self, mx: impl Into<Option<&'m Matrix>>) -> Path {
        let mut p = Path::default();
        unsafe {
            sb::C_SkPathBuilder_detach(
                self.native_mut(),
                mx.into().native_ptr_or_null(),
                p.native_mut(),
            )
        };
        p
    }

    /// Sets [`PathFillType`], the rule used to fill [`Path`]. While there is no
    /// check that `ft` is legal, values outside of [`PathFillType`] are not supported.
    ///
    /// - `ft`: [`PathFillType`] to be used by [`Path`]s generated from this builder.
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn set_fill_type(&mut self, ft: PathFillType) -> &mut Self {
        self.native_mut().fFillType = ft;
        self
    }

    /// Specifies whether [`Path`] is volatile; whether it will be altered or discarded
    /// by the caller after it is drawn. [`Path`] by default have volatile set false, allowing
    /// Skia to attach a cache of data which speeds repeated drawing.
    ///
    /// Mark temporary paths, discarded or modified after use, as volatile
    /// to inform Skia that the path need not be cached.
    ///
    /// Mark animating [`Path`] volatile to improve performance.
    /// Mark unchanging [`Path`] non-volatile to improve repeated rendering.
    ///
    /// raster surface [`Path`] draws are affected by volatile for some shadows.
    /// GPU surface [`Path`] draws are affected by volatile for some shadows and concave geometries.
    ///
    /// - `is_volatile`: true if caller will alter [`Path`] after drawing
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn set_is_volatile(&mut self, is_volatile: bool) -> &mut Self {
        self.native_mut().fIsVolatile = is_volatile;
        self
    }

    /// Sets [`PathBuilder`] to its initial state.
    /// Removes verb array, [`Point`] array, and weights, and sets [`PathFillType`] to [`PathFillType::Winding`].
    /// Internal storage associated with [`PathBuilder`] is preserved.
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn reset(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().reset();
        }
        self
    }

    /// Specifies the beginning of contour. If the previous verb was a "move" verb,
    /// then this just replaces the point value of that move, otherwise it appends a new
    /// "move" verb to the builder using the point.
    ///
    /// Thus, each contour can only have 1 move verb in it (the last one specified).
    pub fn move_to(&mut self, pt: impl Into<Point>) -> &mut Self {
        unsafe {
            self.native_mut().moveTo(pt.into().into_native());
        }
        self
    }

    /// Adds line from last point to [`Point`] p. If [`PathBuilder`] is empty, or last [`PathVerb`] is
    /// [`PathVerb::Close`], last point is set to (0, 0) before adding line.
    ///
    /// `line_to()` first appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array, if needed.
    /// `line_to()` then appends [`PathVerb::Line`] to verb array and [`Point`] p to [`Point`] array.
    ///
    /// - `pt`: end [`Point`] of added line
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn line_to(&mut self, pt: impl Into<Point>) -> &mut Self {
        unsafe {
            self.native_mut().lineTo(pt.into().into_native());
        }
        self
    }

    /// Adds quad from last point towards [`Point`] p1, to [`Point`] p2.
    /// If [`PathBuilder`] is empty, or last [`PathVerb`] is [`PathVerb::Close`], last point is set to (0, 0)
    /// before adding quad.
    ///
    /// Appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array, if needed;
    /// then appends [`PathVerb::Quad`] to verb array; and [`Point`] p1, p2
    /// to [`Point`] array.
    ///
    /// - `p1`: control [`Point`] of added quad
    /// - `p2`: end [`Point`] of added quad
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn quad_to(&mut self, p1: impl Into<Point>, p2: impl Into<Point>) -> &mut Self {
        unsafe {
            self.native_mut()
                .quadTo(p1.into().into_native(), p2.into().into_native());
        }
        self
    }

    /// Adds conic from last point towards pt1, to pt2, weighted by w.
    /// If [`PathBuilder`] is empty, or last [`PathVerb`] is [`PathVerb::Close`], last point is set to (0, 0)
    /// before adding conic.
    ///
    /// Appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array, if needed.
    ///
    /// If w is finite and not one, appends [`PathVerb::Conic`] to verb array;
    /// and pt1, pt2 to [`Point`] array; and w to conic weights.
    ///
    /// If w is one, appends [`PathVerb::Quad`] to verb array, and
    /// pt1, pt2 to [`Point`] array.
    ///
    /// If w is not finite, appends [`PathVerb::Line`] twice to verb array, and
    /// pt1, pt2 to [`Point`] array.
    ///
    /// - `pt1`: control [`Point`] of conic
    /// - `pt2`: end [`Point`] of conic
    /// - `w`: weight of added conic
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn conic_to(&mut self, p1: impl Into<Point>, p2: impl Into<Point>, w: scalar) -> &mut Self {
        unsafe {
            self.native_mut()
                .conicTo(p1.into().into_native(), p2.into().into_native(), w);
        }
        self
    }

    /// Adds cubic from last point towards [`Point`] p1, then towards [`Point`] p2, ending at
    /// [`Point`] p3. If [`PathBuilder`] is empty, or last [`PathVerb`] is [`PathVerb::Close`], last point is
    /// set to (0, 0) before adding cubic.
    ///
    /// Appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array, if needed;
    /// then appends [`PathVerb::Cubic`] to verb array; and [`Point`] p1, p2, p3
    /// to [`Point`] array.
    ///
    /// - `p1`: first control [`Point`] of cubic
    /// - `p2`: second control [`Point`] of cubic
    /// - `p3`: end [`Point`] of cubic
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn cubic_to(
        &mut self,
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        p3: impl Into<Point>,
    ) -> &mut Self {
        unsafe {
            self.native_mut().cubicTo(
                p1.into().into_native(),
                p2.into().into_native(),
                p3.into().into_native(),
            )
        };
        self
    }

    /// Appends [`PathVerb::Close`] to [`PathBuilder`]. A closed contour connects the first and last [`Point`]
    /// with line, forming a continuous loop. Open and closed contour draw the same
    /// with [`crate::PaintStyle::Fill`]. With [`crate::PaintStyle::Stroke`], open contour draws
    /// [`crate::PaintCap`] at contour start and end; closed contour draws
    /// [`crate::PaintJoin`] at contour start and end.
    ///
    /// `close()` has no effect if [`PathBuilder`] is empty or last [`PathVerb`] is [`PathVerb::Close`].
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn close(&mut self) -> &mut Self {
        unsafe {
            self.native_mut().close();
        }
        self
    }

    /// Append a series of `line_to(...)`
    ///
    /// - `points`: array of [`Point`]
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn polyline_to(&mut self, points: &[Point]) -> &mut Self {
        unsafe {
            sb::C_SkPathBuilder_polylineTo(
                self.native_mut(),
                points.native().as_ptr(),
                points.len(),
            );
        }
        self
    }

    /// Adds beginning of contour relative to last point.
    /// If [`PathBuilder`] is empty, starts contour at (dx, dy).
    /// Otherwise, start contour at last point offset by (dx, dy).
    /// Function name stands for "relative move to".
    ///
    /// - `pt`: vector offset from last point to contour start
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn r_move_to(&mut self, pt: impl Into<Vector>) -> &mut Self {
        unsafe {
            self.native_mut().rMoveTo(pt.into().into_native());
        }
        self
    }

    /// Adds line from last point to vector given by pt. If [`PathBuilder`] is empty, or last
    /// [`PathVerb`] is [`PathVerb::Close`], last point is set to (0, 0) before adding line.
    ///
    /// Appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array, if needed;
    /// then appends [`PathVerb::Line`] to verb array and line end to [`Point`] array.
    /// Line end is last point plus vector given by pt.
    /// Function name stands for "relative line to".
    ///
    /// - `pt`: vector offset from last point to line end
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn r_line_to(&mut self, pt: impl Into<Vector>) -> &mut Self {
        unsafe {
            self.native_mut().rLineTo(pt.into().into_native());
        }
        self
    }

    /// Adds quad from last point towards vector pt1, to vector pt2.
    /// If [`PathBuilder`] is empty, or last [`PathVerb`]
    /// is [`PathVerb::Close`], last point is set to (0, 0) before adding quad.
    ///
    /// Appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array,
    /// if needed; then appends [`PathVerb::Quad`] to verb array; and appends quad
    /// control and quad end to [`Point`] array.
    /// Quad control is last point plus vector pt1.
    /// Quad end is last point plus vector pt2.
    /// Function name stands for "relative quad to".
    ///
    /// - `pt1`: offset vector from last point to quad control
    /// - `pt2`: offset vector from last point to quad end
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn r_quad_to(&mut self, pt1: impl Into<Vector>, pt2: impl Into<Vector>) -> &mut Self {
        unsafe {
            self.native_mut()
                .rQuadTo(pt1.into().into_native(), pt2.into().into_native());
        }
        self
    }

    /// Adds conic from last point towards vector p1, to vector p2,
    /// weighted by w. If [`PathBuilder`] is empty, or last [`PathVerb`]
    /// is [`PathVerb::Close`], last point is set to (0, 0) before adding conic.
    ///
    /// Appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array,
    /// if needed.
    ///
    /// If w is finite and not one, next appends [`PathVerb::Conic`] to verb array,
    /// and w is recorded as conic weight; otherwise, if w is one, appends
    /// [`PathVerb::Quad`] to verb array; or if w is not finite, appends [`PathVerb::Line`]
    /// twice to verb array.
    ///
    /// In all cases appends [`Point`] control and end to [`Point`] array.
    /// control is last point plus vector p1.
    /// end is last point plus vector p2.
    ///
    /// Function name stands for "relative conic to".
    ///
    /// - `pt1`: offset vector from last point to conic control
    /// - `pt2`: offset vector from last point to conic end
    /// - `w`: weight of added conic
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn r_conic_to(
        &mut self,
        pt1: impl Into<Vector>,
        pt2: impl Into<Vector>,
        w: scalar,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .rConicTo(pt1.into().into_native(), pt2.into().into_native(), w);
        }
        self
    }

    /// Adds cubic from last point towards vector pt1, then towards
    /// vector pt2, to vector pt3.
    /// If [`PathBuilder`] is empty, or last [`PathVerb`]
    /// is [`PathVerb::Close`], last point is set to (0, 0) before adding cubic.
    ///
    /// Appends [`PathVerb::Move`] to verb array and (0, 0) to [`Point`] array,
    /// if needed; then appends [`PathVerb::Cubic`] to verb array; and appends cubic
    /// control and cubic end to [`Point`] array.
    /// Cubic control is last point plus vector (dx1, dy1).
    /// Cubic end is last point plus vector (dx2, dy2).
    /// Function name stands for "relative cubic to".
    ///
    /// - `pt1`: offset vector from last point to first cubic control
    /// - `pt2`: offset vector from last point to second cubic control
    /// - `pt3`: offset vector from last point to cubic end
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn r_cubic_to(
        &mut self,
        pt1: impl Into<Vector>,
        pt2: impl Into<Vector>,
        pt3: impl Into<Vector>,
    ) -> &mut Self {
        unsafe {
            self.native_mut().rCubicTo(
                pt1.into().into_native(),
                pt2.into().into_native(),
                pt3.into().into_native(),
            );
        }
        self
    }

    /// Appends arc to [`PathBuilder`], relative to last [`Path`] [`Point`]. Arc is implemented by one or
    /// more conic, weighted to describe part of oval with radii (rx, ry) rotated by
    /// `x_axis_rotate` degrees. Arc curves from last [`PathBuilder`] [`Point`] to relative end [`Point`]:
    /// (dx, dy), choosing one of four possible routes: clockwise or
    /// counterclockwise, and smaller or larger. If [`PathBuilder`] is empty, the start arc [`Point`]
    /// is (0, 0).
    ///
    /// Arc sweep is always less than 360 degrees. `arc_to()` appends line to end [`Point`]
    /// if either radii are zero, or if last [`Path`] [`Point`] equals end [`Point`].
    /// `arc_to()` scales radii (rx, ry) to fit last [`Path`] [`Point`] and end [`Point`] if both are
    /// greater than zero but too small to describe an arc.
    ///
    /// `arc_to()` appends up to four conic curves.
    /// `arc_to()` implements the functionality of svg arc, although SVG "sweep-flag" value is
    /// opposite the integer value of sweep; SVG "sweep-flag" uses 1 for clockwise, while
    /// [`PathDirection::CW`] cast to int is zero.
    ///
    /// - `r`: radii on axes before x-axis rotation
    /// - `x_axis_rotate`: x-axis rotation in degrees; positive values are clockwise
    /// - `large_arc`: chooses smaller or larger arc
    /// - `sweep`: chooses clockwise or counterclockwise arc
    /// - `dxdy`: offset end of arc from last [`Path`] point
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn r_arc_to(
        &mut self,
        r: impl Into<Vector>,
        x_axis_rotate: scalar,
        large_arc: ArcSize,
        sweep: PathDirection,
        dxdy: impl Into<Vector>,
    ) -> &mut Self {
        let r = r.into();
        let d = dxdy.into();
        unsafe {
            self.native_mut().rArcTo(
                r.into_native(),
                x_axis_rotate,
                large_arc,
                sweep,
                d.into_native(),
            );
        }
        self
    }

    /// Appends arc to the builder. Arc added is part of ellipse
    /// bounded by oval, from `start_angle` through `sweep_angle`. Both `start_angle` and
    /// `sweep_angle` are measured in degrees, where zero degrees is aligned with the
    /// positive x-axis, and positive sweeps extends arc clockwise.
    ///
    /// `arc_to()` adds line connecting the builder's last point to initial arc point if `force_move_to`
    /// is false and the builder is not empty. Otherwise, added contour begins with first point
    /// of arc. Angles greater than -360 and less than 360 are treated modulo 360.
    ///
    /// - `oval`: bounds of ellipse containing arc
    /// - `start_angle_deg`: starting angle of arc in degrees
    /// - `sweep_angle_deg`: sweep, in degrees. Positive is clockwise; treated modulo 360
    /// - `force_move_to`: true to start a new contour with arc
    ///
    /// # Returns
    /// reference to the builder
    pub fn arc_to(
        &mut self,
        oval: impl AsRef<Rect>,
        start_angle_deg: scalar,
        sweep_angle_deg: scalar,
        force_move_to: bool,
    ) -> &mut Self {
        unsafe {
            self.native_mut().arcTo(
                oval.as_ref().native(),
                start_angle_deg,
                sweep_angle_deg,
                force_move_to,
            );
        }
        self
    }

    /// Appends arc to [`Path`], after appending line if needed. Arc is implemented by conic
    /// weighted to describe part of circle. Arc is contained by tangent from
    /// last [`Path`] point to p1, and tangent from p1 to p2. Arc
    /// is part of circle sized to radius, positioned so it touches both tangent lines.
    ///
    /// If last [`Path`] [`Point`] does not start arc, `arc_to()` appends connecting line to [`Path`].
    /// The length of vector from p1 to p2 does not affect arc.
    ///
    /// Arc sweep is always less than 180 degrees. If radius is zero, or if
    /// tangents are nearly parallel, `arc_to()` appends line from last [`Path`] [`Point`] to p1.
    ///
    /// `arc_to()` appends at most one line and one conic.
    /// `arc_to()` implements the functionality of PostScript arct and HTML Canvas `arcTo`.
    ///
    /// - `p1`: [`Point`] common to pair of tangents
    /// - `p2`: end of second tangent
    /// - `radius`: distance from arc to circle center
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn arc_to_tangent(
        &mut self,
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        radius: scalar,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .arcTo1(p1.into().into_native(), p2.into().into_native(), radius);
        }
        self
    }

    /// Appends arc to [`Path`]. Arc is implemented by one or more conic weighted to describe
    /// part of oval with radii (r.fX, r.fY) rotated by `x_axis_rotate` degrees. Arc curves
    /// from last [`Path`] [`Point`] to (xy.fX, xy.fY), choosing one of four possible routes:
    /// clockwise or counterclockwise,
    /// and smaller or larger.
    ///
    /// Arc sweep is always less than 360 degrees. `arc_to_radius()` appends line to xy if either
    /// radii are zero, or if last [`Path`] [`Point`] equals (xy.fX, xy.fY). `arc_to_radius()` scales radii r to
    /// fit last [`Path`] [`Point`] and xy if both are greater than zero but too small to describe
    /// an arc.
    ///
    /// `arc_to_radius()` appends up to four conic curves.
    /// `arc_to_radius()` implements the functionality of SVG arc, although SVG sweep-flag value is
    /// opposite the integer value of sweep; SVG sweep-flag uses 1 for clockwise, while
    /// [`PathDirection::CW`] cast to int is zero.
    ///
    /// - `r`: radii on axes before x-axis rotation
    /// - `x_axis_rotate`: x-axis rotation in degrees; positive values are clockwise
    /// - `large_arc`: chooses smaller or larger arc
    /// - `sweep`: chooses clockwise or counterclockwise arc
    /// - `xy`: end of arc
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn arc_to_radius(
        &mut self,
        r: impl Into<Point>,
        x_axis_rotate: scalar,
        large_arc: ArcSize,
        sweep: PathDirection,
        xy: impl Into<Point>,
    ) -> &mut Self {
        unsafe {
            self.native_mut().arcTo2(
                r.into().into_native(),
                x_axis_rotate,
                large_arc,
                sweep,
                xy.into().into_native(),
            );
        }
        self
    }

    /// Appends arc to the builder, as the start of new contour. Arc added is part of ellipse
    /// bounded by oval, from `start_angle` through `sweep_angle`. Both `start_angle` and
    /// `sweep_angle` are measured in degrees, where zero degrees is aligned with the
    /// positive x-axis, and positive sweeps extends arc clockwise.
    ///
    /// If `sweep_angle` <= -360, or `sweep_angle` >= 360; and `start_angle` modulo 90 is nearly
    /// zero, append oval instead of arc. Otherwise, `sweep_angle` values are treated
    /// modulo 360, and arc may or may not draw depending on numeric rounding.
    ///
    /// - `oval`: bounds of ellipse containing arc
    /// - `start_angle_deg`: starting angle of arc in degrees
    /// - `sweep_angle_deg`: sweep, in degrees. Positive is clockwise; treated modulo 360
    ///
    /// # Returns
    /// reference to this builder
    pub fn add_arc(
        &mut self,
        oval: impl AsRef<Rect>,
        start_angle_deg: scalar,
        sweep_angle_deg: scalar,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .addArc(oval.as_ref().native(), start_angle_deg, sweep_angle_deg);
        }
        self
    }

    pub fn add_line(&mut self, a: impl Into<Point>, b: impl Into<Point>) -> &mut Self {
        self.move_to(a).line_to(b)
    }

    /// Adds a new contour to the [`PathBuilder`], defined by the rect, and wound in the
    /// specified direction. The verbs added to the path will be:
    ///
    /// [`PathVerb::Move`], [`PathVerb::Line`], [`PathVerb::Line`], [`PathVerb::Line`], [`PathVerb::Close`]
    ///
    /// start specifies which corner to begin the contour:
    ///     0: upper-left  corner
    ///     1: upper-right corner
    ///     2: lower-right corner
    ///     3: lower-left  corner
    ///
    /// This start point also acts as the implied beginning of the subsequent,
    /// contour, if it does not have an explicit `move_to()`. e.g.
    ///
    ///     path.add_rect(...)
    ///     // if we don't say move_to() here, we will use the rect's start point
    ///     path.line_to(...)
    ///
    /// - `rect`: [`Rect`] to add as a closed contour
    /// - `dir`: [`PathDirection`] to orient the new contour
    /// - `start_index`: initial corner of [`Rect`] to add
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn add_rect(
        &mut self,
        rect: impl AsRef<Rect>,
        dir: impl Into<Option<PathDirection>>,
        start_index: impl Into<Option<usize>>,
    ) -> &mut Self {
        let dir = dir.into().unwrap_or_default();
        let start_index = start_index.into().unwrap_or(0);
        unsafe {
            self.native_mut()
                .addRect(rect.as_ref().native(), dir, start_index.try_into().unwrap());
        }
        self
    }

    /// Adds oval to [`PathBuilder`], appending [`PathVerb::Move`], four [`PathVerb::Conic`], and [`PathVerb::Close`].
    /// Oval is upright ellipse bounded by [`Rect`] oval with radii equal to half oval width
    /// and half oval height. Oval begins at (oval.right, oval.center_y()) and continues
    /// clockwise if dir is [`PathDirection::CW`], counterclockwise if dir is [`PathDirection::CCW`].
    ///
    /// - `rect`: bounds of ellipse added
    /// - `dir`: [`PathDirection`] to wind ellipse
    /// - `start_index`: index of initial point of ellipse
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn add_oval(
        &mut self,
        rect: impl AsRef<Rect>,
        dir: impl Into<Option<PathDirection>>,
        start_index: impl Into<Option<usize>>,
    ) -> &mut Self {
        let dir = dir.into().unwrap_or_default();
        // m86: default start index changed from 0 to 1
        let start_index = start_index.into().unwrap_or(1);
        unsafe {
            self.native_mut()
                .addOval(rect.as_ref().native(), dir, start_index.try_into().unwrap());
        }
        self
    }

    /// Appends [`RRect`] to [`PathBuilder`], creating a new closed contour. If dir is [`PathDirection::CW`],
    /// [`RRect`] winds clockwise. If dir is [`PathDirection::CCW`], [`RRect`] winds counterclockwise.
    ///
    /// After appending, [`PathBuilder`] may be empty, or may contain: [`Rect`], oval, or [`RRect`].
    ///
    /// - `rect`: [`RRect`] to add
    /// - `dir`: [`PathDirection`] to wind [`RRect`]
    /// - `start_index`: index of initial point of [`RRect`]
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn add_rrect(
        &mut self,
        rect: impl AsRef<RRect>,
        dir: impl Into<Option<PathDirection>>,
        start_index: impl Into<Option<usize>>,
    ) -> &mut Self {
        let dir = dir.into().unwrap_or_default();
        // m86: default start index changed from 0 to 6 or 7 depending on the path's direction.
        let start_index =
            start_index
                .into()
                .unwrap_or(if dir == PathDirection::CW { 6 } else { 7 });
        unsafe {
            self.native_mut().addRRect(
                rect.as_ref().native(),
                dir,
                start_index.try_into().unwrap(),
            );
        }
        self
    }

    /// Adds circle centered at (x, y) of size radius to [`PathBuilder`], appending [`PathVerb::Move`],
    /// four [`PathVerb::Conic`], and [`PathVerb::Close`]. Circle begins at: (x + radius, y), continuing
    /// clockwise if dir is [`PathDirection::CW`], and counterclockwise if dir is [`PathDirection::CCW`].
    ///
    /// Has no effect if radius is zero or negative.
    ///
    /// - `center`: center of circle
    /// - `radius`: distance from center to edge
    /// - `dir`: [`PathDirection`] to wind circle
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn add_circle(
        &mut self,
        center: impl Into<Point>,
        radius: scalar,
        dir: impl Into<Option<PathDirection>>,
    ) -> &mut Self {
        let center = center.into();
        let dir = dir.into().unwrap_or_default();
        unsafe {
            self.native_mut()
                .addCircle(center.into_native(), radius, dir);
        }
        self
    }

    /// Adds contour created from line array, adding (pts.len() - 1) line segments.
    /// Contour added starts at `pts[0]`, then adds a line for every additional [`Point`]
    /// in pts array. If close is true, appends [`PathVerb::Close`] to [`Path`], connecting
    /// `pts[count - 1]` and `pts[0]`.
    ///
    /// - `pts`: array of line sharing end and start [`Point`]
    /// - `close`: true to add line connecting contour end and start
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn add_polygon(&mut self, pts: &[Point], close: bool) -> &mut Self {
        unsafe {
            sb::C_SkPathBuilder_addPolygon(
                self.native_mut(),
                pts.native().as_ptr(),
                pts.len(),
                close,
            );
        }
        self
    }

    /// Appends src to [`PathBuilder`].
    ///
    /// If mode is [`path::AddPathMode::Append`], src verb array, [`Point`] array, and conic weights are
    /// added unaltered. If mode is [`path::AddPathMode::Extend`], add line before appending
    /// verbs, [`Point`], and conic weights.
    ///
    /// - `path`: [`Path`] verbs, [`Point`], and conic weights to add
    /// - `mode`: [`path::AddPathMode::Append`] or [`path::AddPathMode::Extend`]
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn add_path(
        &mut self,
        path: &Path,
        mode: impl Into<Option<path::AddPathMode>>,
    ) -> &mut Self {
        self.add_path_with_offset(path, (0.0, 0.0), mode)
    }

    /// Appends src to [`PathBuilder`], offset by (dx, dy).
    ///
    /// If mode is [`path::AddPathMode::Append`], src verb array, [`Point`] array, and conic weights are
    /// added unaltered except for the applied offset. If mode is [`path::AddPathMode::Extend`], add line
    /// before appending verbs, [`Point`], and conic weights.
    ///
    /// - `path`: [`Path`] verbs, [`Point`], and conic weights to add
    /// - `offset`: offset applied to src before appending
    /// - `mode`: [`path::AddPathMode::Append`] or [`path::AddPathMode::Extend`]
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn add_path_with_offset(
        &mut self,
        path: &Path,
        offset: impl Into<Vector>,
        mode: impl Into<Option<path::AddPathMode>>,
    ) -> &mut Self {
        let offset = offset.into();
        unsafe {
            self.native_mut().addPath(
                path.native(),
                offset.x,
                offset.y,
                mode.into().unwrap_or(path::AddPathMode::Append),
            )
        };
        self
    }

    /// Appends src to [`PathBuilder`], transformed by matrix. Transformed curves may have different
    /// verbs, [`Point`], and conic weights.
    ///
    /// If mode is [`path::AddPathMode::Append`], src verb array, [`Point`] array, and conic weights are
    /// added unaltered. If mode is [`path::AddPathMode::Extend`], add line before appending
    /// verbs, [`Point`], and conic weights.
    ///
    /// - `src`: [`Path`] verbs, [`Point`], and conic weights to add
    /// - `matrix`: transform applied to src
    /// - `mode`: [`path::AddPathMode::Append`] or [`path::AddPathMode::Extend`]
    pub fn add_path_with_transform(
        &mut self,
        src: &Path,
        matrix: &Matrix,
        mode: impl Into<Option<path::AddPathMode>>,
    ) {
        unsafe {
            self.native_mut().addPath1(
                src.native(),
                matrix.native(),
                mode.into().unwrap_or(path::AddPathMode::Append),
            )
        };
    }

    /// Grows [`PathBuilder`] verb array and [`Point`] array to contain additional space.
    /// May improve performance and use less memory by
    /// reducing the number and size of allocations when creating [`PathBuilder`].
    ///
    /// - `extra_pt_count`: number of additional [`Point`] to allocate
    /// - `extra_verb_count`: number of additional verbs
    /// - `extra_conic_count`: number of additional conic weights
    pub fn inc_reserve(
        &mut self,
        extra_pt_count: usize,
        extra_verb_count: usize,
        extra_conic_count: usize,
    ) {
        unsafe {
            self.native_mut().incReserve(
                extra_pt_count.try_into().unwrap(),
                extra_verb_count.try_into().unwrap(),
                extra_conic_count.try_into().unwrap(),
            )
        }
    }

    /// Offsets [`Point`] array by (dx, dy).
    ///
    /// - `d`: offset added to [`Point`] array coordinates
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn offset(&mut self, d: impl Into<Vector>) -> &mut Self {
        let d = d.into();
        unsafe {
            self.native_mut().offset(d.x, d.y);
        }
        self
    }

    /// Transforms verb array, [`Point`] array, and weight by matrix.
    /// transform may change verbs and increase their number.
    ///
    /// - `matrix`: [`Matrix`] to apply to [`Path`]
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn transform(&mut self, matrix: &Matrix) -> &mut Self {
        unsafe {
            self.native_mut().transform(matrix.native());
        }
        self
    }

    /// Returns true if the builder is empty, or all of its points are finite.
    pub fn is_finite(&self) -> bool {
        unsafe { self.native().isFinite() }
    }

    /// Replaces [`PathFillType`] with its inverse. The inverse of [`PathFillType`] describes the area
    /// unmodified by the original [`PathFillType`].
    ///
    /// # Returns
    /// reference to [`PathBuilder`]
    pub fn toggle_inverse_fill_type(&mut self) -> &mut Self {
        let n = self.native_mut();
        n.fFillType = n.fFillType.toggle_inverse();
        self
    }

    /// Returns if [`Path`] is empty.
    /// Empty [`PathBuilder`] may have [`PathFillType`] but has no [`Point`], [`PathVerb`], or conic weight.
    /// [`PathBuilder::new()`] constructs empty [`PathBuilder`]; `reset()` and `rewind()` make [`Path`] empty.
    ///
    /// # Returns
    /// true if the path contains no [`PathVerb`] array
    pub fn is_empty(&self) -> bool {
        unsafe { sb::C_SkPathBuilder_isEmpty(self.native()) }
    }

    /// Returns last point on [`PathBuilder`]. Returns `None` if [`Point`] array is empty.
    ///
    /// # Returns
    /// last [`Point`] if [`Point`] array contains one or more [`Point`], otherwise `None`
    pub fn get_last_pt(&self) -> Option<Point> {
        let mut p = Point::default();
        unsafe { sb::C_SkPathBuilder_getLastPt(self.native(), p.native_mut()) }.then_some(p)
    }

    /// Change the point at the specified index (see `count_points()`).
    /// If index is out of range, the call does nothing.
    ///
    /// - `index`: which point to replace
    /// - `p`: the new point value
    pub fn set_point(&mut self, index: usize, p: impl Into<Point>) {
        let p = p.into();
        unsafe { sb::C_SkPathBuilder_setPoint(self.native_mut(), index, *p.native()) }
    }

    /// Change the last point in the builder.
    /// If the builder is empty, the call does nothing.
    ///
    /// - `p`: the new point value
    pub fn set_last_point(&mut self, p: impl Into<Point>) {
        let len = self.points().len();
        if len != 0 {
            self.set_point(len - 1, p);
        };
    }

    /// Sets the last point on the path. If [`Point`] array is empty, append [`PathVerb::Move`] to
    /// verb array and append p to [`Point`] array.
    ///
    /// - `p`: last point
    #[deprecated(since = "0.93.0", note = "Use set_last_point() or set_point()")]
    pub fn set_last_pt(&mut self, p: impl Into<Point>) {
        let p = p.into();
        unsafe { self.native_mut().setLastPt(p.into_native()) };
    }

    /// Returns the number of points in [`PathBuilder`].
    /// [`Point`] count is initially zero.
    ///
    /// # Returns
    /// [`PathBuilder`] [`Point`] array length
    pub fn count_points(&self) -> usize {
        unsafe { sb::C_SkPathBuilder_countPoints(self.native()) }
    }

    /// Returns if [`PathFillType`] describes area outside [`Path`] geometry. The inverse fill area
    /// extends indefinitely.
    ///
    /// # Returns
    /// true if [`PathFillType`] is [`PathFillType::InverseWinding`] or [`PathFillType::InverseEvenOdd`]
    pub fn is_inverse_fill_type(&self) -> bool {
        self.fill_type().is_inverse()
    }

    pub fn points(&self) -> &[Point] {
        unsafe {
            let mut len = 0;
            let points = sb::C_SkPathBuilder_points(self.native(), &mut len);
            safer::from_raw_parts(Point::from_native_ptr(points), len)
        }
    }

    pub fn verbs(&self) -> &[PathVerb] {
        unsafe {
            let mut len = 0;
            let verbs = sb::C_SkPathBuilder_verbs(self.native(), &mut len);
            safer::from_raw_parts(verbs, len)
        }
    }

    pub fn conic_weights(&self) -> &[scalar] {
        unsafe {
            let mut len = 0;
            let weights = sb::C_SkPathBuilder_conicWeights(self.native(), &mut len);
            safer::from_raw_parts(weights, len)
        }
    }
}

pub type DumpFormat = skia_bindings::SkPathBuilder_DumpFormat;
variant_name!(DumpFormat::Hex);

impl PathBuilder {
    /// Dumps the path to a string using the specified format.
    ///
    /// # Arguments
    /// * `format` - The format to use for dumping (Decimal or Hex)
    ///
    /// # Returns
    /// A string representation of the path
    pub fn dump_to_string(&self, format: DumpFormat) -> String {
        let mut str = crate::interop::String::default();
        unsafe {
            sb::C_SkPathBuilder_dumpToString(self.native(), format, str.native_mut());
        }
        str.as_str().to_owned()
    }

    /// Dumps the path to stdout using the specified format.
    ///
    /// # Arguments
    /// * `format` - The format to use for dumping (Decimal or Hex)
    pub fn dump(&self, format: DumpFormat) {
        unsafe {
            sb::C_SkPathBuilder_dump(self.native(), format);
        }
    }
}

impl PathBuilder {
    pub fn contains(&self, point: impl Into<Point>) -> bool {
        unsafe { self.native().contains(point.into().into_native()) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{paint, surfaces, Paint};

    use super::*;

    #[test]
    fn test_creation_snapshot_and_detach() {
        let mut builder = PathBuilder::new();
        let _path = builder.snapshot();
        let _path = builder.detach();
    }

    #[test]
    fn issue_1195() {
        let mut surface = surfaces::raster_n32_premul((1000, 1000)).unwrap();
        let canvas = surface.canvas();
        let mut paint = Paint::default();
        paint.set_style(paint::Style::Stroke);
        let mut path = PathBuilder::new();
        path.move_to((250., 250.));
        path.cubic_to((300., 300.), (700., 700.), (750., 750.));
        let path_sh = path.snapshot();
        canvas.draw_path(&path_sh, &paint);
    }

    #[test]
    fn test_equality() {
        let mut a = PathBuilder::new();
        let mut b = PathBuilder::new();

        // Empty builders should be equal
        assert_eq!(a, b);

        // Different paths should not be equal
        a.move_to((0., 0.));
        assert_ne!(a, b);

        // Same paths should be equal
        b.move_to((0., 0.));
        assert_eq!(a, b);

        // Different fill types should not be equal
        b.set_fill_type(PathFillType::EvenOdd);
        assert_ne!(a, b);

        a.set_fill_type(PathFillType::EvenOdd);
        assert_eq!(a, b);
    }

    #[test]
    fn test_compute_bounds() {
        let mut builder = PathBuilder::new();

        // Empty builder should return empty rect (0, 0, 0, 0), not None
        let empty_bounds = builder.compute_finite_bounds().unwrap();
        assert!(empty_bounds.is_empty());
        assert_eq!(empty_bounds, Rect::new(0., 0., 0., 0.));

        let empty_tight = builder.compute_tight_bounds().unwrap();
        assert!(empty_tight.is_empty());

        // Deprecated method should also return empty rect
        #[allow(deprecated)]
        let bounds = builder.compute_bounds();
        assert!(bounds.is_empty());

        // Add a simple rectangle
        builder.move_to((10., 20.));
        builder.line_to((100., 20.));
        builder.line_to((100., 80.));
        builder.line_to((10., 80.));
        builder.close();

        let finite_bounds = builder.compute_finite_bounds().unwrap();
        assert_eq!(finite_bounds.left, 10.);
        assert_eq!(finite_bounds.top, 20.);
        assert_eq!(finite_bounds.right, 100.);
        assert_eq!(finite_bounds.bottom, 80.);

        // For a polygon, tight bounds should equal finite bounds
        let tight_bounds = builder.compute_tight_bounds().unwrap();
        assert_eq!(finite_bounds, tight_bounds);

        // Test with curves - finite bounds includes control points
        let mut curve_builder = PathBuilder::new();
        curve_builder.move_to((0., 0.));
        curve_builder.cubic_to((50., 100.), (150., 100.), (200., 0.));

        let finite = curve_builder.compute_finite_bounds().unwrap();
        let tight = curve_builder.compute_tight_bounds().unwrap();

        // Finite bounds should include control points
        assert_eq!(finite.left, 0.);
        assert_eq!(finite.right, 200.);
        assert_eq!(finite.top, 0.);
        assert_eq!(finite.bottom, 100.);

        // Tight bounds should be smaller (not including full extent of control points)
        assert!(tight.bottom < finite.bottom);
    }

    #[test]
    fn test_dump_to_string() {
        let mut builder = PathBuilder::new();
        builder.move_to((10.5, 20.25));
        builder.line_to((100.0, 50.0));
        builder.cubic_to((30.0, 40.0), (70.0, 80.0), (90.0, 100.0));
        builder.close();

        // Test decimal format
        let decimal = builder.dump_to_string(DumpFormat::Decimal);
        println!("Decimal format:\n{}", decimal);
        assert!(decimal.contains("10.5"));
        assert!(decimal.contains("20.25"));

        // Test hex format
        let hex = builder.dump_to_string(DumpFormat::Hex);
        println!("Hex format:\n{}", hex);
        assert!(hex.contains("0x"));

        // Both should contain verb information
        assert!(decimal.contains("path") || decimal.contains("move") || decimal.contains("line"));
    }

    #[test]
    fn test_add_path_with_offset() {
        let mut src_builder = PathBuilder::new();
        src_builder.move_to((1.0, 2.0)).line_to((3.0, 4.0));
        let src = src_builder.detach();

        let mut dst = PathBuilder::new();
        dst.add_path_with_offset(&src, (10.0, 20.0), None);

        let points = dst.points();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].x, 11.0);
        assert_eq!(points[0].y, 22.0);
        assert_eq!(points[1].x, 13.0);
        assert_eq!(points[1].y, 24.0);
    }

    #[test]
    fn test_contains() {
        let mut builder = PathBuilder::new();
        builder.add_rect(Rect::new(10., 10., 100., 100.), None, None);

        assert!(builder.contains((50., 50.)));
        assert!(builder.contains((10., 10.)));
        assert!(!builder.contains((5., 5.)));
        assert!(!builder.contains((150., 150.)));
    }

    #[test]
    fn test_add_circle() {
        let mut builder = PathBuilder::new();
        builder.add_circle((50., 50.), 25., None);

        let bounds = builder.compute_finite_bounds().unwrap();
        assert_eq!(bounds, Rect::new(25., 25., 75., 75.));

        assert!(builder.contains((50., 50.)));
        assert!(!builder.contains((10., 10.)));
    }
}
