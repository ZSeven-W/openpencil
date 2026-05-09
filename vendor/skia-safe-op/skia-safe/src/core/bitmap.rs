use std::{ffi, fmt, ptr};

use skia_bindings::{self as sb, SkBitmap};

use crate::{
    prelude::*, AlphaType, Color, Color4f, ColorSpace, ColorType, IPoint, IRect, ISize, Image,
    ImageInfo, Matrix, Paint, PixelRef, Pixmap, SamplingOptions, Shader, TileMode,
};

/// [`Bitmap`] describes a two-dimensional raster pixel array. [`Bitmap`] is built on [`ImageInfo`],
/// containing integer width and height, [`ColorType`] and [`AlphaType`] describing the pixel
/// format, and [`ColorSpace`] describing the range of colors. [`Bitmap`] points to [`PixelRef`],
/// which describes the physical array of pixels. [`ImageInfo`] bounds may be located anywhere fully
/// inside [PixelRef] bounds.
///
/// [`Bitmap`] can be drawn using [crate::Canvas]. [`Bitmap`] can be a drawing destination for
/// [`crate::Canvas`] draw member functions. [`Bitmap`] flexibility as a pixel container limits some
/// optimizations available to the target platform.
///
/// If pixel array is primarily read-only, use [`Image`] for better performance.
///
/// If pixel array is primarily written to, use [`crate::Surface`] for better performance.
///
/// Declaring [`Bitmap`] const prevents altering [`ImageInfo`]: the [`Bitmap`] height, width, and so
/// on cannot change. It does not affect [`PixelRef`]: a caller may write its pixels. Declaring
/// [`Bitmap`] const affects [`Bitmap`] configuration, not its contents.
///
/// [`Bitmap`] is not thread safe. Each thread must have its own copy of [`Bitmap`] fields, although
/// threads may share the underlying pixel array.
pub type Bitmap = Handle<SkBitmap>;

impl NativeDrop for SkBitmap {
    fn drop(&mut self) {
        unsafe { sb::C_SkBitmap_destruct(self) }
    }
}

impl NativeClone for SkBitmap {
    /// Copies settings from `self` to returned [`Bitmap`]. Shares pixels if `self` has pixels
    /// allocated, so both bitmaps reference the same pixels.
    fn clone(&self) -> Self {
        unsafe { SkBitmap::new1(self) }
    }
}

impl Default for Bitmap {
    /// See [`Bitmap::new()`].
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Bitmap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bitmap")
            .field("pixmap", self.pixmap())
            .finish()
    }
}

impl Bitmap {
    /// Creates an empty [`Bitmap`] without pixels, with [`ColorType::Unknown`], [`AlphaType::Unknown`],
    /// and with a width and height of zero. [`PixelRef`] origin is set to `(0, 0)`.
    ///
    /// Use [`Self::set_info()`] to associate [`ColorType`], [`AlphaType`], width, and height after
    /// [`Bitmap`] has been created.
    pub fn new() -> Self {
        Self::construct(|bitmap| unsafe { sb::C_SkBitmap_Construct(bitmap) })
    }

    /// Swaps the fields of the two bitmaps.
    pub fn swap(&mut self, other: &mut Self) {
        unsafe { self.native_mut().swap(other.native_mut()) }
    }

    /// Returns a constant reference to the [`Pixmap`] holding the [`Bitmap`] pixel address, row
    /// bytes, and [`ImageInfo`].
    pub fn pixmap(&self) -> &Pixmap {
        Pixmap::from_native_ref(&self.native().fPixmap)
    }

    /// Returns width, height, [`AlphaType`], [ColorType], and [`ColorSpace`].
    pub fn info(&self) -> &ImageInfo {
        self.pixmap().info()
    }

    /// Returns pixel count in each row. Should be equal or less than `row_bytes()` /
    /// `info().bytes_per_pixel()`.
    ///
    /// May be less than `pixel_ref().width()`. Will not exceed `pixel_ref().width()` less
    /// `pixel_ref_origin().x`.
    pub fn width(&self) -> i32 {
        self.pixmap().width()
    }

    /// Returns pixel row count.
    ///
    /// Maybe be less than `pixel_ref().height()`. Will not exceed `pixel_ref().height()` less
    /// `pixel_ref_origin().y`.
    pub fn height(&self) -> i32 {
        self.pixmap().height()
    }

    pub fn color_type(&self) -> ColorType {
        self.pixmap().color_type()
    }

    pub fn alpha_type(&self) -> AlphaType {
        self.pixmap().alpha_type()
    }

    /// Returns [`ColorSpace`], the range of colors, associated with [`ImageInfo`]. The returned
    /// [`ColorSpace`] is immutable.
    pub fn color_space(&self) -> Option<ColorSpace> {
        ColorSpace::from_unshared_ptr(unsafe { self.native().colorSpace() })
    }

    /// Returns number of bytes per pixel required by [`ColorType`].
    ///
    /// Returns zero if `color_type()` is [`ColorType::Unknown`].
    pub fn bytes_per_pixel(&self) -> usize {
        self.info().bytes_per_pixel()
    }

    /// Returns number of pixels that fit on row. Should be greater than or equal to `width()`.
    pub fn row_bytes_as_pixels(&self) -> usize {
        self.pixmap().row_bytes_as_pixels()
    }

    /// Returns bit shift converting row bytes to row pixels.
    ///
    /// Returns zero for [`ColorType::Unknown`].
    pub fn shift_per_pixel(&self) -> usize {
        self.pixmap().shift_per_pixel()
    }

    /// Returns `true` if either `width()` or `height()` are zero.
    ///
    /// Does not check if [`PixelRef`] is `None`; call `draws_nothing()` to check `width()`,
    /// `height()`, and [`PixelRef`].
    pub fn is_empty(&self) -> bool {
        self.info().is_empty()
    }

    /// Returns `true` if [`PixelRef`] is `None`.
    ///
    /// Does not check if `width()` or `height()` are zero; call `draws_nothing()` to check
    /// `width()`, `height()`, and [`PixelRef`].
    pub fn is_null(&self) -> bool {
        self.native().fPixelRef.fPtr.is_null()
    }

    /// Returns `true` if `width()` or `height()` are zero, or if [`PixelRef`] is `None`.
    ///
    /// If `true`, [`Bitmap`] has no effect when drawn or drawn into.
    pub fn draws_nothing(&self) -> bool {
        self.is_empty() || self.is_null()
    }

    /// Returns row bytes, the interval from one pixel row to the next. Row bytes is at least as
    /// large as: `width()` * `info().bytes_per_pixel()`.
    ///
    /// Returns zero if `color_type()` is [`ColorType::Unknown`], or if row bytes supplied to
    /// `set_info()` is not large enough to hold a row of pixels.
    pub fn row_bytes(&self) -> usize {
        self.pixmap().row_bytes()
    }

    /// Sets [`AlphaType`], if `alpha_type` is compatible with [`ColorType`]. Returns `true` unless
    /// `alpha_type` is [`AlphaType::Unknown`] and current [`AlphaType`] is not [`AlphaType::Unknown`].
    ///
    /// Returns `true` if [`ColorType`] is [`ColorType::Unknown`]. `alpha_type` is ignored, and
    /// [`AlphaType`] remains [`AlphaType::Unknown`].
    ///
    /// Returns `true` if [`ColorType`] is [`ColorType::RGB565`] or [`ColorType::Gray8`].
    /// `alpha_type` is ignored, and [`AlphaType`] remains [`AlphaType::Opaque`].
    ///
    /// If [`ColorType`] is [`ColorType::ARGB4444`], [`ColorType::RGBA8888`],
    /// [`ColorType::BGRA8888`], or [`ColorType::RGBAF16`]: returns `true` unless `alpha_type` is
    /// [`AlphaType::Unknown`] and [`AlphaType`] is not [`AlphaType::Unknown`]. If [`AlphaType`] is
    /// [`AlphaType::Unknown`], `alpha_type` is ignored.
    ///
    /// If [`ColorType`] is [`ColorType::Alpha8`], returns `true` unless `alpha_type` is
    /// [`AlphaType::Unknown`] and [`AlphaType`] is not [`AlphaType::Unknown`]. If [`AlphaType`] is
    /// kUnknown_SkAlphaType, `alpha_type` is ignored. If `alpha_type` is [`AlphaType::Unpremul`],
    /// it is treated as [`AlphaType::Premul`].
    ///
    /// This changes [`AlphaType`] in [`PixelRef`]; all bitmaps sharing [`PixelRef`] are affected.
    pub fn set_alpha_type(&mut self, alpha_type: AlphaType) -> bool {
        unsafe { self.native_mut().setAlphaType(alpha_type) }
    }

    /// Sets the [`ColorSpace`] associated with this [`Bitmap`].
    ///
    /// The raw pixel data is not altered by this call; no conversion is
    /// performed.
    ///
    /// This changes [`ColorSpace`] in [`PixelRef`]; all bitmaps sharing [`PixelRef`]
    /// are affected.
    pub fn set_color_space(&mut self, color_space: impl Into<Option<ColorSpace>>) {
        unsafe {
            sb::C_SkBitmap_setColorSpace(self.native_mut(), color_space.into().into_ptr_or_null())
        }
    }

    /// Returns pixel address, the base address corresponding to the pixel origin.
    pub fn pixels(&mut self) -> *mut ffi::c_void {
        self.pixmap().writable_addr()
    }

    /// Returns minimum memory required for pixel storage.  
    /// Does not include unused memory on last row when `row_bytes_as_pixels()` exceeds `width()`.  
    /// Returns [`usize::MAX`] if result does not fit in `usize`.  
    /// Returns zero if `height()` or `width()` is 0.  
    /// Returns `height()` times `row_bytes()` if `color_type()` is [`ColorType::Unknown`].
    pub fn compute_byte_size(&self) -> usize {
        self.pixmap().compute_byte_size()
    }

    /// Returns `true` if pixels can not change.
    ///
    /// Most immutable [`Bitmap`] checks trigger an assert only on debug builds.
    pub fn is_immutable(&self) -> bool {
        unsafe { self.native().isImmutable() }
    }

    /// Sets internal flag to mark [`Bitmap`] as immutable. Once set, pixels can not change. Any
    /// other bitmap sharing the same [`PixelRef`] are also marked as immutable.
    ///
    /// Once [`PixelRef`] is marked immutable, the setting cannot be cleared.
    ///
    /// Writing to immutable [`Bitmap`] pixels triggers an assert on debug builds.
    pub fn set_immutable(&mut self) {
        unsafe { self.native_mut().setImmutable() }
    }

    /// Returns `true` if [`AlphaType`] is set to hint that all pixels are opaque; their alpha value
    /// is implicitly or explicitly `1.0`. If `true`, and all pixels are not opaque, Skia may draw
    /// incorrectly.
    ///
    /// Does not check if [ColorType] allows alpha, or if any pixel value has transparency.
    pub fn is_opaque(&self) -> bool {
        self.pixmap().is_opaque()
    }

    /// Resets to its initial state; all fields are set to zero, as if [`Bitmap`] had
    /// been initialized by [`Bitmap::new()`].
    ///
    /// Sets width, height, row bytes to zero; pixel address to `None`; [`ColorType`] to
    /// [`ColorType::Unknown`]; and [`AlphaType`] to [`AlphaType::Unknown`].
    ///
    /// If [`PixelRef`] is allocated, its reference count is decreased by one, releasing its memory
    /// if [`Bitmap`] is the sole owner.
    pub fn reset(&mut self) {
        unsafe { self.native_mut().reset() }
    }

    /// Returns `true `if all pixels are opaque. [`ColorType`] determines how pixels are encoded, and
    /// whether pixel describes alpha. Returns `true` for [`ColorType`] without alpha in each pixel;
    /// for other [`ColorType`], returns `true` if all pixels have alpha values equivalent to 1.0 or
    /// greater.
    ///
    /// Returns `false` for [`ColorType::Unknown`].
    pub fn compute_is_opaque(bm: &Self) -> bool {
        unsafe { sb::C_SkBitmap_ComputeIsOpaque(bm.native()) }
    }

    /// Returns `IRect { 0, 0, width(), height() }`.
    pub fn bounds(&self) -> IRect {
        self.info().bounds()
    }

    /// Returns `ISize { width(), height() }`.
    pub fn dimensions(&self) -> ISize {
        self.info().dimensions()
    }

    /// Returns the bounds of this bitmap, offset by its [`PixelRef`] origin.
    pub fn get_subset(&self) -> IRect {
        let origin = self.pixel_ref_origin();
        IRect::from_xywh(origin.x, origin.y, self.width(), self.height())
    }

    /// Sets width, height, [`AlphaType`], [ColorType], [`ColorSpace`], and optional `row_bytes`.
    /// Frees pixels, and returns `true` if successful.
    ///
    /// `row_bytes` must equal or exceed `image_info.min_row_bytes()`. If `image_info.color_space()`
    /// is [`ColorType::Unknown`], `row_bytes` is ignored and treated as zero; for all other
    /// [`ColorSpace`] values, `row_bytes` of zero is treated as `image_info.min_row_bytes()`.
    ///
    /// Calls `reset()` and returns `false` if:
    /// - rowBytes exceeds 31 bits
    /// - `image_info.width()` is negative
    /// - `image_info.height()` is negative
    /// - `row_bytes` is positive and less than `image_info.width()` times
    ///   `image_info.bytes_per_pixel()`
    #[must_use]
    pub fn set_info(
        &mut self,
        image_info: &ImageInfo,
        row_bytes: impl Into<Option<usize>>,
    ) -> bool {
        unsafe {
            self.native_mut()
                .setInfo(image_info.native(), row_bytes.into().unwrap_or(0))
        }
    }

    /// Sets [`ImageInfo`] to info following the rules in `set_info()` and allocates pixel memory.
    /// Memory is zeroed.
    ///
    /// Returns `false` and calls `reset()` if [`ImageInfo`] could not be set, or memory could not
    /// be allocated, or memory could not optionally be zeroed.
    ///
    /// On most platforms, allocating pixel memory may succeed even though there is not sufficient
    /// memory to hold pixels; allocation does not take place until the pixels are written to. The
    /// actual behavior depends on the platform implementation of `calloc()`.
    #[must_use]
    pub fn try_alloc_pixels_flags(&mut self, image_info: &ImageInfo) -> bool {
        unsafe {
            self.native_mut().tryAllocPixelsFlags(
                image_info.native(),
                sb::SkBitmap_AllocFlags_kZeroPixels_AllocFlag as _,
            )
        }
    }

    /// Sets [`ImageInfo`] to info following the rules in `set_info()` and allocates pixel memory.
    /// Memory is zeroed.
    ///
    /// Returns `false` and calls `reset()` if [`ImageInfo`] could not be set, or memory could not be
    /// allocated, or memory could not optionally be zeroed.
    ///
    /// On most platforms, allocating pixel memory may succeed even though there is not sufficient
    /// memory to hold pixels; allocation does not take place until the pixels are written to. The
    /// actual behavior depends on the platform implementation of `calloc()`.
    pub fn alloc_pixels_flags(&mut self, image_info: &ImageInfo) {
        if !self.try_alloc_pixels_flags(image_info) {
            panic!("Bitmap::alloc_pixels_flags failed");
        }
    }

    /// Sets [`ImageInfo`] to info following the rules in `set_info()` and allocates pixel memory.
    /// `row_bytes` must equal or exceed `info.width()` times `info.bytes_per_pixel()`, or equal
    /// `None`. Pass in `None` for `row_bytes` to compute the minimum valid value.
    ///
    /// Returns `false` and calls `reset()` if [`ImageInfo`] could not be set, or memory could not be
    /// allocated.
    ///
    /// On most platforms, allocating pixel memory may succeed even though there is not sufficient
    /// memory to hold pixels; allocation does not take place until the pixels are written to. The
    /// actual behavior depends on the platform implementation of `malloc()`.
    #[must_use]
    pub fn try_alloc_pixels_info(
        &mut self,
        image_info: &ImageInfo,
        row_bytes: impl Into<Option<usize>>,
    ) -> bool {
        let row_bytes = row_bytes
            .into()
            .unwrap_or_else(|| image_info.min_row_bytes());
        unsafe {
            self.native_mut()
                .tryAllocPixels(image_info.native(), row_bytes)
        }
    }

    /// Sets [`ImageInfo`] to info following the rules in `set_info()` and allocates pixel memory.
    /// `row_bytes` must equal or exceed `info.width()` times `info.bytes_per_pixel()`, or equal
    /// `None`. Pass in `None` for `row_bytes` to compute the minimum valid value.
    ///
    /// Aborts execution if SkImageInfo could not be set, or memory could
    /// be allocated.
    ///
    /// On most platforms, allocating pixel memory may succeed even though there is not sufficient
    /// memory to hold pixels; allocation does not take place until the pixels are written to. The
    /// actual behavior depends on the platform implementation of `malloc()`.
    pub fn alloc_pixels_info(
        &mut self,
        image_info: &ImageInfo,
        row_bytes: impl Into<Option<usize>>,
    ) {
        if !self.try_alloc_pixels_info(image_info, row_bytes.into()) {
            panic!("Bitmap::alloc_pixels_info failed");
        }
    }

    /// Sets [`ImageInfo`] to width, height, and native color type; and allocates pixel memory. If
    /// `is_opaque` is `true`, sets [`ImageInfo`] to [`AlphaType::Opaque`]; otherwise, sets to
    /// [`AlphaType::Premul`].
    ///
    /// Calls `reset()` and returns `false` if width exceeds 29 bits or is negative, or height is
    /// negative.
    ///
    /// Returns `false` if allocation fails.
    ///
    /// Use to create [`Bitmap`] that matches [`crate::PMColor`], the native pixel arrangement on
    /// the platform. [`Bitmap`] drawn to output device skips converting its pixel format.
    #[must_use]
    pub fn try_alloc_n32_pixels(
        &mut self,
        (width, height): (i32, i32),
        is_opaque: impl Into<Option<bool>>,
    ) -> bool {
        unsafe {
            sb::C_SkBitmap_tryAllocN32Pixels(
                self.native_mut(),
                width,
                height,
                is_opaque.into().unwrap_or(false),
            )
        }
    }

    /// Sets [`ImageInfo`] to width, height, and native color type; and allocates pixel memory. If
    /// `is_opaque` is `true`, sets [`ImageInfo`] to [`AlphaType::Opaque`]; otherwise, sets to
    /// [`AlphaType::Premul`].
    ///
    /// Aborts if width exceeds 29 bits or is negative, or height is negative, or allocation fails.
    ///
    /// Use to create [`Bitmap`] that matches [`crate::PMColor`], the native pixel arrangement on
    /// the platform. [`Bitmap`] drawn to output device skips converting its pixel format.
    pub fn alloc_n32_pixels(
        &mut self,
        (width, height): (i32, i32),
        is_opaque: impl Into<Option<bool>>,
    ) {
        if !self.try_alloc_n32_pixels((width, height), is_opaque.into().unwrap_or(false)) {
            panic!("Bitmap::alloc_n32_pixels_failed");
        }
    }

    // TODO: wrap installPixels with releaseProc.

    /// Sets [`ImageInfo`] to info following the rules in `set_info()`, and creates [`PixelRef`]
    /// containing `pixels` and `row_bytes`.
    ///
    /// If [`ImageInfo`] could not be set, or `row_bytes` is less than `info.min_row_bytes(): calls
    /// `reset()`, and returns `false`.
    ///
    /// Otherwise, if pixels equals `ptr::null_mut()`: sets [`ImageInfo`], returns `true`.
    ///
    /// Caller must ensure that pixels are valid for the lifetime of [`Bitmap`] and [`PixelRef`].
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn install_pixels(
        &mut self,
        info: &ImageInfo,
        pixels: *mut ffi::c_void,
        row_bytes: usize,
    ) -> bool {
        self.native_mut()
            .installPixels(info.native(), pixels, row_bytes, None, ptr::null_mut())
    }

    // TODO: wrap installPixels with SkPixmap&

    // TODO: setPixels()?

    /// Allocates pixel memory with HeapAllocator, and replaces existing [`PixelRef`]. The
    /// allocation size is determined by [`ImageInfo`] width, height, and [`ColorType`].
    ///
    /// Returns `false` if `info().color_type()` is [`ColorType::Unknown`], or allocation fails.
    #[must_use]
    pub fn try_alloc_pixels(&mut self) -> bool {
        unsafe { sb::C_SkBitmap_tryAllocPixels(self.native_mut()) }
    }

    /// Allocates pixel memory with HeapAllocator, and replaces existing [`PixelRef`]. The
    /// allocation size is determined by [`ImageInfo`] width, height, and [`ColorType`].
    ///
    /// Aborts if `info().color_type()` is [`ColorType::Unknown`], or allocation fails.
    pub fn alloc_pixels(&mut self) {
        if !self.try_alloc_pixels() {
            panic!("Bitmap::alloc_pixels failed");
        }
    }

    // TODO: allocPixels(Allocator*)

    // TODO: find a way to return pixel ref without increasing the ref count here?

    /// Returns [`PixelRef`], which contains: pixel base address; its dimensions; and `row_bytes()`,
    /// the interval from one row to the next. Does not change [`PixelRef`] reference count.
    /// [`PixelRef`] may be shared by multiple bitmaps.
    ///
    /// If [`PixelRef`] has not been set, returns `None`.
    pub fn pixel_ref(&self) -> Option<PixelRef> {
        PixelRef::from_unshared_ptr(self.native().fPixelRef.fPtr)
    }

    /// Returns origin of pixels within [`PixelRef`]. [`Bitmap`] bounds is always contained
    /// by [`PixelRef`] bounds, which may be the same size or larger. Multiple [`Bitmap`]
    /// can share the same [`PixelRef`], where each [`Bitmap`] has different bounds.
    ///
    /// The returned origin added to [`Bitmap`] dimensions equals or is smaller than the
    /// [`PixelRef`] dimensions.
    ///
    /// Returns `(0, 0)` if [`PixelRef`] is `None`.
    pub fn pixel_ref_origin(&self) -> IPoint {
        IPoint::from_native_c(unsafe { sb::C_SkBitmap_pixelRefOrigin(self.native()) })
    }

    /// Replaces `pixel_ref` and origin in [`Bitmap`]. `offset` specifies the offset within the
    /// [`PixelRef`] pixels for the top-left corner of the bitmap.
    ///
    /// Asserts in debug builds if offset is out of range. Pins offset to legal range in release
    /// builds.
    ///
    /// The caller is responsible for ensuring that the pixels match the [`ColorType`] and
    /// [`AlphaType`] in [`ImageInfo`].
    pub fn set_pixel_ref(
        &mut self,
        pixel_ref: impl Into<Option<PixelRef>>,
        offset: impl Into<IPoint>,
    ) {
        let offset = offset.into();
        unsafe {
            sb::C_SkBitmap_setPixelRef(
                self.native_mut(),
                pixel_ref.into().into_ptr_or_null(),
                offset.x,
                offset.y,
            )
        }
    }

    /// Returns `true` if [`Bitmap`] can be drawn.
    pub fn is_ready_to_draw(&self) -> bool {
        unsafe { sb::C_SkBitmap_readyToDraw(self.native()) }
    }

    /// Returns a unique value corresponding to the pixels in [`PixelRef`].  
    /// Returns a different value after `notify_pixels_changed()` has been called.  
    /// Returns zero if [`PixelRef`] is `None`.
    ///
    /// Determines if pixels have changed since last examined.
    pub fn generation_id(&self) -> u32 {
        unsafe { self.native().getGenerationID() }
    }

    /// Marks that pixels in [`PixelRef`] have changed. Subsequent calls to `generation_id()` return
    /// a different value.
    pub fn notify_pixels_changed(&self) {
        unsafe { self.native().notifyPixelsChanged() }
    }

    /// Replaces pixel values with `c`, interpreted as being in the sRGB [`ColorSpace`]. All pixels
    /// contained by [`Self::bounds()`] are affected. If the [`Self::color_type()`] is
    /// [`ColorType::Gray8`] or [`ColorType::RGB565`], then alpha is ignored; RGB is treated as
    /// opaque. If [`Self::color_type()`] is [`ColorType::Alpha8`], then RGB is ignored.
    ///
    /// Input color is ultimately converted to an [`Color4f`], so [`Self::erase_color_4f`] will have
    /// higher color resolution.
    pub fn erase_color(&self, c: impl Into<Color>) {
        unsafe { self.native().eraseColor1(c.into().into_native()) }
    }

    /// Replaces pixel values with `c`, interpreted as being in the sRGB [`ColorSpace`]. All pixels
    /// contained by [`Self::bounds()`] are affected. If the [`Self::color_type()`] is
    /// [`ColorType::Gray8`] or [ColorType::RGB565], then alpha is ignored; RGB is treated as
    /// opaque. If [`Self::color_type()`] is [`ColorType::Alpha8`], then RGB is ignored.
    pub fn erase_color_4f(&self, c: impl AsRef<Color4f>) {
        unsafe { self.native().eraseColor(c.as_ref().into_native()) }
    }

    /// Replaces pixel values with unpremultiplied color built from `a`, `r`, `g`, and `b`,
    /// interpreted as being in the sRGB [`ColorSpace`]. All pixels contained by [`Self::bounds()`]
    /// are affected. If the [`Self::color_type()`] is [`ColorType::Gray8`] or [`ColorType::RGB565`],
    /// then `a` is ignored; `r`, `g`, and `b` are treated as opaque. If [`Self::color_type()`] is
    /// [`ColorType::Alpha8`], then `r`, `g`, and `b` are ignored.
    pub fn erase_argb(&self, a: u8, r: u8, g: u8, b: u8) {
        unsafe { sb::C_SkBitmap_eraseARGB(self.native(), a.into(), r.into(), g.into(), b.into()) }
    }

    /// Replaces pixel values inside area with c. interpreted as being in the sRGB [`ColorSpace`].
    /// If area does not intersect `bounds()`, call has no effect.
    ///
    /// If the `color_type()` is [`ColorType::Gray8`] [`ColorType::RGB565`], then alpha is ignored;
    /// RGB is treated as opaque. If `color_type()` is [`ColorType::Alpha8`], then RGB is ignored.
    ///
    /// Input color is ultimately converted to an [`Color4f`], so [`Self::erase_4f`] will have
    /// higher color resolution.
    pub fn erase(&self, c: impl Into<Color>, area: impl AsRef<IRect>) {
        unsafe {
            self.native()
                .erase1(c.into().into_native(), area.as_ref().native())
        }
    }

    /// Replaces pixel values inside area with c. interpreted as being in the sRGB [`ColorSpace`].
    /// If area does not intersect `bounds()`, call has no effect.
    ///
    /// If the `color_type()` is [`ColorType::Gray8`] [`ColorType::RGB565`], then alpha is ignored;
    /// RGB is treated as opaque. If `color_type()` is [`ColorType::Alpha8`], then RGB is ignored.
    pub fn erase_4f(&self, c: impl AsRef<Color4f>, area: impl AsRef<IRect>) {
        unsafe {
            self.native()
                .erase(c.as_ref().into_native(), area.as_ref().native())
        }
    }

    /// Returns pixel at `(x, y)` as unpremultiplied color.  
    /// Returns black with alpha if [`ColorType`] is [`ColorType::Alpha8`]
    ///
    /// Input is not validated: out of bounds values of `x` or `y` trigger an `assert()`.
    ///
    /// Fails if [`ColorType`] is [`ColorType::Unknown`] or pixel address is `nullptr`.
    ///
    /// [`ColorSpace`] in [`ImageInfo`] is ignored. Some color precision may be lost in the
    /// conversion to unpremultiplied color; original pixel data may have additional precision.
    pub fn get_color(&self, p: impl Into<IPoint>) -> Color {
        self.pixmap().get_color(p)
    }

    /// Returns pixel at `(x, y)` as unpremultiplied color.
    /// Returns black with alpha if [ColorType] is [ColorType::Alpha8]
    ///
    /// Input is not validated: out of bounds values of x or y trigger an `assert()`.
    ///
    /// Fails if [ColorType] is [ColorType::Unknown] or pixel address is `None`.
    ///
    /// [`ColorSpace`] in [`ImageInfo`] is ignored. Some color precision may be lost in the
    /// conversion to unpremultiplied color.
    pub fn get_color_4f(&self, p: impl Into<IPoint>) -> Color4f {
        self.pixmap().get_color_4f(p)
    }

    /// Look up the pixel at `(x,y)` and return its alpha component, normalized to `[0..1]`. This is
    /// roughly equivalent to `get_color().a()`, but can be more efficient (and more precise if the
    /// pixels store more than 8 bits per component).
    pub fn get_alpha_f(&self, p: impl Into<IPoint>) -> f32 {
        self.pixmap().get_alpha_f(p)
    }

    /// Returns pixel address at `(x, y)`.
    ///
    /// Input is not validated: out of bounds values of `x` or `y`, or [`ColorType::Unknown`],
    /// trigger an `assert()`. Returns `nullptr` if [`ColorType`] is [`ColorType::Unknown`], or
    /// [`PixelRef`] is `nullptr`.
    ///
    /// Performs a lookup of pixel size; for better performance, call one of: `get_addr8()`,
    /// `get_addr16()`, or `get_addr32()`.
    pub fn get_addr(&self, p: impl Into<IPoint>) -> *const ffi::c_void {
        let p = p.into();
        unsafe { self.native().getAddr(p.x, p.y) }
    }

    // TODO: get_addr_32(), get_addr_16(), get_addr_8()

    /// Shares [`PixelRef`] with `dst`. Pixels are not copied; [`Bitmap`] and dst point to the same
    /// pixels; dst [`Self::bounds()`] are set to the intersection of subset and the original
    /// [`Self::bounds()`].
    ///
    /// Subset may be larger than [`Self::bounds()`]. Any area outside of [`Self::bounds()`] is
    /// ignored.
    ///
    /// Any contents of dst are discarded.
    ///
    /// Return `false` if:
    /// - dst is `nullptr`
    /// - [`PixelRef`] is `nullptr`
    /// - subset does not intersect [`Self::bounds()`]
    ///
    /// example: <https://fiddle.skia.org/c/@Bitmap_extractSubset>
    pub fn extract_subset(&self, dst: &mut Self, subset: impl AsRef<IRect>) -> bool {
        unsafe {
            self.native()
                .extractSubset(dst.native_mut(), subset.as_ref().native())
        }
    }

    /// Copies a [`crate::Rect`] of pixels from [`Bitmap`] to `dst_pixels`. Copy starts at `(src_x,
    /// src_y)`, and does not exceed [`Bitmap`] `(width(), height())`.
    ///
    /// `dst_info` specifies width, height, [ColorType], [`AlphaType`], and [`ColorSpace`] of
    /// destination.  
    /// `dst_row_bytes` specifics the gap from one destination row to the next. Returns `true` if
    /// pixels are copied. Returns `false` if:
    /// - `dst_info` has no address
    /// - `dst_row_bytes` is less than `dst_info.min_row_bytes()`
    /// - [`PixelRef`] is `nullptr`
    ///
    /// Pixels are copied only if pixel conversion is possible. If [`Self::color_type()`] is
    /// [`ColorType::Gray8`], or [`ColorType::Alpha8`]; `dst_info.color_type()` must match. If
    /// [`Self::color_type()`] is [`ColorType::Gray8`], `dst_info.color_space()` must match. If
    /// [`Self::alpha_type()`] is [`AlphaType::Opaque`], `dst_info.alpha_type()` must match. If
    /// [`Self::color_space()`] is `nullptr`, `dst_info.color_space()` must match. Returns `false`
    /// if pixel conversion is not possible.
    ///
    /// `src_x` and `src_y` may be negative to copy only top or left of source. Returns `false` if
    /// [`Self::width()`] or [`Self::height()`] is zero or negative. Returns `false` if `abs(src_x)`
    /// `>=` [`Self::width()`], or if `abs(src_y) >=` [`Self::height()`].
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_pixels(
        &self,
        dst_info: &ImageInfo,
        dst_pixels: *mut ffi::c_void,
        dst_row_bytes: usize,
        src_x: i32,
        src_y: i32,
    ) -> bool {
        self.native()
            .readPixels(dst_info.native(), dst_pixels, dst_row_bytes, src_x, src_y)
    }

    // TODO: read_pixels(Pixmap)
    // TODO: write_pixels(Pixmap)

    /// Sets dst to alpha described by pixels. Returns `false` if `dst` cannot be written to or
    /// `dst` pixels cannot be allocated.
    ///
    /// If `paint` is not `None` and contains [`crate::MaskFilter`], [`crate::MaskFilter`] generates
    /// mask alpha from [`Bitmap`]. Uses `HeapAllocator` to reserve memory for `dst` [`PixelRef`].
    /// Returns offset to top-left position for `dst` for alignment with [`Bitmap`]; `(0, 0)` unless
    /// [crate::MaskFilter] generates mask.
    pub fn extract_alpha(&self, dst: &mut Self, paint: Option<&Paint>) -> Option<IPoint> {
        let mut offset = IPoint::default();
        unsafe {
            sb::C_SkBitmap_extractAlpha(
                self.native(),
                dst.native_mut(),
                paint.native_ptr_or_null(),
                offset.native_mut(),
            )
        }
        .then_some(offset)
    }

    /// Copies [`Bitmap`] pixel address, row bytes, and [`ImageInfo`] to pixmap, if address is
    /// available, and returns `Some(Pixmap)`. If pixel address is not available, return `None`
    /// and leave pixmap unchanged.
    ///
    /// example: <https://fiddle.skia.org/c/@Bitmap_peekPixels>
    pub fn peek_pixels(&self) -> Option<Pixmap> {
        let mut pixmap = Pixmap::default();
        unsafe { self.native().peekPixels(pixmap.native_mut()) }.then_some(pixmap)
    }

    /// Make a shader with the specified tiling, matrix and sampling.  
    /// Defaults to clamp in both X and Y.
    pub fn to_shader<'a>(
        &self,
        tile_modes: impl Into<Option<(TileMode, TileMode)>>,
        sampling: impl Into<SamplingOptions>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        let tile_modes = tile_modes.into();
        let sampling = sampling.into();
        let local_matrix = local_matrix.into();
        Shader::from_ptr(unsafe {
            let tmx = tile_modes.map(|tm| tm.0).unwrap_or_default();
            let tmy = tile_modes.map(|tm| tm.1).unwrap_or_default();
            sb::C_SkBitmap_makeShader(
                self.native(),
                tmx,
                tmy,
                sampling.native(),
                local_matrix.native_ptr_or_null(),
            )
        })
    }

    /// Returns a new image from the bitmap. If the bitmap is marked immutable, this will
    /// share the pixel buffer. If not, it will make a copy of the pixels for the image.
    pub fn as_image(&self) -> Image {
        Image::from_ptr(unsafe { sb::C_SkBitmap_asImage(self.native()) }).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::TileMode;
    use crate::{AlphaType, Bitmap, Canvas, ColorSpace, ColorType, ImageInfo, SamplingOptions};

    #[test]
    fn create_clone_and_drop() {
        let bm = Bitmap::new();
        #[allow(clippy::redundant_clone)]
        let _bm2 = bm.clone();
    }

    #[test]
    fn get_info() {
        let bm = Bitmap::new();
        let _info = bm.info();
    }

    #[test]
    fn empty_bitmap_shader() {
        let bm = Bitmap::new();
        let _shader = bm.to_shader(None, SamplingOptions::default(), None);
    }

    #[test]
    fn shader_with_tile_mode() {
        let bm = Bitmap::new();
        let _shader = bm.to_shader(
            (TileMode::Decal, TileMode::Mirror),
            SamplingOptions::default(),
            None,
        );
    }

    #[test]
    fn test_get_subset() {
        let bm = Bitmap::new();
        let _ = bm.get_subset();
    }

    #[test]
    fn test_pixel_ref_origin() {
        let bm = Bitmap::new();
        let _ = bm.pixel_ref_origin();
    }

    /// Test for: <https://github.com/rust-skia/rust-skia/issues/669>
    #[test]
    fn cant_get_a_canvas_for_a_non_drawable_bitmap() {
        let info = ImageInfo::new(
            (400, 400),
            ColorType::BGRA8888,
            AlphaType::Premul,
            ColorSpace::new_srgb(),
        );
        let mut bitmap = Bitmap::new();
        if !bitmap.set_info(&info, None) {
            panic!("set_info failed");
        }

        let canvas = Canvas::from_bitmap(&bitmap, None);
        assert!(canvas.is_none());
    }
}
