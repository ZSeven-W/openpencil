use std::{fmt, sync::Arc};

use jian_ops_schema::PenDocument;

use crate::{
    CanonicalHash, CanonicalHashError, ClientOpId, CollabApplyError, CollabIdError, CollabMessage,
    CollabTxn, CommitSeq, DiffError, EditChanges, OpaqueTicket, Participant, ParticipantLeft,
    ParticipantPresence, ProtocolError, Reject, RejectCode, SnapshotError, UndoOutcome,
    UndoRequestId, UndoResult,
};

pub const DEFAULT_GUEST_COMMIT_BUFFER_ENTRIES: usize = 64;
pub const DEFAULT_GUEST_COMMIT_BUFFER_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_GUEST_COMMIT_BUFFER_ENTRIES: usize = 4_096;
pub const MAX_GUEST_COMMIT_BUFFER_BYTES: usize = 256 * 1024 * 1024;
pub const DEFAULT_GUEST_UNDO_INDEX_ENTRIES: usize = 256;
pub const MAX_GUEST_UNDO_INDEX_ENTRIES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestSessionConfig {
    pub commit_buffer_entries: usize,
    pub commit_buffer_bytes: usize,
    pub undo_index_entries: usize,
}

impl Default for GuestSessionConfig {
    fn default() -> Self {
        Self {
            commit_buffer_entries: DEFAULT_GUEST_COMMIT_BUFFER_ENTRIES,
            commit_buffer_bytes: DEFAULT_GUEST_COMMIT_BUFFER_BYTES,
            undo_index_entries: DEFAULT_GUEST_UNDO_INDEX_ENTRIES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestConnectionState {
    AwaitingSnapshot,
    Active,
    Disconnected,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestInstallReason {
    Snapshot,
    RemoteCommit,
    OwnCommit,
    UndoCompensation,
    RejectRollback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingEditStatus {
    Submitted,
    AwaitingCatchUp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingCancelReason {
    PropertyConflict { node_id: String, field: String },
    StructuralConflict,
    Rejected(RejectCode),
    AlreadySatisfied,
}

#[derive(Clone)]
pub struct PendingEdit {
    pub(crate) client_op_id: ClientOpId,
    pub(crate) base_seq: CommitSeq,
    pub(crate) before: Arc<PenDocument>,
    pub(crate) desired_after: Arc<PenDocument>,
    pub(crate) submitted_txn: CollabTxn,
    pub(crate) changes: EditChanges,
    pub(crate) status: PendingEditStatus,
    pub(crate) deferred_reject: Option<Reject>,
    pub(crate) id_counter_before: Option<u64>,
}

impl PendingEdit {
    pub fn client_op_id(&self) -> &ClientOpId {
        &self.client_op_id
    }

    pub const fn base_seq(&self) -> CommitSeq {
        self.base_seq
    }

    pub fn before_document(&self) -> &PenDocument {
        &self.before
    }

    pub fn desired_document(&self) -> &PenDocument {
        &self.desired_after
    }

    pub fn submitted_txn(&self) -> &CollabTxn {
        &self.submitted_txn
    }

    pub fn changes(&self) -> &EditChanges {
        &self.changes
    }

    pub const fn status(&self) -> PendingEditStatus {
        self.status
    }
}

impl fmt::Debug for PendingEdit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PendingEdit")
            .field("client_op_id", &self.client_op_id)
            .field("base_seq", &self.base_seq)
            .field("submitted_txn", &self.submitted_txn)
            .field("changes", &self.changes)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedGuestKey {
    pub generation: u64,
    pub seq: CommitSeq,
    pub candidate_hash: CanonicalHash,
}

pub(crate) struct GuestPostInstall {
    pub confirmed_document: Arc<PenDocument>,
    pub confirmed_seq: CommitSeq,
    pub confirmed_hash: CanonicalHash,
    pub displayed_document: Arc<PenDocument>,
    pub pending: Option<PendingEdit>,
    pub state: GuestConnectionState,
    pub discard_buffer_through: Option<CommitSeq>,
    pub pending_cancel: Option<(ClientOpId, PendingCancelReason)>,
    pub undo_index_add: Option<UndoIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UndoIndexEntry {
    pub client_op_id: ClientOpId,
    pub seq: CommitSeq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExpectedCompensation {
    pub client_op_id: ClientOpId,
    pub seq: CommitSeq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletedUndoResult {
    request_id: UndoRequestId,
    target_client_op_id: ClientOpId,
    owner_seq: CommitSeq,
    outcome: UndoOutcome,
    compensation_client_op_id: Option<ClientOpId>,
    details_hash: [u8; 32],
}

impl CompletedUndoResult {
    pub(crate) fn from_result(result: &UndoResult) -> Self {
        Self {
            request_id: result.request_id.clone(),
            target_client_op_id: result.target_client_op_id.clone(),
            owner_seq: result.owner_seq,
            outcome: result.outcome,
            compensation_client_op_id: result.compensation_client_op_id.clone(),
            details_hash: undo_details_hash(result.details.as_deref()),
        }
    }

    pub(crate) fn has_request_id(&self, request_id: &UndoRequestId) -> bool {
        &self.request_id == request_id
    }

    pub(crate) fn matches(&self, result: &UndoResult) -> bool {
        self.request_id == result.request_id
            && self.target_client_op_id == result.target_client_op_id
            && self.owner_seq == result.owner_seq
            && self.outcome == result.outcome
            && self.compensation_client_op_id == result.compensation_client_op_id
            && self.details_hash == undo_details_hash(result.details.as_deref())
    }
}

fn undo_details_hash(details: Option<&str>) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    match details {
        Some(details) => {
            hasher.update(&[1]);
            hasher.update(details.as_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
    *hasher.finalize().as_bytes()
}

/// Candidate editor document that must be installed before guest state
/// advances. The token is intentionally not cloneable.
pub struct PreparedGuestInstall {
    pub(crate) key: PreparedGuestKey,
    pub(crate) reason: GuestInstallReason,
    pub(crate) candidate_document: Option<PenDocument>,
    pub(crate) post: GuestPostInstall,
}

impl PreparedGuestInstall {
    pub fn candidate_document(&self) -> Option<&PenDocument> {
        self.candidate_document.as_ref()
    }

    pub fn take_candidate_document(&mut self) -> Option<PenDocument> {
        self.candidate_document.take()
    }

    pub const fn candidate_hash(&self) -> CanonicalHash {
        self.key.candidate_hash
    }

    pub const fn confirmed_seq(&self) -> CommitSeq {
        self.key.seq
    }

    pub const fn reason(&self) -> GuestInstallReason {
        self.reason
    }
}

impl fmt::Debug for PreparedGuestInstall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedGuestInstall")
            .field("generation", &self.key.generation)
            .field("confirmed_seq", &self.key.seq)
            .field("candidate_hash", &self.key.candidate_hash)
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum GuestEffect {
    Send(CollabMessage),
    PrepareInstall(Box<PreparedGuestInstall>),
    ParticipantJoined(Participant),
    ParticipantLeft(ParticipantLeft),
    PresenceChanged(ParticipantPresence),
    PendingCancelled {
        client_op_id: ClientOpId,
        reason: PendingCancelReason,
    },
    /// Verify an owner renewal against the existing Noise-bound transport
    /// identity before replacing that connection's expiry deadline.
    VerifyRenewal {
        ticket: OpaqueTicket,
    },
    UndoResult(UndoResult),
    SessionEnded {
        reason: crate::ByeReason,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum GuestError {
    #[error(
        "guest session configuration `{field}` is invalid: {actual} (expected {minimum}..={maximum})"
    )]
    InvalidConfig {
        field: &'static str,
        actual: usize,
        minimum: usize,
        maximum: usize,
    },
    #[error("welcome frame is invalid: {0}")]
    InvalidWelcomeFrame(#[source] ProtocolError),
    #[error("welcome metadata is invalid: {reason}")]
    InvalidWelcome { reason: &'static str },
    #[error("welcome peer namespace is invalid: {0}")]
    InvalidNamespace(#[from] CollabIdError),
    #[error("collaboration frame failed bounded validation: {0}")]
    InvalidFrame(#[source] ProtocolError),
    #[error("collaboration frame belongs to a different session")]
    WrongSession,
    #[error("collaboration frame belongs to a different epoch")]
    WrongEpoch,
    #[error("message `{kind}` is not allowed from owner to guest")]
    WrongDirection { kind: &'static str },
    #[error("guest session is waiting for its authenticated snapshot")]
    AwaitingSnapshot,
    #[error("guest is disconnected and read-only")]
    Disconnected,
    #[error("guest collaboration session has ended")]
    SessionEnded,
    #[error("viewer role cannot mutate the collaboration document")]
    ViewerReadOnly,
    #[error("one local document edit is already pending")]
    PendingEditExists,
    #[error("guest has no confirmed collaboration document")]
    NoConfirmedDocument,
    #[error("guest client operation counter is exhausted")]
    ClientCounterExhausted,
    #[error("guest undo request counter is exhausted")]
    UndoCounterExhausted,
    #[error("selective undo target does not belong to this guest peer")]
    InvalidUndoTarget,
    #[error("selective undo target is not an undoable committed property edit")]
    UnknownUndoTarget,
    #[error("one selective undo request is already pending")]
    UndoPending,
    #[error("owner undo result does not match the pending request")]
    UndoResultMismatch,
    #[error("owner undo compensation does not match the reserved client operation")]
    UndoCompensationMismatch,
    #[error("a document installation is already awaiting confirmation")]
    InstallPending,
    #[error("prepared guest install does not match the active installation")]
    PreparedInstallMismatch,
    #[error("installed document hash differs from the prepared candidate")]
    InstalledHashMismatch {
        expected: CanonicalHash,
        actual: CanonicalHash,
    },
    #[error("snapshot is invalid: {0}")]
    InvalidSnapshot(#[from] SnapshotError),
    #[error("snapshot document schema differs from Welcome")]
    SnapshotSchemaMismatch,
    #[error("snapshot sequence predates guest confirmed state")]
    SnapshotSequenceRegressed,
    #[error("initial authenticated snapshot sequence differs from Welcome")]
    InitialSnapshotSequenceMismatch,
    #[error("initial authenticated snapshot already contains the guest namespace")]
    NamespaceAlreadyPresentInInitialSnapshot,
    #[error("commit sequence zero is invalid")]
    InvalidCommitSequence,
    #[error("two different commits claim sequence {seq:?}")]
    CommitSequenceCollision { seq: CommitSeq },
    #[error("commit author does not match the authenticated roster")]
    UnknownCommitAuthor,
    #[error("owner rejection does not target this guest peer")]
    RejectPeerMismatch,
    #[error("own authoritative commit result differs from the pending local edit")]
    PendingCommitMismatch,
    #[error("commit replay hash differs from the owner-declared hash")]
    CommitHashMismatch {
        expected: CanonicalHash,
        actual: CanonicalHash,
    },
    #[error("buffered commit count exceeds {limit}")]
    CommitBufferEntriesExceeded { limit: usize },
    #[error("buffered commit bytes exceed {limit}")]
    CommitBufferBytesExceeded { limit: usize },
    #[error("participant roster update conflicts with retained identity")]
    RosterConflict,
    #[error("collaboration edit is not supported: {0}")]
    UnsupportedEdit(#[from] DiffError),
    #[error("collaboration transaction replay failed: {0}")]
    Apply(#[from] CollabApplyError),
    #[error("collaboration hashing failed: {0}")]
    CanonicalHash(#[from] CanonicalHashError),
}
