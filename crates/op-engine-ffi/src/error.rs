//! ABI error plumbing for `op-engine-ffi` — status-carrying errors,
//! the create-time error slot, and UTF-8 / byte-buffer marshalling.

use crate::OpStatus;
use std::cell::RefCell;
use std::ptr;

/// Soft cap on error strings and other small ABI byte payloads.
pub(crate) const STRING_CAP: usize = 16 * 1024 * 1024;
/// Cap on the `.op` document payload passed through `OpCreateDesc`.
pub(crate) const DOCUMENT_CAP: usize = 256 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct FfiError {
    pub status: OpStatus,
    pub message: String,
}

impl FfiError {
    pub fn new(status: OpStatus, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(OpStatus::InvalidArg, message)
    }
}

pub(crate) type FfiResult<T> = Result<T, FfiError>;

thread_local! {
    static CREATE_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

pub(crate) fn set_create_error(message: impl Into<String>) {
    CREATE_ERROR.with(|slot| *slot.borrow_mut() = message.into());
}

pub(crate) fn clear_create_error() {
    CREATE_ERROR.with(|slot| slot.borrow_mut().clear());
}

pub(crate) fn create_error() -> String {
    CREATE_ERROR.with(|slot| slot.borrow().clone())
}

/// Read a caller-owned UTF-8 byte range into a `String`, enforcing caps.
///
/// # Safety
///
/// `pointer` must cover `length` readable bytes when non-null.
pub(crate) unsafe fn read_utf8(
    pointer: *const u8,
    length: usize,
    cap: usize,
    label: &str,
) -> FfiResult<String> {
    if length > cap {
        return Err(FfiError::invalid(format!(
            "{label} length exceeds {cap} bytes"
        )));
    }
    if length == 0 {
        return Ok(String::new());
    }
    if pointer.is_null() {
        return Err(FfiError::invalid(format!(
            "{label} pointer is null with nonzero length"
        )));
    }
    if length > isize::MAX as usize {
        return Err(FfiError::invalid(format!("{label} length overflows")));
    }
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) };
    let text = std::str::from_utf8(bytes)
        .map_err(|_| FfiError::invalid(format!("{label} is not valid UTF-8")))?;
    Ok(text.to_owned())
}

/// Copy a byte value into a caller-owned buffer, reporting the required
/// length through `required` when the buffer is too small.
///
/// # Safety
///
/// `buffer` must cover `length` writable bytes when non-null and
/// `required` must be writable.
pub(crate) unsafe fn write_bytes(
    value: &[u8],
    buffer: *mut u8,
    length: usize,
    required: *mut usize,
) -> FfiResult<()> {
    if required.is_null() {
        return Err(FfiError::invalid("required-length pointer is null"));
    }
    unsafe { required.write(value.len()) };
    if buffer.is_null() {
        if length == 0 {
            return Ok(());
        }
        return Err(FfiError::invalid(
            "output buffer is null with nonzero length",
        ));
    }
    if length > isize::MAX as usize {
        return Err(FfiError::invalid("output buffer length overflows"));
    }
    let copied = length.min(value.len());
    if copied != 0 {
        unsafe { ptr::copy_nonoverlapping(value.as_ptr(), buffer, copied) };
    }
    Ok(())
}
