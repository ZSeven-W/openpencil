use std::{
    ffi::{CStr, CString},
    fmt, io,
    ops::Deref,
    path::Path,
};

use skia_bindings::{self as sb, SkData};

use crate::{interop::RustStream, prelude::*};

pub type Data = RCHandle<SkData>;
unsafe_send_sync!(Data);
require_base_type!(SkData, sb::SkNVRefCnt);

impl NativeRefCounted for SkData {
    fn _ref(&self) {
        unsafe { sb::C_SkData_ref(self) }
    }

    fn _unref(&self) {
        unsafe { sb::C_SkData_unref(self) }
    }

    fn unique(&self) -> bool {
        unsafe { sb::C_SkData_unique(self) }
    }
}

impl Deref for Data {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl PartialEq for Data {
    // Although there is an implementation in SkData for equality testing, we
    // prefer to stay on the Rust side for that.
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl fmt::Debug for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Data").field("size", &self.size()).finish()
    }
}

impl Data {
    pub fn size(&self) -> usize {
        self.native().fSpan.fSize
    }

    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { safer::from_raw_parts(self.native().fSpan.fPtr as _, self.size()) }
    }

    // TODO:
    // pub unsafe fn writable_data(&mut self) -> &mut [u8]

    pub fn copy_range(&self, offset: usize, buffer: &mut [u8]) -> &Self {
        buffer.copy_from_slice(&self.as_bytes()[offset..offset + buffer.len()]);
        self
    }

    // TODO: rename to copy_from() ? or from_bytes()?
    pub fn new_copy(data: &[u8]) -> Self {
        Data::from_ptr(unsafe { sb::C_SkData_MakeWithCopy(data.as_ptr() as _, data.len()) })
            .unwrap()
    }

    /// Constructs Data from a given byte slice without copying it.
    ///
    /// Users must make sure that the underlying slice will outlive the lifetime of the Data.
    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new_bytes(data: &[u8]) -> Self {
        Data::from_ptr(sb::C_SkData_MakeWithoutCopy(data.as_ptr() as _, data.len())).unwrap()
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn new_uninitialized(length: usize) -> Data {
        Data::from_ptr(sb::C_SkData_MakeUninitialized(length)).unwrap()
    }

    pub fn new_zero_initialized(length: usize) -> Data {
        Data::from_ptr(unsafe { sb::C_SkData_MakeZeroInitialized(length) }).unwrap()
    }

    // TODO: use Range as stand in for offset / length?
    pub fn new_subset(data: &Data, offset: usize, length: usize) -> Data {
        Data::from_ptr(unsafe { sb::C_SkData_MakeSubset(data.native(), offset, length) }).unwrap()
    }

    /// Constructs Data from a copy of a &str.
    ///
    /// Functions that use `Data` as a string container usually expect it to contain a c-string
    /// including the terminating 0 byte, so this function converts the Rust `str` to a `CString`
    /// and calls [`Self::new_cstr()`].
    pub fn new_str(str: impl AsRef<str>) -> Data {
        Self::new_cstr(&CString::new(str.as_ref()).unwrap())
    }

    /// Constructs Data from a &CStr by copying its contents.
    pub fn new_cstr(cstr: &CStr) -> Data {
        Data::from_ptr(unsafe { sb::C_SkData_MakeWithCString(cstr.as_ptr()) }).unwrap()
    }

    /// Create a new `Data` referencing the file with the specified path. If the file cannot be
    /// opened, the path contains 0 bytes, or the path is not valid UTF-8, this returns `None`.
    ///
    /// This function opens the file as a memory mapped file for the lifetime of `Data` returned.
    pub fn from_filename(path: impl AsRef<Path>) -> Option<Self> {
        let path = CString::new(path.as_ref().to_str()?).ok()?;
        Data::from_ptr(unsafe { sb::C_SkData_MakeFromFileName(path.as_ptr()) })
    }

    // TODO: MakeFromFile (is there a way to wrap this safely?)

    /// Attempt to read size bytes into a [`Data`]. If the read succeeds, return the data,
    /// else return `None`. Either way the stream's cursor may have been changed as a result
    /// of calling read().
    pub fn from_stream(mut stream: impl io::Read, size: usize) -> Option<Self> {
        let mut stream = RustStream::new(&mut stream);
        Data::from_ptr(unsafe { sb::C_SkData_MakeFromStream(stream.stream_mut(), size) })
    }

    pub fn new_empty() -> Self {
        Data::from_ptr(unsafe { sb::C_SkData_MakeEmpty() }).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl RefCount for SkData {
        fn ref_cnt(&self) -> usize {
            self._base.ref_cnt()
        }
    }

    #[test]
    fn data_supports_equals() {
        let x: &[u8] = &[1u8, 2u8, 3u8];
        let d1 = Data::new_copy(x);
        let d2 = Data::new_copy(x);
        assert!(d1 == d2)
    }

    #[test]
    fn from_stream_empty() {
        let data = [];
        let cursor = io::Cursor::new(data);
        let data = Data::from_stream(cursor, 0).unwrap();
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn from_stream() {
        let data = [1u8];
        let cursor = io::Cursor::new(data);
        let data = Data::from_stream(cursor, 1).unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0], 1u8);
    }
}
