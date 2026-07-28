use std::{fmt, str::FromStr};

use jian_ops_schema::{node::PenNode, PenDocument};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{
    finite::validate_finite,
    serde_context::{inline_document, inline_node, inline_txn, to_isolated_value},
    typed_images, CanonicalHashError, CanonicalHashParseError, CollabTxn,
};

/// Version of the canonical collaboration hash contract.
pub const CANONICAL_HASH_VERSION: u16 = 1;
/// Hash algorithm frozen by collaboration protocol v1.
pub const CANONICAL_HASH_ALGORITHM: &str = "blake3";
/// Domain prefix for complete canonical documents.
pub const DOCUMENT_HASH_DOMAIN: &str = "openpencil-collab/document/v1";
/// Domain prefix for canonical nodes and their authored subtrees.
pub const NODE_HASH_DOMAIN: &str = "openpencil-collab/node/v1";
/// Domain prefix for exact collaboration transactions.
pub const TXN_HASH_DOMAIN: &str = "openpencil-collab/txn/v1";

/// A BLAKE3 digest of a domain-separated canonical JSON encoding.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CanonicalHash([u8; 32]);

impl CanonicalHash {
    pub const BYTE_LEN: usize = 32;
    pub const HEX_LEN: usize = Self::BYTE_LEN * 2;

    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::BYTE_LEN] {
        &self.0
    }
}

impl fmt::Debug for CanonicalHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for CanonicalHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for CanonicalHash {
    type Err = CanonicalHashParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != Self::HEX_LEN
            || !value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(CanonicalHashParseError::InvalidEncoding);
        }

        let mut bytes = [0_u8; Self::BYTE_LEN];
        for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = (hex_nibble(chunk[0]) << 4) | hex_nibble(chunk[1]);
        }
        Ok(Self(bytes))
    }
}

impl Serialize for CanonicalHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for CanonicalHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        encoded.parse().map_err(D::Error::custom)
    }
}

/// Encode any serde value using the deterministic collaboration-v1 JSON rules.
///
/// Object keys are sorted lexicographically, arrays retain authored order, and
/// the encoding contains no insignificant whitespace.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalHashError> {
    validate_finite(value).map_err(|_| CanonicalHashError::NonFiniteNumber)?;
    let isolated = to_isolated_value(value)?;
    if !isolated.images.is_empty() {
        return Err(CanonicalHashError::ImageBearingGenericValue);
    }
    let mut output = Vec::new();
    write_canonical_value(&isolated.value, &mut output)?;
    Ok(output)
}

pub fn canonical_document_hash(
    document: &PenDocument,
) -> Result<CanonicalHash, CanonicalHashError> {
    if typed_images::document_has_external_image_ref(document) {
        return Err(CanonicalHashError::ExternalImageReference);
    }
    let value = typed_canonical_value(document, inline_document)?;
    hash_value(&value, DOCUMENT_HASH_DOMAIN)
}

pub fn canonical_node_hash(node: &PenNode) -> Result<CanonicalHash, CanonicalHashError> {
    if typed_images::node_has_external_image_ref(node) {
        return Err(CanonicalHashError::ExternalImageReference);
    }
    let value = typed_canonical_value(node, inline_node)?;
    hash_value(&value, NODE_HASH_DOMAIN)
}

pub fn canonical_txn_hash(txn: &CollabTxn) -> Result<CanonicalHash, CanonicalHashError> {
    if typed_images::txn_has_external_image_ref(txn) {
        return Err(CanonicalHashError::ExternalImageReference);
    }
    let value = typed_canonical_value(txn, inline_txn)?;
    hash_value(&value, TXN_HASH_DOMAIN)
}

fn typed_canonical_value<T: Serialize>(
    value: &T,
    inline: fn(&mut Value, &serde_json::Map<String, Value>),
) -> Result<Value, CanonicalHashError> {
    validate_finite(value).map_err(|_| CanonicalHashError::NonFiniteNumber)?;
    let mut isolated = to_isolated_value(value)?;
    inline(&mut isolated.value, &isolated.images);
    Ok(isolated.value)
}

fn hash_value(value: &Value, domain: &str) -> Result<CanonicalHash, CanonicalHashError> {
    let mut encoded = Vec::new();
    write_canonical_value(value, &mut encoded)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain.as_bytes());
    hasher.update(&[0]);
    hasher.update(&encoded);
    let digest = hasher.finalize();
    let mut bytes = [0_u8; CanonicalHash::BYTE_LEN];
    bytes.copy_from_slice(digest.as_bytes());
    Ok(CanonicalHash(bytes))
}

fn write_canonical_value(value: &Value, output: &mut Vec<u8>) -> Result<(), CanonicalHashError> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => output.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(value) => output.extend_from_slice(value.to_string().as_bytes()),
        Value::String(value) => {
            serde_json::to_writer(output, value)?;
        }
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                write_canonical_value(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                serde_json::to_writer(&mut *output, key)?;
                output.push(b':');
                write_canonical_value(value, output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("encoding was validated before decoding"),
    }
}
