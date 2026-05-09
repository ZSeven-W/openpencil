use std::{fmt, marker::PhantomData};

use skia_bindings::{
    self as sb, SkPath, SkPathContourIter, SkPathContourIter_Rec, SkPathIter, SkPathIter_Rec,
};

use crate::{prelude::*, PathVerb, Point};

#[repr(transparent)]
pub struct PathIter<'a>(SkPathIter, PhantomData<&'a Handle<SkPath>>);

impl NativeAccess for PathIter<'_> {
    type Native = SkPathIter;

    fn native(&self) -> &SkPathIter {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkPathIter {
        &mut self.0
    }
}

impl Drop for PathIter<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkPathIter_destruct(&mut self.0) }
    }
}

impl fmt::Debug for PathIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathIter").finish()
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PathIterRec<'a>(SkPathIter_Rec, PhantomData<&'a ()>);

impl NativeAccess for PathIterRec<'_> {
    type Native = SkPathIter_Rec;

    fn native(&self) -> &Self::Native {
        &self.0
    }

    fn native_mut(&mut self) -> &mut Self::Native {
        &mut self.0
    }
}

impl fmt::Debug for PathIterRec<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathIterRec")
            .field("points", &self.points())
            .field("conic_weight", &self.conic_weight())
            .field("verb", &self.verb())
            .finish()
    }
}

impl PathIterRec<'_> {
    pub fn points(&self) -> &[Point] {
        unsafe {
            safer::from_raw_parts(
                Point::from_native_ptr(self.native().fPoints.fPtr),
                self.native().fPoints.fSize,
            )
        }
    }

    pub fn conic_weight(&self) -> f32 {
        self.native().fConicWeight
    }

    pub fn verb(&self) -> PathVerb {
        self.native().fVerb
    }
}

impl<'a> PathIter<'a> {
    pub fn new(points: &'a [Point], verbs: &'a [PathVerb], conics: &'a [f32]) -> Self {
        let iter: SkPathIter = construct(|iter| unsafe {
            sb::C_SkPathIter_Construct(
                points.native().as_ptr(),
                points.len(),
                verbs.as_ptr(),
                verbs.len(),
                conics.as_ptr(),
                conics.len(),
                iter,
            )
        });
        Self(iter, PhantomData)
    }

    pub(crate) fn from_native_c(native: SkPathIter) -> Self {
        Self(native, PhantomData)
    }
}

impl<'a> Iterator for PathIter<'a> {
    type Item = PathIterRec<'a>;

    /// Holds the current verb, and its associated points
    /// move:  `points()[0]`
    /// line:  `points()[0..1]`
    /// quad:  `points()[0..2]`
    /// conic: `points()[0..2]` `conic_weight()`
    /// cubic: `points()[0..3]`
    /// close: `points()[0..1]` ... as if close were a line from `pts[0]` to `pts[1]`
    fn next(&mut self) -> Option<Self::Item> {
        try_construct(|r| unsafe { sb::C_SkPathIter_next(self.native_mut(), r) })
            .map(|r| PathIterRec(r.into_inner(), PhantomData))
    }
}

impl PathIter<'_> {
    pub fn peek_next_verb(&mut self) -> Option<PathVerb> {
        let mut verb = PathVerb::Close;
        unsafe { sb::C_SkPathIter_peekNextVerb(self.native_mut(), &mut verb) }.then_some(verb)
    }
}

#[repr(transparent)]
pub struct PathContourIter<'a>(SkPathContourIter, PhantomData<&'a Handle<SkPath>>);

impl NativeAccess for PathContourIter<'_> {
    type Native = SkPathContourIter;

    fn native(&self) -> &SkPathContourIter {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkPathContourIter {
        &mut self.0
    }
}

impl Drop for PathContourIter<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkPathContourIter_destruct(&mut self.0) }
    }
}

impl fmt::Debug for PathContourIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathContourIter").finish()
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PathContourIterRec<'a>(SkPathContourIter_Rec, PhantomData<&'a ()>);

impl NativeAccess for PathContourIterRec<'_> {
    type Native = SkPathContourIter_Rec;

    fn native(&self) -> &Self::Native {
        &self.0
    }

    fn native_mut(&mut self) -> &mut Self::Native {
        &mut self.0
    }
}

impl fmt::Debug for PathContourIterRec<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathContourIterRec")
            .field("points", &self.points())
            .field("verb", &self.verbs())
            .field("conics", &self.conics())
            .finish()
    }
}

impl PathContourIterRec<'_> {
    pub fn points(&self) -> &[Point] {
        unsafe {
            safer::from_raw_parts(
                Point::from_native_ptr(self.native().fPoints.fPtr),
                self.native().fPoints.fSize,
            )
        }
    }

    pub fn verbs(&self) -> &[PathVerb] {
        unsafe { safer::from_raw_parts(self.native().fVerbs.fPtr, self.native().fVerbs.fSize) }
    }

    pub fn conics(&self) -> &[f32] {
        unsafe { safer::from_raw_parts(self.native().fConics.fPtr, self.native().fConics.fSize) }
    }
}

impl<'a> PathContourIter<'a> {
    pub fn new(points: &'a [Point], verbs: &'a [PathVerb], conics: &'a [f32]) -> Self {
        let iter: SkPathContourIter = construct(|iter| unsafe {
            sb::C_SkPathContourIter_Construct(
                points.native().as_ptr(),
                points.len(),
                verbs.as_ptr(),
                verbs.len(),
                conics.as_ptr(),
                conics.len(),
                iter,
            )
        });
        Self(iter, PhantomData)
    }

    #[allow(unused)]
    pub(crate) fn from_native_c(native: SkPathContourIter) -> Self {
        Self(native, PhantomData)
    }
}

impl<'a> Iterator for PathContourIter<'a> {
    type Item = PathContourIterRec<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        try_construct(|r| unsafe { sb::C_SkPathContourIter_next(self.native_mut(), r) })
            .map(|r| PathContourIterRec(r.into_inner(), PhantomData))
    }
}
