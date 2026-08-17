//! Minimal HiLog writer (OHOS only).
//!
//! The twin of `op-engine-jni/src/alog.rs`: binds `libhilog_ndk.z.so`'s
//! `OH_LOG_Print` directly so the NAPI layer can emit the paired
//! surface-ownership lines and diagnostics that acceptance scripts assert
//! via `hdc hilog`, without pulling in a `log`-facade backend.

#![cfg(all(target_os = "linux", target_env = "ohos"))]

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};

// `libhilog_ndk.z.so` ships in the OHOS NDK sysroot on every API level.
#[link(name = "hilog_ndk.z")]
extern "C" {
    /// `hilog/log.h`. Variadic; every call here passes exactly one `%{public}s`
    /// argument so the message is never re-interpreted as a format string.
    fn OH_LOG_Print(
        log_type: c_int,
        level: c_int,
        domain: c_uint,
        tag: *const c_char,
        fmt: *const c_char,
        ...
    ) -> c_int;
}

/// `LOG_APP` — the only log type available to third-party applications.
const LOG_APP: c_int = 0;
/// `LOG_INFO` (from `hilog/log.h`).
const LOG_INFO: c_int = 4;
/// `LOG_ERROR` (from `hilog/log.h`).
const LOG_ERROR: c_int = 6;

/// Service domain for every line this crate emits. Third-party app domains
/// are 16-bit values chosen by the app; `0xF000` keeps OpenPencil's lines
/// filterable with `hdc hilog | grep 0xF000`.
const DOMAIN: c_uint = 0xF000;

/// Writes one INFO line under `tag`. Best-effort: a `tag`/`msg` carrying an
/// interior NUL is dropped rather than truncated mid-message
/// (`CString::new` fails), and the call never panics.
pub fn info(tag: &str, msg: &str) {
    write(LOG_INFO, tag, msg);
}

/// Writes one ERROR line under `tag`. Best-effort, like [`info`].
pub fn error(tag: &str, msg: &str) {
    write(LOG_ERROR, tag, msg);
}

fn write(level: c_int, tag: &str, msg: &str) {
    let (Ok(tag), Ok(msg)) = (CString::new(tag), CString::new(msg)) else {
        return;
    };
    // `%{public}s` keeps the payload out of HiLog's privacy redaction; the
    // messages here are diagnostics, never user content.
    let format = c"%{public}s";
    // SAFETY: all three pointers are valid NUL-terminated C strings for the
    // duration of the call, and the single vararg matches the one conversion
    // in the format string.
    unsafe {
        OH_LOG_Print(
            LOG_APP,
            level,
            DOMAIN,
            tag.as_ptr(),
            format.as_ptr(),
            msg.as_ptr(),
        )
    };
}
