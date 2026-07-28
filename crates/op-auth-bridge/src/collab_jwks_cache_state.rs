use std::time::{Duration, Instant};

use crate::{CollabJwks, CollabJwksError, CollabUnionPolicy};

#[derive(Default)]
pub(super) struct CacheState {
    pub(super) keyset: Option<CollabJwks>,
    pub(super) policy: Option<CollabUnionPolicy>,
    pub(super) etag: Option<String>,
    pub(super) fresh_until: Option<Instant>,
    pub(super) last_refresh_attempt: Option<Instant>,
    pub(super) last_successful_refresh: Option<Instant>,
    pub(super) last_unknown_kid_refresh: Option<Instant>,
}

pub(super) fn validate_etag(
    etag: Option<String>,
    maximum: usize,
) -> Result<Option<String>, CollabJwksError> {
    if etag
        .as_ref()
        .is_some_and(|value| value.len() > maximum || !valid_http_etag(value))
    {
        return Err(CollabJwksError::InvalidEtag { maximum });
    }
    Ok(etag)
}

fn valid_http_etag(value: &str) -> bool {
    let value = value.strip_prefix("W/").unwrap_or(value);
    let bytes = value.as_bytes();
    bytes.len() >= 2
        && bytes.first() == Some(&b'"')
        && bytes.last() == Some(&b'"')
        && bytes[1..bytes.len() - 1]
            .iter()
            .all(|byte| *byte == 0x21 || (0x23..=0x7e).contains(byte))
}

pub(super) fn fresh_until(now: Instant, response_age: u64, maximum_age: u64) -> Option<Instant> {
    now.checked_add(Duration::from_secs(response_age.min(maximum_age)))
}

pub(super) fn recently(previous: Option<Instant>, now: Instant, interval_seconds: u64) -> bool {
    previous.is_some_and(|previous| {
        now.checked_duration_since(previous)
            .is_none_or(|elapsed| elapsed < Duration::from_secs(interval_seconds))
    })
}
