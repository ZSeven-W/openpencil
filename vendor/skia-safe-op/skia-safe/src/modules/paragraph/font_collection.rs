use crate::{
    interop::{self, FromStrs, VecSink},
    prelude::*,
    textlayout::ParagraphCache,
    FontMgr, FontStyle, Typeface, Unichar,
};
use skia_bindings::{self as sb, skia_textlayout_FontCollection};
use std::{ffi, fmt, ptr};

use super::FontArguments;

pub type FontCollection = RCHandle<skia_textlayout_FontCollection>;

impl NativeRefCountedBase for skia_textlayout_FontCollection {
    type Base = sb::SkRefCntBase;
}

impl fmt::Debug for FontCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontCollection")
            .field("font_managers_count", &self.font_managers_count())
            .field("fallback_manager", &self.fallback_manager())
            .field("font_fallback_enabled", &self.font_fallback_enabled())
            .field("paragraph_cache", &self.paragraph_cache())
            .finish()
    }
}

impl FontCollection {
    pub fn new() -> Self {
        Self::from_ptr(unsafe { sb::C_FontCollection_new() }).unwrap()
    }

    pub fn font_managers_count(&self) -> usize {
        unsafe { self.native().getFontManagersCount() }
    }

    pub fn set_asset_font_manager(&mut self, font_manager: impl Into<Option<FontMgr>>) {
        unsafe {
            sb::C_FontCollection_setAssetFontManager(
                self.native_mut(),
                font_manager.into().into_ptr_or_null(),
            )
        }
    }

    pub fn set_dynamic_font_manager(&mut self, font_manager: impl Into<Option<FontMgr>>) {
        unsafe {
            sb::C_FontCollection_setDynamicFontManager(
                self.native_mut(),
                font_manager.into().into_ptr_or_null(),
            )
        }
    }

    pub fn set_test_font_manager(&mut self, font_manager: impl Into<Option<FontMgr>>) {
        unsafe {
            sb::C_FontCollection_setTestFontManager(
                self.native_mut(),
                font_manager.into().into_ptr_or_null(),
            )
        }
    }

    pub fn set_default_font_manager<'a>(
        &mut self,
        font_manager: impl Into<Option<FontMgr>>,
        default_family_name: impl Into<Option<&'a str>>,
    ) {
        let font_manager = font_manager.into();
        unsafe {
            match default_family_name.into() {
                Some(name) => {
                    let name = ffi::CString::new(name).unwrap();
                    sb::C_FontCollection_setDefaultFontManager2(
                        self.native_mut(),
                        font_manager.into_ptr_or_null(),
                        name.as_ptr(),
                    )
                }
                None => sb::C_FontCollection_setDefaultFontManager(
                    self.native_mut(),
                    font_manager.into_ptr_or_null(),
                ),
            }
        }
    }

    pub fn set_default_font_manager_and_family_names(
        &mut self,
        font_manager: impl Into<Option<FontMgr>>,
        family_names: &[impl AsRef<str>],
    ) {
        let font_manager = font_manager.into();
        let family_names = interop::Strings::from_strs(family_names);
        unsafe {
            sb::C_FontCollection_setDefaultFontManager3(
                self.native_mut(),
                font_manager.into_ptr_or_null(),
                family_names.native(),
            )
        }
    }

    pub fn fallback_manager(&self) -> Option<FontMgr> {
        FontMgr::from_ptr(unsafe { sb::C_FontCollection_getFallbackManager(self.native()) })
    }

    pub fn find_typefaces(
        &mut self,
        family_names: &[impl AsRef<str>],
        font_style: FontStyle,
    ) -> Vec<Typeface> {
        self.find_typefaces_with_font_arguments(family_names, font_style, None)
    }

    pub fn find_typefaces_with_font_arguments<'fa>(
        &mut self,
        family_names: &[impl AsRef<str>],
        font_style: FontStyle,
        font_args: impl Into<Option<&'fa FontArguments>>,
    ) -> Vec<Typeface> {
        let family_names = interop::Strings::from_strs(family_names);

        let mut typefaces: Vec<Typeface> = Vec::new();
        let mut set_typefaces = |tfs: &mut [sb::sk_sp<sb::SkTypeface>]| {
            typefaces = tfs
                .iter_mut()
                .filter_map(|sp| {
                    let ptr = sp.fPtr;
                    sp.fPtr = ptr::null_mut();
                    Typeface::from_ptr(ptr)
                })
                .collect()
        };

        unsafe {
            sb::C_FontCollection_findTypefaces(
                self.native_mut(),
                family_names.native(),
                font_style.into_native(),
                font_args.into().native_ptr_or_null(),
                VecSink::new_mut(&mut set_typefaces).native_mut(),
            )
        };
        typefaces
    }

    pub fn default_fallback_char<'fa>(
        &mut self,
        unicode: Unichar,
        font_style: FontStyle,
        locale: impl AsRef<str>,
        font_args: impl Into<Option<&'fa FontArguments>>,
    ) -> Option<Typeface> {
        self.default_fallback_char_and_families(
            unicode,
            &[] as &[&str],
            font_style,
            locale,
            font_args,
        )
    }

    pub fn default_fallback_char_and_families<'fa>(
        &mut self,
        unicode: Unichar,
        family_names: &[impl AsRef<str>],
        font_style: FontStyle,
        locale: impl AsRef<str>,
        font_args: impl Into<Option<&'fa FontArguments>>,
    ) -> Option<Typeface> {
        let family_names = interop::Strings::from_strs(family_names);
        let locale = interop::String::from_str(locale.as_ref());
        Typeface::from_ptr(unsafe {
            sb::C_FontCollection_defaultFallback(
                self.native_mut(),
                unicode,
                family_names.native(),
                font_style.into_native(),
                locale.native(),
                font_args.into().native_ptr_or_null(),
            )
        })
    }

    pub fn default_fallback(&mut self) -> Option<Typeface> {
        Typeface::from_ptr(unsafe { sb::C_FontCollection_defaultFallback2(self.native_mut()) })
    }

    pub fn default_emoji_fallback(
        &mut self,
        emoji_start: Unichar,
        font_style: FontStyle,
        locale: impl AsRef<str>,
    ) -> Option<Typeface> {
        let locale = interop::String::from_str(locale.as_ref());
        Typeface::from_ptr(unsafe {
            sb::C_FontCollection_defaultEmojiFallback(
                self.native_mut(),
                emoji_start,
                font_style.into_native(),
                locale.native(),
            )
        })
    }

    pub fn disable_font_fallback(&mut self) {
        unsafe { self.native_mut().disableFontFallback() }
    }

    pub fn enable_font_fallback(&mut self) {
        unsafe { self.native_mut().enableFontFallback() }
    }

    pub fn font_fallback_enabled(&self) -> bool {
        unsafe { sb::C_FontCollection_fontFallbackEnabled(self.native()) }
    }

    pub fn paragraph_cache(&self) -> &ParagraphCache {
        ParagraphCache::from_native_ref(unsafe {
            &*sb::C_FontCollection_paragraphCache(self.native_mut_force())
        })
    }

    pub fn paragraph_cache_mut(&mut self) -> &mut ParagraphCache {
        ParagraphCache::from_native_ref_mut(unsafe {
            &mut *sb::C_FontCollection_paragraphCache(self.native_mut())
        })
    }

    pub fn clear_caches(&mut self) {
        unsafe { self.native_mut().clearCaches() }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::textlayout::FontCollection;
    use crate::{FontMgr, FontStyle};

    #[test]
    #[serial_test::serial]
    fn ref_counts() {
        let mut fc = FontCollection::new();
        assert_eq!(fc.native().ref_counted_base()._ref_cnt(), 1);

        let fm = FontMgr::new();
        let fm_base = fm.native().ref_counted_base()._ref_cnt();

        fc.set_default_font_manager(fm.clone(), None);
        assert_eq!(fm.native().ref_counted_base()._ref_cnt(), fm_base + 1);

        let cloned_fc = fc.clone();
        assert_eq!(fm.native().ref_counted_base()._ref_cnt(), fm_base + 1);
        assert_eq!(fc.native().ref_counted_base()._ref_cnt(), 2);
        drop(cloned_fc);
        assert_eq!(fc.native().ref_counted_base()._ref_cnt(), 1);
        assert_eq!(fm.native().ref_counted_base()._ref_cnt(), fm_base + 1);

        {
            let fmc = fc.fallback_manager().unwrap();
            assert_eq!(fmc.native().ref_counted_base()._ref_cnt(), fm_base + 2);
            drop(fmc);
        }

        fc.set_default_font_manager(None, None);
        assert_eq!(fm.native().ref_counted_base()._ref_cnt(), fm_base);
        drop(fm);
        drop(fc);
    }

    #[test]
    #[serial_test::serial]
    fn find_typefaces() {
        let mut fc = FontCollection::new();
        fc.set_default_font_manager(FontMgr::new(), None);
        println!("find typeface:");
        for typeface in fc.find_typefaces(
            &[
                "Arial",
                "Tahoma",
                "Fira Code",
                "JetBrains Mono",
                "Not Existing",
            ],
            FontStyle::default(),
        ) {
            println!("typeface: {}", typeface.family_name());
        }
    }
}
