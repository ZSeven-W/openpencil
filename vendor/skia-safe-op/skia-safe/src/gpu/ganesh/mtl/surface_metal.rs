use crate::{gpu, prelude::*, Surface, SurfaceProps};
use skia_bindings as sb;

/// Creates [`Surface`] from CAMetalLayer.
/// Returned [`Surface`] takes a reference on the CAMetalLayer. The ref on the layer will be
/// released when the [`Surface`] is destroyed.
///
/// Only available when Metal API is enabled.
///
/// Will grab the current drawable from the layer and use its texture as a `backend_rt` to
/// create a renderable surface.
///
/// * `context` - GPU context
/// * `layer` - [`gpu::mtl::Handle`] (expected to be a CAMetalLayer*)
/// * `sample_cnt` - samples per pixel, or 0 to disable full scene anti-aliasing
/// * `color_space` - range of colors; may be `None`
/// * `surface_props` - LCD striping orientation and setting for device independent
///                        fonts; may be `None`
/// * `drawable` - Pointer to drawable to be filled in when this surface is
///                        instantiated; may not be `None`
///
/// Returns: created [`Surface`], or `None`
#[allow(clippy::missing_safety_doc)]
#[allow(clippy::too_many_arguments)]
pub unsafe fn wrap_ca_metal_layer(
    context: &mut gpu::RecordingContext,
    layer: gpu::mtl::Handle,
    origin: gpu::SurfaceOrigin,
    sample_cnt: impl Into<Option<usize>>,
    color_type: crate::ColorType,
    color_space: impl Into<Option<crate::ColorSpace>>,
    surface_props: Option<&SurfaceProps>,
    drawable: *mut gpu::mtl::Handle,
) -> Option<Surface> {
    Surface::from_ptr(sb::C_SkSurfaces_WrapCAMetalLayer(
        context.native_mut(),
        layer,
        origin,
        sample_cnt.into().unwrap_or(0).try_into().unwrap(),
        color_type.into_native(),
        color_space.into().into_ptr_or_null(),
        surface_props.native_ptr_or_null(),
        drawable,
    ))
}

/// Creates [`Surface`] from MTKView.
/// Returned [`Surface`] takes a reference on the MTKView. The ref on the layer will be
/// released when the [`Surface`] is destroyed.
///
/// Only available when Metal API is enabled.
///
/// Will grab the current drawable from the layer and use its texture as a `backend_rt` to
/// create a renderable surface.
///
/// * `context` - GPU context
/// * `layer` - [`gpu::mtl::Handle`] (expected to be a MTKView*)
/// * `sample_cnt` - samples per pixel, or 0 to disable full scene anti-aliasing
/// * `color_space` - range of colors; may be `None`
/// * `surface_props` - LCD striping orientation and setting for device independent
///                        fonts; may be `None`
///
/// Returns: created [`Surface`], or `None`
#[allow(clippy::missing_safety_doc)]
#[cfg(feature = "metal")]
pub unsafe fn wrap_mtk_view(
    context: &mut gpu::RecordingContext,
    mtk_view: gpu::mtl::Handle,
    origin: gpu::SurfaceOrigin,
    sample_count: impl Into<Option<usize>>,
    color_type: crate::ColorType,
    color_space: impl Into<Option<crate::ColorSpace>>,
    surface_props: Option<&SurfaceProps>,
) -> Option<Surface> {
    Surface::from_ptr(sb::C_SkSurfaces_WrapMTKView(
        context.native_mut(),
        mtk_view,
        origin,
        sample_count.into().unwrap_or(0).try_into().unwrap(),
        color_type.into_native(),
        color_space.into().into_ptr_or_null(),
        surface_props.native_ptr_or_null(),
    ))
}
