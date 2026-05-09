use std::{
    ffi::{CStr, CString},
    fmt,
    marker::PhantomData,
    os::raw,
};

use skia_bindings::{
    self as sb, RustRunHandler, SkShaper, SkShaper_BiDiRunIterator, SkShaper_FontRunIterator,
    SkShaper_LanguageRunIterator, SkShaper_RunHandler, SkShaper_RunIterator,
    SkShaper_ScriptRunIterator, SkTextBlobBuilderRunHandler,
};

use crate::{prelude::*, scalar, Font, FontMgr, FourByteTag, Point, TextBlob};

// The following three are re-exported in `modules.rs` via `mod shapers {}`.
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) mod core_text;
pub(crate) mod harfbuzz;
pub(crate) mod unicode;

pub use run_handler::RunHandler;

pub type Shaper = RefHandle<SkShaper>;
unsafe_send_sync!(Shaper);

impl NativeDrop for SkShaper {
    fn drop(&mut self) {
        unsafe { sb::C_SkShaper_delete(self) }
    }
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new(None)
    }
}

impl fmt::Debug for Shaper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shaper").finish()
    }
}

impl Shaper {
    #[deprecated(since = "0.74.0", note = "use shapers::primitive::primitive_text()")]
    pub fn new_primitive() -> Self {
        crate::shapers::primitive::primitive_text()
    }

    pub fn new_shaper_driven_wrapper(
        fallback_font_mgr: impl Into<Option<FontMgr>>,
    ) -> Option<Self> {
        crate::shapers::hb::shaper_driven_wrapper(fallback_font_mgr)
    }

    pub fn new_shape_then_wrap(fallback_font_mgr: impl Into<Option<FontMgr>>) -> Option<Self> {
        crate::shapers::hb::shape_then_wrap(fallback_font_mgr)
    }

    #[deprecated(
        since = "0.74.0",
        note = "use shapers::hb::shape_dont_wrap_or_reorder()"
    )]
    pub fn new_shape_dont_wrap_or_reorder(
        fallback_font_mgr: impl Into<Option<FontMgr>>,
    ) -> Option<Self> {
        crate::shapers::hb::shape_dont_wrap_or_reorder(fallback_font_mgr)
    }

    pub fn purge_harf_buzz_cache() {
        unsafe { sb::SkShaper_PurgeHarfBuzzCache() }
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn new_core_text(line_break_mode: crate::shapers::ct::LineBreakMode) -> Self {
        #[cfg(feature = "embed-icudtl")]
        crate::icu::init();

        Self::from_ptr(unsafe { sb::C_SkShaper_MakeCoreText(line_break_mode) }).unwrap()
    }

    pub fn new(font_mgr: impl Into<Option<FontMgr>>) -> Self {
        #[cfg(feature = "embed-icudtl")]
        crate::icu::init();

        Self::from_ptr(unsafe { sb::C_SkShaper_Make(font_mgr.into().into_ptr_or_null()) }).unwrap()
    }

    pub fn purge_caches() {
        unsafe { sb::SkShaper_PurgeCaches() }
    }
}

pub use skia_bindings::SkShaper_Feature as Feature;

pub trait RunIterator {
    fn consume(&mut self);
    fn end_of_current_run(&self) -> usize;
    fn at_end(&self) -> bool;
}

impl<T> RunIterator for RefHandle<T>
where
    T: NativeDrop,
    T: NativeBase<SkShaper_RunIterator>,
{
    fn consume(&mut self) {
        unsafe { sb::C_SkShaper_RunIterator_consume(self.native_mut().base_mut()) }
    }

    fn end_of_current_run(&self) -> usize {
        unsafe { sb::C_SkShaper_RunIterator_endOfCurrentRun(self.native().base()) }
    }

    fn at_end(&self) -> bool {
        unsafe { sb::C_SkShaper_RunIterator_atEnd(self.native().base()) }
    }
}

pub type FontRunIterator = RefHandle<SkShaper_FontRunIterator>;

impl NativeBase<SkShaper_RunIterator> for SkShaper_FontRunIterator {}

impl NativeDrop for SkShaper_FontRunIterator {
    fn drop(&mut self) {
        unsafe { sb::C_SkShaper_RunIterator_delete(self.base_mut()) }
    }
}

impl fmt::Debug for FontRunIterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontRunIterator")
            .field("current_font", &self.current_font())
            .finish()
    }
}

impl FontRunIterator {
    pub fn current_font(&self) -> &Font {
        Font::from_native_ref(unsafe {
            &*sb::C_SkShaper_FontRunIterator_currentFont(self.native())
        })
    }
}

impl Shaper {
    pub fn new_font_mgr_run_iterator<'a>(
        utf8: &'a str,
        font: &Font,
        fallback: impl Into<Option<FontMgr>>,
    ) -> Borrows<'a, FontRunIterator> {
        let bytes = utf8.as_bytes();
        FontRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_MakeFontMgrRunIterator(
                bytes.as_ptr() as _,
                bytes.len(),
                font.native(),
                fallback.into().into_ptr_or_null(),
            )
        })
        .unwrap()
        .borrows(utf8)
    }

    // TODO: m79: wrap MakeFontMgrRunIterator with requestName (borrowed), requestStyle and
    //       a LanguageRunIterator.

    pub fn new_trivial_font_run_iterator(font: &Font, utf8_bytes: usize) -> FontRunIterator {
        FontRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_TrivialFontRunIterator_new(font.native(), utf8_bytes)
        })
        .unwrap()
    }
}

pub type BiDiRunIterator = RefHandle<SkShaper_BiDiRunIterator>;

impl NativeBase<SkShaper_RunIterator> for SkShaper_BiDiRunIterator {}

impl NativeDrop for SkShaper_BiDiRunIterator {
    fn drop(&mut self) {
        unsafe { sb::C_SkShaper_RunIterator_delete(self.base_mut()) }
    }
}

impl fmt::Debug for BiDiRunIterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BiDiRunIterator")
            .field("current_level", &self.current_level())
            .finish()
    }
}

impl BiDiRunIterator {
    pub fn current_level(&self) -> u8 {
        unsafe { sb::C_SkShaper_BiDiRunIterator_currentLevel(self.native()) }
    }
}

impl Shaper {
    pub fn new_bidi_run_iterator(utf8: &str, bidi_level: u8) -> Option<Borrows<BiDiRunIterator>> {
        let bytes = utf8.as_bytes();
        BiDiRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_MakeBidiRunIterator(bytes.as_ptr() as _, bytes.len(), bidi_level)
        })
        .map(|i| i.borrows(utf8))
    }

    pub fn new_icu_bidi_run_iterator(utf8: &str, level: u8) -> Option<Borrows<BiDiRunIterator>> {
        let bytes = utf8.as_bytes();
        BiDiRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_MakeIcuBidiRunIterator(bytes.as_ptr() as _, bytes.len(), level)
        })
        .map(|i| i.borrows(utf8))
    }

    #[deprecated(
        since = "0.74.0",
        note = "use shapers::primitive::trivial_bidi_run_iterator()"
    )]
    pub fn new_trivial_bidi_run_iterator(bidi_level: u8, utf8_bytes: usize) -> BiDiRunIterator {
        shapers::primitive::trivial_bidi_run_iterator(bidi_level, utf8_bytes)
    }
}

pub type ScriptRunIterator = RefHandle<SkShaper_ScriptRunIterator>;

impl NativeBase<SkShaper_RunIterator> for SkShaper_ScriptRunIterator {}

impl NativeDrop for SkShaper_ScriptRunIterator {
    fn drop(&mut self) {
        unsafe { sb::C_SkShaper_RunIterator_delete(self.base_mut()) }
    }
}

impl fmt::Debug for ScriptRunIterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScriptRunIterator")
            .field("current_script", &self.current_script())
            .finish()
    }
}

impl ScriptRunIterator {
    pub fn current_script(&self) -> FourByteTag {
        FourByteTag::from_native_c(unsafe {
            sb::C_SkShaper_ScriptRunIterator_currentScript(self.native())
        })
    }
}

impl Shaper {
    pub fn new_script_run_iterator(utf8: &str, script: FourByteTag) -> Borrows<ScriptRunIterator> {
        let bytes = utf8.as_bytes();
        ScriptRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_MakeScriptRunIterator(
                bytes.as_ptr() as _,
                bytes.len(),
                script.into_native(),
            )
        })
        .unwrap()
        .borrows(utf8)
    }

    // TODO: wrap MakeSkUnicodeHbScriptRunIterator (m88: uses type SkUnicode defined in src/).

    pub fn new_hb_icu_script_run_iterator(utf8: &str) -> Borrows<ScriptRunIterator> {
        let bytes = utf8.as_bytes();
        ScriptRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_MakeHbIcuScriptRunIterator(bytes.as_ptr() as _, bytes.len())
        })
        .unwrap()
        .borrows(utf8)
    }

    #[deprecated(
        since = "0.74.0",
        note = "use shapers::primitive::trivial_script_run_iterator"
    )]
    pub fn new_trivial_script_run_iterator(bidi_level: u8, utf8_bytes: usize) -> ScriptRunIterator {
        shapers::primitive::trivial_script_run_iterator(bidi_level, utf8_bytes)
    }
}

pub type LanguageRunIterator = RefHandle<SkShaper_LanguageRunIterator>;

impl NativeBase<SkShaper_RunIterator> for SkShaper_LanguageRunIterator {}

impl NativeDrop for SkShaper_LanguageRunIterator {
    fn drop(&mut self) {
        unsafe { sb::C_SkShaper_RunIterator_delete(self.base_mut()) }
    }
}

impl fmt::Debug for LanguageRunIterator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LanguageRunIterator")
            .field("current_language", &self.current_language())
            .finish()
    }
}

impl LanguageRunIterator {
    pub fn current_language(&self) -> &CStr {
        unsafe {
            CStr::from_ptr(sb::C_SkShaper_LanguageRunIterator_currentLanguage(
                self.native(),
            ))
        }
    }
}

impl Shaper {
    pub fn new_std_language_run_iterator(utf8: &str) -> Option<LanguageRunIterator> {
        // a LanguageRunIterator never accesses the UTF8 string, so it's safe to
        // not borrow the string.
        let bytes = utf8.as_bytes();
        LanguageRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_MakeStdLanguageRunIterator(bytes.as_ptr() as _, bytes.len())
        })
    }

    pub fn new_trivial_language_run_iterator(
        language: impl AsRef<str>,
        utf8_bytes: usize,
    ) -> LanguageRunIterator {
        let language = CString::new(language.as_ref()).unwrap();
        LanguageRunIterator::from_ptr(unsafe {
            sb::C_SkShaper_TrivialLanguageRunIterator_new(language.as_ptr(), utf8_bytes)
        })
        .unwrap()
    }
}

pub mod run_handler {
    use std::{ffi::CStr, ops::Range, slice};

    use skia_bindings::{SkShaper_RunHandler_Buffer, SkShaper_RunHandler_RunInfo};

    use crate::{prelude::*, Font, FourByteTag, GlyphId, Point, Vector};

    pub trait RunHandler {
        fn begin_line(&mut self);
        fn run_info(&mut self, info: &RunInfo);
        fn commit_run_info(&mut self);
        fn run_buffer(&mut self, info: &RunInfo) -> Buffer;
        fn commit_run_buffer(&mut self, info: &RunInfo);
        fn commit_line(&mut self);
    }

    #[derive(Debug)]
    pub struct RunInfo<'a> {
        pub font: &'a Font,
        pub bidi_level: u8,
        pub script: FourByteTag,
        pub language: Option<&'a str>,
        pub advance: Vector,
        pub glyph_count: usize,
        pub utf8_range: Range<usize>,
    }

    impl RunInfo<'_> {
        pub(crate) fn from_native(ri: &SkShaper_RunHandler_RunInfo) -> Self {
            let utf8_range = ri.utf8Range;
            RunInfo {
                font: Font::from_native_ref(unsafe { &*ri.fFont }),
                bidi_level: ri.fBidiLevel,
                script: ri.fScript.into(),
                language: unsafe {
                    if ri.fLanguage.is_null() {
                        None
                    } else {
                        CStr::from_ptr(ri.fLanguage).to_str().ok()
                    }
                },
                advance: Vector::from_native_c(ri.fAdvance),
                glyph_count: ri.glyphCount,
                utf8_range: utf8_range.fBegin..utf8_range.fBegin + utf8_range.fSize,
            }
        }
    }

    #[derive(Debug)]
    pub struct Buffer<'a> {
        pub glyphs: &'a mut [GlyphId],
        pub positions: &'a mut [Point],
        pub offsets: Option<&'a mut [Point]>,
        pub clusters: Option<&'a mut [u32]>,
        pub point: Point,
    }

    impl<'a> Buffer<'a> {
        pub fn new(
            glyphs: &'a mut [GlyphId],
            positions: &'a mut [Point],
            point: impl Into<Option<Point>>,
        ) -> Self {
            Buffer {
                glyphs,
                positions,
                offsets: None,
                clusters: None,
                point: point.into().unwrap_or_default(),
            }
        }

        #[allow(unused)]
        pub(crate) unsafe fn from_native(
            buffer: &SkShaper_RunHandler_Buffer,
            glyph_count: usize,
        ) -> Buffer {
            let offsets = buffer.offsets.into_non_null().map(|mut offsets| {
                slice::from_raw_parts_mut(Point::from_native_ref_mut(offsets.as_mut()), glyph_count)
            });

            let clusters = buffer
                .clusters
                .into_non_null()
                .map(|clusters| slice::from_raw_parts_mut(clusters.as_ptr(), glyph_count));

            Buffer {
                glyphs: safer::from_raw_parts_mut(buffer.glyphs, glyph_count),
                positions: safer::from_raw_parts_mut(
                    Point::from_native_ptr_mut(buffer.positions),
                    glyph_count,
                ),
                offsets,
                clusters,
                point: Point::from_native_c(buffer.point),
            }
        }

        pub(crate) fn native_buffer_mut(
            &mut self,
            glyph_count: usize,
        ) -> SkShaper_RunHandler_Buffer {
            assert_eq!(self.glyphs.len(), glyph_count);
            assert_eq!(self.positions.len(), glyph_count);
            if let Some(offsets) = &self.offsets {
                assert_eq!(offsets.len(), glyph_count)
            }
            if let Some(clusters) = &self.clusters {
                assert_eq!(clusters.len(), glyph_count)
            }
            SkShaper_RunHandler_Buffer {
                glyphs: self.glyphs.as_mut_ptr(),
                positions: self.positions.native_mut().as_mut_ptr(),
                offsets: self.offsets.native_mut().as_ptr_or_null_mut(),
                clusters: self.clusters.as_ptr_or_null_mut(),
                point: self.point.into_native(),
            }
        }
    }
}

pub trait AsRunHandler<'a> {
    type RunHandler: AsNativeRunHandler + 'a;
    fn as_run_handler<'b>(&'b mut self) -> Self::RunHandler
    where
        'b: 'a;
}

/// A trait for accessing the native run handler instance used in the shape_native* functions.
pub trait AsNativeRunHandler {
    fn as_native_run_handler(&mut self) -> &mut SkShaper_RunHandler;
}

impl<'a, T: RunHandler> AsRunHandler<'a> for T {
    type RunHandler = RustRunHandler;

    fn as_run_handler<'b>(&'b mut self) -> Self::RunHandler
    where
        'b: 'a,
    {
        let param = unsafe { rust_run_handler::new_param(self) };
        rust_run_handler::from_param(&param)
    }
}

impl Shaper {
    pub fn shape<'a, 'b: 'a>(
        &self,
        utf8: &str,
        font: &Font,
        left_to_right: bool,
        width: scalar,
        run_handler: &'b mut impl AsRunHandler<'a>,
    ) {
        let bytes = utf8.as_bytes();
        let mut run_handler = run_handler.as_run_handler();

        unsafe {
            sb::C_SkShaper_shape(
                self.native(),
                bytes.as_ptr() as _,
                bytes.len(),
                font.native(),
                left_to_right,
                width,
                run_handler.as_native_run_handler(),
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_with_iterators<'a, 'b: 'a>(
        &self,
        utf8: &str,
        font_run_iterator: &mut FontRunIterator,
        bidi_run_iterator: &mut BiDiRunIterator,
        script_run_iterator: &mut ScriptRunIterator,
        language_run_iterator: &mut LanguageRunIterator,
        width: scalar,
        run_handler: &'b mut impl AsRunHandler<'a>,
    ) {
        let mut run_handler = run_handler.as_run_handler();

        let bytes = utf8.as_bytes();
        unsafe {
            sb::C_SkShaper_shape2(
                self.native(),
                bytes.as_ptr() as _,
                bytes.len(),
                font_run_iterator.native_mut(),
                bidi_run_iterator.native_mut(),
                script_run_iterator.native_mut(),
                language_run_iterator.native_mut(),
                width,
                run_handler.as_native_run_handler(),
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_with_iterators_and_features<'a, 'b: 'a>(
        &self,
        utf8: &str,
        font_run_iterator: &mut FontRunIterator,
        bidi_run_iterator: &mut BiDiRunIterator,
        script_run_iterator: &mut ScriptRunIterator,
        language_run_iterator: &mut LanguageRunIterator,
        features: &[Feature],
        width: scalar,
        run_handler: &'b mut impl AsRunHandler<'a>,
    ) {
        let mut run_handler = run_handler.as_run_handler();

        let bytes = utf8.as_bytes();
        unsafe {
            sb::C_SkShaper_shape3(
                self.native(),
                bytes.as_ptr() as _,
                bytes.len(),
                font_run_iterator.native_mut(),
                bidi_run_iterator.native_mut(),
                script_run_iterator.native_mut(),
                language_run_iterator.native_mut(),
                features.as_ptr(),
                features.len(),
                width,
                run_handler.as_native_run_handler(),
            )
        }
    }
}

mod rust_run_handler {
    use std::mem;

    use skia_bindings as sb;
    use skia_bindings::{
        RustRunHandler, RustRunHandler_Param, SkShaper_RunHandler, SkShaper_RunHandler_Buffer,
        SkShaper_RunHandler_RunInfo, TraitObject,
    };

    use crate::{
        prelude::*,
        shaper::{run_handler::RunInfo, AsNativeRunHandler, RunHandler},
    };

    impl NativeBase<SkShaper_RunHandler> for RustRunHandler {}

    impl AsNativeRunHandler for RustRunHandler {
        fn as_native_run_handler(&mut self) -> &mut SkShaper_RunHandler {
            self.base_mut()
        }
    }

    pub unsafe fn new_param(run_handler: &mut dyn RunHandler) -> RustRunHandler_Param {
        RustRunHandler_Param {
            trait_: mem::transmute::<&mut dyn RunHandler, TraitObject>(run_handler),
            beginLine: Some(begin_line),
            runInfo: Some(run_info),
            commitRunInfo: Some(commit_run_info),
            runBuffer: Some(run_buffer),
            commitRunBuffer: Some(commit_run_buffer),
            commitLine: Some(commit_line),
        }
    }

    pub fn from_param(param: &RustRunHandler_Param) -> RustRunHandler {
        construct(|rh| unsafe { sb::C_RustRunHandler_construct(rh, param) })
    }

    extern "C" fn begin_line(to: TraitObject) {
        to_run_handler(to).begin_line()
    }

    extern "C" fn run_info(to: TraitObject, ri: *const SkShaper_RunHandler_RunInfo) {
        to_run_handler(to).run_info(&RunInfo::from_native(unsafe { &*ri }));
    }

    extern "C" fn commit_run_info(to: TraitObject) {
        to_run_handler(to).commit_run_info()
    }

    extern "C" fn run_buffer(
        to: TraitObject,
        ri: *const SkShaper_RunHandler_RunInfo,
    ) -> SkShaper_RunHandler_Buffer {
        let ri = unsafe { &*ri };
        to_run_handler(to)
            .run_buffer(&RunInfo::from_native(ri))
            .native_buffer_mut(ri.glyphCount)
    }

    extern "C" fn commit_run_buffer(to: TraitObject, ri: *const SkShaper_RunHandler_RunInfo) {
        to_run_handler(to).commit_run_buffer(&RunInfo::from_native(unsafe { &*ri }))
    }

    extern "C" fn commit_line(to: TraitObject) {
        to_run_handler(to).commit_line()
    }

    fn to_run_handler<'a>(to: TraitObject) -> &'a mut dyn RunHandler {
        unsafe { mem::transmute(to) }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct TextBlobBuilderRunHandler<'text>(SkTextBlobBuilderRunHandler, PhantomData<&'text str>);

impl NativeAccess for TextBlobBuilderRunHandler<'_> {
    type Native = SkTextBlobBuilderRunHandler;

    fn native(&self) -> &SkTextBlobBuilderRunHandler {
        &self.0
    }
    fn native_mut(&mut self) -> &mut SkTextBlobBuilderRunHandler {
        &mut self.0
    }
}

impl NativeBase<SkShaper_RunHandler> for SkTextBlobBuilderRunHandler {}

impl TextBlobBuilderRunHandler<'_> {
    pub fn new(text: &str, offset: impl Into<Point>) -> TextBlobBuilderRunHandler {
        let ptr = text.as_ptr();
        // we can safely pass a ptr to the utf8 text string to the RunHandler, because it does not
        // expect it to be 0 terminated, but this introduces another problem because
        // we can never be sure that the RunHandler callbacks refer to that range. For
        // now we ensure that by not exposing the RunHandler of a TextBlobBuilder.
        let run_handler = construct(|rh| unsafe {
            sb::C_SkTextBlobBuilderRunHandler_construct(
                rh,
                ptr as *const raw::c_char,
                offset.into().native(),
            )
        });
        TextBlobBuilderRunHandler(run_handler, PhantomData)
    }

    pub fn make_blob(&mut self) -> Option<TextBlob> {
        TextBlob::from_ptr(unsafe { sb::C_SkTextBlobBuilderRunHandler_makeBlob(self.native_mut()) })
    }

    pub fn end_point(&mut self) -> Point {
        Point::from_native_c(unsafe {
            sb::C_SkTextBlobBuilderRunHandler_endPoint(self.native_mut())
        })
    }
}

impl<'a> AsRunHandler<'a> for TextBlobBuilderRunHandler<'_> {
    type RunHandler = &'a mut SkShaper_RunHandler;

    fn as_run_handler<'b>(&'b mut self) -> Self::RunHandler
    where
        'b: 'a,
    {
        (*self).as_native_run_handler()
    }
}

impl AsNativeRunHandler for &mut SkShaper_RunHandler {
    fn as_native_run_handler(&mut self) -> &mut SkShaper_RunHandler {
        self
    }
}

impl AsNativeRunHandler for TextBlobBuilderRunHandler<'_> {
    fn as_native_run_handler(&mut self) -> &mut SkShaper_RunHandler {
        self.0.base_mut()
    }
}

impl Shaper {
    pub fn shape_text_blob(
        &self,
        text: &str,
        font: &Font,
        left_to_right: bool,
        width: scalar,
        offset: impl Into<Point>,
    ) -> Option<(TextBlob, Point)> {
        let bytes = text.as_bytes();
        let mut builder = TextBlobBuilderRunHandler::new(text, offset);
        unsafe {
            sb::C_SkShaper_shape(
                self.native(),
                bytes.as_ptr() as _,
                bytes.len(),
                font.native(),
                left_to_right,
                width,
                builder.native_mut().base_mut(),
            )
        };
        builder.make_blob().map(|tb| (tb, builder.end_point()))
    }
}

pub(crate) mod shapers {
    use super::{BiDiRunIterator, ScriptRunIterator, Shaper};

    #[deprecated(since = "0.74.0", note = "use shapers::primitive::primitive_text()")]
    pub fn primitive() -> Shaper {
        primitive::primitive_text()
    }

    #[deprecated(
        since = "0.74.0",
        note = "use shapers::primitive::trivial_bidi_run_iterator()"
    )]
    pub fn trivial_bidi_run_iterator(bidi_level: u8, utf8_bytes: usize) -> BiDiRunIterator {
        primitive::trivial_bidi_run_iterator(bidi_level, utf8_bytes)
    }

    #[deprecated(
        since = "0.74.0",
        note = "use shapers::primitive::trivial_script_run_iterator()"
    )]
    pub fn trivial_script_run_iterator(bidi_level: u8, utf8_bytes: usize) -> ScriptRunIterator {
        primitive::trivial_script_run_iterator(bidi_level, utf8_bytes)
    }

    pub mod primitive {
        use skia_bindings as sb;

        use crate::shaper::{BiDiRunIterator, ScriptRunIterator, Shaper};

        pub fn primitive_text() -> Shaper {
            Shaper::from_ptr(unsafe { sb::C_SkShapers_Primitive_PrimitiveText() }).unwrap()
        }

        pub fn trivial_bidi_run_iterator(bidi_level: u8, utf8_bytes: usize) -> BiDiRunIterator {
            BiDiRunIterator::from_ptr(unsafe {
                sb::C_SkShapers_Primitive_TrivialBidiRunIterator_new(bidi_level, utf8_bytes)
            })
            .unwrap()
        }

        pub fn trivial_script_run_iterator(bidi_level: u8, utf8_bytes: usize) -> ScriptRunIterator {
            ScriptRunIterator::from_ptr(unsafe {
                sb::C_SkShapers_Primitive_TrivialScriptRunIterator_new(bidi_level, utf8_bytes)
            })
            .unwrap()
        }
    }
}

pub mod icu {
    /// On Windows, and if the default feature "embed-icudtl" is _not_ set, this function writes the
    /// file `icudtl.dat` into the current executable's directory making sure that it's available
    /// when text shaping is used in Skia.
    ///
    /// If your executable directory can not be written to, make sure that `icudtl.dat` is
    /// available.
    ///
    /// Note that it is currently not possible to load `icudtl.dat` from another location.
    ///
    /// If the default feature "embed-icudtl" is set, the `icudtl.dat` file is directly used from
    /// memory, so no `icudtl.dat` file is needed.
    pub fn init() {
        skia_bindings::icu::init();
    }

    #[test]
    #[serial_test::serial]
    fn test_text_blob_builder_run_handler() {
        use crate::{Font, FontMgr, FontStyle};
        init();
        let str = "العربية";
        let mut text_blob_builder_run_handler =
            crate::shaper::TextBlobBuilderRunHandler::new(str, crate::Point::default());

        let font_mgr = FontMgr::new();
        let default_typeface = font_mgr
            .legacy_make_typeface(None, FontStyle::default())
            .unwrap();
        let default_font = Font::new(default_typeface, 10.0);
        let shaper = crate::Shaper::new(font_mgr);
        shaper.shape(
            str,
            &default_font,
            false,
            10000.0,
            &mut text_blob_builder_run_handler,
        );

        let blob = text_blob_builder_run_handler.make_blob().unwrap();
        let bounds = blob.bounds();
        assert!(bounds.width() > 0.0 && bounds.height() > 0.0);
    }

    #[test]
    #[serial_test::serial]
    fn icu_init_is_idempotent() {
        init();
        init();
    }
}
