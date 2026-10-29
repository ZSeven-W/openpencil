use crate::{HostedAccess, HostedAccessError, MAX_HOSTED_ACCESS_BYTES};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use url::Url;
use zeroize::Zeroizing;

/// Borrow all strings so malformed input cannot leave deserialized secrets in
/// ordinary heap strings. The input owner zeroizes the backing bytes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessWire<'a> {
    version: u32,
    token: &'a str,
    token_id: &'a str,
    token_type: &'a str,
    issuer: &'a str,
    subject: &'a str,
    // Mandatory for this consumer: an older signed runtime must not silently
    // fall back to shared credentials when product deletion is enforced.
    product: &'a str,
    audience: &'a str,
    scopes: [&'a str; 2],
    issued_at_unix: u64,
    expires_at_unix: u64,
    gateway_base_url: &'a str,
}

pub(crate) fn decode(bytes: &[u8], now: u64) -> Result<HostedAccess, HostedAccessError> {
    // This ASCII wire format has no fields requiring JSON escapes. Reject
    // escapes before serde can allocate an unzeroized string scratch buffer.
    if bytes.is_empty()
        || bytes.len() > MAX_HOSTED_ACCESS_BYTES
        || !bytes.is_ascii()
        || bytes.contains(&b'\\')
    {
        return Err(HostedAccessError::InvalidPayload);
    }
    let wire: AccessWire<'_> =
        serde_json::from_slice(bytes).map_err(|_| HostedAccessError::InvalidPayload)?;
    let lifetime = wire.expires_at_unix.checked_sub(wire.issued_at_unix);
    if wire.version != 1
        || wire.product != "openpencil"
        || wire.token_type != "Bearer"
        || wire.audience != "llm-gateway"
        || wire.scopes != ["llm:models", "llm:invoke"]
        || !canonical_bearer(wire.token)
        || !lower_hex(wire.token_id, 32)
        || !canonical_subject(wire.subject)
        || !canonical_issuer(wire.issuer)
        || !canonical_gateway(wire.gateway_base_url)
        || wire.issued_at_unix == 0
        || wire.expires_at_unix > i64::MAX as u64
        || !lifetime.is_some_and(|ttl| (60..=300).contains(&ttl))
        || wire.issued_at_unix > now.saturating_add(30)
    {
        return Err(HostedAccessError::InvalidPayload);
    }
    if wire.expires_at_unix <= now {
        return Err(HostedAccessError::Expired);
    }
    Ok(HostedAccess {
        subject: wire.subject.to_owned(),
        issuer: wire.issuer.to_owned(),
        gateway_base_url: wire.gateway_base_url.to_owned(),
        expires_at_unix: wire.expires_at_unix,
        bearer: Zeroizing::new(wire.token.to_owned()),
    })
}

fn canonical_bearer(token: &str) -> bool {
    let Some(encoded) = token.strip_prefix("opllm_") else {
        return false;
    };
    if encoded.len() != 43 {
        return false;
    }
    let mut decoded = Zeroizing::new([0_u8; 32]);
    matches!(URL_SAFE_NO_PAD.decode_slice(encoded, &mut *decoded), Ok(32))
}

fn lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_subject(value: &str) -> bool {
    value.len() == 36
        && value != "00000000-0000-0000-0000-000000000000"
        && value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

fn https_url(value: &str) -> Option<Url> {
    if value.is_empty() || value.len() > 2048 || !value.is_ascii() {
        return None;
    }
    let parsed = Url::parse(value).ok()?;
    (parsed.scheme() == "https"
        && parsed.host_str().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none())
    .then_some(parsed)
}

fn canonical_issuer(value: &str) -> bool {
    https_url(value).is_some_and(|url| url.origin().ascii_serialization() == value)
}

fn canonical_gateway(value: &str) -> bool {
    https_url(value).is_some_and(|url| {
        url.as_str() == value
            && url.path().ends_with("/v1")
            && url.path().split('/').skip(1).all(|part| {
                !part.is_empty()
                    && !matches!(part, "." | "..")
                    && part.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~')
                    })
            })
    })
}
