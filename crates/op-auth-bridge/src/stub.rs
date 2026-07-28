//! Fallback backend for builds without a prebuilt auth library: every
//! operation is a no-op and `available()` is false, so hosts keep the
//! account UI hidden while the build stays green on every platform.

use crate::status::AuthStatus;
use crate::{AuthInitConfig, CollabTicketProvider, UNAVAILABLE_COLLAB_TICKET_PROVIDER};

pub(crate) fn available() -> bool {
    false
}

pub(crate) fn collab_ticket_provider() -> &'static dyn CollabTicketProvider {
    &UNAVAILABLE_COLLAB_TICKET_PROVIDER
}

pub(crate) fn init(_config: &AuthInitConfig) -> bool {
    false
}

pub(crate) fn restore() -> bool {
    false
}

pub(crate) fn login_begin() -> u64 {
    0
}

pub(crate) fn poll(_handle: u64) -> AuthStatus {
    AuthStatus::Idle
}

pub(crate) fn cancel(_handle: u64) {}

pub(crate) fn sign_out() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn stub_is_inert() {
        assert!(!available());
        assert!(!init(&AuthInitConfig {
            base_url: "https://sso.example".to_string(),
            storage_dir: PathBuf::from("/tmp"),
            device_name: "test".to_string(),
            app_version: "0.0.0".to_string(),
        }));
        assert!(!restore());
        assert_eq!(login_begin(), 0);
        assert_eq!(poll(0), AuthStatus::Idle);
        cancel(1);
        sign_out();
    }
}
