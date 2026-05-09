pub type GlyphId = skia_bindings::SkGlyphID;

pub type Unichar = skia_bindings::SkUnichar;

// note std::time::Duration is used in place of MSec in public
// facing functions.
// pub(crate) type MSec = skia_bindings::SkMSec;
// pub(crate) const MSEC_MAX: u32 = std::i32::MAX as u32;

#[cfg(feature = "gpu")]
#[deprecated(since = "0.60.0", note = "Use gpu::Budgeted")]
pub type Budgeted = crate::gpu::Budgeted;
