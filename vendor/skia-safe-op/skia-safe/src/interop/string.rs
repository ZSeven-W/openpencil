#![allow(dead_code)]
use std::{fmt, str};

use crate::prelude::*;
use skia_bindings::{self as sb, SkString};

pub type String = Handle<SkString>;
unsafe_send_sync!(String);

impl NativeDrop for SkString {
    fn drop(&mut self) {
        unsafe {
            sb::C_SkString_destruct(self);
        }
    }
}

impl NativeClone for SkString {
    fn clone(&self) -> Self {
        construct(|unitialized| unsafe { sb::C_SkString_CopyConstruct(unitialized, self) })
    }
}

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl Default for String {
    fn default() -> Self {
        Self::from_str("")
    }
}

impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl String {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(str: impl AsRef<str>) -> String {
        let bytes = str.as_ref().as_bytes();
        Handle::from_native_c(unsafe { SkString::new3(bytes.as_ptr() as _, bytes.len()) })
    }

    pub fn as_str(&self) -> &str {
        self.native().as_str()
    }
}

pub trait AsStr {
    fn as_str(&self) -> &str;
}

impl AsStr for Handle<SkString> {
    fn as_str(&self) -> &str {
        self.native().as_str()
    }
}

impl AsStr for SkString {
    fn as_str(&self) -> &str {
        let mut size = 0;
        let slice = unsafe {
            let ptr = sb::C_SkString_c_str_size(self, &mut size) as *const u8;
            safer::from_raw_parts(ptr, size)
        };
        std::str::from_utf8(slice).unwrap_or_default()
    }
}

impl AsStr for sb::std_string_view {
    fn as_str(&self) -> &str {
        let slice = unsafe {
            let mut size = 0;
            let ptr = sb::C_string_view_ptr_size(self, &mut size) as *const u8;
            safer::from_raw_parts(ptr, size)
        };
        str::from_utf8(slice).unwrap_or_default()
    }
}

impl AsStr for sb::std_string {
    fn as_str(&self) -> &str {
        let slice = unsafe {
            let mut size = 0;
            let ptr = sb::C_string_ptr_size(self, &mut size) as *const u8;
            safer::from_raw_parts(ptr, size)
        };
        str::from_utf8(slice).unwrap_or_default()
    }
}

pub trait SetStr {
    fn set_str(&mut self, str: impl AsRef<str>);
}

impl SetStr for Handle<SkString> {
    fn set_str(&mut self, str: impl AsRef<str>) {
        self.native_mut().set_str(str)
    }
}

impl SetStr for SkString {
    fn set_str(&mut self, str: impl AsRef<str>) {
        let bytes = str.as_ref().as_bytes();
        unsafe { self.set1(bytes.as_ptr() as _, bytes.len()) }
    }
}

pub trait FromStrs {
    fn from_strs(strings: &[impl AsRef<str>]) -> Self;
}

impl FromStrs for Vec<String> {
    fn from_strs(strings: &[impl AsRef<str>]) -> Self {
        strings
            .iter()
            .map(|s| String::from_str(s.as_ref()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{SetStr, String};

    #[test]
    fn string_from_rust_and_back() {
        let str = "Hello";
        let string = String::from_str(str);
        assert_eq!(str, string.as_str())
    }

    #[test]
    fn set_string() {
        let mut hello = String::from_str("Hello");
        hello.set_str(String::from_str("World"));
        assert_eq!("World", hello.as_str());
    }
}
