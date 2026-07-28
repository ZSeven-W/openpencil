use std::{fmt, num::NonZeroU64, sync::Arc};

use jian_ops_schema::PenDocument;

use crate::{
    CanonicalHash, CanonicalHashError, CollabApplyError, CollabMessage, Commit, CommitSeq,
    ConnectionPrincipal, EditChanges, OpaqueTicket, Participant, PeerId, PeerNamespace,
    ProtocolError, Snapshot, UndoRequest, UndoRequestId, UndoResult, Welcome, WireLimits,
};

pub const FIRST_CLIENT_OP_COUNTER: u64 = 1;
pub const DEFAULT_MAX_PARTICIPANTS: usize = 16;
pub const DEFAULT_RESULT_WINDOW_ENTRIES: usize = 128;
pub const DEFAULT_RESULT_WINDOW_BYTES_PER_PEER: usize = 8 * 1024 * 1024;
pub const DEFAULT_COMMIT_LOG_ENTRIES: usize = 64;
pub const DEFAULT_COMMIT_LOG_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_UNDO_LEDGER_ENTRIES: usize = 512;
pub const DEFAULT_UNDO_LEDGER_BYTES: usize = 64 * 1024 * 1024;

pub const MAX_SESSION_PARTICIPANTS: usize = 64;
pub const MAX_RESULT_WINDOW_ENTRIES: usize = 4_096;
pub const MAX_RESULT_WINDOW_BYTES_PER_PEER: usize = 64 * 1024 * 1024;
pub const MAX_TOTAL_RESULT_WINDOW_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_COMMIT_LOG_ENTRIES: usize = 4_096;
pub const MAX_COMMIT_LOG_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_UNDO_LEDGER_ENTRIES: usize = 4_096;
pub const MAX_UNDO_LEDGER_BYTES: usize = 256 * 1024 * 1024;

/// Host-local connection identity. It is never serialized onto the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionKey(NonZeroU64);

impl ConnectionKey {
    pub fn new(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self)
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }
}

/// Session-owned memory and roster limits that are separate from wire limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLimits {
    pub max_participants: usize,
    pub result_window_entries: usize,
    pub result_window_bytes_per_peer: usize,
    pub commit_log_entries: usize,
    pub commit_log_bytes: usize,
    pub undo_ledger_entries: usize,
    pub undo_ledger_bytes: usize,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_participants: DEFAULT_MAX_PARTICIPANTS,
            result_window_entries: DEFAULT_RESULT_WINDOW_ENTRIES,
            result_window_bytes_per_peer: DEFAULT_RESULT_WINDOW_BYTES_PER_PEER,
            commit_log_entries: DEFAULT_COMMIT_LOG_ENTRIES,
            commit_log_bytes: DEFAULT_COMMIT_LOG_BYTES,
            undo_ledger_entries: DEFAULT_UNDO_LEDGER_ENTRIES,
            undo_ledger_bytes: DEFAULT_UNDO_LEDGER_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OwnerSessionConfig {
    pub wire_limits: WireLimits,
    pub session_limits: SessionLimits,
}

/// Admission output created by a host only after authentication and owner ACL.
#[derive(Clone, PartialEq, Eq)]
pub struct AdmissionGrant {
    principal: ConnectionPrincipal,
    peer_namespace: PeerNamespace,
}

impl AdmissionGrant {
    pub fn new(principal: ConnectionPrincipal, peer_namespace: PeerNamespace) -> Self {
        Self {
            principal,
            peer_namespace,
        }
    }

    pub fn principal(&self) -> &ConnectionPrincipal {
        &self.principal
    }

    pub fn peer_namespace(&self) -> &PeerNamespace {
        &self.peer_namespace
    }

    pub(crate) fn into_parts(self) -> (ConnectionPrincipal, PeerNamespace) {
        (self.principal, self.peer_namespace)
    }
}

impl fmt::Debug for AdmissionGrant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmissionGrant")
            .field("principal", &self.principal)
            .field("peer_namespace", &"[REDACTED]")
            .finish()
    }
}

/// Atomic activation output captured at one owner-actor state boundary.
#[derive(Debug)]
pub struct PeerActivation {
    pub connection: ConnectionKey,
    pub welcome: Welcome,
    pub snapshot: Option<Snapshot>,
    pub joined: Participant,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum PreparedSourceKey {
    Submit,
    Undo {
        request_id: UndoRequestId,
        target_client_op_id: crate::ClientOpId,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct PreparedKey {
    pub peer_id: PeerId,
    pub counter: u64,
    pub txn_hash: CanonicalHash,
    pub expected_seq: CommitSeq,
    pub source: PreparedSourceKey,
}

pub(crate) struct PreparedUndoFinalize {
    pub reply_to: ConnectionKey,
    pub result: UndoResult,
    pub reverted_fields: Vec<crate::undo_ledger::FieldKey>,
}

/// Candidate document that must be installed before its Commit is finalized.
///
/// This token is intentionally not `Clone`. The owner actor may either abort
/// it before installation or confirm exactly one installation.
pub struct PreparedCommit {
    pub(crate) key: PreparedKey,
    pub(crate) candidate_document: Option<PenDocument>,
    pub(crate) commit: Commit,
    pub(crate) encoded_commit_size: usize,
    pub(crate) next_id_counter: Option<u64>,
    pub(crate) changes: EditChanges,
    pub(crate) undo: Option<PreparedUndoFinalize>,
}

impl PreparedCommit {
    pub fn candidate_document(&self) -> Option<&PenDocument> {
        self.candidate_document.as_ref()
    }

    pub fn take_candidate_document(&mut self) -> Option<PenDocument> {
        self.candidate_document.take()
    }

    pub fn candidate_hash(&self) -> CanonicalHash {
        self.commit.doc_hash
    }

    pub fn commit_seq(&self) -> CommitSeq {
        self.commit.seq
    }
}

impl fmt::Debug for PreparedCommit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedCommit")
            .field("counter", &self.key.counter)
            .field("commit_seq", &self.commit.seq)
            .field("candidate_hash", &self.commit.doc_hash)
            .field("encoded_commit_size", &self.encoded_commit_size)
            .field("next_id_counter", &self.next_id_counter)
            .finish_non_exhaustive()
    }
}

/// Undo request with identity injected from the active connection registry.
#[derive(Debug)]
pub struct BoundUndoRequest {
    pub participant: Participant,
    pub request: UndoRequest,
}

/// Read-only peer sequencing state for diagnostics and host integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerProgress {
    pub next_counter: Option<u64>,
    pub next_id_counter: Option<u64>,
    pub retired_floor: u64,
    pub retained_results: usize,
    pub next_undo_counter: Option<u64>,
    pub undo_retired_floor: u64,
    pub retained_undo_results: usize,
    pub applied_through: CommitSeq,
    pub active: bool,
}

/// Transport-neutral effects emitted by the owner state machine.
#[derive(Debug)]
pub enum OwnerEffect {
    Reply {
        to: ConnectionKey,
        message: CollabMessage,
    },
    /// A cached authoritative Commit reply sharing the retained commit record.
    ReplyCommit {
        to: ConnectionKey,
        commit: Arc<Commit>,
    },
    Broadcast {
        message: CollabMessage,
    },
    /// A finalized authoritative Commit shared by every outbound peer.
    BroadcastCommit {
        commit: Arc<Commit>,
    },
    CommitBatch {
        to: ConnectionKey,
        commits: Vec<Arc<Commit>>,
    },
    Snapshot {
        to: ConnectionKey,
        snapshot: Box<Snapshot>,
    },
    PrepareInstall(Box<PreparedCommit>),
    /// A finalized selective undo. The host sends `result` to `reply_to`,
    /// then broadcasts the authoritative compensation Commit.
    UndoCommitted {
        reply_to: ConnectionKey,
        result: UndoResult,
        commit: Arc<Commit>,
    },
    VerifyRenewal {
        connection: ConnectionKey,
        ticket: OpaqueTicket,
    },
    UndoRequested(BoundUndoRequest),
    Close {
        connection: ConnectionKey,
        reason: crate::ByeReason,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(
        "owner session configuration `{field}` is invalid: {actual} (expected {minimum}..={maximum})"
    )]
    InvalidConfig {
        field: &'static str,
        actual: usize,
        minimum: usize,
        maximum: usize,
    },
    #[error("owner session result-window memory product exceeds {maximum} bytes")]
    ResultWindowMemoryProduct { maximum: usize },
    #[error("initial collaboration document is invalid: {0}")]
    InvalidInitialDocument(#[source] CollabApplyError),
    #[error("collaboration hashing failed: {0}")]
    CanonicalHash(#[from] CanonicalHashError),
    #[error("collaboration frame failed bounded validation: {0}")]
    InvalidFrame(#[source] ProtocolError),
    #[error("collaboration frame belongs to a different session")]
    WrongSession,
    #[error("collaboration frame belongs to a different epoch")]
    WrongEpoch,
    #[error("message `{kind}` is not allowed from peer to owner")]
    WrongDirection { kind: &'static str },
    #[error("connection is not active in this owner session")]
    UnknownConnection,
    #[error("connection is closing and cannot submit more frames")]
    ConnectionClosing,
    #[error("connection is already bound in this owner session")]
    ConnectionAlreadyBound,
    #[error("owner session has ended")]
    SessionEnded,
    #[error("owner session must start with exactly one owner grant")]
    InvalidOwnerGrant,
    #[error("remote admission cannot add another owner")]
    MultipleOwners,
    #[error("peer is already active; use same-epoch resume")]
    PeerAlreadyActive,
    #[error("peer already exists; use same-epoch resume")]
    PeerAlreadyExists,
    #[error("participant id is already retained by this epoch")]
    DuplicateParticipant,
    #[error("peer namespace is already retained by this epoch")]
    DuplicateNamespace,
    #[error("peer namespace already appears in the collaboration document")]
    NamespaceAlreadyPresentInDocument,
    #[error("session participant limit has been reached")]
    ParticipantLimit,
    #[error("admission identifier `{field}` is empty or too long")]
    InvalidAdmissionIdentifier { field: &'static str },
    #[error("verified admission profile field `{field}` is invalid")]
    InvalidAdmissionProfile { field: &'static str },
    #[error("resume grant does not match the retained peer binding")]
    ResumeBindingMismatch,
    #[error("client operation peer id does not match its active connection")]
    ClientOpPeerMismatch,
    #[error("retained client operation id was reused with a different transaction")]
    ClientOpReuse,
    #[error("retained undo request id was reused with a different target")]
    UndoRequestReuse,
    #[error("owner undo request counter is exhausted")]
    UndoCounterExhausted,
    #[error("owner selective undo target is not an available committed property edit")]
    OwnUndoTargetUnavailable,
    #[error("client base sequence is ahead of the owner sequence")]
    FutureBaseSequence,
    #[error("catch-up sequence is ahead of the owner sequence")]
    FutureCatchUpSequence,
    #[error("applied sequence is ahead of the owner sequence")]
    FutureAppliedSequence,
    #[error("applied sequence regressed for this peer")]
    AppliedSequenceRegressed,
    #[error("owner sequence is exhausted")]
    SequenceExhausted,
    #[error("another document installation is awaiting confirmation")]
    InstallPending,
    #[error("prepared commit does not match the active installation")]
    PreparedCommitMismatch,
    #[error("installed document hash differs from the prepared candidate")]
    InstalledHashMismatch {
        expected: CanonicalHash,
        actual: CanonicalHash,
    },
    #[error("live owner document drifted from the last finalized collaboration state")]
    DocumentDrift,
    #[error("selective undo metadata could not be interpreted: {0}")]
    UndoDiff(#[source] crate::DiffError),
    #[error("selective undo compensation could not be applied: {0}")]
    UndoApply(#[source] CollabApplyError),
    #[error("renewed authentication identity or channel binding changed")]
    RenewalBindingMismatch,
    #[error("renewed ticket does not extend the retained expiry")]
    RenewalDidNotExtend,
    #[error("peer does not exist in this owner session")]
    UnknownPeer,
}

impl SessionError {
    pub(crate) fn invalid_frame(error: ProtocolError) -> Self {
        Self::InvalidFrame(error)
    }
}
