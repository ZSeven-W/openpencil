use std::{fmt, ptr, slice};

use skia_bindings::{
    self as sb, SkTextBlob, SkTextBlobBuilder, SkTextBlob_Iter, SkTextBlob_Iter_Run, SkTypeface,
};

use crate::{
    prelude::*, scalar, EncodedText, Font, GlyphId, Paint, Point, RSXform, Rect, Typeface,
};

pub type TextBlob = RCHandle<SkTextBlob>;
unsafe_send_sync!(TextBlob);
require_base_type!(SkTextBlob, sb::SkNVRefCnt);

impl NativeRefCounted for SkTextBlob {
    fn _ref(&self) {
        unsafe { sb::C_SkTextBlob_ref(self) };
    }

    fn _unref(&self) {
        unsafe { sb::C_SkTextBlob_unref(self) }
    }

    fn unique(&self) -> bool {
        unsafe { sb::C_SkTextBlob_unique(self) }
    }
}

impl fmt::Debug for TextBlob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextBlob")
            .field("bounds", &self.bounds())
            .field("unique_id", &self.unique_id())
            .finish()
    }
}

impl TextBlob {
    pub fn new(str: impl AsRef<str>, font: &Font) -> Option<Self> {
        Self::from_str(str, font)
    }

    pub fn bounds(&self) -> &Rect {
        Rect::from_native_ref(&self.native().fBounds)
    }

    pub fn unique_id(&self) -> u32 {
        self.native().fUniqueID
    }

    // TODO: consider to provide an inplace variant.
    pub fn get_intercepts(&self, bounds: [scalar; 2], paint: Option<&Paint>) -> Vec<scalar> {
        unsafe {
            let count = self.native().getIntercepts(
                bounds.as_ptr(),
                ptr::null_mut(),
                paint.native_ptr_or_null(),
            );
            let mut intervals = vec![Default::default(); count.try_into().unwrap()];
            let count_2 = self.native().getIntercepts(
                bounds.as_ptr(),
                intervals.as_mut_ptr(),
                paint.native_ptr_or_null(),
            );
            assert_eq!(count, count_2);
            intervals
        }
    }

    pub fn from_str(str: impl AsRef<str>, font: &Font) -> Option<TextBlob> {
        Self::from_text(str.as_ref(), font)
    }

    pub fn from_text(text: impl EncodedText, font: &Font) -> Option<TextBlob> {
        let (ptr, size, encoding) = text.as_raw();
        TextBlob::from_ptr(unsafe {
            sb::C_SkTextBlob_MakeFromText(ptr, size, font.native(), encoding.into_native())
        })
    }

    pub fn from_pos_text_h(
        text: impl EncodedText,
        x_pos: &[scalar],
        const_y: scalar,
        font: &Font,
    ) -> Option<TextBlob> {
        // TODO: avoid that somehow.
        assert_eq!(x_pos.len(), font.count_text(&text));
        let (ptr, size, encoding) = text.as_raw();
        TextBlob::from_ptr(unsafe {
            sb::C_SkTextBlob_MakeFromPosTextH(
                ptr,
                size,
                x_pos.as_ptr(),
                x_pos.len(),
                const_y,
                font.native(),
                encoding.into_native(),
            )
        })
    }

    pub fn from_pos_text(text: impl EncodedText, pos: &[Point], font: &Font) -> Option<TextBlob> {
        assert_eq!(pos.len(), font.count_text(&text));
        let (ptr, size, encoding) = text.as_raw();
        TextBlob::from_ptr(unsafe {
            sb::C_SkTextBlob_MakeFromPosText(
                ptr,
                size,
                pos.native().as_ptr(),
                pos.len(),
                font.native(),
                encoding.into_native(),
            )
        })
    }

    pub fn from_rsxform(
        text: impl EncodedText,
        xform: &[RSXform],
        font: &Font,
    ) -> Option<TextBlob> {
        assert_eq!(xform.len(), font.count_text(&text));
        let (ptr, size, encoding) = text.as_raw();
        TextBlob::from_ptr(unsafe {
            sb::C_SkTextBlob_MakeFromRSXform(
                ptr,
                size,
                xform.native().as_ptr(),
                xform.len(),
                font.native(),
                encoding.into_native(),
            )
        })
    }
}

pub type TextBlobBuilder = Handle<SkTextBlobBuilder>;
unsafe_send_sync!(TextBlobBuilder);

impl NativeDrop for SkTextBlobBuilder {
    fn drop(&mut self) {
        unsafe { sb::C_SkTextBlobBuilder_destruct(self) }
    }
}

impl fmt::Debug for TextBlobBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextBlobBuilder").finish()
    }
}

impl TextBlobBuilder {
    pub fn new() -> Self {
        Self::from_native_c(unsafe { SkTextBlobBuilder::new() })
    }

    pub fn make(&mut self) -> Option<TextBlob> {
        TextBlob::from_ptr(unsafe { sb::C_SkTextBlobBuilder_make(self.native_mut()) })
    }

    pub fn alloc_run(
        &mut self,
        font: &Font,
        count: usize,
        offset: impl Into<Point>,
        bounds: Option<&Rect>,
    ) -> &mut [GlyphId] {
        let offset = offset.into();
        unsafe {
            let buffer = &*self.native_mut().allocRun(
                font.native(),
                count.try_into().unwrap(),
                offset.x,
                offset.y,
                bounds.native_ptr_or_null(),
            );
            safer::from_raw_parts_mut(buffer.glyphs, count)
        }
    }

    pub fn alloc_run_pos_h(
        &mut self,
        font: &Font,
        count: usize,
        y: scalar,
        bounds: Option<&Rect>,
    ) -> (&mut [GlyphId], &mut [scalar]) {
        unsafe {
            let buffer = &*self.native_mut().allocRunPosH(
                font.native(),
                count.try_into().unwrap(),
                y,
                bounds.native_ptr_or_null(),
            );
            (
                safer::from_raw_parts_mut(buffer.glyphs, count),
                safer::from_raw_parts_mut(buffer.pos, count),
            )
        }
    }

    pub fn alloc_run_pos(
        &mut self,
        font: &Font,
        count: usize,
        bounds: Option<&Rect>,
    ) -> (&mut [GlyphId], &mut [Point]) {
        unsafe {
            let buffer = &*self.native_mut().allocRunPos(
                font.native(),
                count.try_into().unwrap(),
                bounds.native_ptr_or_null(),
            );
            (
                safer::from_raw_parts_mut(buffer.glyphs, count),
                safer::from_raw_parts_mut(buffer.pos as *mut Point, count),
            )
        }
    }

    pub fn alloc_run_rsxform(
        &mut self,
        font: &Font,
        count: usize,
    ) -> (&mut [GlyphId], &mut [RSXform]) {
        unsafe {
            let buffer = &*self
                .native_mut()
                .allocRunRSXform(font.native(), count.try_into().unwrap());
            (
                safer::from_raw_parts_mut(buffer.glyphs, count),
                safer::from_raw_parts_mut(buffer.pos as *mut RSXform, count),
            )
        }
    }

    pub fn alloc_run_text(
        &mut self,
        font: &Font,
        count: usize,
        offset: impl Into<Point>,
        text_byte_count: usize,
        bounds: Option<&Rect>,
    ) -> (&mut [GlyphId], &mut [u8], &mut [u32]) {
        let offset = offset.into();
        unsafe {
            let buffer = &*self.native_mut().allocRunText(
                font.native(),
                count.try_into().unwrap(),
                offset.x,
                offset.y,
                text_byte_count.try_into().unwrap(),
                bounds.native_ptr_or_null(),
            );
            (
                safer::from_raw_parts_mut(buffer.glyphs, count),
                safer::from_raw_parts_mut(buffer.utf8text as *mut u8, text_byte_count),
                safer::from_raw_parts_mut(buffer.clusters, count),
            )
        }
    }

    pub fn alloc_run_text_pos_h(
        &mut self,
        font: &Font,
        count: usize,
        y: scalar,
        text_byte_count: usize,
        bounds: Option<&Rect>,
    ) -> (&mut [GlyphId], &mut [scalar], &mut [u8], &mut [u32]) {
        unsafe {
            let buffer = &*self.native_mut().allocRunTextPosH(
                font.native(),
                count.try_into().unwrap(),
                y,
                text_byte_count.try_into().unwrap(),
                bounds.native_ptr_or_null(),
            );
            (
                safer::from_raw_parts_mut(buffer.glyphs, count),
                safer::from_raw_parts_mut(buffer.pos, count),
                safer::from_raw_parts_mut(buffer.utf8text as *mut u8, text_byte_count),
                safer::from_raw_parts_mut(buffer.clusters, count),
            )
        }
    }

    pub fn alloc_run_text_pos(
        &mut self,
        font: &Font,
        count: usize,
        text_byte_count: usize,
        bounds: Option<&Rect>,
    ) -> (&mut [GlyphId], &mut [Point], &mut [u8], &mut [u32]) {
        unsafe {
            let buffer = &*self.native_mut().allocRunTextPos(
                font.native(),
                count.try_into().unwrap(),
                text_byte_count.try_into().unwrap(),
                bounds.native_ptr_or_null(),
            );
            (
                safer::from_raw_parts_mut(buffer.glyphs, count),
                safer::from_raw_parts_mut(buffer.pos as *mut Point, count),
                safer::from_raw_parts_mut(buffer.utf8text as *mut u8, text_byte_count),
                safer::from_raw_parts_mut(buffer.clusters, count),
            )
        }
    }

    pub fn alloc_run_text_rsxform(
        &mut self,
        font: &Font,
        count: usize,
        text_byte_count: usize,
        bounds: Option<&Rect>,
    ) -> (&mut [GlyphId], &mut [RSXform], &mut [u8], &mut [u32]) {
        unsafe {
            let buffer = &*self.native_mut().allocRunTextPos(
                font.native(),
                count.try_into().unwrap(),
                text_byte_count.try_into().unwrap(),
                bounds.native_ptr_or_null(),
            );
            (
                safer::from_raw_parts_mut(buffer.glyphs, count),
                safer::from_raw_parts_mut(buffer.pos as *mut RSXform, count),
                safer::from_raw_parts_mut(buffer.utf8text as *mut u8, text_byte_count),
                safer::from_raw_parts_mut(buffer.clusters, count),
            )
        }
    }
}

pub type TextBlobIter<'a> = Borrows<'a, Handle<SkTextBlob_Iter>>;

pub struct TextBlobRun<'a> {
    typeface: *mut SkTypeface,
    pub glyph_indices: &'a [GlyphId],
}

impl fmt::Debug for TextBlobRun<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextBlobRun")
            .field("typeface", self.typeface())
            .field("glyph_indices", &self.glyph_indices)
            .finish()
    }
}

impl TextBlobRun<'_> {
    pub fn typeface(&self) -> &Option<Typeface> {
        Typeface::from_unshared_ptr_ref(&self.typeface)
    }
}

impl<'a> Borrows<'a, Handle<SkTextBlob_Iter>> {
    pub fn new(text_blob: &'a TextBlob) -> Self {
        Handle::from_native_c(unsafe { SkTextBlob_Iter::new(text_blob.native()) })
            .borrows(text_blob)
    }
}

impl NativeDrop for SkTextBlob_Iter {
    fn drop(&mut self) {
        unsafe { sb::C_SkTextBlob_Iter_destruct(self) }
    }
}

impl<'a> Iterator for Borrows<'a, Handle<SkTextBlob_Iter>> {
    type Item = TextBlobRun<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let mut run = SkTextBlob_Iter_Run {
            fTypeface: ptr::null_mut(),
            fGlyphCount: 0,
            fGlyphIndices: ptr::null_mut(),
        };
        unsafe {
            if self.native_mut().next(&mut run) {
                let indices = if !run.fGlyphIndices.is_null() && run.fGlyphCount != 0 {
                    slice::from_raw_parts(run.fGlyphIndices, run.fGlyphCount.try_into().unwrap())
                } else {
                    &[]
                };

                Some(TextBlobRun {
                    typeface: run.fTypeface,
                    glyph_indices: indices,
                })
            } else {
                None
            }
        }
    }
}

#[test]
fn test_point_size_and_alignment_equals_size_of_two_scalars_used_in_alloc_run_pos() {
    use std::mem;
    assert_eq!(mem::size_of::<Point>(), mem::size_of::<[scalar; 2]>());
    assert_eq!(mem::align_of::<Point>(), mem::align_of::<[scalar; 2]>());
}
