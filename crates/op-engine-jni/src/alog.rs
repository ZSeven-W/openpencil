//! Minimal logcat writer (Android only).
//!
//! Binds `liblog`'s `__android_log_write` directly so the JNI layer can
//! emit the paired window-ownership lines and diagnostics acceptance
//! scripts assert via `adb logcat`, without pulling in a `log`-facade
//! backend.

#![cfg(target_os = "android")]

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

// `liblog` is a system library on every Android API level.
#[link(name = "log")]
extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

/// `ANDROID_LOG_INFO` (from `<android/log.h>`).
const ANDROID_LOG_INFO: c_int = 4;
/// `ANDROID_LOG_ERROR` (from `<android/log.h>`).
const ANDROID_LOG_ERROR: c_int = 6;

/// Writes one INFO line to logcat under `tag`. Best-effort: a `tag`/`msg`
/// carrying an interior NUL is dropped rather than truncated mid-message
/// (`CString::new` fails), and the call never panics.
pub fn info(tag: &str, msg: &str) {
    write(ANDROID_LOG_INFO, tag, msg);
}

/// Writes one ERROR line to logcat under `tag`. Best-effort, like [`info`].
pub fn error(tag: &str, msg: &str) {
    write(ANDROID_LOG_ERROR, tag, msg);
}

fn write(prio: c_int, tag: &str, msg: &str) {
    if let (Ok(tag), Ok(msg)) = (CString::new(tag), CString::new(msg)) {
        // SAFETY: both pointers are valid NUL-terminated C strings for the
        // duration of the call.
        unsafe { __android_log_write(prio, tag.as_ptr(), msg.as_ptr()) };
    }
}
