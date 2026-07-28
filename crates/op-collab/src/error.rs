use crate::{CanonicalHash, CollabIdError, Role};

/// An error while producing a deterministic collaboration hash.
#[derive(Debug, thiserror::Error)]
pub enum CanonicalHashError {
    #[error("canonical JSON rejects non-finite floating-point values")]
    NonFiniteNumber,
    #[error("canonical collaboration hashes reject file-table image references")]
    ExternalImageReference,
    #[error("generic canonical JSON cannot encode image-bearing schema values; use a typed hash")]
    ImageBearingGenericValue,
    #[error("value cannot be represented as canonical JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// An invalid canonical-hash wire value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonicalHashParseError {
    #[error("canonical hash must contain exactly 64 lowercase hexadecimal characters")]
    InvalidEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpaqueTicketError {
    #[error("opaque ticket must not be empty")]
    Empty,
    #[error("opaque ticket must be valid UTF-8")]
    InvalidUtf8,
    #[error("opaque ticket has {actual} bytes; limit is {limit}")]
    TooLarge { actual: usize, limit: usize },
}

/// A version, shape, or resource-limit error at the collaboration wire layer.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("credential-bearing renewal frames require the dedicated zeroizing ticket codec")]
    SensitiveCredentialRequiresDedicatedCodec,
    #[error("collaboration envelope exceeds the {limit}-byte limit: {actual} bytes")]
    EnvelopeTooLarge { actual: usize, limit: usize },
    #[error("collaboration transaction exceeds the {limit}-byte limit: {actual} bytes")]
    TransactionTooLarge { actual: usize, limit: usize },
    #[error("collaboration protocol version {actual} is unsupported; expected {expected}")]
    UnsupportedVersion { actual: u16, expected: u16 },
    #[error("collaboration hash version {actual} is unsupported; expected {expected}")]
    UnsupportedHashVersion { actual: u16, expected: u16 },
    #[error("wire limit `{field}` must be in 1..={maximum}; found {actual}")]
    InvalidWireLimit {
        field: &'static str,
        actual: u32,
        maximum: u32,
    },
    #[error("collaboration transaction has {actual} operations; limit is {limit}")]
    TooManyOperations { actual: usize, limit: usize },
    #[error("failed to decode collaboration JSON: {0}")]
    Decode(#[source] serde_json::Error),
    #[error("collaboration JSON nesting depth {actual} exceeds the limit {limit}")]
    JsonNestingTooDeep { actual: usize, limit: usize },
    #[error("failed to encode collaboration JSON: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("collaboration JSON rejects non-finite floating-point values")]
    NonFiniteNumber,
    #[error("collaboration wire data must inline image sources instead of using file-table refs")]
    ExternalImageReference,
    #[error("collaboration snapshot is invalid: {0}")]
    InvalidSnapshot(#[from] SnapshotError),
    #[error("collaboration transaction is invalid: {0}")]
    InvalidTransaction(#[source] CollabApplyError),
    #[error("collaboration field `{field}` must not be empty")]
    EmptyIdentifier { field: &'static str },
    #[error("collaboration field `{field}` has {actual} bytes; limit is {limit}")]
    IdentifierTooLong {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    #[error("collaboration profile field `{field}` is invalid")]
    InvalidParticipantProfile { field: &'static str },
    #[error("welcome contains an invalid peer namespace: {0}")]
    InvalidPeerNamespace(#[from] CollabIdError),
    #[error("collaboration presence field `{field}` is not finite or valid")]
    InvalidPresence { field: &'static str },
    #[error("collaboration presence exceeds the {limit}-byte limit: {actual} bytes")]
    PresenceTooLarge { actual: usize, limit: usize },
}

/// A failed exact operation over a canonical document.
#[derive(Debug, thiserror::Error)]
pub enum CollabApplyError {
    #[error("role {role:?} cannot apply document transactions")]
    PermissionDenied { role: Role },
    #[error("collaboration transaction must contain at least one operation")]
    EmptyTransaction,
    #[error("collaboration documents and transactions reject non-finite floating-point values")]
    NonFiniteNumber,
    #[error("collaboration transaction has {actual} operations; limit is {limit}")]
    TooManyOperations { actual: usize, limit: usize },
    #[error("operation {operation} processes a {actual}-node subtree; limit is {limit}")]
    SubtreeTooLarge {
        operation: usize,
        actual: usize,
        limit: usize,
    },
    #[error("document has {actual} page/node items; limit is {limit}")]
    TooManyDocumentNodes { actual: usize, limit: usize },
    #[error("document tree depth {actual} exceeds the limit {limit}")]
    TreeTooDeep { actual: usize, limit: usize },
    #[error("identifier has {actual} bytes; limit is {limit}")]
    IdentifierTooLong { actual: usize, limit: usize },
    #[error("transaction validation visited {actual} nodes; limit is {limit}")]
    ValidationBudgetExceeded { actual: usize, limit: usize },
    #[error("document contains an empty page or node id")]
    EmptyId,
    #[error("document contains duplicate id `{id}`")]
    DuplicateId { id: String },
    #[error("document cannot contain both non-empty pages and legacy root children")]
    MixedDocumentRoots,
    #[error("page `{page_id}` does not exist")]
    PageNotFound { page_id: String },
    #[error("node `{node_id}` does not exist on the requested page")]
    NodeNotFound { node_id: String },
    #[error("parent node `{parent_id}` does not exist on the requested page")]
    ParentNotFound { parent_id: String },
    #[error("node `{parent_id}` cannot contain children")]
    ParentNotContainer { parent_id: String },
    #[error("index {index} is outside the valid insertion range 0..={len}")]
    IndexOutOfBounds { index: usize, len: usize },
    #[error("node `{node_id}` hash precondition failed: expected {expected}, found {actual}")]
    HashMismatch {
        node_id: String,
        expected: CanonicalHash,
        actual: CanonicalHash,
    },
    #[error("replacement id `{replacement_id}` does not match target id `{node_id}`")]
    ReplacementIdChanged {
        node_id: String,
        replacement_id: String,
    },
    #[error(
        "node `{node_id}` parent precondition failed: expected {expected:?}, found {actual:?}"
    )]
    ParentMismatch {
        node_id: String,
        expected: Option<String>,
        actual: Option<String>,
    },
    #[error("node `{node_id}` index precondition failed: expected {expected}, found {actual}")]
    IndexMismatch {
        node_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("moving node `{node_id}` under `{new_parent_id}` would create a cycle")]
    MoveWouldCycle {
        node_id: String,
        new_parent_id: String,
    },
    #[error("new id `{id}` does not belong to required namespace `{namespace}`")]
    NamespaceViolation { id: String, namespace: String },
    #[error(
        "new id `{id}` uses counter {actual}, below the retained namespace high-water {minimum}"
    )]
    IdCounterBehind {
        id: String,
        actual: u64,
        minimum: u64,
    },
    #[error("namespace id counter is exhausted; cannot introduce `{id}`")]
    IdCounterExhausted { id: String },
    #[error("canonical hash failed: {0}")]
    CanonicalHash(#[from] CanonicalHashError),
}

/// A snapshot that is unsafe or inconsistent to install.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot contains a non-finite floating-point value")]
    NonFiniteNumber,
    #[error("snapshot must inline image sources instead of using file-table refs")]
    ExternalImageReference,
    #[error("snapshot document is invalid: {0}")]
    InvalidDocument(#[from] CollabApplyError),
    #[error("snapshot canonical hash failed: {0}")]
    CanonicalHash(#[from] CanonicalHashError),
    #[error("snapshot hash mismatch: expected {expected}, found {actual}")]
    HashMismatch {
        expected: CanonicalHash,
        actual: CanonicalHash,
    },
}
