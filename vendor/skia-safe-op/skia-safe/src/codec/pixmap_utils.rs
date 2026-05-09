use crate::{prelude::*, EncodedOrigin, ImageInfo, Pixmap};
use skia_bindings as sb;

/// Copy the pixels in src into dst, applying the orientation transformations specified
/// by origin. If the inputs are invalid, this returns `false` and no copy is made.
/// # Safety
/// Unsafe in that it modifies the underlying pixels of `dst`.
pub unsafe fn orient(dst: &mut Pixmap, src: &Pixmap, origin: EncodedOrigin) -> bool {
    sb::C_SkPixmapUtils_Orient(dst.native_mut(), src.native(), origin.into_native())
}

/// Return a copy of the provided ImageInfo with the width and height swapped.
pub fn swap_width_height(info: &ImageInfo) -> ImageInfo {
    ImageInfo::construct(|ii| unsafe { sb::C_SkPixmapUtils_SwapWidthHeight(ii, info.native()) })
}
