use super::*;
use std::cell::Cell;

thread_local! {
    static FREES: Cell<usize> = const { Cell::new(0) };
    static WAS_ZEROIZED: Cell<bool> = const { Cell::new(false) };
}

unsafe extern "C" fn free_fixture(pointer: *mut u8, length: usize) {
    // SAFETY: tests transfer a boxed slice matching the status length.
    let bytes = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(pointer, length)) };
    WAS_ZEROIZED.set(bytes.iter().all(|byte| *byte == 0));
    FREES.set(FREES.get() + 1);
}

fn status(bytes: Option<Vec<u8>>, error: i32) -> OpAuthHostedAccessStatus {
    FREES.set(0);
    WAS_ZEROIZED.set(false);
    let (pointer, length) = if let Some(bytes) = bytes {
        let mut boxed = bytes.into_boxed_slice();
        let pair = (boxed.as_mut_ptr(), boxed.len());
        std::mem::forget(boxed);
        pair
    } else {
        (std::ptr::null_mut(), 0)
    };
    OpAuthHostedAccessStatus {
        payload: pointer,
        payload_len: length,
        provider_error: error,
    }
}

#[test]
fn closed_poll_states_and_errors() {
    assert!(matches!(
        decode_poll(0, status(None, 0), free_fixture),
        Ok(HostedAccessPoll::Pending)
    ));
    assert!(matches!(
        decode_poll(3, status(None, 0), free_fixture),
        Err(HostedAccessError::RequestNotFound)
    ));
    for (code, expected) in [
        (1, HostedAccessError::NotSignedIn),
        (2, HostedAccessError::NetworkUnavailable),
        (3, HostedAccessError::RateLimited),
        (4, HostedAccessError::RequestRejected),
        (5, HostedAccessError::Internal),
        (6, HostedAccessError::Unavailable),
        (7, HostedAccessError::Cancelled),
        (i32::MAX, HostedAccessError::Internal),
    ] {
        assert!(
            matches!(decode_poll(2, status(None, code), free_fixture), Ok(HostedAccessPoll::Failed(error)) if error == expected)
        );
    }
    assert!(matches!(
        decode_poll(99, status(None, 0), free_fixture),
        Err(HostedAccessError::InvalidPayload)
    ));
}

#[test]
fn every_inconsistent_or_malformed_payload_is_zeroized_and_freed_once() {
    for code in [0, 1, 2, 3, 99] {
        let raw = status(Some(b"malformed-secret-bearing-payload".to_vec()), 0);
        assert!(decode_poll(code, raw, free_fixture).is_err());
        assert_eq!(FREES.get(), 1);
        assert!(WAS_ZEROIZED.get());
    }
}

#[test]
fn oversized_payload_returns_ownership_without_dereferencing_it() {
    let raw = status(Some(vec![7; MAX_HOSTED_ACCESS_BYTES + 1]), 0);
    assert!(matches!(
        decode_poll(1, raw, free_fixture),
        Err(HostedAccessError::InvalidPayload)
    ));
    assert_eq!(FREES.get(), 1);
    assert!(!WAS_ZEROIZED.get());
}

#[test]
fn ready_requires_nonempty_buffer_and_clear_provider_error() {
    assert!(decode_poll(1, status(None, 0), free_fixture).is_err());
    let raw = status(Some(b"secret".to_vec()), 5);
    assert!(decode_poll(1, raw, free_fixture).is_err());
    assert_eq!(FREES.get(), 1);
    assert!(WAS_ZEROIZED.get());
    let mut raw = status(None, 0);
    raw.payload_len = 64;
    assert!(decode_poll(0, raw, free_fixture).is_err());
}
