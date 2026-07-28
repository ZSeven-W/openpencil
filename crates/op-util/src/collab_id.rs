//! Canonical collaboration namespace and document-id grammar.
//!
//! These types deliberately contain no randomness or session policy. The
//! session owner is responsible for assigning a unique, unpredictable peer
//! namespace; this module gives every open-source caller one parser and one
//! formatter for that assigned value.

use std::fmt;
use std::str::FromStr;

/// Maximum encoded length of an owner-assigned peer namespace.
pub const MAX_PEER_NAMESPACE_LEN: usize = 64;

/// A validated, case-sensitive collaboration peer namespace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerNamespace(String);

impl PeerNamespace {
    /// Parse a namespace containing only ASCII letters, digits, or `-`.
    pub fn parse(value: impl Into<String>) -> Result<Self, CollabIdError> {
        Self::try_from(value.into())
    }

    /// The exact validated namespace bytes.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for PeerNamespace {
    type Error = CollabIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_namespace(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for PeerNamespace {
    type Error = CollabIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_owned())
    }
}

impl FromStr for PeerNamespace {
    type Err = CollabIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value)
    }
}

impl AsRef<str> for PeerNamespace {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for PeerNamespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A canonical `c_<namespace>_<counter>` document id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamespacedId {
    namespace: PeerNamespace,
    counter: u64,
}

impl NamespacedId {
    /// Build an id from an already validated namespace and a counter.
    pub const fn new(namespace: PeerNamespace, counter: u64) -> Self {
        Self { namespace, counter }
    }

    /// Parse a canonical collaboration document id.
    pub fn parse(value: &str) -> Result<Self, CollabIdError> {
        Self::try_from(value)
    }

    /// Namespace that owns this id.
    pub fn namespace(&self) -> &PeerNamespace {
        &self.namespace
    }

    /// Monotonic counter encoded in this id.
    pub const fn counter(&self) -> u64 {
        self.counter
    }
}

impl TryFrom<&str> for NamespacedId {
    type Error = CollabIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let body = value
            .strip_prefix("c_")
            .ok_or(CollabIdError::MissingIdPrefix)?;
        let (namespace, counter) = body
            .split_once('_')
            .ok_or(CollabIdError::MissingCounterSeparator)?;
        let namespace = PeerNamespace::try_from(namespace)?;
        let counter = parse_counter(counter)?;
        Ok(Self::new(namespace, counter))
    }
}

impl TryFrom<String> for NamespacedId {
    type Error = CollabIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl FromStr for NamespacedId {
    type Err = CollabIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value)
    }
}

impl fmt::Display for NamespacedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c_{}_{}", self.namespace, self.counter)
    }
}

/// Typed failure while parsing a collaboration namespace or document id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollabIdError {
    /// A peer namespace must contain at least one byte.
    EmptyNamespace,
    /// A peer namespace is longer than [`MAX_PEER_NAMESPACE_LEN`].
    NamespaceTooLong { actual: usize },
    /// A peer namespace contains a byte outside ASCII alphanumeric or `-`.
    InvalidNamespaceByte { index: usize },
    /// A collaboration id does not start with `c_`.
    MissingIdPrefix,
    /// A collaboration id has no `_` between namespace and counter.
    MissingCounterSeparator,
    /// The counter is empty, non-decimal, has a leading zero, or exceeds `u64`.
    InvalidCounter,
}

impl fmt::Display for CollabIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyNamespace => f.write_str("peer namespace is empty"),
            Self::NamespaceTooLong { actual } => write!(
                f,
                "peer namespace is {actual} bytes; maximum is {MAX_PEER_NAMESPACE_LEN}"
            ),
            Self::InvalidNamespaceByte { index } => {
                write!(f, "peer namespace has an invalid byte at index {index}")
            }
            Self::MissingIdPrefix => f.write_str("collaboration id must start with `c_`"),
            Self::MissingCounterSeparator => {
                f.write_str("collaboration id is missing its counter separator")
            }
            Self::InvalidCounter => f.write_str("collaboration id counter is not canonical u64"),
        }
    }
}

impl std::error::Error for CollabIdError {}

fn validate_namespace(namespace: &str) -> Result<(), CollabIdError> {
    if namespace.is_empty() {
        return Err(CollabIdError::EmptyNamespace);
    }
    if namespace.len() > MAX_PEER_NAMESPACE_LEN {
        return Err(CollabIdError::NamespaceTooLong {
            actual: namespace.len(),
        });
    }
    if let Some((index, _)) = namespace
        .bytes()
        .enumerate()
        .find(|(_, byte)| !byte.is_ascii_alphanumeric() && *byte != b'-')
    {
        return Err(CollabIdError::InvalidNamespaceByte { index });
    }
    Ok(())
}

fn parse_counter(counter: &str) -> Result<u64, CollabIdError> {
    let canonical_decimal = !counter.is_empty() && (counter == "0" || !counter.starts_with('0'));
    if !canonical_decimal || !counter.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CollabIdError::InvalidCounter);
    }
    counter
        .parse::<u64>()
        .map_err(|_| CollabIdError::InvalidCounter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_namespace_accepts_grammar_boundaries() {
        let max = format!("A-{}", "z".repeat(MAX_PEER_NAMESPACE_LEN - 2));

        assert_eq!(PeerNamespace::try_from("a").unwrap().as_str(), "a");
        assert_eq!(PeerNamespace::try_from(max.as_str()).unwrap().as_str(), max);
    }

    #[test]
    fn peer_namespace_reports_invalid_inputs() {
        assert_eq!(
            PeerNamespace::try_from(""),
            Err(CollabIdError::EmptyNamespace)
        );
        assert_eq!(
            PeerNamespace::try_from("a".repeat(MAX_PEER_NAMESPACE_LEN + 1)),
            Err(CollabIdError::NamespaceTooLong {
                actual: MAX_PEER_NAMESPACE_LEN + 1
            })
        );
        for (value, index) in [
            ("peer_one", 4),
            ("peer one", 4),
            ("peer/one", 4),
            ("设备", 0),
        ] {
            assert_eq!(
                PeerNamespace::try_from(value),
                Err(CollabIdError::InvalidNamespaceByte { index })
            );
        }
    }

    #[test]
    fn namespaced_id_display_and_parse_round_trip() {
        for counter in [0, 1, 42, u64::MAX] {
            let id = NamespacedId::new(PeerNamespace::try_from("peer-a7").unwrap(), counter);
            let encoded = id.to_string();
            let decoded = NamespacedId::parse(&encoded).unwrap();

            assert_eq!(decoded, id);
            assert_eq!(decoded.namespace().as_str(), "peer-a7");
            assert_eq!(decoded.counter(), counter);
        }
    }

    #[test]
    fn namespaced_id_rejects_noncanonical_forms() {
        for value in [
            "",
            "peer_1",
            "x_peer_1",
            "c_peer",
            "c__1",
            "c_peer_",
            "c_peer_00",
            "c_peer_01",
            "c_peer_-1",
            "c_peer_1x",
            "c_peer_18446744073709551616",
            "c_peer_one_1",
        ] {
            assert!(
                NamespacedId::try_from(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn from_str_uses_the_same_canonical_grammar() {
        let namespace: PeerNamespace = "Peer-9".parse().unwrap();
        let id: NamespacedId = "c_Peer-9_9".parse().unwrap();

        assert_eq!(namespace, id.namespace().clone());
        assert_eq!(id.counter(), 9);
    }
}
