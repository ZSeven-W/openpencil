use skia_bindings::{self as sb, SkFontMgr, SkFontStyleSet, SkRefCntBase};
use std::{ffi::CString, fmt, mem, os::raw::c_char, ptr};

use crate::{
    interop::{self, DynamicMemoryWStream},
    prelude::*,
    FontStyle, Typeface, Unichar,
};

pub type FontStyleSet = RCHandle<SkFontStyleSet>;

impl NativeBase<SkRefCntBase> for SkFontStyleSet {}

impl NativeRefCountedBase for SkFontStyleSet {
    type Base = SkRefCntBase;
}

impl Default for FontStyleSet {
    fn default() -> Self {
        FontStyleSet::new_empty()
    }
}

impl fmt::Debug for FontStyleSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontStyleSet")
            // TODO: clarify why self has to be mut.
            // .field("count", &self.count())
            .finish()
    }
}

impl FontStyleSet {
    pub fn count(&mut self) -> usize {
        unsafe {
            sb::C_SkFontStyleSet_count(self.native_mut())
                .try_into()
                .unwrap()
        }
    }

    pub fn style(&mut self, index: usize) -> (FontStyle, Option<String>) {
        assert!(index < self.count());

        let mut font_style = FontStyle::default();
        let mut style = interop::String::default();
        unsafe {
            sb::C_SkFontStyleSet_getStyle(
                self.native_mut(),
                index.try_into().unwrap(),
                font_style.native_mut(),
                style.native_mut(),
            )
        }

        let style = style.as_str();
        // Note: Android's FontMgr returns empty style names.
        let name = (!style.is_empty()).then(|| style.into());

        (font_style, name)
    }

    pub fn new_typeface(&mut self, index: usize) -> Option<Typeface> {
        assert!(index < self.count());

        Typeface::from_ptr(unsafe {
            sb::C_SkFontStyleSet_createTypeface(self.native_mut(), index.try_into().unwrap())
        })
    }

    pub fn match_style(&mut self, pattern: FontStyle) -> Option<Typeface> {
        Typeface::from_ptr(unsafe {
            sb::C_SkFontStyleSet_matchStyle(self.native_mut(), pattern.native())
        })
    }

    pub fn new_empty() -> Self {
        FontStyleSet::from_ptr(unsafe { sb::C_SkFontStyleSet_CreateEmpty() }).unwrap()
    }
}

pub type FontMgr = RCHandle<SkFontMgr>;

impl NativeBase<SkRefCntBase> for SkFontMgr {}

impl NativeRefCountedBase for SkFontMgr {
    type Base = SkRefCntBase;
}

impl Default for FontMgr {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for FontMgr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<_> = self.family_names().collect();
        f.debug_struct("FontMgr")
            .field("family_names", &names)
            .finish()
    }
}

impl FontMgr {
    // Deprecated by Skia, but we continue to support it. This returns a font manager with
    // system fonts for the current platform.
    pub fn new() -> Self {
        FontMgr::from_ptr(unsafe { sb::C_SkFontMgr_NewSystem() }).unwrap()
    }

    pub fn empty() -> Self {
        FontMgr::from_ptr(unsafe { sb::C_SkFontMgr_RefEmpty() }).unwrap()
    }

    // Custom empty manager. This avoids scanning system fonts when they are not required.
    //
    // Returns `None` on platforms where Skia is not compiled with freetype (e.g. Windows)
    pub fn custom_empty() -> Option<Self> {
        FontMgr::from_ptr(unsafe { sb::C_SkFontMgr_NewCustomEmpty() })
    }

    pub fn count_families(&self) -> usize {
        unsafe { self.native().countFamilies().try_into().unwrap() }
    }

    pub fn family_name(&self, index: usize) -> String {
        assert!(index < self.count_families());
        let mut family_name = interop::String::default();
        unsafe {
            self.native()
                .getFamilyName(index.try_into().unwrap(), family_name.native_mut())
        }
        family_name.as_str().into()
    }

    pub fn family_names(&self) -> impl Iterator<Item = String> + use<'_> {
        (0..self.count_families()).map(move |i| self.family_name(i))
    }

    #[deprecated(since = "0.41.0", note = "Use new_style_set")]
    pub fn new_styleset(&self, index: usize) -> FontStyleSet {
        self.new_style_set(index)
    }

    pub fn new_style_set(&self, index: usize) -> FontStyleSet {
        assert!(index < self.count_families());
        FontStyleSet::from_ptr(unsafe {
            sb::C_SkFontMgr_createStyleSet(self.native(), index.try_into().unwrap())
        })
        .unwrap()
    }

    pub fn match_family(&self, family_name: impl AsRef<str>) -> FontStyleSet {
        let family_name = CString::new(family_name.as_ref()).unwrap();
        FontStyleSet::from_ptr(unsafe {
            sb::C_SkFontMgr_matchFamily(self.native(), family_name.as_ptr())
        })
        .unwrap()
    }

    pub fn match_family_style(
        &self,
        family_name: impl AsRef<str>,
        style: FontStyle,
    ) -> Option<Typeface> {
        let family_name = CString::new(family_name.as_ref()).unwrap();
        Typeface::from_ptr(unsafe {
            sb::C_SkFontMgr_matchFamilyStyle(self.native(), family_name.as_ptr(), style.native())
        })
    }

    // TODO: support IntoIterator / AsRef<str> for bcp_47?
    pub fn match_family_style_character(
        &self,
        family_name: impl AsRef<str>,
        style: FontStyle,
        bcp_47: &[&str],
        character: Unichar,
    ) -> Option<Typeface> {
        let family_name = CString::new(family_name.as_ref()).unwrap();
        // create backing store for the pointer array.
        let bcp_47: Vec<CString> = bcp_47.iter().map(|s| CString::new(*s).unwrap()).collect();
        // note: mutability needed to comply to the C type "const char* bcp47[]".
        let mut bcp_47: Vec<*const c_char> = bcp_47.iter().map(|cs| cs.as_ptr()).collect();

        Typeface::from_ptr(unsafe {
            sb::C_SkFontMgr_matchFamilyStyleCharacter(
                self.native(),
                family_name.as_ptr(),
                style.native(),
                bcp_47.as_mut_ptr(),
                bcp_47.len().try_into().unwrap(),
                character,
            )
        })
    }

    #[deprecated(since = "0.35.0", note = "Removed without replacement")]
    pub fn match_face_style(&self, _typeface: impl AsRef<Typeface>, _style: FontStyle) -> ! {
        panic!("Removed without replacement")
    }

    // pub fn new_from_data(
    //     &self,
    //     bytes: &[u8],
    //     ttc_index: impl Into<Option<usize>>,
    // ) -> Option<Typeface> {
    //     let data: Data = Data::new_copy(bytes);
    //     Typeface::from_ptr(unsafe {
    //         sb::C_SkFontMgr_makeFromData(
    //             self.native(),
    //             data.into_ptr(),
    //             ttc_index.into().unwrap_or_default().try_into().unwrap(),
    //         )
    //     })
    // }

    pub fn new_from_data(
        &self,
        bytes: &[u8],
        ttc_index: impl Into<Option<usize>>,
    ) -> Option<Typeface> {
        let mut stream = DynamicMemoryWStream::from_bytes(bytes);
        let mut stream = stream.detach_as_stream();
        Typeface::from_ptr(unsafe {
            let stream_ptr = stream.native_mut() as *mut _;
            // makeFromStream takes ownership of the stream, so don't drop it.
            mem::forget(stream);
            sb::C_SkFontMgr_makeFromStream(
                self.native(),
                stream_ptr,
                ttc_index.into().unwrap_or_default().try_into().unwrap(),
            )
        })
    }

    pub fn legacy_make_typeface<'a>(
        &self,
        family_name: impl Into<Option<&'a str>>,
        style: FontStyle,
    ) -> Option<Typeface> {
        let family_name: Option<CString> = family_name
            .into()
            .and_then(|family_name| CString::new(family_name).ok());

        Typeface::from_ptr(unsafe {
            sb::C_SkFontMgr_legacyMakeTypeface(
                self.native(),
                family_name
                    .as_ref()
                    .map(|n| n.as_ptr())
                    .unwrap_or(ptr::null()),
                style.into_native(),
            )
        })
    }

    // TODO: makeFromStream(.., ttcIndex).
}

#[cfg(test)]
mod tests {
    use crate::FontMgr;

    #[test]
    #[serial_test::serial]
    fn create_all_typefaces() {
        let font_mgr = FontMgr::default();
        let families = font_mgr.count_families();
        println!("FontMgr families: {families}");
        // This test requires that the default system font manager returns at least one family for now.
        assert!(families > 0);
        // print all family names and styles
        for i in 0..families {
            let name = font_mgr.family_name(i);
            println!("font_family: {name}");
            let mut style_set = font_mgr.new_style_set(i);
            for style_index in 0..style_set.count() {
                let (_, style_name) = style_set.style(style_index);
                if let Some(style_name) = style_name {
                    println!("  style: {style_name}");
                }
                let face = style_set.new_typeface(style_index);
                drop(face);
            }
        }
    }
}
