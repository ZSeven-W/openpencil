use crate::{prelude::*, YUVColorSpace};
use skia_bindings::{self as sb, SkColorMatrix};
use std::fmt;

pub type ColorMatrix = Handle<SkColorMatrix>;
unsafe_send_sync!(ColorMatrix);

impl NativeDrop for SkColorMatrix {
    fn drop(&mut self) {}
}

impl PartialEq for ColorMatrix {
    fn eq(&self, other: &Self) -> bool {
        let mut array_self = [0.0f32; 20];
        let mut array_other = [0.0f32; 20];
        self.get_row_major(&mut array_self);
        other.get_row_major(&mut array_other);
        array_self == array_other
    }
}

impl Default for ColorMatrix {
    fn default() -> Self {
        ColorMatrix::construct(|cm| unsafe { sb::C_SkColorMatrix_Construct(cm) })
    }
}

impl fmt::Debug for ColorMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorMatrix")
            .field("mat", &self.native().fMat)
            .finish()
    }
}

impl ColorMatrix {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        m00: f32,
        m01: f32,
        m02: f32,
        m03: f32,
        m04: f32,
        m10: f32,
        m11: f32,
        m12: f32,
        m13: f32,
        m14: f32,
        m20: f32,
        m21: f32,
        m22: f32,
        m23: f32,
        m24: f32,
        m30: f32,
        m31: f32,
        m32: f32,
        m33: f32,
        m34: f32,
    ) -> Self {
        ColorMatrix::construct(|cm| unsafe {
            sb::C_SkColorMatrix_Construct2(
                cm, m00, m01, m02, m03, m04, m10, m11, m12, m13, m14, m20, m21, m22, m23, m24, m30,
                m31, m32, m33, m34,
            )
        })
    }

    pub fn rgb_to_yuv(rgb: YUVColorSpace) -> Self {
        Self::from_native_c(unsafe { sb::SkColorMatrix_RGBtoYUV(rgb) })
    }

    pub fn yuv_to_rgb(yuv: YUVColorSpace) -> Self {
        Self::from_native_c(unsafe { sb::SkColorMatrix_YUVtoRGB(yuv) })
    }

    pub fn set_identity(&mut self) {
        unsafe { self.native_mut().setIdentity() }
    }

    pub fn set_scale(
        &mut self,
        r_scale: f32,
        g_scale: f32,
        b_scale: f32,
        a_scale: impl Into<Option<f32>>,
    ) {
        unsafe {
            self.native_mut()
                .setScale(r_scale, g_scale, b_scale, a_scale.into().unwrap_or(1.0))
        }
    }

    pub fn post_translate(&mut self, dr: f32, dg: f32, db: f32, da: f32) {
        unsafe { self.native_mut().postTranslate(dr, dg, db, da) }
    }

    pub fn set_concat(&mut self, a: &ColorMatrix, b: &ColorMatrix) {
        unsafe { self.native_mut().setConcat(a.native(), b.native()) }
    }

    pub fn pre_concat(&mut self, mat: &ColorMatrix) {
        let self_ptr = self.native() as *const _;
        unsafe { self.native_mut().setConcat(self_ptr, mat.native()) }
    }

    pub fn post_concat(&mut self, mat: &ColorMatrix) {
        let self_ptr = self.native() as *const _;
        unsafe { self.native_mut().setConcat(mat.native(), self_ptr) }
    }

    pub fn set_saturation(&mut self, sat: f32) {
        unsafe { self.native_mut().setSaturation(sat) }
    }

    pub fn set_row_major(&mut self, src: &[f32; 20]) {
        unsafe {
            sb::C_SkColorMatrix_setRowMajor(self.native_mut(), src.as_ptr());
        }
    }

    pub fn get_row_major(&self, dst: &mut [f32; 20]) {
        unsafe {
            sb::C_SkColorMatrix_getRowMajor(self.native(), dst.as_mut_ptr());
        }
    }
}
