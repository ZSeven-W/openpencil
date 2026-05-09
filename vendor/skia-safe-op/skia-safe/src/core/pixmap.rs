use crate::{
    prelude::*, AlphaType, Color, Color4f, ColorSpace, ColorType, IPoint, IRect, ISize, ImageInfo,
    SamplingOptions,
};
use skia_bindings::{self as sb, SkPixmap};
use std::{ffi::c_void, fmt, marker::PhantomData, mem, os::raw, ptr, slice};

#[repr(transparent)]
pub struct Pixmap<'a> {
    inner: Handle<SkPixmap>,
    pd: PhantomData<&'a mut [u8]>,
}

impl NativeDrop for SkPixmap {
    fn drop(&mut self) {
        unsafe { sb::C_SkPixmap_destruct(self) }
    }
}

impl Default for Pixmap<'_> {
    fn default() -> Self {
        Self::from_native_c(SkPixmap {
            fPixels: ptr::null(),
            fRowBytes: 0,
            fInfo: construct(|ii| unsafe { sb::C_SkImageInfo_Construct(ii) }),
        })
    }
}

impl fmt::Debug for Pixmap<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pixmap")
            .field("row_bytes", &self.row_bytes())
            .field("info", self.info())
            .finish()
    }
}

impl<'pixels> Pixmap<'pixels> {
    pub fn new(info: &ImageInfo, pixels: &'pixels mut [u8], row_bytes: usize) -> Option<Self> {
        if row_bytes < info.min_row_bytes() {
            return None;
        }
        if pixels.len() < info.compute_byte_size(row_bytes) {
            return None;
        }

        Some(Pixmap::from_native_c(SkPixmap {
            fPixels: pixels.as_mut_ptr() as _,
            fRowBytes: row_bytes,
            fInfo: info.native().clone(),
        }))
    }

    pub fn reset(&mut self) -> &mut Self {
        unsafe { self.native_mut().reset() }
        self
    }

    // TODO: reset() function that re-borrows pixels?

    pub fn set_color_space(&mut self, color_space: impl Into<Option<ColorSpace>>) -> &mut Self {
        unsafe {
            sb::C_SkPixmap_setColorSpace(self.native_mut(), color_space.into().into_ptr_or_null())
        }
        self
    }

    #[must_use]
    pub fn extract_subset(&self, area: impl AsRef<IRect>) -> Option<Self> {
        let mut pixmap = Pixmap::default();
        unsafe {
            self.native()
                .extractSubset(pixmap.native_mut(), area.as_ref().native())
        }
        .then_some(pixmap)
    }

    pub fn info(&self) -> &ImageInfo {
        ImageInfo::from_native_ref(&self.native().fInfo)
    }

    pub fn row_bytes(&self) -> usize {
        self.native().fRowBytes
    }

    pub fn addr(&self) -> *const c_void {
        self.native().fPixels
    }

    pub fn width(&self) -> i32 {
        self.info().width()
    }

    pub fn height(&self) -> i32 {
        self.info().height()
    }

    pub fn dimensions(&self) -> ISize {
        self.info().dimensions()
    }

    pub fn color_type(&self) -> ColorType {
        self.info().color_type()
    }

    pub fn alpha_type(&self) -> AlphaType {
        self.info().alpha_type()
    }

    pub fn color_space(&self) -> Option<ColorSpace> {
        ColorSpace::from_unshared_ptr(unsafe { self.native().colorSpace() })
    }

    pub fn is_opaque(&self) -> bool {
        self.alpha_type().is_opaque()
    }

    pub fn bounds(&self) -> IRect {
        IRect::from_wh(self.width(), self.height())
    }

    pub fn row_bytes_as_pixels(&self) -> usize {
        self.row_bytes() >> self.shift_per_pixel()
    }

    pub fn shift_per_pixel(&self) -> usize {
        self.info().shift_per_pixel()
    }

    pub fn compute_byte_size(&self) -> usize {
        self.info().compute_byte_size(self.row_bytes())
    }

    pub fn compute_is_opaque(&self) -> bool {
        unsafe { self.native().computeIsOpaque() }
    }

    pub fn get_color(&self, p: impl Into<IPoint>) -> Color {
        let p = p.into();
        self.assert_pixel_exists(p);
        Color::from_native_c(unsafe { self.native().getColor(p.x, p.y) })
    }

    pub fn get_color_4f(&self, p: impl Into<IPoint>) -> Color4f {
        let p = p.into();
        self.assert_pixel_exists(p);
        Color4f::from_native_c(unsafe { self.native().getColor4f(p.x, p.y) })
    }

    pub fn get_alpha_f(&self, p: impl Into<IPoint>) -> f32 {
        let p = p.into();
        self.assert_pixel_exists(p);
        unsafe { self.native().getAlphaf(p.x, p.y) }
    }

    // Helper to test if the pixel does exist physically in memory.
    fn assert_pixel_exists(&self, p: impl Into<IPoint>) {
        let p = p.into();
        assert!(!self.addr().is_null());
        assert!(p.x >= 0 && p.x < self.width());
        assert!(p.y >= 0 && p.y < self.height());
    }

    pub fn addr_at(&self, p: impl Into<IPoint>) -> *const c_void {
        let p = p.into();
        self.assert_pixel_exists(p);
        unsafe {
            (self.addr() as *const raw::c_char).add(self.info().compute_offset(p, self.row_bytes()))
                as _
        }
    }

    // TODO: addr8(), addr16(), addr32(), addr64(), addrF16(),
    //       addr8_at(), addr16_at(), addr32_at(), addr64_at(), addrF16_at()

    pub fn writable_addr(&self) -> *mut c_void {
        self.addr() as _
    }

    pub fn writable_addr_at(&self, p: impl Into<IPoint>) -> *mut c_void {
        self.addr_at(p) as _
    }

    // TODO: writable_addr8
    // TODO: writable_addr16
    // TODO: writable_addr32
    // TODO: writable_addr64
    // TODO: writable_addrF16

    pub fn read_pixels<P>(
        &self,
        dst_info: &ImageInfo,
        pixels: &mut [P],
        dst_row_bytes: usize,
        src: impl Into<IPoint>,
    ) -> bool {
        if !dst_info.valid_pixels(dst_row_bytes, pixels) {
            return false;
        }

        let src = src.into();

        unsafe {
            self.native().readPixels(
                dst_info.native(),
                pixels.as_mut_ptr() as _,
                dst_row_bytes,
                src.x,
                src.y,
            )
        }
    }

    /// Access the underlying pixels as a byte array. This is a rust-skia specific function.
    pub fn bytes(&self) -> Option<&'pixels [u8]> {
        let addr = self.addr().into_non_null()?;
        let len = self.compute_byte_size();
        Some(unsafe { slice::from_raw_parts(addr.as_ptr() as *const _, len) })
    }

    pub fn bytes_mut(&mut self) -> Option<&'pixels mut [u8]> {
        let addr = self.writable_addr().into_non_null()?;
        let len = self.compute_byte_size();
        Some(unsafe { slice::from_raw_parts_mut(addr.as_ptr() as *mut u8, len) })
    }

    /// Access the underlying pixels. This is a rust-skia specific function.
    ///
    /// The `Pixel` type must implement the _unsafe_ trait [`Pixel`] and must return `true` in
    /// [`Pixel::matches_color_type()`] when matched against the [`ColorType`] of this Pixmap's
    /// pixels.
    pub fn pixels<P: Pixel>(&self) -> Option<&'pixels [P]> {
        let addr = self.addr().into_non_null()?;

        let info = self.info();
        let ct = info.color_type();
        let pixel_size = mem::size_of::<P>();

        if info.bytes_per_pixel() == pixel_size && P::matches_color_type(ct) {
            let len = self.compute_byte_size() / pixel_size;
            return Some(unsafe { slice::from_raw_parts(addr.as_ptr() as *const _, len) });
        }

        None
    }

    pub fn read_pixels_to_pixmap(&self, dst: &mut Pixmap, src: impl Into<IPoint>) -> bool {
        let Some(dst_bytes) = dst.bytes_mut() else {
            return false;
        };
        self.read_pixels(dst.info(), dst_bytes, dst.row_bytes(), src)
    }

    pub fn scale_pixels(&self, dst: &mut Pixmap, sampling: impl Into<SamplingOptions>) -> bool {
        let sampling = sampling.into();
        unsafe { self.native().scalePixels(dst.native(), sampling.native()) }
    }

    pub fn erase(&mut self, color: impl Into<Color>, subset: Option<&IRect>) -> bool {
        let color = color.into().into_native();
        unsafe {
            match subset {
                Some(subset) => self.native().erase(color, subset.native()),
                None => self.native().erase(color, self.bounds().native()),
            }
        }
    }

    pub fn erase_4f(&mut self, color: impl AsRef<Color4f>, subset: Option<&IRect>) -> bool {
        let color = color.as_ref();
        unsafe {
            self.native()
                .erase1(color.native(), subset.native_ptr_or_null())
        }
    }

    fn from_native_c(pixmap: SkPixmap) -> Self {
        Self {
            inner: Handle::from_native_c(pixmap),
            pd: PhantomData,
        }
    }

    #[must_use]
    pub(crate) fn from_native_ref(n: &SkPixmap) -> &Self {
        unsafe { transmute_ref(n) }
    }

    #[must_use]
    pub(crate) fn from_native_ptr(np: *const SkPixmap) -> *const Self {
        // Should be safe as long `Pixmap` is represented with repr(Transparent).
        np as _
    }

    pub(crate) fn native_mut(&mut self) -> &mut SkPixmap {
        self.inner.native_mut()
    }

    pub(crate) fn native(&self) -> &SkPixmap {
        self.inner.native()
    }
}

/// Implement this trait to use a pixel type in [`Handle<Pixmap>::pixels()`].
///
/// # Safety
///
/// This trait is unsafe because external [`Pixel`] implementations may lie about their
/// [`ColorType`] or fail to match the alignment of the pixels stored in [`Handle<Pixmap>`].
pub unsafe trait Pixel: Copy {
    /// `true` if the type matches the color type's format.
    fn matches_color_type(ct: ColorType) -> bool;
}

unsafe impl Pixel for u8 {
    fn matches_color_type(ct: ColorType) -> bool {
        matches!(ct, ColorType::Alpha8 | ColorType::Gray8)
    }
}

unsafe impl Pixel for [u8; 2] {
    fn matches_color_type(ct: ColorType) -> bool {
        matches!(ct, ColorType::R8G8UNorm | ColorType::A16UNorm)
    }
}

unsafe impl Pixel for (u8, u8) {
    fn matches_color_type(ct: ColorType) -> bool {
        matches!(ct, ColorType::R8G8UNorm | ColorType::A16UNorm)
    }
}

unsafe impl Pixel for [u8; 4] {
    fn matches_color_type(ct: ColorType) -> bool {
        matches!(
            ct,
            ColorType::RGBA8888 | ColorType::RGB888x | ColorType::BGRA8888
        )
    }
}

unsafe impl Pixel for (u8, u8, u8, u8) {
    fn matches_color_type(ct: ColorType) -> bool {
        matches!(
            ct,
            ColorType::RGBA8888 | ColorType::RGB888x | ColorType::BGRA8888
        )
    }
}

unsafe impl Pixel for [f32; 4] {
    fn matches_color_type(ct: ColorType) -> bool {
        matches!(ct, ColorType::RGBAF32)
    }
}

unsafe impl Pixel for (f32, f32, f32, f32) {
    fn matches_color_type(ct: ColorType) -> bool {
        matches!(ct, ColorType::RGBAF32)
    }
}

unsafe impl Pixel for u32 {
    fn matches_color_type(ct: ColorType) -> bool {
        ct == ColorType::N32
    }
}

unsafe impl Pixel for Color {
    fn matches_color_type(ct: ColorType) -> bool {
        ct == ColorType::N32
    }
}

unsafe impl Pixel for Color4f {
    fn matches_color_type(ct: ColorType) -> bool {
        ct == ColorType::RGBAF32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixmap_mutably_borrows_pixels() {
        let mut pixels = [0u8; 2 * 2 * 4];
        let info = ImageInfo::new(
            (2, 2),
            ColorType::RGBA8888,
            AlphaType::Premul,
            ColorSpace::new_srgb(),
        );
        let mut pixmap = Pixmap::new(&info, &mut pixels, info.min_row_bytes()).unwrap();
        // this must fail to compile:
        // let _pixel = pixels[0];
        // use `.bytes()`, or `bytes_mut()` instead.
        pixmap.reset();
    }

    #[test]
    fn addr_may_return_null_from_a_default_pixmap() {
        let pixmap = Pixmap::default();
        assert!(pixmap.addr().is_null());
        assert!(pixmap.writable_addr().is_null());
    }
}
