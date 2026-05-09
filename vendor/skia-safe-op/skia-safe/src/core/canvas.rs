use std::{cell::UnsafeCell, ffi::CString, fmt, marker::PhantomData, mem, ops::Deref, ptr, slice};

use sb::SkCanvas_FilterSpan;
use skia_bindings::{
    self as sb, SkAutoCanvasRestore, SkCanvas, SkCanvas_SaveLayerRec, SkColorSpace, SkImageFilter,
    SkPaint, SkRect, U8CPU,
};

#[cfg(feature = "gpu")]
use crate::gpu;
use crate::{
    prelude::*, scalar, Bitmap, BlendMode, ClipOp, Color, Color4f, Data, Drawable, FilterMode,
    Font, GlyphId, IPoint, IRect, ISize, Image, ImageFilter, ImageInfo, Matrix, Paint, Path,
    Picture, Pixmap, Point, QuickReject, RRect, RSXform, Rect, Region, SamplingOptions, Shader,
    Surface, SurfaceProps, TextBlob, TextEncoding, TileMode, Vector, Vertices, M44,
};
use crate::{Arc, ColorSpace};

pub use lattice::Lattice;

bitflags! {
    /// [`SaveLayerFlags`] provides options that may be used in any combination in [`SaveLayerRec`],
    /// defining how layer allocated by [`Canvas::save_layer()`] operates. It may be set to zero,
    /// [`PRESERVE_LCD_TEXT`], [`INIT_WITH_PREVIOUS`], or both flags.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SaveLayerFlags: u32 {
        const PRESERVE_LCD_TEXT = sb::SkCanvas_SaveLayerFlagsSet_kPreserveLCDText_SaveLayerFlag as _;
        /// initializes with previous contents
        const INIT_WITH_PREVIOUS = sb::SkCanvas_SaveLayerFlagsSet_kInitWithPrevious_SaveLayerFlag as _;
        const F16_COLOR_TYPE = sb::SkCanvas_SaveLayerFlagsSet_kF16ColorType as _;
    }
}

/// [`SaveLayerRec`] contains the state used to create the layer.
#[repr(C)]
pub struct SaveLayerRec<'a> {
    // We _must_ store _references_ to the native types here, because not all of them are native
    // transmutable, like ImageFilter or Image, which are represented as ref counted pointers and so
    // we would store a reference to a pointer only.
    bounds: Option<&'a SkRect>,
    paint: Option<&'a SkPaint>,
    filters: SkCanvas_FilterSpan,
    backdrop: Option<&'a SkImageFilter>,
    backdrop_tile_mode: sb::SkTileMode,
    color_space: Option<&'a SkColorSpace>,
    flags: SaveLayerFlags,
    experimental_backdrop_scale: scalar,
}

native_transmutable!(SkCanvas_SaveLayerRec, SaveLayerRec<'_>);

impl Default for SaveLayerRec<'_> {
    /// Sets [`Self::bounds`], [`Self::paint`], and [`Self::backdrop`] to `None`. Clears
    /// [`Self::flags`].
    ///
    /// Returns empty [`SaveLayerRec`]
    fn default() -> Self {
        SaveLayerRec::construct(|slr| unsafe { sb::C_SkCanvas_SaveLayerRec_Construct(slr) })
    }
}

impl Drop for SaveLayerRec<'_> {
    fn drop(&mut self) {
        unsafe { sb::C_SkCanvas_SaveLayerRec_destruct(self.native_mut()) }
    }
}

impl fmt::Debug for SaveLayerRec<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SaveLayerRec")
            .field("bounds", &self.bounds.map(Rect::from_native_ref))
            .field("paint", &self.paint.map(Paint::from_native_ref))
            .field(
                "backdrop",
                &ImageFilter::from_unshared_ptr_ref(&(self.backdrop.as_ptr_or_null() as *mut _)),
            )
            .field("backdrop_tile_mode", &self.backdrop_tile_mode)
            .field(
                "color_space",
                &ColorSpace::from_unshared_ptr_ref(&(self.color_space.as_ptr_or_null() as *mut _)),
            )
            .field("flags", &self.flags)
            .field(
                "experimental_backdrop_scale",
                &self.experimental_backdrop_scale,
            )
            .finish()
    }
}

impl<'a> SaveLayerRec<'a> {
    /// Hints at layer size limit
    #[must_use]
    pub fn bounds(mut self, bounds: &'a Rect) -> Self {
        self.bounds = Some(bounds.native());
        self
    }

    /// Modifies overlay
    #[must_use]
    pub fn paint(mut self, paint: &'a Paint) -> Self {
        self.paint = Some(paint.native());
        self
    }

    /// If not `None`, this triggers the same initialization behavior as setting
    /// [`SaveLayerFlags::INIT_WITH_PREVIOUS`] on [`Self::flags`]: the current layer is copied into
    /// the new layer, rather than initializing the new layer with transparent-black. This is then
    /// filtered by [`Self::backdrop`] (respecting the current clip).
    #[must_use]
    pub fn backdrop(mut self, backdrop: &'a ImageFilter) -> Self {
        self.backdrop = Some(backdrop.native());
        self
    }

    /// If the layer is initialized with prior content (and/or with a backdrop filter) and this
    /// would require sampling outside of the available backdrop, this is the tilemode applied
    /// to the boundary of the prior layer's image.
    #[must_use]
    pub fn backdrop_tile_mode(mut self, backdrop_tile_mode: TileMode) -> Self {
        self.backdrop_tile_mode = backdrop_tile_mode;
        self
    }

    /// If not `None`, this triggers a color space conversion when the layer is restored. It
    /// will be as if the layer's contents are drawn in this color space. Filters from
    /// `backdrop` and `paint` will be applied in this color space.
    pub fn color_space(mut self, color_space: &'a ColorSpace) -> Self {
        self.color_space = Some(color_space.native());
        self
    }

    /// Preserves LCD text, creates with prior layer contents
    #[must_use]
    pub fn flags(mut self, flags: SaveLayerFlags) -> Self {
        self.flags = flags;
        self
    }
}

/// Selects if an array of points are drawn as discrete points, as lines, or as an open polygon.
pub use sb::SkCanvas_PointMode as PointMode;
variant_name!(PointMode::Polygon);

/// [`SrcRectConstraint`] controls the behavior at the edge of source [`Rect`], provided to
/// [`Canvas::draw_image_rect()`] when there is any filtering. If kStrict is set, then extra code is
/// used to ensure it nevers samples outside of the src-rect.
///
/// [`SrcRectConstraint::Strict`] disables the use of mipmaps and anisotropic filtering.
pub use sb::SkCanvas_SrcRectConstraint as SrcRectConstraint;
variant_name!(SrcRectConstraint::Fast);

/// Provides access to Canvas's pixels.
///
/// Returned by [`Canvas::access_top_layer_pixels()`]
#[derive(Debug)]
pub struct TopLayerPixels<'a> {
    /// Address of pixels
    pub pixels: &'a mut [u8],
    /// Writable pixels' [`ImageInfo`]
    pub info: ImageInfo,
    /// Writable pixels' row bytes
    pub row_bytes: usize,
    /// [`Canvas`] top layer origin, its top-left corner
    pub origin: IPoint,
}

/// Used to pass either a slice of [`Point`] or [`RSXform`] to [`Canvas::draw_glyphs_at`].
#[derive(Clone, Debug)]
pub enum GlyphPositions<'a> {
    Points(&'a [Point]),
    RSXforms(&'a [RSXform]),
}

impl<'a> From<&'a [Point]> for GlyphPositions<'a> {
    fn from(points: &'a [Point]) -> Self {
        Self::Points(points)
    }
}

impl<'a> From<&'a [RSXform]> for GlyphPositions<'a> {
    fn from(rs_xforms: &'a [RSXform]) -> Self {
        Self::RSXforms(rs_xforms)
    }
}

///  [`Canvas`] provides an interface for drawing, and how the drawing is clipped and transformed.
///  [`Canvas`] contains a stack of [`Matrix`] and clip values.
///
///  [`Canvas`] and [`Paint`] together provide the state to draw into [`Surface`] or `Device`.
///  Each [`Canvas`] draw call transforms the geometry of the object by the concatenation of all
///  [`Matrix`] values in the stack. The transformed geometry is clipped by the intersection
///  of all of clip values in the stack. The [`Canvas`] draw calls use [`Paint`] to supply drawing
///  state such as color, [`crate::Typeface`], text size, stroke width, [`Shader`] and so on.
///
///  To draw to a pixel-based destination, create raster surface or GPU surface.
///  Request [`Canvas`] from [`Surface`] to obtain the interface to draw.
///  [`Canvas`] generated by raster surface draws to memory visible to the CPU.
///  [`Canvas`] generated by GPU surface uses Vulkan or OpenGL to draw to the GPU.
///
///  To draw to a document, obtain [`Canvas`] from SVG canvas, document PDF, or
///  [`crate::PictureRecorder`]. [`crate::Document`] based [`Canvas`] and other [`Canvas`]
///  subclasses reference Device describing the destination.
///
///  [`Canvas`] can be constructed to draw to [`Bitmap`] without first creating raster surface.
///  This approach may be deprecated in the future.
#[repr(transparent)]
pub struct Canvas(UnsafeCell<SkCanvas>);

impl Canvas {
    pub(self) fn native(&self) -> &SkCanvas {
        unsafe { &*self.0.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn native_mut(&self) -> &mut SkCanvas {
        unsafe { &mut (*self.0.get()) }
    }
}

impl fmt::Debug for Canvas {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Canvas")
            .field("image_info", &self.image_info())
            .field("props", &self.props())
            .field("base_layer_size", &self.base_layer_size())
            .field("save_count", &self.save_count())
            .field("local_clip_bounds", &self.local_clip_bounds())
            .field("device_clip_bounds", &self.device_clip_bounds())
            .field("local_to_device", &self.local_to_device())
            .finish()
    }
}

/// Represents a [`Canvas`] that is owned and dropped when it goes out of scope _and_ is bound to
/// the lifetime of some other value (an array of pixels for example).
///
/// Access to the [`Canvas`] functions are resolved with the [`Deref`] trait.
#[repr(transparent)]
pub struct OwnedCanvas<'lt>(ptr::NonNull<Canvas>, PhantomData<&'lt ()>);

impl Deref for OwnedCanvas<'_> {
    type Target = Canvas;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl Drop for OwnedCanvas<'_> {
    /// Draws saved layers, if any.
    /// Frees up resources used by [`Canvas`].
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_destructor>
    fn drop(&mut self) {
        unsafe { sb::C_SkCanvas_delete(self.native()) }
    }
}

impl Default for OwnedCanvas<'_> {
    /// Creates an empty [`Canvas`] with no backing device or pixels, with
    /// a width and height of zero.
    ///
    /// Returns empty [`Canvas`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_empty_constructor>
    fn default() -> Self {
        let ptr = unsafe { sb::C_SkCanvas_newEmpty() };
        Canvas::own_from_native_ptr(ptr).unwrap()
    }
}

impl fmt::Debug for OwnedCanvas<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OwnedCanvas").field(self as &Canvas).finish()
    }
}

impl Canvas {
    /// Allocates raster [`Canvas`] that will draw directly into pixels.
    ///
    /// [`Canvas`] is returned if all parameters are valid.
    /// Valid parameters include:
    /// - `info` dimensions are zero or positive
    /// - `info` contains [`crate::ColorType`] and [`crate::AlphaType`] supported by raster surface
    /// - `row_bytes` is `None` or large enough to contain info width pixels of [`crate::ColorType`]
    ///
    /// Pass `None` for `row_bytes` to compute `row_bytes` from info width and size of pixel.
    /// If `row_bytes` is not `None`, it must be equal to or greater than `info` width times
    /// bytes required for [`crate::ColorType`].
    ///
    /// Pixel buffer size should be info height times computed `row_bytes`.
    /// Pixels are not initialized.
    /// To access pixels after drawing, call `flush()` or [`Self::peek_pixels()`].
    ///
    /// - `info` width, height, [`crate::ColorType`], [`crate::AlphaType`], [`crate::ColorSpace`],
    ///   of raster surface; width, or height, or both, may be zero
    /// - `pixels` pointer to destination pixels buffer
    /// - `row_bytes` interval from one [`Surface`] row to the next, or zero
    /// - `props` LCD striping orientation and setting for device independent fonts;
    ///   may be `None`
    ///
    /// Returns [`OwnedCanvas`] if all parameters are valid; otherwise, `None`.
    pub fn from_raster_direct<'pixels>(
        info: &ImageInfo,
        pixels: &'pixels mut [u8],
        row_bytes: impl Into<Option<usize>>,
        props: Option<&SurfaceProps>,
    ) -> Option<OwnedCanvas<'pixels>> {
        let row_bytes = row_bytes.into().unwrap_or_else(|| info.min_row_bytes());
        if info.valid_pixels(row_bytes, pixels) {
            let ptr = unsafe {
                sb::C_SkCanvas_MakeRasterDirect(
                    info.native(),
                    pixels.as_mut_ptr() as _,
                    row_bytes,
                    props.native_ptr_or_null(),
                )
            };
            Self::own_from_native_ptr(ptr)
        } else {
            None
        }
    }

    /// Allocates raster [`Canvas`] specified by inline image specification. Subsequent [`Canvas`]
    /// calls draw into pixels.
    /// [`crate::ColorType`] is set to [`crate::ColorType::n32()`].
    /// [`crate::AlphaType`] is set to [`crate::AlphaType::Premul`].
    /// To access pixels after drawing, call `flush()` or [`Self::peek_pixels()`].
    ///
    /// [`OwnedCanvas`] is returned if all parameters are valid.
    /// Valid parameters include:
    /// - width and height are zero or positive
    /// - `row_bytes` is zero or large enough to contain width pixels of [`crate::ColorType::n32()`]
    ///
    /// Pass `None` for `row_bytes` to compute `row_bytes` from width and size of pixel.
    /// If `row_bytes` is greater than zero, it must be equal to or greater than width times bytes
    /// required for [`crate::ColorType`].
    ///
    /// Pixel buffer size should be height times `row_bytes`.
    ///
    /// - `size` pixel column and row count on raster surface created; must both be zero or greater
    /// - `pixels` pointer to destination pixels buffer; buffer size should be height times
    ///   `row_bytes`
    /// - `row_bytes` interval from one [`Surface`] row to the next, or zero
    ///
    /// Returns [`OwnedCanvas`] if all parameters are valid; otherwise, `None`
    pub fn from_raster_direct_n32<'pixels>(
        size: impl Into<ISize>,
        pixels: &'pixels mut [u32],
        row_bytes: impl Into<Option<usize>>,
    ) -> Option<OwnedCanvas<'pixels>> {
        let info = ImageInfo::new_n32_premul(size, None);
        let pixels_ptr: *mut u8 = pixels.as_mut_ptr() as _;
        let pixels_u8: &'pixels mut [u8] =
            unsafe { slice::from_raw_parts_mut(pixels_ptr, mem::size_of_val(pixels)) };
        Self::from_raster_direct(&info, pixels_u8, row_bytes, None)
    }

    /// Creates [`Canvas`] of the specified dimensions without a [`Surface`].
    /// Used by subclasses with custom implementations for draw member functions.
    ///
    /// If props equals `None`, [`SurfaceProps`] are created with `SurfaceProps::InitType` settings,
    /// which choose the pixel striping direction and order. Since a platform may dynamically change
    /// its direction when the device is rotated, and since a platform may have multiple monitors
    /// with different characteristics, it is best not to rely on this legacy behavior.
    ///
    /// - `size` with and height zero or greater
    /// - `props` LCD striping orientation and setting for device independent fonts;
    ///   may be `None`
    ///
    /// Returns [`Canvas`] placeholder with dimensions
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_int_int_const_SkSurfaceProps_star>
    #[allow(clippy::new_ret_no_self)]
    pub fn new<'lt>(
        size: impl Into<ISize>,
        props: Option<&SurfaceProps>,
    ) -> Option<OwnedCanvas<'lt>> {
        let size = size.into();
        if size.width >= 0 && size.height >= 0 {
            let ptr = unsafe {
                sb::C_SkCanvas_newWidthHeightAndProps(
                    size.width,
                    size.height,
                    props.native_ptr_or_null(),
                )
            };
            Canvas::own_from_native_ptr(ptr)
        } else {
            None
        }
    }

    /// Constructs a canvas that draws into bitmap.
    /// Use props to match the device characteristics, like LCD striping.
    ///
    /// bitmap is copied so that subsequently editing bitmap will not affect constructed [`Canvas`].
    ///
    /// - `bitmap` width, height, [`crate::ColorType`], [`crate::AlphaType`], and pixel storage of
    ///   raster surface
    /// - `props` order and orientation of RGB striping; and whether to use device independent fonts
    ///
    /// Returns [`Canvas`] that can be used to draw into bitmap
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_const_SkBitmap_const_SkSurfaceProps>
    pub fn from_bitmap<'lt>(
        bitmap: &Bitmap,
        props: Option<&SurfaceProps>,
    ) -> Option<OwnedCanvas<'lt>> {
        // <https://github.com/rust-skia/rust-skia/issues/669>
        if !bitmap.is_ready_to_draw() {
            return None;
        }
        let props_ptr = props.native_ptr_or_null();
        let ptr = if props_ptr.is_null() {
            unsafe { sb::C_SkCanvas_newFromBitmap(bitmap.native()) }
        } else {
            unsafe { sb::C_SkCanvas_newFromBitmapAndProps(bitmap.native(), props_ptr) }
        };
        Canvas::own_from_native_ptr(ptr)
    }

    /// Returns [`ImageInfo`] for [`Canvas`]. If [`Canvas`] is not associated with raster surface or
    /// GPU surface, returned [`crate::ColorType`] is set to [`crate::ColorType::Unknown`]
    ///
    /// Returns dimensions and [`crate::ColorType`] of [`Canvas`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_imageInfo>
    pub fn image_info(&self) -> ImageInfo {
        let mut ii = ImageInfo::default();
        unsafe { sb::C_SkCanvas_imageInfo(self.native(), ii.native_mut()) };
        ii
    }

    /// Copies [`SurfaceProps`], if [`Canvas`] is associated with raster surface or GPU surface, and
    /// returns `true`. Otherwise, returns `false` and leave props unchanged.
    ///
    /// - `props` storage for writable [`SurfaceProps`]
    ///
    /// Returns `true` if [`SurfaceProps`] was copied
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_getProps>
    pub fn props(&self) -> Option<SurfaceProps> {
        let mut sp = SurfaceProps::default();
        unsafe { self.native().getProps(sp.native_mut()) }.then_some(sp)
    }

    /// Returns the [`SurfaceProps`] associated with the canvas (i.e., at the base of the layer
    /// stack).
    pub fn base_props(&self) -> SurfaceProps {
        SurfaceProps::from_native_c(unsafe { self.native().getBaseProps() })
    }

    /// Returns the [`SurfaceProps`] associated with the canvas that are currently active (i.e., at
    /// the top of the layer stack). This can differ from [`Self::base_props`] depending on the flags
    /// passed to saveLayer (see [`SaveLayerFlags`]).
    pub fn top_props(&self) -> SurfaceProps {
        SurfaceProps::from_native_c(unsafe { self.native().getTopProps() })
    }

    /// Gets the size of the base or root layer in global canvas coordinates. The
    /// origin of the base layer is always (0,0). The area available for drawing may be
    /// smaller (due to clipping or saveLayer).
    ///
    /// Returns integral size of base layer
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_getBaseLayerSize>
    pub fn base_layer_size(&self) -> ISize {
        let mut size = ISize::default();
        unsafe { sb::C_SkCanvas_getBaseLayerSize(self.native(), size.native_mut()) }
        size
    }

    /// Creates [`Surface`] matching info and props, and associates it with [`Canvas`].
    /// Returns `None` if no match found.
    ///
    /// If props is `None`, matches [`SurfaceProps`] in [`Canvas`]. If props is `None` and
    /// [`Canvas`] does not have [`SurfaceProps`], creates [`Surface`] with default
    /// [`SurfaceProps`].
    ///
    /// - `info` width, height, [`crate::ColorType`], [`crate::AlphaType`], and
    ///   [`crate::ColorSpace`]
    /// - `props` [`SurfaceProps`] to match; may be `None` to match [`Canvas`]
    ///
    /// Returns [`Surface`] matching info and props, or `None` if no match is available
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_makeSurface>
    pub fn new_surface(&self, info: &ImageInfo, props: Option<&SurfaceProps>) -> Option<Surface> {
        Surface::from_ptr(unsafe {
            sb::C_SkCanvas_makeSurface(self.native_mut(), info.native(), props.native_ptr_or_null())
        })
    }

    /// Returns Ganesh context of the GPU surface associated with [`Canvas`].
    ///
    /// Returns GPU context, if available; `None` otherwise
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_recordingContext>
    #[cfg(feature = "gpu")]
    pub fn recording_context(&self) -> Option<gpu::RecordingContext> {
        gpu::RecordingContext::from_unshared_ptr(unsafe {
            sb::C_SkCanvas_recordingContext(self.native())
        })
    }

    /// Returns the [`gpu::DirectContext`].
    /// This is a rust-skia helper for that makes it simpler to call [`Image::encode`].
    #[cfg(feature = "gpu")]
    pub fn direct_context(&self) -> Option<gpu::DirectContext> {
        self.recording_context()
            .and_then(|mut c| c.as_direct_context())
    }

    /// Sometimes a canvas is owned by a surface. If it is, [`Self::surface()`] will return a bare
    /// pointer to that surface, else this will return `None`.
    ///
    /// # Safety
    /// This function is unsafe because it is not clear how exactly the lifetime of the canvas
    /// relates to surface returned.
    /// See also [`OwnedCanvas`], [`RCHandle<SkSurface>::canvas()`].
    pub unsafe fn surface(&self) -> Option<Surface> {
        // TODO: It might be possible to make this safe by returning a _kind of_ reference to the
        //       Surface that can not be cloned and stays bound to the lifetime of canvas.
        //       But even then, the Surface might exist twice then, which is confusing, but
        //       probably safe, because the first instance is borrowed by the canvas.
        Surface::from_unshared_ptr(self.native().getSurface())
    }

    /// Returns the pixel base address, [`ImageInfo`], `row_bytes`, and origin if the pixels
    /// can be read directly.
    ///
    /// - `info` storage for writable pixels' [`ImageInfo`]
    /// - `row_bytes` storage for writable pixels' row bytes
    /// - `origin` storage for [`Canvas`] top layer origin, its top-left corner
    ///
    /// Returns address of pixels, or `None` if inaccessible
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_accessTopLayerPixels_a>
    /// example: <https://fiddle.skia.org/c/@Canvas_accessTopLayerPixels_b>
    pub fn access_top_layer_pixels(&self) -> Option<TopLayerPixels> {
        let mut info = ImageInfo::default();
        let mut row_bytes = 0;
        let mut origin = IPoint::default();
        let ptr = unsafe {
            self.native_mut().accessTopLayerPixels(
                info.native_mut(),
                &mut row_bytes,
                origin.native_mut(),
            )
        };
        if !ptr.is_null() {
            let size = info.compute_byte_size(row_bytes);
            let pixels = unsafe { slice::from_raw_parts_mut(ptr as _, size) };
            Some(TopLayerPixels {
                pixels,
                info,
                row_bytes,
                origin,
            })
        } else {
            None
        }
    }

    // TODO: accessTopRasterHandle()

    /// Returns `true` if [`Canvas`] has direct access to its pixels.
    ///
    /// Pixels are readable when `Device` is raster. Pixels are not readable when [`Canvas`] is
    /// returned from GPU surface, returned by [`crate::Document::begin_page()`], returned by
    /// [`Handle<SkPictureRecorder>::begin_recording()`], or [`Canvas`] is the base of a utility
    /// class like `DebugCanvas`.
    ///
    /// pixmap is valid only while [`Canvas`] is in scope and unchanged. Any [`Canvas`] or
    /// [`Surface`] call may invalidate the pixmap values.
    ///
    /// Returns [`Pixmap`] if [`Canvas`] has direct access to pixels
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_peekPixels>
    pub fn peek_pixels(&self) -> Option<Pixmap> {
        let mut pixmap = Pixmap::default();
        unsafe { self.native_mut().peekPixels(pixmap.native_mut()) }.then_some(pixmap)
    }

    /// Copies [`Rect`] of pixels from [`Canvas`] into `dst_pixels`. [`Matrix`] and clip are
    /// ignored.
    ///
    /// Source [`Rect`] corners are `src_point` and `(image_info().width(), image_info().height())`.
    /// Destination [`Rect`] corners are `(0, 0)` and `(dst_Info.width(), dst_info.height())`.
    /// Copies each readable pixel intersecting both rectangles, without scaling,
    /// converting to `dst_info.color_type()` and `dst_info.alpha_type()` if required.
    ///
    /// Pixels are readable when `Device` is raster, or backed by a GPU.
    /// Pixels are not readable when [`Canvas`] is returned by [`crate::Document::begin_page()`],
    /// returned by [`Handle<SkPictureRecorder>::begin_recording()`], or [`Canvas`] is the base of a
    /// utility class like `DebugCanvas`.
    ///
    /// The destination pixel storage must be allocated by the caller.
    ///
    /// Pixel values are converted only if [`crate::ColorType`] and [`crate::AlphaType`]
    /// do not match. Only pixels within both source and destination rectangles
    /// are copied. `dst_pixels` contents outside [`Rect`] intersection are unchanged.
    ///
    /// Pass negative values for `src_point.x` or `src_point.y` to offset pixels across or down
    /// destination.
    ///
    /// Does not copy, and returns `false` if:
    /// - Source and destination rectangles do not intersect.
    /// - [`Canvas`] pixels could not be converted to `dst_info.color_type()` or
    ///   `dst_info.alpha_type()`.
    /// - [`Canvas`] pixels are not readable; for instance, [`Canvas`] is document-based.
    /// - `dst_row_bytes` is too small to contain one row of pixels.
    ///
    /// - `dst_info` width, height, [`crate::ColorType`], and [`crate::AlphaType`] of dstPixels
    /// - `dst_pixels` storage for pixels; `dst_info.height()` times `dst_row_bytes`, or larger
    /// - `dst_row_bytes` size of one destination row; `dst_info.width()` times pixel size, or
    ///   larger
    /// - `src_point` offset into readable pixels; may be negative
    ///
    /// Returns `true` if pixels were copied
    #[must_use]
    pub fn read_pixels(
        &self,
        dst_info: &ImageInfo,
        dst_pixels: &mut [u8],
        dst_row_bytes: usize,
        src_point: impl Into<IPoint>,
    ) -> bool {
        let src_point = src_point.into();
        let required_size = dst_info.compute_byte_size(dst_row_bytes);
        (dst_pixels.len() >= required_size)
            && unsafe {
                self.native_mut().readPixels(
                    dst_info.native(),
                    dst_pixels.as_mut_ptr() as _,
                    dst_row_bytes,
                    src_point.x,
                    src_point.y,
                )
            }
    }

    /// Copies [`Rect`] of pixels from [`Canvas`] into pixmap. [`Matrix`] and clip are
    /// ignored.
    ///
    /// Source [`Rect`] corners are `(src.x, src.y)` and `(image_info().width(),
    /// image_info().height())`.
    /// Destination [`Rect`] corners are `(0, 0)` and `(pixmap.width(), pixmap.height())`.
    /// Copies each readable pixel intersecting both rectangles, without scaling,
    /// converting to `pixmap.color_type()` and `pixmap.alpha_type()` if required.
    ///
    /// Pixels are readable when `Device` is raster, or backed by a GPU. Pixels are not readable
    /// when [`Canvas`] is returned by [`crate::Document::begin_page()`], returned by
    /// [`Handle<SkPictureRecorder>::begin_recording()`], or [`Canvas`] is the base of a utility
    /// class like `DebugCanvas`.
    ///
    /// Caller must allocate pixel storage in pixmap if needed.
    ///
    /// Pixel values are converted only if [`crate::ColorType`] and [`crate::AlphaType`] do not
    /// match. Only pixels within both source and destination [`Rect`] are copied. pixmap pixels
    /// contents outside [`Rect`] intersection are unchanged.
    ///
    /// Pass negative values for `src.x` or `src.y` to offset pixels across or down pixmap.
    ///
    /// Does not copy, and returns `false` if:
    /// - Source and destination rectangles do not intersect.
    /// - [`Canvas`] pixels could not be converted to `pixmap.color_type()` or
    ///   `pixmap.alpha_type()`.
    /// - [`Canvas`] pixels are not readable; for instance, [`Canvas`] is document-based.
    /// - [`Pixmap`] pixels could not be allocated.
    /// - `pixmap.row_bytes()` is too small to contain one row of pixels.
    ///
    /// - `pixmap` storage for pixels copied from [`Canvas`]
    /// - `src` offset into readable pixels ; may be negative
    ///
    /// Returns `true` if pixels were copied
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_readPixels_2>
    #[must_use]
    pub fn read_pixels_to_pixmap(&self, pixmap: &mut Pixmap, src: impl Into<IPoint>) -> bool {
        let src = src.into();
        unsafe { self.native_mut().readPixels1(pixmap.native(), src.x, src.y) }
    }

    /// Copies [`Rect`] of pixels from [`Canvas`] into bitmap. [`Matrix`] and clip are
    /// ignored.
    ///
    /// Source [`Rect`] corners are `(src.x, src.y)` and `(image_info().width(),
    /// image_info().height())`.
    /// Destination [`Rect`] corners are `(0, 0)` and `(bitmap.width(), bitmap.height())`.
    /// Copies each readable pixel intersecting both rectangles, without scaling,
    /// converting to `bitmap.color_type()` and `bitmap.alpha_type()` if required.
    ///
    /// Pixels are readable when `Device` is raster, or backed by a GPU. Pixels are not readable
    /// when [`Canvas`] is returned by [`crate::Document::begin_page()`], returned by
    /// [`Handle<SkPictureRecorder>::begin_recording()`], or [`Canvas`] is the base of a utility
    /// class like DebugCanvas.
    ///
    /// Caller must allocate pixel storage in bitmap if needed.
    ///
    /// [`Bitmap`] values are converted only if [`crate::ColorType`] and [`crate::AlphaType`]
    /// do not match. Only pixels within both source and destination rectangles
    /// are copied. [`Bitmap`] pixels outside [`Rect`] intersection are unchanged.
    ///
    /// Pass negative values for srcX or srcY to offset pixels across or down bitmap.
    ///
    /// Does not copy, and returns `false` if:
    /// - Source and destination rectangles do not intersect.
    /// - [`Canvas`] pixels could not be converted to `bitmap.color_type()` or
    ///   `bitmap.alpha_type()`.
    /// - [`Canvas`] pixels are not readable; for instance, [`Canvas`] is document-based.
    /// - bitmap pixels could not be allocated.
    /// - `bitmap.row_bytes()` is too small to contain one row of pixels.
    ///
    /// - `bitmap` storage for pixels copied from [`Canvas`]
    /// - `src` offset into readable pixels; may be negative
    ///
    /// Returns `true` if pixels were copied
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_readPixels_3>
    #[must_use]
    pub fn read_pixels_to_bitmap(&self, bitmap: &mut Bitmap, src: impl Into<IPoint>) -> bool {
        let src = src.into();
        unsafe {
            self.native_mut()
                .readPixels2(bitmap.native_mut(), src.x, src.y)
        }
    }

    /// Copies [`Rect`] from pixels to [`Canvas`]. [`Matrix`] and clip are ignored.
    /// Source [`Rect`] corners are `(0, 0)` and `(info.width(), info.height())`.
    /// Destination [`Rect`] corners are `(offset.x, offset.y)` and
    /// `(image_info().width(), image_info().height())`.
    ///
    /// Copies each readable pixel intersecting both rectangles, without scaling,
    /// converting to `image_info().color_type()` and `image_info().alpha_type()` if required.
    ///
    /// Pixels are writable when `Device` is raster, or backed by a GPU.
    /// Pixels are not writable when [`Canvas`] is returned by [`crate::Document::begin_page()`],
    /// returned by [`Handle<SkPictureRecorder>::begin_recording()`], or [`Canvas`] is the base of a
    /// utility class like `DebugCanvas`.
    ///
    /// Pixel values are converted only if [`crate::ColorType`] and [`crate::AlphaType`]
    /// do not match. Only pixels within both source and destination rectangles
    /// are copied. [`Canvas`] pixels outside [`Rect`] intersection are unchanged.
    ///
    /// Pass negative values for `offset.x` or `offset.y` to offset pixels to the left or
    /// above [`Canvas`] pixels.
    ///
    /// Does not copy, and returns `false` if:
    /// - Source and destination rectangles do not intersect.
    /// - pixels could not be converted to [`Canvas`] `image_info().color_type()` or
    ///   `image_info().alpha_type()`.
    /// - [`Canvas`] pixels are not writable; for instance, [`Canvas`] is document-based.
    /// - `row_bytes` is too small to contain one row of pixels.
    ///
    /// - `info` width, height, [`crate::ColorType`], and [`crate::AlphaType`] of pixels
    /// - `pixels` pixels to copy, of size `info.height()` times `row_bytes`, or larger
    /// - `row_bytes` size of one row of pixels; info.width() times pixel size, or larger
    /// - `offset` offset into [`Canvas`] writable pixels; may be negative
    ///
    /// Returns `true` if pixels were written to [`Canvas`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_writePixels>
    #[must_use]
    pub fn write_pixels(
        &self,
        info: &ImageInfo,
        pixels: &[u8],
        row_bytes: usize,
        offset: impl Into<IPoint>,
    ) -> bool {
        let offset = offset.into();
        let required_size = info.compute_byte_size(row_bytes);
        (pixels.len() >= required_size)
            && unsafe {
                self.native_mut().writePixels(
                    info.native(),
                    pixels.as_ptr() as _,
                    row_bytes,
                    offset.x,
                    offset.y,
                )
            }
    }

    /// Copies [`Rect`] from pixels to [`Canvas`]. [`Matrix`] and clip are ignored.
    /// Source [`Rect`] corners are `(0, 0)` and `(bitmap.width(), bitmap.height())`.
    ///
    /// Destination [`Rect`] corners are `(offset.x, offset.y)` and
    /// `(image_info().width(), image_info().height())`.
    ///
    /// Copies each readable pixel intersecting both rectangles, without scaling,
    /// converting to `image_info().color_type()` and `image_info().alpha_type()` if required.
    ///
    /// Pixels are writable when `Device` is raster, or backed by a GPU. Pixels are not writable
    /// when [`Canvas`] is returned by [`crate::Document::begin_page()`], returned by
    /// [`Handle<SkPictureRecorder>::begin_recording()`], or [`Canvas`] is the base of a utility
    /// class like `DebugCanvas`.
    ///
    /// Pixel values are converted only if [`crate::ColorType`] and [`crate::AlphaType`]
    /// do not match. Only pixels within both source and destination rectangles
    /// are copied. [`Canvas`] pixels outside [`Rect`] intersection are unchanged.
    ///
    /// Pass negative values for `offset` to offset pixels to the left or
    /// above [`Canvas`] pixels.
    ///
    /// Does not copy, and returns `false` if:
    /// - Source and destination rectangles do not intersect.
    /// - bitmap does not have allocated pixels.
    /// - bitmap pixels could not be converted to [`Canvas`] `image_info().color_type()` or
    ///   `image_info().alpha_type()`.
    /// - [`Canvas`] pixels are not writable; for instance, [`Canvas`] is document based.
    /// - bitmap pixels are inaccessible; for instance, bitmap wraps a texture.
    ///
    /// - `bitmap` contains pixels copied to [`Canvas`]
    /// - `offset` offset into [`Canvas`] writable pixels; may be negative
    ///
    /// Returns `true` if pixels were written to [`Canvas`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_writePixels_2>
    /// example: <https://fiddle.skia.org/c/@State_Stack_a>
    /// example: <https://fiddle.skia.org/c/@State_Stack_b>
    #[must_use]
    pub fn write_pixels_from_bitmap(&self, bitmap: &Bitmap, offset: impl Into<IPoint>) -> bool {
        let offset = offset.into();
        unsafe {
            self.native_mut()
                .writePixels1(bitmap.native(), offset.x, offset.y)
        }
    }

    /// Saves [`Matrix`] and clip.
    /// Calling [`Self::restore()`] discards changes to [`Matrix`] and clip,
    /// restoring the [`Matrix`] and clip to their state when [`Self::save()`] was called.
    ///
    /// [`Matrix`] may be changed by [`Self::translate()`], [`Self::scale()`], [`Self::rotate()`],
    /// [`Self::skew()`], [`Self::concat()`], [`Self::set_matrix()`], and [`Self::reset_matrix()`].
    /// Clip may be changed by [`Self::clip_rect()`], [`Self::clip_rrect()`], [`Self::clip_path()`],
    /// [`Self::clip_region()`].
    ///
    /// Saved [`Canvas`] state is put on a stack; multiple calls to [`Self::save()`] should be
    /// balance by an equal number of calls to [`Self::restore()`].
    ///
    /// Call [`Self::restore_to_count()`] with result to restore this and subsequent saves.
    ///
    /// Returns depth of saved stack
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_save>
    pub fn save(&self) -> usize {
        unsafe { self.native_mut().save().try_into().unwrap() }
    }

    // The save_layer(bounds, paint) variants have been replaced by SaveLayerRec.

    /// Saves [`Matrix`] and clip, and allocates [`Surface`] for subsequent drawing.
    ///
    /// Calling [`Self::restore()`] discards changes to [`Matrix`] and clip, and blends layer with
    /// alpha opacity onto prior layer.
    ///
    /// [`Matrix`] may be changed by [`Self::translate()`], [`Self::scale()`], [`Self::rotate()`],
    /// [`Self::skew()`], [`Self::concat()`], [`Self::set_matrix()`], and [`Self::reset_matrix()`].
    /// Clip may be changed by [`Self::clip_rect()`], [`Self::clip_rrect()`], [`Self::clip_path()`],
    /// [`Self::clip_region()`].
    ///
    /// [`Rect`] bounds suggests but does not define layer size. To clip drawing to a specific
    /// rectangle, use [`Self::clip_rect()`].
    ///
    /// alpha of zero is fully transparent, 1.0 is fully opaque.
    ///
    /// Call [`Self::restore_to_count()`] with result to restore this and subsequent saves.
    ///
    /// - `bounds` hint to limit the size of layer; may be `None`
    /// - `alpha` opacity of layer
    ///
    /// Returns depth of saved stack
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_saveLayerAlpha>
    pub fn save_layer_alpha_f(&self, bounds: impl Into<Option<Rect>>, alpha: f32) -> usize {
        unsafe {
            self.native_mut()
                .saveLayerAlphaf(bounds.into().native().as_ptr_or_null(), alpha)
        }
        .try_into()
        .unwrap()
    }

    /// Helper that accepts an int between 0 and 255, and divides it by 255.0
    pub fn save_layer_alpha(&self, bounds: impl Into<Option<Rect>>, alpha: U8CPU) -> usize {
        self.save_layer_alpha_f(bounds, alpha as f32 * (1.0 / 255.0))
    }

    /// Saves [`Matrix`] and clip, and allocates [`Surface`] for subsequent drawing.
    ///
    /// Calling [`Self::restore()`] discards changes to [`Matrix`] and clip,
    /// and blends [`Surface`] with alpha opacity onto the prior layer.
    ///
    /// [`Matrix`] may be changed by [`Self::translate()`], [`Self::scale()`], [`Self::rotate()`],
    /// [`Self::skew()`], [`Self::concat()`], [`Self::set_matrix()`], and [`Self::reset_matrix()`].
    /// Clip may be changed by [`Self::clip_rect()`], [`Self::clip_rrect()`], [`Self::clip_path()`],
    /// [`Self::clip_region()`].
    ///
    /// [`SaveLayerRec`] contains the state used to create the layer.
    ///
    /// Call [`Self::restore_to_count()`] with result to restore this and subsequent saves.
    ///
    /// - `layer_rec` layer state
    ///
    /// Returns depth of save state stack before this call was made.
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_saveLayer_3>
    pub fn save_layer(&self, layer_rec: &SaveLayerRec) -> usize {
        unsafe { self.native_mut().saveLayer1(layer_rec.native()) }
            .try_into()
            .unwrap()
    }

    /// Removes changes to [`Matrix`] and clip since [`Canvas`] state was
    /// last saved. The state is removed from the stack.
    ///
    /// Does nothing if the stack is empty.
    ///
    /// example: <https://fiddle.skia.org/c/@AutoCanvasRestore_restore>
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_restore>
    pub fn restore(&self) -> &Self {
        unsafe { self.native_mut().restore() };
        self
    }

    /// Returns the number of saved states, each containing: [`Matrix`] and clip.
    /// Equals the number of [`Self::save()`] calls less the number of [`Self::restore()`] calls
    /// plus one.
    /// The save count of a new canvas is one.
    ///
    /// Returns depth of save state stack
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_getSaveCount>
    pub fn save_count(&self) -> usize {
        unsafe { self.native().getSaveCount() }.try_into().unwrap()
    }

    /// Restores state to [`Matrix`] and clip values when [`Self::save()`], [`Self::save_layer()`],
    /// or [`Self::save_layer_alpha()`] returned `save_count`.
    ///
    /// Does nothing if `save_count` is greater than state stack count.
    /// Restores state to initial values if `save_count` is less than or equal to one.
    ///
    /// - `saveCount` depth of state stack to restore
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_restoreToCount>
    pub fn restore_to_count(&self, save_count: usize) -> &Self {
        unsafe {
            self.native_mut()
                .restoreToCount(save_count.try_into().unwrap())
        }
        self
    }

    /// Translates [`Matrix`] by `d`.
    ///
    /// Mathematically, replaces [`Matrix`] with a translation matrix premultiplied with [`Matrix`].
    ///
    /// This has the effect of moving the drawing by `(d.x, d.y)` before transforming the result
    /// with [`Matrix`].
    ///
    /// - `d` distance to translate
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_translate>
    pub fn translate(&self, d: impl Into<Vector>) -> &Self {
        let d = d.into();
        unsafe { self.native_mut().translate(d.x, d.y) }
        self
    }

    /// Scales [`Matrix`] by `sx` on the x-axis and `sy` on the y-axis.
    ///
    /// Mathematically, replaces [`Matrix`] with a scale matrix premultiplied with [`Matrix`].
    ///
    /// This has the effect of scaling the drawing by `(sx, sy)` before transforming the result with
    /// [`Matrix`].
    ///
    /// - `sx` amount to scale on x-axis
    /// - `sy` amount to scale on y-axis
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_scale>
    pub fn scale(&self, (sx, sy): (scalar, scalar)) -> &Self {
        unsafe { self.native_mut().scale(sx, sy) }
        self
    }

    /// Rotates [`Matrix`] by degrees about a point at `(p.x, p.y)`. Positive degrees rotates
    /// clockwise.
    ///
    /// Mathematically, constructs a rotation matrix; premultiplies the rotation matrix by a
    /// translation matrix; then replaces [`Matrix`] with the resulting matrix premultiplied with
    /// [`Matrix`].
    ///
    /// This has the effect of rotating the drawing about a given point before transforming the
    /// result with [`Matrix`].
    ///
    /// - `degrees` amount to rotate, in degrees
    /// - `p` the point to rotate about
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_rotate_2>
    pub fn rotate(&self, degrees: scalar, p: Option<Point>) -> &Self {
        unsafe {
            match p {
                Some(point) => self.native_mut().rotate1(degrees, point.x, point.y),
                None => self.native_mut().rotate(degrees),
            }
        }
        self
    }

    /// Skews [`Matrix`] by `sx` on the x-axis and `sy` on the y-axis. A positive value of `sx`
    /// skews the drawing right as y-axis values increase; a positive value of `sy` skews the
    /// drawing down as x-axis values increase.
    ///
    /// Mathematically, replaces [`Matrix`] with a skew matrix premultiplied with [`Matrix`].
    ///
    /// This has the effect of skewing the drawing by `(sx, sy)` before transforming the result with
    /// [`Matrix`].
    ///
    /// - `sx` amount to skew on x-axis
    /// - `sy` amount to skew on y-axis
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_skew>
    pub fn skew(&self, (sx, sy): (scalar, scalar)) -> &Self {
        unsafe { self.native_mut().skew(sx, sy) }
        self
    }

    /// Replaces [`Matrix`] with matrix premultiplied with existing [`Matrix`].
    ///
    /// This has the effect of transforming the drawn geometry by matrix, before transforming the
    /// result with existing [`Matrix`].
    ///
    /// - `matrix` matrix to premultiply with existing [`Matrix`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_concat>
    pub fn concat(&self, matrix: &Matrix) -> &Self {
        unsafe { self.native_mut().concat(matrix.native()) }
        self
    }

    pub fn concat_44(&self, m: &M44) -> &Self {
        unsafe { self.native_mut().concat1(m.native()) }
        self
    }

    /// Replaces [`Matrix`] with `matrix`.
    /// Unlike [`Self::concat()`], any prior matrix state is overwritten.
    ///
    /// - `matrix` matrix to copy, replacing existing [`Matrix`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_setMatrix>
    pub fn set_matrix(&self, matrix: &M44) -> &Self {
        unsafe { self.native_mut().setMatrix(matrix.native()) }
        self
    }

    /// Sets [`Matrix`] to the identity matrix.
    /// Any prior matrix state is overwritten.
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_resetMatrix>
    pub fn reset_matrix(&self) -> &Self {
        unsafe { self.native_mut().resetMatrix() }
        self
    }

    /// Replaces clip with the intersection or difference of clip and `rect`,
    /// with an aliased or anti-aliased clip edge. `rect` is transformed by [`Matrix`]
    /// before it is combined with clip.
    ///
    /// - `rect` [`Rect`] to combine with clip
    /// - `op` [`ClipOp`] to apply to clip
    /// - `do_anti_alias` `true` if clip is to be anti-aliased
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_clipRect>
    pub fn clip_rect(
        &self,
        rect: impl AsRef<Rect>,
        op: impl Into<Option<ClipOp>>,
        do_anti_alias: impl Into<Option<bool>>,
    ) -> &Self {
        unsafe {
            self.native_mut().clipRect(
                rect.as_ref().native(),
                op.into().unwrap_or_default(),
                do_anti_alias.into().unwrap_or_default(),
            )
        }
        self
    }

    pub fn clip_irect(&self, irect: impl AsRef<IRect>, op: impl Into<Option<ClipOp>>) -> &Self {
        let r = Rect::from(*irect.as_ref());
        self.clip_rect(r, op, false)
    }

    /// Replaces clip with the intersection or difference of clip and `rrect`,
    /// with an aliased or anti-aliased clip edge.
    /// `rrect` is transformed by [`Matrix`]
    /// before it is combined with clip.
    ///
    /// - `rrect` [`RRect`] to combine with clip
    /// - `op` [`ClipOp`] to apply to clip
    /// - `do_anti_alias` `true` if clip is to be anti-aliased
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_clipRRect>
    pub fn clip_rrect(
        &self,
        rrect: impl AsRef<RRect>,
        op: impl Into<Option<ClipOp>>,
        do_anti_alias: impl Into<Option<bool>>,
    ) -> &Self {
        unsafe {
            self.native_mut().clipRRect(
                rrect.as_ref().native(),
                op.into().unwrap_or_default(),
                do_anti_alias.into().unwrap_or_default(),
            )
        }
        self
    }

    /// Replaces clip with the intersection or difference of clip and `path`,
    /// with an aliased or anti-aliased clip edge. [`crate::PathFillType`] determines if `path`
    /// describes the area inside or outside its contours; and if path contour overlaps
    /// itself or another path contour, whether the overlaps form part of the area.
    /// `path` is transformed by [`Matrix`] before it is combined with clip.
    ///
    /// - `path` [`Path`] to combine with clip
    /// - `op` [`ClipOp`] to apply to clip
    /// - `do_anti_alias` `true` if clip is to be anti-aliased
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_clipPath>
    pub fn clip_path(
        &self,
        path: &Path,
        op: impl Into<Option<ClipOp>>,
        do_anti_alias: impl Into<Option<bool>>,
    ) -> &Self {
        unsafe {
            self.native_mut().clipPath(
                path.native(),
                op.into().unwrap_or_default(),
                do_anti_alias.into().unwrap_or_default(),
            )
        }
        self
    }

    pub fn clip_shader(&self, shader: impl Into<Shader>, op: impl Into<Option<ClipOp>>) -> &Self {
        unsafe {
            sb::C_SkCanvas_clipShader(
                self.native_mut(),
                shader.into().into_ptr(),
                op.into().unwrap_or(ClipOp::Intersect),
            )
        }
        self
    }

    /// Replaces clip with the intersection or difference of clip and [`Region`] `device_rgn`.
    /// Resulting clip is aliased; pixels are fully contained by the clip.
    /// `device_rgn` is unaffected by [`Matrix`].
    ///
    /// - `device_rgn` [`Region`] to combine with clip
    /// - `op` [`ClipOp`] to apply to clip
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_clipRegion>
    pub fn clip_region(&self, device_rgn: &Region, op: impl Into<Option<ClipOp>>) -> &Self {
        unsafe {
            self.native_mut()
                .clipRegion(device_rgn.native(), op.into().unwrap_or_default())
        }
        self
    }

    // quickReject() functions are implemented as a trait.

    /// Returns bounds of clip, transformed by inverse of [`Matrix`]. If clip is empty,
    /// return [`Rect::new_empty()`], where all [`Rect`] sides equal zero.
    ///
    /// [`Rect`] returned is outset by one to account for partial pixel coverage if clip
    /// is anti-aliased.
    ///
    /// Returns bounds of clip in local coordinates
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_getLocalClipBounds>
    pub fn local_clip_bounds(&self) -> Option<Rect> {
        let r = Rect::construct(|r| unsafe { sb::C_SkCanvas_getLocalClipBounds(self.native(), r) });
        (!r.is_empty()).then_some(r)
    }

    /// Returns [`IRect`] bounds of clip, unaffected by [`Matrix`]. If clip is empty,
    /// return [`Rect::new_empty()`], where all [`Rect`] sides equal zero.
    ///
    /// Unlike [`Self::local_clip_bounds()`], returned [`IRect`] is not outset.
    ///
    /// Returns bounds of clip in `Device` coordinates
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_getDeviceClipBounds>
    pub fn device_clip_bounds(&self) -> Option<IRect> {
        let r =
            IRect::construct(|r| unsafe { sb::C_SkCanvas_getDeviceClipBounds(self.native(), r) });
        (!r.is_empty()).then_some(r)
    }

    /// Fills clip with color `color`.
    /// `mode` determines how ARGB is combined with destination.
    ///
    /// - `color` [`Color4f`] representing unpremultiplied color.
    /// - `mode` [`BlendMode`] used to combine source color and destination
    pub fn draw_color(
        &self,
        color: impl Into<Color4f>,
        mode: impl Into<Option<BlendMode>>,
    ) -> &Self {
        unsafe {
            self.native_mut()
                .drawColor(&color.into().into_native(), mode.into().unwrap_or_default())
        }
        self
    }

    /// Fills clip with color `color` using [`BlendMode::Src`].
    /// This has the effect of replacing all pixels contained by clip with `color`.
    ///
    /// - `color` [`Color4f`] representing unpremultiplied color.
    pub fn clear(&self, color: impl Into<Color4f>) -> &Self {
        self.draw_color(color, BlendMode::Src)
    }

    /// Makes [`Canvas`] contents undefined. Subsequent calls that read [`Canvas`] pixels,
    /// such as drawing with [`BlendMode`], return undefined results. `discard()` does
    /// not change clip or [`Matrix`].
    ///
    /// `discard()` may do nothing, depending on the implementation of [`Surface`] or `Device`
    /// that created [`Canvas`].
    ///
    /// `discard()` allows optimized performance on subsequent draws by removing
    /// cached data associated with [`Surface`] or `Device`.
    /// It is not necessary to call `discard()` once done with [`Canvas`];
    /// any cached data is deleted when owning [`Surface`] or `Device` is deleted.
    pub fn discard(&self) -> &Self {
        unsafe { sb::C_SkCanvas_discard(self.native_mut()) }
        self
    }

    /// Fills clip with [`Paint`] `paint`. [`Paint`] components, [`Shader`],
    /// [`crate::ColorFilter`], [`ImageFilter`], and [`BlendMode`] affect drawing;
    /// [`crate::MaskFilter`] and [`crate::PathEffect`] in `paint` are ignored.
    ///
    /// - `paint` graphics state used to fill [`Canvas`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawPaint>
    pub fn draw_paint(&self, paint: &Paint) -> &Self {
        unsafe { self.native_mut().drawPaint(paint.native()) }
        self
    }

    /// Draws `pts` using clip, [`Matrix`] and [`Paint`] `pain`.
    /// if the number of points is less than one, has no effect.
    /// `mode` may be one of: [`PointMode::Points`], [`PointMode::Lines`], or [`PointMode::Polygon`]
    ///
    /// If `mode` is [`PointMode::Points`], the shape of point drawn depends on `paint`
    /// [`crate::paint::Cap`]. If `paint` is set to [`crate::paint::Cap::Round`], each point draws a
    /// circle of diameter [`Paint`] stroke width. If `paint` is set to [`crate::paint::Cap::Square`]
    /// or [`crate::paint::Cap::Butt`], each point draws a square of width and height
    /// [`Paint`] stroke width.
    ///
    /// If `mode` is [`PointMode::Lines`], each pair of points draws a line segment.
    /// One line is drawn for every two points; each point is used once. If count is odd,
    /// the final point is ignored.
    ///
    /// If mode is [`PointMode::Polygon`], each adjacent pair of points draws a line segment.
    /// count minus one lines are drawn; the first and last point are used once.
    ///
    /// Each line segment respects `paint` [`crate::paint::Cap`] and [`Paint`] stroke width.
    /// [`crate::paint::Style`] is ignored, as if were set to [`crate::paint::Style::Stroke`].
    ///
    /// Always draws each element one at a time; is not affected by
    /// [`crate::paint::Join`], and unlike [`Self::draw_path()`], does not create a mask from all points
    /// and lines before drawing.
    ///
    /// - `mode` whether pts draws points or lines
    /// - `pts` array of points to draw
    /// - `paint` stroke, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawPoints>
    pub fn draw_points(&self, mode: PointMode, pts: &[Point], paint: &Paint) -> &Self {
        unsafe {
            sb::C_SkCanvas_drawPoints(
                self.native_mut(),
                mode,
                pts.native().as_ptr(),
                pts.len(),
                paint.native(),
            )
        }
        self
    }

    /// Draws point `p` using clip, [`Matrix`] and [`Paint`] paint.
    ///
    /// The shape of point drawn depends on `paint` [`crate::paint::Cap`].
    /// If `paint` is set to [`crate::paint::Cap::Round`], draw a circle of diameter [`Paint`]
    /// stroke width. If `paint` is set to [`crate::paint::Cap::Square`] or
    /// [`crate::paint::Cap::Butt`], draw a square of width and height [`Paint`] stroke width.
    /// [`crate::paint::Style`] is ignored, as if were set to [`crate::paint::Style::Stroke`].
    ///
    /// - `p` top-left edge of circle or square
    /// - `paint` stroke, blend, color, and so on, used to draw
    pub fn draw_point(&self, p: impl Into<Point>, paint: &Paint) -> &Self {
        let p = p.into();
        unsafe { self.native_mut().drawPoint(p.x, p.y, paint.native()) }
        self
    }

    /// Draws line segment from `p1` to `p2` using clip, [`Matrix`], and [`Paint`] paint.
    /// In paint: [`Paint`] stroke width describes the line thickness;
    /// [`crate::paint::Cap`] draws the end rounded or square;
    /// [`crate::paint::Style`] is ignored, as if were set to [`crate::paint::Style::Stroke`].
    ///
    /// - `p1` start of line segment
    /// - `p2` end of line segment
    /// - `paint` stroke, blend, color, and so on, used to draw
    pub fn draw_line(&self, p1: impl Into<Point>, p2: impl Into<Point>, paint: &Paint) -> &Self {
        let (p1, p2) = (p1.into(), p2.into());
        unsafe {
            self.native_mut()
                .drawLine(p1.x, p1.y, p2.x, p2.y, paint.native())
        }
        self
    }

    /// Draws [`Rect`] rect using clip, [`Matrix`], and [`Paint`] `paint`.
    /// In paint: [`crate::paint::Style`] determines if rectangle is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness, and
    /// [`crate::paint::Join`] draws the corners rounded or square.
    ///
    /// - `rect` rectangle to draw
    /// - `paint` stroke or fill, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawRect>
    pub fn draw_rect(&self, rect: impl AsRef<Rect>, paint: &Paint) -> &Self {
        unsafe {
            self.native_mut()
                .drawRect(rect.as_ref().native(), paint.native())
        }
        self
    }

    /// Draws [`IRect`] rect using clip, [`Matrix`], and [`Paint`] `paint`.
    /// In `paint`: [`crate::paint::Style`] determines if rectangle is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness, and
    /// [`crate::paint::Join`] draws the corners rounded or square.
    ///
    /// - `rect` rectangle to draw
    /// - `paint` stroke or fill, blend, color, and so on, used to draw
    pub fn draw_irect(&self, rect: impl AsRef<IRect>, paint: &Paint) -> &Self {
        self.draw_rect(Rect::from(*rect.as_ref()), paint)
    }

    /// Draws [`Region`] region using clip, [`Matrix`], and [`Paint`] `paint`.
    /// In `paint`: [`crate::paint::Style`] determines if rectangle is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness, and
    /// [`crate::paint::Join`] draws the corners rounded or square.
    ///
    /// - `region` region to draw
    /// - `paint` [`Paint`] stroke or fill, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawRegion>
    pub fn draw_region(&self, region: &Region, paint: &Paint) -> &Self {
        unsafe {
            self.native_mut()
                .drawRegion(region.native(), paint.native())
        }
        self
    }

    /// Draws oval oval using clip, [`Matrix`], and [`Paint`].
    /// In `paint`: [`crate::paint::Style`] determines if oval is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness.
    ///
    /// - `oval` [`Rect`] bounds of oval
    /// - `paint` [`Paint`] stroke or fill, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawOval>
    pub fn draw_oval(&self, oval: impl AsRef<Rect>, paint: &Paint) -> &Self {
        unsafe {
            self.native_mut()
                .drawOval(oval.as_ref().native(), paint.native())
        }
        self
    }

    /// Draws [`RRect`] rrect using clip, [`Matrix`], and [`Paint`] `paint`.
    /// In `paint`: [`crate::paint::Style`] determines if rrect is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness.
    ///
    /// `rrect` may represent a rectangle, circle, oval, uniformly rounded rectangle, or
    /// may have any combination of positive non-square radii for the four corners.
    ///
    /// - `rrect` [`RRect`] with up to eight corner radii to draw
    /// - `paint` [`Paint`] stroke or fill, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawRRect>
    pub fn draw_rrect(&self, rrect: impl AsRef<RRect>, paint: &Paint) -> &Self {
        unsafe {
            self.native_mut()
                .drawRRect(rrect.as_ref().native(), paint.native())
        }
        self
    }

    /// Draws [`RRect`] outer and inner
    /// using clip, [`Matrix`], and [`Paint`] `paint`.
    /// outer must contain inner or the drawing is undefined.
    /// In paint: [`crate::paint::Style`] determines if [`RRect`] is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness.
    /// If stroked and [`RRect`] corner has zero length radii, [`crate::paint::Join`] can
    /// draw corners rounded or square.
    ///
    /// GPU-backed platforms optimize drawing when both outer and inner are
    /// concave and outer contains inner. These platforms may not be able to draw
    /// [`Path`] built with identical data as fast.
    ///
    /// - `outer` [`RRect`] outer bounds to draw
    /// - `inner` [`RRect`] inner bounds to draw
    /// - `paint` [`Paint`] stroke or fill, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawDRRect_a>
    /// example: <https://fiddle.skia.org/c/@Canvas_drawDRRect_b>
    pub fn draw_drrect(
        &self,
        outer: impl AsRef<RRect>,
        inner: impl AsRef<RRect>,
        paint: &Paint,
    ) -> &Self {
        unsafe {
            self.native_mut().drawDRRect(
                outer.as_ref().native(),
                inner.as_ref().native(),
                paint.native(),
            )
        }
        self
    }

    /// Draws circle at center with radius using clip, [`Matrix`], and [`Paint`] `paint`.
    /// If radius is zero or less, nothing is drawn.
    /// In `paint`: [`crate::paint::Style`] determines if circle is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness.
    ///
    /// - `center` circle center
    /// - `radius` half the diameter of circle
    /// - `paint` [`Paint`] stroke or fill, blend, color, and so on, used to draw
    pub fn draw_circle(&self, center: impl Into<Point>, radius: scalar, paint: &Paint) -> &Self {
        let center = center.into();
        unsafe {
            self.native_mut()
                .drawCircle(center.x, center.y, radius, paint.native())
        }
        self
    }

    /// Draws arc using clip, [`Matrix`], and [`Paint`] paint.
    ///
    /// Arc is part of oval bounded by oval, sweeping from `start_angle` to `start_angle` plus
    /// `sweep_angle`. `start_angle` and `sweep_angle` are in degrees.
    ///
    /// `start_angle` of zero places start point at the right middle edge of oval.
    /// A positive `sweep_angle` places arc end point clockwise from start point;
    /// a negative `sweep_angle` places arc end point counterclockwise from start point.
    /// `sweep_angle` may exceed 360 degrees, a full circle.
    /// If `use_center` is `true`, draw a wedge that includes lines from oval
    /// center to arc end points. If `use_center` is `false`, draw arc between end points.
    ///
    /// If [`Rect`] oval is empty or `sweep_angle` is zero, nothing is drawn.
    ///
    /// - `oval` [`Rect`] bounds of oval containing arc to draw
    /// - `start_angle` angle in degrees where arc begins
    /// - `sweep_angle` sweep angle in degrees; positive is clockwise
    /// - `use_center` if `true`, include the center of the oval
    /// - `paint` [`Paint`] stroke or fill, blend, color, and so on, used to draw
    pub fn draw_arc(
        &self,
        oval: impl AsRef<Rect>,
        start_angle: scalar,
        sweep_angle: scalar,
        use_center: bool,
        paint: &Paint,
    ) -> &Self {
        unsafe {
            self.native_mut().drawArc(
                oval.as_ref().native(),
                start_angle,
                sweep_angle,
                use_center,
                paint.native(),
            )
        }
        self
    }

    /// Draws arc using clip, [`Matrix`], and [`Paint`] paint.
    ///
    /// Arc is part of oval bounded by oval, sweeping from `start_angle` to `start_angle` plus
    /// `sweep_angle`. `start_angle` and `sweep_angle` are in degrees.
    ///
    /// `start_angle` of zero places start point at the right middle edge of oval.
    /// A positive `sweep_angle` places arc end point clockwise from start point;
    /// a negative `sweep_angle` places arc end point counterclockwise from start point.
    /// `sweep_angle` may exceed 360 degrees, a full circle.
    /// If `use_center` is `true`, draw a wedge that includes lines from oval
    /// center to arc end points. If `use_center` is `false`, draw arc between end points.
    ///
    /// If [`Rect`] oval is empty or `sweep_angle` is zero, nothing is drawn.
    ///
    /// - `arc` [`Arc`] SkArc specifying oval, startAngle, sweepAngle, and arc-vs-wedge
    /// - `paint` [`Paint`] stroke or fill, blend, color, and so on, used to draw
    pub fn draw_arc_2(&self, arc: &Arc, paint: &Paint) -> &Self {
        self.draw_arc(
            arc.oval,
            arc.start_angle,
            arc.sweep_angle,
            arc.is_wedge(),
            paint,
        );
        self
    }

    /// Draws [`RRect`] bounded by [`Rect`] rect, with corner radii `(rx, ry)` using clip,
    /// [`Matrix`], and [`Paint`] `paint`.
    ///
    /// In `paint`: [`crate::paint::Style`] determines if [`RRect`] is stroked or filled;
    /// if stroked, [`Paint`] stroke width describes the line thickness.
    /// If `rx` or `ry` are less than zero, they are treated as if they are zero.
    /// If `rx` plus `ry` exceeds rect width or rect height, radii are scaled down to fit.
    /// If `rx` and `ry` are zero, [`RRect`] is drawn as [`Rect`] and if stroked is affected by
    /// [`crate::paint::Join`].
    ///
    /// - `rect` [`Rect`] bounds of [`RRect`] to draw
    /// - `rx` axis length on x-axis of oval describing rounded corners
    /// - `ry` axis length on y-axis of oval describing rounded corners
    /// - `paint` stroke, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawRoundRect>
    pub fn draw_round_rect(
        &self,
        rect: impl AsRef<Rect>,
        rx: scalar,
        ry: scalar,
        paint: &Paint,
    ) -> &Self {
        unsafe {
            self.native_mut()
                .drawRoundRect(rect.as_ref().native(), rx, ry, paint.native())
        }
        self
    }

    /// Draws [`Path`] path using clip, [`Matrix`], and [`Paint`] `paint`.
    /// [`Path`] contains an array of path contour, each of which may be open or closed.
    ///
    /// In `paint`: [`crate::paint::Style`] determines if [`RRect`] is stroked or filled:
    /// if filled, [`crate::PathFillType`] determines whether path contour describes inside or
    /// outside of fill; if stroked, [`Paint`] stroke width describes the line thickness,
    /// [`crate::paint::Cap`] describes line ends, and [`crate::paint::Join`] describes how
    /// corners are drawn.
    ///
    /// - `path` [`Path`] to draw
    /// - `paint` stroke, blend, color, and so on, used to draw
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawPath>
    pub fn draw_path(&self, path: &Path, paint: &Paint) -> &Self {
        unsafe { self.native_mut().drawPath(path.native(), paint.native()) }
        self
    }

    pub fn draw_image(
        &self,
        image: impl AsRef<Image>,
        left_top: impl Into<Point>,
        paint: Option<&Paint>,
    ) -> &Self {
        let left_top = left_top.into();
        self.draw_image_with_sampling_options(image, left_top, SamplingOptions::default(), paint)
    }

    pub fn draw_image_rect(
        &self,
        image: impl AsRef<Image>,
        src: Option<(&Rect, SrcRectConstraint)>,
        dst: impl AsRef<Rect>,
        paint: &Paint,
    ) -> &Self {
        self.draw_image_rect_with_sampling_options(
            image,
            src,
            dst,
            SamplingOptions::default(),
            paint,
        )
    }

    pub fn draw_image_with_sampling_options(
        &self,
        image: impl AsRef<Image>,
        left_top: impl Into<Point>,
        sampling: impl Into<SamplingOptions>,
        paint: Option<&Paint>,
    ) -> &Self {
        let left_top = left_top.into();
        unsafe {
            self.native_mut().drawImage(
                image.as_ref().native(),
                left_top.x,
                left_top.y,
                sampling.into().native(),
                paint.native_ptr_or_null(),
            )
        }
        self
    }

    pub fn draw_image_rect_with_sampling_options(
        &self,
        image: impl AsRef<Image>,
        src: Option<(&Rect, SrcRectConstraint)>,
        dst: impl AsRef<Rect>,
        sampling: impl Into<SamplingOptions>,
        paint: &Paint,
    ) -> &Self {
        let sampling = sampling.into();
        match src {
            Some((src, constraint)) => unsafe {
                self.native_mut().drawImageRect(
                    image.as_ref().native(),
                    src.native(),
                    dst.as_ref().native(),
                    sampling.native(),
                    paint.native(),
                    constraint,
                )
            },
            None => unsafe {
                self.native_mut().drawImageRect1(
                    image.as_ref().native(),
                    dst.as_ref().native(),
                    sampling.native(),
                    paint.native(),
                )
            },
        }
        self
    }

    /// Draws [`Image`] `image` stretched proportionally to fit into [`Rect`] `dst`.
    /// [`IRect`] `center` divides the image into nine sections: four sides, four corners, and
    /// the center. Corners are unmodified or scaled down proportionately if their sides
    /// are larger than `dst`; center and four sides are scaled to fit remaining space, if any.
    ///
    /// Additionally transform draw using clip, [`Matrix`], and optional [`Paint`] `paint`.
    ///
    /// If [`Paint`] `paint` is supplied, apply [`crate::ColorFilter`], alpha, [`ImageFilter`], and
    /// [`BlendMode`]. If `image` is [`crate::ColorType::Alpha8`], apply [`Shader`].
    /// If `paint` contains [`crate::MaskFilter`], generate mask from `image` bounds.
    /// Any [`crate::MaskFilter`] on `paint` is ignored as is paint anti-aliasing state.
    ///
    /// If generated mask extends beyond image bounds, replicate image edge colors, just
    /// as [`Shader`] made from [`RCHandle<Image>::to_shader()`] with [`crate::TileMode::Clamp`] set
    /// replicates the image edge color when it samples outside of its bounds.
    ///
    /// - `image` [`Image`] containing pixels, dimensions, and format
    /// - `center` [`IRect`] edge of image corners and sides
    /// - `dst` destination [`Rect`] of image to draw to
    /// - `filter` what technique to use when sampling the image
    /// - `paint` [`Paint`] containing [`BlendMode`], [`crate::ColorFilter`], [`ImageFilter`],
    ///    and so on; or `None`
    pub fn draw_image_nine(
        &self,
        image: impl AsRef<Image>,
        center: impl AsRef<IRect>,
        dst: impl AsRef<Rect>,
        filter_mode: FilterMode,
        paint: Option<&Paint>,
    ) -> &Self {
        unsafe {
            self.native_mut().drawImageNine(
                image.as_ref().native(),
                center.as_ref().native(),
                dst.as_ref().native(),
                filter_mode,
                paint.native_ptr_or_null(),
            )
        }
        self
    }

    /// Draws [`Image`] `image` stretched proportionally to fit into [`Rect`] `dst`.
    ///
    /// [`lattice::Lattice`] lattice divides image into a rectangular grid.
    /// Each intersection of an even-numbered row and column is fixed;
    /// fixed lattice elements never scale larger than their initial
    /// size and shrink proportionately when all fixed elements exceed the bitmap
    /// dimension. All other grid elements scale to fill the available space, if any.
    ///
    /// Additionally transform draw using clip, [`Matrix`], and optional [`Paint`] `paint`.
    ///
    /// If [`Paint`] `paint` is supplied, apply [`crate::ColorFilter`], alpha, [`ImageFilter`], and
    /// [`BlendMode`]. If image is [`crate::ColorType::Alpha8`], apply [`Shader`].
    /// If `paint` contains [`crate::MaskFilter`], generate mask from image bounds.
    /// Any [`crate::MaskFilter`] on `paint` is ignored as is `paint` anti-aliasing state.
    ///
    /// If generated mask extends beyond bitmap bounds, replicate bitmap edge colors,
    /// just as [`Shader`] made from `SkShader::MakeBitmapShader` with
    /// [`crate::TileMode::Clamp`] set replicates the bitmap edge color when it samples
    /// outside of its bounds.
    ///
    /// - `image` [`Image`] containing pixels, dimensions, and format
    /// - `lattice` division of bitmap into fixed and variable rectangles
    /// - `dst` destination [`Rect`] of image to draw to
    /// - `filter` what technique to use when sampling the image
    /// - `paint` [`Paint`] containing [`BlendMode`], [`crate::ColorFilter`], [`ImageFilter`],
    ///   and so on; or `None`
    pub fn draw_image_lattice(
        &self,
        image: impl AsRef<Image>,
        lattice: &Lattice,
        dst: impl AsRef<Rect>,
        filter: FilterMode,
        paint: Option<&Paint>,
    ) -> &Self {
        unsafe {
            self.native_mut().drawImageLattice(
                image.as_ref().native(),
                &lattice.native().native,
                dst.as_ref().native(),
                filter,
                paint.native_ptr_or_null(),
            )
        }
        self
    }

    // TODO: drawSimpleText?

    /// Draws [`String`], with origin at `(origin.x, origin.y)`, using clip, [`Matrix`], [`Font`]
    /// `font`, and [`Paint`] `paint`.
    ///
    /// This function uses the default character-to-glyph mapping from the [`crate::Typeface`] in
    /// font.  It does not perform typeface fallback for characters not found in the
    /// [`crate::Typeface`].  It does not perform kerning; glyphs are positioned based on their
    /// default advances.
    ///
    /// Text size is affected by [`Matrix`] and [`Font`] text size. Default text size is 12 point.
    ///
    /// All elements of `paint`: [`crate::PathEffect`], [`crate::MaskFilter`], [`Shader`],
    /// [`crate::ColorFilter`], and [`ImageFilter`]; apply to text. By default, draws filled black
    /// glyphs.
    ///
    /// - `str` character code points drawn,
    ///    ending with a char value of zero
    /// - `origin` start of string on x,y-axis
    /// - `font` typeface, text size and so, used to describe the text
    /// - `paint` blend, color, and so on, used to draw
    pub fn draw_str(
        &self,
        str: impl AsRef<str>,
        origin: impl Into<Point>,
        font: &Font,
        paint: &Paint,
    ) -> &Self {
        // rust specific, based on drawSimpleText with fixed UTF8 encoding,
        // implementation is similar to Font's *_str methods.
        let origin = origin.into();
        let bytes = str.as_ref().as_bytes();
        unsafe {
            self.native_mut().drawSimpleText(
                bytes.as_ptr() as _,
                bytes.len(),
                TextEncoding::UTF8.into_native(),
                origin.x,
                origin.y,
                font.native(),
                paint.native(),
            )
        }
        self
    }

    /// Draws glyphs at positions relative to `origin` styled with `font` and `paint` with
    /// supporting utf8 and cluster information.
    ///
    /// This function draw glyphs at the given positions relative to the given origin. It does not
    /// perform typeface fallback for glyphs not found in the [`crate::Typeface`] in font.
    ///
    /// The drawing obeys the current transform matrix and clipping.
    ///
    /// All elements of paint: [`crate::PathEffect`], [`crate::MaskFilter`], [`Shader`],
    /// [`crate::ColorFilter`], and [`ImageFilter`]; apply to text. By default, draws filled black
    /// glyphs.
    ///
    /// - `count`           number of glyphs to draw
    /// - `glyphs`          the array of glyphIDs to draw
    /// - `positions`       where to draw each glyph relative to origin
    /// - `clusters`        array of size count of cluster information
    /// - `utf8_text`       utf8text supporting information for the glyphs
    /// - `origin`          the origin of all the positions
    /// - `font`            typeface, text size and so, used to describe the text
    /// - `paint`           blend, color, and so on, used to draw
    #[allow(clippy::too_many_arguments)]
    pub fn draw_glyphs_utf8(
        &self,
        glyphs: &[GlyphId],
        positions: &[Point],
        clusters: &[u32],
        utf8_text: impl AsRef<str>,
        origin: impl Into<Point>,
        font: &Font,
        paint: &Paint,
    ) {
        let count = glyphs.len();
        if count == 0 {
            return;
        }
        assert_eq!(positions.len(), count);
        assert_eq!(clusters.len(), count);
        let utf8_text = utf8_text.as_ref().as_bytes();
        let origin = origin.into();
        unsafe {
            sb::C_SkCanvas_drawGlyphs2(
                self.native_mut(),
                glyphs.as_ptr(),
                count,
                positions.native().as_ptr(),
                clusters.as_ptr(),
                utf8_text.as_ptr() as _,
                utf8_text.len(),
                origin.into_native(),
                font.native(),
                paint.native(),
            )
        }
    }

    /// Draws `count` glyphs, at positions relative to `origin` styled with `font` and `paint`.
    ///
    /// This function draw glyphs at the given positions relative to the given origin.
    /// It does not perform typeface fallback for glyphs not found in the [`crate::Typeface`]] in
    /// font.
    ///
    /// The drawing obeys the current transform matrix and clipping.
    ///
    /// All elements of paint: [`crate::PathEffect`], [`crate::MaskFilter`], [`Shader`],
    /// [`crate::ColorFilter`], and [`ImageFilter`]; apply to text. By default, draws filled black
    /// glyphs.
    ///
    /// - `count`       number of glyphs to draw
    /// - `glyphs`      the array of glyphIDs to draw
    /// - `positions`   where to draw each glyph relative to origin, either a `&[Point]` or
    ///                `&[RSXform]` slice
    /// - `origin`      the origin of all the positions
    /// - `font`        typeface, text size and so, used to describe the text
    /// - `paint`       blend, color, and so on, used to draw
    pub fn draw_glyphs_at<'a>(
        &self,
        glyphs: &[GlyphId],
        positions: impl Into<GlyphPositions<'a>>,
        origin: impl Into<Point>,
        font: &Font,
        paint: &Paint,
    ) {
        let count = glyphs.len();
        if count == 0 {
            return;
        }
        let positions: GlyphPositions = positions.into();
        let origin = origin.into();

        let glyphs = glyphs.as_ptr();
        let origin = origin.into_native();
        let font = font.native();
        let paint = paint.native();

        match positions {
            GlyphPositions::Points(points) => {
                assert_eq!(points.len(), count);
                unsafe {
                    sb::C_SkCanvas_drawGlyphs(
                        self.native_mut(),
                        glyphs,
                        count,
                        points.native().as_ptr(),
                        origin,
                        font,
                        paint,
                    )
                }
            }
            GlyphPositions::RSXforms(xforms) => {
                assert_eq!(xforms.len(), count);
                unsafe {
                    sb::C_SkCanvas_drawGlyphsRSXform(
                        self.native_mut(),
                        glyphs,
                        count,
                        xforms.native().as_ptr(),
                        origin,
                        font,
                        paint,
                    )
                }
            }
        }
    }

    /// Draws [`TextBlob`] blob at `(origin.x, origin.y)`, using clip, [`Matrix`], and [`Paint`]
    /// paint.
    ///
    /// `blob` contains glyphs, their positions, and paint attributes specific to text:
    /// [`crate::Typeface`], [`Paint`] text size, [`Paint`] text scale x, [`Paint`] text skew x,
    /// [`Paint`] align, [`Paint`] hinting, anti-alias, [`Paint`] fake bold, [`Paint`] font embedded
    /// bitmaps, [`Paint`] full hinting spacing, LCD text, [`Paint`] linear text, and [`Paint`]
    /// subpixel text.
    ///
    /// [`TextEncoding`] must be set to [`TextEncoding::GlyphId`].
    ///
    /// Elements of `paint`: [`crate::PathEffect`], [`crate::MaskFilter`], [`Shader`],
    /// [`crate::ColorFilter`], and [`ImageFilter`]; apply to blob.
    ///
    /// - `blob` glyphs, positions, and their paints' text size, typeface, and so on
    /// - `origin` horizontal and vertical offset applied to blob
    /// - `paint` blend, color, stroking, and so on, used to draw
    pub fn draw_text_blob(
        &self,
        blob: impl AsRef<TextBlob>,
        origin: impl Into<Point>,
        paint: &Paint,
    ) -> &Self {
        let origin = origin.into();
        #[cfg(all(feature = "textlayout", feature = "embed-icudtl"))]
        crate::icu::init();
        unsafe {
            self.native_mut().drawTextBlob(
                blob.as_ref().native(),
                origin.x,
                origin.y,
                paint.native(),
            )
        }
        self
    }

    /// Draws [`Picture`] picture, using clip and [`Matrix`]; transforming picture with
    /// [`Matrix`] matrix, if provided; and use [`Paint`] `paint` alpha, [`crate::ColorFilter`],
    /// [`ImageFilter`], and [`BlendMode`], if provided.
    ///
    /// If paint is not `None`, then the picture is always drawn into a temporary layer before
    /// actually landing on the canvas. Note that drawing into a layer can also change its
    /// appearance if there are any non-associative blend modes inside any of the pictures elements.
    ///
    /// - `picture` recorded drawing commands to play
    /// - `matrix` [`Matrix`] to rotate, scale, translate, and so on; may be `None`
    /// - `paint` [`Paint`] to apply transparency, filtering, and so on; may be `None`
    pub fn draw_picture(
        &self,
        picture: impl AsRef<Picture>,
        matrix: Option<&Matrix>,
        paint: Option<&Paint>,
    ) -> &Self {
        unsafe {
            self.native_mut().drawPicture(
                picture.as_ref().native(),
                matrix.native_ptr_or_null(),
                paint.native_ptr_or_null(),
            )
        }
        self
    }

    /// Draws [`Vertices`] vertices, a triangle mesh, using clip and [`Matrix`].
    /// If `paint` contains an [`Shader`] and vertices does not contain tex coords, the shader is
    /// mapped using the vertices' positions.
    ///
    /// [`BlendMode`] is ignored if [`Vertices`] does not have colors. Otherwise, it combines
    ///   - the [`Shader`] if [`Paint`] contains [`Shader`
    ///   - or the opaque [`Paint`] color if [`Paint`] does not contain [`Shader`]
    ///
    /// as the src of the blend and the interpolated vertex colors as the dst.
    ///
    /// [`crate::MaskFilter`], [`crate::PathEffect`], and antialiasing on [`Paint`] are ignored.
    //
    /// - `vertices` triangle mesh to draw
    /// - `mode` combines vertices' colors with [`Shader`] if present or [`Paint`] opaque color if
    ///   not. Ignored if the vertices do not contain color.
    /// - `paint` specifies the [`Shader`], used as [`Vertices`] texture, and
    ///   [`crate::ColorFilter`].
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawVertices>
    /// example: <https://fiddle.skia.org/c/@Canvas_drawVertices_2>
    pub fn draw_vertices(&self, vertices: &Vertices, mode: BlendMode, paint: &Paint) -> &Self {
        unsafe {
            self.native_mut()
                .drawVertices(vertices.native(), mode, paint.native())
        }
        self
    }

    /// Draws a Coons patch: the interpolation of four cubics with shared corners,
    /// associating a color, and optionally a texture [`Point`], with each corner.
    ///
    /// [`Point`] array cubics specifies four [`Path`] cubic starting at the top-left corner,
    /// in clockwise order, sharing every fourth point. The last [`Path`] cubic ends at the
    /// first point.
    ///
    /// Color array color associates colors with corners in top-left, top-right,
    /// bottom-right, bottom-left order.
    ///
    /// If paint contains [`Shader`], [`Point`] array `tex_coords` maps [`Shader`] as texture to
    /// corners in top-left, top-right, bottom-right, bottom-left order. If `tex_coords` is
    /// `None`, [`Shader`] is mapped using positions (derived from cubics).
    ///
    /// [`BlendMode`] is ignored if colors is `None`. Otherwise, it combines
    ///   - the [`Shader`] if [`Paint`] contains [`Shader`]
    ///   - or the opaque [`Paint`] color if [`Paint`] does not contain [`Shader`]
    ///
    /// as the src of the blend and the interpolated patch colors as the dst.
    ///
    /// [`crate::MaskFilter`], [`crate::PathEffect`], and antialiasing on [`Paint`] are ignored.
    ///
    /// - `cubics` [`Path`] cubic array, sharing common points
    /// - `colors` color array, one for each corner
    /// - `tex_coords` [`Point`] array of texture coordinates, mapping [`Shader`] to corners;
    ///   may be `None`
    /// - `mode` combines patch's colors with [`Shader`] if present or [`Paint`] opaque color if
    ///    not. Ignored if colors is `None`.
    /// - `paint` [`Shader`], [`crate::ColorFilter`], [`BlendMode`], used to draw
    pub fn draw_patch<'a>(
        &self,
        cubics: &[Point; 12],
        colors: impl Into<Option<&'a [Color; 4]>>,
        tex_coords: Option<&[Point; 4]>,
        mode: BlendMode,
        paint: &Paint,
    ) -> &Self {
        let colors = colors
            .into()
            .map(|c| c.native().as_ptr())
            .unwrap_or(ptr::null());
        unsafe {
            self.native_mut().drawPatch(
                cubics.native().as_ptr(),
                colors,
                tex_coords
                    .map(|tc| tc.native().as_ptr())
                    .unwrap_or(ptr::null()),
                mode,
                paint.native(),
            )
        }
        self
    }

    /// Draws a set of sprites from atlas, using clip, [`Matrix`], and optional [`Paint`] paint.
    /// paint uses anti-alias, alpha, [`crate::ColorFilter`], [`ImageFilter`], and [`BlendMode`]
    /// to draw, if present. For each entry in the array, [`Rect`] tex locates sprite in
    /// atlas, and [`RSXform`] xform transforms it into destination space.
    ///
    /// [`crate::MaskFilter`] and [`crate::PathEffect`] on paint are ignored.
    ///
    /// xform, tex, and colors if present, must contain the same number of entries.
    /// Optional colors are applied for each sprite using [`BlendMode`] mode, treating
    /// sprite as source and colors as destination.
    /// Optional `cull_rect` is a conservative bounds of all transformed sprites.
    /// If `cull_rect` is outside of clip, canvas can skip drawing.
    ///
    /// * `atlas` - [`Image`] containing sprites
    /// * `xform` - [`RSXform`] mappings for sprites in atlas
    /// * `tex` - [`Rect`] locations of sprites in atlas
    /// * `colors` - one per sprite, blended with sprite using [`BlendMode`]; may be `None`
    /// * `count` - number of sprites to draw
    /// * `mode` - [`BlendMode`] combining colors and sprites
    /// * `sampling` - [`SamplingOptions`] used when sampling from the atlas image
    /// * `cull_rect` - bounds of transformed sprites for efficient clipping; may be `None`
    /// * `paint` - [`crate::ColorFilter`], [`ImageFilter`], [`BlendMode`], and so on; may be `None`
    #[allow(clippy::too_many_arguments)]
    pub fn draw_atlas<'a>(
        &self,
        atlas: &Image,
        xform: &[RSXform],
        tex: &[Rect],
        colors: impl Into<Option<&'a [Color]>>,
        mode: BlendMode,
        sampling: impl Into<SamplingOptions>,
        cull_rect: impl Into<Option<Rect>>,
        paint: impl Into<Option<&'a Paint>>,
    ) {
        let count = xform.len();
        assert_eq!(tex.len(), count);
        let colors = colors.into();
        if let Some(color_slice) = colors {
            assert_eq!(color_slice.len(), count);
        }
        unsafe {
            sb::C_SkCanvas_drawAtlas(
                self.native_mut(),
                atlas.native(),
                xform.native().as_ptr(),
                count,
                tex.native().as_ptr(),
                colors.native().as_ptr_or_null(),
                mode,
                sampling.into().native(),
                cull_rect.into().native().as_ptr_or_null(),
                paint.into().native_ptr_or_null(),
            )
        }
    }

    /// Draws [`Drawable`] drawable using clip and [`Matrix`], concatenated with
    /// optional matrix.
    ///
    /// If [`Canvas`] has an asynchronous implementation, as is the case when it is recording into
    /// [`Picture`], then drawable will be referenced, so that [`RCHandle<Drawable>::draw()`] can be
    /// called when the operation is finalized. To force immediate drawing, call
    /// [`RCHandle<Drawable>::draw()`] instead.
    ///
    /// - `drawable` custom struct encapsulating drawing commands
    /// - `matrix` transformation applied to drawing; may be `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawDrawable>
    pub fn draw_drawable(&self, drawable: &mut Drawable, matrix: Option<&Matrix>) {
        unsafe {
            self.native_mut()
                .drawDrawable(drawable.native_mut(), matrix.native_ptr_or_null())
        }
    }

    /// Draws [`Drawable`] drawable using clip and [`Matrix`], offset by `(offset.x, offset.y)`.
    ///
    /// If [`Canvas`] has an asynchronous implementation, as is the case when it is recording into
    /// [`Picture`], then drawable will be referenced, so that [`RCHandle<Drawable>::draw()`] can be
    /// called when the operation is finalized. To force immediate drawing, call
    /// [`RCHandle<Drawable>::draw()`] instead.
    ///
    /// - `drawable` custom struct encapsulating drawing commands
    /// - `offset` offset into [`Canvas`] writable pixels on x,y-axis
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_drawDrawable_2>
    pub fn draw_drawable_at(&self, drawable: &mut Drawable, offset: impl Into<Point>) {
        let offset = offset.into();
        unsafe {
            self.native_mut()
                .drawDrawable1(drawable.native_mut(), offset.x, offset.y)
        }
    }

    /// Associates [`Rect`] on [`Canvas`] when an annotation; a key-value pair, where the key is
    /// a UTF-8 string, and optional value is stored as [`Data`].
    ///
    /// Only some canvas implementations, such as recording to [`Picture`], or drawing to
    /// document PDF, use annotations.
    ///
    /// - `rect` [`Rect`] extent of canvas to annotate
    /// - `key` string used for lookup
    /// - `value` data holding value stored in annotation
    pub fn draw_annotation(&self, rect: impl AsRef<Rect>, key: &str, value: &Data) -> &Self {
        let key = CString::new(key).unwrap();
        unsafe {
            self.native_mut().drawAnnotation(
                rect.as_ref().native(),
                key.as_ptr(),
                value.native_mut_force(),
            )
        }
        self
    }

    /// Returns `true` if clip is empty; that is, nothing will draw.
    ///
    /// May do work when called; it should not be called more often than needed. However, once
    /// called, subsequent calls perform no work until clip changes.
    ///
    /// Returns `true` if clip is empty
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_isClipEmpty>
    pub fn is_clip_empty(&self) -> bool {
        unsafe { sb::C_SkCanvas_isClipEmpty(self.native()) }
    }

    /// Returns `true` if clip is [`Rect`] and not empty.
    /// Returns `false` if the clip is empty, or if it is not [`Rect`].
    ///
    /// Returns `true` if clip is [`Rect`] and not empty
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_isClipRect>
    pub fn is_clip_rect(&self) -> bool {
        unsafe { sb::C_SkCanvas_isClipRect(self.native()) }
    }

    /// Returns the current transform from local coordinates to the 'device', which for most
    /// purposes means pixels.
    ///
    /// Returns transformation from local coordinates to device / pixels.
    pub fn local_to_device(&self) -> M44 {
        M44::construct(|m| unsafe { sb::C_SkCanvas_getLocalToDevice(self.native(), m) })
    }

    /// Throws away the 3rd row and column in the matrix, so be warned.
    pub fn local_to_device_as_3x3(&self) -> Matrix {
        self.local_to_device().to_m33()
    }

    /// DEPRECATED
    /// Legacy version of [`Self::local_to_device()`], which strips away any Z information, and just
    /// returns a 3x3 version.
    ///
    /// Returns 3x3 version of [`Self::local_to_device()`]
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_getTotalMatrix>
    /// example: <https://fiddle.skia.org/c/@Clip>
    #[deprecated(
        since = "0.38.0",
        note = "use local_to_device() or local_to_device_as_3x3() instead"
    )]
    pub fn total_matrix(&self) -> Matrix {
        let mut matrix = Matrix::default();
        unsafe { sb::C_SkCanvas_getTotalMatrix(self.native(), matrix.native_mut()) };
        matrix
    }

    //
    // internal helper
    //

    pub(crate) fn own_from_native_ptr<'lt>(native: *mut SkCanvas) -> Option<OwnedCanvas<'lt>> {
        if !native.is_null() {
            Some(OwnedCanvas::<'lt>(
                ptr::NonNull::new(
                    Self::borrow_from_native(unsafe { &*native }) as *const _ as *mut _
                )
                .unwrap(),
                PhantomData,
            ))
        } else {
            None
        }
    }

    pub(crate) fn borrow_from_native(native: &SkCanvas) -> &Self {
        unsafe { transmute_ref(native) }
    }
}

impl QuickReject<Rect> for Canvas {
    /// Returns `true` if [`Rect`] `rect`, transformed by [`Matrix`], can be quickly determined to
    /// be outside of clip. May return `false` even though rect is outside of clip.
    ///
    /// Use to check if an area to be drawn is clipped out, to skip subsequent draw calls.
    ///
    /// - `rect` [`Rect`] to compare with clip
    ///
    /// Returns `true` if `rect`, transformed by [`Matrix`], does not intersect clip
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_quickReject>
    fn quick_reject(&self, rect: &Rect) -> bool {
        unsafe { self.native().quickReject(rect.native()) }
    }
}

impl QuickReject<Path> for Canvas {
    /// Returns `true` if `path`, transformed by [`Matrix`], can be quickly determined to be
    /// outside of clip. May return `false` even though `path` is outside of clip.
    ///
    /// Use to check if an area to be drawn is clipped out, to skip subsequent draw calls.
    ///
    /// - `path` [`Path`] to compare with clip
    ///
    /// Returns `true` if `path`, transformed by [`Matrix`], does not intersect clip
    ///
    /// example: <https://fiddle.skia.org/c/@Canvas_quickReject_2>
    fn quick_reject(&self, path: &Path) -> bool {
        unsafe { self.native().quickReject1(path.native()) }
    }
}

pub trait SetMatrix {
    /// DEPRECATED -- use [`M44`] version
    #[deprecated(since = "0.38.0", note = "Use M44 version")]
    fn set_matrix(&self, matrix: &Matrix) -> &Self;
}

impl SetMatrix for Canvas {
    /// DEPRECATED -- use [`M44`] version
    fn set_matrix(&self, matrix: &Matrix) -> &Self {
        unsafe { self.native_mut().setMatrix1(matrix.native()) }
        self
    }
}

//
// Lattice
//

pub mod lattice {
    use crate::{prelude::*, Color, IRect};
    use skia_bindings::{self as sb, SkCanvas_Lattice};
    use std::marker::PhantomData;

    /// [`Lattice`] divides [`crate::Bitmap`] or [`crate::Image`] into a rectangular grid.
    /// Grid entries on even columns and even rows are fixed; these entries are
    /// always drawn at their original size if the destination is large enough.
    /// If the destination side is too small to hold the fixed entries, all fixed
    /// entries are proportionately scaled down to fit.
    /// The grid entries not on even columns and rows are scaled to fit the
    /// remaining space, if any.
    #[derive(Debug)]
    pub struct Lattice<'a> {
        /// x-axis values dividing bitmap
        pub x_divs: &'a [i32],
        /// y-axis values dividing bitmap
        pub y_divs: &'a [i32],
        /// array of fill types
        pub rect_types: Option<&'a [RectType]>,
        /// source bounds to draw from
        pub bounds: Option<IRect>,
        /// array of colors
        pub colors: Option<&'a [Color]>,
    }

    #[derive(Debug)]
    pub(crate) struct Ref<'a> {
        pub native: SkCanvas_Lattice,
        pd: PhantomData<&'a Lattice<'a>>,
    }

    impl Lattice<'_> {
        pub(crate) fn native(&self) -> Ref {
            if let Some(rect_types) = self.rect_types {
                let rect_count = (self.x_divs.len() + 1) * (self.y_divs.len() + 1);
                assert_eq!(rect_count, rect_types.len());
                // even though rect types may not include any FixedColor refs,
                // we expect the colors slice with a proper size here, this
                // saves us for going over the types array and looking for FixedColor
                // entries.
                assert_eq!(rect_count, self.colors.unwrap().len());
            }

            let native = SkCanvas_Lattice {
                fXDivs: self.x_divs.as_ptr(),
                fYDivs: self.y_divs.as_ptr(),
                fRectTypes: self.rect_types.as_ptr_or_null(),
                fXCount: self.x_divs.len().try_into().unwrap(),
                fYCount: self.y_divs.len().try_into().unwrap(),
                fBounds: self.bounds.native().as_ptr_or_null(),
                fColors: self.colors.native().as_ptr_or_null(),
            };
            Ref {
                native,
                pd: PhantomData,
            }
        }
    }

    /// Optional setting per rectangular grid entry to make it transparent,
    /// or to fill the grid entry with a color.
    pub use sb::SkCanvas_Lattice_RectType as RectType;
    variant_name!(RectType::FixedColor);
}

#[must_use]
#[derive(Debug)]
/// Stack helper class calls [`Canvas::restore_to_count()`] when [`AutoCanvasRestore`]
/// goes out of scope. Use this to guarantee that the canvas is restored to a known
/// state.
pub struct AutoRestoredCanvas<'a> {
    canvas: &'a Canvas,
    restore: SkAutoCanvasRestore,
}

impl Deref for AutoRestoredCanvas<'_> {
    type Target = Canvas;
    fn deref(&self) -> &Self::Target {
        self.canvas
    }
}

impl NativeAccess for AutoRestoredCanvas<'_> {
    type Native = SkAutoCanvasRestore;

    fn native(&self) -> &SkAutoCanvasRestore {
        &self.restore
    }

    fn native_mut(&mut self) -> &mut SkAutoCanvasRestore {
        &mut self.restore
    }
}

impl Drop for AutoRestoredCanvas<'_> {
    /// Restores [`Canvas`] to saved state. Drop is called when container goes out of scope.
    fn drop(&mut self) {
        unsafe { sb::C_SkAutoCanvasRestore_destruct(self.native_mut()) }
    }
}

impl AutoRestoredCanvas<'_> {
    /// Restores [`Canvas`] to saved state immediately. Subsequent calls and [`Self::drop()`] have
    /// no effect.
    pub fn restore(&mut self) {
        unsafe { sb::C_SkAutoCanvasRestore_restore(self.native_mut()) }
    }
}

pub enum AutoCanvasRestore {}

impl AutoCanvasRestore {
    // TODO: rename to save(), add a method to Canvas, perhaps named auto_restored()?
    /// Preserves [`Canvas::save()`] count. Optionally saves [`Canvas`] clip and [`Canvas`] matrix.
    ///
    /// - `canvas` [`Canvas`] to guard
    /// - `do_save` call [`Canvas::save()`]
    ///
    /// Returns utility to restore [`Canvas`] state on destructor
    pub fn guard(canvas: &Canvas, do_save: bool) -> AutoRestoredCanvas {
        let restore = construct(|acr| unsafe {
            sb::C_SkAutoCanvasRestore_Construct(acr, canvas.native_mut(), do_save)
        });

        AutoRestoredCanvas { canvas, restore }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        canvas::SaveLayerFlags, canvas::SaveLayerRec, surfaces, AlphaType, Canvas, ClipOp, Color,
        ColorType, ImageInfo, OwnedCanvas, Rect,
    };

    #[test]
    fn test_raster_direct_creation_and_clear_in_memory() {
        let info = ImageInfo::new((2, 2), ColorType::RGBA8888, AlphaType::Unpremul, None);
        assert_eq!(8, info.min_row_bytes());
        let mut bytes: [u8; 8 * 2] = Default::default();
        {
            let canvas = Canvas::from_raster_direct(&info, bytes.as_mut(), None, None).unwrap();
            canvas.clear(Color::RED);
        }

        assert_eq!(0xff, bytes[0]);
        assert_eq!(0x00, bytes[1]);
        assert_eq!(0x00, bytes[2]);
        assert_eq!(0xff, bytes[3]);
    }

    #[test]
    fn test_raster_direct_n32_creation_and_clear_in_memory() {
        let mut pixels: [u32; 4] = Default::default();
        {
            let canvas = Canvas::from_raster_direct_n32((2, 2), pixels.as_mut(), None).unwrap();
            canvas.clear(Color::RED);
        }

        // TODO: equals to 0xff0000ff on macOS, but why? Endianness should be the same.
        // assert_eq!(0xffff0000, pixels[0]);
    }

    #[test]
    fn test_empty_canvas_creation() {
        let canvas = OwnedCanvas::default();
        drop(canvas)
    }

    #[test]
    fn test_save_layer_rec_lifetimes() {
        let rect = Rect::default();
        {
            let _rec = SaveLayerRec::default()
                .flags(SaveLayerFlags::PRESERVE_LCD_TEXT)
                .bounds(&rect);
        }
    }

    #[test]
    fn test_make_surface() {
        let mut pixels: [u32; 4] = Default::default();
        let canvas = Canvas::from_raster_direct_n32((2, 2), pixels.as_mut(), None).unwrap();
        let ii = canvas.image_info();
        let mut surface = canvas.new_surface(&ii, None).unwrap();
        dbg!(&canvas as *const _);
        drop(canvas);

        let canvas = surface.canvas();
        dbg!(canvas as *const _);
        canvas.clear(Color::RED);
    }

    #[test]
    fn clip_options_overloads() {
        let c = OwnedCanvas::default();
        // do_anti_alias
        c.clip_rect(Rect::default(), None, true);
        // clip_op
        c.clip_rect(Rect::default(), ClipOp::Difference, None);
        // both
        c.clip_rect(Rect::default(), ClipOp::Difference, true);
    }

    /// Regression test for: <https://github.com/rust-skia/rust-skia/issues/427>
    #[test]
    fn test_local_and_device_clip_bounds() {
        let mut surface =
            surfaces::raster(&ImageInfo::new_n32_premul((100, 100), None), 0, None).unwrap();
        let _ = surface.canvas().device_clip_bounds();
        let _ = surface.canvas().local_clip_bounds();
        let _ = surface.canvas().local_to_device();
    }
}
