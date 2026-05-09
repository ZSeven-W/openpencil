use std::fmt;

use skia_bindings::{self as sb, SkFlattenable, SkRefCntBase, SkShader};

use crate::{prelude::*, ColorFilter, ColorSpace, Image, Matrix, NativeFlattenable, TileMode};

pub type Shader = RCHandle<SkShader>;
unsafe_send_sync!(Shader);
require_type_equality!(sb::SkShader_INHERITED, SkFlattenable);

impl NativeBase<SkRefCntBase> for SkShader {}
impl NativeBase<SkFlattenable> for SkShader {}

impl NativeRefCountedBase for SkShader {
    type Base = SkRefCntBase;
    fn ref_counted_base(&self) -> &Self::Base {
        self.base()
    }
}

impl NativeFlattenable for SkShader {
    fn native_flattenable(&self) -> &SkFlattenable {
        self.base()
    }

    fn native_deserialize(data: &[u8]) -> *mut Self {
        unsafe { sb::C_SkShader_Deserialize(data.as_ptr() as _, data.len()) }
    }
}

impl Default for Shader {
    fn default() -> Self {
        shaders::empty()
    }
}

impl fmt::Debug for Shader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shader")
            .field("is_opaque", &self.is_opaque())
            .field("image", &self.image())
            .finish()
    }
}

/// Shaders specify the source color(s) for what is being drawn. If a paint
/// has no shader, then the paint's color is used. If the paint has a
/// shader, then the shader's color(s) are use instead, but they are
/// modulated by the paint's alpha. This makes it easy to create a shader
/// once (e.g. bitmap tiling or gradient) and then change its transparency
/// w/o having to modify the original shader... only the paint's alpha needs
/// to be modified.
impl Shader {
    /// Returns `true` if the shader is guaranteed to produce only opaque
    /// colors, subject to the [`crate::Paint`] using the shader to apply an opaque
    /// alpha value. Subclasses should override this to allow some
    /// optimizations.
    pub fn is_opaque(&self) -> bool {
        unsafe { sb::C_SkShader_isOpaque(self.native()) }
    }
    /// Returns iff this shader is backed by a single [`Image`].
    /// If not, returns `None`.
    pub fn image(&self) -> Option<(Image, Matrix, (TileMode, TileMode))> {
        unsafe {
            let mut matrix = Matrix::default();
            let mut tile_mode = [TileMode::default(); 2];
            let image = Image::from_unshared_ptr(
                self.native()
                    .isAImage(matrix.native_mut(), tile_mode.as_mut_ptr()),
            );
            #[allow(clippy::tuple_array_conversions)]
            image.map(|i| (i, matrix, (tile_mode[0], tile_mode[1])))
        }
    }

    pub fn is_a_image(&self) -> bool {
        unsafe { sb::C_SkShader_isAImage(self.native()) }
    }

    /// Return a shader that will apply the specified `local_matrix` to this shader.
    /// The specified matrix will be applied before any matrix associated with this shader.
    #[must_use]
    pub fn with_local_matrix(&self, matrix: &Matrix) -> Self {
        Self::from_ptr(unsafe {
            sb::C_SkShader_makeWithLocalMatrix(self.native(), matrix.native())
        })
        .unwrap()
    }

    /// Create a new shader that produces the same colors as invoking this shader and then applying
    /// the color filter.
    #[must_use]
    pub fn with_color_filter(&self, color_filter: impl Into<ColorFilter>) -> Self {
        Self::from_ptr(unsafe {
            sb::C_SkShader_makeWithColorFilter(self.native(), color_filter.into().into_ptr())
        })
        .unwrap()
    }

    /// Return a shader that will compute this shader in a context such that any child shaders
    /// return RGBA values converted to the `input_cs` colorspace.
    ///
    /// It is then assumed that the RGBA values returned by this shader have been transformed into
    /// `output_cs` by the shader being wrapped.  By default, shaders are assumed to return values
    /// in the destination colorspace and premultiplied. Using a different `output_cs` than `input_cs`
    /// allows custom shaders to replace the color management Skia normally performs w/o forcing
    /// authors to otherwise manipulate surface/image color info to avoid unnecessary or incorrect
    /// work.
    ///
    /// If the shader is not performing colorspace conversion but needs to operate in the `input_cs`
    /// then it should have `output_cs` be the same as `input_cs`. Regardless of the `output_cs` here,
    /// the RGBA values of the returned [`Shader`] are always converted from `output_cs` to the
    /// destination surface color space.
    ///
    /// A `None` `input_cs` is assumed to be the destination CS.
    /// A `None` `output_cs` is assumed to be the `input_cs`.
    #[must_use]
    pub fn with_working_color_space(
        &self,
        input_cs: impl Into<Option<ColorSpace>>,
        output_cs: impl Into<Option<ColorSpace>>,
    ) -> Self {
        Self::from_ptr(unsafe {
            sb::C_SkShader_makeWithWorkingColorSpace(
                self.native(),
                input_cs.into().into_ptr_or_null(),
                output_cs.into().into_ptr_or_null(),
            )
        })
        .unwrap()
    }
}

pub mod shaders {
    use skia_bindings as sb;

    use crate::{
        prelude::*, Blender, Color, Color4f, ColorSpace, Image, Matrix, Rect, SamplingOptions,
        Shader, TileMode,
    };

    pub fn empty() -> Shader {
        Shader::from_ptr(unsafe { sb::C_SkShaders_Empty() }).unwrap()
    }

    pub fn color(color: impl Into<Color>) -> Shader {
        let color = color.into();
        Shader::from_ptr(unsafe { sb::C_SkShaders_Color(color.into_native()) }).unwrap()
    }

    pub fn color_in_space(color: impl AsRef<Color4f>, space: impl Into<ColorSpace>) -> Shader {
        Shader::from_ptr(unsafe {
            sb::C_SkShaders_Color2(color.as_ref().native(), space.into().into_ptr())
        })
        .unwrap()
    }

    pub fn blend(
        blender: impl Into<Blender>,
        dst: impl Into<Shader>,
        src: impl Into<Shader>,
    ) -> Shader {
        Shader::from_ptr(unsafe {
            sb::C_SkShaders_Blend(
                blender.into().into_ptr(),
                dst.into().into_ptr(),
                src.into().into_ptr(),
            )
        })
        .unwrap()
    }

    pub fn coord_clamp(shader: impl Into<Shader>, rect: impl AsRef<Rect>) -> Option<Shader> {
        Shader::from_ptr(unsafe {
            sb::C_SkShaders_CoordClamp(shader.into().into_ptr(), rect.as_ref().native())
        })
    }

    /// Create an [`Shader`] that will sample the 'image'. This is equivalent to [`Image::to_shader`].
    pub fn image<'a>(
        image: impl Into<Image>,
        tm: (TileMode, TileMode),
        options: &SamplingOptions,
        matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        Shader::from_ptr(unsafe {
            sb::C_SkShaders_Image(
                image.into().into_ptr(),
                tm.0,
                tm.1,
                options.native(),
                matrix.into().native_ptr_or_null(),
            )
        })
    }

    /// Create an [`Shader`] that will sample 'image' with minimal processing. This is equivalent to
    /// [`Image::to_raw_shader`].
    pub fn raw_image<'a>(
        image: impl Into<Image>,
        tm: (TileMode, TileMode),
        options: &SamplingOptions,
        matrix: impl Into<Option<&'a Matrix>>,
    ) -> Option<Shader> {
        Shader::from_ptr(unsafe {
            sb::C_SkShaders_RawImage(
                image.into().into_ptr(),
                tm.0,
                tm.1,
                options.native(),
                matrix.into().native_ptr_or_null(),
            )
        })
    }

    // Re-export gradient shader factory functions from effects::gradient::shaders
    pub use crate::effects::gradient::shaders::{
        linear_gradient, radial_gradient, sweep_gradient, two_point_conical_gradient,
    };
}
