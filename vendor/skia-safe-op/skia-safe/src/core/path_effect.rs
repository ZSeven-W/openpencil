use crate::{prelude::*, Matrix, NativeFlattenable, Path, PathBuilder, Rect, StrokeRec};
use sb::SkPathEffect_INHERITED;
use skia_bindings::{self as sb, SkFlattenable, SkPathEffect, SkRefCntBase};
use std::fmt;

pub type PathEffect = RCHandle<SkPathEffect>;
unsafe_send_sync!(PathEffect);
require_type_equality!(SkPathEffect_INHERITED, SkFlattenable);

impl NativeBase<SkRefCntBase> for SkPathEffect {}
impl NativeBase<SkFlattenable> for SkPathEffect {}

impl NativeRefCountedBase for SkPathEffect {
    type Base = SkRefCntBase;
}

impl NativeFlattenable for SkPathEffect {
    fn native_flattenable(&self) -> &SkFlattenable {
        self.base()
    }

    fn native_deserialize(data: &[u8]) -> *mut Self {
        unsafe { sb::C_SkPathEffect_Deserialize(data.as_ptr() as _, data.len()) }
    }
}

impl fmt::Debug for PathEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathEffect")
            .field("needs_ctm", &self.needs_ctm())
            .finish()
    }
}

impl PathEffect {
    pub fn sum(first: impl Into<PathEffect>, second: impl Into<PathEffect>) -> PathEffect {
        PathEffect::from_ptr(unsafe {
            sb::C_SkPathEffect_MakeSum(first.into().into_ptr(), second.into().into_ptr())
        })
        .unwrap()
    }

    pub fn compose(first: impl Into<PathEffect>, second: impl Into<PathEffect>) -> PathEffect {
        PathEffect::from_ptr(unsafe {
            sb::C_SkPathEffect_MakeCompose(first.into().into_ptr(), second.into().into_ptr())
        })
        .unwrap()
    }

    pub fn filter_path(
        &self,
        src: &Path,
        stroke_rec: &StrokeRec,
        cull_rect: impl AsRef<Rect>,
    ) -> Option<(PathBuilder, StrokeRec)> {
        let mut dst = PathBuilder::new();
        let mut stroke_rec_r = stroke_rec.clone();
        self.filter_path_inplace(&mut dst, src, &mut stroke_rec_r, cull_rect)
            .then_some((dst, stroke_rec_r))
    }

    pub fn filter_path_inplace(
        &self,
        dst: &mut PathBuilder,
        src: &Path,
        stroke_rec: &mut StrokeRec,
        cull_rect: impl AsRef<Rect>,
    ) -> bool {
        unsafe {
            self.native().filterPath(
                dst.native_mut(),
                src.native(),
                stroke_rec.native_mut(),
                cull_rect.as_ref().native(),
                Matrix::new_identity().native(),
            )
        }
    }

    pub fn filter_path_inplace_with_matrix(
        &self,
        dst: &mut PathBuilder,
        src: &Path,
        stroke_rec: &mut StrokeRec,
        cull_rect: impl AsRef<Rect>,
        ctm: &Matrix,
    ) -> bool {
        unsafe {
            self.native().filterPath(
                dst.native_mut(),
                src.native(),
                stroke_rec.native_mut(),
                cull_rect.as_ref().native(),
                ctm.native(),
            )
        }
    }

    pub fn needs_ctm(&self) -> bool {
        unsafe { self.native().needsCTM() }
    }
}
