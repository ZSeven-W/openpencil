use std::{collections::BTreeMap, sync::Arc};

use crate::{
    codec::{validate_profile, validate_wire_limits},
    CanonicalHash, ClientOpId, CollabApplyError, CollabMessage, Commit, CommitSeq, ConnectionKey,
    ConnectionPrincipal, OwnerEffect, OwnerSessionConfig, Participant, PeerNamespace, PeerProgress,
    Reject, RejectCode, SessionError, UndoResult, VerifiedAuthMetadata, MAX_COMMIT_LOG_BYTES,
    MAX_COMMIT_LOG_ENTRIES, MAX_RESULT_WINDOW_BYTES_PER_PEER, MAX_RESULT_WINDOW_ENTRIES,
    MAX_SESSION_PARTICIPANTS, MAX_TOTAL_RESULT_WINDOW_BYTES, MAX_UNDO_LEDGER_BYTES,
    MAX_UNDO_LEDGER_ENTRIES,
};

const SUBMIT_ENVELOPE_ALLOWANCE: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct CommitRecord {
    pub(crate) commit: Arc<Commit>,
    pub(crate) weight: usize,
}

#[derive(Clone)]
pub(crate) enum CachedSubmitResult {
    Commit(Arc<CommitRecord>),
    Reject { reject: Reject, weight: usize },
}

impl CachedSubmitResult {
    pub(crate) fn reply(&self, to: ConnectionKey) -> OwnerEffect {
        match self {
            Self::Commit(record) => OwnerEffect::ReplyCommit {
                to,
                commit: record.commit.clone(),
            },
            Self::Reject { reject, .. } => OwnerEffect::Reply {
                to,
                message: CollabMessage::Reject(reject.clone()),
            },
        }
    }

    pub(crate) fn weight(&self) -> usize {
        match self {
            Self::Commit(record) => record.weight,
            Self::Reject { weight, .. } => *weight,
        }
    }
}

#[derive(Clone)]
pub(crate) struct StoredSubmitResult {
    pub(crate) txn_hash: CanonicalHash,
    pub(crate) result: CachedSubmitResult,
}

#[derive(Clone)]
pub(crate) struct StoredUndoResult {
    pub(crate) target_client_op_id: ClientOpId,
    pub(crate) result: UndoResult,
    pub(crate) weight: usize,
}

pub(crate) struct PeerState {
    pub(crate) principal: ConnectionPrincipal,
    pub(crate) namespace: PeerNamespace,
    pub(crate) connection: Option<ConnectionKey>,
    pub(crate) next_counter: Option<u64>,
    pub(crate) next_id_counter: Option<u64>,
    pub(crate) retired_floor: u64,
    pub(crate) results: BTreeMap<u64, StoredSubmitResult>,
    pub(crate) result_bytes: usize,
    pub(crate) next_undo_counter: Option<u64>,
    pub(crate) undo_retired_floor: u64,
    pub(crate) undo_results: BTreeMap<u64, StoredUndoResult>,
    pub(crate) undo_result_bytes: usize,
    pub(crate) applied_through: CommitSeq,
}

impl PeerState {
    pub(crate) fn participant(&self) -> Participant {
        self.principal.roster_participant()
    }

    pub(crate) fn progress(&self) -> PeerProgress {
        PeerProgress {
            next_counter: self.next_counter,
            next_id_counter: self.next_id_counter,
            retired_floor: self.retired_floor,
            retained_results: self.results.len(),
            next_undo_counter: self.next_undo_counter,
            undo_retired_floor: self.undo_retired_floor,
            retained_undo_results: self.undo_results.len(),
            applied_through: self.applied_through,
            active: self.connection.is_some(),
        }
    }
}

pub(crate) enum CounterClass {
    Retained(StoredSubmitResult),
    Expired,
    Gap,
    Current,
}

pub(crate) enum UndoCounterClass {
    Retained(StoredUndoResult),
    Expired,
    Gap,
    Current,
}

pub(crate) fn reject_for(
    client_op_id: &ClientOpId,
    owner_seq: CommitSeq,
    code: RejectCode,
) -> Reject {
    Reject {
        client_op_id: client_op_id.clone(),
        owner_seq,
        code,
        details: None,
    }
}

pub(crate) fn reject_code_for_apply(error: &CollabApplyError) -> RejectCode {
    match error {
        CollabApplyError::PermissionDenied { .. }
        | CollabApplyError::NamespaceViolation { .. }
        | CollabApplyError::IdCounterBehind { .. }
        | CollabApplyError::IdCounterExhausted { .. } => RejectCode::PermissionDenied,
        CollabApplyError::TooManyOperations { .. }
        | CollabApplyError::SubtreeTooLarge { .. }
        | CollabApplyError::TooManyDocumentNodes { .. }
        | CollabApplyError::TreeTooDeep { .. }
        | CollabApplyError::IdentifierTooLong { .. }
        | CollabApplyError::ValidationBudgetExceeded { .. } => RejectCode::ResourceLimit,
        CollabApplyError::PageNotFound { .. }
        | CollabApplyError::NodeNotFound { .. }
        | CollabApplyError::ParentNotFound { .. }
        | CollabApplyError::ParentNotContainer { .. }
        | CollabApplyError::IndexOutOfBounds { .. }
        | CollabApplyError::HashMismatch { .. }
        | CollabApplyError::ReplacementIdChanged { .. }
        | CollabApplyError::ParentMismatch { .. }
        | CollabApplyError::IndexMismatch { .. }
        | CollabApplyError::MoveWouldCycle { .. } => RejectCode::PreconditionFailed,
        CollabApplyError::EmptyTransaction
        | CollabApplyError::NonFiniteNumber
        | CollabApplyError::EmptyId
        | CollabApplyError::DuplicateId { .. }
        | CollabApplyError::MixedDocumentRoots
        | CollabApplyError::CanonicalHash(_) => RejectCode::InvalidOperation,
    }
}

pub(crate) fn peer_to_owner_message(message: &CollabMessage) -> bool {
    matches!(
        message,
        CollabMessage::Submit(_)
            | CollabMessage::CatchUp(_)
            | CollabMessage::Applied(_)
            | CollabMessage::RenewTicket(_)
            | CollabMessage::UndoRequest(_)
            | CollabMessage::PresenceUpdate(_)
            | CollabMessage::Bye(_)
    )
}

pub(crate) fn validate_config(config: OwnerSessionConfig) -> Result<(), SessionError> {
    validate_wire_limits(config.wire_limits).map_err(SessionError::invalid_frame)?;
    let limits = config.session_limits;
    validate_usize_limit(
        "max_participants",
        limits.max_participants,
        1,
        MAX_SESSION_PARTICIPANTS,
    )?;
    validate_usize_limit(
        "result_window_entries",
        limits.result_window_entries,
        1,
        MAX_RESULT_WINDOW_ENTRIES,
    )?;
    let minimum_bytes = usize::try_from(config.wire_limits.max_txn_bytes)
        .unwrap_or(usize::MAX)
        .saturating_add(SUBMIT_ENVELOPE_ALLOWANCE);
    validate_usize_limit(
        "result_window_bytes_per_peer",
        limits.result_window_bytes_per_peer,
        minimum_bytes,
        MAX_RESULT_WINDOW_BYTES_PER_PEER,
    )?;
    validate_usize_limit(
        "commit_log_entries",
        limits.commit_log_entries,
        1,
        MAX_COMMIT_LOG_ENTRIES,
    )?;
    validate_usize_limit(
        "commit_log_bytes",
        limits.commit_log_bytes,
        minimum_bytes,
        MAX_COMMIT_LOG_BYTES,
    )?;
    validate_usize_limit(
        "undo_ledger_entries",
        limits.undo_ledger_entries,
        1,
        MAX_UNDO_LEDGER_ENTRIES,
    )?;
    validate_usize_limit(
        "undo_ledger_bytes",
        limits.undo_ledger_bytes,
        minimum_bytes,
        MAX_UNDO_LEDGER_BYTES,
    )?;
    if limits
        .max_participants
        .checked_mul(limits.result_window_bytes_per_peer)
        .and_then(|bytes| bytes.checked_mul(2))
        .is_none_or(|total| total > MAX_TOTAL_RESULT_WINDOW_BYTES)
    {
        return Err(SessionError::ResultWindowMemoryProduct {
            maximum: MAX_TOTAL_RESULT_WINDOW_BYTES,
        });
    }
    Ok(())
}

fn validate_usize_limit(
    field: &'static str,
    actual: usize,
    minimum: usize,
    maximum: usize,
) -> Result<(), SessionError> {
    if actual < minimum || actual > maximum {
        return Err(SessionError::InvalidConfig {
            field,
            actual,
            minimum,
            maximum,
        });
    }
    Ok(())
}

pub(crate) fn validate_principal(
    principal: &ConnectionPrincipal,
    config: OwnerSessionConfig,
) -> Result<(), SessionError> {
    let maximum = usize::try_from(config.wire_limits.max_identifier_bytes).unwrap_or(usize::MAX);
    for (field, value) in [
        ("participant_id", principal.participant_id().as_ref()),
        ("peer_id", principal.peer_id().as_ref()),
    ] {
        if value.is_empty() || value.len() > maximum {
            return Err(SessionError::InvalidAdmissionIdentifier { field });
        }
    }
    validate_auth_profile(principal.auth())
}

pub(crate) fn validate_auth_profile(auth: &VerifiedAuthMetadata) -> Result<(), SessionError> {
    validate_profile(
        auth.display_name.as_deref(),
        auth.avatar_url.as_deref(),
        "display_name",
        "avatar_url",
    )
    .map_err(|error| match error {
        crate::ProtocolError::InvalidParticipantProfile { field } => {
            SessionError::InvalidAdmissionProfile { field }
        }
        _ => unreachable!("profile validation returns only profile errors"),
    })
}

pub(crate) fn same_binding(left: &ConnectionPrincipal, right: &ConnectionPrincipal) -> bool {
    left.participant_id() == right.participant_id()
        && left.peer_id() == right.peer_id()
        && left.role() == right.role()
        && same_auth_binding(left.auth(), right.auth())
}

pub(crate) fn same_auth_binding(left: &VerifiedAuthMetadata, right: &VerifiedAuthMetadata) -> bool {
    left.issuer == right.issuer
        && left.subject == right.subject
        && left.device_id == right.device_id
        && left.proof_binding == right.proof_binding
}
