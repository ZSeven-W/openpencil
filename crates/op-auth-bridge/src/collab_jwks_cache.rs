//! Bounded, transport-neutral JWKS cache with rotation-aware refresh.

use std::fmt;
use std::sync::{Mutex, TryLockError};
use std::time::{Duration, Instant};

use crate::{
    collab_claims::valid_jwks_endpoint, CollabJwks, CollabJwksError, CollabJwksFetchError,
    CollabUnionPolicy, CollabUnionPolicyError, CollabVerifierConfigError,
    DEFAULT_MAX_COLLAB_JWKS_BYTES, DEFAULT_MAX_COLLAB_JWKS_KEYS, HARD_MAX_COLLAB_JWKS_BYTES,
    HARD_MAX_COLLAB_JWKS_KEYS,
};

#[path = "collab_jwks_cache_state.rs"]
mod cache_state;
use cache_state::{fresh_until, recently, validate_etag, CacheState};

pub const DEFAULT_MAX_COLLAB_JWKS_ETAG_BYTES: usize = 256;
pub const DEFAULT_MAX_COLLAB_JWKS_AGE_SECONDS: u64 = 5 * 60;
pub const DEFAULT_UNKNOWN_KID_REFRESH_SECONDS: u64 = 30;
pub const DEFAULT_FAILED_REFRESH_BACKOFF_SECONDS: u64 = 1;

const HARD_MAX_COLLAB_JWKS_ETAG_BYTES: usize = 1_024;
const HARD_MAX_COLLAB_JWKS_AGE_SECONDS: u64 = 60 * 60;
const HARD_MAX_UNKNOWN_KID_REFRESH_SECONDS: u64 = 15 * 60;
const HARD_MAX_FAILED_REFRESH_BACKOFF_SECONDS: u64 = 60;
const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Resource and refresh limits enforced by [`CollabJwksCache`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollabJwksCacheLimits {
    pub max_body_bytes: usize,
    pub max_keys: usize,
    pub max_etag_bytes: usize,
    pub max_age_seconds: u64,
    pub unknown_kid_refresh_seconds: u64,
    pub failed_refresh_backoff_seconds: u64,
}

impl Default for CollabJwksCacheLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: DEFAULT_MAX_COLLAB_JWKS_BYTES,
            max_keys: DEFAULT_MAX_COLLAB_JWKS_KEYS,
            max_etag_bytes: DEFAULT_MAX_COLLAB_JWKS_ETAG_BYTES,
            max_age_seconds: DEFAULT_MAX_COLLAB_JWKS_AGE_SECONDS,
            unknown_kid_refresh_seconds: DEFAULT_UNKNOWN_KID_REFRESH_SECONDS,
            failed_refresh_backoff_seconds: DEFAULT_FAILED_REFRESH_BACKOFF_SECONDS,
        }
    }
}

impl CollabJwksCacheLimits {
    pub fn validate(self) -> Result<Self, CollabVerifierConfigError> {
        if self.max_body_bytes == 0
            || self.max_body_bytes > HARD_MAX_COLLAB_JWKS_BYTES
            || self.max_keys == 0
            || self.max_keys > HARD_MAX_COLLAB_JWKS_KEYS
            || self.max_etag_bytes == 0
            || self.max_etag_bytes > HARD_MAX_COLLAB_JWKS_ETAG_BYTES
            || self.max_age_seconds == 0
            || self.max_age_seconds > HARD_MAX_COLLAB_JWKS_AGE_SECONDS
            || self.unknown_kid_refresh_seconds == 0
            || self.unknown_kid_refresh_seconds > HARD_MAX_UNKNOWN_KID_REFRESH_SECONDS
            || self.failed_refresh_backoff_seconds == 0
            || self.failed_refresh_backoff_seconds > HARD_MAX_FAILED_REFRESH_BACKOFF_SECONDS
        {
            return Err(CollabVerifierConfigError::InvalidCacheLimits);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CollabJwksFetchRequest<'a> {
    pub endpoint: &'a str,
    pub etag: Option<&'a str>,
    pub maximum_body_bytes: usize,
}

pub enum CollabJwksFetchResponse {
    Modified {
        body: Vec<u8>,
        etag: Option<String>,
        max_age_seconds: u64,
    },
    NotModified {
        etag: Option<String>,
        max_age_seconds: u64,
    },
}

impl fmt::Debug for CollabJwksFetchResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Modified {
                body,
                etag,
                max_age_seconds,
            } => formatter
                .debug_struct("Modified")
                .field("body_bytes", &body.len())
                .field("etag", &etag.as_ref().map(|_| "[REDACTED]"))
                .field("max_age_seconds", max_age_seconds)
                .finish(),
            Self::NotModified {
                etag,
                max_age_seconds,
            } => formatter
                .debug_struct("NotModified")
                .field("etag", &etag.as_ref().map(|_| "[REDACTED]"))
                .field("max_age_seconds", max_age_seconds)
                .finish(),
        }
    }
}

/// Synchronous transport for a pinned policy or legacy JWKS endpoint.
///
/// A native adapter must use normal certificate and hostname validation,
/// request exactly `request.endpoint`, disable redirects, authentication, and
/// cookies, accept only successful JSON or `304 Not Modified` responses, and
/// enforce the requested size while streaming rather than after allocating the
/// full body. Blocking transports must propagate `fetch_cancellable`.
pub trait CollabJwksFetcher: Send + Sync {
    fn fetch(
        &self,
        request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError>;

    fn fetch_cancellable(
        &self,
        request: CollabJwksFetchRequest<'_>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        if cancelled() {
            return Err(CollabJwksFetchError::Cancelled);
        }
        let response = self.fetch(request)?;
        if cancelled() {
            Err(CollabJwksFetchError::Cancelled)
        } else {
            Ok(response)
        }
    }
}

/// Thread-safe key cache backed by an injected pinned-endpoint fetcher.
pub struct CollabJwksCache<F> {
    endpoint: String,
    fetcher: F,
    limits: CollabJwksCacheLimits,
    source: CacheSource,
    state: Mutex<CacheState>,
}

#[derive(Clone, Debug)]
enum CacheSource {
    LegacyJwks,
    SignedPolicy { expected_issuer: String },
}

impl<F: CollabJwksFetcher> CollabJwksCache<F> {
    pub fn new(
        endpoint: impl Into<String>,
        fetcher: F,
        limits: CollabJwksCacheLimits,
    ) -> Result<Self, CollabVerifierConfigError> {
        let endpoint = endpoint.into();
        if !valid_jwks_endpoint(&endpoint) {
            return Err(CollabVerifierConfigError::InvalidJwksEndpoint);
        }
        Ok(Self {
            endpoint,
            fetcher,
            limits: limits.validate()?,
            source: CacheSource::LegacyJwks,
            state: Mutex::new(CacheState::default()),
        })
    }

    pub(crate) fn new_signed_policy(
        endpoint: impl Into<String>,
        expected_issuer: impl Into<String>,
        fetcher: F,
        limits: CollabJwksCacheLimits,
    ) -> Result<Self, CollabVerifierConfigError> {
        let endpoint = endpoint.into();
        if !valid_jwks_endpoint(&endpoint) {
            return Err(CollabVerifierConfigError::InvalidPolicyEndpoint);
        }
        Ok(Self {
            endpoint,
            fetcher,
            limits: limits.validate()?,
            source: CacheSource::SignedPolicy {
                expected_issuer: expected_issuer.into(),
            },
            state: Mutex::new(CacheState::default()),
        })
    }

    pub(crate) fn verification_key(
        &self,
        key_id: &str,
        now: Instant,
    ) -> Result<[u8; 32], CollabJwksError> {
        self.verification_key_at(key_id, now, None, &never_cancelled)
    }

    pub(crate) fn policy_verification_key(
        &self,
        key_id: &str,
        now: Instant,
        now_unix_seconds: u64,
    ) -> Result<[u8; 32], CollabJwksError> {
        self.verification_key_at(key_id, now, Some(now_unix_seconds), &never_cancelled)
    }

    fn verification_key_at(
        &self,
        key_id: &str,
        now: Instant,
        now_unix_seconds: Option<u64>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<[u8; 32], CollabJwksError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CollabJwksError::CacheUnavailable)?;
        self.verification_key_locked(&mut state, key_id, now, now_unix_seconds, cancelled)
    }

    pub(crate) fn verification_key_cancellable(
        &self,
        key_id: &str,
        now: Instant,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<[u8; 32], CollabJwksError> {
        self.verification_key_at_cancellable(key_id, now, None, cancelled)
    }

    pub(crate) fn policy_verification_key_cancellable(
        &self,
        key_id: &str,
        now: Instant,
        now_unix_seconds: u64,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<[u8; 32], CollabJwksError> {
        self.verification_key_at_cancellable(key_id, now, Some(now_unix_seconds), cancelled)
    }

    fn verification_key_at_cancellable(
        &self,
        key_id: &str,
        now: Instant,
        now_unix_seconds: Option<u64>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<[u8; 32], CollabJwksError> {
        ensure_not_cancelled(cancelled)?;
        let mut state = loop {
            ensure_not_cancelled(cancelled)?;
            match self.state.try_lock() {
                Ok(state) => break state,
                Err(TryLockError::Poisoned(_)) => {
                    return Err(CollabJwksError::CacheUnavailable);
                }
                Err(TryLockError::WouldBlock) => {
                    std::thread::sleep(CANCELLATION_POLL_INTERVAL);
                }
            }
        };
        self.verification_key_locked(&mut state, key_id, now, now_unix_seconds, cancelled)
    }

    fn verification_key_locked(
        &self,
        state: &mut CacheState,
        key_id: &str,
        now: Instant,
        now_unix_seconds: Option<u64>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<[u8; 32], CollabJwksError> {
        ensure_not_cancelled(cancelled)?;
        self.validate_clock_mode(now_unix_seconds)?;
        let previous_unknown_kid_refresh = state.last_unknown_kid_refresh;
        let mut unknown_kid_refresh_due = false;
        let policy_active = self.policy_active(state, now_unix_seconds).is_ok();
        let fresh = policy_active && state.fresh_until.is_some_and(|until| now < until);
        if fresh {
            if let Some(key) = self.cached_key(state, key_id, now_unix_seconds) {
                ensure_not_cancelled(cancelled)?;
                return Ok(key);
            }
            if recently(
                state.last_successful_refresh,
                now,
                self.limits.unknown_kid_refresh_seconds,
            ) || recently(
                state.last_unknown_kid_refresh,
                now,
                self.limits.unknown_kid_refresh_seconds,
            ) {
                ensure_not_cancelled(cancelled)?;
                return Err(CollabJwksError::UnknownKey);
            }
            unknown_kid_refresh_due = true;
        }

        if recently(
            state.last_refresh_attempt,
            now,
            self.limits.failed_refresh_backoff_seconds,
        ) {
            // A successful response may deliberately advertise max-age=0
            // while serving a bounded last-known-good keyset. Reuse an
            // already-known key during only that successful-attempt backoff;
            // a failed refresh leaves the timestamps unequal and continues
            // to fail closed.
            if state.last_refresh_attempt == state.last_successful_refresh {
                self.policy_active(state, now_unix_seconds)?;
                if let Some(key) = self.cached_key(state, key_id, now_unix_seconds) {
                    ensure_not_cancelled(cancelled)?;
                    return Ok(key);
                }
            }
            ensure_not_cancelled(cancelled)?;
            return Err(CollabJwksError::RefreshThrottled);
        }
        let previous_refresh_attempt = state.last_refresh_attempt;
        if unknown_kid_refresh_due {
            state.last_unknown_kid_refresh = Some(now);
        }
        state.last_refresh_attempt = Some(now);
        if let Err(error) = self.refresh_locked(state, now, now_unix_seconds, cancelled) {
            if matches!(
                error,
                CollabJwksError::Fetch(CollabJwksFetchError::Cancelled)
            ) {
                state.last_refresh_attempt = previous_refresh_attempt;
                state.last_unknown_kid_refresh = previous_unknown_kid_refresh;
            }
            return Err(error);
        }
        ensure_not_cancelled(cancelled)?;
        self.policy_active(state, now_unix_seconds)?;
        self.cached_key(state, key_id, now_unix_seconds)
            .ok_or(CollabJwksError::UnknownKey)
    }

    pub fn cached_key_count(&self) -> Result<usize, CollabJwksError> {
        let state = self
            .state
            .lock()
            .map_err(|_| CollabJwksError::CacheUnavailable)?;
        Ok(state.keyset.as_ref().map_or(0, CollabJwks::len))
    }

    fn refresh_locked(
        &self,
        state: &mut CacheState,
        now: Instant,
        now_unix_seconds: Option<u64>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), CollabJwksError> {
        ensure_not_cancelled(cancelled)?;
        let response = self.fetcher.fetch_cancellable(
            CollabJwksFetchRequest {
                endpoint: &self.endpoint,
                etag: state.etag.as_deref(),
                maximum_body_bytes: self.limits.max_body_bytes,
            },
            cancelled,
        )?;
        ensure_not_cancelled(cancelled)?;
        match response {
            CollabJwksFetchResponse::Modified {
                body,
                etag,
                max_age_seconds,
            } => {
                let etag = validate_etag(etag, self.limits.max_etag_bytes)?;
                let (keyset, policy) = match &self.source {
                    CacheSource::LegacyJwks => (
                        CollabJwks::from_json(
                            &body,
                            self.limits.max_body_bytes,
                            self.limits.max_keys,
                        )?,
                        None,
                    ),
                    CacheSource::SignedPolicy { expected_issuer } => {
                        let now_unix_seconds =
                            now_unix_seconds.ok_or(CollabUnionPolicyError::Inactive)?;
                        let policy = CollabUnionPolicy::from_json(
                            &body,
                            self.limits.max_body_bytes,
                            expected_issuer,
                            now_unix_seconds,
                        )?;
                        if policy.key_count() > self.limits.max_keys {
                            return Err(CollabJwksError::TooManyKeys {
                                maximum: self.limits.max_keys,
                            });
                        }
                        if let Some(current) = state.policy.as_ref() {
                            policy.ensure_successor_of(current)?;
                        }
                        (policy.keyset().clone(), Some(policy))
                    }
                };
                ensure_not_cancelled(cancelled)?;
                state.keyset = Some(keyset);
                state.policy = policy;
                state.etag = etag;
                state.fresh_until = fresh_until(now, max_age_seconds, self.limits.max_age_seconds);
            }
            CollabJwksFetchResponse::NotModified {
                etag,
                max_age_seconds,
            } => {
                if state.keyset.is_none() {
                    return Err(CollabJwksError::NotModifiedWithoutCache);
                }
                let etag = validate_etag(etag, self.limits.max_etag_bytes)?;
                ensure_not_cancelled(cancelled)?;
                if let Some(etag) = etag {
                    state.etag = Some(etag);
                }
                state.fresh_until = fresh_until(now, max_age_seconds, self.limits.max_age_seconds);
            }
        }
        state.last_successful_refresh = Some(now);
        Ok(())
    }

    fn validate_clock_mode(&self, now_unix_seconds: Option<u64>) -> Result<(), CollabJwksError> {
        match (&self.source, now_unix_seconds) {
            (CacheSource::LegacyJwks, None) | (CacheSource::SignedPolicy { .. }, Some(_)) => Ok(()),
            _ => Err(CollabUnionPolicyError::Inactive.into()),
        }
    }

    fn policy_active(
        &self,
        state: &CacheState,
        now_unix_seconds: Option<u64>,
    ) -> Result<(), CollabJwksError> {
        match &self.source {
            CacheSource::LegacyJwks => Ok(()),
            CacheSource::SignedPolicy { .. } => state
                .policy
                .as_ref()
                .ok_or(CollabUnionPolicyError::Inactive)?
                .ensure_active_at(now_unix_seconds.ok_or(CollabUnionPolicyError::Inactive)?)
                .map_err(Into::into),
        }
    }

    fn cached_key(
        &self,
        state: &CacheState,
        key_id: &str,
        now_unix_seconds: Option<u64>,
    ) -> Option<[u8; 32]> {
        match &self.source {
            CacheSource::LegacyJwks => state
                .keyset
                .as_ref()
                .and_then(|keyset| keyset.verification_key(key_id)),
            CacheSource::SignedPolicy { .. } => state
                .policy
                .as_ref()?
                .verification_key_at(key_id, now_unix_seconds?),
        }
    }
}

fn never_cancelled() -> bool {
    false
}

fn ensure_not_cancelled(cancelled: &dyn Fn() -> bool) -> Result<(), CollabJwksError> {
    if cancelled() {
        Err(CollabJwksFetchError::Cancelled.into())
    } else {
        Ok(())
    }
}

impl<F> fmt::Debug for CollabJwksCache<F> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabJwksCache")
            .field("endpoint", &self.endpoint)
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
#[path = "collab_jwks_cache_cancellation_tests.rs"]
mod cancellation_tests;

#[cfg(test)]
#[path = "collab_policy_cache_tests.rs"]
mod policy_tests;

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use base64::Engine as _;
    use ed25519_dalek::SigningKey;
    use serde_json::json;

    use super::*;

    #[derive(Default)]
    struct FetchState {
        requests: Vec<(String, Option<String>, usize)>,
        responses: VecDeque<Result<CollabJwksFetchResponse, CollabJwksFetchError>>,
    }

    #[derive(Clone)]
    struct RecordingFetcher {
        state: Arc<Mutex<FetchState>>,
    }

    impl RecordingFetcher {
        fn with_responses(
            responses: impl IntoIterator<Item = Result<CollabJwksFetchResponse, CollabJwksFetchError>>,
        ) -> (Self, Arc<Mutex<FetchState>>) {
            let state = Arc::new(Mutex::new(FetchState {
                requests: Vec::new(),
                responses: responses.into_iter().collect(),
            }));
            (
                Self {
                    state: Arc::clone(&state),
                },
                state,
            )
        }
    }

    impl CollabJwksFetcher for RecordingFetcher {
        fn fetch(
            &self,
            request: CollabJwksFetchRequest<'_>,
        ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
            let mut state = self.state.lock().expect("test fetch state");
            state.requests.push((
                request.endpoint.to_owned(),
                request.etag.map(str::to_owned),
                request.maximum_body_bytes,
            ));
            state
                .responses
                .pop_front()
                .unwrap_or(Err(CollabJwksFetchError::Unavailable))
        }
    }

    fn key(seed: u8) -> [u8; 32] {
        SigningKey::from_bytes(&[seed; 32])
            .verifying_key()
            .to_bytes()
    }

    fn jwks(keys: &[(&str, u8)]) -> Vec<u8> {
        let keys = keys
            .iter()
            .map(|(key_id, seed)| {
                json!({
                    "kty": "OKP",
                    "crv": "Ed25519",
                    "alg": "Ed25519",
                    "use": "sig",
                    "key_ops": ["verify"],
                    "kid": key_id,
                    "x": URL_SAFE_NO_PAD.encode(key(*seed)),
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&json!({ "keys": keys })).unwrap()
    }

    fn modified(
        body: Vec<u8>,
        etag: &str,
        max_age_seconds: u64,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        Ok(CollabJwksFetchResponse::Modified {
            body,
            etag: Some(etag.to_owned()),
            max_age_seconds,
        })
    }

    #[test]
    fn fresh_cache_uses_etag_and_accepts_not_modified() {
        let (fetcher, state) = RecordingFetcher::with_responses([
            modified(jwks(&[("key_A", 1)]), "\"v1\"", 10),
            Ok(CollabJwksFetchResponse::NotModified {
                etag: None,
                max_age_seconds: 10,
            }),
        ]);
        let cache = CollabJwksCache::new(
            "https://issuer.example/jwks",
            fetcher,
            CollabJwksCacheLimits::default(),
        )
        .unwrap();
        let now = Instant::now();

        assert_eq!(cache.verification_key("key_A", now).unwrap(), key(1));
        assert_eq!(
            cache
                .verification_key("key_A", now + Duration::from_secs(5))
                .unwrap(),
            key(1)
        );
        assert_eq!(
            cache
                .verification_key("key_A", now + Duration::from_secs(10))
                .unwrap(),
            key(1)
        );

        let state = state.lock().unwrap();
        assert_eq!(state.requests.len(), 2);
        assert_eq!(state.requests[0].1, None);
        assert_eq!(state.requests[1].1.as_deref(), Some("\"v1\""));
        assert_eq!(state.requests[0].2, DEFAULT_MAX_COLLAB_JWKS_BYTES);
    }

    #[test]
    fn unknown_key_refresh_is_throttled_then_allows_rotation() {
        let (fetcher, state) = RecordingFetcher::with_responses([
            modified(jwks(&[("key_A", 1)]), "\"v1\"", 300),
            modified(jwks(&[("key_A", 1), ("key_B", 2)]), "\"v2\"", 300),
        ]);
        let cache = CollabJwksCache::new(
            "https://issuer.example/jwks",
            fetcher,
            CollabJwksCacheLimits::default(),
        )
        .unwrap();
        let now = Instant::now();

        assert_eq!(cache.verification_key("key_A", now).unwrap(), key(1));
        assert_eq!(
            cache.verification_key("key_B", now + Duration::from_secs(1)),
            Err(CollabJwksError::UnknownKey)
        );
        assert_eq!(state.lock().unwrap().requests.len(), 1);
        assert_eq!(
            cache
                .verification_key("key_B", now + Duration::from_secs(31))
                .unwrap(),
            key(2)
        );
        assert_eq!(state.lock().unwrap().requests.len(), 2);
    }

    #[test]
    fn invalid_refresh_never_replaces_cache_and_stale_keys_fail_closed() {
        let (fetcher, state) = RecordingFetcher::with_responses([
            modified(jwks(&[("key_A", 1)]), "\"v1\"", 1),
            modified(b"{not-json".to_vec(), "\"bad\"", 300),
            Ok(CollabJwksFetchResponse::NotModified {
                etag: None,
                max_age_seconds: 30,
            }),
        ]);
        let cache = CollabJwksCache::new(
            "https://issuer.example/jwks",
            fetcher,
            CollabJwksCacheLimits::default(),
        )
        .unwrap();
        let now = Instant::now();
        assert_eq!(cache.verification_key("key_A", now).unwrap(), key(1));

        assert_eq!(
            cache.verification_key("key_A", now + Duration::from_secs(2)),
            Err(CollabJwksError::MalformedJson)
        );
        assert_eq!(
            cache.verification_key("key_A", now + Duration::from_millis(2_500)),
            Err(CollabJwksError::RefreshThrottled)
        );
        assert_eq!(
            cache
                .verification_key("key_A", now + Duration::from_secs(3))
                .unwrap(),
            key(1)
        );

        let state = state.lock().unwrap();
        assert_eq!(state.requests.len(), 3);
        assert_eq!(state.requests[2].1.as_deref(), Some("\"v1\""));
    }

    #[test]
    fn zero_max_age_success_reuses_known_key_only_during_success_backoff() {
        let (fetcher, state) = RecordingFetcher::with_responses([
            modified(jwks(&[("key_A", 1)]), "\"v1\"", 0),
            Ok(CollabJwksFetchResponse::NotModified {
                etag: None,
                max_age_seconds: 0,
            }),
        ]);
        let cache = CollabJwksCache::new(
            "https://issuer.example/jwks",
            fetcher,
            CollabJwksCacheLimits::default(),
        )
        .unwrap();
        let now = Instant::now();

        assert_eq!(cache.verification_key("key_A", now).unwrap(), key(1));
        assert_eq!(
            cache
                .verification_key("key_A", now + Duration::from_millis(500))
                .unwrap(),
            key(1)
        );
        assert_eq!(state.lock().unwrap().requests.len(), 1);
        assert_eq!(
            cache
                .verification_key("key_A", now + Duration::from_secs(1))
                .unwrap(),
            key(1)
        );
        assert_eq!(state.lock().unwrap().requests.len(), 2);
    }

    #[test]
    fn cache_rejects_untrusted_configuration_and_invalid_304() {
        let (fetcher, _) = RecordingFetcher::with_responses([]);
        assert!(matches!(
            CollabJwksCache::new(
                "http://issuer.example/jwks",
                fetcher.clone(),
                CollabJwksCacheLimits::default()
            ),
            Err(CollabVerifierConfigError::InvalidJwksEndpoint)
        ));
        let invalid_limits = CollabJwksCacheLimits {
            max_keys: 0,
            ..CollabJwksCacheLimits::default()
        };
        assert!(matches!(
            CollabJwksCache::new("https://issuer.example/jwks", fetcher, invalid_limits),
            Err(CollabVerifierConfigError::InvalidCacheLimits)
        ));

        let (fetcher, _) =
            RecordingFetcher::with_responses([Ok(CollabJwksFetchResponse::NotModified {
                etag: None,
                max_age_seconds: 10,
            })]);
        let cache = CollabJwksCache::new(
            "https://issuer.example/jwks",
            fetcher,
            CollabJwksCacheLimits::default(),
        )
        .unwrap();
        assert_eq!(
            cache.verification_key("key_A", Instant::now()),
            Err(CollabJwksError::NotModifiedWithoutCache)
        );

        let (fetcher, _) =
            RecordingFetcher::with_responses([Ok(CollabJwksFetchResponse::Modified {
                body: jwks(&[("key_A", 1)]),
                etag: Some("\"unsafe\r\nheader\"".to_owned()),
                max_age_seconds: 10,
            })]);
        let cache = CollabJwksCache::new(
            "https://issuer.example/jwks",
            fetcher,
            CollabJwksCacheLimits::default(),
        )
        .unwrap();
        assert_eq!(
            cache.verification_key("key_A", Instant::now()),
            Err(CollabJwksError::InvalidEtag {
                maximum: DEFAULT_MAX_COLLAB_JWKS_ETAG_BYTES
            })
        );
    }
}
