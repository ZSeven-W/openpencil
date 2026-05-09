use std::fmt;
use std::marker::PhantomData;

use skia_bindings::{
    self as sb, SkContourMeasure, SkContourMeasureIter, SkContourMeasure_ForwardVerbIterator,
    SkContourMeasure_VerbMeasure, SkRefCntBase,
};

use crate::{prelude::*, scalar, Matrix, Path, PathBuilder, PathVerb, Point, Vector};

pub type ContourMeasure = RCHandle<SkContourMeasure>;
unsafe_send_sync!(ContourMeasure);

impl NativeRefCountedBase for SkContourMeasure {
    type Base = SkRefCntBase;
}

bitflags! {
    /// Flags that control what [`ContourMeasure::get_matrix()`] computes.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MatrixFlags : u32 {
        /// Compute the position component.
        const GET_POSITION = sb::SkContourMeasure_MatrixFlags_kGetPosition_MatrixFlag as _;
        /// Compute the tangent component.
        const GET_TANGENT = sb::SkContourMeasure_MatrixFlags_kGetTangent_MatrixFlag as _;
        /// Compute both position and tangent components.
        const GET_POS_AND_TAN = Self::GET_POSITION.bits() | Self::GET_TANGENT.bits();
    }
}

impl Default for MatrixFlags {
    fn default() -> Self {
        Self::GET_POS_AND_TAN
    }
}

impl fmt::Debug for ContourMeasure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContourMeasure")
            .field("length", &self.length())
            .field("is_closed", &self.is_closed())
            .finish()
    }
}

impl ContourMeasure {
    /// Returns the length of the contour.
    pub fn length(&self) -> scalar {
        unsafe { sb::C_SkContourMeasure_length(self.native()) }
    }

    /// Pins `distance` to `0 <= distance <= length()`, then computes the corresponding
    /// position and tangent.
    ///
    /// - `distance`: distance along the contour.
    #[must_use]
    pub fn pos_tan(&self, distance: scalar) -> Option<(Point, Vector)> {
        let mut p = Point::default();
        let mut v = Vector::default();
        unsafe {
            self.native()
                .getPosTan(distance, p.native_mut(), v.native_mut())
        }
        .then_some((p, v))
    }

    #[must_use]
    /// Pins `distance` to `0 <= distance <= length()`, then computes the corresponding
    /// matrix (by calling [`Self::pos_tan()`]).
    ///
    /// Returns `None` if there is no path, or a zero-length path was specified.
    ///
    /// - `distance`: distance along the contour.
    /// - `flags`: controls whether position, tangent, or both are computed.
    pub fn get_matrix(
        &self,
        distance: scalar,
        flags: impl Into<Option<MatrixFlags>>,
    ) -> Option<Matrix> {
        let mut m = Matrix::default();
        unsafe {
            self.native().getMatrix(
                distance,
                m.native_mut(),
                // note: depending on the OS, different representation types are generated for
                // MatrixFlags, so the try_into() is required, even though clippy complains about
                // it.
                #[allow(clippy::useless_conversion)]
                flags.into().unwrap_or_default().bits().try_into().unwrap(),
            )
        }
        .then_some(m)
    }

    #[deprecated(since = "0.94.0", note = "Use get_segment()")]
    #[must_use]
    /// Given a start and stop distance, appends the intervening segment(s) to `path_builder`.
    ///
    /// If the segment is zero-length, returns `false`; otherwise returns `true`.
    /// `start_d` and `stop_d` are pinned to legal values (`0..length()`). If
    /// `start_d > stop_d`, returns `false` and leaves `path_builder` untouched.
    ///
    /// Begins the segment with a `move_to` if `start_with_move_to` is `true`.
    ///
    /// - `start_d`: start distance along the contour.
    /// - `stop_d`: stop distance along the contour.
    /// - `path_builder`: destination that receives the segment.
    /// - `start_with_move_to`: whether to begin with `move_to`.
    pub fn segment(
        &self,
        start_d: scalar,
        stop_d: scalar,
        path_builder: &mut PathBuilder,
        start_with_move_to: bool,
    ) -> bool {
        self.get_segment(start_d, stop_d, path_builder, start_with_move_to)
    }

    #[must_use]
    /// Given a start and stop distance, appends the intervening segment(s) to `path_builder`.
    ///
    /// If the segment is zero-length, returns `false`; otherwise returns `true`.
    /// `start_d` and `stop_d` are pinned to legal values (`0..length()`). If
    /// `start_d > stop_d`, returns `false` and leaves `path_builder` untouched.
    ///
    /// Begins the segment with a `move_to` if `start_with_move_to` is `true`.
    ///
    /// - `start_d`: start distance along the contour.
    /// - `stop_d`: stop distance along the contour.
    /// - `path_builder`: destination that receives the segment.
    /// - `start_with_move_to`: whether to begin with `move_to`.
    pub fn get_segment(
        &self,
        start_d: scalar,
        stop_d: scalar,
        path_builder: &mut PathBuilder,
        start_with_move_to: bool,
    ) -> bool {
        unsafe {
            self.native().getSegment(
                start_d,
                stop_d,
                path_builder.native_mut(),
                start_with_move_to,
            )
        }
    }

    /// Returns `true` if the contour is closed.
    pub fn is_closed(&self) -> bool {
        unsafe { sb::C_SkContourMeasure_isClosed(self.native()) }
    }

    /// Returns an iterator over measurement data for the contour's verbs.
    pub fn verbs(&self) -> ForwardVerbIterator {
        let iterator =
            construct(|iterator| unsafe { sb::C_SkContourMeasure_begin(self.native(), iterator) });

        ForwardVerbIterator {
            iterator,
            contour_measure: self,
        }
    }
}

/// Utility for iterating over a contour's verbs.
pub struct ForwardVerbIterator<'a> {
    iterator: SkContourMeasure_ForwardVerbIterator,
    contour_measure: &'a ContourMeasure,
}
unsafe_send_sync!(ForwardVerbIterator<'_>);

impl fmt::Debug for ForwardVerbIterator<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForwardVerbIterator").finish()
    }
}

impl PartialEq for ForwardVerbIterator<'_> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            sb::C_SkContourMeasure_ForwardVerbIterator_Equals(&self.iterator, &other.iterator)
        }
    }
}

impl<'a> Iterator for ForwardVerbIterator<'a> {
    type Item = VerbMeasure<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let end = construct(|end| unsafe {
            sb::C_SkContourMeasure_end(self.contour_measure.native(), end)
        });
        if unsafe { sb::C_SkContourMeasure_ForwardVerbIterator_Equals(&self.iterator, &end) } {
            return None;
        }
        let item = construct(|item| unsafe {
            sb::C_SkContourMeasure_ForwardVerbIterator_item(&self.iterator, item)
        });
        unsafe { sb::C_SkContourMeasure_ForwardVerbIterator_next(&mut self.iterator) };
        Some(VerbMeasure {
            verb_measure: item,
            _pd: PhantomData,
        })
    }
}

/// Measurement data for an individual verb.
pub struct VerbMeasure<'a> {
    verb_measure: SkContourMeasure_VerbMeasure,
    _pd: PhantomData<ForwardVerbIterator<'a>>,
}
unsafe_send_sync!(VerbMeasure<'_>);

impl fmt::Debug for VerbMeasure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerbMeasure")
            .field("verb", &self.verb())
            .field("distance", &self.distance())
            .field("points", &self.points())
            .finish()
    }
}

impl VerbMeasure<'_> {
    /// Returns the verb type.
    pub fn verb(&self) -> PathVerb {
        self.verb_measure.fVerb
    }

    /// Returns the cumulative distance along the current contour.
    pub fn distance(&self) -> scalar {
        self.verb_measure.fDistance
    }

    /// Returns the verb points.
    pub fn points(&self) -> &[Point] {
        unsafe {
            safer::from_raw_parts(
                Point::from_native_ptr(self.verb_measure.fPts.fPtr),
                self.verb_measure.fPts.fSize,
            )
        }
    }
}

pub type ContourMeasureIter = Handle<SkContourMeasureIter>;
unsafe_send_sync!(ContourMeasureIter);

impl NativeDrop for SkContourMeasureIter {
    fn drop(&mut self) {
        unsafe {
            sb::C_SkContourMeasureIter_destruct(self);
        }
    }
}

impl Iterator for ContourMeasureIter {
    type Item = ContourMeasure;

    /// Iterates through contours in the path, returning a [`ContourMeasure`] for each contour.
    /// Returns `None` when iteration is complete.
    ///
    /// Only non-zero-length contours are returned, where a contour is the segments
    /// between a move verb and either:
    /// - the next move verb,
    /// - one or more close verbs,
    /// - or the end of the path.
    ///
    /// Zero-length contours are skipped.
    fn next(&mut self) -> Option<Self::Item> {
        ContourMeasure::from_ptr(unsafe { sb::C_SkContourMeasureIter_next(self.native_mut()) })
    }
}

impl fmt::Debug for ContourMeasureIter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContourMeasureIter").finish()
    }
}

impl ContourMeasureIter {
    /// Initializes the iterator with a path.
    ///
    /// The parts of the path that are needed are copied, so the caller is free
    /// to modify or delete the path after this call.
    ///
    /// `res_scale` controls the precision of the measure. Values greater than
    /// `1` increase precision (and may slow down the computation).
    ///
    /// - `path`: source path to iterate.
    /// - `force_closed`: whether open contours are treated as closed.
    /// - `res_scale`: optional precision scale (defaults to `1.0`).
    pub fn new(path: &Path, force_closed: bool, res_scale: impl Into<Option<scalar>>) -> Self {
        Self::from_path(path, force_closed, res_scale)
    }

    /// Initializes the iterator with a path.
    ///
    /// The parts of the path that are needed are copied, so the caller is free
    /// to modify or delete the path after this call.
    ///
    /// `res_scale` controls the precision of the measure. Values greater than
    /// `1` increase precision (and may slow down the computation).
    ///
    /// - `path`: source path to iterate.
    /// - `force_closed`: whether open contours are treated as closed.
    /// - `res_scale`: optional precision scale (defaults to `1.0`).
    pub fn from_path(
        path: &Path,
        force_closed: bool,
        res_scale: impl Into<Option<scalar>>,
    ) -> Self {
        Self::from_native_c(unsafe {
            SkContourMeasureIter::new1(path.native(), force_closed, res_scale.into().unwrap_or(1.0))
        })
    }

    /// Resets the iterator with a path.
    ///
    /// The parts of the path that are needed are copied, so the caller is free
    /// to modify or delete the path after this call.
    ///
    /// - `path`: source path to iterate.
    /// - `force_closed`: whether open contours are treated as closed.
    /// - `res_scale`: optional precision scale (defaults to `1.0`).
    pub fn reset(
        &mut self,
        path: &Path,
        force_closed: bool,
        res_scale: impl Into<Option<scalar>>,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .reset(path.native(), force_closed, res_scale.into().unwrap_or(1.0))
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::ContourMeasureIter;
    use crate::{Path, Rect};

    #[test]
    fn contour_and_verb_measure() {
        let p = Path::rect(Rect::new(0.0, 0.0, 10.0, 10.0), None);
        let measure = ContourMeasureIter::new(&p, true, None);
        for contour in measure {
            for verb in contour.verbs() {
                println!("verb: {verb:?}")
            }
        }
    }
}
