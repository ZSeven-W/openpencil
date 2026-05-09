use std::{fmt, marker::PhantomData, mem::forget, ptr};

use skia_bindings::{self as sb, SkPath, SkPath_Iter, SkPath_RawIter};

use crate::PathIter;
use crate::{
    interop::DynamicMemoryWStream, path_types, prelude::*, scalar, Data, Matrix, PathDirection,
    PathFillType, PathVerb, Point, RRect, Rect, Vector,
};

/// [`Path`] contain geometry. [`Path`] may be empty, or contain one or more verbs that
/// outline a figure. [`Path`] always starts with a move verb to a Cartesian coordinate,
/// and may be followed by additional verbs that add lines or curves.
/// Adding a close verb makes the geometry into a continuous loop, a closed contour.
/// [`Path`] may contain any number of contours, each beginning with a move verb.
///
/// [`Path`] contours may contain only a move verb, or may also contain lines,
/// quadratic beziers, conics, and cubic beziers. [`Path`] contours may be open or
/// closed.
///
/// When used to draw a filled area, [`Path`] describes whether the fill is inside or
/// outside the geometry. [`Path`] also describes the winding rule used to fill
/// overlapping contours.
///
/// Internally, [`Path`] lazily computes metrics likes bounds and convexity. Call
/// [`Path::update_bounds_cache`] to make [`Path`] thread safe.
pub type Path = Handle<SkPath>;
unsafe impl Send for Path {}

impl NativeDrop for SkPath {
    /// Releases ownership of any shared data and deletes data if [`Path`] is sole owner.
    ///
    /// example: <https://fiddle.skia.org/c/@Path_destructor>
    fn drop(&mut self) {
        unsafe { sb::C_SkPath_destruct(self) }
    }
}

impl NativeClone for SkPath {
    /// Constructs a copy of an existing path.
    /// Copy constructor makes two paths identical by value. Internally, path and
    /// the returned result share pointer values. The underlying verb array, [`Point`] array
    /// and weights are copied when modified.
    ///
    /// Creating a [`Path`] copy is very efficient and never allocates memory.
    /// [`Path`] are always copied by value from the interface; the underlying shared
    /// pointers are not exposed.
    ///
    /// * `path` - [`Path`] to copy by value
    ///
    /// Returns: copy of [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_copy_const_SkPath>
    fn clone(&self) -> Self {
        unsafe { SkPath::new1(self) }
    }
}

impl NativePartialEq for SkPath {
    /// Compares a and b; returns `true` if [`path::FillType`], verb array, [`Point`] array, and weights
    /// are equivalent.
    ///
    /// * `a` - [`Path`] to compare
    /// * `b` - [`Path`] to compare
    ///
    /// Returns: `true` if [`Path`] pair are equivalent
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkPath_Equals(self, rhs) }
    }
}

impl Default for Handle<SkPath> {
    /// See [`Self::new()`]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Path")
            .field("fill_type", &self.fill_type())
            .field("is_convex", &self.is_convex())
            .field("is_oval", &self.is_oval())
            .field("is_rrect", &self.is_rrect())
            .field("is_empty", &self.is_empty())
            .field("is_last_contour_closed", &self.is_last_contour_closed())
            .field("is_finite", &self.is_finite())
            .field("is_volatile", &self.is_volatile())
            .field("is_line", &self.is_line())
            .field("count_points", &self.count_points())
            .field("count_verbs", &self.count_verbs())
            .field("approximate_bytes_used", &self.approximate_bytes_used())
            .field("bounds", &self.bounds())
            .field("is_rect", &self.is_rect())
            .field("segment_masks", &self.segment_masks())
            .field("generation_id", &self.generation_id())
            .field("is_valid", &self.is_valid())
            .finish()
    }
}

/// [`Path`] contain geometry. [`Path`] may be empty, or contain one or more verbs that
/// outline a figure. [`Path`] always starts with a move verb to a Cartesian coordinate,
/// and may be followed by additional verbs that add lines or curves.
/// Adding a close verb makes the geometry into a continuous loop, a closed contour.
/// [`Path`] may contain any number of contours, each beginning with a move verb.
///
/// [`Path`] contours may contain only a move verb, or may also contain lines,
/// quadratic beziers, conics, and cubic beziers. [`Path`] contours may be open or
/// closed.
///
/// When used to draw a filled area, [`Path`] describes whether the fill is inside or
/// outside the geometry. [`Path`] also describes the winding rule used to fill
/// overlapping contours.
///
/// Internally, [`Path`] lazily computes metrics likes bounds and convexity. Call
/// [`Path::update_bounds_cache`] to make [`Path`] thread safe.
impl Path {
    /// Create a new path with the specified spans.
    ///
    /// The points and weights arrays are read in order, based on the sequence of verbs.
    ///
    /// Move    1 point
    /// Line    1 point
    /// Quad    2 points
    /// Conic   2 points and 1 weight
    /// Cubic   3 points
    /// Close   0 points
    ///
    /// If an illegal sequence of verbs is encountered, or the specified number of points
    /// or weights is not sufficient given the verbs, an empty Path is returned.
    ///
    /// A legal sequence of verbs consists of any number of Contours. A contour always begins
    /// with a Move verb, followed by 0 or more segments: Line, Quad, Conic, Cubic, followed
    /// by an optional Close.
    pub fn raw(
        points: &[Point],
        verbs: &[PathVerb],
        conic_weights: &[scalar],
        fill_type: PathFillType,
        is_volatile: impl Into<Option<bool>>,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Raw(
                path,
                points.native().as_ptr(),
                points.len(),
                verbs.as_ptr(),
                verbs.len(),
                conic_weights.as_ptr(),
                conic_weights.len(),
                fill_type,
                is_volatile.into().unwrap_or(false),
            )
        })
    }

    /// Create a new path with the specified spans.
    ///
    /// The points and weights arrays are read in order, based on the sequence of verbs.
    ///
    /// Move    1 point
    /// Line    1 point
    /// Quad    2 points
    /// Conic   2 points and 1 weight
    /// Cubic   3 points
    /// Close   0 points
    ///
    /// If an illegal sequence of verbs is encountered, or the specified number of points
    /// or weights is not sufficient given the verbs, an empty Path is returned.
    ///
    /// A legal sequence of verbs consists of any number of Contours. A contour always begins
    /// with a Move verb, followed by 0 or more segments: Line, Quad, Conic, Cubic, followed
    /// by an optional Close.
    #[deprecated(since = "0.88.0", note = "use raw()")]
    pub fn new_from(
        points: &[Point],
        verbs: &[u8],
        conic_weights: &[scalar],
        fill_type: PathFillType,
        is_volatile: impl Into<Option<bool>>,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Make(
                path,
                points.native().as_ptr(),
                points.len(),
                verbs.as_ptr(),
                verbs.len(),
                conic_weights.as_ptr(),
                conic_weights.len(),
                fill_type,
                is_volatile.into().unwrap_or(false),
            )
        })
    }

    pub fn rect_with_fill_type(
        rect: impl AsRef<Rect>,
        fill_type: PathFillType,
        dir: impl Into<Option<PathDirection>>,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Rect(
                path,
                rect.as_ref().native(),
                fill_type,
                dir.into().unwrap_or_default(),
            )
        })
    }

    pub fn rect(rect: impl AsRef<Rect>, dir: impl Into<Option<PathDirection>>) -> Self {
        Self::rect_with_fill_type(rect, PathFillType::default(), dir)
    }

    pub fn oval(oval: impl AsRef<Rect>, dir: impl Into<Option<PathDirection>>) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Oval(path, oval.as_ref().native(), dir.into().unwrap_or_default())
        })
    }

    pub fn oval_with_start_index(
        oval: impl AsRef<Rect>,
        dir: PathDirection,
        start_index: usize,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_OvalWithStartIndex(
                path,
                oval.as_ref().native(),
                dir,
                start_index.try_into().unwrap(),
            )
        })
    }

    pub fn circle(
        center: impl Into<Point>,
        radius: scalar,
        dir: impl Into<Option<PathDirection>>,
    ) -> Self {
        let center = center.into();
        Self::construct(|path| unsafe {
            sb::C_SkPath_Circle(
                path,
                center.x,
                center.y,
                radius,
                dir.into().unwrap_or(PathDirection::CW),
            )
        })
    }

    pub fn rrect(rect: impl AsRef<RRect>, dir: impl Into<Option<PathDirection>>) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_RRect(path, rect.as_ref().native(), dir.into().unwrap_or_default())
        })
    }

    pub fn rrect_with_start_index(
        rect: impl AsRef<RRect>,
        dir: PathDirection,
        start_index: usize,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_RRectWithStartIndex(
                path,
                rect.as_ref().native(),
                dir,
                start_index.try_into().unwrap(),
            )
        })
    }

    pub fn polygon(
        pts: &[Point],
        is_closed: bool,
        fill_type: impl Into<Option<PathFillType>>,
        is_volatile: impl Into<Option<bool>>,
    ) -> Self {
        Self::construct(|path| unsafe {
            sb::C_SkPath_Polygon(
                path,
                pts.native().as_ptr(),
                pts.len(),
                is_closed,
                fill_type.into().unwrap_or_default(),
                is_volatile.into().unwrap_or(false),
            )
        })
    }

    pub fn line(a: impl Into<Point>, b: impl Into<Point>) -> Self {
        Self::polygon(&[a.into(), b.into()], false, None, None)
    }

    /// Constructs an empty [`Path`]. By default, [`Path`] has no verbs, no [`Point`], and no weights.
    ///
    /// Returns: empty [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_empty_constructor>
    pub fn new_with_fill_type(fill_type: PathFillType) -> Self {
        Self::construct(|path| unsafe { sb::C_SkPath_Construct(path, fill_type) })
    }

    pub fn new() -> Self {
        Self::new_with_fill_type(PathFillType::default())
    }

    /// Returns a copy of this path in the current state.
    pub fn snapshot(&self) -> Self {
        self.clone()
    }

    /// Returns `true` if [`Path`] contain equal verbs and equal weights.
    /// If [`Path`] contain one or more conics, the weights must match.
    ///
    /// `conic_to()` may add different verbs depending on conic weight, so it is not
    /// trivial to interpolate a pair of [`Path`] containing conics with different
    /// conic weight values.
    ///
    /// * `compare` - [`Path`] to compare
    ///
    /// Returns: `true` if [`Path`] verb array and weights are equivalent
    ///
    /// example: <https://fiddle.skia.org/c/@Path_isInterpolatable>
    pub fn is_interpolatable(&self, compare: &Path) -> bool {
        unsafe { self.native().isInterpolatable(compare.native()) }
    }

    /// Interpolates between [`Path`] with [`Point`] array of equal size.
    /// Copy verb array and weights to out, and set out [`Point`] array to a weighted
    /// average of this [`Point`] array and ending [`Point`] array, using the formula:
    /// (Path Point * weight) + ending Point * (1 - weight).
    ///
    /// weight is most useful when between zero (ending [`Point`] array) and
    /// one (this Point_Array); will work with values outside of this
    /// range.
    ///
    /// `interpolate()` returns an empty [`Path`] if [`Point`] array is not the same size
    /// as ending [`Point`] array. Call `is_interpolatable()` to check [`Path`] compatibility
    /// prior to calling `make_interpolate`().
    ///
    /// * `ending` - [`Point`] array averaged with this [`Point`] array
    /// * `weight` - contribution of this [`Point`] array, and
    ///                one minus contribution of ending [`Point`] array
    ///
    /// Returns: [`Path`] replaced by interpolated averages
    ///
    /// example: <https://fiddle.skia.org/c/@Path_interpolate>
    pub fn interpolate(&self, ending: &Path, weight: scalar) -> Option<Self> {
        let mut out = Path::default();
        self.interpolate_inplace(ending, weight, &mut out)
            .then_some(out)
    }

    /// Interpolates between [`Path`] with [`Point`] array of equal size.
    /// Copy verb array and weights to out, and set out [`Point`] array to a weighted
    /// average of this [`Point`] array and ending [`Point`] array, using the formula:
    /// `(Path Point * weight) + ending Point * (1 - weight)`.
    ///
    /// `weight` is most useful when between zero (ending [`Point`] array) and
    /// one (this Point_Array); will work with values outside of this
    /// range.
    ///
    /// `interpolate_inplace()` returns `false` and leaves out unchanged if [`Point`] array is not
    /// the same size as ending [`Point`] array. Call `is_interpolatable()` to check [`Path`]
    /// compatibility prior to calling `interpolate_inplace()`.
    ///
    /// * `ending` - [`Point`] array averaged with this [`Point`] array
    /// * `weight` - contribution of this [`Point`] array, and
    ///                one minus contribution of ending [`Point`] array
    /// * `out` - [`Path`] replaced by interpolated averages
    ///
    /// Returns: `true` if [`Path`] contain same number of [`Point`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_interpolate>
    pub fn interpolate_inplace(&self, ending: &Path, weight: scalar, out: &mut Path) -> bool {
        unsafe {
            self.native()
                .interpolate(ending.native(), weight, out.native_mut())
        }
    }

    /// Returns [`PathFillType`], the rule used to fill [`Path`].
    ///
    /// Returns: current [`PathFillType`] setting
    pub fn fill_type(&self) -> PathFillType {
        unsafe { sb::C_SkPath_getFillType(self.native()) }
    }

    pub fn with_fill_type(&self, new_fill_type: PathFillType) -> Path {
        Self::construct(|p| unsafe { sb::C_SkPath_makeFillType(self.native(), new_fill_type, p) })
    }

    /// Returns if FillType describes area outside [`Path`] geometry. The inverse fill area
    /// extends indefinitely.
    ///
    /// Returns: `true` if FillType is `InverseWinding` or `InverseEvenOdd`
    pub fn is_inverse_fill_type(&self) -> bool {
        self.fill_type().is_inverse()
    }

    /// Creates an [`Path`] with the same properties and data, and with [`PathFillType`] replaced
    /// with its inverse.  The inverse of [`PathFillType`] describes the area unmodified by the
    /// original FillType.
    pub fn with_toggle_inverse_fill_type(&self) -> Self {
        Self::construct(|p| unsafe {
            sb::C_SkPath_makeToggleInverseFillType(self.native(), p);
        })
    }

    /// Returns `true` if the path is convex. If necessary, it will first compute the convexity.
    pub fn is_convex(&self) -> bool {
        unsafe { self.native().isConvex() }
    }

    /// Returns `true` if this path is recognized as an oval or circle.
    ///
    /// bounds receives bounds of oval.
    ///
    /// bounds is unmodified if oval is not found.
    ///
    /// * `bounds` - storage for bounding [`Rect`] of oval; may be `None`
    ///
    /// Returns: `true` if [`Path`] is recognized as an oval or circle
    ///
    /// example: <https://fiddle.skia.org/c/@Path_isOval>
    pub fn is_oval(&self) -> Option<Rect> {
        let mut bounds = Rect::default();
        unsafe { self.native().isOval(bounds.native_mut()) }.then_some(bounds)
    }

    /// Returns [`RRect`] if path is representable as [`RRect`].
    /// Returns `None` if path is representable as oval, circle, or [`Rect`].
    ///
    /// Returns: [`RRect`] if [`Path`] contains only [`RRect`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_isRRect>
    pub fn is_rrect(&self) -> Option<RRect> {
        let mut rrect = RRect::default();
        unsafe { self.native().isRRect(rrect.native_mut()) }.then_some(rrect)
    }

    /// Returns if [`Path`] is empty.
    /// Empty [`Path`] may have FillType but has no [`Point`], [`Verb`], or conic weight.
    /// [`Path::default()`] constructs empty [`Path`]; `reset()` and `rewind()` make [`Path`] empty.
    ///
    /// Returns: `true` if the path contains no [`Verb`] array
    pub fn is_empty(&self) -> bool {
        unsafe { self.native().isEmpty() }
    }

    /// Returns if contour is closed.
    /// Contour is closed if [`Path`] [`Verb`] array was last modified by `close()`. When stroked,
    /// closed contour draws [`crate::paint::Join`] instead of [`crate::paint::Cap`] at first and last [`Point`].
    ///
    /// Returns: `true` if the last contour ends with a [`Verb::Close`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_isLastContourClosed>
    pub fn is_last_contour_closed(&self) -> bool {
        unsafe { self.native().isLastContourClosed() }
    }

    /// Returns `true` for finite [`Point`] array values between negative SK_ScalarMax and
    /// positive SK_ScalarMax. Returns `false` for any [`Point`] array value of
    /// SK_ScalarInfinity, SK_ScalarNegativeInfinity, or SK_ScalarNaN.
    ///
    /// Returns: `true` if all [`Point`] values are finite
    pub fn is_finite(&self) -> bool {
        unsafe { self.native().isFinite() }
    }

    /// Returns `true` if the path is volatile; it will not be altered or discarded
    /// by the caller after it is drawn. [`Path`] by default have volatile set `false`, allowing
    /// [`crate::Surface`] to attach a cache of data which speeds repeated drawing. If `true`, [`crate::Surface`]
    /// may not speed repeated drawing.
    ///
    /// Returns: `true` if caller will alter [`Path`] after drawing
    pub fn is_volatile(&self) -> bool {
        self.native().fIsVolatile
    }

    /// Return a copy of [`Path`] with `is_volatile` indicating whether it will be altered
    /// or discarded by the caller after it is drawn. [`Path`] by default have volatile
    /// set `false`, allowing Skia to attach a cache of data which speeds repeated drawing.
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
    /// * `is_volatile` - `true` if caller will alter [`Path`] after drawing
    ///
    /// Returns: [`Path`]
    pub fn with_is_volatile(&self, is_volatile: bool) -> Self {
        Self::construct(|p| unsafe { sb::C_SkPath_makeIsVolatile(self.native(), is_volatile, p) })
    }

    /// Tests if line between [`Point`] pair is degenerate.
    /// Line with no length or that moves a very short distance is degenerate; it is
    /// treated as a point.
    ///
    /// exact changes the equality test. If `true`, returns `true` only if p1 equals p2.
    /// If `false`, returns `true` if p1 equals or nearly equals p2.
    ///
    /// * `p1` - line start point
    /// * `p2` - line end point
    /// * `exact` - if `false`, allow nearly equals
    ///
    /// Returns: `true` if line is degenerate; its length is effectively zero
    ///
    /// example: <https://fiddle.skia.org/c/@Path_IsLineDegenerate>
    pub fn is_line_degenerate(p1: impl Into<Point>, p2: impl Into<Point>, exact: bool) -> bool {
        unsafe { SkPath::IsLineDegenerate(p1.into().native(), p2.into().native(), exact) }
    }

    /// Tests if quad is degenerate.
    /// Quad with no length or that moves a very short distance is degenerate; it is
    /// treated as a point.
    ///
    /// * `p1` - quad start point
    /// * `p2` - quad control point
    /// * `p3` - quad end point
    /// * `exact` - if `true`, returns `true` only if p1, p2, and p3 are equal;
    ///               if `false`, returns `true` if p1, p2, and p3 are equal or nearly equal
    ///
    /// Returns: `true` if quad is degenerate; its length is effectively zero
    pub fn is_quad_degenerate(
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        p3: impl Into<Point>,
        exact: bool,
    ) -> bool {
        unsafe {
            SkPath::IsQuadDegenerate(
                p1.into().native(),
                p2.into().native(),
                p3.into().native(),
                exact,
            )
        }
    }

    /// Tests if cubic is degenerate.
    /// Cubic with no length or that moves a very short distance is degenerate; it is
    /// treated as a point.
    ///
    /// * `p1` - cubic start point
    /// * `p2` - cubic control point 1
    /// * `p3` - cubic control point 2
    /// * `p4` - cubic end point
    /// * `exact` - if `true`, returns `true` only if p1, p2, p3, and p4 are equal;
    ///               if `false`, returns `true` if p1, p2, p3, and p4 are equal or nearly equal
    ///
    /// Returns: `true` if cubic is degenerate; its length is effectively zero
    pub fn is_cubic_degenerate(
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        p3: impl Into<Point>,
        p4: impl Into<Point>,
        exact: bool,
    ) -> bool {
        unsafe {
            SkPath::IsCubicDegenerate(
                p1.into().native(),
                p2.into().native(),
                p3.into().native(),
                p4.into().native(),
                exact,
            )
        }
    }

    /// Returns `true` if [`Path`] contains only one line;
    /// [`Verb`] array has two entries: [`Verb::Move`], [`Verb::Line`].
    /// If [`Path`] contains one line and line is not `None`, line is set to
    /// line start point and line end point.
    /// Returns `false` if [`Path`] is not one line; line is unaltered.
    ///
    /// * `line` - storage for line. May be `None`
    ///
    /// Returns: `true` if [`Path`] contains exactly one line
    ///
    /// example: <https://fiddle.skia.org/c/@Path_isLine>
    pub fn is_line(&self) -> Option<(Point, Point)> {
        let mut line = [Point::default(); 2];
        #[allow(clippy::tuple_array_conversions)]
        unsafe { self.native().isLine(line.native_mut().as_mut_ptr()) }
            .then_some((line[0], line[1]))
    }

    /// Return a read-only view into the path's points.
    pub fn points(&self) -> &[Point] {
        unsafe {
            let mut len = 0;
            let points = sb::C_SkPath_points(self.native(), &mut len);
            safer::from_raw_parts(Point::from_native_ptr(points), len)
        }
    }

    /// Return a read-only view into the path's verbs.
    pub fn verbs(&self) -> &[PathVerb] {
        unsafe {
            let mut len = 0;
            let verbs = sb::C_SkPath_verbs(self.native(), &mut len);
            safer::from_raw_parts(verbs, len)
        }
    }

    /// Return a read-only view into the path's conic-weights.
    pub fn conic_weights(&self) -> &[scalar] {
        unsafe {
            let mut len = 0;
            let weights = sb::C_SkPath_conicWeights(self.native(), &mut len);
            safer::from_raw_parts(weights, len)
        }
    }

    pub fn count_points(&self) -> usize {
        self.points().len()
    }

    pub fn count_verbs(&self) -> usize {
        self.verbs().len()
    }

    /// Return the last point, or `None`
    ///
    /// Returns: The last if the path contains one or more [`Point`], else returns `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Path_getLastPt>
    pub fn last_pt(&self) -> Option<Point> {
        let mut p = Point::default();
        unsafe { sb::C_SkPath_getLastPt(self.native(), p.native_mut()) }.then_some(p)
    }
}

impl Path {
    /// Returns [`Point`] at index in [`Point`] array. Valid range for index is
    /// 0 to `count_points()` - 1.
    /// Returns `None` if index is out of range.
    /// DEPRECATED
    ///
    /// * `index` - [`Point`] array element selector
    ///
    /// Returns: [`Point`] array value
    ///
    /// example: <https://fiddle.skia.org/c/@Path_getPoint>
    #[deprecated(since = "0.91.0", note = "use points()")]
    pub fn get_point(&self, index: usize) -> Option<Point> {
        let p = Point::from_native_c(unsafe { self.native().getPoint(index.try_into().ok()?) });
        // Assuming that count_points() is somewhat slow, we check the index when a Point(0,0) is
        // returned.
        if p != Point::default() || index < self.count_points() {
            Some(p)
        } else {
            None
        }
    }

    /// Returns number of points in [`Path`].
    /// Copies N points from the path into the span, where N = min(#points, span capacity)
    /// DEPRECATED
    /// * `points` - span to receive the points. may be empty
    ///
    /// Returns: the number of points in the path
    ///
    /// example: <https://fiddle.skia.org/c/@Path_getPoints>
    #[deprecated(since = "0.91.0", note = "use points()")]
    pub fn get_points(&self, points: &mut [Point]) -> usize {
        unsafe {
            sb::C_SkPath_getPoints(
                self.native(),
                points.native_mut().as_mut_ptr(),
                points.len(),
            )
        }
    }

    /// Returns number of points in [`Path`].
    /// Copies N points from the path into the span, where N = min(#points, span capacity)
    /// DEPRECATED
    ///
    /// * `verbs` - span to store the verbs. may be empty.
    ///
    /// Returns: the number of verbs in the path
    ///
    /// example: <https://fiddle.skia.org/c/@Path_getVerbs>
    #[deprecated(since = "0.91.0")]
    pub fn get_verbs(&self, verbs: &mut [u8]) -> usize {
        unsafe { sb::C_SkPath_getVerbs(self.native(), verbs.as_mut_ptr(), verbs.len()) }
    }
}

impl Path {
    /// Returns the approximate byte size of the [`Path`] in memory.
    ///
    /// Returns: approximate size
    pub fn approximate_bytes_used(&self) -> usize {
        unsafe { self.native().approximateBytesUsed() }
    }

    /// Returns the min/max of the path's 'trimmed' points. The trimmed points are all of the
    /// points in the path, with the exception of the path having more than one contour, and the
    /// final contour containing only a [`Verb::Move`]. In that case the trailing [`Verb::Move`] point
    /// is ignored when computing the bounds.
    ///
    /// If the path has no verbs, or the path contains non-finite values,
    /// then `{0, 0, 0, 0}` is returned. (see `is_finite`())
    ///
    /// Returns: bounds of the path's points
    pub fn bounds(&self) -> &Rect {
        Rect::from_native_ref(unsafe { &*sb::C_SkPath_getBounds(self.native()) })
    }

    /// Updates internal bounds so that subsequent calls to `bounds()` are instantaneous.
    /// Unaltered copies of [`Path`] may also access cached bounds through `bounds()`.
    ///
    /// For now, identical to calling `bounds()` and ignoring the returned value.
    ///
    /// Call to prepare [`Path`] subsequently drawn from multiple threads,
    /// to avoid a race condition where each draw separately computes the bounds.
    pub fn update_bounds_cache(&mut self) -> &mut Self {
        self.bounds();
        self
    }

    /// Returns minimum and maximum axes values of the lines and curves in [`Path`].
    /// Returns (0, 0, 0, 0) if [`Path`] contains no points.
    /// Returned bounds width and height may be larger or smaller than area affected
    /// when [`Path`] is drawn.
    ///
    /// Includes [`Point`] associated with [`Verb::Move`] that define empty
    /// contours.
    ///
    /// Behaves identically to `bounds()` when [`Path`] contains
    /// only lines. If [`Path`] contains curves, computed bounds includes
    /// the maximum extent of the quad, conic, or cubic; is slower than `bounds()`;
    /// and unlike `bounds()`, does not cache the result.
    ///
    /// Returns: tight bounds of curves in [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_computeTightBounds>
    pub fn compute_tight_bounds(&self) -> Rect {
        Rect::construct(|r| unsafe { sb::C_SkPath_computeTightBounds(self.native(), r) })
    }

    /// Returns `true` if rect is contained by [`Path`].
    /// May return `false` when rect is contained by [`Path`].
    ///
    /// For now, only returns `true` if [`Path`] has one contour and is convex.
    /// rect may share points and edges with [`Path`] and be contained.
    /// Returns `true` if rect is empty, that is, it has zero width or height; and
    /// the [`Point`] or line described by rect is contained by [`Path`].
    ///
    /// * `rect` - [`Rect`], line, or [`Point`] checked for containment
    ///
    /// Returns: `true` if rect is contained
    ///
    /// example: <https://fiddle.skia.org/c/@Path_conservativelyContainsRect>
    pub fn conservatively_contains_rect(&self, rect: impl AsRef<Rect>) -> bool {
        unsafe {
            self.native()
                .conservativelyContainsRect(rect.as_ref().native())
        }
    }
}

impl Path {
    /// Approximates conic with quad array. Conic is constructed from start [`Point`] p0,
    /// control [`Point`] p1, end [`Point`] p2, and weight w.
    /// Quad array is stored in pts; this storage is supplied by caller.
    /// Maximum quad count is 2 to the pow2.
    /// Every third point in array shares last [`Point`] of previous quad and first [`Point`] of
    /// next quad. Maximum pts storage size is given by:
    /// (1 + 2 * (1 << pow2)) * sizeof([`Point`]).
    ///
    /// Returns quad count used the approximation, which may be smaller
    /// than the number requested.
    ///
    /// conic weight determines the amount of influence conic control point has on the curve.
    /// w less than one represents an elliptical section. w greater than one represents
    /// a hyperbolic section. w equal to one represents a parabolic section.
    ///
    /// Two quad curves are sufficient to approximate an elliptical conic with a sweep
    /// of up to 90 degrees; in this case, set pow2 to one.
    ///
    /// * `p0` - conic start [`Point`]
    /// * `p1` - conic control [`Point`]
    /// * `p2` - conic end [`Point`]
    /// * `w` - conic weight
    /// * `pts` - storage for quad array
    /// * `pow2` - quad count, as power of two, normally 0 to 5 (1 to 32 quad curves)
    ///
    /// Returns: number of quad curves written to pts
    pub fn convert_conic_to_quads(
        p0: impl Into<Point>,
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        w: scalar,
        pts: &mut [Point],
        pow2: usize,
    ) -> Option<usize> {
        let (p0, p1, p2) = (p0.into(), p1.into(), p2.into());
        let max_pts_count = 1 + 2 * (1 << pow2);
        if pts.len() >= max_pts_count {
            Some(unsafe {
                SkPath::ConvertConicToQuads(
                    p0.native(),
                    p1.native(),
                    p2.native(),
                    w,
                    pts.native_mut().as_mut_ptr(),
                    pow2.try_into().unwrap(),
                )
                .try_into()
                .unwrap()
            })
        } else {
            None
        }
    }

    // TODO: return type is probably worth a struct.

    /// Returns `Some(Rect, bool, PathDirection)` if [`Path`] is equivalent to [`Rect`] when filled.
    /// If `false`: rect, `is_closed`, and direction are unchanged.
    /// If `true`: rect, `is_closed`, and direction are written to.
    ///
    /// rect may be smaller than the [`Path`] bounds. [`Path`] bounds may include [`Verb::Move`] points
    /// that do not alter the area drawn by the returned rect.
    ///
    /// Returns: `Some(rect, is_closed, direction)` if [`Path`] contains [`Rect`]
    /// * `rect` - bounds of [`Rect`]
    /// * `is_closed` - set to `true` if [`Path`] is closed
    /// * `direction` - to [`Rect`] direction
    ///
    /// example: <https://fiddle.skia.org/c/@Path_isRect>
    pub fn is_rect(&self) -> Option<(Rect, bool, PathDirection)> {
        let mut rect = Rect::default();
        let mut is_closed = Default::default();
        let mut direction = PathDirection::default();
        unsafe {
            self.native()
                .isRect(rect.native_mut(), &mut is_closed, &mut direction)
        }
        .then_some((rect, is_closed, direction))
    }
}

/// AddPathMode chooses how `add_path()` appends. Adding one [`Path`] to another can extend
/// the last contour or start a new contour.
pub use sb::SkPath_AddPathMode as AddPathMode;
variant_name!(AddPathMode::Append);

impl Path {
    /// Return a copy of [`Path`] with verb array, [`Point`] array, and weight transformed
    /// by matrix. `try_make_transform` may change verbs and increase their number.
    ///
    /// If the resulting path has any non-finite values, returns `None`.
    ///
    /// * `matrix` - [`Matrix`] to apply to [`Path`]
    ///
    /// Returns: [`Path`] if finite, or `None`
    pub fn try_make_transform(&self, matrix: &Matrix) -> Option<Path> {
        Path::try_construct(|path| unsafe {
            sb::C_SkPath_tryMakeTransform(self.native(), matrix.native(), path)
        })
    }

    pub fn try_make_offset(&self, d: impl Into<Vector>) -> Option<Path> {
        let d = d.into();
        Path::try_construct(|path| unsafe {
            sb::C_SkPath_tryMakeOffset(self.native(), d.x, d.y, path)
        })
    }

    pub fn try_make_scale(&self, (sx, sy): (scalar, scalar)) -> Option<Path> {
        Path::try_construct(|path| unsafe {
            sb::C_SkPath_tryMakeScale(self.native(), sx, sy, path)
        })
    }

    // TODO: I think we should keep only the make_ variants.

    /// Return a copy of [`Path`] with verb array, [`Point`] array, and weight transformed
    /// by matrix. `with_transform` may change verbs and increase their number.
    ///
    /// If the resulting path has any non-finite values, this will still return a path
    /// but that path will return `true` for `is_finite()`.
    ///
    /// The newer pattern is to call [`try_make_transform`](Self::try_make_transform) which will only return a
    /// path if the result is finite.
    ///
    /// * `matrix` - [`Matrix`] to apply to [`Path`]
    ///
    /// Returns: [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_transform>
    #[must_use]
    pub fn with_transform(&self, matrix: &Matrix) -> Path {
        Path::construct(|path| unsafe {
            sb::C_SkPath_makeTransform(self.native(), matrix.native(), path)
        })
    }

    #[must_use]
    pub fn make_transform(&self, m: &Matrix) -> Path {
        self.with_transform(m)
    }

    /// Returns [`Path`] with [`Point`] array offset by `(d.x, d.y)`.
    ///
    /// * `d` - offset added to [`Point`] array coordinates
    ///
    /// Returns: [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_offset>
    #[must_use]
    pub fn with_offset(&self, d: impl Into<Vector>) -> Path {
        let d = d.into();
        Path::construct(|path| unsafe { sb::C_SkPath_makeOffset(self.native(), d.x, d.y, path) })
    }

    #[must_use]
    pub fn make_offset(&self, d: impl Into<Vector>) -> Path {
        self.with_offset(d)
    }

    #[must_use]
    pub fn make_scale(&self, (sx, sy): (scalar, scalar)) -> Path {
        self.make_transform(&Matrix::scale((sx, sy)))
    }
}

/// SegmentMask constants correspond to each drawing Verb type in [`crate::Path`]; for instance, if
/// [`crate::Path`] only contains lines, only the [`crate::path::SegmentMask::LINE`] bit is set.
pub type SegmentMask = path_types::PathSegmentMask;

impl Path {
    /// Returns a mask, where each set bit corresponds to a [`SegmentMask`] constant
    /// if [`Path`] contains one or more verbs of that type.
    /// Returns zero if [`Path`] contains no lines, or curves: quads, conics, or cubics.
    ///
    /// `segment_masks()` returns a cached result; it is very fast.
    ///
    /// Returns: [`SegmentMask`] bits or zero
    pub fn segment_masks(&self) -> SegmentMask {
        SegmentMask::from_bits_truncate(unsafe { self.native().getSegmentMasks() })
    }
}

/// Verb instructs [`Path`] how to interpret one or more [`Point`] and optional conic weight;
/// manage contour, and terminate [`Path`].
pub type Verb = sb::SkPath_Verb;
variant_name!(Verb::Line);

// SK_HIDE_PATH_EDIT_METHODS

impl Path {
    /// Specifies whether [`Path`] is volatile; whether it will be altered or discarded
    /// by the caller after it is drawn. [`Path`] by default have volatile set `false`, allowing
    /// `Device` to attach a cache of data which speeds repeated drawing.
    ///
    /// Mark temporary paths, discarded or modified after use, as volatile
    /// to inform `Device` that the path need not be cached.
    ///
    /// Mark animating [`Path`] volatile to improve performance.
    /// Mark unchanging [`Path`] non-volatile to improve repeated rendering.
    ///
    /// raster surface [`Path`] draws are affected by volatile for some shadows.
    /// GPU surface [`Path`] draws are affected by volatile for some shadows and concave geometries.
    ///
    /// * `is_volatile` - `true` if caller will alter [`Path`] after drawing
    ///
    /// Returns: reference to [`Path`]
    pub fn set_is_volatile(&mut self, is_volatile: bool) -> &mut Self {
        self.native_mut().fIsVolatile = is_volatile;
        self
    }

    /// Exchanges the verb array, [`Point`] array, weights, and [`PathFillType`] with other.
    /// Cached state is also exchanged. `swap()` internally exchanges pointers, so
    /// it is lightweight and does not allocate memory.
    ///
    /// `swap()` usage has largely been replaced by PartialEq.
    /// [`Path`] do not copy their content on assignment until they are written to,
    /// making assignment as efficient as swap().
    ///
    /// * `other` - [`Path`] exchanged by value
    ///
    /// example: <https://fiddle.skia.org/c/@Path_swap>
    pub fn swap(&mut self, other: &mut Path) -> &mut Self {
        unsafe { self.native_mut().swap(other.native_mut()) }
        self
    }

    /// Sets `FillType`, the rule used to fill [`Path`]. While there is no check
    /// that `ft` is legal, values outside of `FillType` are not supported.
    pub fn set_fill_type(&mut self, ft: PathFillType) -> &mut Self {
        self.native_mut().fFillType = ft;
        self
    }

    /// Replaces FillType with its inverse. The inverse of FillType describes the area
    /// unmodified by the original FillType.
    pub fn toggle_inverse_fill_type(&mut self) -> &mut Self {
        let n = self.native_mut();
        n.fFillType = n.fFillType.toggle_inverse();
        self
    }

    /// Sets [`Path`] to its initial state.
    /// Removes verb array, [`Point`] array, and weights, and sets FillType to `Winding`.
    /// Internal storage associated with [`Path`] is released.
    ///
    /// Returns: reference to [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_reset>
    pub fn reset(&mut self) -> &mut Self {
        unsafe { self.native_mut().reset() };
        self
    }
}

impl Path {
    /// Returns a copy of this path in the current state, and resets the path to empty.
    pub fn detach(&mut self) -> Self {
        let result = self.clone();
        self.reset();
        result
    }

    pub fn iter(&self) -> PathIter {
        PathIter::from_native_c(construct(|iter| unsafe {
            sb::C_SkPath_iter(self.native(), iter)
        }))
    }
}

/// Iterates through verb array, and associated [`Point`] array and conic weight.
/// Provides options to treat open contours as closed, and to ignore
/// degenerate data.
#[repr(transparent)]
pub struct Iter<'a>(SkPath_Iter, PhantomData<&'a Handle<SkPath>>);

impl NativeAccess for Iter<'_> {
    type Native = SkPath_Iter;

    fn native(&self) -> &SkPath_Iter {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkPath_Iter {
        &mut self.0
    }
}

impl Drop for Iter<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkPath_Iter_destruct(&mut self.0) }
    }
}

impl Default for Iter<'_> {
    /// Initializes [`Iter`] with an empty [`Path`]. `next()` on [`Iter`] returns
    /// [`Verb::Done`].
    /// Call `set_path` to initialize [`Iter`] at a later time.
    ///
    /// Returns: [`Iter`] of empty [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_Iter_Iter>
    fn default() -> Self {
        Iter(unsafe { SkPath_Iter::new() }, PhantomData)
    }
}

impl fmt::Debug for Iter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Iter")
            .field("conic_weight", &self.conic_weight())
            .field("is_close_line", &self.is_close_line())
            .field("is_closed_contour", &self.is_closed_contour())
            .finish()
    }
}

impl Iter<'_> {
    /// Sets [`Iter`] to return elements of verb array, [`Point`] array, and conic weight in
    /// path. If `force_close` is `true`, [`Iter`] will add [`Verb::Line`] and [`Verb::Close`] after each
    /// open contour. path is not altered.
    ///
    /// * `path` - [`Path`] to iterate
    /// * `force_close` - `true` if open contours generate [`Verb::Close`]
    ///
    /// Returns: [`Iter`] of path
    ///
    /// example: <https://fiddle.skia.org/c/@Path_Iter_const_SkPath>
    pub fn new(path: &Path, force_close: bool) -> Self {
        Self(
            unsafe { SkPath_Iter::new1(path.native(), force_close) },
            PhantomData,
        )
    }

    /// Sets [`Iter`] to return elements of verb array, [`Point`] array, and conic weight in
    /// path. If `force_close` is `true`, [`Iter`] will add [`Verb::Line`] and [`Verb::Close`] after each
    /// open contour. path is not altered.
    ///
    /// * `path` - [`Path`] to iterate
    /// * `force_close` - `true` if open contours generate [`Verb::Close`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_Iter_setPath>
    pub fn set_path(&mut self, path: &Path, force_close: bool) {
        unsafe {
            self.0.setPath(path.native(), force_close);
        }
    }

    /// Returns conic weight if `next()` returned [`Verb::Conic`].
    ///
    /// If `next()` has not been called, or `next()` did not return [`Verb::Conic`],
    /// result is `None`.
    ///
    /// Returns: conic weight for conic [`Point`] returned by `next()`
    pub fn conic_weight(&self) -> Option<scalar> {
        #[allow(clippy::map_clone)]
        self.native()
            .fConicWeights
            .into_non_null()
            .map(|p| unsafe { *p.as_ref() })
    }

    /// Returns `true` if last [`Verb::Line`] returned by `next()` was generated
    /// by [`Verb::Close`]. When `true`, the end point returned by `next()` is
    /// also the start point of contour.
    ///
    /// If `next()` has not been called, or `next()` did not return [`Verb::Line`],
    /// result is undefined.
    ///
    /// Returns: `true` if last [`Verb::Line`] was generated by [`Verb::Close`]
    pub fn is_close_line(&self) -> bool {
        unsafe { sb::C_SkPath_Iter_isCloseLine(self.native()) }
    }

    /// Returns `true` if subsequent calls to `next()` return [`Verb::Close`] before returning
    /// [`Verb::Move`]. if `true`, contour [`Iter`] is processing may end with [`Verb::Close`], or
    /// [`Iter`] may have been initialized with force close set to `true`.
    ///
    /// Returns: `true` if contour is closed
    ///
    /// example: <https://fiddle.skia.org/c/@Path_Iter_isClosedContour>
    pub fn is_closed_contour(&self) -> bool {
        unsafe { self.native().isClosedContour() }
    }
}

impl Iterator for Iter<'_> {
    type Item = (Verb, Vec<Point>);

    /// Returns next [`Verb`] in verb array, and advances [`Iter`].
    /// When verb array is exhausted, returns [`Verb::Done`].
    ///
    /// Zero to four [`Point`] are stored in pts, depending on the returned [`Verb`].
    ///
    /// * `pts` - storage for [`Point`] data describing returned [`Verb`]
    ///
    /// Returns: next [`Verb`] from verb array
    ///
    /// example: <https://fiddle.skia.org/c/@Path_RawIter_next>
    fn next(&mut self) -> Option<Self::Item> {
        let mut points = [Point::default(); Verb::MAX_POINTS];
        let verb = unsafe { self.native_mut().next(points.native_mut().as_mut_ptr()) };
        if verb != Verb::Done {
            Some((verb, points[0..verb.points()].into()))
        } else {
            None
        }
    }
}

#[repr(transparent)]
#[deprecated(
    since = "0.30.0",
    note = "User Iter instead, RawIter will soon be removed."
)]
pub struct RawIter<'a>(SkPath_RawIter, PhantomData<&'a Handle<SkPath>>);

#[allow(deprecated)]
impl NativeAccess for RawIter<'_> {
    type Native = SkPath_RawIter;

    fn native(&self) -> &SkPath_RawIter {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkPath_RawIter {
        &mut self.0
    }
}

#[allow(deprecated)]
impl Drop for RawIter<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkPath_RawIter_destruct(&mut self.0) }
    }
}

#[allow(deprecated)]
impl Default for RawIter<'_> {
    fn default() -> Self {
        RawIter(
            construct(|ri| unsafe { sb::C_SkPath_RawIter_Construct(ri) }),
            PhantomData,
        )
    }
}

#[allow(deprecated)]
impl RawIter<'_> {
    pub fn new(path: &Path) -> RawIter {
        RawIter::default().set_path(path)
    }

    pub fn set_path(mut self, path: &Path) -> RawIter {
        unsafe { self.native_mut().setPath(path.native()) }
        let r = RawIter(self.0, PhantomData);
        forget(self);
        r
    }

    pub fn peek(&self) -> Verb {
        unsafe { sb::C_SkPath_RawIter_peek(self.native()) }
    }

    pub fn conic_weight(&self) -> scalar {
        self.native().fConicWeight
    }
}

#[allow(deprecated)]
impl Iterator for RawIter<'_> {
    type Item = (Verb, Vec<Point>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut points = [Point::default(); Verb::MAX_POINTS];

        let verb = unsafe { self.native_mut().next(points.native_mut().as_mut_ptr()) };
        (verb != Verb::Done).then(|| (verb, points[0..verb.points()].into()))
    }
}

impl Path {
    /// Returns `true` if the point is contained by [`Path`], taking into
    /// account [`PathFillType`].
    ///
    /// * `point` - the point to test
    ///
    /// Returns: `true` if [`Point`] is in [`Path`]
    ///
    /// example: <https://fiddle.skia.org/c/@Path_contains>
    pub fn contains(&self, point: impl Into<Point>) -> bool {
        let point = point.into();
        unsafe { self.native().contains(point.into_native()) }
    }

    /// Writes text representation of [`Path`] to [`Data`].
    /// Set `dump_as_hex` `true` to generate exact binary representations
    /// of floating point numbers used in [`Point`] array and conic weights.
    ///
    /// * `dump_as_hex` - `true` if scalar values are written as hexadecimal
    ///
    /// example: <https://fiddle.skia.org/c/@Path_dump>
    pub fn dump_as_data(&self, dump_as_hex: bool) -> Data {
        let mut stream = DynamicMemoryWStream::new();
        unsafe {
            self.native()
                .dump(stream.native_mut().base_mut(), dump_as_hex);
        }
        stream.detach_as_data()
    }

    /// See [`Path::dump_as_data()`]
    pub fn dump(&self) {
        unsafe { self.native().dump(ptr::null_mut(), false) }
    }

    /// See [`Path::dump_as_data()`]
    pub fn dump_hex(&self) {
        unsafe { self.native().dump(ptr::null_mut(), true) }
    }

    // TODO: writeToMemory()?

    /// Writes [`Path`] to buffer, returning the buffer written to, wrapped in [`Data`].
    ///
    /// `serialize()` writes [`PathFillType`], verb array, [`Point`] array, conic weight, and
    /// additionally writes computed information like convexity and bounds.
    ///
    /// `serialize()` should only be used in concert with `read_from_memory`().
    /// The format used for [`Path`] in memory is not guaranteed.
    ///
    /// Returns: [`Path`] data wrapped in [`Data`] buffer
    ///
    /// example: <https://fiddle.skia.org/c/@Path_serialize>
    pub fn serialize(&self) -> Data {
        Data::from_ptr(unsafe { sb::C_SkPath_serialize(self.native()) }).unwrap()
    }

    // TODO: ReadFromMemory

    pub fn deserialize(data: &Data) -> Option<Path> {
        let mut path = Path::default();
        let bytes = data.as_bytes();
        unsafe { sb::C_SkPath_ReadFromMemory(path.native_mut(), bytes.as_ptr() as _, bytes.len()) }
            .then_some(path)
    }

    /// (See skbug.com/40032862)
    /// Returns a non-zero, globally unique value. A different value is returned
    /// if verb array, [`Point`] array, or conic weight changes.
    ///
    /// Setting [`PathFillType`] does not change generation identifier.
    ///
    /// Each time the path is modified, a different generation identifier will be returned.
    /// [`PathFillType`] does affect generation identifier on Android framework.
    ///
    /// Returns: non-zero, globally unique value
    ///
    /// example: <https://fiddle.skia.org/c/@Path_getGenerationID>
    pub fn generation_id(&self) -> u32 {
        unsafe { self.native().getGenerationID() }
    }

    /// Returns if [`Path`] data is consistent. Corrupt [`Path`] data is detected if
    /// internal values are out of range or internal storage does not match
    /// array dimensions.
    ///
    /// Returns: `true` if [`Path`] data is consistent
    pub fn is_valid(&self) -> bool {
        unsafe { self.native().isValid() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_points() {
        let p = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);
        let points_count = p.count_points();
        assert_eq!(points_count, 4);
    }

    #[test]
    fn test_fill_type() {
        let mut p = Path::default();
        assert_eq!(p.fill_type(), PathFillType::Winding);
        p.set_fill_type(PathFillType::EvenOdd);
        assert_eq!(p.fill_type(), PathFillType::EvenOdd);
        assert!(!p.is_inverse_fill_type());
        p.toggle_inverse_fill_type();
        assert_eq!(p.fill_type(), PathFillType::InverseEvenOdd);
        assert!(p.is_inverse_fill_type());
    }

    #[test]
    fn test_is_volatile() {
        let mut p = Path::default();
        assert!(!p.is_volatile());
        p.set_is_volatile(true);
        assert!(p.is_volatile());
    }

    #[test]
    fn test_path_rect() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        let path = Path::rect(r, None);
        assert_eq!(*path.bounds(), r);
    }

    #[test]
    fn test_points_verbs_conic_weights() {
        let path = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);

        // Test points()
        let points = path.points();
        assert_eq!(points.len(), 4);

        // Test verbs()
        let verbs = path.verbs();
        assert_eq!(verbs.len(), 5); // Move + 4 Lines + Close

        // Test conic_weights()
        let weights = path.conic_weights();
        assert_eq!(weights.len(), 0); // Rectangle has no conics
    }

    #[test]
    fn test_with_offset() {
        let path = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);
        let offset_path = path.with_offset((5.0, 5.0));

        assert_eq!(*offset_path.bounds(), Rect::new(5.0, 5.0, 15.0, 15.0));
    }

    #[test]
    fn test_with_transform() {
        let path = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);
        let matrix = Matrix::scale((2.0, 2.0));
        let transformed = path.with_transform(&matrix);

        assert_eq!(*transformed.bounds(), Rect::new(0.0, 0.0, 20.0, 20.0));
    }

    #[test]
    fn test_try_make_transform() {
        let path = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);

        // Test with finite transform
        let matrix = Matrix::scale((2.0, 2.0));
        let result = path.try_make_transform(&matrix);
        assert!(result.is_some());
        let transformed = result.unwrap();
        assert_eq!(*transformed.bounds(), Rect::new(0.0, 0.0, 20.0, 20.0));

        // Test with extreme scale that might produce non-finite values
        let extreme_matrix = Matrix::scale((f32::MAX, f32::MAX));
        let result = path.try_make_transform(&extreme_matrix);
        // The result depends on whether the transform produces finite values
        // This test documents the behavior
        if let Some(transformed) = result {
            assert!(transformed.is_finite());
        }
    }

    #[test]
    fn test_try_make_offset() {
        let path = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);

        // Test with finite offset
        let result = path.try_make_offset((5.0, 5.0));
        assert!(result.is_some());
        let offset_path = result.unwrap();
        assert_eq!(*offset_path.bounds(), Rect::new(5.0, 5.0, 15.0, 15.0));
    }

    #[test]
    fn test_try_make_scale() {
        let path = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);

        // Test with finite scale
        let result = path.try_make_scale((3.0, 3.0));
        assert!(result.is_some());
        let scaled = result.unwrap();
        assert_eq!(*scaled.bounds(), Rect::new(0.0, 0.0, 30.0, 30.0));
    }

    #[test]
    fn test_serialize_deserialize() {
        let path = Path::rect(Rect::new(10.0, 10.0, 20.0, 20.0), None);

        let data = path.serialize();
        let deserialized = Path::deserialize(&data).unwrap();

        assert_eq!(path, deserialized);
    }
}
