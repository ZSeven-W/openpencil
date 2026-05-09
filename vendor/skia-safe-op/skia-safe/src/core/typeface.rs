use std::{fmt, io, ptr};

use skia_bindings::{self as sb, SkRefCntBase, SkTypeface, SkTypeface_LocalizedStrings};

use crate::font_arguments;
use crate::font_parameters::VariationAxis;
use crate::interop::{self, NativeStreamBase, RustStream, RustWStream, StreamAsset};
use crate::prelude::*;
use crate::{
    Data, EncodedText, FontArguments, FontMgr, FontStyle, FourByteTag, GlyphId, Rect, Unichar,
};

pub type TypefaceId = skia_bindings::SkTypefaceID;
pub type FontTableTag = skia_bindings::SkFontTableTag;

pub use skia_bindings::SkTypeface_SerializeBehavior as SerializeBehavior;
variant_name!(SerializeBehavior::DontIncludeData);

#[derive(Clone, PartialEq, Eq, Debug)]
/// Localized family-name entry returned by [`LocalizedStringsIter`].
pub struct LocalizedString {
    pub string: String,
    pub language: String,
}

pub type FactoryId = FourByteTag;

pub type Typeface = RCHandle<SkTypeface>;
unsafe_send_sync!(Typeface);
require_base_type!(SkTypeface, sb::SkWeakRefCnt);

impl NativeRefCountedBase for SkTypeface {
    type Base = SkRefCntBase;
}

impl fmt::Debug for Typeface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Typeface")
            .field("font_style", &self.font_style())
            .field("is_fixed_pitch", &self.is_fixed_pitch())
            .field("unique_id", &self.unique_id())
            .field("bounds", &self.bounds())
            .finish()
    }
}

impl Typeface {
    /// Returns the typeface's intrinsic style attributes.
    pub fn font_style(&self) -> FontStyle {
        FontStyle::construct(|fs| unsafe { sb::C_SkTypeface_fontStyle(self.native(), fs) })
    }

    /// Returns `true` if [`Self::font_style()`] weight is at least semi-bold.
    pub fn is_bold(&self) -> bool {
        unsafe { self.native().isBold() }
    }

    /// Returns `true` if [`Self::font_style()`] is not upright.
    pub fn is_italic(&self) -> bool {
        unsafe { self.native().isItalic() }
    }

    /// Returns `true` if the typeface claims to be fixed-pitch.
    ///
    /// This is a style bit; advance widths may vary even when this returns `true`.
    pub fn is_fixed_pitch(&self) -> bool {
        unsafe { self.native().isFixedPitch() }
    }

    /// Returns the design variation coordinates for this typeface.
    ///
    /// Returns the number of axes and copies the coordinates if available.
    /// Returns `None` on error.
    pub fn variation_design_position(
        &self,
    ) -> Option<Vec<font_arguments::variation_position::Coordinate>> {
        unsafe {
            let r = sb::C_SkTypeface_getVariationDesignPosition(self.native(), ptr::null_mut(), 0);
            if r != -1 {
                let mut v = vec![font_arguments::variation_position::Coordinate::default(); r as _];
                let elements = sb::C_SkTypeface_getVariationDesignPosition(
                    self.native(),
                    v.native_mut().as_mut_ptr(),
                    r as _,
                );
                assert_eq!(elements, r);
                Some(v)
            } else {
                None
            }
        }
    }

    /// Returns the design variation parameters for this typeface.
    ///
    /// Returns the number of axes and copies the parameters if available.
    /// Returns `None` on error.
    pub fn variation_design_parameters(&self) -> Option<Vec<VariationAxis>> {
        unsafe {
            let r =
                sb::C_SkTypeface_getVariationDesignParameters(self.native(), ptr::null_mut(), 0);
            if r != -1 {
                let mut v = vec![VariationAxis::default(); r as _];
                let elements = sb::C_SkTypeface_getVariationDesignParameters(
                    self.native(),
                    v.native_mut().as_mut_ptr(),
                    r as _,
                );
                assert_eq!(elements, r);
                Some(v)
            } else {
                None
            }
        }
    }

    /// Returns `true` if the typeface is internally being fake-bolded.
    pub fn is_synthetic_bold(&self) -> bool {
        unsafe { self.native().isSyntheticBold() }
    }

    /// Returns `true` if the typeface is internally being fake-obliqued.
    pub fn is_synthetic_oblique(&self) -> bool {
        unsafe { self.native().isSyntheticOblique() }
    }

    pub fn unique_id(&self) -> TypefaceId {
        self.native().fUniqueID
    }

    /// Returns `true` if the two typefaces reference the same underlying font.
    ///
    /// `None` does not compare equal to any typeface.
    ///
    /// - `face_a`: first typeface.
    /// - `face_b`: second typeface.
    // Decided not to support PartialEq instead of this function,
    // because Skia does not support the operator ==.
    pub fn equal(face_a: impl AsRef<Typeface>, face_b: impl AsRef<Typeface>) -> bool {
        unsafe { SkTypeface::Equal(face_a.as_ref().native(), face_b.as_ref().native()) }
    }

    /// Returns a typeface based on this typeface and parameterized by `arguments`.
    ///
    /// If `arguments` does not supply a value for some font parameter, the value from this
    /// typeface is used.
    ///
    /// - `arguments`: clone parameters to apply.
    pub fn clone_with_arguments(&self, arguments: &FontArguments) -> Option<Typeface> {
        Typeface::from_ptr(unsafe { sb::C_SkTypeface_makeClone(self.native(), arguments.native()) })
    }

    /// Writes a signature sufficient to reconstruct a typeface referencing the same font.
    ///
    /// - `write`: destination stream.
    /// - `behavior`: controls whether font data is included.
    pub fn serialize_stream(&self, mut write: impl io::Write, behavior: SerializeBehavior) {
        let mut stream = RustWStream::new(&mut write);
        unsafe { sb::C_SkTypeface_serialize2(self.native(), stream.stream_mut(), behavior) }
    }

    /// Returns serialized typeface data.
    ///
    /// - `behavior`: controls whether font data is included.
    // TODO: return Data as impl Deref<[u8]> / Borrow<[u8]> here?
    pub fn serialize(&self, behavior: SerializeBehavior) -> Data {
        Data::from_ptr(unsafe { sb::C_SkTypeface_serialize(self.native(), behavior) }).unwrap()
    }

    // TODO: Wrap Deserialize(Read?)
    /// Deserializes a typeface previously produced by [`Self::serialize()`] or
    /// [`Self::serialize_stream()`].
    ///
    /// Goes through all registered typeface factories and `last_resort_mgr` when provided.
    ///
    /// - `data`: serialized typeface bytes.
    /// - `last_resort_mgr`: optional fallback font manager.
    pub fn make_deserialize(
        mut data: impl io::Read,
        last_resort_mgr: impl Into<Option<FontMgr>>,
    ) -> Option<Typeface> {
        let mut stream = RustStream::new(&mut data);
        Typeface::from_ptr(unsafe {
            sb::C_SkTypeface_MakeDeserialize(
                stream.stream_mut(),
                last_resort_mgr.into().into_ptr_or_null(),
            )
        })
    }

    /// Converts UTF-32 code points to glyph IDs.
    ///
    /// - `uni`: UTF-32 code points.
    /// - `glyphs`: output glyph IDs.
    pub fn unichars_to_glyphs(&self, uni: &[Unichar], glyphs: &mut [GlyphId]) {
        unsafe {
            sb::C_SkTypeface_unicharsToGlyphs(
                self.native(),
                uni.as_ptr(),
                uni.len(),
                glyphs.as_mut_ptr(),
                glyphs.len(),
            )
        }
    }

    /// Converts UTF-8 text to glyph IDs.
    ///
    /// - `str`: input text.
    /// - `glyphs`: output glyph IDs.
    pub fn str_to_glyphs(&self, str: impl AsRef<str>, glyphs: &mut [GlyphId]) -> usize {
        self.text_to_glyphs(str.as_ref(), glyphs)
    }

    /// Converts encoded text to glyph IDs.
    ///
    /// - `text`: encoded text input.
    /// - `glyphs`: output glyph IDs.
    pub fn text_to_glyphs(&self, text: impl EncodedText, glyphs: &mut [GlyphId]) -> usize {
        let (ptr, size, encoding) = text.as_raw();
        unsafe {
            sb::C_SkTypeface_textToGlyphs(
                self.native(),
                ptr,
                size,
                encoding.into_native(),
                glyphs.as_mut_ptr(),
                glyphs.len(),
            )
        }
    }

    /// Returns the glyph ID corresponding to a Unicode code point.
    ///
    /// Returns `0` when the code point is not supported.
    ///
    /// - `unichar`: Unicode code point.
    pub fn unichar_to_glyph(&self, unichar: Unichar) -> GlyphId {
        unsafe { self.native().unicharToGlyph(unichar) }
    }

    /// Returns the number of glyphs in the typeface.
    pub fn count_glyphs(&self) -> usize {
        unsafe { self.native().countGlyphs().try_into().unwrap() }
    }

    /// Returns the number of tables in the font.
    pub fn count_tables(&self) -> usize {
        unsafe { self.native().countTables().try_into().unwrap() }
    }

    #[deprecated(since = "0.88.0", note = "use read_table_tags")]
    pub fn table_tags(&self) -> Option<Vec<FontTableTag>> {
        self.read_table_tags()
    }

    /// Returns the list of table tags in the font.
    ///
    /// Returns `None` on error.
    pub fn read_table_tags(&self) -> Option<Vec<FontTableTag>> {
        let mut v: Vec<FontTableTag> = vec![0; self.count_tables()];
        (unsafe { sb::C_SkTypeface_readTableTags(self.native(), v.as_mut_ptr(), v.len()) } != 0)
            .then_some(v)
    }

    /// Returns the size, in bytes, of table `tag`.
    ///
    /// Returns `None` if the table is not present.
    ///
    /// - `tag`: table tag.
    pub fn get_table_size(&self, tag: FontTableTag) -> Option<usize> {
        let size = unsafe { self.native().getTableSize(tag) };
        if size != 0 {
            Some(size)
        } else {
            None
        }
    }

    /// Copies table `tag` data into `data`.
    ///
    /// Returns the number of bytes copied.
    ///
    /// - `tag`: table tag.
    /// - `data`: destination buffer.
    pub fn get_table_data(&self, tag: FontTableTag, data: &mut [u8]) -> usize {
        unsafe {
            self.native()
                .getTableData(tag, 0, data.len(), data.as_mut_ptr() as _)
        }
    }

    /// Returns an immutable copy of table `tag` data.
    ///
    /// Returns `None` if the table is not found.
    ///
    /// - `tag`: table tag.
    pub fn copy_table_data(&self, tag: FontTableTag) -> Option<Data> {
        Data::from_ptr(unsafe { sb::C_SkTypeface_copyTableData(self.native(), tag) })
    }

    /// Returns the units-per-em value for this typeface.
    ///
    /// Returns `None` on error.
    pub fn units_per_em(&self) -> Option<i32> {
        let units = unsafe { self.native().getUnitsPerEm() };
        if units != 0 {
            Some(units)
        } else {
            None
        }
    }

    /// Returns horizontal kerning adjustments for `glyphs`.
    ///
    /// Adjustments are in design units relative to units-per-em.
    ///
    /// - `glyphs`: input glyph run.
    /// - `adjustments`: output adjustments; length should be `glyphs.len() - 1`.
    // note: adjustments slice length must be equal to glyph's len - 1.
    pub fn get_kerning_pair_adjustments(
        &self,
        glyphs: &[GlyphId],
        adjustments: &mut [i32],
    ) -> bool {
        unsafe {
            sb::C_SkTypeface_getKerningPairAdjustments(
                self.native(),
                glyphs.as_ptr(),
                glyphs.len(),
                adjustments.as_mut_ptr(),
                adjustments.len(),
            )
        }
    }

    /// Returns an iterator over all family names specified by the font.
    pub fn new_family_name_iterator(&self) -> impl Iterator<Item = LocalizedString> {
        LocalizedStringsIter::from_ptr(unsafe { self.native().createFamilyNameIterator() }).unwrap()
    }

    /// Returns the family name for this typeface as UTF-8.
    pub fn family_name(&self) -> String {
        let mut name = interop::String::default();
        unsafe {
            self.native().getFamilyName(name.native_mut());
        };
        name.as_str().into()
    }

    /// Returns the PostScript name for this typeface.
    ///
    /// The value may vary with variation parameters.
    pub fn post_script_name(&self) -> Option<String> {
        let mut name = interop::String::default();
        unsafe { self.native().getPostScriptName(name.native_mut()) }.then(|| name.as_str().into())
    }

    /// Returns a user-facing name for the primary resource backing this typeface.
    ///
    /// Returns `None` when no resource name is available.
    pub fn resource_name(&self) -> Option<String> {
        let mut name = interop::String::default();
        let num_resources = unsafe { self.native().getResourceName(name.native_mut()) };
        if num_resources == 0 {
            return None;
        }
        Some(name.as_str().into())
    }

    /// Returns raw font data bytes and the TTC index.
    ///
    /// Returns `None` on failure.
    pub fn to_font_data(&self) -> Option<(Vec<u8>, usize)> {
        let mut ttc_index = 0;
        StreamAsset::from_ptr(unsafe { sb::C_SkTypeface_openStream(self.native(), &mut ttc_index) })
            .and_then(|mut stream| {
                let length = unsafe { sb::C_SkStreamAsset_getLength(stream.native()) };
                let mut data = vec![0u8; length];
                let stream = stream.native_mut().as_stream_mut();
                let read =
                    unsafe { sb::C_SkStream_read(stream, data.as_mut_ptr() as _, data.len()) };
                (read == data.len()).then_some((data, ttc_index.try_into().unwrap()))
            })
    }

    // TODO: openExistingStream()

    // TODO: createScalerContext()

    /// Returns the union of glyph bounds, scaled to 1pt.
    pub fn bounds(&self) -> Rect {
        Rect::construct(|r| unsafe { sb::C_SkTypeface_getBounds(self.native(), r) })
    }

    // TODO: Register()
}

pub type LocalizedStringsIter = RefHandle<SkTypeface_LocalizedStrings>;

impl NativeDrop for SkTypeface_LocalizedStrings {
    fn drop(&mut self) {
        unsafe { sb::C_SkTypeface_LocalizedStrings_unref(self) }
    }
}

impl fmt::Debug for LocalizedStringsIter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalizedStringsIter").finish()
    }
}

impl Iterator for LocalizedStringsIter {
    type Item = LocalizedString;

    /// Returns the next localized family-name entry.
    fn next(&mut self) -> Option<Self::Item> {
        let mut string = interop::String::default();
        let mut language = interop::String::default();
        unsafe {
            sb::C_SkTypeface_LocalizedStrings_next(
                self.native_mut(),
                string.native_mut(),
                language.native_mut(),
            )
        }
        .then(|| LocalizedString {
            string: string.as_str().into(),
            language: language.as_str().into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{FontMgr, FontStyle};

    use super::{SerializeBehavior, Typeface};

    #[test]
    fn serialize_and_deserialize_default_typeface() {
        let tf = FontMgr::new()
            .legacy_make_typeface(None, FontStyle::normal())
            .unwrap();

        let serialized = tf.serialize(SerializeBehavior::DoIncludeData);
        // On Android, the deserialized typeface name changes from sans-serif to Roboto.
        // (which is probably OK, because Roboto _is_ the default font, so we do another
        // serialization / deserialization and compare the family name with already deserialized
        // one.)
        let deserialized =
            Typeface::make_deserialize(Cursor::new(serialized.as_bytes()), None).unwrap();
        let serialized2 = deserialized.serialize(SerializeBehavior::DoIncludeData);
        let deserialized2 =
            Typeface::make_deserialize(Cursor::new(serialized2.as_bytes()), None).unwrap();

        // why aren't they not equal?
        assert!(!Typeface::equal(&tf, &deserialized));
        assert_eq!(deserialized.family_name(), deserialized2.family_name());
    }

    #[test]
    fn family_name_iterator_owns_the_strings_and_returns_at_least_one_name_for_the_default_typeface(
    ) {
        let fm = FontMgr::default();
        let tf = fm.legacy_make_typeface(None, FontStyle::normal()).unwrap();
        let family_names = tf.new_family_name_iterator();
        drop(tf);

        let mut any = false;
        for name in family_names {
            println!("family: {}, language: {}", name.string, name.language);
            any = true
        }
        assert!(any);
    }

    #[test]
    fn get_font_data_of_default() {
        let tf = FontMgr::new()
            .legacy_make_typeface(None, FontStyle::normal())
            .unwrap();
        let (data, _ttc_index) = tf.to_font_data().unwrap();
        assert!(!data.is_empty());
    }
}
