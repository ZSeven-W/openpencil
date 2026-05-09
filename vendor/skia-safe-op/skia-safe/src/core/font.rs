use std::{fmt, ptr};

use skia_bindings::{self as sb, SkFont, SkFont_PrivFlags};

use crate::{
    interop::VecSink, prelude::*, scalar, EncodedText, FontHinting, FontMetrics, GlyphId, Paint,
    Path, Point, Rect, Typeface, Unichar,
};

pub type Edging = skia_bindings::SkFont_Edging;
variant_name!(Edging::Alias);

pub type Font = Handle<SkFont>;
unsafe_send_sync!(Font);

impl NativeDrop for SkFont {
    fn drop(&mut self) {
        unsafe { sb::C_SkFont_destruct(self) }
    }
}

impl NativeClone for SkFont {
    fn clone(&self) -> Self {
        construct(|f| unsafe { sb::C_SkFont_CopyConstruct(f, self) })
    }
}

impl NativePartialEq for SkFont {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_SkFont_Equals(self, rhs) }
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::from_native_c(unsafe { SkFont::new() })
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("is_force_auto_hinting", &self.is_force_auto_hinting())
            .field("is_embedded_bitmaps", &self.is_embedded_bitmaps())
            .field("is_subpixel", &self.is_subpixel())
            .field("is_linear_metrics", &self.is_linear_metrics())
            .field("is_embolden", &self.is_embolden())
            .field("is_baseline_snap", &self.is_baseline_snap())
            .field("edging", &self.edging())
            .field("hinting", &self.hinting())
            .field("typeface", &self.typeface())
            .field("size", &self.size())
            .field("scale_x", &self.scale_x())
            .field("skew_x", &self.skew_x())
            .field("metrics", &self.metrics())
            .field("spacing", &self.spacing())
            .finish()
    }
}

impl Font {
    pub fn new(typeface: impl Into<Typeface>, size: impl Into<Option<scalar>>) -> Self {
        Self::from_typeface(typeface, size)
    }

    pub fn from_typeface(typeface: impl Into<Typeface>, size: impl Into<Option<scalar>>) -> Self {
        match size.into() {
            None => Self::construct(|font| unsafe {
                sb::C_SkFont_ConstructFromTypeface(font, typeface.into().into_ptr())
            }),
            Some(size) => Self::construct(|font| unsafe {
                sb::C_SkFont_ConstructFromTypefaceWithSize(font, typeface.into().into_ptr(), size)
            }),
        }
    }

    pub fn from_typeface_with_params(
        typeface: impl Into<Typeface>,
        size: scalar,
        scale: scalar,
        skew: scalar,
    ) -> Self {
        Self::construct(|font| unsafe {
            sb::C_SkFont_ConstructFromTypefaceWithSizeScaleAndSkew(
                font,
                typeface.into().into_ptr(),
                size,
                scale,
                skew,
            )
        })
    }

    pub fn is_force_auto_hinting(&self) -> bool {
        self.has_flag(sb::SkFont_PrivFlags_kForceAutoHinting_PrivFlag)
    }

    pub fn is_embedded_bitmaps(&self) -> bool {
        self.has_flag(sb::SkFont_PrivFlags_kEmbeddedBitmaps_PrivFlag)
    }

    pub fn is_subpixel(&self) -> bool {
        self.has_flag(sb::SkFont_PrivFlags_kSubpixel_PrivFlag)
    }

    pub fn is_linear_metrics(&self) -> bool {
        self.has_flag(sb::SkFont_PrivFlags_kLinearMetrics_PrivFlag)
    }

    pub fn is_embolden(&self) -> bool {
        self.has_flag(sb::SkFont_PrivFlags_kEmbolden_PrivFlag)
    }

    pub fn is_baseline_snap(&self) -> bool {
        self.has_flag(sb::SkFont_PrivFlags_kBaselineSnap_PrivFlag)
    }

    fn has_flag(&self, flag: SkFont_PrivFlags) -> bool {
        (SkFont_PrivFlags::from(self.native().fFlags) & flag) != 0
    }

    pub fn set_force_auto_hinting(&mut self, force_auto_hinting: bool) -> &mut Self {
        unsafe { self.native_mut().setForceAutoHinting(force_auto_hinting) }
        self
    }

    pub fn set_embedded_bitmaps(&mut self, embedded_bitmaps: bool) -> &mut Self {
        unsafe { self.native_mut().setEmbeddedBitmaps(embedded_bitmaps) }
        self
    }

    pub fn set_subpixel(&mut self, subpixel: bool) -> &mut Self {
        unsafe { self.native_mut().setSubpixel(subpixel) }
        self
    }

    pub fn set_linear_metrics(&mut self, linear_metrics: bool) -> &mut Self {
        unsafe { self.native_mut().setLinearMetrics(linear_metrics) }
        self
    }

    pub fn set_embolden(&mut self, embolden: bool) -> &mut Self {
        unsafe { self.native_mut().setEmbolden(embolden) }
        self
    }

    pub fn set_baseline_snap(&mut self, baseline_snap: bool) -> &mut Self {
        unsafe { self.native_mut().setBaselineSnap(baseline_snap) }
        self
    }

    pub fn edging(&self) -> Edging {
        unsafe { sb::C_SkFont_getEdging(self.native()) }
    }

    pub fn set_edging(&mut self, edging: Edging) -> &mut Self {
        unsafe { self.native_mut().setEdging(edging) }
        self
    }

    pub fn set_hinting(&mut self, hinting: FontHinting) -> &mut Self {
        unsafe { self.native_mut().setHinting(hinting) }
        self
    }

    pub fn hinting(&self) -> FontHinting {
        unsafe { sb::C_SkFont_getHinting(self.native()) }
    }

    #[must_use]
    pub fn with_size(&self, size: scalar) -> Option<Self> {
        if size >= 0.0 && !size.is_infinite() && !size.is_nan() {
            let mut font = unsafe { SkFont::new() };
            unsafe { sb::C_SkFont_makeWithSize(self.native(), size, &mut font) }
            Some(Self::from_native_c(font))
        } else {
            None
        }
    }

    pub fn typeface(&self) -> Typeface {
        Typeface::from_unshared_ptr(self.native().fTypeface.fPtr)
            .expect("typeface is expected to be non-null")
    }

    pub fn size(&self) -> scalar {
        self.native().fSize
    }

    pub fn scale_x(&self) -> scalar {
        self.native().fScaleX
    }

    pub fn skew_x(&self) -> scalar {
        self.native().fSkewX
    }

    pub fn set_typeface(&mut self, tf: impl Into<Typeface>) -> &mut Self {
        unsafe { sb::C_SkFont_setTypeface(self.native_mut(), tf.into().into_ptr()) }
        self
    }

    pub fn set_size(&mut self, size: scalar) -> &mut Self {
        unsafe { self.native_mut().setSize(size) }
        self
    }

    pub fn set_scale_x(&mut self, scale_x: scalar) -> &mut Self {
        unsafe { self.native_mut().setScaleX(scale_x) }
        self
    }

    pub fn set_skew_x(&mut self, skew_x: scalar) -> &mut Self {
        unsafe { self.native_mut().setSkewX(skew_x) }
        self
    }

    pub fn str_to_glyphs(&self, str: impl AsRef<str>, glyphs: &mut [GlyphId]) -> usize {
        self.text_to_glyphs(str.as_ref(), glyphs)
    }

    pub fn text_to_glyphs(&self, text: impl EncodedText, glyphs: &mut [GlyphId]) -> usize {
        let (ptr, size, encoding) = text.as_raw();
        unsafe {
            sb::C_SkFont_textToGlyphs(
                self.native(),
                ptr,
                size,
                encoding.into_native(),
                glyphs.as_mut_ptr(),
                glyphs.len(),
            )
        }
    }

    pub fn count_str(&self, str: impl AsRef<str>) -> usize {
        self.count_text(str.as_ref())
    }

    pub fn count_text(&self, text: impl EncodedText) -> usize {
        let (ptr, size, encoding) = text.as_raw();
        unsafe {
            sb::C_SkFont_textToGlyphs(
                self.native(),
                ptr,
                size,
                encoding.into_native(),
                ptr::null_mut(),
                0usize,
            )
        }
    }

    // convenience function
    pub fn str_to_glyphs_vec(&self, str: impl AsRef<str>) -> Vec<GlyphId> {
        self.text_to_glyphs_vec(str.as_ref())
    }

    // convenience function
    pub fn text_to_glyphs_vec(&self, text: impl EncodedText) -> Vec<GlyphId> {
        let count = self.count_text(&text);
        let mut glyphs: Vec<GlyphId> = vec![Default::default(); count];
        let resulting_count = self.text_to_glyphs(text, glyphs.as_mut_slice());
        assert_eq!(count, resulting_count);
        glyphs
    }

    pub fn measure_str(&self, str: impl AsRef<str>, paint: Option<&Paint>) -> (scalar, Rect) {
        self.measure_text(str.as_ref(), paint)
    }

    pub fn measure_text(&self, text: impl EncodedText, paint: Option<&Paint>) -> (scalar, Rect) {
        let mut bounds = Rect::default();
        let (ptr, size, encoding) = text.as_raw();
        let width = unsafe {
            self.native().measureText(
                ptr,
                size,
                encoding.into_native(),
                bounds.native_mut(),
                paint.native_ptr_or_null(),
            )
        };

        (width, bounds)
    }

    pub fn unichar_to_glyph(&self, uni: Unichar) -> GlyphId {
        unsafe { self.native().unicharToGlyph(uni) }
    }

    pub fn unichar_to_glyphs(&self, uni: &[Unichar], glyphs: &mut [GlyphId]) {
        assert_eq!(uni.len(), glyphs.len());
        unsafe {
            sb::C_SkFont_unicharsToGlyphs(
                self.native(),
                uni.as_ptr(),
                uni.len(),
                glyphs.as_mut_ptr(),
            )
        }
    }

    pub fn get_widths(&self, glyphs: &[GlyphId], widths: &mut [scalar]) {
        self.get_widths_bounds(glyphs, Some(widths), None, None)
    }

    pub fn get_widths_bounds(
        &self,
        glyphs: &[GlyphId],
        mut widths: Option<&mut [scalar]>,
        mut bounds: Option<&mut [Rect]>,
        paint: Option<&Paint>,
    ) {
        let count = glyphs.len();

        {
            if let Some(slice) = &widths {
                assert_eq!(count, slice.len())
            };
            if let Some(slice) = &bounds {
                assert_eq!(count, slice.len())
            };
        }

        let bounds_ptr = bounds.native_mut().as_ptr_or_null_mut();
        let widths_ptr = widths.as_ptr_or_null_mut();
        let paint_ptr = paint.native_ptr_or_null();

        unsafe {
            sb::C_SkFont_getWidthBounds(
                self.native(),
                glyphs.as_ptr(),
                count,
                widths_ptr,
                bounds_ptr,
                paint_ptr,
            )
        }
    }

    pub fn get_bounds(&self, glyphs: &[GlyphId], bounds: &mut [Rect], paint: Option<&Paint>) {
        self.get_widths_bounds(glyphs, None, Some(bounds), paint)
    }

    pub fn get_pos(&self, glyphs: &[GlyphId], pos: &mut [Point], origin: Option<Point>) {
        let count = glyphs.len();
        assert_eq!(count, pos.len());

        let origin = origin.unwrap_or_default();

        unsafe {
            sb::C_SkFont_getPos(
                self.native(),
                glyphs.as_ptr(),
                count,
                pos.native_mut().as_mut_ptr(),
                *origin.native(),
            )
        }
    }

    pub fn get_x_pos(&self, glyphs: &[GlyphId], x_pos: &mut [scalar], origin: Option<scalar>) {
        let count = glyphs.len();
        assert_eq!(count, x_pos.len());
        let origin = origin.unwrap_or_default();

        unsafe {
            sb::C_SkFont_getXPos(
                self.native(),
                glyphs.as_ptr(),
                count,
                x_pos.as_mut_ptr(),
                origin,
            )
        }
    }

    pub fn get_intercepts<'a>(
        &self,
        glyphs: &[GlyphId],
        pos: &[Point],
        (top, bottom): (scalar, scalar),
        paint: impl Into<Option<&'a Paint>>,
    ) -> Vec<scalar> {
        assert_eq!(glyphs.len(), pos.len());
        let count = glyphs.len();
        let mut r: Vec<scalar> = Vec::new();
        let mut set = |scalars: &[scalar]| r = scalars.to_vec();
        unsafe {
            sb::C_SkFont_getIntercepts(
                self.native(),
                glyphs.as_ptr(),
                count,
                pos.native().as_ptr(),
                top,
                bottom,
                paint.into().native_ptr_or_null(),
                VecSink::new(&mut set).native_mut(),
            );
        }
        r
    }

    pub fn get_path(&self, glyph_id: GlyphId) -> Option<Path> {
        let mut path = Path::default();
        unsafe { sb::C_SkFont_getPath(self.native(), glyph_id, path.native_mut()) }.then_some(path)
    }

    // TODO: getPaths() (needs a function to be passed, but supports a context).

    pub fn metrics(&self) -> (scalar, FontMetrics) {
        let mut line_spacing = 0.0;
        let fm =
            FontMetrics::construct(|fm| line_spacing = unsafe { self.native().getMetrics(fm) });
        (line_spacing, fm)
    }

    pub fn spacing(&self) -> scalar {
        unsafe { self.native().getMetrics(ptr::null_mut()) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{FontMgr, FontStyle};

    use super::*;

    #[test]
    fn test_flags() {
        let font_mgr = FontMgr::new();
        let typeface = font_mgr
            .legacy_make_typeface(None, FontStyle::normal())
            .unwrap();
        let mut font = Font::new(typeface, 10.0);

        font.set_force_auto_hinting(true);
        assert!(font.is_force_auto_hinting());
        font.set_force_auto_hinting(false);
        assert!(!font.is_force_auto_hinting());

        font.set_embolden(true);
        assert!(font.is_embolden());
        font.set_embolden(false);
        assert!(!font.is_embolden());
    }
}
