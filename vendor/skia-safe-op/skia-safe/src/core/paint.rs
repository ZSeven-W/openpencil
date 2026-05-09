use crate::Blender;
use crate::{
    prelude::*, scalar, BlendMode, Color, Color4f, ColorFilter, ColorSpace, ImageFilter,
    MaskFilter, PathEffect, Shader,
};
use core::fmt;

use skia_bindings::{self as sb, SkPaint};

pub use sb::SkPaint_Style as Style;
variant_name!(Style::Fill);

pub use sb::SkPaint_Cap as Cap;
variant_name!(Cap::Butt);

pub use sb::SkPaint_Join as Join;
variant_name!(Join::Miter);

pub type Paint = Handle<SkPaint>;
unsafe_send_sync!(Paint);

impl NativeDrop for SkPaint {
    fn drop(&mut self) {
        unsafe { sb::C_SkPaint_destruct(self) }
    }
}

impl NativeClone for SkPaint {
    fn clone(&self) -> Self {
        unsafe { SkPaint::new2(self) }
    }
}

impl NativePartialEq for SkPaint {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkPaint_Equals(self, rhs) }
    }
}

impl Default for Handle<SkPaint> {
    fn default() -> Self {
        Paint::from_native_c(unsafe { SkPaint::new() })
    }
}

impl fmt::Debug for Paint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paint")
            .field("is_anti_alias", &self.is_anti_alias())
            .field("is_dither", &self.is_dither())
            .field("style", &self.style())
            .field("color", &self.color4f())
            .field("stroke_width", &self.stroke_width())
            .field("stroke_miter", &self.stroke_miter())
            .field("stroke_cap", &self.stroke_cap())
            .field("stroke_join", &self.stroke_join())
            .field("color_filter", &self.color_filter())
            .field("blend_mode", &self.as_blend_mode())
            .field("path_effect", &self.path_effect())
            .field("mask_filter", &self.mask_filter())
            .field("image_filter", &self.image_filter())
            .finish()
    }
}

impl Paint {
    pub fn new<'a>(
        color: impl AsRef<Color4f>,
        color_space: impl Into<Option<&'a ColorSpace>>,
    ) -> Paint {
        let color_space = color_space.into();
        Paint::from_native_c(unsafe {
            SkPaint::new1(
                color.as_ref().native(),
                color_space.native_ptr_or_null_mut_force(),
            )
        })
    }

    pub fn reset(&mut self) -> &mut Self {
        unsafe { self.native_mut().reset() }
        self
    }

    pub fn is_anti_alias(&self) -> bool {
        unsafe { self.native().__bindgen_anon_1.fBitfields.fAntiAlias() != 0 }
    }

    pub fn set_anti_alias(&mut self, anti_alias: bool) -> &mut Self {
        unsafe {
            self.native_mut()
                .__bindgen_anon_1
                .fBitfields
                .set_fAntiAlias(anti_alias as _);
        }
        self
    }

    pub fn is_dither(&self) -> bool {
        unsafe { self.native().__bindgen_anon_1.fBitfields.fDither() != 0 }
    }

    pub fn set_dither(&mut self, dither: bool) -> &mut Self {
        unsafe {
            self.native_mut()
                .__bindgen_anon_1
                .fBitfields
                .set_fDither(dither as _);
        }
        self
    }

    pub fn style(&self) -> Style {
        unsafe { sb::C_SkPaint_getStyle(self.native()) }
    }

    pub fn set_style(&mut self, style: Style) -> &mut Self {
        unsafe { self.native_mut().setStyle(style) }
        self
    }

    pub fn set_stroke(&mut self, stroke: bool) -> &mut Self {
        unsafe { self.native_mut().setStroke(stroke) }
        self
    }

    pub fn color(&self) -> Color {
        self.color4f().to_color()
    }

    pub fn color4f(&self) -> Color4f {
        Color4f::from_native_c(self.native().fColor4f)
    }

    pub fn set_color(&mut self, color: impl Into<Color>) -> &mut Self {
        let color = color.into();
        unsafe { self.native_mut().setColor(color.into_native()) }
        self
    }

    pub fn set_color4f<'a>(
        &mut self,
        color: impl AsRef<Color4f>,
        color_space: impl Into<Option<&'a ColorSpace>>,
    ) -> &mut Self {
        let color_space: Option<&'a ColorSpace> = color_space.into();
        unsafe {
            self.native_mut().setColor1(
                color.as_ref().native(),
                color_space.native_ptr_or_null_mut_force(),
            )
        }
        self
    }

    pub fn alpha_f(&self) -> f32 {
        self.color4f().a
    }

    pub fn alpha(&self) -> u8 {
        unsafe { sb::C_SkPaint_getAlpha(self.native()) }
    }

    pub fn set_alpha_f(&mut self, alpha: f32) -> &mut Self {
        unsafe { self.native_mut().setAlphaf(alpha) }
        self
    }

    pub fn set_alpha(&mut self, alpha: u8) -> &mut Self {
        self.set_alpha_f(f32::from(alpha) * (1.0 / 255.0))
    }

    pub fn set_argb(&mut self, a: u8, r: u8, g: u8, b: u8) -> &mut Self {
        unsafe {
            self.native_mut()
                .setARGB(a.into(), r.into(), g.into(), b.into())
        }
        self
    }

    pub fn stroke_width(&self) -> scalar {
        self.native().fWidth
    }

    pub fn set_stroke_width(&mut self, width: scalar) -> &mut Self {
        unsafe { self.native_mut().setStrokeWidth(width) }
        self
    }

    pub fn stroke_miter(&self) -> scalar {
        self.native().fMiterLimit
    }

    pub fn set_stroke_miter(&mut self, miter_limit: scalar) -> &mut Self {
        unsafe { self.native_mut().setStrokeMiter(miter_limit) }
        self
    }

    pub fn stroke_cap(&self) -> Cap {
        unsafe { sb::C_SkPaint_getStrokeCap(self.native()) }
    }

    pub fn set_stroke_cap(&mut self, cap: Cap) -> &mut Self {
        unsafe { self.native_mut().setStrokeCap(cap) }
        self
    }

    pub fn stroke_join(&self) -> Join {
        unsafe { sb::C_SkPaint_getStrokeJoin(self.native()) }
    }

    pub fn set_stroke_join(&mut self, join: Join) -> &mut Self {
        unsafe { self.native_mut().setStrokeJoin(join) }
        self
    }

    pub fn shader(&self) -> Option<Shader> {
        Shader::from_unshared_ptr(self.native().fShader.fPtr)
    }

    pub fn set_shader(&mut self, shader: impl Into<Option<Shader>>) -> &mut Self {
        unsafe { sb::C_SkPaint_setShader(self.native_mut(), shader.into().into_ptr_or_null()) }
        self
    }

    pub fn color_filter(&self) -> Option<ColorFilter> {
        ColorFilter::from_unshared_ptr(self.native().fColorFilter.fPtr)
    }

    pub fn set_color_filter(&mut self, color_filter: impl Into<Option<ColorFilter>>) -> &mut Self {
        unsafe {
            sb::C_SkPaint_setColorFilter(self.native_mut(), color_filter.into().into_ptr_or_null())
        }
        self
    }

    pub fn as_blend_mode(&self) -> Option<BlendMode> {
        let mut bm = BlendMode::default();
        unsafe { sb::C_SkPaint_asBlendMode(self.native(), &mut bm) }.then_some(bm)
    }

    pub fn blend_mode_or(&self, default_mode: BlendMode) -> BlendMode {
        unsafe { self.native().getBlendMode_or(default_mode) }
    }

    #[deprecated(
        since = "0.42.0",
        note = "Use as_blend_mode() or blend_mode_or() instead."
    )]
    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode_or(BlendMode::SrcOver)
    }

    pub fn is_src_over(&self) -> bool {
        unsafe { self.native().isSrcOver() }
    }

    pub fn set_blend_mode(&mut self, mode: BlendMode) -> &mut Self {
        unsafe { self.native_mut().setBlendMode(mode) }
        self
    }

    pub fn blender(&self) -> Option<Blender> {
        Blender::from_unshared_ptr(self.native().fBlender.fPtr)
    }

    pub fn set_blender(&mut self, blender: impl Into<Option<Blender>>) -> &mut Self {
        unsafe { sb::C_SkPaint_setBlender(self.native_mut(), blender.into().into_ptr_or_null()) }
        self
    }

    pub fn path_effect(&self) -> Option<PathEffect> {
        PathEffect::from_unshared_ptr(self.native().fPathEffect.fPtr)
    }

    pub fn set_path_effect(&mut self, path_effect: impl Into<Option<PathEffect>>) -> &mut Self {
        unsafe {
            sb::C_SkPaint_setPathEffect(self.native_mut(), path_effect.into().into_ptr_or_null())
        }
        self
    }

    pub fn mask_filter(&self) -> Option<MaskFilter> {
        MaskFilter::from_unshared_ptr(self.native().fMaskFilter.fPtr)
    }

    pub fn set_mask_filter(&mut self, mask_filter: impl Into<Option<MaskFilter>>) -> &mut Self {
        unsafe {
            sb::C_SkPaint_setMaskFilter(self.native_mut(), mask_filter.into().into_ptr_or_null())
        }
        self
    }

    pub fn image_filter(&self) -> Option<ImageFilter> {
        ImageFilter::from_unshared_ptr(self.native().fImageFilter.fPtr)
    }

    pub fn set_image_filter(&mut self, image_filter: impl Into<Option<ImageFilter>>) -> &mut Self {
        unsafe {
            sb::C_SkPaint_setImageFilter(self.native_mut(), image_filter.into().into_ptr_or_null())
        }
        self
    }

    pub fn nothing_to_draw(&self) -> bool {
        unsafe { self.native().nothingToDraw() }
    }
}

#[test]
fn default_creation() {
    let paint = Paint::default();
    drop(paint)
}

#[test]
fn method_chaining_compiles() {
    let mut paint = Paint::default();
    let _paint = paint.reset().reset();
}

#[test]
fn union_flags() {
    let mut paint = Paint::default();
    assert!(!paint.is_anti_alias());
    assert!(!paint.is_dither());
    assert_eq!(paint.style(), Style::Fill);

    {
        paint.set_anti_alias(true);

        assert!(paint.is_anti_alias());
        assert!(!paint.is_dither());
        assert_eq!(paint.style(), Style::Fill);

        paint.set_anti_alias(false);
    }

    {
        paint.set_style(Style::StrokeAndFill);

        assert!(!paint.is_anti_alias());
        assert!(!paint.is_dither());
        assert_eq!(paint.style(), Style::StrokeAndFill);

        paint.set_style(Style::Fill);
    }
}

#[test]
fn set_color4f_color_space() {
    let mut paint = Paint::default();
    let color = Color4f::from(Color::DARK_GRAY);
    let color_space = ColorSpace::new_srgb();
    paint.set_color4f(color, None);
    paint.set_color4f(color, &color_space);
    let color2 = Color4f::from(Color::DARK_GRAY);
    paint.set_color4f(color2, Some(&color_space));
}
