use crate::{
    interop::{FromStrs, String},
    prelude::*,
};
use skia_bindings::{self as sb, SkStrings};
use std::{fmt, ops::Index};

pub type Strings = Handle<SkStrings>;
unsafe_send_sync!(Strings);

impl NativeDrop for SkStrings {
    fn drop(&mut self) {
        unsafe {
            sb::C_SkStrings_destruct(self);
        }
    }
}

impl fmt::Debug for Strings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Strings").field(&self.as_slice()).finish()
    }
}

impl Handle<SkStrings> {
    /// Constructs a native Strings array from a slice of SkStrings by moving them.
    pub fn new(mut strings: Vec<String>) -> Self {
        Strings::construct(|s| unsafe {
            sb::C_SkStrings_construct(s, strings.native_mut().as_mut_ptr(), strings.len())
        })
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        let mut count = 0;
        unsafe {
            sb::C_SkStrings_ptr_count(self.native(), &mut count);
        }
        count
    }

    pub fn as_slice(&self) -> &[String] {
        let mut count = 0;
        let ptr = unsafe { sb::C_SkStrings_ptr_count(self.native(), &mut count) };
        unsafe { safer::from_raw_parts(ptr as *const String, count) }
    }
}

impl Index<usize> for Strings {
    type Output = String;
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl FromStrs for Strings {
    fn from_strs(strings: &[impl AsRef<str>]) -> Self {
        Strings::new(strings.iter().map(String::from_str).collect())
    }
}

#[test]
fn test_strings() {
    let strings = ["Hello", "World"];
    let strings = Strings::from_strs(&strings);
    assert_eq!(strings[0].as_str(), "Hello");
    assert_eq!(strings[1].as_str(), "World");
}
