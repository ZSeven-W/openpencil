use crate::{
    prelude::*, typeface::FactoryId, Data, Drawable, FontArguments, FontMetrics, FontStyle,
    GlyphId, Path, Rect, Typeface,
};
use skia_bindings::{self as sb, SkCustomTypefaceBuilder};
use std::fmt;

pub type CustomTypefaceBuilder = Handle<SkCustomTypefaceBuilder>;
unsafe_send_sync!(CustomTypefaceBuilder);

impl NativeDrop for SkCustomTypefaceBuilder {
    fn drop(&mut self) {
        unsafe { sb::C_SkCustomTypefaceBuilder_destruct(self) }
    }
}

impl fmt::Debug for CustomTypefaceBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomTypefaceBuilder").finish()
    }
}

impl CustomTypefaceBuilder {
    pub fn new() -> Self {
        Self::from_native_c(unsafe { SkCustomTypefaceBuilder::new() })
    }

    pub fn set_glyph<'a>(
        &mut self,
        glyph_id: GlyphId,
        advance: f32,
        typeface_glyph: impl Into<TypefaceGlyph<'a>>,
    ) -> &mut Self {
        unsafe {
            use TypefaceGlyph::*;
            match typeface_glyph.into() {
                Path(path) => self.native_mut().setGlyph(glyph_id, advance, path.native()),
                DrawableAndBounds(drawable, bounds) => sb::C_SkCustomTypefaceBuilder_setGlyph(
                    self.native_mut(),
                    glyph_id,
                    advance,
                    drawable.into_ptr(),
                    bounds.native(),
                ),
            }
        }
        self
    }

    pub fn set_metrics(
        &mut self,
        font_metrics: &FontMetrics,
        scale: impl Into<Option<f32>>,
    ) -> &mut Self {
        unsafe {
            self.native_mut()
                .setMetrics(font_metrics.native(), scale.into().unwrap_or(1.0))
        }
        self
    }

    pub fn set_font_style(&mut self, font_style: FontStyle) -> &mut Self {
        unsafe { self.native_mut().setFontStyle(font_style.into_native()) }
        self
    }

    pub fn detach(&mut self) -> Option<Typeface> {
        Typeface::from_ptr(unsafe { sb::C_SkCustomTypefaceBuilder_detach(self.native_mut()) })
    }

    pub const FACTORY_ID: FactoryId = FactoryId::from_chars('u', 's', 'e', 'r');

    // TODO: MakeFromStream
    // TODO: This is a stand-in for `from_stream`.

    pub fn from_data(data: impl Into<Data>, font_arguments: &FontArguments) -> Option<Typeface> {
        Typeface::from_ptr(unsafe {
            sb::C_SkCustomTypefaceBuilder_FromData(data.into().into_ptr(), font_arguments.native())
        })
    }
}

#[derive(Debug)]
pub enum TypefaceGlyph<'a> {
    Path(&'a Path),
    DrawableAndBounds(Drawable, Rect),
}

impl<'a> From<&'a Path> for TypefaceGlyph<'a> {
    fn from(path: &'a Path) -> Self {
        Self::Path(path)
    }
}

impl From<(Drawable, Rect)> for TypefaceGlyph<'_> {
    fn from((drawable, bounds): (Drawable, Rect)) -> Self {
        Self::DrawableAndBounds(drawable, bounds)
    }
}

#[test]
fn build_custom_typeface() {
    let mut builder = CustomTypefaceBuilder::new();
    let path = Path::new();
    builder.set_glyph(10u16, 0.0, &path);
    builder.set_glyph(11u16, 0.0, &path);
    let typeface = builder.detach().unwrap();
    assert_eq!(typeface.native().ref_counted_base()._ref_cnt(), 1);
}
