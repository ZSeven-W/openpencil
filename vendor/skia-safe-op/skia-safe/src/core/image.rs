use crate::{
    gpu, prelude::*, AlphaType, Bitmap, ColorSpace, ColorType, Data, EncodedImageFormat, IPoint,
    IRect, ISize, ImageFilter, ImageGenerator, ImageInfo, Matrix, Paint, Picture, Pixmap, Recorder,
    SamplingOptions, Shader, SurfaceProps, TextureCompressionType, TileMode,
};
use skia_bindings::{self as sb, SkImage, SkRefCntBase};
use std::{fmt, ptr};

pub use super::CubicResampler;

#[deprecated(since = "0.62.0", note = "Use TextureCompressionType")]
pub use crate::TextureCompressionType as CompressionType;

#[deprecated(since = "0.63.0", note = "Use images::BitDepth")]
pub use images::BitDepth;

pub mod images {
    use std::{mem, ptr};

    use skia_bindings as sb;

    #[allow(unused)] // doc only
    use crate::ColorType;
    use crate::{
        prelude::*, AlphaType, Bitmap, ColorSpace, Data, IPoint, IRect, ISize, Image, ImageFilter,
        ImageGenerator, ImageInfo, Matrix, Paint, Picture, SurfaceProps, TextureCompressionType,
    };

    /// Creates a CPU-backed [`Image`] from `bitmap`, sharing or copying `bitmap` pixels. If the bitmap
    /// is marked immutable, and its pixel memory is shareable, it may be shared
    /// instead of copied.
    ///
    /// [`Image`] is returned if bitmap is valid. Valid [`Bitmap`] parameters include:
    /// dimensions are greater than zero;
    /// each dimension fits in 29 bits;
    /// [`ColorType`] and [`AlphaType`] are valid, and [`ColorType`] is not [`ColorType::Unknown`];
    /// row bytes are large enough to hold one row of pixels;
    /// pixel address is not `None`.
    ///
    /// * `bitmap` - [`ImageInfo`], row bytes, and pixels
    ///
    /// Returns: created [`Image`], or `None`
    pub fn raster_from_bitmap(bitmap: &Bitmap) -> Option<Image> {
        Image::from_ptr(unsafe { sb::C_SkImages_RasterFromBitmap(bitmap.native()) })
    }

    /// Creates a CPU-backed [`Image`] from compressed data.
    ///
    /// This method will decompress the compressed data and create an image wrapping
    /// it. Any mipmap levels present in the compressed data are discarded.
    ///
    /// * `data` - compressed data to store in [`Image`]
    /// * `dimension` - width and height of full [`Image`]
    /// * `ty` - type of compression used
    ///
    /// Returns: created [`Image`], or `None`
    pub fn raster_from_compressed_texture_data(
        data: impl Into<Data>,
        dimensions: impl Into<ISize>,
        ty: TextureCompressionType,
    ) -> Option<Image> {
        let dimensions = dimensions.into();
        Image::from_ptr(unsafe {
            sb::C_SkImages_RasterFromCompressedTextureData(
                data.into().into_ptr(),
                dimensions.width,
                dimensions.height,
                ty,
            )
        })
    }

    /// Return a [`Image`] using the encoded data, but attempts to defer decoding until the
    /// image is actually used/drawn. This deferral allows the system to cache the result, either on the
    /// CPU or on the GPU, depending on where the image is drawn. If memory is low, the cache may
    /// be purged, causing the next draw of the image to have to re-decode.
    ///
    /// If `alpha_type` is `None`, the image's alpha type will be chosen automatically based on the
    /// image format. Transparent images will default to [`AlphaType::Premul`]. If `alpha_type` contains
    /// [`AlphaType::Premul`] or [`AlphaType::Unpremul`], that alpha type will be used. Forcing opaque
    /// (passing [`AlphaType::Opaque`]) is not allowed, and will return `None`.
    ///
    /// If the encoded format is not supported, `None` is returned.
    ///
    /// * `encoded` - the encoded data
    ///
    /// Returns: created [`Image`], or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_DeferredFromEncodedData>
    pub fn deferred_from_encoded_data(
        data: impl Into<Data>,
        alpha_type: impl Into<Option<AlphaType>>,
    ) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImages_DeferredFromEncodedData(
                data.into().into_ptr(),
                alpha_type
                    .into()
                    .map(|at| &at as *const _)
                    .unwrap_or(ptr::null()),
            )
        })
    }

    /// Creates [`Image`] from data returned by `image_generator`. The image data will not be created
    /// (on either the CPU or GPU) until the image is actually drawn.
    /// Generated data is owned by [`Image`] and may not be shared or accessed.
    ///
    /// [`Image`] is returned if generator data is valid. Valid data parameters vary by type of data
    /// and platform.
    ///
    /// `image_generator` may wrap [`Picture`] data, codec data, or custom data.
    ///
    /// * `image_generator` - stock or custom routines to retrieve [`Image`]
    ///
    /// Returns: created [`Image`], or `None`
    pub fn deferred_from_generator(mut image_generator: ImageGenerator) -> Option<Image> {
        let image = Image::from_ptr(unsafe {
            sb::C_SkImages_DeferredFromGenerator(image_generator.native_mut())
        });
        mem::forget(image_generator);
        image
    }

    pub use skia_bindings::SkImages_BitDepth as BitDepth;
    variant_name!(BitDepth::F16);

    /// Creates [`Image`] from picture. Returned [`Image`] width and height are set by dimensions.
    /// [`Image`] draws picture with matrix and paint, set to `bit_depth` and `color_space`.
    ///
    /// The Picture data is not turned into an image (CPU or GPU) until it is drawn.
    ///
    /// If matrix is `None`, draws with identity [`Matrix`]. If paint is `None`, draws
    /// with default [`Paint`]. `color_space` may be `None`.
    ///
    /// * `picture` - stream of drawing commands
    /// * `dimensions` - width and height
    /// * `matrix` - [`Matrix`] to rotate, scale, translate, and so on; may be `None`
    /// * `paint` - [`Paint`] to apply transparency, filtering, and so on; may be `None`
    /// * `bit_depth` - 8-bit integer or 16-bit float: per component
    /// * `color_space` - range of colors; may be `None`
    /// * `props` - props to use when rasterizing the picture
    ///
    /// Returns: created [`Image`], or `None`
    pub fn deferred_from_picture(
        picture: impl Into<Picture>,
        dimensions: impl Into<ISize>,
        matrix: Option<&Matrix>,
        paint: Option<&Paint>,
        bit_depth: BitDepth,
        color_space: impl Into<Option<ColorSpace>>,
        props: impl Into<Option<SurfaceProps>>,
    ) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImages_DeferredFromPicture(
                picture.into().into_ptr(),
                dimensions.into().native(),
                matrix.native_ptr_or_null(),
                paint.native_ptr_or_null(),
                bit_depth,
                color_space.into().into_ptr_or_null(),
                props.into().unwrap_or_default().native(),
            )
        })
    }

    // TODO: RasterFromPixmapCopy
    // TODO: RasterFromPixmap

    /// Creates CPU-backed [`Image`] from pixel data described by info.
    /// The pixels data will *not* be copied.
    ///
    /// [`Image`] is returned if [`ImageInfo`] is valid. Valid [`ImageInfo`] parameters include:
    /// dimensions are greater than zero;
    /// each dimension fits in 29 bits;
    /// [`ColorType`] and [`AlphaType`] are valid, and [`ColorType`] is not [`ColorType::Unknown`];
    /// `row_bytes` are large enough to hold one row of pixels;
    /// pixels is not `None`, and contains enough data for [`Image`].
    ///
    /// * `info` - contains width, height, [`AlphaType`], [`ColorType`], [`ColorSpace`]
    /// * `pixels` - address or pixel storage
    /// * `row_bytes` - size of pixel row or larger
    ///
    /// Returns: [`Image`] sharing pixels, or `None`
    pub fn raster_from_data(
        info: &ImageInfo,
        pixels: impl Into<Data>,
        row_bytes: usize,
    ) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImages_RasterFromData(info.native(), pixels.into().into_ptr(), row_bytes)
        })
    }

    /// Creates a filtered [`Image`] on the CPU. filter processes the src image, potentially
    /// changing the color, position, and size. subset is the bounds of src that are processed by
    /// filter. `clip_bounds` is the expected bounds of the filtered [`Image`]. `out_subset` is
    /// required storage for the actual bounds of the filtered [`Image`]. `offset` is required
    /// storage for translation of returned [`Image`].
    ///
    /// Returns `None` a filtered result could not be created.
    ///
    /// Useful for animation of [`ImageFilter`] that varies size from frame to frame. `out_subset`
    /// describes the valid bounds of returned image. offset translates the returned [`Image`] to
    /// keep subsequent animation frames aligned with respect to each other.
    ///
    /// * `src` - the image to be filtered
    /// * `filter` - the image filter to be applied
    /// * `subset` - bounds of [`Image`] processed by filter
    /// * `clip_bounds` - expected bounds of filtered [`Image`]
    ///
    /// Returns filtered SkImage, or `None`:
    /// * `out_subset` - storage for returned [`Image`] bounds
    /// * `offset` - storage for returned [`Image`] translation Returns: filtered [`Image`], or
    ///   `None`
    pub fn make_with_filter(
        image: impl Into<Image>,
        image_filter: &ImageFilter,
        subset: impl AsRef<IRect>,
        clip_bounds: impl AsRef<IRect>,
    ) -> Option<(Image, IRect, IPoint)> {
        let mut out_subset = IRect::default();
        let mut offset = IPoint::default();

        unsafe {
            Image::from_ptr(sb::C_SkImages_MakeWithFilter(
                image.into().into_ptr(),
                image_filter.native(),
                subset.as_ref().native(),
                clip_bounds.as_ref().native(),
                out_subset.native_mut(),
                offset.native_mut(),
            ))
        }
        .map(|i| (i, out_subset, offset));
        None
    }
}

/// CachingHint selects whether Skia may internally cache [`Bitmap`] generated by
/// decoding [`Image`], or by copying [`Image`] from GPU to CPU. The default behavior
/// allows caching [`Bitmap`].
///
/// Choose [`CachingHint::Disallow`] if [`Image`] pixels are to be used only once, or
/// if [`Image`] pixels reside in a cache outside of Skia, or to reduce memory pressure.
///
/// Choosing [`CachingHint::Allow`] does not ensure that pixels will be cached.
/// [`Image`] pixels may not be cached if memory requirements are too large or
/// pixels are not accessible.
pub use skia_bindings::SkImage_CachingHint as CachingHint;
variant_name!(CachingHint::Allow);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(C)]
pub struct RequiredProperties {
    pub mipmapped: bool,
}

native_transmutable!(sb::SkImage_RequiredProperties, RequiredProperties);

/// [`Image`] describes a two dimensional array of pixels to draw. The pixels may be
/// decoded in a raster bitmap, encoded in a [`Picture`] or compressed data stream,
/// or located in GPU memory as a GPU texture.
///
/// [`Image`] cannot be modified after it is created. [`Image`] may allocate additional
/// storage as needed; for instance, an encoded [`Image`] may decode when drawn.
///
/// [`Image`] width and height are greater than zero. Creating an [`Image`] with zero width
/// or height returns [`Image`] equal to nullptr.
///
/// [`Image`] may be created from [`Bitmap`], [`Pixmap`], [`crate::Surface`], [`Picture`], encoded streams,
/// GPU texture, YUV_ColorSpace data, or hardware buffer. Encoded streams supported
/// include BMP, GIF, HEIF, ICO, JPEG, PNG, WBMP, WebP. Supported encoding details
/// vary with platform.
pub type Image = RCHandle<SkImage>;
unsafe_send_sync!(Image);
require_base_type!(SkImage, sb::SkRefCnt);

impl NativeBase<SkRefCntBase> for SkImage {}

impl NativeRefCountedBase for SkImage {
    type Base = SkRefCntBase;
}

impl fmt::Debug for Image {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Image");
        let d = d
            .field("image_info", &self.image_info())
            .field("unique_id", &self.unique_id())
            .field("alpha_type", &self.alpha_type())
            .field("color_type", &self.color_type())
            .field("color_space", &self.color_space())
            .field("is_texture_backed", &self.is_texture_backed());
        #[cfg(feature = "gpu")]
        let d = d.field("texture_size", &self.texture_size());
        d.field("has_mipmaps", &self.has_mipmaps())
            .field("is_lazy_generated", &self.is_lazy_generated())
            .finish()
    }
}

impl Image {
    /// Creates [`Image`] from [`ImageInfo`], sharing pixels.
    ///
    /// [`Image`] is returned if [`ImageInfo`] is valid. Valid [`ImageInfo`] parameters include:
    /// dimensions are greater than zero;
    /// each dimension fits in 29 bits;
    /// [`ColorType`] and [`AlphaType`] are valid, and [`ColorType`] is not [`ColorType::Unknown`];
    /// rowBytes are large enough to hold one row of pixels;
    /// pixels is not nullptr, and contains enough data for [`Image`].
    ///
    /// - `info`       contains width, height, [`AlphaType`], [`ColorType`], [`ColorSpace`]
    /// - `pixels`     address or pixel storage
    /// - `rowBytes`   size of pixel row or larger
    ///
    /// Returns: [`Image`] sharing pixels, or `None`
    #[deprecated(since = "0.63.0", note = "use images::raster_from_data()")]
    pub fn from_raster_data(
        info: &ImageInfo,
        pixels: impl Into<Data>,
        row_bytes: usize,
    ) -> Option<Image> {
        images::raster_from_data(info, pixels, row_bytes)
    }

    /// Creates [`Image`] from bitmap, sharing or copying bitmap pixels. If the bitmap
    /// is marked immutable, and its pixel memory is shareable, it may be shared
    /// instead of copied.
    ///
    /// [`Image`] is returned if bitmap is valid. Valid [`Bitmap`] parameters include:
    /// dimensions are greater than zero;
    /// each dimension fits in 29 bits;
    /// [`ColorType`] and [`AlphaType`] are valid, and [`ColorType`] is not [`ColorType::Unknown`];
    /// row bytes are large enough to hold one row of pixels;
    /// pixel address is not `null`.
    ///
    /// - `bitmap`   [`ImageInfo`], row bytes, and pixels
    ///
    /// Returns: created [`Image`], or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_MakeFromBitmap>
    #[deprecated(since = "0.63.0", note = "use images::raster_from_bitmap()")]
    pub fn from_bitmap(bitmap: &Bitmap) -> Option<Image> {
        images::raster_from_bitmap(bitmap)
    }

    /// Creates [`Image`] from data returned by `image_generator`. Generated data is owned by [`Image`] and
    /// may not be shared or accessed.
    ///
    /// [`Image`] is returned if generator data is valid. Valid data parameters vary by type of data
    /// and platform.
    ///
    /// imageGenerator may wrap [`Picture`] data, codec data, or custom data.
    ///
    /// - `image_generator`   stock or custom routines to retrieve [`Image`]
    ///
    /// Returns: created [`Image`], or `None`
    #[deprecated(since = "0.63.0", note = "use images::deferred_from_generator()")]
    pub fn from_generator(image_generator: ImageGenerator) -> Option<Image> {
        images::deferred_from_generator(image_generator)
    }

    /// See [`Self::from_encoded_with_alpha_type()`]
    pub fn from_encoded(data: impl Into<Data>) -> Option<Image> {
        images::deferred_from_encoded_data(data, None)
    }

    /// Return an image backed by the encoded data, but attempt to defer decoding until the image
    /// is actually used/drawn. This deferral allows the system to cache the result, either on the
    /// CPU or on the GPU, depending on where the image is drawn. If memory is low, the cache may
    /// be purged, causing the next draw of the image to have to re-decode.
    ///
    /// If alphaType is `None`, the image's alpha type will be chosen automatically based on the
    /// image format. Transparent images will default to [`AlphaType::Premul`]. If alphaType contains
    /// [`AlphaType::Premul`] or [`AlphaType::Unpremul`], that alpha type will be used. Forcing opaque
    /// (passing [`AlphaType::Opaque`]) is not allowed, and will return nullptr.
    ///
    /// This is similar to `decode_to_{raster,texture}`, but this method will attempt to defer the
    /// actual decode, while the `decode_to`... method explicitly decode and allocate the backend
    /// when the call is made.
    ///
    /// If the encoded format is not supported, `None` is returned.
    ///
    /// - `encoded`   the encoded data
    ///
    /// Returns: created [`Image`], or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_MakeFromEncoded>
    pub fn from_encoded_with_alpha_type(
        data: impl Into<Data>,
        alpha_type: impl Into<Option<AlphaType>>,
    ) -> Option<Image> {
        images::deferred_from_encoded_data(data, alpha_type)
    }

    #[deprecated(since = "0.35.0", note = "Removed without replacement")]
    pub fn decode_to_raster(_encoded: &[u8], _subset: impl Into<Option<IRect>>) -> ! {
        panic!("Removed without replacement")
    }

    /// Creates a CPU-backed [`Image`] from compressed data.
    ///
    /// This method will decompress the compressed data and create an image wrapping
    /// it. Any mipmap levels present in the compressed data are discarded.
    ///
    /// - `data`      compressed data to store in [`Image`]
    /// - `width`     width of full [`Image`]
    /// - `height`    height of full [`Image`]
    /// - `ty`        type of compression used
    ///
    /// Returns: created [`Image`], or `None`
    #[deprecated(
        since = "0.63.0",
        note = "use images::raster_from_compressed_texture_data()"
    )]
    pub fn new_raster_from_compressed(
        data: impl Into<Data>,
        dimensions: impl Into<ISize>,
        ty: TextureCompressionType,
    ) -> Option<Image> {
        images::raster_from_compressed_texture_data(data, dimensions, ty)
    }

    /// See [`Self::from_picture_with_props()`]
    #[deprecated(since = "0.63.0", note = "use images::deferred_from_picture()")]
    pub fn from_picture(
        picture: impl Into<Picture>,
        dimensions: impl Into<ISize>,
        matrix: Option<&Matrix>,
        paint: Option<&Paint>,
        bit_depth: BitDepth,
        color_space: impl Into<Option<ColorSpace>>,
    ) -> Option<Image> {
        images::deferred_from_picture(
            picture,
            dimensions,
            matrix,
            paint,
            bit_depth,
            color_space,
            None,
        )
    }

    /// Creates [`Image`] from picture. Returned [`Image`] width and height are set by dimensions.
    /// [`Image`] draws picture with matrix and paint, set to bitDepth and colorSpace.
    ///
    /// If matrix is `None`, draws with identity [`Matrix`]. If paint is `None`, draws
    /// with default [`Paint`]. color_space may be `None`.
    ///
    /// - `picture`      stream of drawing commands
    /// - `dimensions`   width and height
    /// - `matrix`       [`Matrix`] to rotate, scale, translate, and so on; may be `None`
    /// - `paint`        [`Paint`] to apply transparency, filtering, and so on; may be `None`
    /// - `bitDepth`     8-bit integer or 16-bit float: per component
    /// - `color_space`  range of colors; may be `None`
    /// - `props`        props to use when rasterizing the picture
    ///
    /// Returns: created [`Image`], or `None`
    #[deprecated(since = "0.63.0", note = "use images::deferred_from_picture()")]
    pub fn from_picture_with_props(
        picture: impl Into<Picture>,
        dimensions: impl Into<ISize>,
        matrix: Option<&Matrix>,
        paint: Option<&Paint>,
        bit_depth: BitDepth,
        color_space: impl Into<Option<ColorSpace>>,
        props: SurfaceProps,
    ) -> Option<Image> {
        images::deferred_from_picture(
            picture,
            dimensions,
            matrix,
            paint,
            bit_depth,
            color_space,
            Some(props),
        )
    }

    /// Creates a GPU-backed [`Image`] from compressed data.
    ///
    /// This method will return an [`Image`] representing the compressed data.
    /// If the GPU doesn't support the specified compression method, the data
    /// will be decompressed and then wrapped in a GPU-backed image.
    ///
    /// Note: one can query the supported compression formats via
    /// [`gpu::RecordingContext::compressed_backend_format`].
    ///
    /// - `context`      GPU context
    /// - `data`         compressed data to store in [`Image`]
    /// - `width`        width of full [`Image`]
    /// - `height`       height of full [`Image`]
    /// - `ty`           type of compression used
    /// - `mipmapped`    does 'data' contain data for all the mipmap levels?
    /// - `is_protected`  do the contents of 'data' require DRM protection (on Vulkan)?
    ///
    /// Returns: created [`Image`], or `None`
    #[cfg(feature = "gpu")]
    #[deprecated(
        since = "0.63.0",
        note = "use gpu::images::texture_from_compressed_texture_data()"
    )]
    pub fn new_texture_from_compressed(
        context: &mut gpu::DirectContext,
        data: Data,
        dimensions: impl Into<ISize>,
        ty: TextureCompressionType,
        mipmapped: impl Into<Option<gpu::Mipmapped>>,
        is_protected: impl Into<Option<gpu::Protected>>,
    ) -> Option<Image> {
        gpu::images::texture_from_compressed_texture_data(
            context,
            data,
            dimensions,
            ty,
            mipmapped,
            is_protected,
        )
    }

    #[cfg(feature = "gpu")]
    #[deprecated(since = "0.35.0", note = "Removed without replacement")]
    pub fn from_compressed(
        _context: &mut gpu::RecordingContext,
        _data: Data,
        _dimensions: impl Into<ISize>,
        _ct: TextureCompressionType,
    ) -> ! {
        panic!("Removed without replacement.")
    }

    /// Creates [`Image`] from GPU texture associated with context. GPU texture must stay
    /// valid and unchanged until `texture_release_proc` is called. `texture_release_proc` is
    /// passed `release_context` when [`Image`] is deleted or no longer refers to texture.
    ///
    /// [`Image`] is returned if format of `backend_texture` is recognized and supported.
    /// Recognized formats vary by GPU back-end.
    ///
    /// Note: When using a DDL recording context, `texture_release_proc` will be called on the
    /// GPU thread after the DDL is played back on the direct context.
    ///
    /// * `context`               GPU context
    /// * `backend_texture`       Texture residing on GPU
    /// * `origin`                Origin of `backend_texture`
    /// * `color_type`            Color type of the resulting image
    /// * `alpha_type`            Alpha type of the resulting image
    /// * `color_space`           This describes the color space of this image's contents, as
    ///                           seen after sampling. In general, if the format of the backend
    ///                           texture is SRGB, some linear `color_space` should be supplied
    ///                           (e.g., [`ColorSpace::new_srgb_linear()`])). If the format of the
    ///                           backend texture is linear, then the `color_space` should include
    ///                           a description of the transfer function as
    ///                           well (e.g., [`ColorSpace::MakeSRGB`]()).
    /// * `texture_release_proc`  Function called when texture can be released
    /// * `release_context`       State passed to `texture_release_proc`
    ///
    /// Returns: Created [`Image`], or `None`
    #[cfg(feature = "gpu")]
    pub fn from_texture(
        context: &mut gpu::RecordingContext,
        backend_texture: &gpu::BackendTexture,
        origin: gpu::SurfaceOrigin,
        color_type: ColorType,
        alpha_type: AlphaType,
        color_space: impl Into<Option<ColorSpace>>,
    ) -> Option<Image> {
        gpu::images::borrow_texture_from(
            context,
            backend_texture,
            origin,
            color_type,
            alpha_type,
            color_space,
        )
    }

    #[deprecated(since = "0.27.0", note = "renamed, use new_cross_context_from_pixmap")]
    #[cfg(feature = "gpu")]
    pub fn from_pixmap_cross_context(
        context: &mut gpu::DirectContext,
        pixmap: &Pixmap,
        build_mips: bool,
        limit_to_max_texture_size: impl Into<Option<bool>>,
    ) -> Option<Image> {
        gpu::images::cross_context_texture_from_pixmap(
            context,
            pixmap,
            build_mips,
            limit_to_max_texture_size,
        )
    }

    /// Creates [`Image`] from pixmap. [`Image`] is uploaded to GPU back-end using context.
    ///
    /// Created [`Image`] is available to other GPU contexts, and is available across thread
    /// boundaries. All contexts must be in the same GPU share group, or otherwise
    /// share resources.
    ///
    /// When [`Image`] is no longer referenced, context releases texture memory
    /// asynchronously.
    ///
    /// [`ColorSpace`] of [`Image`] is determined by `pixmap.color_space()`.
    ///
    /// [`Image`] is returned referring to GPU back-end if context is not `None`,
    /// format of data is recognized and supported, and if context supports moving
    /// resources between contexts. Otherwise, pixmap pixel data is copied and [`Image`]
    /// as returned in raster format if possible; `None` may be returned.
    /// Recognized GPU formats vary by platform and GPU back-end.
    ///
    /// - `context`                 GPU context
    /// - `pixmap`                  [`ImageInfo`], pixel address, and row bytes
    /// - `build_mips`               create [`Image`] as mip map if `true`
    /// - `limit_to_max_texture_size`   downscale image to GPU maximum texture size, if necessary
    ///
    /// Returns: created [`Image`], or `None`
    #[cfg(feature = "gpu")]
    #[deprecated(
        since = "0.63.0",
        note = "use gpu::images::cross_context_texture_from_pixmap()"
    )]
    pub fn new_cross_context_from_pixmap(
        context: &mut gpu::DirectContext,
        pixmap: &Pixmap,
        build_mips: bool,
        limit_to_max_texture_size: impl Into<Option<bool>>,
    ) -> Option<Image> {
        gpu::images::cross_context_texture_from_pixmap(
            context,
            pixmap,
            build_mips,
            limit_to_max_texture_size,
        )
    }

    /// Creates [`Image`] from `backend_texture` associated with context. `backend_texture` and
    /// returned [`Image`] are managed internally, and are released when no longer needed.
    ///
    /// [`Image`] is returned if format of `backend_texture` is recognized and supported.
    /// Recognized formats vary by GPU back-end.
    ///
    /// - `context`          GPU context
    /// - `backend_texture`   texture residing on GPU
    /// - `texture_origin`    origin of `backend_texture`
    /// - `color_type`        color type of the resulting image
    /// - `alpha_type`        alpha type of the resulting image
    /// - `color_space`       range of colors; may be `None`
    ///
    /// Returns: created [`Image`], or `None`
    #[cfg(feature = "gpu")]
    #[deprecated(since = "0.63.0", note = "use gpu::images::adopt_texture_from()")]
    pub fn from_adopted_texture(
        context: &mut gpu::RecordingContext,
        backend_texture: &gpu::BackendTexture,
        texture_origin: gpu::SurfaceOrigin,
        color_type: ColorType,
        alpha_type: impl Into<Option<AlphaType>>,
        color_space: impl Into<Option<ColorSpace>>,
    ) -> Option<Image> {
        gpu::images::adopt_texture_from(
            context,
            backend_texture,
            texture_origin,
            color_type,
            alpha_type,
            color_space,
        )
    }

    /// Creates an [`Image`] from `YUV[A]` planar textures. This requires that the textures stay valid
    /// for the lifetime of the image. The `ReleaseContext` can be used to know when it is safe to
    /// either delete or overwrite the textures. If `ReleaseProc` is provided it is also called before
    /// return on failure.
    ///
    /// - `context`             GPU context
    /// - `yuva_textures`        A set of textures containing YUVA data and a description of the
    ///                           data and transformation to RGBA.
    /// - `image_color_space`     range of colors of the resulting image after conversion to RGB;
    ///                           may be `None`
    /// - `texture_release_proc`  called when the backend textures can be released
    /// - `release_context`      state passed to `texture_release_proc`
    ///
    /// Returns: created [`Image`], or `None`
    #[cfg(feature = "gpu")]
    #[deprecated(
        since = "0.63.0",
        note = "use gpu::images::texture_from_yuva_textures()"
    )]
    pub fn from_yuva_textures(
        context: &mut gpu::RecordingContext,
        yuva_textures: &gpu::YUVABackendTextures,
        image_color_space: impl Into<Option<ColorSpace>>,
    ) -> Option<Image> {
        gpu::images::texture_from_yuva_textures(context, yuva_textures, image_color_space)
    }

    /// Creates [`Image`] from [`crate::YUVAPixmaps`].
    ///
    /// The image will remain planar with each plane converted to a texture using the passed
    /// [`gpu::RecordingContext`].
    ///
    /// [`crate::YUVAPixmaps`] has a [`crate::YUVAInfo`] which specifies the transformation from YUV to RGB.
    /// The [`ColorSpace`] of the resulting RGB values is specified by `image_color_space`. This will
    /// be the [`ColorSpace`] reported by the image and when drawn the RGB values will be converted
    /// from this space into the destination space (if the destination is tagged).
    ///
    /// Currently, this is only supported using the GPU backend and will fail if context is `None`.
    ///
    /// [`crate::YUVAPixmaps`] does not need to remain valid after this returns.
    ///
    /// - `context`                 GPU context
    /// - `pixmaps`                 The planes as pixmaps with supported [`crate::YUVAInfo`] that
    ///                               specifies conversion to RGB.
    /// - `build_mips`               create internal YUVA textures as mip map if `Yes`. This is
    ///                               silently ignored if the context does not support mip maps.
    /// - `limit_to_max_texture_size`   downscale image to GPU maximum texture size, if necessary
    /// - `image_color_space`         range of colors of the resulting image; may be `None`
    ///
    /// Returns: created [`Image`], or `None`
    #[cfg(feature = "gpu")]
    #[deprecated(
        since = "0.63.0",
        note = "use gpu::images::texture_from_yuva_pixmaps()"
    )]
    pub fn from_yuva_pixmaps(
        context: &mut gpu::RecordingContext,
        yuva_pixmaps: &crate::YUVAPixmaps,
        build_mips: impl Into<Option<gpu::Mipmapped>>,
        limit_to_max_texture_size: impl Into<Option<bool>>,
        image_color_space: impl Into<Option<ColorSpace>>,
    ) -> Option<Image> {
        gpu::images::texture_from_yuva_pixmaps(
            context,
            yuva_pixmaps,
            build_mips,
            limit_to_max_texture_size,
            image_color_space,
        )
    }

    #[cfg(feature = "gpu")]
    #[deprecated(since = "0.37.0", note = "Removed without replacement")]
    pub fn from_nv12_textures_copy(
        _context: &mut gpu::DirectContext,
        _yuv_color_space: crate::YUVColorSpace,
        _nv12_textures: &[gpu::BackendTexture; 2],
        _image_origin: gpu::SurfaceOrigin,
        _image_color_space: impl Into<Option<ColorSpace>>,
    ) -> ! {
        panic!("Removed without replacement")
    }

    /// Returns a [`ImageInfo`] describing the width, height, color type, alpha type, and color space
    /// of the [`Image`].
    ///
    /// Returns: image info of [`Image`].
    pub fn image_info(&self) -> &ImageInfo {
        ImageInfo::from_native_ref(&self.native().fInfo)
    }

    /// Returns pixel count in each row.
    ///
    /// Returns: pixel width in [`Image`]
    pub fn width(&self) -> i32 {
        self.image_info().width()
    }

    /// Returns pixel row count.
    ///
    /// Returns: pixel height in [`Image`]
    pub fn height(&self) -> i32 {
        self.image_info().height()
    }

    /// Returns [`ISize`] `{ width(), height() }`.
    ///
    /// Returns: integral size of `width()` and `height()`
    pub fn dimensions(&self) -> ISize {
        self.image_info().dimensions()
    }

    /// Returns [`IRect`] `{ 0, 0, width(), height() }`.
    ///
    /// Returns: integral rectangle from origin to `width()` and `height()`
    pub fn bounds(&self) -> IRect {
        self.image_info().bounds()
    }

    /// Returns value unique to image. [`Image`] contents cannot change after [`Image`] is
    /// created. Any operation to create a new [`Image`] will receive generate a new
    /// unique number.
    ///
    /// Returns: unique identifier
    pub fn unique_id(&self) -> u32 {
        self.native().fUniqueID
    }

    /// Returns [`AlphaType`].
    ///
    /// [`AlphaType`] returned was a parameter to an [`Image`] constructor,
    /// or was parsed from encoded data.
    ///
    /// Returns: [`AlphaType`] in [`Image`]
    ///
    /// example: <https://fiddle.skia.org/c/@Image_alphaType>
    pub fn alpha_type(&self) -> AlphaType {
        unsafe { self.native().alphaType() }
    }

    /// Returns [`ColorType`] if known; otherwise, returns [`ColorType::Unknown`].
    ///
    /// Returns: [`ColorType`] of [`Image`]
    ///
    /// example: <https://fiddle.skia.org/c/@Image_colorType>
    pub fn color_type(&self) -> ColorType {
        ColorType::from_native_c(unsafe { self.native().colorType() })
    }

    /// Returns a smart pointer to [`ColorSpace`], the range of colors, associated with
    /// [`Image`].  The smart pointer tracks the number of objects sharing this
    /// [`ColorSpace`] reference so the memory is released when the owners destruct.
    ///
    /// The returned [`ColorSpace`] is immutable.
    ///
    /// [`ColorSpace`] returned was passed to an [`Image`] constructor,
    /// or was parsed from encoded data. [`ColorSpace`] returned may be ignored when [`Image`]
    /// is drawn, depending on the capabilities of the [`crate::Surface`] receiving the drawing.
    ///
    /// Returns: [`ColorSpace`] in [`Image`], or `None`, wrapped in a smart pointer
    ///
    /// example: <https://fiddle.skia.org/c/@Image_refColorSpace>
    pub fn color_space(&self) -> Option<ColorSpace> {
        ColorSpace::from_unshared_ptr(unsafe { self.native().colorSpace() })
    }

    /// Returns `true` if [`Image`] pixels represent transparency only. If `true`, each pixel
    /// is packed in 8 bits as defined by [`ColorType::Alpha8`].
    ///
    /// Returns: `true` if pixels represent a transparency mask
    ///
    /// example: <https://fiddle.skia.org/c/@Image_isAlphaOnly>
    pub fn is_alpha_only(&self) -> bool {
        unsafe { self.native().isAlphaOnly() }
    }

    /// Returns `true` if pixels ignore their alpha value and are treated as fully opaque.
    ///
    /// Returns: `true` if [`AlphaType`] is [`AlphaType::Opaque`]
    pub fn is_opaque(&self) -> bool {
        self.alpha_type().is_opaque()
    }

    /// Make a shader with the specified tiling and mipmap sampling.
    pub fn to_shader<'a>(
        &self,
        tile_modes: impl Into<Option<(TileMode, TileMode)>>,
        sampling: impl Into<SamplingOptions>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        let tile_modes = tile_modes.into();
        let tm1 = tile_modes.map(|(tm, _)| tm).unwrap_or_default();
        let tm2 = tile_modes.map(|(_, tm)| tm).unwrap_or_default();
        let sampling = sampling.into();

        Shader::from_ptr(unsafe {
            sb::C_SkImage_makeShader(
                self.native(),
                tm1,
                tm2,
                sampling.native(),
                local_matrix.into().native_ptr_or_null(),
            )
        })
    }

    /// `to_raw_shader` functions like `to_shader`, but for images that contain non-color data.
    /// This includes images encoding things like normals, material properties (eg, roughness),
    /// heightmaps, or any other purely mathematical data that happens to be stored in an image.
    /// These types of images are useful with some programmable shaders (see: [`crate::RuntimeEffect`]).
    ///
    /// Raw image shaders work like regular image shaders (including filtering and tiling), with
    /// a few major differences:
    ///   - No color space transformation is ever applied (the color space of the image is ignored).
    ///   - Images with an alpha type of `Unpremul` are *not* automatically premultiplied.
    ///   - Bicubic filtering is not supported. If [`SamplingOptions::use_cubic`] is `true`, these
    ///     factories will return `None`.
    pub fn to_raw_shader<'a>(
        &self,
        tile_modes: impl Into<Option<(TileMode, TileMode)>>,
        sampling: impl Into<SamplingOptions>,
        local_matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        let tile_modes = tile_modes.into();
        let tm1 = tile_modes.map(|(tm, _)| tm).unwrap_or_default();
        let tm2 = tile_modes.map(|(_, tm)| tm).unwrap_or_default();
        let sampling = sampling.into();

        Shader::from_ptr(unsafe {
            sb::C_SkImage_makeRawShader(
                self.native(),
                tm1,
                tm2,
                sampling.native(),
                local_matrix.into().native_ptr_or_null(),
            )
        })
    }

    /// Copies [`Image`] pixel address, row bytes, and [`ImageInfo`] to pixmap, if address
    /// is available, and returns `true`. If pixel address is not available, return
    /// `false` and leave pixmap unchanged.
    ///
    /// - `pixmap`   storage for pixel state if pixels are readable; otherwise, ignored
    ///
    /// Returns: `true` if [`Image`] has direct access to pixels
    ///
    /// example: <https://fiddle.skia.org/c/@Image_peekPixels>
    pub fn peek_pixels(&self) -> Option<Pixmap> {
        let mut pixmap = Pixmap::default();
        unsafe { self.native().peekPixels(pixmap.native_mut()) }.then_some(pixmap)
    }

    /// Returns `true` if the contents of [`Image`] was created on or uploaded to GPU memory,
    /// and is available as a GPU texture.
    ///
    /// Returns: `true` if [`Image`] is a GPU texture
    ///
    /// example: <https://fiddle.skia.org/c/@Image_isTextureBacked>
    pub fn is_texture_backed(&self) -> bool {
        unsafe { sb::C_SkImage_isTextureBacked(self.native()) }
    }

    /// Returns an approximation of the amount of texture memory used by the image. Returns
    /// zero if the image is not texture backed or if the texture has an external format.
    pub fn texture_size(&self) -> usize {
        unsafe { sb::C_SkImage_textureSize(self.native()) }
    }

    /// Returns `true` if [`Image`] can be drawn on either raster surface or GPU surface.
    /// If recorder is None, tests if SkImage draws on raster surface;
    /// otherwise, tests if SkImage draws on the associated GPU surface.
    ///
    /// [`Image`] backed by GPU texture may become invalid if associated context is
    /// invalid. lazy image may be invalid and may not draw to raster surface or
    /// GPU surface or both.
    ///
    /// - `context`   GPU context
    ///
    /// Returns: `true` if [`Image`] can be drawn
    ///
    /// example: <https://fiddle.skia.org/c/@Image_isValid>
    pub fn is_valid(&self, recorder: Option<&mut dyn Recorder>) -> bool {
        unsafe {
            sb::C_SkImage_isValid(
                self.native(),
                recorder
                    .map(|r| r.as_recorder_ref())
                    .native_ptr_or_null_mut(),
            )
        }
    }

    /// Create a new image by copying this image and scaling to fit the [`ImageInfo`]'s dimensions
    /// and converting the pixels into the ImageInfo's [`crate::ColorInfo`].
    ///
    /// This is done retaining the domain (backend) of the image (e.g. gpu, raster).
    ///
    /// Returns `None` if the requested [`crate::ColorInfo`] is not supported, its dimensions are out
    /// of range.
    pub fn make_scaled(
        &self,
        info: &ImageInfo,
        scaling: impl Into<SamplingOptions>,
    ) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImage_makeScaled(self.native(), info.native(), scaling.into().native())
        })
    }

    /// See [`Self::flush_with_info()`]
    #[cfg(feature = "gpu")]
    #[deprecated(since = "0.63.0", note = "use gpu::DirectContext::flush()")]
    pub fn flush<'a>(
        &self,
        context: &mut gpu::DirectContext,
        flush_info: impl Into<Option<&'a gpu::FlushInfo>>,
    ) -> gpu::SemaphoresSubmitted {
        context.flush(flush_info)
    }

    /// Flushes any pending uses of texture-backed images in the GPU backend. If the image is not
    /// texture-backed (including promise texture images) or if the [`gpu::DirectContext`] does not
    /// have the same context ID as the context backing the image then this is a no-op.
    ///
    /// If the image was not used in any non-culled draws in the current queue of work for the
    /// passed [`gpu::DirectContext`] then this is a no-op unless the [`gpu::FlushInfo`] contains semaphores or
    /// a finish proc. Those are respected even when the image has not been used.
    ///
    /// - `context`   the context on which to flush pending usages of the image.
    /// - `info`      flush options
    #[cfg(feature = "gpu")]
    #[deprecated(since = "0.46.0", note = "use gpu::DirectContext::flush()")]
    pub fn flush_with_info(
        &self,
        context: &mut gpu::DirectContext,
        flush_info: &gpu::FlushInfo,
    ) -> gpu::SemaphoresSubmitted {
        context.flush(flush_info)
    }

    /// Version of `flush()` that uses a default [`gpu::FlushInfo`]. Also submits the flushed work to the
    /// GPU.
    #[cfg(feature = "gpu")]
    #[deprecated(since = "0.63.0", note = "use gpu::DirectContext::flush_and_submit()")]
    pub fn flush_and_submit(&self, context: &mut gpu::DirectContext) {
        context.flush_and_submit();
    }

    /// Retrieves the back-end texture. If [`Image`] has no back-end texture, `None`is returned.
    ///
    /// If `flush_pending_gr_context_io` is `true`, completes deferred I/O operations.
    ///
    /// If origin in not `None`, copies location of content drawn into [`Image`].
    ///
    /// - `flush_pending_gr_context_io`   flag to flush outstanding requests
    ///
    /// Returns: back-end API texture handle; invalid on failure
    #[cfg(feature = "gpu")]
    #[deprecated(
        since = "0.63.0",
        note = "use gpu::images::get_backend_texture_from_image()"
    )]
    pub fn backend_texture(
        &self,
        flush_pending_gr_context_io: bool,
    ) -> Option<(gpu::BackendTexture, gpu::SurfaceOrigin)> {
        gpu::images::get_backend_texture_from_image(self, flush_pending_gr_context_io)
    }

    /// Copies [`crate::Rect`] of pixels from [`Image`] to `dst_pixels`. Copy starts at offset (`src_x`, `src_y`),
    /// and does not exceed [`Image`] (width(), height()).
    ///
    /// Graphite has deprecated this API in favor of the equivalent asynchronous API on
    /// `skgpu::graphite::Context` (with an optional explicit synchronization).
    ///
    /// `dst_info` specifies width, height, [`ColorType`], [`AlphaType`], and [`ColorSpace`] of
    /// destination. `dst_row_bytes` specifies the gap from one destination row to the next.
    /// Returns `true` if pixels are copied. Returns `false` if:
    /// - `dst_info`.`addr()` equals `None`
    /// - `dst_row_bytes` is less than `dst_info.min_row_bytes()`
    /// - [`crate::PixelRef`] is `None`
    ///
    /// Pixels are copied only if pixel conversion is possible. If [`Image`] [`ColorType`] is
    /// [`ColorType::Gray8`], or [`ColorType::Alpha8`]; `dst_info.color_type()` must match.
    /// If [`Image`] [`ColorType`] is [`ColorType::Gray8`], `dst_info`.`color_space()` must match.
    /// If [`Image`] [`AlphaType`] is [`AlphaType::Opaque`], `dst_info`.`alpha_type()` must
    /// match. If [`Image`] [`ColorSpace`] is `None`, `dst_info.color_space()` must match. Returns
    /// `false` if pixel conversion is not possible.
    ///
    /// `src_x` and `src_y` may be negative to copy only top or left of source. Returns
    /// `false` if `width()` or `height()` is zero or negative.
    /// Returns `false` if abs(`src_x`) >= Image width(), or if abs(`src_y`) >= Image height().
    ///
    /// If `caching_hint` is [`CachingHint::Allow`], pixels may be retained locally.
    /// If `caching_hint` is [`CachingHint::Disallow`], pixels are not added to the local cache.
    ///
    /// - `context`       the [`gpu::DirectContext`] in play, if it exists
    /// - `dst_info`       destination width, height, [`ColorType`], [`AlphaType`], [`ColorSpace`]
    /// - `dst_pixels`     destination pixel storage
    /// - `dst_row_bytes`   destination row length
    /// - `src_x`          column index whose absolute value is less than `width()`
    /// - `src_y`          row index whose absolute value is less than `height()`
    /// - `caching_hint`   whether the pixels should be cached locally
    ///
    /// Returns: `true` if pixels are copied to `dst_pixels`
    #[cfg(feature = "gpu")]
    pub fn read_pixels_with_context<'a, P>(
        &self,
        context: impl Into<Option<&'a mut gpu::DirectContext>>,
        dst_info: &ImageInfo,
        pixels: &mut [P],
        dst_row_bytes: usize,
        src: impl Into<IPoint>,
        caching_hint: CachingHint,
    ) -> bool {
        if !dst_info.valid_pixels(dst_row_bytes, pixels) {
            return false;
        }

        let src = src.into();

        unsafe {
            self.native().readPixels(
                context.into().native_ptr_or_null_mut(),
                dst_info.native(),
                pixels.as_mut_ptr() as _,
                dst_row_bytes,
                src.x,
                src.y,
                caching_hint,
            )
        }
    }

    /// Copies a [`crate::Rect`] of pixels from [`Image`] to dst. Copy starts at (`src_x`, `src_y`), and
    /// does not exceed [`Image`] (width(), height()).
    ///
    /// Graphite has deprecated this API in favor of the equivalent asynchronous API on
    /// `skgpu::graphite::Context` (with an optional explicit synchronization).
    ///
    /// dst specifies width, height, [`ColorType`], [`AlphaType`], [`ColorSpace`], pixel storage,
    /// and row bytes of destination. dst.`row_bytes()` specifics the gap from one destination
    /// row to the next. Returns `true` if pixels are copied. Returns `false` if:
    /// - dst pixel storage equals `None`
    /// - dst.`row_bytes` is less than [`ImageInfo::min_row_bytes`]
    /// - [`crate::PixelRef`] is `None`
    ///
    /// Pixels are copied only if pixel conversion is possible. If [`Image`] [`ColorType`] is
    /// [`ColorType::Gray8`], or [`ColorType::Alpha8`]; dst.`color_type()` must match.
    /// If [`Image`] [`ColorType`] is [`ColorType::Gray8`], dst.`color_space()` must match.
    /// If [`Image`] [`AlphaType`] is [`AlphaType::Opaque`], dst.`alpha_type()` must
    /// match. If [`Image`] [`ColorSpace`] is `None`, dst.`color_space()` must match. Returns
    /// `false` if pixel conversion is not possible.
    ///
    /// `src_x` and `src_y` may be negative to copy only top or left of source. Returns
    /// `false` if `width()` or `height()` is zero or negative.
    /// Returns `false` if abs(`src_x`) >= Image width(), or if abs(`src_y`) >= Image height().
    ///
    /// If `caching_hint` is [`CachingHint::Allow`], pixels may be retained locally.
    /// If `caching_hint` is [`CachingHint::Disallow`], pixels are not added to the local cache.
    ///
    /// - `context`       the [`gpu::DirectContext`] in play, if it exists
    /// - `dst`           destination [`Pixmap`]:[`ImageInfo`], pixels, row bytes
    /// - `src_x`          column index whose absolute value is less than `width()`
    /// - `src_y`          row index whose absolute value is less than `height()`
    /// - `caching_hint`   whether the pixels should be cached `locally_z`
    ///
    /// Returns: `true` if pixels are copied to dst
    #[cfg(feature = "gpu")]
    pub fn read_pixels_to_pixmap_with_context<'a>(
        &self,
        context: impl Into<Option<&'a mut gpu::DirectContext>>,
        dst: &Pixmap,
        src: impl Into<IPoint>,
        caching_hint: CachingHint,
    ) -> bool {
        let src = src.into();

        unsafe {
            self.native().readPixels1(
                context.into().native_ptr_or_null_mut(),
                dst.native(),
                src.x,
                src.y,
                caching_hint,
            )
        }
    }

    // _not_ deprecated, because we support separate functions in `gpu` feature builds.
    /// See [`Self::read_pixels_with_context()`]
    pub fn read_pixels<P>(
        &self,
        dst_info: &ImageInfo,
        pixels: &mut [P],
        dst_row_bytes: usize,
        src: impl Into<IPoint>,
        caching_hint: CachingHint,
    ) -> bool {
        if !dst_info.valid_pixels(dst_row_bytes, pixels) {
            return false;
        }

        let src = src.into();

        unsafe {
            self.native().readPixels(
                ptr::null_mut(),
                dst_info.native(),
                pixels.as_mut_ptr() as _,
                dst_row_bytes,
                src.x,
                src.y,
                caching_hint,
            )
        }
    }

    /// See [`Self::read_pixels_to_pixmap_with_context()`]
    #[cfg(feature = "gpu")]
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn read_pixels_to_pixmap(
        &self,
        dst: &Pixmap,
        src: impl Into<IPoint>,
        caching_hint: CachingHint,
    ) -> bool {
        let src = src.into();

        self.native()
            .readPixels1(ptr::null_mut(), dst.native(), src.x, src.y, caching_hint)
    }

    // TODO:
    // AsyncReadResult,
    // ReadPixelsContext,
    // ReadPixelsCallback,
    // RescaleGamma,
    // RescaleMode,
    // asyncRescaleAndReadPixels,
    // asyncRescaleAndReadPixelsYUV420,
    // asyncRescaleAndReadPixelsYUVA420

    /// Copies [`Image`] to dst, scaling pixels to fit `dst.width()` and `dst.height()`, and
    /// converting pixels to match `dst.color_type()` and `dst.alpha_type()`. Returns `true` if
    /// pixels are copied. Returns `false` if `dst.addr()` is `None`, or `dst.row_bytes()` is
    /// less than dst [`ImageInfo::min_row_bytes`].
    ///
    /// Pixels are copied only if pixel conversion is possible. If [`Image`] [`ColorType`] is
    /// [`ColorType::Gray8`], or [`ColorType::Alpha8`]; `dst.color_type()` must match.
    /// If [`Image`] [`ColorType`] is [`ColorType::Gray8`], `dst.color_space()` must match.
    /// If [`Image`] [`AlphaType`] is [`AlphaType::Opaque`], `dst.alpha_type()` must
    /// match. If [`Image`] [`ColorSpace`] is `None`, `dst.color_space()` must match. Returns
    /// `false` if pixel conversion is not possible.
    ///
    /// If `caching_hint` is [`CachingHint::Allow`], pixels may be retained locally.
    /// If `caching_hint` is [`CachingHint::Disallow`], pixels are not added to the local cache.
    ///
    /// - `dst`             destination [`Pixmap`]:[`ImageInfo`], pixels, row bytes
    ///
    /// Returns: `true` if pixels are scaled to fit dst
    #[must_use]
    pub fn scale_pixels(
        &self,
        dst: &Pixmap,
        sampling: impl Into<SamplingOptions>,
        caching_hint: impl Into<Option<CachingHint>>,
    ) -> bool {
        unsafe {
            self.native().scalePixels(
                dst.native(),
                sampling.into().native(),
                caching_hint.into().unwrap_or(CachingHint::Allow),
            )
        }
    }

    /// Encodes [`Image`] pixels, returning result as [`Data`].
    ///
    ///  Returns `None` if encoding fails, or if `encoded_image_format` is not supported.
    ///
    ///  [`Image`] encoding in a format requires both building with one or more of:
    ///  SK_ENCODE_JPEG, SK_ENCODE_PNG, SK_ENCODE_WEBP; and platform support
    ///  for the encoded format.
    ///
    ///  If SK_BUILD_FOR_MAC or SK_BUILD_FOR_IOS is defined, `encoded_image_format` can
    ///  additionally be one of: [`EncodedImageFormat::ICO`], [`EncodedImageFormat::BMP`],
    ///  [`EncodedImageFormat::GIF`].
    ///
    ///  quality is a platform and format specific metric trading off size and encoding
    ///  error. When used, quality equaling 100 encodes with the least error. quality may
    ///  be ignored by the encoder.
    ///
    ///  * `context` - the [`gpu::DirectContext`] in play, if it exists; can be `None`
    ///  * `encoded_image_format` - one of: [`EncodedImageFormat::JPEG`], [`EncodedImageFormat::PNG`],
    ///                             [`EncodedImageFormat::WEBP`]
    ///  * `quality` - encoder specific metric with 100 equaling best
    ///
    ///  Returns: encoded [`Image`], or `None`
    ///
    ///  example: <https://fiddle.skia.org/c/@Image_encodeToData>
    #[cfg(feature = "gpu")]
    #[deprecated(since = "0.63.0", note = "Use encode")]
    pub fn encode_to_data_with_context(
        &self,
        context: impl Into<Option<gpu::DirectContext>>,
        image_format: EncodedImageFormat,
        quality: impl Into<Option<u32>>,
    ) -> Option<Data> {
        let mut context = context.into();
        self.encode(context.as_mut(), image_format, quality)
    }

    /// See [`Self::encode_to_data_with_quality`]
    #[deprecated(
        since = "0.63.0",
        note = "Support for encoding GPU backed images without a context was removed, use `encode_to_data_with_context` instead"
    )]
    pub fn encode_to_data(&self, image_format: EncodedImageFormat) -> Option<Data> {
        self.encode(None, image_format, 100)
    }

    /// Encodes [`Image`] pixels, returning result as [`Data`].
    ///
    /// Returns `None` if encoding fails, or if `encoded_image_format` is not supported.
    ///
    /// [`Image`] encoding in a format requires both building with one or more of:
    /// SK_ENCODE_JPEG, SK_ENCODE_PNG, SK_ENCODE_WEBP; and platform support
    /// for the encoded format.
    ///
    /// If SK_BUILD_FOR_MAC or SK_BUILD_FOR_IOS is defined, `encoded_image_format` can
    /// additionally be one of: [`EncodedImageFormat::ICO`], [`EncodedImageFormat::BMP`],
    /// [`EncodedImageFormat::GIF`].
    ///
    /// quality is a platform and format specific metric trading off size and encoding
    /// error. When used, quality equaling 100 encodes with the least error. quality may
    /// be ignored by the encoder.
    ///
    /// - `encoded_image_format`   one of: [`EncodedImageFormat::JPEG`], [`EncodedImageFormat::PNG`],
    ///                            [`EncodedImageFormat::WEBP`]
    /// - `quality`              encoder specific metric with 100 equaling best
    ///
    /// Returns: encoded [`Image`], or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_encodeToData>
    #[deprecated(
        since = "0.63.0",
        note = "Support for encoding GPU backed images without a context was removed, use `encode_to_data_with_context` instead"
    )]
    pub fn encode_to_data_with_quality(
        &self,
        image_format: EncodedImageFormat,
        quality: u32,
    ) -> Option<Data> {
        self.encode(None, image_format, quality)
    }

    /// Returns encoded [`Image`] pixels as [`Data`], if [`Image`] was created from supported
    /// encoded stream format. Platform support for formats vary and may require building
    /// with one or more of: SK_ENCODE_JPEG, SK_ENCODE_PNG, SK_ENCODE_WEBP.
    ///
    /// Returns `None` if [`Image`] contents are not encoded.
    ///
    /// Returns: encoded [`Image`], or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_refEncodedData>
    pub fn encoded_data(&self) -> Option<Data> {
        Data::from_ptr_const(unsafe { sb::C_SkImage_refEncodedData(self.native()) })
    }

    /// Returns subset of this image.
    ///
    /// Returns `None` if any of the following are true:
    ///   - Subset is empty
    ///   - Subset is not contained inside the image's bounds
    ///   - Pixels in the image could not be read or copied
    ///   - This image is texture-backed and the provided context is null or does not match
    ///     the source image's context.
    ///
    /// If the source image was texture-backed, the resulting image will be texture-backed also.
    /// Otherwise, the returned image will be raster-backed.
    ///
    /// * `recorder` - the recorder of the source image (`None` is ok if the
    ///                            source image was texture-backed).
    /// * `subset` - bounds of returned [`Image`]
    /// * `required_properties` - properties the returned [`Image`] must possess (e.g. mipmaps)
    ///
    /// Returns: the subsetted image, or `None`
    pub fn make_subset(
        &self,
        recorder: Option<&mut dyn Recorder>,
        subset: impl AsRef<IRect>,
        required_properties: RequiredProperties,
    ) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImage_makeSubset(
                self.native(),
                recorder
                    .map(|r| r.as_recorder_ref())
                    .native_ptr_or_null_mut(),
                subset.as_ref().native(),
                required_properties.native(),
            )
        })
    }

    /// Returns `true` if the image has mipmap levels.
    pub fn has_mipmaps(&self) -> bool {
        unsafe { self.native().hasMipmaps() }
    }

    /// Returns an image with the same "base" pixels as the this image, but with mipmap levels
    /// automatically generated and attached.
    pub fn with_default_mipmaps(&self) -> Option<Image> {
        Image::from_ptr(unsafe { sb::C_SkImage_withDefaultMipmaps(self.native()) })
    }

    /// See [`Self::new_texture_image_budgeted`]
    #[cfg(feature = "gpu")]
    pub fn new_texture_image(
        &self,
        context: &mut gpu::DirectContext,
        mipmapped: gpu::Mipmapped,
    ) -> Option<Image> {
        self.new_texture_image_budgeted(context, mipmapped, gpu::Budgeted::Yes)
    }

    /// Returns [`Image`] backed by GPU texture associated with context. Returned [`Image`] is
    /// compatible with [`crate::Surface`] created with `dst_color_space`. The returned [`Image`] respects
    /// mipmapped setting; if mipmapped equals [`gpu::Mipmapped::Yes`], the backing texture
    /// allocates mip map levels.
    ///
    /// The mipmapped parameter is effectively treated as `No` if MIP maps are not supported by the
    /// GPU.
    ///
    /// Returns original [`Image`] if the image is already texture-backed, the context matches, and
    /// mipmapped is compatible with the backing GPU texture. [`crate::Budgeted`] is ignored in this case.
    ///
    /// Returns `None` if context is `None`, or if [`Image`] was created with another
    /// [`gpu::DirectContext`].
    ///
    /// - `direct_context`  the [`gpu::DirectContext`] in play, if it exists
    /// - `mipmapped`      whether created [`Image`] texture must allocate mip map levels
    /// - `budgeted`       whether to count a newly created texture for the returned image
    ///                     counts against the context's budget.
    ///
    /// Returns: created [`Image`], or `None`
    #[cfg(feature = "gpu")]
    pub fn new_texture_image_budgeted(
        &self,
        direct_context: &mut gpu::DirectContext,
        mipmapped: gpu::Mipmapped,
        budgeted: gpu::Budgeted,
    ) -> Option<Image> {
        gpu::images::texture_from_image(direct_context, self, mipmapped, budgeted)
    }

    /// Returns raster image or lazy image. Copies [`Image`] backed by GPU texture into
    /// CPU memory if needed. Returns original [`Image`] if decoded in raster bitmap,
    /// or if encoded in a stream.
    ///
    /// Returns `None` if backed by GPU texture and copy fails.
    ///
    /// Returns: raster image, lazy image, or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_makeNonTextureImage>
    #[deprecated(since = "0.64.0", note = "use make_non_texture_image()")]
    pub fn to_non_texture_image(&self) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImage_makeNonTextureImage(self.native(), ptr::null_mut())
        })
    }

    /// Returns raster image or lazy image. Copies [`Image`] backed by GPU texture into
    /// CPU memory if needed. Returns original [`Image`] if decoded in raster bitmap,
    /// or if encoded in a stream.
    ///
    /// Returns `None` if backed by GPU texture and copy fails.
    ///
    /// Returns: raster image, lazy image, or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_makeNonTextureImage>
    pub fn make_non_texture_image<'a>(
        &self,
        context: impl Into<Option<&'a mut gpu::DirectContext>>,
    ) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImage_makeNonTextureImage(
                self.native(),
                context.into().native_ptr_or_null_mut(),
            )
        })
    }

    /// Returns raster image. Copies [`Image`] backed by GPU texture into CPU memory,
    /// or decodes [`Image`] from lazy image. Returns original [`Image`] if decoded in
    /// raster bitmap.
    ///
    /// Returns `None` if copy, decode, or pixel read fails.
    ///
    /// If `caching_hint` is [`CachingHint::Allow`], pixels may be retained locally.
    /// If `caching_hint` is [`CachingHint::Disallow`], pixels are not added to the local cache.
    ///
    /// Returns: raster image, or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_makeRasterImage>
    #[deprecated(since = "0.64.0", note = "use make_raster_image()")]
    pub fn to_raster_image(&self, caching_hint: impl Into<Option<CachingHint>>) -> Option<Image> {
        let caching_hint = caching_hint.into().unwrap_or(CachingHint::Disallow);
        Image::from_ptr(unsafe {
            sb::C_SkImage_makeRasterImage(self.native(), ptr::null_mut(), caching_hint)
        })
    }

    /// Returns raster image. Copies [`Image`] backed by GPU texture into CPU memory,
    /// or decodes [`Image`] from lazy image. Returns original [`Image`] if decoded in
    /// raster bitmap.
    ///
    /// Returns `None` if copy, decode, or pixel read fails.
    ///
    /// If `caching_hint` is [`CachingHint::Allow`], pixels may be retained locally.
    /// If `caching_hint` is [`CachingHint::Disallow`], pixels are not added to the local cache.
    ///
    /// Returns: raster image, or `None`
    ///
    /// example: <https://fiddle.skia.org/c/@Image_makeRasterImage>
    pub fn make_raster_image<'a>(
        &self,
        context: impl Into<Option<&'a mut gpu::DirectContext>>,
        caching_hint: impl Into<Option<CachingHint>>,
    ) -> Option<Image> {
        let caching_hint = caching_hint.into().unwrap_or(CachingHint::Disallow);
        Image::from_ptr(unsafe {
            sb::C_SkImage_makeRasterImage(
                self.native(),
                context.into().native_ptr_or_null_mut(),
                caching_hint,
            )
        })
    }

    /// Creates filtered [`Image`]. filter processes original [`Image`], potentially changing
    /// color, position, and size. subset is the bounds of original [`Image`] processed
    /// by filter. `clip_bounds` is the expected bounds of the filtered [`Image`]. `out_subset`
    /// is required storage for the actual bounds of the filtered [`Image`]. offset is
    /// required storage for translation of returned [`Image`].
    ///
    /// Returns `None` if [`Image`] could not be created or if the recording context provided doesn't
    /// match the GPU context in which the image was created. If `None` is returned, `out_subset`
    /// and offset are undefined.
    ///
    /// Useful for animation of [`ImageFilter`] that varies size from frame to frame.
    /// Returned [`Image`] is created larger than required by filter so that GPU texture
    /// can be reused with different sized effects. `out_subset` describes the valid bounds
    /// of GPU texture returned. offset translates the returned [`Image`] to keep subsequent
    /// animation frames aligned with respect to each other.
    ///
    /// - `context`      the [`gpu::RecordingContext`] in play - if it exists
    /// - `filter`       how [`Image`] is sampled when transformed
    /// - `subset`       bounds of [`Image`] processed by filter
    /// - `clip_bounds`   expected bounds of filtered [`Image`]
    /// - `out_subset`    storage for returned [`Image`] bounds
    /// - `offset`       storage for returned [`Image`] translation
    ///
    /// Returns: filtered [`Image`], or `None`
    #[deprecated(since = "0.67.0", note = "use images::make_with_filter()")]
    pub fn new_with_filter(
        &self,
        _context: Option<&mut gpu::RecordingContext>,
        filter: &ImageFilter,
        clip_bounds: impl Into<IRect>,
        subset: impl Into<IRect>,
    ) -> Option<(Image, IRect, IPoint)> {
        images::make_with_filter(self, filter, subset.into(), clip_bounds.into())
    }

    // TODO: MakeBackendTextureFromSkImage()

    /// Returns `true` if [`Image`] is backed by an image-generator or other service that creates
    /// and caches its pixels or texture on-demand.
    ///
    /// Returns: `true` if [`Image`] is created as needed
    ///
    /// example: <https://fiddle.skia.org/c/@Image_isLazyGenerated_a>
    /// example: <https://fiddle.skia.org/c/@Image_isLazyGenerated_b>
    pub fn is_lazy_generated(&self) -> bool {
        unsafe { sb::C_SkImage_isLazyGenerated(self.native()) }
    }

    /// Creates [`Image`] in target [`ColorSpace`].
    /// Returns `None` if [`Image`] could not be created.
    ///
    /// Returns original [`Image`] if it is in target [`ColorSpace`].
    /// Otherwise, converts pixels from [`Image`] [`ColorSpace`] to target [`ColorSpace`].
    /// If [`Image`] `color_space()` returns `None`, [`Image`] [`ColorSpace`] is assumed to be `s_rgb`.
    ///
    /// If this image is graphite-backed, the recorder parameter is required.
    ///
    /// * `target_color_space` - [`ColorSpace`] describing color range of returned [`Image`]
    /// * `recorder` - The Recorder in which to create the new image
    /// * `required_properties` - properties the returned [`Image`] must possess (e.g. mipmaps)
    ///
    /// Returns: created [`Image`] in target [`ColorSpace`]
    pub fn make_color_space(
        &self,
        recorder: Option<&mut dyn Recorder>,
        color_space: impl Into<Option<ColorSpace>>,
        required_properties: RequiredProperties,
    ) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImage_makeColorSpace(
                self.native(),
                recorder
                    .map(|r| r.as_recorder_ref())
                    .native_ptr_or_null_mut(),
                color_space.into().into_ptr_or_null(),
                required_properties.native(),
            )
        })
    }

    /// Creates a new [`Image`] identical to this one, but with a different [`ColorSpace`].
    /// This does not convert the underlying pixel data, so the resulting image will draw
    /// differently.
    pub fn reinterpret_color_space(&self, new_color_space: impl Into<ColorSpace>) -> Option<Image> {
        Image::from_ptr(unsafe {
            sb::C_SkImage_reinterpretColorSpace(self.native(), new_color_space.into().into_ptr())
        })
    }
}
