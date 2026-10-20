//! Independently negotiated ABI-v4 Hosted access adapter.

use crate::{HostedAccess, HostedAccessError, HostedAccessPoll, MAX_HOSTED_ACCESS_BYTES};
#[cfg(op_auth_hosted_access_prebuilt)]
use crate::{HostedAccessProvider, HostedAccessRequestId};

#[repr(C)]
struct OpAuthHostedAccessStatus {
    payload: *mut u8,
    payload_len: usize,
    provider_error: i32,
}

#[cfg(op_auth_hosted_access_prebuilt)]
extern "C" {
    fn op_auth_hosted_access_begin(request_id: *mut u64) -> i32;
    fn op_auth_hosted_access_poll(request_id: u64, status: *mut OpAuthHostedAccessStatus) -> i32;
    fn op_auth_hosted_access_cancel(request_id: u64);
    fn op_auth_hosted_session_epoch() -> u64;
    fn op_auth_string_free(pointer: *mut u8, length: usize);
}

#[cfg(op_auth_hosted_access_prebuilt)]
struct RealHostedAccessProvider;

#[cfg(op_auth_hosted_access_prebuilt)]
pub(crate) fn provider() -> &'static dyn HostedAccessProvider {
    &RealHostedAccessProvider
}

#[cfg(op_auth_hosted_access_prebuilt)]
impl HostedAccessProvider for RealHostedAccessProvider {
    fn available(&self) -> bool {
        true
    }

    fn session_epoch(&self) -> u64 {
        // SAFETY: no arguments; this scalar export is part of ABI v4.
        unsafe { op_auth_hosted_session_epoch() }
    }

    fn begin(&self) -> Result<HostedAccessRequestId, HostedAccessError> {
        let mut raw_id = 0;
        // SAFETY: the out-pointer remains valid for the call; ABI v4 is linked.
        let code = unsafe { op_auth_hosted_access_begin(&mut raw_id) };
        if code != 0 {
            return Err(provider_failure(code));
        }
        HostedAccessRequestId::new(raw_id).ok_or(HostedAccessError::Internal)
    }

    fn poll(&self, id: HostedAccessRequestId) -> Result<HostedAccessPoll, HostedAccessError> {
        let mut raw = OpAuthHostedAccessStatus {
            payload: std::ptr::null_mut(),
            payload_len: 0,
            provider_error: 0,
        };
        // SAFETY: the out-pointer remains valid for the call; the provider owns
        // the returned allocation until the bridge returns it through free.
        let code = unsafe { op_auth_hosted_access_poll(id.get(), &mut raw) };
        decode_poll(code, raw, op_auth_string_free)
    }

    fn cancel(&self, id: HostedAccessRequestId) {
        // SAFETY: request ids are opaque scalar handles, independent of login.
        unsafe { op_auth_hosted_access_cancel(id.get()) };
    }
}

fn decode_poll(
    code: i32,
    raw: OpAuthHostedAccessStatus,
    free: unsafe extern "C" fn(*mut u8, usize),
) -> Result<HostedAccessPoll, HostedAccessError> {
    let payload = ProviderPayload {
        pointer: raw.payload,
        length: raw.payload_len,
        free,
    };
    match code {
        0 if payload.is_absent() && raw.provider_error == 0 => Ok(HostedAccessPoll::Pending),
        1 if raw.provider_error == 0 => payload.take().map(HostedAccessPoll::Ready),
        2 if payload.is_absent() => Ok(HostedAccessPoll::Failed(provider_failure(
            raw.provider_error,
        ))),
        3 if payload.is_absent() && raw.provider_error == 0 => {
            Err(HostedAccessError::RequestNotFound)
        }
        _ => Err(HostedAccessError::InvalidPayload),
    }
}

struct ProviderPayload {
    pointer: *mut u8,
    length: usize,
    free: unsafe extern "C" fn(*mut u8, usize),
}

impl ProviderPayload {
    fn is_absent(&self) -> bool {
        self.pointer.is_null() && self.length == 0
    }

    fn take(self) -> Result<HostedAccess, HostedAccessError> {
        if self.pointer.is_null() || self.length == 0 || self.length > MAX_HOSTED_ACCESS_BYTES {
            return Err(HostedAccessError::InvalidPayload);
        }
        // SAFETY: a signed/debug-opted-in ABI-v4 provider transfers a writable
        // allocation of exactly length bytes, which this guard frees once.
        let bytes = unsafe { std::slice::from_raw_parts(self.pointer, self.length) }.to_vec();
        HostedAccess::from_provider_json(bytes)
    }
}

impl Drop for ProviderPayload {
    fn drop(&mut self) {
        if self.pointer.is_null() {
            return;
        }
        // Do not dereference out-of-policy lengths. The provider's free export
        // must also zeroize allocations before returning them to its allocator.
        if self.length <= MAX_HOSTED_ACCESS_BYTES {
            for index in 0..self.length {
                // SAFETY: the ABI transfers this writable allocation to us.
                unsafe { self.pointer.add(index).write_volatile(0) };
            }
        }
        // SAFETY: ownership returns exactly once through the original allocator.
        unsafe { (self.free)(self.pointer, self.length) };
    }
}

fn provider_failure(code: i32) -> HostedAccessError {
    match code {
        1 => HostedAccessError::NotSignedIn,
        2 => HostedAccessError::NetworkUnavailable,
        3 => HostedAccessError::RateLimited,
        4 => HostedAccessError::RequestRejected,
        6 => HostedAccessError::Unavailable,
        7 => HostedAccessError::Cancelled,
        _ => HostedAccessError::Internal,
    }
}

#[cfg(test)]
#[path = "real_hosted_access_tests.rs"]
mod tests;
