//! Optional ABI-v2 collaboration-ticket adapter.
//!
//! This module is compiled only when the selected prebuilt explicitly ships
//! `ABI_VERSION` containing `2`. Legacy archives have no such metadata, so
//! their link never references these additional symbols.

use crate::{
    CollabTicketError, CollabTicketPoll, CollabTicketProvider, CollabTicketProviderErrorCode,
    CollabTicketRequest, CollabTicketRequestId, OpaqueCollabTicket, MAX_COLLAB_TICKET_BYTES,
};

const ABI_OK: i32 = 0;
const POLL_PENDING: i32 = 0;
const POLL_READY: i32 = 1;
const POLL_FAILED: i32 = 2;
const POLL_REQUEST_NOT_FOUND: i32 = 3;

const ERROR_NOT_SIGNED_IN: i32 = 1;
const ERROR_NETWORK_UNAVAILABLE: i32 = 2;
const ERROR_RATE_LIMITED: i32 = 3;
const ERROR_REQUEST_REJECTED: i32 = 4;

#[repr(C)]
struct OpAuthCollabTicketStatus {
    ticket: *mut u8,
    ticket_len: usize,
    expires_at_unix_hint: u64,
    provider_error: i32,
}

extern "C" {
    fn op_auth_collab_ticket_begin(
        dh_pub_x25519: *const u8,
        dh_pub_x25519_len: usize,
        request_id: *mut u64,
    ) -> i32;
    fn op_auth_collab_ticket_poll(request_id: u64, status: *mut OpAuthCollabTicketStatus) -> i32;
    fn op_auth_collab_ticket_cancel(request_id: u64);
    fn op_auth_string_free(ptr: *mut u8, len: usize);
}

struct RealCollabTicketProvider;

static REAL_COLLAB_TICKET_PROVIDER: RealCollabTicketProvider = RealCollabTicketProvider;

pub(crate) fn provider() -> &'static dyn CollabTicketProvider {
    &REAL_COLLAB_TICKET_PROVIDER
}

impl CollabTicketProvider for RealCollabTicketProvider {
    fn available(&self) -> bool {
        true
    }

    fn begin_ticket(
        &self,
        request: CollabTicketRequest,
    ) -> Result<CollabTicketRequestId, CollabTicketError> {
        let mut raw_request_id = 0_u64;
        // SAFETY: the public key and out-pointer remain valid for the call.
        let code = unsafe {
            op_auth_collab_ticket_begin(
                request.dh_pub_x25519().as_ptr(),
                request.dh_pub_x25519().len(),
                &mut raw_request_id,
            )
        };
        if code != ABI_OK {
            return Err(provider_failure(code));
        }
        CollabTicketRequestId::new(raw_request_id).ok_or_else(|| provider_failure(0))
    }

    fn poll_ticket(&self, id: CollabTicketRequestId) -> CollabTicketPoll {
        let mut raw = OpAuthCollabTicketStatus {
            ticket: std::ptr::null_mut(),
            ticket_len: 0,
            expires_at_unix_hint: 0,
            provider_error: 0,
        };
        // SAFETY: `raw` is a valid out-pointer for the duration of the call.
        let code = unsafe { op_auth_collab_ticket_poll(id.get(), &mut raw) };
        match code {
            POLL_PENDING => {
                discard_ticket_buffer(raw.ticket, raw.ticket_len);
                CollabTicketPoll::Pending
            }
            POLL_READY => match take_ticket_buffer(raw.ticket, raw.ticket_len) {
                Ok(ticket) => CollabTicketPoll::Ready {
                    ticket,
                    expires_at_unix_hint: (raw.expires_at_unix_hint != 0)
                        .then_some(raw.expires_at_unix_hint),
                },
                Err(error) => CollabTicketPoll::Failed(error),
            },
            POLL_FAILED => {
                discard_ticket_buffer(raw.ticket, raw.ticket_len);
                CollabTicketPoll::Failed(provider_failure(raw.provider_error))
            }
            POLL_REQUEST_NOT_FOUND => {
                discard_ticket_buffer(raw.ticket, raw.ticket_len);
                CollabTicketPoll::Failed(CollabTicketError::RequestNotFound { id: id.get() })
            }
            _ => {
                discard_ticket_buffer(raw.ticket, raw.ticket_len);
                CollabTicketPoll::Failed(provider_failure(0))
            }
        }
    }

    fn cancel_ticket(&self, id: CollabTicketRequestId) {
        // SAFETY: request ids are opaque scalar handles.
        unsafe { op_auth_collab_ticket_cancel(id.get()) };
    }
}

fn take_ticket_buffer(
    pointer: *mut u8,
    length: usize,
) -> Result<OpaqueCollabTicket, CollabTicketError> {
    if pointer.is_null() || length == 0 || length > MAX_COLLAB_TICKET_BYTES {
        if !pointer.is_null() {
            // Do not dereference an out-of-policy length from the FFI boundary.
            unsafe { op_auth_string_free(pointer, length) };
        }
        return Err(CollabTicketError::InvalidTicketSize {
            actual: length,
            maximum: MAX_COLLAB_TICKET_BYTES,
        });
    }
    // SAFETY: ABI v2 guarantees `length` readable and writable bytes owned by
    // the caller until `op_auth_string_free`.
    let bytes = unsafe { std::slice::from_raw_parts(pointer, length) }.to_vec();
    zeroize_and_free(pointer, length);
    OpaqueCollabTicket::new(bytes)
}

fn discard_ticket_buffer(pointer: *mut u8, length: usize) {
    if pointer.is_null() {
        return;
    }
    if length <= MAX_COLLAB_TICKET_BYTES {
        zeroize_and_free(pointer, length);
    } else {
        // The length is outside the public ABI policy, so avoid dereferencing
        // it and return ownership through the provider's allocator.
        unsafe { op_auth_string_free(pointer, length) };
    }
}

fn zeroize_and_free(pointer: *mut u8, length: usize) {
    for index in 0..length {
        // SAFETY: ABI v2 transfers a writable allocation of exactly `length`
        // bytes. Volatile writes prevent ticket bytes from surviving the copy.
        unsafe { pointer.add(index).write_volatile(0) };
    }
    // SAFETY: the allocation is returned exactly once to its owner.
    unsafe { op_auth_string_free(pointer, length) };
}

fn provider_failure(code: i32) -> CollabTicketError {
    let code = match code {
        ERROR_NOT_SIGNED_IN => CollabTicketProviderErrorCode::NotSignedIn,
        ERROR_NETWORK_UNAVAILABLE => CollabTicketProviderErrorCode::NetworkUnavailable,
        ERROR_RATE_LIMITED => CollabTicketProviderErrorCode::RateLimited,
        ERROR_REQUEST_REJECTED => CollabTicketProviderErrorCode::RequestRejected,
        _ => CollabTicketProviderErrorCode::Internal,
    };
    CollabTicketError::ProviderFailure { code }
}
