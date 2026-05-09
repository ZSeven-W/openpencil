use skia_bindings as sb;

use crate::{gpu, prelude::*, surface::BackendHandleAccess, ImageInfo, Surface, SurfaceProps};

/// Returns [`Surface`] on GPU indicated by context. Allocates memory for pixels, based on the
/// width, height, and [`crate::ColorType`] in [`ImageInfo`].  budgeted selects whether allocation
/// for pixels is tracked by context. `image_info` describes the pixel format in
/// [`crate::ColorType`], and transparency in [`crate::AlphaType`], and color matching in
/// [`crate::ColorSpace`].
///
/// `sample_count` requests the number of samples per pixel. Pass zero to disable multi-sample
/// anti-aliasing.  The request is rounded up to the next supported count, or rounded down if it is
/// larger than the maximum supported count.
///
/// `surface_origin` pins either the top-left or the bottom-left corner to the origin.
///
/// `should_create_with_mips` hints that [`crate::Image`] returned by [`Surface::image_snapshot`] is
/// mip map.
///
/// * `context` - GPU context
/// * `image_info` - width, height, [`crate::ColorType`], [`crate::AlphaType`],
///                              [`crate::ColorSpace`]; width, or height, or both, may be zero
/// * `sample_count` - samples per pixel, or 0 to disable full scene anti-aliasing
/// * `surface_props` - LCD striping orientation and setting for device independent fonts; may be
///                              `None`
/// * `should_create_with_mips` - hint that [`Surface`] will host mip map images
///
/// Returns: [`Surface`] if all parameters are valid; otherwise, `None`
#[allow(clippy::too_many_arguments)]
pub fn render_target(
    context: &mut gpu::RecordingContext,
    budgeted: gpu::Budgeted,
    image_info: &ImageInfo,
    sample_count: impl Into<Option<usize>>,
    surface_origin: impl Into<Option<gpu::SurfaceOrigin>>,
    surface_props: Option<&SurfaceProps>,
    should_create_with_mips: impl Into<Option<bool>>,
    is_protected: impl Into<Option<bool>>,
) -> Option<Surface> {
    Surface::from_ptr(unsafe {
        sb::C_SkSurfaces_RenderTarget(
            context.native_mut(),
            budgeted.into_native(),
            image_info.native(),
            sample_count.into().unwrap_or(0).try_into().unwrap(),
            surface_origin
                .into()
                .unwrap_or(gpu::SurfaceOrigin::BottomLeft),
            surface_props.native_ptr_or_null(),
            should_create_with_mips.into().unwrap_or(false),
            is_protected.into().unwrap_or(false),
        )
    })
}

/// Wraps a GPU-backed texture into [`Surface`]. Caller must ensure the texture is
/// valid for the lifetime of returned [`Surface`]. If `sample_cnt` greater than zero,
/// creates an intermediate MSAA [`Surface`] which is used for drawing `backend_texture`.
///
/// [`Surface`] is returned if all parameters are valid. `backend_texture` is valid if
/// its pixel configuration agrees with `color_space` and context; for instance, if
/// `backend_texture` has an sRGB configuration, then context must support sRGB,
/// and `color_space` must be present. Further, `backend_texture` width and height must
/// not exceed context capabilities, and the context must be able to support
/// back-end textures.
///
/// * `context` - GPU context
/// * `backend_texture` - texture residing on GPU
/// * `sample_cnt` - samples per pixel, or 0 to disable full scene anti-aliasing
/// * `color_space` - range of colors; may be `None`
/// * `surface_props` - LCD striping orientation and setting for device independent
///                            fonts; may be `None`
///
/// Returns: [`Surface`] if all parameters are valid; otherwise, `None`
pub fn wrap_backend_texture(
    context: &mut gpu::RecordingContext,
    backend_texture: &gpu::BackendTexture,
    origin: gpu::SurfaceOrigin,
    sample_cnt: impl Into<Option<usize>>,
    color_type: crate::ColorType,
    color_space: impl Into<Option<crate::ColorSpace>>,
    surface_props: Option<&SurfaceProps>,
) -> Option<Surface> {
    Surface::from_ptr(unsafe {
        sb::C_SkSurfaces_WrapBackendTexture(
            context.native_mut(),
            backend_texture.native(),
            origin,
            sample_cnt.into().unwrap_or(0).try_into().unwrap(),
            color_type.into_native(),
            color_space.into().into_ptr_or_null(),
            surface_props.native_ptr_or_null(),
        )
    })
}

/// Wraps a GPU-backed buffer into [`Surface`]. Caller must ensure `backend_render_target`
/// is valid for the lifetime of returned [`Surface`].
///
/// [`Surface`] is returned if all parameters are valid. `backend_render_target` is valid if
/// its pixel configuration agrees with `color_space` and context; for instance, if
/// `backend_render_target` has an sRGB configuration, then context must support sRGB,
/// and `color_space` must be present. Further, `backend_render_target` width and height must
/// not exceed context capabilities, and the context must be able to support
/// back-end render targets.
///
/// * `context` - GPU context
/// * `backend_render_target` - GPU intermediate memory buffer
/// * `origin` - origin of canvas
/// * `color_type` - type of colors in the buffer
/// * `color_space` - range of colors
/// * `surface_props` - LCD striping orientation and setting for device independent
///                                 fonts; may be `None`
///
/// Returns: [`Surface`] if all parameters are valid; otherwise, `None`
pub fn wrap_backend_render_target(
    context: &mut gpu::RecordingContext,
    backend_render_target: &gpu::BackendRenderTarget,
    origin: gpu::SurfaceOrigin,
    color_type: crate::ColorType,
    color_space: impl Into<Option<crate::ColorSpace>>,
    surface_props: Option<&SurfaceProps>,
) -> Option<Surface> {
    Surface::from_ptr(unsafe {
        sb::C_SkSurfaces_WrapBackendRenderTarget(
            context.native_mut(),
            backend_render_target.native(),
            origin,
            color_type.into_native(),
            color_space.into().into_ptr_or_null(),
            surface_props.native_ptr_or_null(),
        )
    })
}

/// Retrieves the back-end texture. If [`Surface`] has no back-end texture, `None`
/// is returned.
///
/// The returned [`gpu::BackendTexture`] should be discarded if the [`Surface`] is drawn to or deleted.
///
/// Returns: GPU texture reference; `None` on failure
pub fn get_backend_texture(
    surface: &mut Surface,
    handle_access: BackendHandleAccess,
) -> Option<gpu::BackendTexture> {
    unsafe {
        let ptr = sb::C_SkSurfaces_GetBackendTexture(surface.native_mut(), handle_access);
        gpu::BackendTexture::from_native_if_valid(ptr)
    }
}

/// Retrieves the back-end render target. If [`Surface`] has no back-end render target, `None`
/// is returned.
///
/// The returned [`gpu::BackendRenderTarget`] should be discarded if the [`Surface`] is drawn to
/// or deleted.
///
/// Returns: GPU render target reference; `None` on failure
pub fn get_backend_render_target(
    surface: &mut Surface,
    handle_access: BackendHandleAccess,
) -> Option<gpu::BackendRenderTarget> {
    unsafe {
        let mut backend_render_target = construct(|rt| sb::C_GrBackendRenderTarget_Construct(rt));
        sb::C_SkSurfaces_GetBackendRenderTarget(
            surface.native_mut(),
            handle_access,
            &mut backend_render_target,
        );

        gpu::BackendRenderTarget::from_native_c_if_valid(backend_render_target)
    }
}

/// If a surface is a Ganesh-backed surface, is being drawn with MSAA, and there is a resolve
/// texture, this call will insert a resolve command into the stream of gpu commands. In order
/// for the resolve to actually have an effect, the work still needs to be flushed and submitted
/// to the GPU after recording the resolve command. If a resolve is not supported or the
/// [`Surface`] has no dirty work to resolve, then this call is a no-op.
///
/// This call is most useful when the [`Surface`] is created by wrapping a single sampled gpu
/// texture, but asking Skia to render with MSAA. If the client wants to use the wrapped texture
/// outside of Skia, the only way to trigger a resolve is either to call this command or use
/// [`gpu::DirectContext::flush`].
pub fn resolve_msaa(surface: &mut Surface) {
    unsafe { sb::C_SkSurfaces_ResolveMSAA(surface.native_mut()) }
}
