use skia_bindings as sb;

use super::BiDiRunIterator;
use crate::prelude::*;

pub fn bidi_run_iterator(utf8: &str, bidi_level: u8) -> Option<Borrows<BiDiRunIterator>> {
    let bytes = utf8.as_bytes();
    BiDiRunIterator::from_ptr(unsafe {
        sb::C_SkShapers_unicode_BidiRunIterator(bytes.as_ptr() as _, bytes.len(), bidi_level)
    })
    .map(|i| i.borrows(utf8))
}
