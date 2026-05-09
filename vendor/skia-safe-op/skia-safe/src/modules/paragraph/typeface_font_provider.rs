use crate::{
    interop::{self, AsStr},
    prelude::*,
    FontMgr, FontStyleSet, Typeface,
};
use skia_bindings as sb;
use std::{
    fmt,
    mem::transmute,
    ops::{Deref, DerefMut},
    ptr,
};

pub type TypefaceFontStyleSet = RCHandle<sb::skia_textlayout_TypefaceFontStyleSet>;
require_base_type!(sb::skia_textlayout_TypefaceFontStyleSet, sb::SkFontStyleSet);

impl NativeRefCountedBase for sb::skia_textlayout_TypefaceFontStyleSet {
    type Base = sb::SkRefCntBase;
}

impl Deref for TypefaceFontStyleSet {
    type Target = FontStyleSet;
    fn deref(&self) -> &Self::Target {
        unsafe { transmute_ref(self) }
    }
}

impl DerefMut for TypefaceFontStyleSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_ref_mut(self) }
    }
}

impl fmt::Debug for TypefaceFontStyleSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypefaceFontStyleSet")
            .field("base", self as &FontStyleSet)
            .field("family_name", &self.family_name())
            .field("alias", &self.alias())
            .finish()
    }
}

impl TypefaceFontStyleSet {
    pub fn new(family_name: impl AsRef<str>) -> Self {
        let family = interop::String::from_str(family_name.as_ref());
        Self::from_ptr(unsafe { sb::C_TypefaceFontStyleSet_new(family.native()) }).unwrap()
    }

    pub fn family_name(&self) -> &str {
        self.native().fFamilyName.as_str()
    }

    pub fn alias(&self) -> &str {
        self.native().fAlias.as_str()
    }

    pub fn append_typeface(&mut self, typeface: Typeface) -> &mut Self {
        unsafe { sb::C_TypefaceFontStyleSet_appendTypeface(self.native_mut(), typeface.into_ptr()) }
        self
    }
}

pub type TypefaceFontProvider = RCHandle<sb::skia_textlayout_TypefaceFontProvider>;
require_base_type!(sb::skia_textlayout_TypefaceFontProvider, sb::SkFontMgr);

impl NativeRefCountedBase for sb::skia_textlayout_TypefaceFontProvider {
    type Base = sb::SkRefCntBase;
}

impl Deref for TypefaceFontProvider {
    type Target = FontMgr;
    fn deref(&self) -> &Self::Target {
        unsafe { transmute_ref(self) }
    }
}

impl DerefMut for TypefaceFontProvider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { transmute_ref_mut(self) }
    }
}

impl Default for TypefaceFontProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl From<TypefaceFontProvider> for FontMgr {
    fn from(provider: TypefaceFontProvider) -> Self {
        unsafe { transmute(provider) }
    }
}

impl fmt::Debug for TypefaceFontProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypefaceFontProvider")
            .field("base", self as &FontMgr)
            .finish()
    }
}

impl TypefaceFontProvider {
    pub fn new() -> Self {
        Self::from_ptr(unsafe { sb::C_TypefaceFontProvider_new() }).unwrap()
    }

    pub fn register_typeface<'a>(
        &mut self,
        typeface: Typeface,
        alias: impl Into<Option<&'a str>>,
    ) -> usize {
        unsafe {
            match alias.into() {
                Some(alias) => {
                    let alias = interop::String::from_str(alias);
                    sb::C_TypefaceFontProvider_registerTypeface(
                        self.native_mut(),
                        typeface.into_ptr(),
                        alias.native(),
                    )
                }
                None => sb::C_TypefaceFontProvider_registerTypeface(
                    self.native_mut(),
                    typeface.into_ptr(),
                    ptr::null(),
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TypefaceFontProvider, TypefaceFontStyleSet};
    use crate::{
        prelude::{NativeAccess, NativeRefCounted, NativeRefCountedBase},
        textlayout::FontCollection,
        FontMgr, FontStyle,
    };

    #[test]
    #[serial_test::serial]
    fn font_style_set_typeface_ref_counts() {
        let mut style_set = TypefaceFontStyleSet::new("");
        assert_eq!(style_set.native().ref_counted_base()._ref_cnt(), 1);

        let tf = FontMgr::new()
            .legacy_make_typeface(None, FontStyle::default())
            .unwrap();
        let base_cnt = tf.native().ref_counted_base()._ref_cnt();

        let tfclone = tf.clone();
        assert_eq!(tf.native().ref_counted_base()._ref_cnt(), base_cnt + 1);

        style_set.append_typeface(tfclone);
        assert_eq!(tf.native().ref_counted_base()._ref_cnt(), base_cnt + 1);

        drop(style_set);
        assert_eq!(tf.native().ref_counted_base()._ref_cnt(), base_cnt);
        drop(tf);
    }

    #[test]
    #[serial_test::serial]
    fn treat_font_provider_as_font_mgr() {
        let mut font_collection = FontCollection::new();
        let typeface = FontMgr::new()
            .legacy_make_typeface(None, FontStyle::default())
            .unwrap();
        let mut manager = TypefaceFontProvider::new();
        manager.register_typeface(typeface, Some("AlArabiya"));
        assert_eq!(font_collection.font_managers_count(), 0);
        font_collection.set_asset_font_manager(Some(manager.into()));
        assert_eq!(font_collection.font_managers_count(), 1);
    }
}
