use std::{
    fmt,
    io::Read as _,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use op_auth_bridge::{OpaqueCollabTicket, MAX_COLLAB_TICKET_BYTES};
use op_collab_relay_protocol::{RelayLocatorVerifier, MAX_INVITE_CHARS};
use reqwest::{
    blocking::Client,
    header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    redirect::Policy,
    StatusCode,
};
#[cfg(any(test, debug_assertions))]
use url::Host;
use url::Url;
use zeroize::Zeroizing;

use crate::{
    OwnerPublishDraft, RelayLocatorIssueError, SignedInviteResponse, SignedLocatorResponse,
};

pub const RELAY_LOCATOR_PUBLISH_PATH: &str = "/v1/locator";
pub const OWNER_PUBLISH_CONTENT_TYPE: &str = "application/vnd.openpencil.relay-owner-publish-v1";
pub const SIGNED_LOCATOR_CONTENT_TYPE: &str = "application/vnd.openpencil.relay-locator-v1";
pub const CONTROL_PLANE_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const CONTROL_PLANE_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
pub const MAX_PUBLISH_AUTHORIZATION_BYTES: usize = 7 + MAX_COLLAB_TICKET_BYTES;

pub struct RelayLocatorHttpClient<V> {
    client: Client,
    endpoint: Url,
    verifier: V,
}

impl<V: RelayLocatorVerifier> RelayLocatorHttpClient<V> {
    pub fn new(endpoint: &str, verifier: V) -> Result<Self, RelayLocatorIssueError> {
        let endpoint = parse_endpoint(endpoint)?;
        if endpoint.scheme() != "https" {
            return Err(RelayLocatorIssueError::InsecureControlPlaneEndpoint);
        }
        Self::build(endpoint, verifier, true)
    }

    #[cfg(any(test, debug_assertions))]
    pub fn new_loopback_http_for_development(
        endpoint: &str,
        verifier: V,
        explicitly_enabled: bool,
    ) -> Result<Self, RelayLocatorIssueError> {
        if !explicitly_enabled {
            return Err(RelayLocatorIssueError::InsecureControlPlaneEndpoint);
        }
        let endpoint = parse_endpoint(endpoint)?;
        let numeric_loopback = match endpoint.host() {
            Some(Host::Ipv4(address)) => address.is_loopback(),
            Some(Host::Ipv6(address)) => address.is_loopback(),
            _ => false,
        };
        if endpoint.scheme() != "http" || !numeric_loopback {
            return Err(RelayLocatorIssueError::InsecureControlPlaneEndpoint);
        }
        Self::build(endpoint, verifier, false)
    }

    pub fn publish(
        &self,
        draft: OwnerPublishDraft,
        ticket: &OpaqueCollabTicket,
    ) -> Result<SignedInviteResponse, RelayLocatorIssueError> {
        let request_body = draft.request().encode_binary();
        let authorization = authorization_header(ticket.expose())?;
        let response = self
            .client
            .post(self.endpoint.clone())
            .header(CONTENT_TYPE, OWNER_PUBLISH_CONTENT_TYPE)
            .header(ACCEPT, SIGNED_LOCATOR_CONTENT_TYPE)
            .header(AUTHORIZATION, authorization)
            .body(request_body.to_vec())
            .send()
            .map_err(|_| RelayLocatorIssueError::PublishTransportUnavailable)?;
        publish_status(response.status())?;
        let content_type_matches = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value == SIGNED_LOCATOR_CONTENT_TYPE);
        if !content_type_matches {
            return Err(RelayLocatorIssueError::PublishResponseRejected);
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_INVITE_CHARS as u64)
        {
            return Err(RelayLocatorIssueError::PublishResponseTooLarge {
                maximum: MAX_INVITE_CHARS,
            });
        }
        let mut body = Vec::with_capacity(MAX_INVITE_CHARS);
        response
            .take((MAX_INVITE_CHARS + 1) as u64)
            .read_to_end(&mut body)
            .map_err(|_| RelayLocatorIssueError::PublishTransportUnavailable)?;
        if body.len() > MAX_INVITE_CHARS {
            return Err(RelayLocatorIssueError::PublishResponseTooLarge {
                maximum: MAX_INVITE_CHARS,
            });
        }
        let encoded = std::str::from_utf8(&body)
            .map_err(|_| RelayLocatorIssueError::PublishResponseRejected)?;
        let locator = SignedLocatorResponse::decode(encoded)
            .map_err(|_| RelayLocatorIssueError::PublishResponseRejected)?;
        draft.complete(locator, &self.verifier, unix_now()?)
    }

    fn build(endpoint: Url, verifier: V, https_only: bool) -> Result<Self, RelayLocatorIssueError> {
        let client = Client::builder()
            .use_rustls_tls()
            .redirect(Policy::none())
            .no_proxy()
            .connect_timeout(CONTROL_PLANE_CONNECT_TIMEOUT)
            .timeout(CONTROL_PLANE_REQUEST_TIMEOUT)
            .https_only(https_only)
            .build()
            .map_err(|_| RelayLocatorIssueError::PublishTransportUnavailable)?;
        Ok(Self {
            client,
            endpoint,
            verifier,
        })
    }
}

impl<V> RelayLocatorHttpClient<V> {
    /// Internal accessors for the pairing client arms, which reuse this
    /// client's hardened builder and validated endpoint host.
    pub(crate) fn http_client(&self) -> &Client {
        &self.client
    }

    pub(crate) fn endpoint_clone(&self) -> Url {
        self.endpoint.clone()
    }
}

impl<V> fmt::Debug for RelayLocatorHttpClient<V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RelayLocatorHttpClient")
            .field("endpoint", &"[REDACTED]")
            .field("verifier", &"[PINNED]")
            .finish()
    }
}

fn parse_endpoint(endpoint: &str) -> Result<Url, RelayLocatorIssueError> {
    let endpoint =
        Url::parse(endpoint).map_err(|_| RelayLocatorIssueError::InvalidControlPlaneEndpoint)?;
    if endpoint.host().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || endpoint.path() != RELAY_LOCATOR_PUBLISH_PATH
    {
        return Err(RelayLocatorIssueError::InvalidControlPlaneEndpoint);
    }
    Ok(endpoint)
}

/// Classify a control-plane publish status.
///
/// Every non-OK status used to collapse into `PublishResponseRejected`, which
/// left the desktop unable to tell an expired ticket from an overloaded
/// service — both surfaced as "the public relay is temporarily unavailable",
/// and only one of them is temporary.
pub(crate) fn publish_status(status: StatusCode) -> Result<(), RelayLocatorIssueError> {
    match status {
        StatusCode::OK => Ok(()),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            Err(RelayLocatorIssueError::PublishUnauthorized)
        }
        StatusCode::TOO_MANY_REQUESTS => Err(RelayLocatorIssueError::PublishRateLimited),
        _ => Err(RelayLocatorIssueError::PublishResponseRejected),
    }
}

pub(crate) fn authorization_header(ticket: &[u8]) -> Result<HeaderValue, RelayLocatorIssueError> {
    if ticket.is_empty() || ticket.len() > MAX_COLLAB_TICKET_BYTES || !valid_b64token(ticket) {
        return Err(RelayLocatorIssueError::InvalidPublishCredential);
    }
    let mut value = Zeroizing::new(Vec::with_capacity(7 + ticket.len()));
    value.extend_from_slice(b"Bearer ");
    value.extend_from_slice(ticket);
    let mut header = HeaderValue::from_bytes(&value)
        .map_err(|_| RelayLocatorIssueError::InvalidPublishCredential)?;
    header.set_sensitive(true);
    Ok(header)
}

fn valid_b64token(value: &[u8]) -> bool {
    let mut padding = false;
    for byte in value {
        if *byte == b'=' {
            padding = true;
        } else if padding
            || !(byte.is_ascii_alphanumeric()
                || matches!(*byte, b'-' | b'.' | b'_' | b'~' | b'+' | b'/'))
        {
            return false;
        }
    }
    value.first() != Some(&b'=')
}

fn unix_now() -> Result<u64, RelayLocatorIssueError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| RelayLocatorIssueError::ClockBeforeUnixEpoch)?
        .as_secs();
    if now == 0 {
        return Err(RelayLocatorIssueError::ZeroCurrentTime);
    }
    Ok(now)
}
