use skia_bindings as sb;

use super::ScriptRunIterator;
use crate::{prelude::*, Borrows, FontMgr, FourByteTag, Shaper};

pub fn shaper_driven_wrapper(fallback_font_mgr: impl Into<Option<FontMgr>>) -> Option<Shaper> {
    #[cfg(feature = "embed-icudtl")]
    crate::icu::init();

    Shaper::from_ptr(unsafe {
        sb::C_SkShapers_HB_ShaperDrivenWrapper(fallback_font_mgr.into().into_ptr_or_null())
    })
}

pub fn shape_then_wrap(fallback_font_mgr: impl Into<Option<FontMgr>>) -> Option<Shaper> {
    #[cfg(feature = "embed-icudtl")]
    crate::icu::init();

    Shaper::from_ptr(unsafe {
        sb::C_SkShapers_HB_ShapeThenWrap(fallback_font_mgr.into().into_ptr_or_null())
    })
}

pub fn shape_dont_wrap_or_reorder(fallback_font_mgr: impl Into<Option<FontMgr>>) -> Option<Shaper> {
    #[cfg(feature = "embed-icudtl")]
    crate::icu::init();

    Shaper::from_ptr(unsafe {
        sb::C_SkShapers_HB_ShapeDontWrapOrReorder(fallback_font_mgr.into().into_ptr_or_null())
    })
}

pub fn script_run_iterator(
    utf8: &str,
    script: impl Into<Option<FourByteTag>>,
) -> Borrows<ScriptRunIterator> {
    let script = script.into();
    if let Some(tag) = script {
        Shaper::new_script_run_iterator(utf8, tag)
    } else {
        Shaper::new_hb_icu_script_run_iterator(utf8)
    }
}
