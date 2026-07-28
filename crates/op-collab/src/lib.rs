//! Transport-free collaboration primitives for OpenPencil.
//!
//! The crate is intentionally platform-neutral and safe to include in the
//! wasm dependency graph. It defines only versioned wire data, deterministic
//! hashing, exact document mutations, and the trust-boundary types used by a
//! host-supplied authentication implementation.

mod apply;
mod apply_budget;
mod apply_context;
mod apply_tree;
mod codec;
mod diff;
mod diff_fields;
mod diff_index;
mod diff_structure;
mod diff_types;
mod error;
mod finite;
mod guest;
mod guest_pending;
mod guest_sync;
mod guest_types;
mod hash;
mod id_high_water;
mod protocol;
mod serde_context;
mod session;
mod session_roster;
mod session_state;
mod session_submit;
mod session_sync;
mod session_types;
mod session_undo;
mod snapshot;
mod ticket_json;
mod typed_images;
mod undo_ledger;

pub use apply::{apply_txn, apply_txn_in_place, apply_txn_tracked, validate_document, AppliedTxn};
pub use apply_context::{ApplyContext, ApplyLimits, InsertPolicy};
pub use codec::{
    encode_commit_frame_to_json_vec, encode_renew_ticket_frame_to_zeroizing_json,
    SensitiveFrameJson, WireEnvelope,
};
pub use diff::diff_supported;
pub use diff_types::{
    DiffContext, DiffError, EditChanges, FieldValue, NodeFieldChange, StructuralChanges,
    StructuralMove, SupportedDiff, SupportedNodeField,
};
pub use error::{
    CanonicalHashError, CanonicalHashParseError, CollabApplyError, OpaqueTicketError,
    ProtocolError, SnapshotError,
};
pub use guest::GuestSessionCore;
pub use guest_types::{
    GuestConnectionState, GuestEffect, GuestError, GuestInstallReason, GuestSessionConfig,
    PendingCancelReason, PendingEdit, PendingEditStatus, PreparedGuestInstall,
    DEFAULT_GUEST_COMMIT_BUFFER_BYTES, DEFAULT_GUEST_COMMIT_BUFFER_ENTRIES,
    DEFAULT_GUEST_UNDO_INDEX_ENTRIES, MAX_GUEST_COMMIT_BUFFER_BYTES,
    MAX_GUEST_COMMIT_BUFFER_ENTRIES, MAX_GUEST_UNDO_INDEX_ENTRIES,
};
pub use hash::{
    canonical_document_hash, canonical_json, canonical_node_hash, canonical_txn_hash,
    CanonicalHash, CANONICAL_HASH_ALGORITHM, CANONICAL_HASH_VERSION, DOCUMENT_HASH_DOMAIN,
    NODE_HASH_DOMAIN, TXN_HASH_DOMAIN,
};
pub use op_util::collab_id::{CollabIdError, NamespacedId, PeerNamespace, MAX_PEER_NAMESPACE_LEN};
pub use protocol::{
    Applied, Bye, ByeReason, CatchUp, ClientOpId, CollabMessage, CollabOp, CollabTxn, Commit,
    CommitAuthor, CommitSeq, ConnectionPrincipal, Epoch, FrameEnvelope, OpaqueTicket, PageRef,
    Participant, ParticipantId, ParticipantLeft, ParticipantPresence, PeerId, Point, Presence,
    Reject, RejectCode, RenewTicket, Role, SessionId, Snapshot, Submit, UndoOutcome, UndoRequest,
    UndoRequestId, UndoResult, VerifiedAuthMetadata, Viewport, Welcome, WireLimits,
    COLLAB_PROTOCOL_VERSION, MAX_COLLAB_PROFILE_AVATAR_URL_BYTES,
    MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES, MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS,
    MAX_DOCUMENT_NODES, MAX_ENVELOPE_BYTES, MAX_IDENTIFIER_BYTES, MAX_OPAQUE_TICKET_BYTES,
    MAX_OPS_PER_TXN, MAX_PRESENCE_BYTES, MAX_PROCESSED_SUBTREE_NODES_PER_OP, MAX_TREE_DEPTH,
    MAX_TXN_BYTES, MAX_VALIDATION_NODE_VISITS_PER_TXN,
};
pub use session::OwnerSessionCore;
pub use session_types::{
    AdmissionGrant, BoundUndoRequest, ConnectionKey, OwnerEffect, OwnerSessionConfig,
    PeerActivation, PeerProgress, PreparedCommit, SessionError, SessionLimits,
    DEFAULT_COMMIT_LOG_BYTES, DEFAULT_COMMIT_LOG_ENTRIES, DEFAULT_MAX_PARTICIPANTS,
    DEFAULT_RESULT_WINDOW_BYTES_PER_PEER, DEFAULT_RESULT_WINDOW_ENTRIES, DEFAULT_UNDO_LEDGER_BYTES,
    DEFAULT_UNDO_LEDGER_ENTRIES, FIRST_CLIENT_OP_COUNTER, MAX_COMMIT_LOG_BYTES,
    MAX_COMMIT_LOG_ENTRIES, MAX_RESULT_WINDOW_BYTES_PER_PEER, MAX_RESULT_WINDOW_ENTRIES,
    MAX_SESSION_PARTICIPANTS, MAX_TOTAL_RESULT_WINDOW_BYTES, MAX_UNDO_LEDGER_BYTES,
    MAX_UNDO_LEDGER_ENTRIES,
};
pub use snapshot::verify_snapshot;
pub use ticket_json::decode_renew_ticket_frame_from_json_slice;
