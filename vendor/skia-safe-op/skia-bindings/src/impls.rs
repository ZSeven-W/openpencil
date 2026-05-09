//! This file contains implementations for types that are re-exported in skia-safe.
//!
//! We could provide trait implementations in skia-safe, but then users of the library would have to
//! import the implementation type _and_ the trait.
//!
//! See also: <https://github.com/rust-lang/rfcs/issues/1880>

use crate::{
    SkAlphaType, SkBlendMode, SkBlendModeCoeff, SkPathFillType, SkPathVerb, SkPath_Verb,
    SkYUVColorSpace,
};
use std::ffi::CStr;

impl SkBlendMode {
    pub fn as_coeff(self) -> Option<(SkBlendModeCoeff, SkBlendModeCoeff)> {
        let mut src = SkBlendModeCoeff::Zero;
        let mut dst = SkBlendModeCoeff::Zero;
        if unsafe { crate::SkBlendMode_AsCoeff(self, &mut src, &mut dst) } {
            Some((src, dst))
        } else {
            None
        }
    }

    pub fn name(self) -> &'static str {
        unsafe {
            let name_ptr = crate::SkBlendMode_Name(self);
            CStr::from_ptr(name_ptr).to_str().unwrap()
        }
    }
}

//
// m84 introduced two different variants of the Path verb types.
// One with Done and one without.
//

impl SkPathVerb {
    /// The maximum number of points an iterator will return for the verb.
    pub const MAX_POINTS: usize = SkPath_Verb::MAX_POINTS;
    /// The number of points an iterator will return for the verb.
    pub fn points(self) -> usize {
        SkPath_Verb::from(self).points()
    }
}

impl SkPath_Verb {
    /// The maximum number of points an iterator will return for the verb.
    pub const MAX_POINTS: usize = 4;
    /// The number of points an iterator will return for the verb.
    pub fn points(self) -> usize {
        match self {
            SkPath_Verb::Move => 1,
            SkPath_Verb::Line => 2,
            SkPath_Verb::Quad => 3,
            SkPath_Verb::Conic => 3,
            SkPath_Verb::Cubic => 4,
            SkPath_Verb::Close => 0,
            SkPath_Verb::Done => 0,
        }
    }
}

impl From<SkPathVerb> for SkPath_Verb {
    fn from(v: SkPathVerb) -> Self {
        match v {
            SkPathVerb::Move => SkPath_Verb::Move,
            SkPathVerb::Line => SkPath_Verb::Line,
            SkPathVerb::Quad => SkPath_Verb::Quad,
            SkPathVerb::Conic => SkPath_Verb::Conic,
            SkPathVerb::Cubic => SkPath_Verb::Cubic,
            SkPathVerb::Close => SkPath_Verb::Close,
        }
    }
}

impl SkPathFillType {
    pub fn is_even_odd(self) -> bool {
        (self as i32 & 1) != 0
    }

    pub fn is_inverse(self) -> bool {
        (self as i32 & 2) != 0
    }

    #[must_use]
    pub fn to_non_inverse(self) -> Self {
        match self {
            Self::Winding => self,
            Self::EvenOdd => self,
            Self::InverseWinding => Self::Winding,
            Self::InverseEvenOdd => Self::EvenOdd,
        }
    }

    pub fn toggle_inverse(self) -> Self {
        match self {
            Self::Winding => Self::InverseWinding,
            Self::EvenOdd => Self::InverseEvenOdd,
            Self::InverseWinding => Self::Winding,
            Self::InverseEvenOdd => Self::EvenOdd,
        }
    }
}

impl SkAlphaType {
    pub fn is_opaque(self) -> bool {
        self == SkAlphaType::Opaque
    }
}

impl SkYUVColorSpace {
    pub fn is_limited_range(self) -> bool {
        unsafe { crate::SkYUVColorSpaceIsLimitedRange(self) }
    }
}

#[cfg(feature = "gl")]
impl From<crate::GrGLenum> for crate::GrGLFormat {
    fn from(e: crate::GrGLenum) -> Self {
        unsafe { crate::C_GrGLFormatFromGLEnum(e) }
    }
}

#[cfg(feature = "gl")]
impl From<crate::GrGLFormat> for crate::GrGLenum {
    fn from(format: crate::GrGLFormat) -> Self {
        unsafe { crate::C_GrGLFormatToEnum(format) }
    }
}

#[cfg(feature = "vulkan")]
mod vulkan {
    impl PartialEq for crate::VkComponentMapping {
        fn eq(&self, other: &Self) -> bool {
            self.r == other.r && self.g == other.g && self.b == other.b && self.a == other.a
        }
    }

    impl Eq for crate::VkComponentMapping {}
}

#[cfg(feature = "d3d")]
mod d3d {
    use std::marker::PhantomData;

    impl<T> Default for crate::gr_cp<T> {
        fn default() -> Self {
            Self {
                fObject: std::ptr::null_mut(),
                _phantom_0: PhantomData,
            }
        }
    }

    impl Default for crate::GrD3DTextureResourceInfo {
        fn default() -> Self {
            let mut instance = std::mem::MaybeUninit::uninit();
            unsafe {
                crate::C_GrD3DTextureResourceInfo_Construct(instance.as_mut_ptr());
                instance.assume_init()
            }
        }
    }
}
