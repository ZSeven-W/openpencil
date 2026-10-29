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
        let supported = supports_linked_abi(version);
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

fn supports_linked_abi(version: u32) -> bool {
    // Both signed and development archives must match their validated declaration.
    if option_env!("OP_AUTH_PREBUILT_ABI_VERSION").and_then(|declared| declared.parse::<u32>().ok())
        != Some(version)
    {
        return false;
    }
    supports_base_abi(version)
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
        let linked = linked_abi_version();
        #[cfg(op_auth_collab_relay_token_prebuilt)]
        if linked >= crate::COLLAB_RELAY_TOKEN_ABI_VERSION {
            return crate::real_collab_ticket::provider_with_relay_token();
        }
        if linked >= crate::COLLAB_TICKET_ABI_VERSION {
            return crate::real_collab_ticket::provider();
        }
        &UNAVAILABLE_COLLAB_TICKET_PROVIDER
    }
    #[cfg(not(op_auth_collab_ticket_prebuilt))]
    {
        &UNAVAILABLE_COLLAB_TICKET_PROVIDER
    }
}

pub(crate) fn hosted_access_provider() -> &'static dyn crate::HostedAccessProvider {
    #[cfg(op_auth_hosted_access_prebuilt)]
    if available() && linked_abi_version() == crate::HOSTED_ACCESS_ABI_VERSION {
        return crate::real_hosted_access::provider();
    }
    &crate::UNAVAILABLE_HOSTED_ACCESS_PROVIDER
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
        "hosted_product": "openpencil",
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

#[cfg(test)]
mod tests {
    use super::supports_linked_abi;

    #[test]
    fn runtime_abi_handshake_rejects_unknown_and_misdeclared_archives() {
        assert!(!supports_linked_abi(0));
        assert!(!supports_linked_abi(5));
        let declared = env!("OP_AUTH_PREBUILT_ABI_VERSION").parse::<u32>().unwrap();
        for version in 1..=4 {
            assert_eq!(supports_linked_abi(version), version == declared);
        }
    }
}
