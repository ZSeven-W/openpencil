use crate::{prelude::*, BlendMode, NativeFlattenable};
use skia_bindings::{self as sb, SkBlender, SkFlattenable, SkRefCntBase};
use std::fmt;

/// [`Blender`] represents a custom blend function in the Skia pipeline.  A blender combines a source
/// color (the result of our paint) and destination color (from the canvas) into a final color.
pub type Blender = RCHandle<SkBlender>;
unsafe_send_sync!(Blender);
require_base_type!(SkBlender, SkFlattenable);

impl NativeRefCountedBase for SkBlender {
    type Base = SkRefCntBase;
}

impl NativeBase<SkFlattenable> for SkBlender {}

impl Blender {
    /// Create a blender that implements the specified [`BlendMode`].
    pub fn mode(mode: BlendMode) -> Blender {
        Blender::from_ptr(unsafe { sb::C_SkBlender_Mode(mode) }).unwrap()
    }
}

impl fmt::Debug for Blender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Blender").finish()
    }
}

impl NativeFlattenable for SkBlender {
    fn native_flattenable(&self) -> &SkFlattenable {
        unsafe { &*(self as *const SkBlender as *const SkFlattenable) }
    }

    fn native_deserialize(data: &[u8]) -> *mut Self {
        unsafe { sb::C_SkBlender_Deserialize(data.as_ptr() as _, data.len()) }
    }
}

impl From<BlendMode> for Blender {
    fn from(mode: BlendMode) -> Self {
        Blender::mode(mode)
    }
}
