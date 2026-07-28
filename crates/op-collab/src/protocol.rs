use std::fmt;

use jian_ops_schema::{node::PenNode, PenDocument};
use serde::{Deserialize, Serialize, Serializer};
use zeroize::Zeroizing;

use crate::{CanonicalHash, OpaqueTicketError};

pub const COLLAB_PROTOCOL_VERSION: u16 = 1;
pub const MAX_ENVELOPE_BYTES: u32 = 64 * 1024 * 1024;
pub const MAX_TXN_BYTES: u32 = 4 * 1024 * 1024;
pub const MAX_OPS_PER_TXN: u32 = 1_024;
pub const MAX_PROCESSED_SUBTREE_NODES_PER_OP: u32 = 4_096;
pub const MAX_DOCUMENT_NODES: u32 = 100_000;
/// Conservative wire depth while the generated PenNode deserializer is recursive.
pub const MAX_TREE_DEPTH: u32 = 16;
pub const MAX_IDENTIFIER_BYTES: u32 = 1_024;
pub const MAX_PRESENCE_BYTES: u32 = 64 * 1024;
pub const MAX_VALIDATION_NODE_VISITS_PER_TXN: u32 = 2_000_000;
pub const MAX_OPAQUE_TICKET_BYTES: usize = 48 * 1024;
/// Maximum UTF-8 size of a verified collaboration display name.
pub const MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES: usize = 320;
/// Maximum Unicode scalar count of a verified collaboration display name.
pub const MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS: usize = 80;
/// Maximum UTF-8 size of a verified collaboration avatar URL.
pub const MAX_COLLAB_PROFILE_AVATAR_URL_BYTES: usize = 2_048;

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id!(SessionId);
string_id!(ParticipantId);
string_id!(PeerId);

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Epoch(pub u64);

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct CommitSeq(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClientOpId {
    pub peer_id: PeerId,
    pub local_counter: u64,
}

/// Idempotency key in an independent counter domain from [`ClientOpId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UndoRequestId {
    pub peer_id: PeerId,
    pub local_counter: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Editor,
    Viewer,
}

/// Verified identity output supplied by the host authentication layer.
///
/// This is metadata only. Ticket parsing, signatures, key rotation, and channel
/// binding live in the open host/auth bridge rather than this transport-free
/// crate. Only issuer/provider secrets remain outside the public repository.
#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedAuthMetadata {
    pub issuer: String,
    pub subject: String,
    pub device_id: String,
    pub proof_binding: String,
    pub expires_at_unix_ms: u64,
    /// Display name authenticated by the admission ticket.
    pub display_name: Option<String>,
    /// HTTPS avatar URL authenticated by the admission ticket.
    pub avatar_url: Option<String>,
}

impl fmt::Debug for VerifiedAuthMetadata {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedAuthMetadata")
            .field("issuer", &self.issuer)
            .field("subject", &"[REDACTED]")
            .field("device_id", &"[REDACTED]")
            .field("proof_binding", &"[REDACTED]")
            .field("expires_at_unix_ms", &self.expires_at_unix_ms)
            .field(
                "display_name",
                &self.display_name.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "avatar_url",
                &self.avatar_url.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

/// Trusted connection context created only after admission succeeds.
///
/// It intentionally has no serde implementation: an inbound frame cannot
/// self-assert a subject, device, participant id, or role.
#[derive(Clone, PartialEq, Eq)]
pub struct ConnectionPrincipal {
    auth: VerifiedAuthMetadata,
    participant_id: ParticipantId,
    peer_id: PeerId,
    role: Role,
}

impl fmt::Debug for ConnectionPrincipal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConnectionPrincipal")
            .field("auth", &self.auth)
            .field("participant_id", &"[REDACTED]")
            .field("peer_id", &"[REDACTED]")
            .field("role", &self.role)
            .finish()
    }
}

impl ConnectionPrincipal {
    pub fn from_verified(
        auth: VerifiedAuthMetadata,
        participant_id: ParticipantId,
        peer_id: PeerId,
        role: Role,
    ) -> Self {
        Self {
            auth,
            participant_id,
            peer_id,
            role,
        }
    }

    pub fn auth(&self) -> &VerifiedAuthMetadata {
        &self.auth
    }

    pub fn participant_id(&self) -> &ParticipantId {
        &self.participant_id
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn role(&self) -> Role {
        self.role
    }

    /// Verified public display name, if the issuer supplied one.
    pub fn display_name(&self) -> Option<&str> {
        self.auth.display_name.as_deref()
    }

    /// Verified public HTTPS avatar URL, if the issuer supplied one.
    pub fn avatar_url(&self) -> Option<&str> {
        self.auth.avatar_url.as_deref()
    }

    /// Project the trusted principal into the public epoch-local roster shape.
    pub fn roster_participant(&self) -> Participant {
        Participant {
            participant_id: self.participant_id.clone(),
            peer_id: self.peer_id.clone(),
            role: self.role,
            display_name: self.auth.display_name.clone(),
            avatar_url: self.auth.avatar_url.clone(),
        }
    }
}

/// A stable collaboration target for either legacy root children or a page.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PageRef {
    DocumentRoot,
    PageId(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum CollabOp {
    InsertExact {
        page: PageRef,
        parent_id: Option<String>,
        index: u32,
        node: PenNode,
    },
    ReplaceExact {
        page: PageRef,
        node_id: String,
        expected_hash: CanonicalHash,
        node: PenNode,
    },
    DeleteExact {
        page: PageRef,
        node_id: String,
        expected_hash: CanonicalHash,
    },
    MoveExact {
        page: PageRef,
        node_id: String,
        expected_parent: Option<String>,
        expected_index: u32,
        new_parent: Option<String>,
        new_index: u32,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollabTxn {
    pub ops: Vec<CollabOp>,
}

impl CollabTxn {
    pub fn new(ops: Vec<CollabOp>) -> Self {
        Self { ops }
    }
}

impl fmt::Debug for CollabTxn {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabTxn")
            .field("op_count", &self.ops.len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Submit {
    pub client_op_id: ClientOpId,
    pub base_seq: CommitSeq,
    pub txn: CollabTxn,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Commit {
    pub client_op_id: ClientOpId,
    pub seq: CommitSeq,
    pub author: CommitAuthor,
    pub txn: CollabTxn,
    pub doc_hash: CanonicalHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitAuthor {
    pub participant_id: ParticipantId,
    pub peer_id: PeerId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectCode {
    StaleBase,
    ExpiredClientOpId,
    CounterGap,
    PermissionDenied,
    PreconditionFailed,
    UnsupportedEdit,
    ResourceLimit,
    InvalidOperation,
    SessionChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reject {
    pub client_op_id: ClientOpId,
    pub owner_seq: CommitSeq,
    pub code: RejectCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatchUp {
    pub after_seq: CommitSeq,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Snapshot {
    pub seq: CommitSeq,
    pub document: PenDocument,
    pub doc_hash: CanonicalHash,
}

impl fmt::Debug for Snapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Snapshot")
            .field("seq", &self.seq)
            .field("doc_hash", &self.doc_hash)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Welcome {
    pub participant_id: ParticipantId,
    pub peer_id: PeerId,
    pub role: Role,
    pub seq: CommitSeq,
    pub peer_namespace: String,
    pub document_schema_version: String,
    pub hash_version: u16,
    pub limits: WireLimits,
    pub participants: Vec<Participant>,
}

/// Owner-authenticated participant metadata used for roster lifecycle.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Participant {
    pub participant_id: ParticipantId,
    pub peer_id: PeerId,
    pub role: Role,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

impl fmt::Debug for Participant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Participant")
            .field("participant_id", &self.participant_id)
            .field("peer_id", &self.peer_id)
            .field("role", &self.role)
            .field(
                "display_name",
                &self.display_name.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "avatar_url",
                &self.avatar_url.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

pub(crate) fn valid_profile_display_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES
        && value.chars().count() <= MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

pub(crate) fn valid_profile_avatar_url(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_COLLAB_PROFILE_AVATAR_URL_BYTES
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        || value.contains(['#', '\\'])
    {
        return false;
    }
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest
        .split_once(['/', '?'])
        .map_or(rest, |(authority, _)| authority);
    valid_https_authority(authority)
}

fn valid_https_authority(authority: &str) -> bool {
    if authority.is_empty() || authority.contains('@') {
        return false;
    }
    if let Some(bracketed) = authority.strip_prefix('[') {
        let Some(closing_bracket) = bracketed.find(']') else {
            return false;
        };
        let host = &bracketed[..closing_bracket];
        let suffix = &bracketed[closing_bracket + 1..];
        return host.parse::<std::net::Ipv6Addr>().is_ok()
            && (suffix.is_empty() || suffix.strip_prefix(':').is_some_and(valid_https_port));
    }
    if authority.contains('[') || authority.contains(']') {
        return false;
    }
    let host = match authority.rsplit_once(':') {
        Some((host, port)) => {
            if host.contains(':') || !valid_https_port(port) {
                return false;
            }
            host
        }
        None => authority,
    };
    valid_dns_or_ipv4_host(host)
}

fn valid_https_port(port: &str) -> bool {
    !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && port.parse::<u16>().is_ok_and(|port| port != 0)
}

fn valid_dns_or_ipv4_host(host: &str) -> bool {
    if host.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Applied {
    pub through_seq: CommitSeq,
}

/// Opaque refreshed admission credential.
///
/// The collaboration core only transports it. Parsing and verification stay
/// in the host authentication layer, and `Debug` never reveals its contents.
#[derive(PartialEq, Eq)]
pub struct OpaqueTicket(Zeroizing<String>);

impl OpaqueTicket {
    pub fn new(value: String) -> Result<Self, OpaqueTicketError> {
        Self::from_zeroizing(Zeroizing::new(value))
    }

    /// Copies validated UTF-8 bytes directly into zeroizing string storage.
    pub fn from_utf8_bytes(value: &[u8]) -> Result<Self, OpaqueTicketError> {
        let value = std::str::from_utf8(value).map_err(|_| OpaqueTicketError::InvalidUtf8)?;
        let mut owned = Zeroizing::new(String::with_capacity(value.len()));
        owned.push_str(value);
        Self::from_zeroizing(owned)
    }

    pub(crate) fn from_zeroizing(value: Zeroizing<String>) -> Result<Self, OpaqueTicketError> {
        if value.is_empty() {
            return Err(OpaqueTicketError::Empty);
        }
        if value.len() > MAX_OPAQUE_TICKET_BYTES {
            return Err(OpaqueTicketError::TooLarge {
                actual: value.len(),
                limit: MAX_OPAQUE_TICKET_BYTES,
            });
        }
        Ok(Self(value))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for OpaqueTicket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OpaqueTicket([REDACTED])")
    }
}

impl Serialize for OpaqueTicket {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(serde::ser::Error::custom(
            "opaque tickets require the dedicated renewal encoder",
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenewTicket {
    pub opaque_ticket: OpaqueTicket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UndoRequest {
    pub request_id: UndoRequestId,
    pub target_client_op_id: ClientOpId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoOutcome {
    Committed,
    Conflict,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UndoResult {
    pub request_id: UndoRequestId,
    pub target_client_op_id: ClientOpId,
    /// Owner sequence at which this result became authoritative.
    ///
    /// For a committed undo this is the compensation Commit sequence. It lets
    /// a reconnecting guest distinguish a compensation still to be replayed
    /// from one already included in a newer Snapshot.
    pub owner_seq: CommitSeq,
    pub outcome: UndoOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensation_client_op_id: Option<ClientOpId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Viewport {
    pub pan_x: f64,
    pub pan_y: f64,
    pub zoom: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Presence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Point>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selection: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editing_node: Option<String>,
}

/// Presence broadcast after the owner injects trusted connection identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParticipantPresence {
    pub participant_id: ParticipantId,
    pub peer_id: PeerId,
    pub presence: Presence,
}

/// Roster removal broadcast. Join metadata uses [`Participant`] directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParticipantLeft {
    pub participant_id: ParticipantId,
    pub peer_id: PeerId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ByeReason {
    Normal,
    OwnerLeft,
    AuthenticationExpired,
    ProtocolError,
    ResourceLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bye {
    pub reason: ByeReason,
}

#[derive(PartialEq, Serialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum CollabMessage {
    Welcome(Welcome),
    Submit(Submit),
    Commit(Commit),
    Reject(Reject),
    CatchUp(CatchUp),
    Snapshot(Box<Snapshot>),
    Applied(Applied),
    /// Bidirectional short-ticket renewal. The receiving transport verifies
    /// identity continuity and the existing Noise static-key binding.
    RenewTicket(RenewTicket),
    UndoRequest(UndoRequest),
    UndoResult(UndoResult),
    /// Client-to-owner update. Identity comes from the connection principal.
    PresenceUpdate(Presence),
    /// Owner-to-peers broadcast with trusted source identity.
    PresenceChanged(ParticipantPresence),
    ParticipantJoined(Participant),
    ParticipantLeft(ParticipantLeft),
    Bye(Bye),
}

impl CollabMessage {
    /// Explicitly duplicates only messages that cannot carry a credential.
    ///
    /// Keeping this separate from [`Clone`] makes accidental renewal-ticket
    /// fan-out a compile-time error while retaining efficient broadcast and
    /// simulation support for ordinary collaboration messages.
    pub fn try_clone_non_sensitive(&self) -> Option<Self> {
        Some(match self {
            Self::Welcome(message) => Self::Welcome(message.clone()),
            Self::Submit(message) => Self::Submit(message.clone()),
            Self::Commit(message) => Self::Commit(message.clone()),
            Self::Reject(message) => Self::Reject(message.clone()),
            Self::CatchUp(message) => Self::CatchUp(message.clone()),
            Self::Snapshot(message) => Self::Snapshot(message.clone()),
            Self::Applied(message) => Self::Applied(message.clone()),
            Self::RenewTicket(_) => return None,
            Self::UndoRequest(message) => Self::UndoRequest(message.clone()),
            Self::UndoResult(message) => Self::UndoResult(message.clone()),
            Self::PresenceUpdate(message) => Self::PresenceUpdate(message.clone()),
            Self::PresenceChanged(message) => Self::PresenceChanged(message.clone()),
            Self::ParticipantJoined(message) => Self::ParticipantJoined(message.clone()),
            Self::ParticipantLeft(message) => Self::ParticipantLeft(message.clone()),
            Self::Bye(message) => Self::Bye(message.clone()),
        })
    }
}

impl fmt::Debug for CollabMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollabMessage")
            .field("kind", &self.kind())
            .finish()
    }
}

#[derive(PartialEq)]
pub struct FrameEnvelope {
    pub(crate) protocol_version: u16,
    pub(crate) session_id: SessionId,
    pub(crate) epoch: Epoch,
    pub(crate) body: CollabMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WireLimits {
    pub max_envelope_bytes: u32,
    pub max_txn_bytes: u32,
    pub max_ops_per_txn: u32,
    pub max_processed_subtree_nodes_per_op: u32,
    pub max_document_nodes: u32,
    pub max_tree_depth: u32,
    pub max_identifier_bytes: u32,
    pub max_presence_bytes: u32,
    pub max_validation_node_visits_per_txn: u32,
}

impl Default for WireLimits {
    fn default() -> Self {
        Self {
            max_envelope_bytes: MAX_ENVELOPE_BYTES,
            max_txn_bytes: MAX_TXN_BYTES,
            max_ops_per_txn: MAX_OPS_PER_TXN,
            max_processed_subtree_nodes_per_op: MAX_PROCESSED_SUBTREE_NODES_PER_OP,
            max_document_nodes: MAX_DOCUMENT_NODES,
            max_tree_depth: MAX_TREE_DEPTH,
            max_identifier_bytes: MAX_IDENTIFIER_BYTES,
            max_presence_bytes: MAX_PRESENCE_BYTES,
            max_validation_node_visits_per_txn: MAX_VALIDATION_NODE_VISITS_PER_TXN,
        }
    }
}
