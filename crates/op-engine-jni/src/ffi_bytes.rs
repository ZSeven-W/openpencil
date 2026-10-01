//! Platform-free marshalling of optional byte ranges onto the C ABI.
//!
//! Several editor entry points take an optional UTF-8 argument as a
//! `(pointer, length)` pair where `NULL` / `0` means "not supplied" (for
//! example the media type and file name of `op_editor_attach_chat_image`).
//! The JNI and NAPI bindings both own their arguments as `Option<Vec<u8>>`
//! before dispatching onto the engine thread; this module is the single
//! rule for turning that into the pair, so both mobile layers send the same
//! "absent" encoding and the rule is unit-tested on the host triple.

/// Drops an empty value: a blank optional argument is the same as an absent
/// one, so the engine applies its own default (sniffed type, generated name)
/// instead of rejecting an empty string.
pub fn non_empty(bytes: Option<Vec<u8>>) -> Option<Vec<u8>> {
    bytes.filter(|bytes| !bytes.is_empty())
}

/// The `(pointer, length)` pair for an optional borrowed range: `NULL` / `0`
/// when absent or empty, otherwise the slice's own address and length. The
/// pointer is only valid while `bytes` is borrowed.
pub fn optional_ptr_len(bytes: Option<&[u8]>) -> (*const u8, usize) {
    match bytes {
        Some(bytes) if !bytes.is_empty() => (bytes.as_ptr(), bytes.len()),
        _ => (std::ptr::null(), 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_and_empty_ranges_marshal_as_null_zero() {
        assert_eq!(optional_ptr_len(None), (std::ptr::null(), 0));
        assert_eq!(optional_ptr_len(Some(&[])), (std::ptr::null(), 0));
    }

    #[test]
    fn present_ranges_keep_their_address_and_length() {
        let bytes = b"image/jpeg".to_vec();
        let (ptr, len) = optional_ptr_len(Some(&bytes));
        assert_eq!(ptr, bytes.as_ptr());
        assert_eq!(len, bytes.len());
    }

    #[test]
    fn blank_optional_arguments_collapse_to_absent() {
        assert_eq!(non_empty(None), None);
        assert_eq!(non_empty(Some(Vec::new())), None);
        assert_eq!(
            non_empty(Some(b"photo.jpg".to_vec())),
            Some(b"photo.jpg".to_vec())
        );
    }
}
