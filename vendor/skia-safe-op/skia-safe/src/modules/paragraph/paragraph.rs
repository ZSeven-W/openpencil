use std::{ffi, fmt, ops::Range};

use skia_bindings as sb;

use super::{
    LineMetrics, PositionWithAffinity, RectHeightStyle, RectWidthStyle, TextBox, TextDirection,
    TextIndex, TextRange,
};
use crate::{
    interop::{Sink, VecSink},
    prelude::*,
    scalar, Canvas, Font, GlyphId, Path, Point, Rect, Size, TextBlob, Unichar,
};

pub type Paragraph = RefHandle<sb::skia_textlayout_Paragraph>;
// <https://github.com/rust-skia/rust-skia/issues/537>
// unsafe_send_sync!(Paragraph);

impl NativeDrop for sb::skia_textlayout_Paragraph {
    fn drop(&mut self) {
        unsafe { sb::C_Paragraph_delete(self) }
    }
}

impl fmt::Debug for Paragraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paragraph")
            .field("max_width", &self.max_width())
            .field("height", &self.height())
            .field("min_intrinsic_width", &self.min_intrinsic_width())
            .field("max_intrinsic_width", &self.max_intrinsic_width())
            .field("alphabetic_baseline", &self.alphabetic_baseline())
            .field("ideographic_baseline", &self.ideographic_baseline())
            .field("longest_line", &self.longest_line())
            .field("did_exceed_max_lines", &self.did_exceed_max_lines())
            .field("line_number", &self.line_number())
            .finish()
    }
}

impl Paragraph {
    pub fn max_width(&self) -> scalar {
        self.native().fWidth
    }

    pub fn height(&self) -> scalar {
        self.native().fHeight
    }

    pub fn min_intrinsic_width(&self) -> scalar {
        self.native().fMinIntrinsicWidth
    }

    pub fn max_intrinsic_width(&self) -> scalar {
        self.native().fMaxIntrinsicWidth
    }

    pub fn alphabetic_baseline(&self) -> scalar {
        self.native().fAlphabeticBaseline
    }

    pub fn ideographic_baseline(&self) -> scalar {
        self.native().fIdeographicBaseline
    }

    pub fn longest_line(&self) -> scalar {
        self.native().fLongestLine
    }

    pub fn did_exceed_max_lines(&self) -> bool {
        self.native().fExceededMaxLines
    }

    pub fn layout(&mut self, width: scalar) {
        unsafe { sb::C_Paragraph_layout(self.native_mut(), width) }
    }

    pub fn paint(&self, canvas: &Canvas, p: impl Into<Point>) {
        let p = p.into();
        unsafe { sb::C_Paragraph_paint(self.native_mut_force(), canvas.native_mut(), p.x, p.y) }
    }

    /// Returns a vector of bounding boxes that enclose all text between
    /// start and end glyph indexes, including start and excluding end
    pub fn get_rects_for_range(
        &self,
        range: Range<usize>,
        rect_height_style: RectHeightStyle,
        rect_width_style: RectWidthStyle,
    ) -> Vec<TextBox> {
        let mut result: Vec<TextBox> = Vec::new();

        let mut set_tb = |tbs: &[sb::skia_textlayout_TextBox]| {
            result = tbs.iter().map(TextBox::from_native_ref).cloned().collect();
        };

        unsafe {
            sb::C_Paragraph_getRectsForRange(
                self.native_mut_force(),
                range.start.try_into().unwrap(),
                range.end.try_into().unwrap(),
                rect_height_style.into_native(),
                rect_width_style.into_native(),
                VecSink::new(&mut set_tb).native_mut(),
            );
        }
        result
    }

    pub fn get_rects_for_placeholders(&self) -> Vec<TextBox> {
        let mut result = Vec::new();

        let mut set_tb = |tbs: &[sb::skia_textlayout_TextBox]| {
            result = tbs.iter().map(TextBox::from_native_ref).cloned().collect();
        };

        unsafe {
            sb::C_Paragraph_getRectsForPlaceholders(
                self.native_mut_force(),
                VecSink::new(&mut set_tb).native_mut(),
            )
        }
        result
    }

    /// Returns the index of the glyph that corresponds to the provided coordinate,
    /// with the top left corner as the origin, and +y direction as down
    pub fn get_glyph_position_at_coordinate(&self, p: impl Into<Point>) -> PositionWithAffinity {
        let p = p.into();
        let mut r = Default::default();
        unsafe {
            sb::C_Paragraph_getGlyphPositionAtCoordinate(self.native_mut_force(), p.x, p.y, &mut r)
        }
        r
    }

    /// Finds the first and last glyphs that define a word containing
    /// the glyph at index offset
    pub fn get_word_boundary(&self, offset: u32) -> Range<usize> {
        let mut range: [usize; 2] = Default::default();
        unsafe {
            sb::C_Paragraph_getWordBoundary(self.native_mut_force(), offset, range.as_mut_ptr())
        }
        range[0]..range[1]
    }

    pub fn get_line_metrics(&self) -> Vec<LineMetrics> {
        let mut result: Vec<LineMetrics> = Vec::new();
        let mut set_lm = |lms: &[sb::skia_textlayout_LineMetrics]| {
            result = lms.iter().map(LineMetrics::from_native_ref).collect();
        };

        unsafe {
            sb::C_Paragraph_getLineMetrics(
                self.native_mut_force(),
                VecSink::new(&mut set_lm).native_mut(),
            )
        }

        result
    }

    pub fn line_number(&self) -> usize {
        unsafe { sb::C_Paragraph_lineNumber(self.native_mut_force()) }
    }

    pub fn mark_dirty(&mut self) {
        unsafe { sb::C_Paragraph_markDirty(self.native_mut()) }
    }

    /// This function will return the number of unresolved glyphs or
    /// `None` if not applicable (has not been shaped yet - valid case)
    pub fn unresolved_glyphs(&mut self) -> Option<usize> {
        unsafe { sb::C_Paragraph_unresolvedGlyphs(self.native_mut()) }
            .try_into()
            .ok()
    }

    pub fn unresolved_codepoints(&mut self) -> Vec<Unichar> {
        let mut result = Vec::new();

        let mut set_chars = |chars: &[Unichar]| {
            result = chars.to_vec();
        };

        unsafe {
            sb::C_Paragraph_unresolvedCodepoints(
                self.native_mut_force(),
                VecSink::new(&mut set_chars).native_mut(),
            )
        }

        result
    }

    pub fn visit<'a, F>(&mut self, mut visitor: F)
    where
        F: FnMut(usize, Option<&'a VisitorInfo>),
    {
        unsafe {
            sb::C_Paragraph_visit(
                self.native_mut(),
                &mut visitor as *mut F as *mut _,
                Some(visitor_trampoline::<'a, F>),
            );
        }

        unsafe extern "C" fn visitor_trampoline<'a, F: FnMut(usize, Option<&'a VisitorInfo>)>(
            ctx: *mut ffi::c_void,
            index: usize,
            info: *const sb::skia_textlayout_Paragraph_VisitorInfo,
        ) {
            let info = if info.is_null() {
                None
            } else {
                Some(VisitorInfo::from_native_ref(&*info))
            };
            (*(ctx as *mut F))(index, info)
        }
    }

    pub fn extended_visit<'a, F>(&mut self, mut visitor: F)
    where
        F: FnMut(usize, Option<&'a ExtendedVisitorInfo>),
    {
        unsafe {
            sb::C_Paragraph_extendedVisit(
                self.native_mut(),
                &mut visitor as *mut F as *mut _,
                Some(visitor_trampoline::<'a, F>),
            );
        }

        unsafe extern "C" fn visitor_trampoline<
            'a,
            F: FnMut(usize, Option<&'a ExtendedVisitorInfo>),
        >(
            ctx: *mut ffi::c_void,
            index: usize,
            info: *const sb::skia_textlayout_Paragraph_ExtendedVisitorInfo,
        ) {
            let info = if info.is_null() {
                None
            } else {
                Some(ExtendedVisitorInfo::from_native_ref(&*info))
            };
            (*(ctx as *mut F))(index, info)
        }
    }

    /// Returns path for a given line
    ///
    ///  * `line_number` - a line number
    ///  * `dest` - a resulting path
    ///  
    /// Returns: a number glyphs that could not be converted to path
    pub fn get_path_at(&mut self, line_number: usize) -> (usize, Path) {
        let mut path = Path::default();
        let unconverted_glyphs = unsafe {
            sb::C_Paragraph_getPath(
                self.native_mut(),
                line_number.try_into().unwrap(),
                path.native_mut(),
            )
        };
        (unconverted_glyphs.try_into().unwrap(), path)
    }

    /// Returns path for a text blob
    ///
    /// * `text_blob` - a text blob
    ///
    /// Returns: a path
    pub fn get_path(text_blob: &mut TextBlob) -> Path {
        Path::construct(|p| unsafe { sb::C_Paragraph_GetPath(text_blob.native_mut(), p) })
    }

    /// Checks if a given text blob contains
    /// glyph with emoji
    ///
    /// * `text_blob` - a text blob
    ///
    /// Returns: `true` if there is such a glyph
    pub fn contains_emoji(&mut self, text_blob: &mut TextBlob) -> bool {
        unsafe { sb::C_Paragraph_containsEmoji(self.native_mut(), text_blob.native_mut()) }
    }

    /// Checks if a given text blob contains colored font or bitmap
    ///
    /// * `text_blob` - a text blob
    ///
    /// Returns: `true` if there is such a glyph
    pub fn contains_color_font_or_bitmap(&mut self, text_blob: &mut TextBlob) -> bool {
        unsafe {
            sb::C_Paragraph_containsColorFontOrBitmap(self.native_mut(), text_blob.native_mut())
        }
    }

    /// Finds the line number of the line that contains the given UTF-8 index.
    ///
    ///  * `index` - a UTF-8 TextIndex into the paragraph
    ///
    ///  Returns: the line number the glyph that corresponds to the
    ///           given `code_unit_index` is in, or -1 if the `code_unit_index`
    ///           is out of bounds, or when the glyph is truncated or
    ///           ellipsized away.
    pub fn get_line_number_at(&self, code_unit_index: TextIndex) -> Option<usize> {
        // Returns -1 if `code_unit_index` is out of range.
        unsafe { sb::C_Paragraph_getLineNumberAt(self.native(), code_unit_index) }
            .try_into()
            .ok()
    }

    /// Finds the line number of the line that contains the given UTF-16 index.
    ///
    /// * `index` - a UTF-16 offset into the paragraph
    ///
    /// Returns: the line number the glyph that corresponds to the
    ///          given `code_unit_index` is in, or -1 if the `code_unit_index`
    ///          is out of bounds, or when the glyph is truncated or
    ///          ellipsized away.
    pub fn get_line_number_at_utf16_offset(&self, code_unit_index: TextIndex) -> Option<usize> {
        // Returns -1 if `code_unit_index` is out of range.
        unsafe {
            sb::C_Paragraph_getLineNumberAtUTF16Offset(self.native_mut_force(), code_unit_index)
        }
        .try_into()
        .ok()
    }

    /// Returns line metrics info for the line
    ///
    /// * `line_number` - a line number
    /// * `line_metrics` - an address to return the info (in case of null just skipped)
    ///
    /// Returns: `true` if the line is found; `false` if not
    pub fn get_line_metrics_at(&self, line_number: usize) -> Option<LineMetrics> {
        let mut r = None;
        let mut set_lm = |lm: &sb::skia_textlayout_LineMetrics| {
            r = Some(LineMetrics::from_native_ref(lm));
        };
        unsafe {
            sb::C_Paragraph_getLineMetricsAt(
                self.native(),
                line_number,
                Sink::new(&mut set_lm).native_mut(),
            )
        }
        r
    }

    /// Returns the visible text on the line (excluding a possible ellipsis)
    ///
    /// * `line_number` - a line number
    /// * `include_spaces` - indicates if the whitespaces should be included
    ///
    /// Returns: the range of the text that is shown in the line
    pub fn get_actual_text_range(&self, line_number: usize, include_spaces: bool) -> TextRange {
        let mut range = [0usize; 2];
        unsafe {
            sb::C_Paragraph_getActualTextRange(
                self.native(),
                line_number,
                include_spaces,
                range.as_mut_ptr(),
            )
        }
        TextRange {
            start: range[0],
            end: range[1],
        }
    }

    /// Finds a glyph cluster for text index
    ///
    /// * `code_unit_index` - a text index
    /// * `glyph_info` - a glyph cluster info filled if not null
    ///
    /// Returns: `true` if glyph cluster was found; `false` if not
    pub fn get_glyph_cluster_at(&self, code_unit_index: TextIndex) -> Option<GlyphClusterInfo> {
        let mut r = None;
        let mut set_fn = |gci: &sb::skia_textlayout_Paragraph_GlyphClusterInfo| {
            r = Some(GlyphClusterInfo::from_native_ref(gci))
        };
        unsafe {
            sb::C_Paragraph_getGlyphClusterAt(
                self.native(),
                code_unit_index,
                Sink::new(&mut set_fn).native_mut(),
            )
        }
        r
    }

    /// Finds the closest glyph cluster for a visual text position
    ///
    /// * `dx` - x coordinate
    /// * `dy` - y coordinate
    /// * `glyph_info` - a glyph cluster info filled if not null
    ///
    /// Returns: `true` if glyph cluster was found; `false` if not
    ///          (which usually means the paragraph is empty)
    pub fn get_closest_glyph_cluster_at(&self, d: impl Into<Point>) -> Option<GlyphClusterInfo> {
        let mut r = None;
        let mut set_fn = |gci: &sb::skia_textlayout_Paragraph_GlyphClusterInfo| {
            r = Some(GlyphClusterInfo::from_native_ref(gci))
        };
        let d = d.into();
        unsafe {
            sb::C_Paragraph_getClosestGlyphClusterAt(
                self.native(),
                d.x,
                d.y,
                Sink::new(&mut set_fn).native_mut(),
            )
        }
        r
    }

    /// Retrieves the information associated with the glyph located at the given
    ///  `code_unit_index`.
    ///
    /// * `code_unit_index` - a UTF-16 offset into the paragraph
    /// * `glyph_info` - an optional GlyphInfo struct to hold the
    ///                  information associated with the glyph found at the
    ///                  given index
    ///
    /// Returns: `false` only if the offset is out of bounds
    pub fn get_glyph_info_at_utf16_offset(&mut self, code_unit_index: usize) -> Option<GlyphInfo> {
        GlyphInfo::try_construct(|gi| unsafe {
            sb::C_Paragraph_getGlyphInfoAtUTF16Offset(self.native_mut(), code_unit_index, gi)
        })
    }

    /// Finds the information associated with the closest glyph to the given
    /// paragraph coordinates.
    ///
    /// * `d` - x/y coordinate
    /// * `glyph_info` - an optional GlyphInfo struct to hold the
    ///                  information associated with the glyph found. The
    ///                  text indices and text ranges are described using
    ///                   UTF-16 offsets
    ///
    /// Returns: `true` if a grapheme cluster was found; `false` if not
    ///          (which usually means the paragraph is empty)
    pub fn get_closest_utf16_glyph_info_at(&mut self, d: impl Into<Point>) -> Option<GlyphInfo> {
        let d = d.into();
        GlyphInfo::try_construct(|gi| unsafe {
            sb::C_Paragraph_getClosestUTF16GlyphInfoAt(self.native_mut(), d.x, d.y, gi)
        })
    }

    /// Returns the font that is used to shape the text at the position
    ///
    /// * `code_unit_index` - text index
    ///
    /// Returns: font info or an empty font info if the text is not found
    pub fn get_font_at(&self, code_unit_index: TextIndex) -> Font {
        Font::construct(|f| unsafe { sb::C_Paragraph_getFontAt(self.native(), code_unit_index, f) })
    }

    /// Returns the font used to shape the text at the given UTF-16 offset.
    ///
    /// * `code_unit_index` - a UTF-16 offset in the paragraph
    ///
    /// Returns: font info or an empty font info if the text is not found
    pub fn get_font_at_utf16_offset(&mut self, code_unit_index: usize) -> Font {
        Font::construct(|f| unsafe {
            sb::C_Paragraph_getFontAtUTF16Offset(self.native_mut(), code_unit_index, f)
        })
    }

    /// Returns the information about all the fonts used to shape the paragraph text
    ///
    /// Returns: a list of fonts and text ranges
    pub fn get_fonts(&self) -> Vec<FontInfo> {
        let mut result = Vec::new();
        let mut set_fn = |fis: &[sb::skia_textlayout_Paragraph_FontInfo]| {
            result = fis.iter().map(FontInfo::from_native_ref).collect();
        };
        unsafe { sb::C_Paragraph_getFonts(self.native(), VecSink::new(&mut set_fn).native_mut()) }
        result
    }
}

pub type VisitorInfo = Handle<sb::skia_textlayout_Paragraph_VisitorInfo>;

impl NativeDrop for sb::skia_textlayout_Paragraph_VisitorInfo {
    fn drop(&mut self) {
        panic!("Internal error, Paragraph visitor can't be created in Rust")
    }
}

impl fmt::Debug for VisitorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisitorInfo")
            .field("font", &self.font())
            .field("origin", &self.origin())
            .field("advance_x", &self.advance_x())
            .field("count", &self.count())
            .field("glyphs", &self.glyphs())
            .field("positions", &self.positions())
            .field("utf8_starts", &self.utf8_starts())
            .field("flags", &self.flags())
            .finish()
    }
}

impl VisitorInfo {
    pub fn font(&self) -> &Font {
        Font::from_native_ref(unsafe { &*self.native().font })
    }

    pub fn origin(&self) -> Point {
        Point::from_native_c(self.native().origin)
    }

    pub fn advance_x(&self) -> scalar {
        self.native().advanceX
    }

    pub fn count(&self) -> usize {
        self.native().count as usize
    }

    pub fn glyphs(&self) -> &[GlyphId] {
        unsafe { safer::from_raw_parts(self.native().glyphs, self.count()) }
    }

    pub fn positions(&self) -> &[Point] {
        unsafe {
            safer::from_raw_parts(
                Point::from_native_ptr(self.native().positions),
                self.count(),
            )
        }
    }

    pub fn utf8_starts(&self) -> &[u32] {
        unsafe { safer::from_raw_parts(self.native().utf8Starts, self.count() + 1) }
    }

    pub fn flags(&self) -> VisitorFlags {
        VisitorFlags::from_bits_truncate(self.native().flags)
    }
}

pub type ExtendedVisitorInfo = Handle<sb::skia_textlayout_Paragraph_ExtendedVisitorInfo>;

impl NativeDrop for sb::skia_textlayout_Paragraph_ExtendedVisitorInfo {
    fn drop(&mut self) {
        panic!("Internal error, Paragraph extended visitor info can't be created in Rust")
    }
}

impl fmt::Debug for ExtendedVisitorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisitorInfo")
            .field("font", &self.font())
            .field("origin", &self.origin())
            .field("advance", &self.advance())
            .field("count", &self.count())
            .field("glyphs", &self.glyphs())
            .field("positions", &self.positions())
            .field("bounds", &self.bounds())
            .field("utf8_starts", &self.utf8_starts())
            .field("flags", &self.flags())
            .finish()
    }
}

impl ExtendedVisitorInfo {
    pub fn font(&self) -> &Font {
        Font::from_native_ref(unsafe { &*self.native().font })
    }

    pub fn origin(&self) -> Point {
        Point::from_native_c(self.native().origin)
    }

    pub fn advance(&self) -> Size {
        Size::from_native_c(self.native().advance)
    }

    pub fn count(&self) -> usize {
        self.native().count as usize
    }

    pub fn glyphs(&self) -> &[GlyphId] {
        unsafe { safer::from_raw_parts(self.native().glyphs, self.count()) }
    }

    pub fn positions(&self) -> &[Point] {
        unsafe {
            safer::from_raw_parts(
                Point::from_native_ptr(self.native().positions),
                self.count(),
            )
        }
    }

    pub fn bounds(&self) -> &[Rect] {
        let ptr = Rect::from_native_ptr(self.native().bounds);
        unsafe { safer::from_raw_parts(ptr, self.count()) }
    }

    pub fn utf8_starts(&self) -> &[u32] {
        unsafe { safer::from_raw_parts(self.native().utf8Starts, self.count() + 1) }
    }

    pub fn flags(&self) -> VisitorFlags {
        VisitorFlags::from_bits_truncate(self.native().flags)
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct VisitorFlags: u32 {
        const WHITE_SPACE = sb::skia_textlayout_Paragraph_VisitorFlags_kWhiteSpace_VisitorFlag as _;
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct GlyphClusterInfo {
    pub bounds: Rect,
    pub text_range: TextRange,
    pub position: TextDirection,
}

impl GlyphClusterInfo {
    fn from_native_ref(native: &sb::skia_textlayout_Paragraph_GlyphClusterInfo) -> Self {
        unsafe {
            Self {
                bounds: *Rect::from_native_ptr(&native.fBounds),
                text_range: TextRange {
                    start: native.fClusterTextRange.start,
                    end: native.fClusterTextRange.end,
                },
                position: native.fGlyphClusterPosition,
            }
        }
    }
}

/// The glyph and grapheme cluster information associated with a unicode
/// codepoint in the paragraph.
#[repr(C)]
#[derive(Clone, PartialEq, Debug)]
pub struct GlyphInfo {
    pub grapheme_layout_bounds: Rect,
    pub grapheme_cluster_text_range: TextRange,
    pub text_direction: TextDirection,
    pub is_ellipsis: bool,
}
native_transmutable!(sb::skia_textlayout_Paragraph_GlyphInfo, GlyphInfo);

#[derive(Clone, PartialEq, Debug)]
pub struct FontInfo {
    pub font: Font,
    pub text_range: TextRange,
}

impl FontInfo {
    pub fn new(font: Font, text_range: &TextRange) -> Self {
        Self {
            font,
            text_range: text_range.clone(),
        }
    }

    fn from_native_ref(native: &sb::skia_textlayout_Paragraph_FontInfo) -> Self {
        Self {
            font: Font::from_native_ref(&native.fFont).clone(),
            text_range: TextRange {
                start: native.fTextRange.start,
                end: native.fTextRange.end,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Paragraph;
    use crate::{
        icu,
        textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, TextStyle},
        FontMgr,
    };

    #[test]
    #[serial_test::serial]
    fn test_line_metrics() {
        let paragraph = mk_lorem_ipsum_paragraph();
        let line_metrics = paragraph.get_line_metrics();
        for (line, lm) in line_metrics.iter().enumerate() {
            println!("line {}: width: {}", line + 1, lm.width)
        }
    }

    /// Regression test for <https://github.com/rust-skia/rust-skia/issues/585>
    #[test]
    #[serial_test::serial]
    fn test_style_metrics() {
        icu::init();

        let mut style = ParagraphStyle::new();
        let ts = TextStyle::new();
        style.set_text_style(&ts);
        let mut font_collection = FontCollection::new();
        font_collection.set_default_font_manager(FontMgr::default(), None);
        let mut paragraph_builder = ParagraphBuilder::new(&style, font_collection);
        paragraph_builder.add_text("Lorem ipsum dolor sit amet\n");
        let mut paragraph = paragraph_builder.build();
        paragraph.layout(100.0);

        let line_metrics = &paragraph.get_line_metrics()[0];
        line_metrics.get_style_metrics(line_metrics.start_index..line_metrics.end_index);
    }

    #[test]
    #[serial_test::serial]
    fn test_font_infos() {
        let paragraph = mk_lorem_ipsum_paragraph();
        let infos = paragraph.get_fonts();
        assert!(!infos.is_empty())
    }

    #[test]
    #[serial_test::serial]
    fn test_visit() {
        let mut paragraph = mk_lorem_ipsum_paragraph();
        let visitor = |line, info| {
            println!("line {line}: {info:?}");
        };
        paragraph.visit(visitor);
    }

    #[test]
    #[serial_test::serial]
    fn test_extended_visit() {
        let mut paragraph = mk_lorem_ipsum_paragraph();
        let visitor = |line, info| {
            println!("line {line}: {info:?}");
        };
        paragraph.extended_visit(visitor);
    }

    fn mk_lorem_ipsum_paragraph() -> Paragraph {
        icu::init();

        let mut font_collection = FontCollection::new();
        font_collection.set_default_font_manager(FontMgr::new(), None);
        let paragraph_style = ParagraphStyle::new();
        let mut paragraph_builder = ParagraphBuilder::new(&paragraph_style, font_collection);
        let ts = TextStyle::new();
        paragraph_builder.push_style(&ts);
        paragraph_builder.add_text(LOREM_IPSUM);
        let mut paragraph = paragraph_builder.build();
        paragraph.layout(256.0);

        return paragraph;

        static LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur at leo at nulla tincidunt placerat. Proin eget purus augue. Quisque et est ullamcorper, pellentesque felis nec, pulvinar massa. Aliquam imperdiet, nulla ut dictum euismod, purus dui pulvinar risus, eu suscipit elit neque ac est. Nullam eleifend justo quis placerat ultricies. Vestibulum ut elementum velit. Praesent et dolor sit amet purus bibendum mattis. Aliquam erat volutpat.";
    }

    /// <https://github.com/rust-skia/rust-skia/issues/984>
    #[test]
    #[serial_test::serial]
    fn skia_crash_macos() {
        let mut font_collection = FontCollection::new();
        font_collection.set_dynamic_font_manager(FontMgr::default());
        let mut p = ParagraphBuilder::new(&ParagraphStyle::new(), font_collection);
        p.add_text("👋test test 🦀");
        let mut paragraph = p.build();
        paragraph.layout(200.);
    }
}
