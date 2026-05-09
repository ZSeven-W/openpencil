use std::fmt;

use skia_bindings::{self as sb, SkFlattenable, SkMaskFilter, SkRefCntBase};

use crate::{prelude::*, scalar, BlurStyle, NativeFlattenable};

/// MaskFilter is the base class for object that perform transformations on the mask before drawing
/// it. An example subclass is Blur.
pub type MaskFilter = RCHandle<SkMaskFilter>;
unsafe_send_sync!(MaskFilter);

impl NativeBase<SkRefCntBase> for SkMaskFilter {}
impl NativeBase<SkFlattenable> for SkMaskFilter {}

impl NativeRefCountedBase for SkMaskFilter {
    type Base = SkRefCntBase;
}

impl NativeFlattenable for SkMaskFilter {
    fn native_flattenable(&self) -> &SkFlattenable {
        self.base()
    }

    fn native_deserialize(data: &[u8]) -> *mut Self {
        unsafe { sb::C_SkMaskFilter_Deserialize(data.as_ptr() as _, data.len()) }
    }
}

impl fmt::Debug for MaskFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaskFilter").finish()
    }
}

impl MaskFilter {
    /// Create a blur mask filter.
    ///
    /// - `style`       The [`BlurStyle`] to use
    /// - `sigma`       Standard deviation of the Gaussian blur to apply. Must be > 0.
    /// - `respect_ctm` if `true` the blur's sigma is modified by the `ctm`.
    ///
    /// Returns the new blur mask filter
    pub fn blur(
        style: BlurStyle,
        sigma: scalar,
        respect_ctm: impl Into<Option<bool>>,
    ) -> Option<Self> {
        Self::from_ptr(unsafe {
            sb::C_SkMaskFilter_MakeBlur(style, sigma, respect_ctm.into().unwrap_or(true))
        })
    }
}
