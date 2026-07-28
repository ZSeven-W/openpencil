use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::SigningKey;
use serde_json::json;

use super::*;

const ISSUER: &str = "https://collab.example.com";
const ENDPOINT: &str = "https://cn.example/api/v1/collab/policy";
const NOW: u64 = 1_800_000_000;
const SIGNATURE: &str = "aeno37t6xdvD-UX4JnBXn4TyV3mh\
    ZY8FWD2dkMtmTCVXwobarKlRDaXIsRxrf1O4MMYh_QggcwOBUUMDqd9hAw";

#[derive(Default)]
struct FetchState {
    requests: Vec<(String, Option<String>)>,
    responses: VecDeque<Result<CollabJwksFetchResponse, CollabJwksFetchError>>,
}

#[derive(Clone)]
struct PolicyFetcher(Arc<Mutex<FetchState>>);

impl CollabJwksFetcher for PolicyFetcher {
    fn fetch(
        &self,
        request: CollabJwksFetchRequest<'_>,
    ) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
        let mut state = self.0.lock().unwrap();
        state
            .requests
            .push((request.endpoint.to_owned(), request.etag.map(str::to_owned)));
        state
            .responses
            .pop_front()
            .unwrap_or(Err(CollabJwksFetchError::Unavailable))
    }
}

fn sequence_key(first_byte: u8) -> SigningKey {
    let mut seed = [0_u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        *byte = first_byte + u8::try_from(index).unwrap();
    }
    SigningKey::from_bytes(&seed)
}

fn key(region: &str, kid: &str, first_byte: u8, activated: i64) -> serde_json::Value {
    json!({
        "region": region,
        "kid": kid,
        "x": URL_SAFE_NO_PAD.encode(sequence_key(first_byte).verifying_key().to_bytes()),
        "published_at_unix": 1_700_000_000,
        "activated_at_unix": activated,
        "retired_at_unix": 0,
        "not_after_unix": 0,
    })
}

fn policy_body() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "version": 1,
        "generation": 7,
        "issuer": ISSUER,
        "not_before_unix": 1_799_900_000,
        "not_after_unix": 1_800_500_000,
        "required_regions": ["cn", "global"],
        "keys": [
            key("cn", "active_key", 1, 1_700_000_300),
            key("cn", "next_key", 41, 0),
            key("global", "remote_active_key", 81, 1_700_000_300),
            key("global", "remote_next_key", 121, 0),
        ],
        "signature": SIGNATURE,
    }))
    .unwrap()
}

fn modified(body: Vec<u8>) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
    Ok(CollabJwksFetchResponse::Modified {
        body,
        etag: Some("\"policy-v7\"".to_owned()),
        max_age_seconds: 300,
    })
}

#[test]
fn signed_policy_cache_uses_only_eligible_keys_and_preserves_etag_refresh() {
    let state = Arc::new(Mutex::new(FetchState {
        responses: VecDeque::from([
            modified(policy_body()),
            Ok(CollabJwksFetchResponse::NotModified {
                etag: None,
                max_age_seconds: 300,
            }),
        ]),
        ..FetchState::default()
    }));
    let cache = CollabJwksCache::new_signed_policy(
        ENDPOINT,
        ISSUER,
        PolicyFetcher(Arc::clone(&state)),
        CollabJwksCacheLimits::default(),
    )
    .unwrap();
    let cache_now = Instant::now();

    assert_eq!(
        cache
            .policy_verification_key("active_key", cache_now, NOW)
            .unwrap(),
        sequence_key(1).verifying_key().to_bytes()
    );
    assert_eq!(cache.cached_key_count().unwrap(), 4);
    assert_eq!(
        cache.policy_verification_key("next_key", cache_now, NOW),
        Err(CollabJwksError::UnknownKey)
    );
    assert_eq!(state.lock().unwrap().requests.len(), 1);

    assert_eq!(
        cache.policy_verification_key("unknown", cache_now + Duration::from_secs(31), NOW + 31),
        Err(CollabJwksError::UnknownKey)
    );
    let requests = &state.lock().unwrap().requests;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], (ENDPOINT.to_owned(), None));
    assert_eq!(
        requests[1],
        (ENDPOINT.to_owned(), Some("\"policy-v7\"".to_owned()))
    );
}

#[test]
fn cached_policy_expiry_forces_refresh_and_still_fails_closed_on_304() {
    let state = Arc::new(Mutex::new(FetchState {
        responses: VecDeque::from([
            modified(policy_body()),
            Ok(CollabJwksFetchResponse::NotModified {
                etag: None,
                max_age_seconds: 300,
            }),
        ]),
        ..FetchState::default()
    }));
    let cache = CollabJwksCache::new_signed_policy(
        ENDPOINT,
        ISSUER,
        PolicyFetcher(Arc::clone(&state)),
        CollabJwksCacheLimits::default(),
    )
    .unwrap();
    let cache_now = Instant::now();
    cache
        .policy_verification_key("active_key", cache_now, NOW)
        .unwrap();

    assert_eq!(
        cache.policy_verification_key(
            "active_key",
            cache_now + Duration::from_secs(1),
            1_800_500_000
        ),
        Err(CollabJwksError::Policy(CollabUnionPolicyError::Inactive))
    );
    assert_eq!(state.lock().unwrap().requests.len(), 2);
}
