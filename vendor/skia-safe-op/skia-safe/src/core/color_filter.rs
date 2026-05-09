use crate::{prelude::*, scalar, BlendMode, Color, Color4f, ColorSpace, NativeFlattenable};
use skia_bindings::{self as sb, SkColorFilter, SkFlattenable, SkRefCntBase};
use std::fmt;

pub type ColorFilter = RCHandle<SkColorFilter>;
unsafe_send_sync!(ColorFilter);
require_type_equality!(sb::SkColorFilter_INHERITED, SkFlattenable);

impl NativeBase<SkRefCntBase> for SkColorFilter {}

impl NativeRefCountedBase for SkColorFilter {
    type Base = SkRefCntBase;
}

impl NativeBase<SkFlattenable> for SkColorFilter {}

impl NativeFlattenable for SkColorFilter {
    fn native_flattenable(&self) -> &SkFlattenable {
        self.base()
    }

    fn native_deserialize(data: &[u8]) -> *mut Self {
        unsafe { sb::C_SkColorFilter_Deserialize(data.as_ptr() as _, data.len()) }
    }
}

impl fmt::Debug for ColorFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorFilter")
            .field("as_a_color_mode", &self.to_a_color_mode())
            .field("as_a_color_matrix", &self.to_a_color_matrix())
            .field("is_alpha_unchanged", &self.is_alpha_unchanged())
            .finish()
    }
}

/// ColorFilters are optional objects in the drawing pipeline. When present in
/// a paint, they are called with the "src" colors, and return new colors, which
/// are then passed onto the next stage (either ImageFilter or Xfermode).
///
/// All subclasses are required to be reentrant-safe : it must be legal to share
/// the same instance between several threads.
impl ColorFilter {
    /// If the filter can be represented by a source color plus Mode, this
    /// returns the color and mode appropriately.
    /// If not, this returns `None` and ignores the parameters.
    pub fn to_a_color_mode(&self) -> Option<(Color, BlendMode)> {
        let mut color: Color = 0.into();
        let mut mode: BlendMode = Default::default();
        unsafe { self.native().asAColorMode(color.native_mut(), &mut mode) }
            .then_some((color, mode))
    }

    /// If the filter can be represented by a 5x4 matrix, this
    /// returns the matrix appropriately.
    /// If not, this returns `None` and ignores the parameter.
    pub fn to_a_color_matrix(&self) -> Option<[scalar; 20]> {
        let mut matrix: [scalar; 20] = Default::default();
        unsafe { self.native().asAColorMatrix(&mut matrix[0]) }.then_some(matrix)
    }

    /// Returns `true` if the filter is guaranteed to never change the alpha of a color it filters.
    pub fn is_alpha_unchanged(&self) -> bool {
        unsafe { self.native().isAlphaUnchanged() }
    }

    #[deprecated(since = "0.79.0", note = "Use filter_color4f()")]
    pub fn filter_color(&self, color: impl Into<Color>) -> Color {
        self.filter_color4f(color.into(), None, None).to_color()
    }

    /// Converts the src color (in src colorspace), into the dst colorspace,
    /// then applies this filter to it, returning the filtered color in the dst colorspace.
    pub fn filter_color4f(
        &self,
        color: impl Into<Color4f>,
        src_color_space: Option<&ColorSpace>,
        dst_color_space: Option<&ColorSpace>,
    ) -> Color4f {
        Color4f::from_native_c(unsafe {
            sb::C_SkColorFilter_filterColor4f(
                self.native(),
                color.into().native(),
                src_color_space.native_ptr_or_null_mut_force(),
                dst_color_space.native_ptr_or_null_mut_force(),
            )
        })
    }

    /// Construct a color filter whose effect is to first apply the inner filter and then apply
    /// this filter, applied to the output of the inner filter.
    ///
    /// result = this(inner(...))
    pub fn composed(&self, inner: impl Into<ColorFilter>) -> Option<Self> {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilter_makeComposed(self.native(), inner.into().into_ptr())
        })
    }

    /// Return a color filter that will compute this filter in a specific color space. By default
    /// all filters operate in the destination (surface) color space. This allows filters like Blend
    /// and Matrix, or runtime color filters to perform their math in a known space.
    pub fn with_working_color_space(&self, color_space: impl Into<ColorSpace>) -> Option<Self> {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilter_withWorkingColorSpace(self.native(), color_space.into().into_ptr())
        })
    }
}

pub mod color_filters {
    use crate::{prelude::*, Color4f, ColorSpace, ColorTable};
    use crate::{scalar, BlendMode, Color, ColorFilter, ColorMatrix};
    use skia_bindings as sb;

    pub fn compose(
        outer: impl Into<ColorFilter>,
        inner: impl Into<ColorFilter>,
    ) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilters_Compose(outer.into().into_ptr(), inner.into().into_ptr())
        })
    }

    /// Blends between the constant color (src) and input color (dst) based on the [`BlendMode`].
    /// If the color space is `None`, the constant color is assumed to be defined in sRGB.
    pub fn blend_with_color_space(
        c: impl Into<Color4f>,
        color_space: impl Into<Option<ColorSpace>>,
        mode: BlendMode,
    ) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilters_Blend2(
                c.into().native(),
                color_space.into().into_ptr_or_null(),
                mode,
            )
        })
    }

    pub fn blend(c: impl Into<Color>, mode: BlendMode) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_Blend(c.into().into_native(), mode) })
    }

    pub use sb::SkColorFilters_Clamp as Clamp;
    variant_name!(Clamp::No);

    pub fn matrix(color_matrix: &ColorMatrix, clamp: impl Into<Option<Clamp>>) -> ColorFilter {
        let clamp = clamp.into().unwrap_or(Clamp::Yes);
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_Matrix(color_matrix.native(), clamp) })
            .unwrap()
    }

    pub fn matrix_row_major(array: &[scalar; 20], clamp: impl Into<Option<Clamp>>) -> ColorFilter {
        let clamp = clamp.into().unwrap_or(Clamp::Yes);
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_MatrixRowMajor(array.as_ptr(), clamp) })
            .unwrap()
    }

    // A version of Matrix which operates in HSLA space instead of RGBA.
    // I.e. HSLA-to-RGBA(Matrix(RGBA-to-HSLA(input))).
    pub fn hsla_matrix_of_color_matrix(color_matrix: &ColorMatrix) -> ColorFilter {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilters_HSLAMatrixOfColorMatrix(color_matrix.native())
        })
        .unwrap()
    }

    /// See [`hsla_matrix_of_color_matrix()`]
    pub fn hsla_matrix(row_major: &[f32; 20]) -> ColorFilter {
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_HSLAMatrix(row_major.as_ptr()) })
            .unwrap()
    }

    pub fn linear_to_srgb_gamma() -> ColorFilter {
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_LinearToSRGBGamma() }).unwrap()
    }

    pub fn srgb_to_linear_gamma() -> ColorFilter {
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_SRGBToLinearGamma() }).unwrap()
    }

    pub fn lerp(
        t: f32,
        dst: impl Into<ColorFilter>,
        src: impl Into<ColorFilter>,
    ) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilters_Lerp(t, dst.into().into_ptr(), src.into().into_ptr())
        })
    }

    /// Create a table color filter, copying the table into the filter, and
    /// applying it to all 4 components.
    /// `a' = table[a];`
    /// `r' = table[r];`
    /// `g' = table[g];`
    /// `b' = table[b];`
    /// Components are operated on in unpremultiplied space. If the incoming
    /// colors are premultiplied, they are temporarily unpremultiplied, then
    /// the table is applied, and then the result is re-multiplied.
    pub fn table(table: &[u8; 256]) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_Table(table.as_ptr()) })
    }

    /// Create a table color filter, with a different table for each
    /// component [A, R, G, B]. If a given table is `None`, then it is
    /// treated as identity, with the component left unchanged. If a table
    /// is not `None`, then its contents are copied into the filter.
    pub fn table_argb<'a>(
        table_a: impl Into<Option<&'a [u8; 256]>>,
        table_r: impl Into<Option<&'a [u8; 256]>>,
        table_g: impl Into<Option<&'a [u8; 256]>>,
        table_b: impl Into<Option<&'a [u8; 256]>>,
    ) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilters_TableARGB(
                table_a.into().map(|t| t.as_ref()).as_ptr_or_null(),
                table_r.into().map(|t| t.as_ref()).as_ptr_or_null(),
                table_g.into().map(|t| t.as_ref()).as_ptr_or_null(),
                table_b.into().map(|t| t.as_ref()).as_ptr_or_null(),
            )
        })
    }

    /// Create a table color filter that holds a ref to the shared color table.
    pub fn table_from_color_table(table: impl Into<ColorTable>) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe { sb::C_SkColorFilters_Table2(table.into().into_ptr()) })
    }

    /// Create a color filter that multiplies the RGB channels by one color, and
    /// then adds a second color, pinning the result for each component to
    /// [0..255]. The alpha components of the mul and add arguments
    /// are ignored.
    pub fn lighting(mul: impl Into<Color>, add: impl Into<Color>) -> Option<ColorFilter> {
        ColorFilter::from_ptr(unsafe {
            sb::C_SkColorFilters_Lighting(mul.into().into_native(), add.into().into_native())
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::{color_filters, BlendMode, Color, Color4f, ColorSpace};

    #[test]
    fn color_mode_roundtrip() {
        let color = Color::CYAN;
        let mode = BlendMode::ColorBurn;
        let cf = color_filters::blend(color, mode).unwrap();
        let (c, m) = cf.to_a_color_mode().unwrap();
        assert_eq!(color, c);
        assert_eq!(mode, m);
    }

    #[test]
    fn ref_count() {
        let color = Color::CYAN;
        let mode = BlendMode::ColorBurn;
        let cf = color_filters::blend(color, mode).unwrap();
        let rc = cf.native()._ref_cnt();
        assert_eq!(1, rc);
    }

    #[test]
    fn filter_color() {
        let color = Color::CYAN;
        let mode = BlendMode::ColorBurn;
        let cf = color_filters::blend(color, mode).unwrap();
        #[allow(deprecated)]
        let _fc = cf.filter_color(Color::DARK_GRAY);
        let _fc = cf.filter_color4f(
            Color4f::new(0.0, 0.0, 0.0, 0.0),
            Some(&ColorSpace::new_srgb()),
            None,
        );
    }
}
