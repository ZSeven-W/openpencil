//! Backend backed by the prebuilt proprietary static library. Compiled
//! only when `build.rs` found `prebuilt/<target>/libop_auth.a`; the ABI
//! handshake still guards against stale artifacts at runtime.

use std::sync::OnceLock;

use crate::status::AuthStatus;
use crate::{
    supports_base_abi, AuthInitConfig, CollabTicketProvider, MAX_SUPPORTED_ABI_VERSION,
    REQUIRED_ABI_VERSION, UNAVAILABLE_COLLAB_TICKET_PROVIDER,
};

#[repr(C)]
struct OpAuthStatus {
    code: i32,
    payload: *mut u8,
    payload_len: usize,
}

extern "C" {
    fn op_auth_abi_version() -> u32;
    fn op_auth_runtime_init(config_json: *const u8, len: usize) -> bool;
    fn op_auth_restore() -> i32;
    fn op_auth_login_begin() -> u64;
    fn op_auth_poll(handle: u64, out: *mut OpAuthStatus) -> i32;
    fn op_auth_cancel(handle: u64);
    fn op_auth_sign_out();
    fn op_auth_string_free(ptr: *mut u8, len: usize);
}

pub(crate) fn available() -> bool {
    static ABI_OK: OnceLock<bool> = OnceLock::new();
    *ABI_OK.get_or_init(|| {
        let version = linked_abi_version();
        let supported = supports_base_abi(version);
        if !supported {
            eprintln!(
                "op-auth-bridge: prebuilt library ABI {version} is outside \
                 supported range {REQUIRED_ABI_VERSION}..={MAX_SUPPORTED_ABI_VERSION}; \
                 account features disabled"
            );
        }
        supported
    })
}

fn linked_abi_version() -> u32 {
    static ABI_VERSION: OnceLock<u32> = OnceLock::new();
    *ABI_VERSION.get_or_init(|| {
        // SAFETY: no arguments; the symbol exists whenever this module compiles.
        unsafe { op_auth_abi_version() }
    })
}

pub(crate) fn collab_ticket_provider() -> &'static dyn CollabTicketProvider {
    if !available() {
        return &UNAVAILABLE_COLLAB_TICKET_PROVIDER;
    }
    #[cfg(op_auth_collab_ticket_prebuilt)]
    {
        if linked_abi_version() >= crate::COLLAB_TICKET_ABI_VERSION {
            return crate::real_collab_ticket::provider();
        }
        &UNAVAILABLE_COLLAB_TICKET_PROVIDER
    }
    #[cfg(not(op_auth_collab_ticket_prebuilt))]
    {
        &UNAVAILABLE_COLLAB_TICKET_PROVIDER
    }
}

pub(crate) fn init(config: &AuthInitConfig) -> bool {
    if !available() {
        return false;
    }
    let json = serde_json::json!({
        "base_url": config.base_url,
        "storage_dir": config.storage_dir.to_string_lossy(),
        "device_name": config.device_name,
        "platform": crate::platform_id(),
        "app_version": config.app_version,
    })
    .to_string();
    // SAFETY: pointer/length describe the live `json` buffer.
    unsafe { op_auth_runtime_init(json.as_ptr(), json.len()) }
}

pub(crate) fn restore() -> bool {
    if !available() {
        return false;
    }
    // SAFETY: no pointer arguments.
    unsafe { op_auth_restore() == 1 }
}

pub(crate) fn login_begin() -> u64 {
    if !available() {
        return 0;
    }
    // SAFETY: no pointer arguments.
    unsafe { op_auth_login_begin() }
}

pub(crate) fn poll(handle: u64) -> AuthStatus {
    if !available() {
        return AuthStatus::Idle;
    }
    let mut raw = OpAuthStatus {
        code: 0,
        payload: std::ptr::null_mut(),
        payload_len: 0,
    };
    // SAFETY: `raw` is a valid out-pointer for the duration of the call.
    let code = unsafe { op_auth_poll(handle, &mut raw) };
    let payload = if raw.payload.is_null() {
        None
    } else {
        // SAFETY: the library handed us `payload_len` readable bytes that we
        // free exactly once after copying.
        let bytes = unsafe { std::slice::from_raw_parts(raw.payload, raw.payload_len) };
        let text = String::from_utf8_lossy(bytes).into_owned();
        unsafe { op_auth_string_free(raw.payload, raw.payload_len) };
        Some(text)
    };
    AuthStatus::decode(code, payload.as_deref())
}

pub(crate) fn cancel(handle: u64) {
    if available() {
        // SAFETY: no pointer arguments.
        unsafe { op_auth_cancel(handle) };
    }
}

pub(crate) fn sign_out() {
    if available() {
        // SAFETY: no pointer arguments.
        unsafe { op_auth_sign_out() };
    }
}
