use skia_bindings as sb;

use crate::{
    gpu::{
        BackendTexture, Budgeted, DirectContext, Mipmapped, Protected, RecordingContext,
        SurfaceOrigin, YUVABackendTextures,
    },
    prelude::*,
    AlphaType, ColorSpace, ColorType, Data, IRect, ISize, Image, Pixmap, TextureCompressionType,
};
#[allow(unused)] // docs only
use crate::{ImageInfo, Surface, YUVAInfo, YUVAPixmaps};

/// Creates GPU-backed [`Image`] from `backend_texture` associated with context.
/// Skia will assume ownership of the resource and will release it when no longer needed.
/// A non-null [`Image`] is returned if format of `backend_texture` is recognized and supported.
/// Recognized formats vary by GPU backend.
/// * `context` - GPU context
/// * `backend_texture` - texture residing on GPU
/// * `texture_origin` - origin of `backend_texture`
/// * `color_type` - color type of the resulting image
/// * `alpha_type` - alpha type of the resulting image
/// * `color_space` - range of colors; may be `None`
///
/// Returns: created [`Image`], or `None`
pub fn adopt_texture_from(
    context: &mut RecordingContext,
    backend_texture: &BackendTexture,
    texture_origin: SurfaceOrigin,
    color_type: ColorType,
    alpha_type: impl Into<Option<AlphaType>>,
    color_space: impl Into<Option<ColorSpace>>,
) -> Option<Image> {
    Image::from_ptr(unsafe {
        sb::C_SkImages_AdoptTextureFrom(
            context.native_mut(),
            backend_texture.native(),
            texture_origin,
            color_type.into_native(),
            alpha_type.into().unwrap_or(AlphaType::Premul),
            color_space.into().into_ptr_or_null(),
        )
    })
}

/// Creates GPU-backed [`Image`] from the provided GPU texture associated with context.
/// GPU texture must stay valid and unchanged until `texture_release_proc` is called by Skia.
/// Skia will call `texture_release_proc` with the passed-in `release_context` when [`Image`]
/// is deleted or no longer refers to the texture.
/// A non-null [`Image`] is returned if format of `backend_texture` is recognized and supported.
/// Recognized formats vary by GPU backend.
/// Note: When using a DDL recording context, `texture_release_proc` will be called on the
/// GPU thread after the DDL is played back on the direct context.
/// * `context` - GPU context
/// * `backend_texture` - texture residing on GPU
/// * `color_space` - This describes the color space of this image's contents, as
///                            seen after sampling. In general, if the format of the backend
///                            texture is SRGB, some linear `color_space` should be supplied
///                            (e.g., [`ColorSpace::new_srgb_linear()`]). If the format of the
///                            backend texture is linear, then the `color_space` should include
///                            a description of the transfer function as
///                            well (e.g., [`ColorSpace::new_srgb()`]).
/// * `texture_release_proc` - function called when texture can be released
/// * `release_context` - state passed to `texture_release_proc`
///
/// Returns: created [`Image`], or `None`
// TODO: add variant with TextureReleaseProc
pub fn borrow_texture_from(
    context: &mut RecordingContext,
    backend_texture: &BackendTexture,
    origin: SurfaceOrigin,
    color_type: ColorType,
    alpha_type: AlphaType,
    color_space: impl Into<Option<ColorSpace>>,
) -> Option<Image> {
    Image::from_ptr(unsafe {
        sb::C_SkImages_BorrowTextureFrom(
            context.native_mut(),
            backend_texture.native(),
            origin,
            color_type.into_native(),
            alpha_type,
            color_space.into().into_ptr_or_null(),
        )
    })
}

/// Creates a GPU-backed [`Image`] from pixmap. It is uploaded to GPU backend using context.
/// Created [`Image`] is available to other GPU contexts, and is available across thread
/// boundaries. All contexts must be in the same GPU share group, or otherwise
/// share resources.
/// When [`Image`] is no longer referenced, context releases texture memory
/// asynchronously.
/// [`ColorSpace`] of [`Image`] is determined by `pixmap.color_space()`.
/// [`Image`] is returned referring to GPU backend if context is not `None`,
/// format of data is recognized and supported, and if context supports moving
/// resources between contexts. Otherwise, pixmap pixel data is copied and [`Image`]
/// as returned in raster format if possible; `None` may be returned.
/// Recognized GPU formats vary by platform and GPU backend.
/// * `context` - GPU context
/// * `pixmap` - [`ImageInfo`], pixel address, and row bytes
/// * `build_mips` - create [`Image`] as mip map if `true`
/// * `limit_to_max_texture_size` - downscale image to GPU maximum texture size, if necessary
///
/// Returns: created [`Image`], or `None`
pub fn cross_context_texture_from_pixmap(
    context: &mut DirectContext,
    pixmap: &Pixmap,
    build_mips: bool,
    limit_to_max_texture_size: impl Into<Option<bool>>,
) -> Option<Image> {
    Image::from_ptr(unsafe {
        sb::C_SkImages_CrossContextTextureFromPixmap(
            context.native_mut(),
            pixmap.native(),
            build_mips,
            limit_to_max_texture_size.into().unwrap_or(false),
        )
    })
}

// TODO: TextureFromCompressedTexture

/// Creates a GPU-backed [`Image`] from compressed data.
/// This method will return an [`Image`] representing the compressed data.
/// If the GPU doesn't support the specified compression method, the data
/// will be decompressed and then wrapped in a GPU-backed image.
/// Note: one can query the supported compression formats via
/// [`RecordingContext::compressed_backend_format`].
/// * `context` - GPU context
/// * `data` - compressed data to store in [`Image`]
/// * `width` - width of full [`Image`]
/// * `height` - height of full [`Image`]
/// * `ty` - type of compression used
/// * `mipmapped` - does 'data' contain data for all the mipmap levels?
/// * `is_protected` - do the contents of 'data' require DRM protection (on Vulkan)?
///
/// Returns: created [`Image`], or `None`
pub fn texture_from_compressed_texture_data(
    context: &mut DirectContext,
    data: Data,
    dimensions: impl Into<ISize>,
    ty: TextureCompressionType,
    mipmapped: impl Into<Option<Mipmapped>>,
    is_protected: impl Into<Option<Protected>>,
) -> Option<Image> {
    let dimensions = dimensions.into();
    let mipmapped = mipmapped.into().unwrap_or(Mipmapped::No);
    let is_protected = is_protected.into().unwrap_or(Protected::No);

    Image::from_ptr(unsafe {
        sb::C_SkImages_TextureFromCompressedTextureData(
            context.native_mut(),
            data.into_ptr(),
            dimensions.width,
            dimensions.height,
            ty,
            mipmapped,
            is_protected,
        )
    })
}

///  Returns [`Image`] backed by GPU texture associated with context. Returned [`Image`] is
///  compatible with [`Surface`] created with `dst_color_space`. The returned [`Image`] respects
///  mipmapped setting; if mipmapped equals [`Mipmapped::Yes`], the backing texture
///  allocates mip map levels.
///  The mipmapped parameter is effectively treated as `No` if MIP maps are not supported by the
///  GPU.
///  Returns original [`Image`] if the image is already texture-backed, the context matches, and
///  mipmapped is compatible with the backing GPU texture. skgpu::Budgeted is ignored in this
///  case.
///  Returns `None` if context is `None`, or if [`Image`] was created with another
///  [`DirectContext`].
///  * `direct_context` - the [`DirectContext`] in play, if it exists
///  * `image` - a non-null pointer to an [`Image`].
///  * `mipmapped` - Whether created [`Image`] texture must allocate mip map levels.
///                  Defaults to `No`.
///  * `budgeted` - Whether to count a newly created texture for the returned image
///                 counts against the context's budget. Defaults to `Yes`.
///  
/// Returns: created [`Image`], or `None`
pub fn texture_from_image(
    direct_context: &mut DirectContext,
    image: &Image,
    mipmapped: Mipmapped,
    budgeted: Budgeted,
) -> Option<Image> {
    Image::from_ptr(unsafe {
        sb::C_SkImages_TextureFromImage(
            direct_context.native_mut(),
            image.native(),
            mipmapped,
            budgeted.into_native(),
        )
    })
}

/// Creates a GPU-backed [`Image`] from [`YUVAPixmaps`].
/// The image will remain planar with each plane converted to a texture using the passed
/// [`RecordingContext`].
/// [`YUVAPixmaps`] has a [`YUVAInfo`] which specifies the transformation from YUV to RGB.
/// The [`ColorSpace`] of the resulting RGB values is specified by `image_color_space`. This will
/// be the [`ColorSpace`] reported by the image and when drawn the RGB values will be converted
/// from this space into the destination space (if the destination is tagged).
/// Currently, this is only supported using the GPU backend and will fail if context is `None`.
/// [`YUVAPixmaps`] does not need to remain valid after this returns.
/// * `context` - GPU context
/// * `pixmaps` - The planes as pixmaps with supported [`YUVAInfo`] that
///               specifies conversion to RGB.
/// * `build_mips` - create internal YUVA textures as mip map if `k_yes`. This is
///                  silently ignored if the context does not support mip maps.
/// * `limit_to_max_texture_size` - downscale image to GPU maximum texture size, if necessary
/// * `image_color_space` - range of colors of the resulting image; may be `None`
///
/// Returns: created [`Image`], or `None`
pub fn texture_from_yuva_pixmaps(
    context: &mut RecordingContext,
    yuva_pixmaps: &crate::YUVAPixmaps,
    build_mips: impl Into<Option<Mipmapped>>,
    limit_to_max_texture_size: impl Into<Option<bool>>,
    image_color_space: impl Into<Option<ColorSpace>>,
) -> Option<Image> {
    Image::from_ptr(unsafe {
        sb::C_SkImages_TextureFromYUVAPixmaps(
            context.native_mut(),
            yuva_pixmaps.native(),
            build_mips.into().unwrap_or(Mipmapped::No),
            limit_to_max_texture_size.into().unwrap_or(false),
            image_color_space.into().into_ptr_or_null(),
        )
    })
}

/// Creates a GPU-backed [`Image`] from `YUV[A]` planar textures. This requires that the textures
/// stay valid for the lifetime of the image. The ReleaseContext can be used to know when it is
/// safe to either delete or overwrite the textures. If ReleaseProc is provided it is also called
/// before return on failure.
/// * `context` - GPU context
/// * `yuva_textures` - A set of textures containing YUVA data and a description of the
///                     data and transformation to RGBA.
/// * `image_color_space` - range of colors of the resulting image after conversion to RGB;
///                         may be `None`
/// * `texture_release_proc` - called when the backend textures can be released
/// * `release_context` - state passed to `texture_release_proc`
///
/// Returns: created [`Image`], or `None`
pub fn texture_from_yuva_textures(
    context: &mut RecordingContext,
    yuva_textures: &YUVABackendTextures,
    image_color_space: impl Into<Option<ColorSpace>>,
) -> Option<Image> {
    Image::from_ptr(unsafe {
        sb::C_SkImages_TextureFromYUVATextures(
            context.native_mut(),
            yuva_textures.native(),
            image_color_space.into().into_ptr_or_null(),
        )
    })
}

// TODO: PromiseTextureFrom
// TODO: PromiseTextureFromYUVA

/// Retrieves the existing backend texture. If [`Image`] is not a Ganesh-backend texture image
/// or otherwise does not have such a texture, `false` is returned. Otherwise, returned will
/// be set to the image's texture.
///
/// If `flush_pending_gr_context_io` is `true`, completes deferred I/O operations.
/// If origin in not `None`, copies location of content drawn into [`Image`].
/// * `out_texture` - Will be set to the underlying texture of the image if non-null.
/// * `flush_pending_gr_context_io` - flag to flush outstanding requests
/// * `origin` - Will be set to the origin orientation of the image if non-null.
///
/// Returns: `None` if a Ganesh backend texture cannot be retrieved.
pub fn get_backend_texture_from_image(
    image: &Image,
    flush_pending_gr_context_io: bool,
) -> Option<(BackendTexture, SurfaceOrigin)> {
    let mut origin = SurfaceOrigin::TopLeft;
    unsafe {
        let backend_texture = sb::C_SkImages_GetBackendTextureFromImage(
            image.native(),
            flush_pending_gr_context_io,
            &mut origin,
        );
        BackendTexture::from_native_if_valid(backend_texture)
    }
    .map(|texture| (texture, origin))
}

// TODO: MakeBackendTextureFromImage
// TODO: GetBackendTextureFromImage (legacy name)

/// Returns subset of this image as a texture-backed image.
///
/// Returns `None` if any of the following are true:
///   - Subset is empty
///   - Subset is not contained inside the image's bounds
///   - Pixels in the source image could not be read or copied
///   - The source image is texture-backed and context does not match the source image's context.
///
/// * `context` - the [`DirectContext`] to which the subset should be uploaded.
/// * `subset` - bounds of returned [`Image`]
///
/// Returns: the subsetted image, uploaded as a texture, or `None`
pub fn subset_texture_from(
    context: &mut DirectContext,
    image: &Image,
    subset: impl AsRef<IRect>,
) -> Option<Image> {
    Image::from_ptr(unsafe {
        sb::C_SkImages_SubsetTextureFrom(
            context.native_mut(),
            image.native(),
            subset.as_ref().native(),
        )
    })
}
