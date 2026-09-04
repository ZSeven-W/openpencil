//! SSRF-safe fetch and validation for authenticated profile avatars.

use hmac::{Hmac, Mac};
use op_editor_ui::collab_avatar_runtime::{
    MAX_AVATAR_ENCODED_BYTES, MAX_AVATAR_SOURCE_EDGE_PX, MAX_AVATAR_SOURCE_PIXELS,
};
use reqwest::header::{ACCEPT, LOCATION};
use sha2::Sha256;
use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

const MAX_AVATAR_URL_BYTES: usize = 2_048;
const MAX_REDIRECTS: usize = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
static AVATAR_REVISION_KEY: OnceLock<Option<[u8; 32]>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileAvatarFetchError {
    UrlNotAllowed,
    DialRejected,
    RequestFailed,
    TimedOut,
    HttpStatus,
    RedirectInvalid,
    TooManyRedirects,
    TooLarge,
    EmptyBody,
    InvalidImage,
    RevisionUnavailable,
}

impl fmt::Display for ProfileAvatarFetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UrlNotAllowed => "avatar URL is not allowed",
            Self::DialRejected => "avatar host is not publicly routable",
            Self::RequestFailed => "avatar request failed",
            Self::TimedOut => "avatar request timed out",
            Self::HttpStatus => "avatar server returned an error status",
            Self::RedirectInvalid => "avatar redirect is invalid",
            Self::TooManyRedirects => "avatar request redirected too many times",
            Self::TooLarge => "avatar image is too large",
            Self::EmptyBody => "avatar response is empty",
            Self::InvalidImage => "avatar response is not a supported image",
            Self::RevisionUnavailable => "avatar revision key is unavailable",
        })
    }
}

impl std::error::Error for ProfileAvatarFetchError {}

/// Stable opaque identity for one validated avatar URL. The raw URL may carry
/// signed CDN query parameters and must not be returned to the browser.
pub fn profile_avatar_revision(url: &str) -> Result<String, ProfileAvatarFetchError> {
    let url = parse_avatar_url(url)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(avatar_revision_key()?)
        .map_err(|_| ProfileAvatarFetchError::RevisionUnavailable)?;
    mac.update(url.as_str().as_bytes());
    Ok(format!("{:x}", mac.finalize().into_bytes()))
}

/// Blocking entry point for desktop workers and serve-web connection threads.
pub fn fetch_profile_avatar_blocking(url: &str) -> Result<Vec<u8>, ProfileAvatarFetchError> {
    crate::chat_runtime::block_on_anywhere(fetch_profile_avatar(url, false))
}

/// Blocking fetch for the locally authenticated account's profile image.
///
/// Unlike remote collaboration profiles, this source may traverse a
/// Clash-style fake-IP TUN when the hostname resolves exclusively into RFC
/// 2544 space.
pub fn fetch_account_avatar_blocking(url: &str) -> Result<Vec<u8>, ProfileAvatarFetchError> {
    crate::chat_runtime::block_on_anywhere(fetch_profile_avatar(url, true))
}

async fn fetch_profile_avatar(
    url: &str,
    allow_account_tunnel: bool,
) -> Result<Vec<u8>, ProfileAvatarFetchError> {
    let mut url = parse_avatar_url(url)?;
    for redirect_count in 0..=MAX_REDIRECTS {
        let client = if allow_account_tunnel {
            with_timeout(
                REQUEST_TIMEOUT,
                crate::public_https_client::tunnel_compatible_account_asset_client(&url),
            )
            .await?
        } else {
            with_timeout(
                REQUEST_TIMEOUT,
                crate::public_https_client::public_https_client(&url),
            )
            .await?
        }
        .map_err(|_| ProfileAvatarFetchError::DialRejected)?;
        let mut response = client
            .get(url.clone())
            .header(ACCEPT, "image/webp,image/png,image/jpeg,image/gif")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|_| ProfileAvatarFetchError::RequestFailed)?;

        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                return Err(ProfileAvatarFetchError::TooManyRedirects);
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(ProfileAvatarFetchError::RedirectInvalid)?;
            url = parse_avatar_url(
                url.join(location)
                    .map_err(|_| ProfileAvatarFetchError::RedirectInvalid)?
                    .as_str(),
            )?;
            continue;
        }
        if !response.status().is_success() {
            return Err(ProfileAvatarFetchError::HttpStatus);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_AVATAR_ENCODED_BYTES as u64)
        {
            return Err(ProfileAvatarFetchError::TooLarge);
        }
        let mut bytes = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(0)
                .min(MAX_AVATAR_ENCODED_BYTES as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ProfileAvatarFetchError::RequestFailed)?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_AVATAR_ENCODED_BYTES {
                return Err(ProfileAvatarFetchError::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        validate_profile_avatar_bytes(&bytes)?;
        return Ok(bytes);
    }
    Err(ProfileAvatarFetchError::TooManyRedirects)
}

fn validate_profile_avatar_bytes(bytes: &[u8]) -> Result<(), ProfileAvatarFetchError> {
    if bytes.is_empty() {
        return Err(ProfileAvatarFetchError::EmptyBody);
    }
    if bytes.len() > MAX_AVATAR_ENCODED_BYTES {
        return Err(ProfileAvatarFetchError::TooLarge);
    }
    let (width, height) = op_editor_ui::image_runtime::encoded_image_dimensions(bytes)
        .ok_or(ProfileAvatarFetchError::InvalidImage)?;
    if width > MAX_AVATAR_SOURCE_EDGE_PX
        || height > MAX_AVATAR_SOURCE_EDGE_PX
        || u64::from(width) * u64::from(height) > MAX_AVATAR_SOURCE_PIXELS
    {
        return Err(ProfileAvatarFetchError::TooLarge);
    }
    Ok(())
}

async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, ProfileAvatarFetchError>
where
    F: std::future::Future<Output = T>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| ProfileAvatarFetchError::TimedOut)
}

fn parse_avatar_url(value: &str) -> Result<reqwest::Url, ProfileAvatarFetchError> {
    if value.is_empty()
        || value.len() > MAX_AVATAR_URL_BYTES
        || value.chars().any(char::is_whitespace)
    {
        return Err(ProfileAvatarFetchError::UrlNotAllowed);
    }
    let url = reqwest::Url::parse(value).map_err(|_| ProfileAvatarFetchError::UrlNotAllowed)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.port() == Some(0)
    {
        return Err(ProfileAvatarFetchError::UrlNotAllowed);
    }
    Ok(url)
}

fn avatar_revision_key() -> Result<&'static [u8; 32], ProfileAvatarFetchError> {
    AVATAR_REVISION_KEY
        .get_or_init(|| {
            let mut key = [0_u8; 32];
            getrandom::fill(&mut key).ok().map(|()| key)
        })
        .as_ref()
        .ok_or(ProfileAvatarFetchError::RevisionUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes[8..12].copy_from_slice(&13_u32.to_be_bytes());
        bytes[12..16].copy_from_slice(b"IHDR");
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        bytes
    }

    #[test]
    fn revision_is_stable_without_exposing_the_url() {
        let url = "https://cdn.example/avatar.png?signature=secret";
        let revision = profile_avatar_revision(url).unwrap();
        assert_eq!(revision, profile_avatar_revision(url).unwrap());
        assert_eq!(revision.len(), 64);
        assert!(!revision.contains("secret"));
    }

    #[test]
    fn url_and_ssrf_guards_reject_unsafe_targets() {
        for url in [
            "http://cdn.example/avatar.png",
            "https://user@cdn.example/avatar.png",
            "https://cdn.example/avatar.png#fragment",
            "https://cdn.example:0/avatar.png",
        ] {
            assert_eq!(
                profile_avatar_revision(url),
                Err(ProfileAvatarFetchError::UrlNotAllowed)
            );
        }
        for url in [
            "https://127.0.0.1/avatar.png",
            "https://10.0.0.1/avatar.png",
            "https://169.254.169.254/latest/meta-data",
            "https://[::1]/avatar.png",
            "https://[fe80::1]/avatar.png",
        ] {
            assert_eq!(
                fetch_profile_avatar_blocking(url),
                Err(ProfileAvatarFetchError::DialRejected),
                "{url}"
            );
        }
    }

    #[test]
    fn encoded_image_dimensions_are_bounded_before_proxying() {
        assert_eq!(validate_profile_avatar_bytes(&png_header(16, 16)), Ok(()));
        assert_eq!(
            validate_profile_avatar_bytes(&png_header(MAX_AVATAR_SOURCE_EDGE_PX + 1, 1)),
            Err(ProfileAvatarFetchError::TooLarge)
        );
        assert_eq!(
            validate_profile_avatar_bytes(b"not an image"),
            Err(ProfileAvatarFetchError::InvalidImage)
        );
    }

    #[test]
    fn resolver_deadline_cancels_a_hanging_dial_future() {
        let result = crate::chat_runtime::block_on_anywhere(with_timeout(
            Duration::from_millis(1),
            std::future::pending::<()>(),
        ));
        assert_eq!(result, Err(ProfileAvatarFetchError::TimedOut));
    }
}
