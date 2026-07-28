//! Strict, resource-bounded parser for the collaboration Ed25519 JWKS.

use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use ed25519_dalek::VerifyingKey;
use serde::Deserialize;

use crate::{
    collab_claims::valid_key_id, CollabJwkErrorKind, CollabJwksError, COLLAB_JWS_ALGORITHM,
};

pub const DEFAULT_MAX_COLLAB_JWKS_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_COLLAB_JWKS_KEYS: usize = 24;
pub const HARD_MAX_COLLAB_JWKS_BYTES: usize = 256 * 1024;
pub const HARD_MAX_COLLAB_JWKS_KEYS: usize = 64;

/// A validated public verification-key set.
#[derive(Clone, PartialEq, Eq)]
pub struct CollabJwks {
    keys: BTreeMap<String, [u8; 32]>,
}

impl CollabJwks {
    pub fn from_json(
        body: &[u8],
        maximum_body_bytes: usize,
        maximum_keys: usize,
    ) -> Result<Self, CollabJwksError> {
        let maximum_body_bytes = maximum_body_bytes.min(HARD_MAX_COLLAB_JWKS_BYTES);
        let maximum_keys = maximum_keys.min(HARD_MAX_COLLAB_JWKS_KEYS);
        if body.is_empty() || body.len() > maximum_body_bytes {
            return Err(CollabJwksError::InvalidBodySize {
                maximum: maximum_body_bytes,
            });
        }
        let wire: JwksWire =
            serde_json::from_slice(body).map_err(|_| CollabJwksError::MalformedJson)?;
        if wire.keys.is_empty() {
            return Err(CollabJwksError::EmptyKeyset);
        }
        if wire.keys.len() > maximum_keys {
            return Err(CollabJwksError::TooManyKeys {
                maximum: maximum_keys,
            });
        }

        let mut keys = BTreeMap::new();
        for (index, key) in wire.keys.into_iter().enumerate() {
            let (key_id, public_key) = validate_key(index, key)?;
            match keys.entry(key_id) {
                Entry::Vacant(entry) => {
                    entry.insert(public_key);
                }
                Entry::Occupied(_) => return Err(CollabJwksError::DuplicateKeyId),
            }
        }
        Ok(Self { keys })
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub(crate) fn verification_key(&self, key_id: &str) -> Option<[u8; 32]> {
        self.keys.get(key_id).copied()
    }

    pub(crate) fn from_verification_keys(keys: BTreeMap<String, [u8; 32]>) -> Self {
        Self { keys }
    }
}

impl fmt::Debug for CollabJwks {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabJwks")
            .field("key_count", &self.keys.len())
            .finish()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JwksWire {
    keys: Vec<JwkWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JwkWire {
    kty: String,
    crv: String,
    alg: String,
    #[serde(rename = "use")]
    public_use: String,
    key_ops: Vec<String>,
    kid: String,
    x: String,
}

fn validate_key(index: usize, key: JwkWire) -> Result<(String, [u8; 32]), CollabJwksError> {
    let invalid = |kind| CollabJwksError::InvalidKey { index, kind };
    if !valid_key_id(&key.kid) {
        return Err(invalid(CollabJwkErrorKind::InvalidKeyId));
    }
    if key.kty != "OKP" {
        return Err(invalid(CollabJwkErrorKind::WrongKeyType));
    }
    if key.crv != "Ed25519" {
        return Err(invalid(CollabJwkErrorKind::WrongCurve));
    }
    if key.alg != COLLAB_JWS_ALGORITHM {
        return Err(invalid(CollabJwkErrorKind::WrongAlgorithm));
    }
    if key.public_use != "sig" {
        return Err(invalid(CollabJwkErrorKind::WrongUse));
    }
    if key.key_ops.as_slice() != ["verify"] {
        return Err(invalid(CollabJwkErrorKind::WrongKeyOperations));
    }
    let public_key =
        decode_public_key(&key.x).ok_or_else(|| invalid(CollabJwkErrorKind::InvalidPublicKey))?;
    VerifyingKey::from_bytes(&public_key)
        .map_err(|_| invalid(CollabJwkErrorKind::InvalidPublicKey))?;
    Ok((key.kid, public_key))
}

fn decode_public_key(value: &str) -> Option<[u8; 32]> {
    if value.contains('=') || value.len() > 64 {
        return None;
    }
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return None;
    }
    decoded.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use serde_json::json;

    fn public_key(seed: u8) -> [u8; 32] {
        SigningKey::from_bytes(&[seed; 32])
            .verifying_key()
            .to_bytes()
    }

    fn key(key_id: &str, public_key: [u8; 32]) -> serde_json::Value {
        json!({
            "kty": "OKP",
            "crv": "Ed25519",
            "alg": "Ed25519",
            "use": "sig",
            "key_ops": ["verify"],
            "kid": key_id,
            "x": URL_SAFE_NO_PAD.encode(public_key),
        })
    }

    #[test]
    fn parses_a_strict_ed25519_keyset() {
        let expected = public_key(7);
        let body = serde_json::to_vec(&json!({ "keys": [key("key_A", expected)] })).unwrap();
        let keyset = CollabJwks::from_json(
            &body,
            DEFAULT_MAX_COLLAB_JWKS_BYTES,
            DEFAULT_MAX_COLLAB_JWKS_KEYS,
        )
        .unwrap();
        assert_eq!(keyset.len(), 1);
        assert_eq!(keyset.verification_key("key_A"), Some(expected));
        assert_eq!(format!("{keyset:?}"), "CollabJwks { key_count: 1 }");
    }

    #[test]
    fn rejects_duplicate_or_private_keys() {
        let duplicate = serde_json::to_vec(&json!({
            "keys": [key("same", public_key(7)), key("same", public_key(8))]
        }))
        .unwrap();
        assert_eq!(
            CollabJwks::from_json(&duplicate, 65_536, 16),
            Err(CollabJwksError::DuplicateKeyId)
        );

        let mut private = key("key_A", public_key(7));
        private["d"] = json!("private-material");
        let private = serde_json::to_vec(&json!({ "keys": [private] })).unwrap();
        assert_eq!(
            CollabJwks::from_json(&private, 65_536, 16),
            Err(CollabJwksError::MalformedJson)
        );
    }

    #[test]
    fn rejects_wrong_profile_and_resource_limits() {
        let mut wrong = key("key_A", public_key(7));
        wrong["alg"] = json!("EdDSA");
        let wrong = serde_json::to_vec(&json!({ "keys": [wrong] })).unwrap();
        assert!(matches!(
            CollabJwks::from_json(&wrong, 65_536, 16),
            Err(CollabJwksError::InvalidKey {
                kind: CollabJwkErrorKind::WrongAlgorithm,
                ..
            })
        ));

        assert_eq!(
            CollabJwks::from_json(b"{\"keys\":[]}", 65_536, 16),
            Err(CollabJwksError::EmptyKeyset)
        );
        let too_many = serde_json::to_vec(&json!({
            "keys": [
                key("key_A", public_key(7)),
                key("key_B", public_key(8))
            ]
        }))
        .unwrap();
        assert_eq!(
            CollabJwks::from_json(&too_many, 65_536, 1),
            Err(CollabJwksError::TooManyKeys { maximum: 1 })
        );
        assert_eq!(
            CollabJwks::from_json(b"{}", 1, 16),
            Err(CollabJwksError::InvalidBodySize { maximum: 1 })
        );
        assert_eq!(
            CollabJwks::from_json(
                &vec![b' '; HARD_MAX_COLLAB_JWKS_BYTES + 1],
                usize::MAX,
                usize::MAX
            ),
            Err(CollabJwksError::InvalidBodySize {
                maximum: HARD_MAX_COLLAB_JWKS_BYTES
            })
        );
    }

    #[test]
    fn rejects_every_jwk_profile_deviation() {
        let cases = [
            ("kid", json!("bad.key"), CollabJwkErrorKind::InvalidKeyId),
            ("kty", json!("EC"), CollabJwkErrorKind::WrongKeyType),
            ("crv", json!("X25519"), CollabJwkErrorKind::WrongCurve),
            ("alg", json!("EdDSA"), CollabJwkErrorKind::WrongAlgorithm),
            ("use", json!("enc"), CollabJwkErrorKind::WrongUse),
            (
                "key_ops",
                json!(["sign"]),
                CollabJwkErrorKind::WrongKeyOperations,
            ),
            ("x", json!("AA"), CollabJwkErrorKind::InvalidPublicKey),
        ];
        for (field, value, expected) in cases {
            let mut candidate = key("key_A", public_key(7));
            candidate[field] = value;
            let body = serde_json::to_vec(&json!({ "keys": [candidate] })).unwrap();
            assert!(matches!(
                CollabJwks::from_json(&body, 65_536, 16),
                Err(CollabJwksError::InvalidKey { kind, .. }) if kind == expected
            ));
        }
    }
}
