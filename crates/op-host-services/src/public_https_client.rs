//! Public-only HTTPS dialing for small, unauthenticated assets.
//!
//! Callers still own response limits and redirect policy. This module owns
//! the security-sensitive connect step: every hostname is resolved, every
//! address is screened, proxies are disabled, and the client is pinned to the
//! screened addresses so DNS rebinding cannot redirect the socket later.

use std::fmt;

/// A deliberately low-detail failure. Public asset URLs are untrusted display
/// metadata, so neither the URL nor resolver/HTTP diagnostics should reach
/// ambient logs through this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicHttpsClientError {
    UrlNotAllowed,
    DialRejected,
}

impl fmt::Display for PublicHttpsClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UrlNotAllowed => formatter.write_str("public asset URL is not allowed"),
            Self::DialRejected => formatter.write_str("public asset destination was rejected"),
        }
    }
}

impl std::error::Error for PublicHttpsClientError {}

/// Build a no-redirect, no-proxy client pinned to the URL's screened public
/// addresses. The URL must already be the exact request/redirect target.
pub async fn public_https_client(
    url: &reqwest::Url,
) -> Result<reqwest::Client, PublicHttpsClientError> {
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port() == Some(0)
    {
        return Err(PublicHttpsClientError::UrlNotAllowed);
    }
    crate::provider_dial::client_for(
        crate::provider_dial::EndpointDialPolicy::PublicOnly,
        url.as_str(),
    )
    .await
    .map_err(|_| PublicHttpsClientError::DialRejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(url: &str) -> Result<reqwest::Client, PublicHttpsClientError> {
        let url = reqwest::Url::parse(url).expect("test URL parses");
        crate::chat_runtime::block_on_anywhere(public_https_client(&url))
    }

    #[test]
    fn rejects_non_https_credentials_and_reserved_literals() {
        assert_eq!(
            build("http://93.184.216.34/avatar.png").unwrap_err(),
            PublicHttpsClientError::UrlNotAllowed
        );
        assert_eq!(
            build("https://user@93.184.216.34/avatar.png").unwrap_err(),
            PublicHttpsClientError::UrlNotAllowed
        );
        assert_eq!(
            build("https://127.0.0.1/avatar.png").unwrap_err(),
            PublicHttpsClientError::DialRejected
        );
        for reserved in [
            "https://10.0.0.1/avatar.png",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/avatar.png",
            "https://[fe80::1]/avatar.png",
            "https://[::ffff:127.0.0.1]/avatar.png",
        ] {
            assert_eq!(
                build(reserved).unwrap_err(),
                PublicHttpsClientError::DialRejected,
                "{reserved}"
            );
        }
    }

    #[test]
    fn public_literal_builds_a_pinned_client_without_connecting() {
        assert!(build("https://93.184.216.34/avatar.png").is_ok());
    }
}
