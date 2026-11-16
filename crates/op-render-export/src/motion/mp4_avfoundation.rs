use std::ffi::{c_char, c_void, CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::NonNull;

use super::encoding_error;
use crate::ExportError;

unsafe extern "C" {
    fn op_motion_mp4_open(
        path: *const c_char,
        width: i32,
        height: i32,
        fps: i32,
        error: *mut c_char,
        capacity: usize,
    ) -> *mut c_void;
    fn op_motion_mp4_push(
        handle: *mut c_void,
        pixels: *const u8,
        index: i64,
        error: *mut c_char,
        capacity: usize,
    ) -> bool;
    fn op_motion_mp4_finish(
        handle: *mut c_void,
        frames: i64,
        error: *mut c_char,
        capacity: usize,
    ) -> bool;
    fn op_motion_mp4_close(handle: *mut c_void);
}

pub(super) fn available() -> bool {
    true
}

pub(super) struct Encoder {
    handle: NonNull<c_void>,
    frame_bytes: usize,
}

impl Encoder {
    pub(super) fn open(
        path: &Path,
        (width, height): (i32, i32),
        fps: u32,
    ) -> Result<Self, ExportError> {
        let path = CString::new(path.as_os_str().as_bytes()).map_err(encoding_error)?;
        let mut error = [0; 512];
        let handle = unsafe {
            op_motion_mp4_open(
                path.as_ptr(),
                width,
                height,
                fps as i32,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        NonNull::new(handle)
            .map(|handle| Self {
                handle,
                frame_bytes: width as usize * height as usize * 4,
            })
            .ok_or_else(|| error_message(&error))
    }

    pub(super) fn push(&mut self, pixels: &[u8], index: u64) -> Result<(), ExportError> {
        let mut error = [0; 512];
        if pixels.len() != self.frame_bytes {
            return Err(encoding_error("invalid BGRA frame length"));
        }
        // The caller supplies exactly the padded BGRA canvas used at open.
        let ok = unsafe {
            op_motion_mp4_push(
                self.handle.as_ptr(),
                pixels.as_ptr(),
                index as i64,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        ok.then_some(()).ok_or_else(|| error_message(&error))
    }

    pub(super) fn finish(self, frames: u64) -> Result<(), ExportError> {
        let mut error = [0; 512];
        let ok = unsafe {
            op_motion_mp4_finish(
                self.handle.as_ptr(),
                frames as i64,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        ok.then_some(()).ok_or_else(|| error_message(&error))
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { op_motion_mp4_close(self.handle.as_ptr()) }
    }
}

fn error_message(bytes: &[c_char]) -> ExportError {
    // The native bridge always writes through snprintf to a zeroed buffer.
    let detail = unsafe { CStr::from_ptr(bytes.as_ptr()) }.to_string_lossy();
    encoding_error(detail)
}
