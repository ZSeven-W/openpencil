//! Offline-signed, cross-region collaboration verification-key policy.

use std::collections::{BTreeMap, BTreeSet};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{
    collab_claims::valid_https_origin, CollabJwks, CollabUnionPolicyError,
    HARD_MAX_COLLAB_JWKS_BYTES,
};

pub const COLLAB_UNION_POLICY_VERSION: u32 = 1;
pub const MAX_COLLAB_UNION_POLICY_REGIONS: usize = 8;
pub const MAX_COLLAB_UNION_POLICY_KEYS: usize = 24;
pub const MAX_COLLAB_UNION_POLICY_LIFETIME_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const COLLAB_UNION_POLICY_ROOT_X: &str = "wiQJcA9o-bydBkfIVnVUJzKA4wtv8Dapn0JYhS_bZ-I";

const POLICY_DOMAIN: &[u8] = b"openpencil/collab-union-policy/v1\0";
const MAX_REGION_ID_BYTES: usize = 32;
const MAX_KEY_ID_BYTES: usize = 128;

/// A verified public-key union authorized by the pinned offline root.
#[derive(Clone, PartialEq, Eq)]
pub struct CollabUnionPolicy {
    generation: u64,
    issuer: String,
    not_before_unix: i64,
    not_after_unix: i64,
    keyset: CollabJwks,
    verification_keys: BTreeMap<String, PolicyVerificationKey>,
    canonical_message: Vec<u8>,
}

impl CollabUnionPolicy {
    pub fn from_json(
        body: &[u8],
        maximum_body_bytes: usize,
        expected_issuer: &str,
        now_unix_seconds: u64,
    ) -> Result<Self, CollabUnionPolicyError> {
        let root = decode_fixed::<32>(COLLAB_UNION_POLICY_ROOT_X)
            .ok_or(CollabUnionPolicyError::InvalidSignature)?;
        Self::from_json_with_root(
            body,
            maximum_body_bytes,
            expected_issuer,
            now_unix_seconds,
            root,
        )
    }

    fn from_json_with_root(
        body: &[u8],
        maximum_body_bytes: usize,
        expected_issuer: &str,
        now_unix_seconds: u64,
        root: [u8; 32],
    ) -> Result<Self, CollabUnionPolicyError> {
        let maximum_body_bytes = maximum_body_bytes.min(HARD_MAX_COLLAB_JWKS_BYTES);
        if body.is_empty() || body.len() > maximum_body_bytes {
            return Err(CollabUnionPolicyError::InvalidBodySize);
        }
        let wire: PolicyWire =
            serde_json::from_slice(body).map_err(|_| CollabUnionPolicyError::MalformedJson)?;
        let canonical = canonicalize(wire, expected_issuer)?;
        let unsigned_json = serde_json::to_vec(&canonical.unsigned)
            .map_err(|_| CollabUnionPolicyError::InvalidProfile)?;
        let mut message = Vec::with_capacity(POLICY_DOMAIN.len() + unsigned_json.len());
        message.extend_from_slice(POLICY_DOMAIN);
        message.extend_from_slice(&unsigned_json);

        let signature = decode_fixed::<64>(&canonical.signature)
            .ok_or(CollabUnionPolicyError::InvalidSignature)?;
        let root = VerifyingKey::from_bytes(&root)
            .map_err(|_| CollabUnionPolicyError::InvalidSignature)?;
        root.verify_strict(&message, &Signature::from_bytes(&signature))
            .map_err(|_| CollabUnionPolicyError::InvalidSignature)?;

        let policy = Self {
            generation: canonical.unsigned.generation,
            issuer: canonical.unsigned.issuer,
            not_before_unix: canonical.unsigned.not_before_unix,
            not_after_unix: canonical.unsigned.not_after_unix,
            keyset: CollabJwks::from_verification_keys(canonical.keys),
            verification_keys: canonical.verification_keys,
            canonical_message: message,
        };
        policy.ensure_active_at(now_unix_seconds)?;
        Ok(policy)
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn key_count(&self) -> usize {
        self.keyset.len()
    }

    pub(crate) fn keyset(&self) -> &CollabJwks {
        &self.keyset
    }

    pub(crate) fn verification_key_at(
        &self,
        key_id: &str,
        now_unix_seconds: u64,
    ) -> Option<[u8; 32]> {
        let now = i64::try_from(now_unix_seconds).ok()?;
        let key = self.verification_keys.get(key_id)?;
        (key.activated_at_unix != 0
            && key.activated_at_unix <= now
            && (key.not_after_unix == 0 || now < key.not_after_unix))
            .then_some(key.public_key)
    }

    pub(crate) fn ensure_successor_of(&self, current: &Self) -> Result<(), CollabUnionPolicyError> {
        if self.generation < current.generation {
            return Err(CollabUnionPolicyError::GenerationRollback);
        }
        if self.generation == current.generation
            && self.canonical_message != current.canonical_message
        {
            return Err(CollabUnionPolicyError::GenerationRewrite);
        }
        Ok(())
    }

    pub(crate) fn ensure_active_at(
        &self,
        now_unix_seconds: u64,
    ) -> Result<(), CollabUnionPolicyError> {
        let now = i64::try_from(now_unix_seconds).map_err(|_| CollabUnionPolicyError::Inactive)?;
        if self.not_before_unix > now || self.not_after_unix <= now {
            return Err(CollabUnionPolicyError::Inactive);
        }
        if self.verification_keys.values().any(|key| {
            key.published_at_unix > now
                || key.activated_at_unix > now
                || key.retired_at_unix > now
                || (key.not_after_unix != 0 && key.not_after_unix <= now)
        }) {
            return Err(CollabUnionPolicyError::Inactive);
        }
        Ok(())
    }
}

impl std::fmt::Debug for CollabUnionPolicy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CollabUnionPolicy")
            .field("generation", &self.generation)
            .field("issuer", &self.issuer)
            .field("not_before_unix", &self.not_before_unix)
            .field("not_after_unix", &self.not_after_unix)
            .field("key_count", &self.keyset.len())
            .finish()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyWire {
    version: u32,
    generation: u64,
    issuer: String,
    not_before_unix: i64,
    not_after_unix: i64,
    required_regions: Vec<String>,
    keys: Vec<PolicyKey>,
    signature: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PolicyKey {
    region: String,
    kid: String,
    x: String,
    published_at_unix: i64,
    activated_at_unix: i64,
    retired_at_unix: i64,
    not_after_unix: i64,
}

#[derive(Serialize)]
struct UnsignedPolicy {
    version: u32,
    generation: u64,
    issuer: String,
    not_before_unix: i64,
    not_after_unix: i64,
    required_regions: Vec<String>,
    keys: Vec<PolicyKey>,
}

struct CanonicalPolicy {
    unsigned: UnsignedPolicy,
    signature: String,
    keys: BTreeMap<String, [u8; 32]>,
    verification_keys: BTreeMap<String, PolicyVerificationKey>,
}

#[derive(Default)]
struct RegionState {
    active: usize,
    next: usize,
    retired: usize,
}

#[derive(Clone, PartialEq, Eq)]
struct PolicyVerificationKey {
    public_key: [u8; 32],
    published_at_unix: i64,
    activated_at_unix: i64,
    retired_at_unix: i64,
    not_after_unix: i64,
}

fn canonicalize(
    wire: PolicyWire,
    expected_issuer: &str,
) -> Result<CanonicalPolicy, CollabUnionPolicyError> {
    if wire.version != COLLAB_UNION_POLICY_VERSION
        || wire.generation == 0
        || wire.not_before_unix <= 0
        || wire.not_after_unix <= wire.not_before_unix
        || wire.not_after_unix - wire.not_before_unix > MAX_COLLAB_UNION_POLICY_LIFETIME_SECONDS
    {
        return Err(CollabUnionPolicyError::InvalidProfile);
    }
    if wire.issuer != expected_issuer || !valid_https_origin(&wire.issuer) {
        return Err(CollabUnionPolicyError::InvalidIssuer);
    }
    if wire.required_regions.is_empty()
        || wire.required_regions.len() > MAX_COLLAB_UNION_POLICY_REGIONS
    {
        return Err(CollabUnionPolicyError::InvalidRegions);
    }
    if wire.keys.is_empty() || wire.keys.len() > MAX_COLLAB_UNION_POLICY_KEYS {
        return Err(CollabUnionPolicyError::InvalidKeys);
    }

    let mut regions = wire.required_regions;
    regions.sort();
    if regions.iter().any(|region| !valid_region_id(region))
        || regions.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(CollabUnionPolicyError::InvalidRegions);
    }
    let mut region_states = regions
        .iter()
        .cloned()
        .map(|region| (region, RegionState::default()))
        .collect::<BTreeMap<_, _>>();

    let mut keys = wire.keys;
    keys.sort_by(|left, right| {
        left.kid
            .cmp(&right.kid)
            .then_with(|| left.region.cmp(&right.region))
    });
    let mut verification_keys = BTreeMap::new();
    let mut policy_verification_keys = BTreeMap::new();
    let mut public_keys = BTreeSet::new();
    for key in &keys {
        if !valid_key_id(&key.kid)
            || !valid_region_id(&key.region)
            || key.published_at_unix <= 0
            || key.activated_at_unix < 0
            || key.retired_at_unix < 0
            || key.not_after_unix < 0
        {
            return Err(CollabUnionPolicyError::InvalidKeys);
        }
        let Some(state) = region_states.get_mut(&key.region) else {
            return Err(CollabUnionPolicyError::InvalidKeys);
        };
        let public_key = decode_fixed::<32>(&key.x).ok_or(CollabUnionPolicyError::InvalidKeys)?;
        VerifyingKey::from_bytes(&public_key).map_err(|_| CollabUnionPolicyError::InvalidKeys)?;
        if verification_keys
            .insert(key.kid.clone(), public_key)
            .is_some()
            || !public_keys.insert(public_key)
        {
            return Err(CollabUnionPolicyError::InvalidKeys);
        }
        policy_verification_keys.insert(
            key.kid.clone(),
            PolicyVerificationKey {
                public_key,
                published_at_unix: key.published_at_unix,
                activated_at_unix: key.activated_at_unix,
                retired_at_unix: key.retired_at_unix,
                not_after_unix: key.not_after_unix,
            },
        );
        if (key.retired_at_unix == 0) != (key.not_after_unix == 0)
            || (key.retired_at_unix != 0
                && (key.activated_at_unix == 0
                    || key.retired_at_unix < key.activated_at_unix
                    || key.not_after_unix <= key.retired_at_unix))
        {
            return Err(CollabUnionPolicyError::InvalidKeyLifecycle);
        }
        if key.retired_at_unix != 0 {
            state.retired += 1;
        } else if key.activated_at_unix == 0 {
            state.next += 1;
        } else {
            state.active += 1;
        }
    }
    if region_states
        .values()
        .any(|state| state.active != 1 || state.next != 1 || state.retired > 1)
    {
        return Err(CollabUnionPolicyError::InvalidRotationPhase);
    }

    Ok(CanonicalPolicy {
        unsigned: UnsignedPolicy {
            version: wire.version,
            generation: wire.generation,
            issuer: wire.issuer,
            not_before_unix: wire.not_before_unix,
            not_after_unix: wire.not_after_unix,
            required_regions: regions,
            keys,
        },
        signature: wire.signature,
        keys: verification_keys,
        verification_keys: policy_verification_keys,
    })
}

fn decode_fixed<const SIZE: usize>(value: &str) -> Option<[u8; SIZE]> {
    if value.contains('=') {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return None;
    }
    decoded.try_into().ok()
}

fn valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_KEY_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_region_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= MAX_REGION_ID_BYTES
        && bytes.first() != Some(&b'-')
        && bytes.last() != Some(&b'-')
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

#[cfg(test)]
#[path = "collab_union_policy_tests.rs"]
mod tests;
