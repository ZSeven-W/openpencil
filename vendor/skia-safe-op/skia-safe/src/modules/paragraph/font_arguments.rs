use crate::{prelude::*, Typeface};
use skia_bindings::{self as sb, skia_textlayout_FontArguments};
use std::{fmt, hash};

pub type FontArguments = Handle<skia_textlayout_FontArguments>;
unsafe_send_sync!(FontArguments);

impl From<crate::FontArguments<'_, '_>> for FontArguments {
    fn from(fa: crate::FontArguments<'_, '_>) -> Self {
        FontArguments::construct(|uninitialized| unsafe {
            sb::C_FontArguments_Construct(fa.native(), uninitialized)
        })
    }
}

impl NativeClone for skia_textlayout_FontArguments {
    fn clone(&self) -> Self {
        unsafe { construct(|fa| sb::C_FontArguments_CopyConstruct(fa, self)) }
    }
}

impl NativeDrop for skia_textlayout_FontArguments {
    fn drop(&mut self) {
        unsafe { sb::C_FontArguments_destruct(self) }
    }
}

impl NativePartialEq for skia_textlayout_FontArguments {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { sb::C_FontArguments_Equals(self, rhs) }
    }
}

impl NativeHash for skia_textlayout_FontArguments {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        state.write_usize(unsafe { sb::C_FontArguments_hash(self) })
    }
}

impl fmt::Debug for FontArguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontArguments").finish()
    }
}

impl FontArguments {
    pub fn clone_typeface(&self, typeface: impl Into<Typeface>) -> Option<Typeface> {
        Typeface::from_ptr(unsafe {
            sb::C_FontArguments_cloneTypeface(self.native(), typeface.into().into_ptr())
        })
    }
}
