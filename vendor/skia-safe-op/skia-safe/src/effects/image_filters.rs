use std::ptr;

use skia_bindings::{self as sb, SkImageFilter, SkRect};

use crate::{
    prelude::*, scalar, Blender, Color, Color4f, ColorChannel, ColorFilter, ColorSpace,
    CubicResampler, IPoint, IRect, ISize, Image, ImageFilter, Matrix, Picture, Point3, Rect,
    SamplingOptions, Shader, TileMode, Vector,
};

/// This is just a convenience type to allow passing [`IRect`]s, [`Rect`]s, and optional references
/// to those types as a crop rect for the image filter factories. It's not intended to be used
/// directly.
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct CropRect(Option<Rect>);

impl CropRect {
    pub const NO_CROP_RECT: CropRect = CropRect(None);

    pub fn rect(&self) -> Option<Rect> {
        self.0
    }

    fn native(&self) -> *const SkRect {
        match &self.0 {
            None => ptr::null(),
            Some(r) => r.native(),
        }
    }
}

impl Default for CropRect {
    fn default() -> Self {
        CropRect::NO_CROP_RECT
    }
}

impl From<Option<CropRect>> for CropRect {
    fn from(opt: Option<CropRect>) -> Self {
        opt.unwrap_or(Self::NO_CROP_RECT)
    }
}

impl From<&CropRect> for CropRect {
    fn from(cr: &CropRect) -> Self {
        *cr
    }
}

impl From<IRect> for CropRect {
    fn from(r: IRect) -> Self {
        Self(Some(Rect::from(r)))
    }
}

impl From<&IRect> for CropRect {
    fn from(r: &IRect) -> Self {
        Self::from(*r)
    }
}

impl From<Rect> for CropRect {
    fn from(r: Rect) -> Self {
        Self(Some(r))
    }
}

impl From<&Rect> for CropRect {
    fn from(r: &Rect) -> Self {
        Self(Some(*r))
    }
}

/// Create a filter that implements a custom blend mode. Each output pixel is the result of
/// combining the corresponding background and foreground pixels using the 4 coefficients:
///    k1 * foreground * background + k2 * foreground + k3 * background + k4
/// * `k1`, `k2`, `k3`, `k4` The four coefficients used to combine the foreground and background.
/// * `enforce_pm_color` - If `true`, the RGB channels will be clamped to the calculated alpha.
/// * `background` - The background content, using the source bitmap when this is null.
/// * `foreground` - The foreground content, using the source bitmap when this is null.
/// * `crop_rect` - Optional rectangle that crops the inputs and output.
#[allow(clippy::too_many_arguments)]
pub fn arithmetic(
    k1: scalar,
    k2: scalar,
    k3: scalar,
    k4: scalar,
    enforce_pm_color: bool,
    background: impl Into<Option<ImageFilter>>,
    foreground: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Arithmetic(
            k1,
            k2,
            k3,
            k4,
            enforce_pm_color,
            background.into().into_ptr_or_null(),
            foreground.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// This filter takes an [`crate::BlendMode`] and uses it to composite the two filters together.
/// * `blender` - The blender that defines the compositing operation
/// * `background` - The Dst pixels used in blending, if null the source bitmap is used.
/// * `foreground` - The Src pixels used in blending, if null the source bitmap is used.
/// * `crop_rect``         Optional rectangle to crop input and output.
pub fn blend(
    mode: impl Into<Blender>,
    background: impl Into<Option<ImageFilter>>,
    foreground: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Blend(
            mode.into().into_ptr(),
            background.into().into_ptr_or_null(),
            foreground.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that blurs its input by the separate X and Y sigmas. The provided tile mode
/// is used when the blur kernel goes outside the input image.
/// * `sigma_x` - The Gaussian sigma value for blurring along the X axis.
/// * `sigma_y` - The Gaussian sigma value for blurring along the Y axis.
/// * `tile_mode` - The tile mode applied at edges .
/// * `input` - The input filter that is blurred, uses source bitmap if this is null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn blur(
    (sigma_x, sigma_y): (scalar, scalar),
    tile_mode: impl Into<Option<TileMode>>,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Blur(
            sigma_x,
            sigma_y,
            tile_mode.into().unwrap_or(TileMode::Decal),
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

pub fn color_filter(
    cf: impl Into<ColorFilter>,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_ColorFilter(
            cf.into().into_ptr(),
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that composes 'inner' with 'outer', such that the results of 'inner' are
/// treated as the source bitmap passed to 'outer', i.e. result = outer(inner(source)).
/// * `outer` - The outer filter that evaluates the results of inner.
/// * `inner` - The inner filter that produces the input to outer.
pub fn compose(
    outer: impl Into<ImageFilter>,
    inner: impl Into<ImageFilter>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Compose(outer.into().into_ptr(), inner.into().into_ptr())
    })
}

/// Create a filter that applies a crop to the result of the 'input' filter. Pixels within the
/// crop rectangle are unmodified from what 'input' produced. Pixels outside of crop match the
/// provided [`TileMode`] (defaulting to `Decal`).
///
/// * `rect` - The cropping geometry
/// * `tile_mode` - The tile mode applied to pixels *outside* of 'crop'
/// * `input` - The input filter that is cropped, uses source image if this is `None`
pub fn crop(
    rect: impl AsRef<Rect>,
    tile_mode: impl Into<Option<TileMode>>,
    input: impl Into<Option<ImageFilter>>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Crop(
            rect.as_ref().native(),
            tile_mode.into().unwrap_or(TileMode::Decal),
            input.into().into_ptr_or_null(),
        )
    })
}

/// Create a filter that moves each pixel in its color input based on an (x,y) vector encoded
/// in its displacement input filter. Two color components of the displacement image are
/// mapped into a vector as `scale * (color[xChannel], color[yChannel])`, where the channel
/// selectors are one of R, G, B, or A.
/// * `x_channel_selector` - RGBA channel that encodes the x displacement per pixel.
/// * `y_channel_selector` - RGBA channel that encodes the y displacement per pixel.
/// * `scale` - Scale applied to displacement extracted from image.
/// * `displacement` - The filter defining the displacement image, or `None` to use source.
/// * `color` - The filter providing the color pixels to be displaced. If `None`,
///                         it will use the source.
/// * `crop_rect` - Optional rectangle that crops the color input and output.
pub fn displacement_map(
    (x_channel_selector, y_channel_selector): (ColorChannel, ColorChannel),
    scale: scalar,
    displacement: impl Into<Option<ImageFilter>>,
    color: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_DisplacementMap(
            x_channel_selector,
            y_channel_selector,
            scale,
            displacement.into().into_ptr_or_null(),
            color.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that draws a drop shadow under the input content. This filter produces an
/// image that includes the inputs' content.
/// * `offset` - The offset of the shadow.
/// * `sigma_x` - The blur radius for the shadow, along the X axis.
/// * `sigma_y` - The blur radius for the shadow, along the Y axis.
/// * `color` - The color of the drop shadow.
/// * `color_space` - The color space of the drop shadow color.
/// * `input` - The input filter, or will use the source bitmap if this is null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn drop_shadow(
    offset: impl Into<Vector>,
    (sigma_x, sigma_y): (scalar, scalar),
    color: impl Into<Color4f>,
    color_space: impl Into<Option<ColorSpace>>,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    let delta = offset.into();
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_DropShadow(
            delta.x,
            delta.y,
            sigma_x,
            sigma_y,
            color.into().native(),
            color_space.into().into_ptr_or_null(),
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that renders a drop shadow, in exactly the same manner as ::DropShadow,
/// except that the resulting image does not include the input content. This allows the shadow
/// and input to be composed by a filter DAG in a more flexible manner.
/// * `offset` - The offset of the shadow.
/// * `sigma_x` - The blur radius for the shadow, along the X axis.
/// * `sigma_y` - The blur radius for the shadow, along the Y axis.
/// * `color` - The color of the drop shadow.
/// * `color_space` - The color space of the drop shadow color.
/// * `input` - The input filter, or will use the source bitmap if this is null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn drop_shadow_only(
    offset: impl Into<Vector>,
    (sigma_x, sigma_y): (scalar, scalar),
    color: impl Into<Color4f>,
    color_space: impl Into<Option<ColorSpace>>,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    let delta = offset.into();

    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_DropShadowOnly(
            delta.x,
            delta.y,
            sigma_x,
            sigma_y,
            color.into().native(),
            color_space.into().into_ptr_or_null(),
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that always produces transparent black.
pub fn empty() -> ImageFilter {
    ImageFilter::from_ptr(unsafe { sb::C_SkImageFilters_Empty() }).unwrap()
}
/// Create a filter that draws the 'src_rect' portion of image into 'dst_rect' using the given
/// filter quality. Similar to [`crate::Canvas::draw_image_rect()`].
///
/// * `image` - The image that is output by the filter, subset by 'srcRect'.
/// * `src_rect` - The source pixels sampled into 'dstRect'
/// * `dst_rect` - The local rectangle to draw the image into.
/// * `sampling` - The sampling to use when drawing the image.
pub fn image<'a>(
    image: impl Into<Image>,
    src_rect: impl Into<Option<&'a Rect>>,
    dst_rect: impl Into<Option<&'a Rect>>,
    sampling: impl Into<Option<SamplingOptions>>,
) -> Option<ImageFilter> {
    let image = image.into();
    let image_rect = Rect::from_iwh(image.width(), image.height());
    let src_rect = src_rect.into().unwrap_or(&image_rect);
    let dst_rect = dst_rect.into().unwrap_or(&image_rect);
    let sampling_options: SamplingOptions = sampling.into().unwrap_or_else(|| {
        CubicResampler {
            b: 1.0 / 3.0,
            c: 1.0 / 3.0,
        }
        .into()
    });

    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Image(
            image.into_ptr(),
            src_rect.as_ref().native(),
            dst_rect.as_ref().native(),
            sampling_options.native(),
        )
    })
}

/// Create a filter that fills 'lens_bounds' with a magnification of the input.
///
/// * `lens_bounds` - The outer bounds of the magnifier effect
/// * `zoom_amount` - The amount of magnification applied to the input image
/// * `inset` - The size or width of the fish-eye distortion around the magnified content
/// * `sampling` - The [`SamplingOptions`] applied to the input image when magnified
/// * `input` - The input filter that is magnified; if null the source bitmap is used
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn magnifier(
    lens_bounds: impl AsRef<Rect>,
    zoom_amount: scalar,
    inset: scalar,
    sampling: SamplingOptions,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Magnifier(
            lens_bounds.as_ref().native(),
            zoom_amount,
            inset,
            sampling.native(),
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that applies an NxM image processing kernel to the input image. This can be
/// used to produce effects such as sharpening, blurring, edge detection, etc.
/// * `kernel_size` - The kernel size in pixels, in each dimension (N by M).
/// * `kernel` - The image processing kernel. Must contain N * M elements, in row order.
/// * `gain` - A scale factor applied to each pixel after convolution. This can be
///                      used to normalize the kernel, if it does not already sum to 1.
/// * `bias` - A bias factor added to each pixel after convolution.
/// * `kernel_offset` - An offset applied to each pixel coordinate before convolution.
///                      This can be used to center the kernel over the image
///                      (e.g., a 3x3 kernel should have an offset of {1, 1}).
/// * `tile_mode` - How accesses outside the image are treated.
/// * `convolve_alpha` - If `true`, all channels are convolved. If `false`, only the RGB channels
///                      are convolved, and alpha is copied from the source image.
/// * `input` - The input image filter, if null the source bitmap is used instead.
/// * `crop_rect` - Optional rectangle to which the output processing will be limited.
#[allow(clippy::too_many_arguments)]
pub fn matrix_convolution(
    kernel_size: impl Into<ISize>,
    kernel: &[scalar],
    gain: scalar,
    bias: scalar,
    kernel_offset: impl Into<IPoint>,
    tile_mode: TileMode,
    convolve_alpha: bool,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    let kernel_size = kernel_size.into();
    assert_eq!(
        (kernel_size.width * kernel_size.height) as usize,
        kernel.len()
    );
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_MatrixConvolution(
            kernel_size.native(),
            kernel.as_ptr(),
            gain,
            bias,
            kernel_offset.into().native(),
            tile_mode,
            convolve_alpha,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that transforms the input image by 'matrix'. This matrix transforms the
/// local space, which means it effectively happens prior to any transformation coming from the
/// [`crate::Canvas`] initiating the filtering.
/// * `matrix` - The matrix to apply to the original content.
/// * `sampling` - How the image will be sampled when it is transformed
/// * `input` - The image filter to transform, or null to use the source image.
pub fn matrix_transform(
    matrix: &Matrix,
    sampling: impl Into<SamplingOptions>,
    input: impl Into<Option<ImageFilter>>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_MatrixTransform(
            matrix.native(),
            sampling.into().native(),
            input.into().into_ptr_or_null(),
        )
    })
}

/// Create a filter that merges the filters together by drawing their results in order
/// with src-over blending.
/// * `filters` - The input filter array to merge. Any None
///                 filter pointers will use the source bitmap instead.
/// * `crop_rect` - Optional rectangle that crops all input filters and the output.
pub fn merge(
    filters: impl IntoIterator<Item = Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    let filter_ptrs: Vec<*mut SkImageFilter> =
        filters.into_iter().map(|f| f.into_ptr_or_null()).collect();
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Merge(
            filter_ptrs.as_ptr(),
            filter_ptrs.len().try_into().unwrap(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that offsets the input filter by the given vector.
/// * `offset` - The offset in local space that the image is shifted.
/// * `input` - The input that will be moved, if null the source bitmap is used instead.
/// * `crop_rect` - Optional rectangle to crop the input and output.
pub fn offset(
    offset: impl Into<Vector>,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    let delta = offset.into();
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Offset(
            delta.x,
            delta.y,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that produces the [`Picture`] as its output, clipped to both 'target_rect' and
/// the picture's internal cull rect.
///
/// * `pic` - The picture that is drawn for the filter output.
/// * `target_rect` - The drawing region for the picture.
pub fn picture<'a>(
    pic: impl Into<Picture>,
    target_rect: impl Into<Option<&'a Rect>>,
) -> Option<ImageFilter> {
    let picture = pic.into();
    let picture_rect = picture.cull_rect();
    let target_rect = target_rect.into().unwrap_or(&picture_rect);

    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Picture(picture.into_ptr(), target_rect.native())
    })
}

pub fn runtime_shader(
    builder: &RuntimeShaderBuilder,
    child_shader_name: impl AsRef<str>,
    input: impl Into<Option<ImageFilter>>,
) -> Option<ImageFilter> {
    let child_shader_name = child_shader_name.as_ref();
    unsafe {
        ImageFilter::from_ptr(sb::C_SkImageFilters_RuntimeShader(
            builder.native() as *const _,
            child_shader_name.as_ptr() as *const _,
            child_shader_name.len(),
            input.into().into_ptr_or_null(),
        ))
    }
}

pub use skia_bindings::SkImageFilters_Dither as Dither;

use super::runtime_effect::RuntimeShaderBuilder;
variant_name!(Dither::Yes);

/// Create a filter that fills the output with the per-pixel evaluation of the [`Shader`]. The
/// shader is defined in the image filter's local coordinate system, so will automatically
/// be affected by [`crate::Canvas`]'s transform.
///
/// Like `image()` and Picture(), this is a leaf filter that can be used to introduce inputs to
/// a complex filter graph, but should generally be combined with a filter that as at least
/// one null input to use the implicit source image.
///
/// * `shader` - The shader that fills the result image
pub fn shader(shader: impl Into<Shader>, crop_rect: impl Into<CropRect>) -> Option<ImageFilter> {
    shader_with_dither(shader, Dither::No, crop_rect)
}

pub fn shader_with_dither(
    shader: impl Into<Shader>,
    dither: Dither,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Shader(shader.into().into_ptr(), dither, crop_rect.into().native())
    })
}

/// Create a tile image filter.
/// * `src` - Defines the pixels to tile
/// * `dst` - Defines the pixel region that the tiles will be drawn to
/// * `input` - The input that will be tiled, if null the source bitmap is used instead.
pub fn tile(
    src: impl AsRef<Rect>,
    dst: impl AsRef<Rect>,
    input: impl Into<Option<ImageFilter>>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Tile(
            src.as_ref().native(),
            dst.as_ref().native(),
            input.into().into_ptr_or_null(),
        )
    })
}

/// Create a filter that dilates each input pixel's channel values to the max value within the
/// given radii along the x and y axes.
/// * `radius_x` - The distance to dilate along the x axis to either side of each pixel.
/// * `radius_y` - The distance to dilate along the y axis to either side of each pixel.
/// * `input` - The image filter that is dilated, using source bitmap if this is null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn dilate(
    (radius_x, radius_y): (scalar, scalar),
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Dilate(
            radius_x,
            radius_y,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that erodes each input pixel's channel values to the minimum channel value
/// within the given radii along the x and y axes.
/// * `radius_x` - The distance to erode along the x axis to either side of each pixel.
/// * `radius_y` - The distance to erode along the y axis to either side of each pixel.
/// * `input` - The image filter that is eroded, using source bitmap if this is null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn erode(
    (radius_x, radius_y): (scalar, scalar),
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_Erode(
            radius_x,
            radius_y,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that calculates the diffuse illumination from a distant light source,
/// interpreting the alpha channel of the input as the height profile of the surface (to
/// approximate normal vectors).
/// * `direction` - The direction to the distance light.
/// * `light_color` - The color of the diffuse light source.
/// * `surface_scale` - Scale factor to transform from alpha values to physical height.
/// * `kd` - Diffuse reflectance coefficient.
/// * `input` - The input filter that defines surface normals (as alpha), or uses the
///                     source bitmap when null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn distant_lit_diffuse(
    direction: impl Into<Point3>,
    light_color: impl Into<Color>,
    surface_scale: scalar,
    kd: scalar,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_DistantLitDiffuse(
            direction.into().native(),
            light_color.into().into_native(),
            surface_scale,
            kd,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that calculates the diffuse illumination from a point light source, using
/// alpha channel of the input as the height profile of the surface (to approximate normal
/// vectors).
/// * `location` - The location of the point light.
/// * `light_color` - The color of the diffuse light source.
/// * `surface_scale` - Scale factor to transform from alpha values to physical height.
/// * `kd` - Diffuse reflectance coefficient.
/// * `input` - The input filter that defines surface normals (as alpha), or uses the
///                     source bitmap when `None`.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn point_lit_diffuse(
    location: impl Into<Point3>,
    light_color: impl Into<Color>,
    surface_scale: scalar,
    kd: scalar,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_PointLitDiffuse(
            location.into().native(),
            light_color.into().into_native(),
            surface_scale,
            kd,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that calculates the diffuse illumination from a spot light source, using
/// alpha channel of the input as the height profile of the surface (to approximate normal
/// vectors). The spot light is restricted to be within 'cutoff_angle' of the vector between
/// the location and target.
/// * `location` - The location of the spot light.
/// * `target` - The location that the spot light is point towards
/// * `falloff_exponent` - Exponential falloff parameter for illumination outside of `cutoff_angle`
/// * `cutoff_angle` - Maximum angle from lighting direction that receives full light
/// * `light_color` - The color of the diffuse light source.
/// * `surface_scale` - Scale factor to transform from alpha values to physical height.
/// * `kd` - Diffuse reflectance coefficient.
/// * `input` - The input filter that defines surface normals (as alpha), or uses the
///                        source bitmap when null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
#[allow(clippy::too_many_arguments)]
pub fn spot_lit_diffuse(
    location: impl Into<Point3>,
    target: impl Into<Point3>,
    falloff_exponent: scalar,
    cutoff_angle: scalar,
    light_color: impl Into<Color>,
    surface_scale: scalar,
    kd: scalar,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_SpotLitDiffuse(
            location.into().native(),
            target.into().native(),
            falloff_exponent,
            cutoff_angle,
            light_color.into().into_native(),
            surface_scale,
            kd,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that calculates the specular illumination from a distant light source,
/// interpreting the alpha channel of the input as the height profile of the surface (to
/// approximate normal vectors).
/// * `direction` - The direction to the distance light.
/// * `light_color` - The color of the specular light source.
/// * `surface_scale` - Scale factor to transform from alpha values to physical height.
/// * `ks` - Specular reflectance coefficient.
/// * `shininess` - The specular exponent determining how shiny the surface is.
/// * `input` - The input filter that defines surface normals (as alpha), or uses the
///                     source bitmap when `None`.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn distant_lit_specular(
    direction: impl Into<Point3>,
    light_color: impl Into<Color>,
    surface_scale: scalar,
    ks: scalar,
    shininess: scalar,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_ImageFilters_DistantLitSpecular(
            direction.into().native(),
            light_color.into().into_native(),
            surface_scale,
            ks,
            shininess,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that calculates the specular illumination from a point light source, using
/// alpha channel of the input as the height profile of the surface (to approximate normal
/// vectors).
/// * `location` - The location of the point light.
/// * `light_color` - The color of the specular light source.
/// * `surface_scale` - Scale factor to transform from alpha values to physical height.
/// * `ks` - Specular reflectance coefficient.
/// * `shininess` - The specular exponent determining how shiny the surface is.
/// * `input` - The input filter that defines surface normals (as alpha), or uses the
///                     source bitmap when `None`.
/// * `crop_rect` - Optional rectangle that crops the input and output.
pub fn point_lit_specular(
    location: impl Into<Point3>,
    light_color: impl Into<Color>,
    surface_scale: scalar,
    ks: scalar,
    shininess: scalar,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_PointLitSpecular(
            location.into().native(),
            light_color.into().into_native(),
            surface_scale,
            ks,
            shininess,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

/// Create a filter that calculates the specular illumination from a spot light source, using
/// alpha channel of the input as the height profile of the surface (to approximate normal
/// vectors). The spot light is restricted to be within 'cutoff_angle' of the vector between
/// the location and target.
/// * `location` - The location of the spot light.
/// * `target` - The location that the spot light is point towards
/// * `falloff_exponent` - Exponential falloff parameter for illumination outside of `cutoff_angle`
/// * `cutoff_angle` - Maximum angle from lighting direction that receives full light
/// * `light_color` - The color of the specular light source.
/// * `surface_scale` - Scale factor to transform from alpha values to physical height.
/// * `ks` - Specular reflectance coefficient.
/// * `shininess` - The specular exponent determining how shiny the surface is.
/// * `input` - The input filter that defines surface normals (as alpha), or uses the
///                        source bitmap when null.
/// * `crop_rect` - Optional rectangle that crops the input and output.
#[allow(clippy::too_many_arguments)]
pub fn spot_lit_specular(
    location: impl Into<Point3>,
    target: impl Into<Point3>,
    falloff_exponent: scalar,
    cutoff_angle: scalar,
    light_color: impl Into<Color>,
    surface_scale: scalar,
    ks: scalar,
    shininess: scalar,
    input: impl Into<Option<ImageFilter>>,
    crop_rect: impl Into<CropRect>,
) -> Option<ImageFilter> {
    ImageFilter::from_ptr(unsafe {
        sb::C_SkImageFilters_SpotLitSpecular(
            location.into().native(),
            target.into().native(),
            falloff_exponent,
            cutoff_angle,
            light_color.into().into_native(),
            surface_scale,
            ks,
            shininess,
            input.into().into_ptr_or_null(),
            crop_rect.into().native(),
        )
    })
}

impl ImageFilter {
    /// [`arithmetic()`]
    pub fn arithmetic<'a>(
        inputs: impl Into<ArithmeticFPInputs>,
        background: impl Into<Option<Self>>,
        foreground: impl Into<Option<Self>>,
        crop_rect: impl Into<Option<&'a IRect>>,
    ) -> Option<Self> {
        let inputs = inputs.into();
        arithmetic(
            inputs.k[0],
            inputs.k[1],
            inputs.k[2],
            inputs.k[3],
            inputs.enforce_pm_color,
            background,
            foreground,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`blur()`]
    pub fn blur<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        sigma: (scalar, scalar),
        tile_mode: impl Into<Option<crate::TileMode>>,
    ) -> Option<Self> {
        blur(sigma, tile_mode, self, crop_rect.into().map(|r| r.into()))
    }

    /// [`color_filter()`]
    pub fn color_filter<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        cf: impl Into<ColorFilter>,
    ) -> Option<Self> {
        color_filter(cf, self, crop_rect.into().map(|r| r.into()))
    }

    /// [`compose()`]
    pub fn compose(outer: impl Into<ImageFilter>, inner: impl Into<ImageFilter>) -> Option<Self> {
        compose(outer, inner)
    }

    /// [`crop()`]
    pub fn crop(
        rect: impl AsRef<Rect>,
        tile_mode: impl Into<Option<TileMode>>,
        input: impl Into<Option<ImageFilter>>,
    ) -> Option<ImageFilter> {
        crop(rect, tile_mode, input)
    }

    /// [`displacement_map()`]
    pub fn displacement_map_effect<'a>(
        channel_selectors: (ColorChannel, ColorChannel),
        scale: scalar,
        displacement: impl Into<Option<ImageFilter>>,
        color: impl Into<Option<ImageFilter>>,
        crop_rect: impl Into<Option<&'a IRect>>,
    ) -> Option<Self> {
        displacement_map(
            channel_selectors,
            scale,
            displacement,
            color,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`distant_lit_diffuse()`]
    pub fn distant_lit_diffuse_lighting<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        direction: impl Into<Point3>,
        light_color: impl Into<Color>,
        surface_scale: scalar,
        kd: scalar,
    ) -> Option<Self> {
        distant_lit_diffuse(
            direction,
            light_color,
            surface_scale,
            kd,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`point_lit_diffuse()`]
    pub fn point_lit_diffuse_lighting<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        location: impl Into<Point3>,
        light_color: impl Into<Color>,
        surface_scale: scalar,
        kd: scalar,
    ) -> Option<Self> {
        point_lit_diffuse(
            location,
            light_color,
            surface_scale,
            kd,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`spot_lit_diffuse()`]
    #[allow(clippy::too_many_arguments)]
    pub fn spot_lit_diffuse_lighting<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        location: impl Into<Point3>,
        target: impl Into<Point3>,
        specular_exponent: scalar,
        cutoff_angle: scalar,
        light_color: impl Into<Color>,
        surface_scale: scalar,
        kd: scalar,
    ) -> Option<Self> {
        spot_lit_diffuse(
            location,
            target,
            specular_exponent,
            cutoff_angle,
            light_color,
            surface_scale,
            kd,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`distant_lit_specular()`]
    pub fn distant_lit_specular_lighting<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        direction: impl Into<Point3>,
        light_color: impl Into<Color>,
        surface_scale: scalar,
        ks: scalar,
        shininess: scalar,
    ) -> Option<Self> {
        distant_lit_specular(
            direction,
            light_color,
            surface_scale,
            ks,
            shininess,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`point_lit_specular()`]
    pub fn point_lit_specular_lighting<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        location: impl Into<Point3>,
        light_color: impl Into<Color>,
        surface_scale: scalar,
        ks: scalar,
        shininess: scalar,
    ) -> Option<Self> {
        point_lit_specular(
            location,
            light_color,
            surface_scale,
            ks,
            shininess,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`spot_lit_specular()`]
    #[allow(clippy::too_many_arguments)]
    pub fn spot_lit_specular_lighting<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        location: impl Into<Point3>,
        target: impl Into<Point3>,
        specular_exponent: scalar,
        cutoff_angle: scalar,
        light_color: impl Into<Color>,
        surface_scale: scalar,
        ks: scalar,
        shininess: scalar,
    ) -> Option<Self> {
        spot_lit_specular(
            location,
            target,
            specular_exponent,
            cutoff_angle,
            light_color,
            surface_scale,
            ks,
            shininess,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`magnifier()`]
    pub fn magnifier<'a>(
        self,
        lens_bounds: impl AsRef<Rect>,
        zoom_amount: scalar,
        inset: scalar,
        sampling_options: SamplingOptions,
        crop_rect: impl Into<Option<&'a IRect>>,
    ) -> Option<Self> {
        magnifier(
            lens_bounds,
            zoom_amount,
            inset,
            sampling_options,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`matrix_convolution()`]
    #[allow(clippy::too_many_arguments)]
    pub fn matrix_convolution<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        kernel_size: impl Into<ISize>,
        kernel: &[scalar],
        gain: scalar,
        bias: scalar,
        kernel_offset: impl Into<IPoint>,
        tile_mode: crate::TileMode,
        convolve_alpha: bool,
    ) -> Option<Self> {
        matrix_convolution(
            kernel_size,
            kernel,
            gain,
            bias,
            kernel_offset,
            tile_mode,
            convolve_alpha,
            self,
            crop_rect.into().map(|r| r.into()),
        )
    }

    /// [`merge()`]
    pub fn merge<'a>(
        filters: impl IntoIterator<Item = Option<Self>>,
        crop_rect: impl Into<Option<&'a IRect>>,
    ) -> Option<Self> {
        merge(filters, crop_rect.into().map(|r| r.into()))
    }

    /// [`dilate()`]
    pub fn dilate<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        radii: (scalar, scalar),
    ) -> Option<Self> {
        dilate(radii, self, crop_rect.into().map(|r| r.into()))
    }

    /// [`erode()`]
    pub fn erode<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        radii: (scalar, scalar),
    ) -> Option<Self> {
        erode(radii, self, crop_rect.into().map(|r| r.into()))
    }

    /// [`offset()`]
    pub fn offset<'a>(
        self,
        crop_rect: impl Into<Option<&'a IRect>>,
        delta: impl Into<Vector>,
    ) -> Option<Self> {
        offset(delta, self, crop_rect.into().map(|r| r.into()))
    }

    /// [`self::picture()`]
    pub fn from_picture<'a>(
        picture: impl Into<Picture>,
        crop_rect: impl Into<Option<&'a Rect>>,
    ) -> Option<Self> {
        self::picture(picture, crop_rect)
    }

    /// [`tile()`]
    pub fn tile(self, src: impl AsRef<Rect>, dst: impl AsRef<Rect>) -> Option<Self> {
        tile(src, dst, self)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ArithmeticFPInputs {
    pub k: [f32; 4],
    pub enforce_pm_color: bool,
}

impl From<([f32; 4], bool)> for ArithmeticFPInputs {
    fn from((k, enforce_pm_color): ([f32; 4], bool)) -> Self {
        ArithmeticFPInputs {
            k,
            enforce_pm_color,
        }
    }
}

impl ArithmeticFPInputs {
    pub fn new(k0: f32, k1: f32, k2: f32, k3: f32, enforce_pm_color: bool) -> Self {
        Self {
            k: [k0, k1, k2, k3],
            enforce_pm_color,
        }
    }
}

impl Picture {
    pub fn as_image_filter<'a>(
        &self,
        crop_rect: impl Into<Option<&'a Rect>>,
    ) -> Option<ImageFilter> {
        self.clone().into_image_filter(crop_rect)
    }

    pub fn into_image_filter<'a>(
        self,
        crop_rect: impl Into<Option<&'a Rect>>,
    ) -> Option<ImageFilter> {
        picture(self, crop_rect)
    }
}

#[cfg(test)]
mod tests {
    use super::CropRect;
    use crate::{IRect, Rect};

    fn cr(crop_rect: impl Into<CropRect>) -> CropRect {
        crop_rect.into()
    }

    #[test]
    fn test_crop_conversion_options() {
        assert_eq!(cr(None), CropRect::NO_CROP_RECT);
        assert_eq!(cr(CropRect::NO_CROP_RECT), CropRect::NO_CROP_RECT);
        #[allow(clippy::needless_borrow)]
        let cr_ref = cr(CropRect::NO_CROP_RECT);
        assert_eq!(cr_ref, CropRect::NO_CROP_RECT);
        let irect = IRect {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        };
        assert_eq!(cr(irect), CropRect(Some(Rect::from(irect))));
        #[allow(clippy::needless_borrow)]
        let cr_by_ref = cr(irect);
        assert_eq!(cr_by_ref, CropRect(Some(Rect::from(irect))));
        let rect = Rect {
            left: 1.0,
            top: 2.0,
            right: 3.0,
            bottom: 4.0,
        };
        assert_eq!(cr(rect), CropRect(Some(rect)));
        #[allow(clippy::needless_borrow)]
        let cr_by_ref = cr(rect);
        assert_eq!(cr_by_ref, CropRect(Some(rect)));
    }

    #[test]
    fn test_drop_shadow_only() {
        let rect = Rect {
            left: 1.0,
            top: 2.0,
            right: 3.0,
            bottom: 4.0,
        };

        let _ = super::drop_shadow_only((10., 10.), (1.0, 1.0), 0x404040, None, None, rect);
    }
}
