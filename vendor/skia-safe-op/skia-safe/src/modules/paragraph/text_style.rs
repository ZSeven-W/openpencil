use super::{FontArguments, FontFamilies, TextBaseline, TextShadow};
use crate::{
    font,
    interop::{self, AsStr, FromStrs, SetStr},
    prelude::*,
    scalar,
    textlayout::{RangeExtensions, EMPTY_INDEX, EMPTY_RANGE},
    Color, FontHinting, FontMetrics, FontStyle, Paint, Typeface,
};
use skia_bindings as sb;
use std::{fmt, ops::Range};

bitflags! {
    /// Multiple decorations can be applied at once. Ex: Underline and overline is
    /// (0x1 | 0x2)
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TextDecoration: u32 {
        const NO_DECORATION = sb::skia_textlayout_TextDecoration::kNoDecoration as _;
        const UNDERLINE = sb::skia_textlayout_TextDecoration::kUnderline as _;
        const OVERLINE = sb::skia_textlayout_TextDecoration::kOverline as _;
        const LINE_THROUGH = sb::skia_textlayout_TextDecoration::kLineThrough as _;
    }
}

pub const ALL_TEXT_DECORATIONS: TextDecoration = TextDecoration::ALL;

impl Default for TextDecoration {
    fn default() -> Self {
        TextDecoration::NO_DECORATION
    }
}

impl TextDecoration {
    pub const ALL: TextDecoration = TextDecoration::all();
}

pub use sb::skia_textlayout_TextDecorationStyle as TextDecorationStyle;
#[test]
fn text_decoration_style_naming() {
    let _ = TextDecorationStyle::Solid;
}

pub use sb::skia_textlayout_TextDecorationMode as TextDecorationMode;
#[test]
fn text_decoration_mode_naming() {
    let _ = TextDecorationMode::Gaps;
}

pub use sb::skia_textlayout_StyleType as StyleType;
#[test]
fn style_type_member_naming() {
    let _ = StyleType::Foreground;
    let _ = StyleType::LetterSpacing;
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Decoration {
    pub ty: TextDecoration,
    pub mode: TextDecorationMode,
    pub color: Color,
    pub style: TextDecorationStyle,
    pub thickness_multiplier: scalar,
}

impl Default for Decoration {
    fn default() -> Self {
        Self {
            ty: TextDecoration::default(),
            mode: TextDecorationMode::default(),
            color: Color::TRANSPARENT,
            style: TextDecorationStyle::default(),
            thickness_multiplier: 1.0,
        }
    }
}

native_transmutable!(sb::skia_textlayout_Decoration, Decoration);

/// Where to vertically align the placeholder relative to the surrounding text.
#[repr(i32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Default)]
pub enum PlaceholderAlignment {
    /// Match the baseline of the placeholder with the baseline.
    #[default]
    Baseline,

    /// Align the bottom edge of the placeholder with the baseline such that the
    /// placeholder sits on top of the baseline.
    AboveBaseline,

    /// Align the top edge of the placeholder with the baseline specified in
    /// such that the placeholder hangs below the baseline.
    BelowBaseline,

    /// Align the top edge of the placeholder with the top edge of the font.
    /// When the placeholder is very tall, the extra space will hang from
    /// the top and extend through the bottom of the line.
    Top,

    /// Align the bottom edge of the placeholder with the top edge of the font.
    /// When the placeholder is very tall, the extra space will rise from
    /// the bottom and extend through the top of the line.
    Bottom,

    /// Align the middle of the placeholder with the middle of the text. When the
    /// placeholder is very tall, the extra space will grow equally from
    /// the top and bottom of the line.
    Middle,
}
native_transmutable!(
    sb::skia_textlayout_PlaceholderAlignment,
    PlaceholderAlignment
);

pub type FontFeature = Handle<sb::skia_textlayout_FontFeature>;
unsafe_send_sync!(FontFeature);

impl NativeDrop for sb::skia_textlayout_FontFeature {
    fn drop(&mut self) {
        unsafe { sb::C_FontFeature_destruct(self) }
    }
}

impl NativeClone for sb::skia_textlayout_FontFeature {
    fn clone(&self) -> Self {
        construct(|ts| unsafe { sb::C_FontFeature_CopyConstruct(ts, self) })
    }
}

impl PartialEq for FontFeature {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.value() == other.value()
    }
}

impl fmt::Debug for FontFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FontFeature")
            .field(&self.name())
            .field(&self.value())
            .finish()
    }
}

impl FontFeature {
    pub fn name(&self) -> &str {
        self.native().fName.as_str()
    }

    pub fn value(&self) -> i32 {
        self.native().fValue
    }
}

#[repr(C)]
#[derive(Clone, Default, Debug)]
pub struct PlaceholderStyle {
    pub width: scalar,
    pub height: scalar,
    pub alignment: PlaceholderAlignment,
    pub baseline: TextBaseline,
    /// Distance from the top edge of the rect to the baseline position. This
    /// baseline will be aligned against the alphabetic baseline of the surrounding
    /// text.
    ///
    /// Positive values drop the baseline lower (positions the rect higher) and
    /// small or negative values will cause the rect to be positioned underneath
    /// the line. When baseline == height, the bottom edge of the rect will rest on
    /// the alphabetic baseline.
    pub baseline_offset: scalar,
}

native_transmutable!(sb::skia_textlayout_PlaceholderStyle, PlaceholderStyle);

impl PartialEq for PlaceholderStyle {
    fn eq(&self, other: &Self) -> bool {
        unsafe { self.native().equals(other.native()) }
    }
}

impl PlaceholderStyle {
    pub fn new(
        width: scalar,
        height: scalar,
        alignment: PlaceholderAlignment,
        baseline: TextBaseline,
        offset: scalar,
    ) -> Self {
        Self {
            width,
            height,
            alignment,
            baseline,
            baseline_offset: offset,
        }
    }
}

pub type TextStyle = Handle<sb::skia_textlayout_TextStyle>;
unsafe_send_sync!(TextStyle);

impl NativeDrop for sb::skia_textlayout_TextStyle {
    fn drop(&mut self) {
        unsafe { sb::C_TextStyle_destruct(self) }
    }
}

impl NativeClone for sb::skia_textlayout_TextStyle {
    fn clone(&self) -> Self {
        construct(|ts| unsafe { sb::C_TextStyle_CopyConstruct(ts, self) })
    }
}

impl NativePartialEq for sb::skia_textlayout_TextStyle {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { self.equals(rhs) }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for TextStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextStyle")
            .field("color", &self.color())
            .field("has_foreground", &self.has_foreground())
            .field("foreground", &self.foreground())
            .field("has_background", &self.has_background())
            .field("background", &self.background())
            .field("decoration", &self.decoration())
            .field("font_style", &self.font_style())
            .field("shadows", &self.shadows())
            .field("font_features", &self.font_features())
            .field("font_size", &self.font_size())
            .field("font_families", &self.font_families())
            .field("baseline_shift", &self.baseline_shift())
            .field("height", &self.height())
            .field("height_override", &self.height_override())
            .field("half_leading", &self.half_leading())
            .field("letter_spacing", &self.letter_spacing())
            .field("word_spacing", &self.word_spacing())
            .field("typeface", &self.typeface())
            .field("locale", &self.locale())
            .field("text_baseline", &self.text_baseline())
            .field("is_placeholder", &self.is_placeholder())
            .field("font_edging", &self.font_edging())
            .field("subpixel", &self.subpixel())
            .field("font_hinting", &self.font_hinting())
            .finish()
    }
}

impl TextStyle {
    pub fn new() -> Self {
        TextStyle::construct(|ts| unsafe { sb::C_TextStyle_Construct(ts) })
    }

    #[deprecated(since = "0.51.0", note = "Use clone_for_placeholder")]
    #[must_use]
    pub fn to_placeholder(&self) -> Self {
        self.clone_for_placeholder()
    }

    #[must_use]
    pub fn clone_for_placeholder(&self) -> Self {
        Self::construct(|ts| unsafe { sb::C_TextStyle_cloneForPlaceholder(self.native(), ts) })
    }

    pub fn equals(&self, other: &TextStyle) -> bool {
        *self == *other
    }

    pub fn equals_by_fonts(&self, that: &TextStyle) -> bool {
        unsafe { self.native().equalsByFonts(that.native()) }
    }

    pub fn match_one_attribute(&self, style_type: StyleType, other: &TextStyle) -> bool {
        unsafe { self.native().matchOneAttribute(style_type, other.native()) }
    }

    pub fn color(&self) -> Color {
        Color::from_native_c(self.native().fColor)
    }

    pub fn set_color(&mut self, color: impl Into<Color>) -> &mut Self {
        self.native_mut().fColor = color.into().into_native();
        self
    }

    pub fn has_foreground(&self) -> bool {
        self.native().fHasForeground
    }

    pub fn foreground(&self) -> Paint {
        Paint::construct(|p| unsafe { sb::C_TextStyle_getForeground(self.native(), p) })
    }

    pub fn set_foreground_paint(&mut self, paint: &Paint) -> &mut Self {
        unsafe { sb::C_TextStyle_setForegroundPaint(self.native_mut(), paint.native()) };
        self
    }

    #[deprecated(since = "0.64.0", note = "use set_foreground_paint()")]
    pub fn set_foreground_color(&mut self, paint: &Paint) -> &mut Self {
        self.set_foreground_paint(paint)
    }

    pub fn clear_foreground_color(&mut self) -> &mut Self {
        self.native_mut().fHasForeground = false;
        self
    }

    pub fn has_background(&self) -> bool {
        self.native().fHasBackground
    }

    pub fn background(&self) -> Paint {
        Paint::construct(|p| unsafe { sb::C_TextStyle_getBackground(self.native(), p) })
    }

    pub fn set_background_paint(&mut self, paint: &Paint) -> &mut Self {
        unsafe { sb::C_TextStyle_setBackgroundPaint(self.native_mut(), paint.native()) };
        self
    }

    #[deprecated(since = "0.64.0", note = "use set_background_paint()")]
    pub fn set_background_color(&mut self, paint: &Paint) -> &mut Self {
        self.set_background_paint(paint)
    }

    pub fn clear_background_color(&mut self) -> &mut Self {
        self.native_mut().fHasBackground = false;
        self
    }

    pub fn decoration(&self) -> &Decoration {
        Decoration::from_native_ref(&self.native().fDecoration)
    }

    pub fn decoration_type(&self) -> TextDecoration {
        self.decoration().ty
    }

    pub fn decoration_mode(&self) -> TextDecorationMode {
        self.decoration().mode
    }

    pub fn decoration_color(&self) -> Color {
        self.decoration().color
    }

    pub fn decoration_style(&self) -> TextDecorationStyle {
        self.decoration().style
    }

    pub fn decoration_thickness_multiplier(&self) -> scalar {
        self.decoration().thickness_multiplier
    }

    pub fn set_decoration(&mut self, decoration: &Decoration) {
        *self.decoration_mut_internal() = *decoration;
    }

    pub fn set_decoration_type(&mut self, decoration: TextDecoration) {
        self.decoration_mut_internal().ty = decoration;
    }

    pub fn set_decoration_mode(&mut self, mode: TextDecorationMode) {
        self.decoration_mut_internal().mode = mode;
    }

    pub fn set_decoration_style(&mut self, style: TextDecorationStyle) {
        self.decoration_mut_internal().style = style;
    }

    pub fn set_decoration_color(&mut self, color: impl Into<Color>) {
        self.decoration_mut_internal().color = color.into();
    }

    pub fn set_decoration_thickness_multiplier(&mut self, multiplier: scalar) {
        self.decoration_mut_internal().thickness_multiplier = multiplier;
    }

    #[deprecated(since = "0.63.1", note = "use set_decoration()")]
    pub fn decoration_mut(&mut self) -> &mut Decoration {
        self.decoration_mut_internal()
    }

    fn decoration_mut_internal(&mut self) -> &mut Decoration {
        Decoration::from_native_ref_mut(&mut self.native_mut().fDecoration)
    }

    pub fn font_style(&self) -> FontStyle {
        FontStyle::from_native_c(self.native().fFontStyle)
    }

    pub fn set_font_style(&mut self, font_style: FontStyle) -> &mut Self {
        self.native_mut().fFontStyle = font_style.into_native();
        self
    }

    pub fn shadows(&self) -> &[TextShadow] {
        unsafe {
            let mut count = 0;
            let ptr = sb::C_TextStyle_getShadows(&self.native().fTextShadows, &mut count);
            safer::from_raw_parts(TextShadow::from_native_ptr(ptr), count)
        }
    }

    pub fn add_shadow(&mut self, shadow: TextShadow) -> &mut Self {
        unsafe { sb::C_TextStyle_addShadow(self.native_mut(), shadow.native()) }
        self
    }

    pub fn reset_shadows(&mut self) -> &mut Self {
        unsafe { sb::C_TextStyle_resetShadows(self.native_mut()) }
        self
    }

    pub fn font_features(&self) -> &[FontFeature] {
        unsafe {
            let mut count = 0;
            let ptr = sb::C_TextStyle_getFontFeatures(&self.native().fFontFeatures, &mut count);
            safer::from_raw_parts(FontFeature::from_native_ptr(ptr), count)
        }
    }

    pub fn add_font_feature(&mut self, font_feature: impl AsRef<str>, value: i32) {
        let font_feature = interop::String::from_str(font_feature);
        unsafe { sb::C_TextStyle_addFontFeature(self.native_mut(), font_feature.native(), value) }
    }

    pub fn reset_font_features(&mut self) {
        unsafe { sb::C_TextStyle_resetFontFeatures(self.native_mut()) }
    }

    pub fn font_arguments(&self) -> Option<&FontArguments> {
        unsafe { sb::C_TextStyle_getFontArguments(self.native()) }
            .into_non_null()
            .map(|ptr| FontArguments::from_native_ref(unsafe { ptr.as_ref() }))
    }

    /// The contents of the [`crate::FontArguments`] will be copied into the [`TextStyle`].
    pub fn set_font_arguments<'fa>(
        &mut self,
        arguments: impl Into<Option<&'fa crate::FontArguments<'fa, 'fa>>>,
    ) {
        unsafe {
            sb::C_TextStyle_setFontArguments(
                self.native_mut(),
                arguments.into().native_ptr_or_null(),
            )
        }
    }

    pub fn font_size(&self) -> scalar {
        self.native().fFontSize
    }

    pub fn set_font_size(&mut self, size: scalar) -> &mut Self {
        self.native_mut().fFontSize = size;
        self
    }

    pub fn font_families(&self) -> FontFamilies {
        unsafe {
            let mut count = 0;
            let ptr = sb::C_TextStyle_getFontFamilies(self.native(), &mut count);
            FontFamilies(safer::from_raw_parts(ptr, count))
        }
    }

    pub fn set_font_families(&mut self, families: &[impl AsRef<str>]) -> &mut Self {
        let families: Vec<interop::String> = FromStrs::from_strs(families);
        let families = families.native();
        unsafe {
            sb::C_TextStyle_setFontFamilies(self.native_mut(), families.as_ptr(), families.len())
        }
        self
    }

    pub fn baseline_shift(&self) -> scalar {
        self.native().fBaselineShift
    }

    pub fn set_baseline_shift(&mut self, baseline_shift: scalar) -> &mut Self {
        self.native_mut().fBaselineShift = baseline_shift;
        self
    }

    pub fn set_height(&mut self, height: scalar) -> &mut Self {
        self.native_mut().fHeight = height;
        self
    }

    pub fn height(&self) -> scalar {
        let n = self.native();
        if n.fHeightOverride {
            n.fHeight
        } else {
            0.0
        }
    }

    pub fn set_height_override(&mut self, height_override: bool) -> &mut Self {
        self.native_mut().fHeightOverride = height_override;
        self
    }

    pub fn height_override(&self) -> bool {
        self.native().fHeightOverride
    }

    pub fn set_half_leading(&mut self, half_leading: bool) -> &mut Self {
        self.native_mut().fHalfLeading = half_leading;
        self
    }

    pub fn half_leading(&self) -> bool {
        self.native().fHalfLeading
    }

    pub fn set_letter_spacing(&mut self, letter_spacing: scalar) -> &mut Self {
        self.native_mut().fLetterSpacing = letter_spacing;
        self
    }

    pub fn letter_spacing(&self) -> scalar {
        self.native().fLetterSpacing
    }

    pub fn set_word_spacing(&mut self, word_spacing: scalar) -> &mut Self {
        self.native_mut().fWordSpacing = word_spacing;
        self
    }

    pub fn word_spacing(&self) -> scalar {
        self.native().fWordSpacing
    }

    pub fn typeface(&self) -> Option<Typeface> {
        Typeface::from_unshared_ptr(self.native().fTypeface.fPtr)
    }

    pub fn set_typeface(&mut self, typeface: impl Into<Option<Typeface>>) -> &mut Self {
        unsafe {
            sb::C_TextStyle_setTypeface(self.native_mut(), typeface.into().into_ptr_or_null())
        }
        self
    }

    pub fn locale(&self) -> &str {
        self.native().fLocale.as_str()
    }

    pub fn set_locale(&mut self, locale: impl AsRef<str>) -> &mut Self {
        self.native_mut().fLocale.set_str(locale);
        self
    }

    pub fn text_baseline(&self) -> TextBaseline {
        self.native().fTextBaseline
    }

    pub fn set_text_baseline(&mut self, baseline: TextBaseline) -> &mut Self {
        self.native_mut().fTextBaseline = baseline;
        self
    }

    pub fn font_metrics(&self) -> FontMetrics {
        FontMetrics::construct(|fm| unsafe { self.native().getFontMetrics(fm) })
    }

    pub fn is_placeholder(&self) -> bool {
        self.native().fIsPlaceholder
    }

    pub fn set_placeholder(&mut self) -> &mut Self {
        self.native_mut().fIsPlaceholder = true;
        self
    }

    pub fn set_font_edging(&mut self, edging: font::Edging) -> &mut Self {
        self.native_mut().fEdging = edging;
        self
    }

    pub fn font_edging(&self) -> font::Edging {
        self.native().fEdging
    }

    pub fn set_subpixel(&mut self, subpixel: bool) -> &mut Self {
        self.native_mut().fSubpixel = subpixel;
        self
    }

    pub fn subpixel(&self) -> bool {
        self.native().fSubpixel
    }

    pub fn set_font_hinting(&mut self, hinting: FontHinting) -> &mut Self {
        self.native_mut().fHinting = hinting;
        self
    }

    pub fn font_hinting(&self) -> FontHinting {
        self.native().fHinting
    }
}

pub type TextIndex = usize;
pub type TextRange = Range<usize>;
pub const EMPTY_TEXT: TextRange = EMPTY_RANGE;

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub struct Block {
    pub range: TextRange,
    pub style: TextStyle,
}

native_transmutable!(sb::skia_textlayout_Block, Block);

impl Default for Block {
    fn default() -> Self {
        Self {
            range: EMPTY_RANGE,
            style: Default::default(),
        }
    }
}

impl Block {
    pub fn new(text_range: TextRange, style: TextStyle) -> Self {
        Self {
            range: text_range,
            style,
        }
    }

    pub fn add(&mut self, tail: TextRange) -> &mut Self {
        debug_assert!(self.range.end == tail.start);
        self.range = self.range.start..self.range.start + self.range.width() + tail.width();
        self
    }
}

pub type BlockIndex = usize;
pub type BlockRange = Range<usize>;

pub const EMPTY_BLOCK: usize = EMPTY_INDEX;
pub const EMPTY_BLOCKS: Range<usize> = EMPTY_RANGE;

#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub struct Placeholder {
    pub range: TextRange,
    pub style: PlaceholderStyle,
    pub text_style: TextStyle,
    pub blocks_before: BlockRange,
    pub text_before: TextRange,
}

native_transmutable!(sb::skia_textlayout_Placeholder, Placeholder);

impl Default for Placeholder {
    fn default() -> Self {
        #[allow(clippy::reversed_empty_ranges)]
        Self {
            range: EMPTY_RANGE,
            style: Default::default(),
            text_style: Default::default(),
            blocks_before: 0..0,
            text_before: 0..0,
        }
    }
}

impl Placeholder {
    pub fn new(
        range: Range<usize>,
        style: PlaceholderStyle,
        text_style: TextStyle,
        blocks_before: BlockRange,
        text_before: TextRange,
    ) -> Self {
        Self {
            range,
            style,
            text_style,
            blocks_before,
            text_before,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn getting_setting_comparing_font_arguments() {
        let mut ts = TextStyle::new();
        let mut fa = crate::FontArguments::default();
        fa.set_collection_index(100);
        ts.set_font_arguments(&fa);
        let tl_fa: FontArguments = fa.into();
        let fa = ts.font_arguments().unwrap();
        assert_eq!(tl_fa, *fa);
        let default_fa: FontArguments = crate::FontArguments::default().into();
        assert_ne!(default_fa, *fa);
    }
}
