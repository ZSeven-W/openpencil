use crate::prelude::*;
use skia_bindings::{self as sb, skia_textlayout_ParagraphCache};
use std::fmt;

pub type ParagraphCache = Handle<skia_textlayout_ParagraphCache>;

impl NativeDrop for skia_textlayout_ParagraphCache {
    fn drop(&mut self) {
        unsafe { sb::C_ParagraphCache_destruct(self) }
    }
}

impl fmt::Debug for ParagraphCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParagraphCache").finish()
    }
}

impl ParagraphCache {
    pub fn new() -> ParagraphCache {
        ParagraphCache::from_native_c(unsafe { skia_textlayout_ParagraphCache::new() })
    }

    pub fn abandon(&mut self) {
        unsafe { self.native_mut().abandon() }
    }

    pub fn reset(&mut self) {
        unsafe { self.native_mut().reset() }
    }

    pub fn print_statistics(&mut self) {
        unsafe { self.native_mut().printStatistics() }
    }

    pub fn turn_on(&mut self, value: bool) {
        unsafe { sb::C_ParagraphCache_turnOn(self.native_mut(), value) }
    }

    pub fn count(&mut self) -> i32 {
        unsafe { sb::C_ParagraphCache_count(self.native_mut()) }
    }
}
