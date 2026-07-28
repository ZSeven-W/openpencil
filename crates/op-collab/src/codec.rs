use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::{
    apply::validate_txn_resource_shape,
    finite::validate_finite,
    protocol::{valid_profile_avatar_url, valid_profile_display_name},
    serde_context::{frame_has_external_image_ref, inline_frame, inline_txn, to_isolated_value},
    ticket_json::reject_renew_ticket_before_generic_value_decode,
    typed_images, verify_snapshot, Applied, ApplyLimits, Bye, CatchUp, ClientOpId, CollabMessage,
    CollabTxn, Commit, Epoch, FrameEnvelope, OpaqueTicket, Participant, ParticipantId,
    ParticipantLeft, ParticipantPresence, PeerId, PeerNamespace, Presence, ProtocolError, Reject,
    SessionId, Snapshot, Submit, UndoRequest, UndoRequestId, UndoResult, Welcome, WireLimits,
    CANONICAL_HASH_VERSION, COLLAB_PROTOCOL_VERSION, MAX_DOCUMENT_NODES, MAX_ENVELOPE_BYTES,
    MAX_IDENTIFIER_BYTES, MAX_OPS_PER_TXN, MAX_PRESENCE_BYTES, MAX_PROCESSED_SUBTREE_NODES_PER_OP,
    MAX_TREE_DEPTH, MAX_TXN_BYTES, MAX_VALIDATION_NODE_VISITS_PER_TXN,
};

impl fmt::Debug for FrameEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FrameEnvelope")
            .field("protocol_version", &self.protocol_version)
            .field("session_id", &self.session_id)
            .field("epoch", &self.epoch)
            .field("body_kind", &self.body.kind())
            .finish()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawFrameEnvelopeRef<'a> {
    protocol_version: u16,
    session_id: &'a SessionId,
    epoch: Epoch,
    body: &'a CollabMessage,
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum RawCommitMessageRef<'a> {
    Commit(&'a Commit),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawCommitFrameEnvelopeRef<'a> {
    protocol_version: u16,
    session_id: &'a SessionId,
    epoch: Epoch,
    body: RawCommitMessageRef<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawRenewTicketRef<'a> {
    opaque_ticket: DedicatedOpaqueTicketRef<'a>,
}

struct DedicatedOpaqueTicketRef<'a>(&'a OpaqueTicket);

impl Serialize for DedicatedOpaqueTicketRef<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.expose())
    }
}

#[derive(Serialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum RawRenewTicketMessageRef<'a> {
    RenewTicket(RawRenewTicketRef<'a>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawRenewTicketFrameEnvelopeRef<'a> {
    protocol_version: u16,
    session_id: &'a SessionId,
    epoch: Epoch,
    body: RawRenewTicketMessageRef<'a>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawFrameEnvelope {
    protocol_version: u16,
    session_id: SessionId,
    epoch: Epoch,
    body: RawNonSensitiveMessage,
}

#[derive(Deserialize)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum RawNonSensitiveMessage {
    Welcome(Welcome),
    Submit(Submit),
    Commit(Commit),
    Reject(Reject),
    CatchUp(CatchUp),
    Snapshot(Box<Snapshot>),
    Applied(Applied),
    UndoRequest(UndoRequest),
    UndoResult(UndoResult),
    PresenceUpdate(Presence),
    PresenceChanged(ParticipantPresence),
    ParticipantJoined(Participant),
    ParticipantLeft(ParticipantLeft),
    Bye(Bye),
}

impl From<RawNonSensitiveMessage> for CollabMessage {
    fn from(message: RawNonSensitiveMessage) -> Self {
        match message {
            RawNonSensitiveMessage::Welcome(value) => Self::Welcome(value),
            RawNonSensitiveMessage::Submit(value) => Self::Submit(value),
            RawNonSensitiveMessage::Commit(value) => Self::Commit(value),
            RawNonSensitiveMessage::Reject(value) => Self::Reject(value),
            RawNonSensitiveMessage::CatchUp(value) => Self::CatchUp(value),
            RawNonSensitiveMessage::Snapshot(value) => Self::Snapshot(value),
            RawNonSensitiveMessage::Applied(value) => Self::Applied(value),
            RawNonSensitiveMessage::UndoRequest(value) => Self::UndoRequest(value),
            RawNonSensitiveMessage::UndoResult(value) => Self::UndoResult(value),
            RawNonSensitiveMessage::PresenceUpdate(value) => Self::PresenceUpdate(value),
            RawNonSensitiveMessage::PresenceChanged(value) => Self::PresenceChanged(value),
            RawNonSensitiveMessage::ParticipantJoined(value) => Self::ParticipantJoined(value),
            RawNonSensitiveMessage::ParticipantLeft(value) => Self::ParticipantLeft(value),
            RawNonSensitiveMessage::Bye(value) => Self::Bye(value),
        }
    }
}

/// Uniquely owned credential-bearing JSON.
///
/// The allocation zeroizes on drop and its debug representation exposes only
/// length metadata.
pub struct SensitiveFrameJson {
    bytes: Zeroizing<Vec<u8>>,
}

impl SensitiveFrameJson {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl fmt::Debug for SensitiveFrameJson {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SensitiveFrameJson")
            .field("encoded_len", &self.bytes.len())
            .finish()
    }
}

impl FrameEnvelope {
    pub fn new(session_id: SessionId, epoch: Epoch, body: CollabMessage) -> Self {
        Self {
            protocol_version: COLLAB_PROTOCOL_VERSION,
            session_id,
            epoch,
            body,
        }
    }

    pub const fn protocol_version(&self) -> u16 {
        self.protocol_version
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn body(&self) -> &CollabMessage {
        &self.body
    }

    pub fn into_body(self) -> CollabMessage {
        self.body
    }

    /// Validates a frame and returns its encoded size without exposing
    /// credential-bearing bytes through a raw buffer API.
    pub fn encoded_len_with_limits(&self, limits: WireLimits) -> Result<usize, ProtocolError> {
        match &self.body {
            CollabMessage::RenewTicket(renewal) => encode_renew_ticket_frame_to_zeroizing_json(
                &self.session_id,
                self.epoch,
                &renewal.opaque_ticket,
                limits,
            )
            .map(|encoded| encoded.as_bytes().len()),
            _ => self
                .to_json_vec_with_limits(limits)
                .map(|encoded| encoded.len()),
        }
    }

    pub fn to_json_vec(&self) -> Result<Vec<u8>, ProtocolError> {
        self.to_json_vec_with_limits(WireLimits::default())
    }

    pub fn to_json_vec_with_limits(&self, limits: WireLimits) -> Result<Vec<u8>, ProtocolError> {
        if matches!(&self.body, CollabMessage::RenewTicket(_)) {
            return Err(ProtocolError::SensitiveCredentialRequiresDedicatedCodec);
        }
        validate_wire_limits(limits)?;
        validate_envelope(self, limits)?;
        validate_txn_limits(&self.body, limits)?;
        let raw = RawFrameEnvelopeRef {
            protocol_version: self.protocol_version,
            session_id: &self.session_id,
            epoch: self.epoch,
            body: &self.body,
        };
        validate_finite(&raw).map_err(|_| ProtocolError::NonFiniteNumber)?;
        if typed_images::message_has_external_image_ref(&self.body) {
            return Err(ProtocolError::ExternalImageReference);
        }
        let mut isolated = to_isolated_value(&raw).map_err(ProtocolError::Encode)?;
        inline_frame(&mut isolated.value, &isolated.images);
        if frame_has_external_image_ref(&mut isolated.value) {
            return Err(ProtocolError::ExternalImageReference);
        }
        let encoded = serde_json::to_vec(&isolated.value).map_err(ProtocolError::Encode)?;
        enforce_envelope_limit(encoded.len(), limits)?;
        enforce_json_nesting_limit(&encoded, json_nesting_limit(limits))?;
        Ok(encoded)
    }

    pub fn from_json_slice(bytes: &[u8]) -> Result<Self, ProtocolError> {
        Self::from_json_slice_with_limits(bytes, WireLimits::default())
    }

    pub fn from_json_slice_with_limits(
        bytes: &[u8],
        limits: WireLimits,
    ) -> Result<Self, ProtocolError> {
        validate_wire_limits(limits)?;
        enforce_envelope_limit(bytes.len(), limits)?;
        enforce_json_nesting_limit(bytes, json_nesting_limit(limits))?;
        reject_renew_ticket_before_generic_value_decode(bytes)?;
        let mut value = decode_json_value(bytes, limits)?;
        if frame_has_external_image_ref(&mut value) {
            return Err(ProtocolError::ExternalImageReference);
        }
        let raw: RawFrameEnvelope = serde_json::from_value(value).map_err(ProtocolError::Decode)?;
        if raw.protocol_version != COLLAB_PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion {
                actual: raw.protocol_version,
                expected: COLLAB_PROTOCOL_VERSION,
            });
        }
        let envelope = Self {
            protocol_version: raw.protocol_version,
            session_id: raw.session_id,
            epoch: raw.epoch,
            body: raw.body.into(),
        };
        validate_envelope(&envelope, limits)?;
        validate_txn_limits(&envelope.body, limits)?;
        Ok(envelope)
    }
}

/// Encodes a renewal frame directly into uniquely owned zeroizing storage.
///
/// The credential is serialized through `serde_json::Serializer`; this path
/// never constructs a `serde_json::Value`, JSON `String`, or ordinary output
/// `Vec<u8>`.
pub fn encode_renew_ticket_frame_to_zeroizing_json(
    session_id: &SessionId,
    epoch: Epoch,
    opaque_ticket: &OpaqueTicket,
    limits: WireLimits,
) -> Result<SensitiveFrameJson, ProtocolError> {
    validate_wire_limits(limits)?;
    require_identifier("session_id", session_id.as_ref(), limits)?;
    let raw = RawRenewTicketFrameEnvelopeRef {
        protocol_version: COLLAB_PROTOCOL_VERSION,
        session_id,
        epoch,
        body: RawRenewTicketMessageRef::RenewTicket(RawRenewTicketRef {
            opaque_ticket: DedicatedOpaqueTicketRef(opaque_ticket),
        }),
    };
    let mut encoded = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *encoded, &raw).map_err(ProtocolError::Encode)?;
    enforce_envelope_limit(encoded.len(), limits)?;
    enforce_json_nesting_limit(&encoded, json_nesting_limit(limits))?;
    Ok(SensitiveFrameJson { bytes: encoded })
}

/// Encodes a Commit envelope without cloning its transaction-heavy payload.
///
/// The produced bytes are byte-identical to encoding an owned
/// [`FrameEnvelope`] containing `CollabMessage::Commit`.
pub fn encode_commit_frame_to_json_vec(
    session_id: &SessionId,
    epoch: Epoch,
    commit: &Commit,
    limits: WireLimits,
) -> Result<Vec<u8>, ProtocolError> {
    validate_wire_limits(limits)?;
    require_identifier("session_id", session_id.as_ref(), limits)?;
    validate_commit(commit, limits)?;
    validate_txn(&commit.txn, limits)?;
    let raw = RawCommitFrameEnvelopeRef {
        protocol_version: COLLAB_PROTOCOL_VERSION,
        session_id,
        epoch,
        body: RawCommitMessageRef::Commit(commit),
    };
    validate_finite(&raw).map_err(|_| ProtocolError::NonFiniteNumber)?;
    if typed_images::txn_has_external_image_ref(&commit.txn) {
        return Err(ProtocolError::ExternalImageReference);
    }
    let mut isolated = to_isolated_value(&raw).map_err(ProtocolError::Encode)?;
    inline_frame(&mut isolated.value, &isolated.images);
    if frame_has_external_image_ref(&mut isolated.value) {
        return Err(ProtocolError::ExternalImageReference);
    }
    let encoded = serde_json::to_vec(&isolated.value).map_err(ProtocolError::Encode)?;
    enforce_envelope_limit(encoded.len(), limits)?;
    enforce_json_nesting_limit(&encoded, json_nesting_limit(limits))?;
    Ok(encoded)
}

fn decode_json_value(bytes: &[u8], limits: WireLimits) -> Result<serde_json::Value, ProtocolError> {
    enforce_json_nesting_limit(bytes, json_nesting_limit(limits))?;
    serde_json::from_slice(bytes).map_err(ProtocolError::Decode)
}

pub(crate) fn json_nesting_limit(limits: WireLimits) -> usize {
    // A PenNode level normally adds an object and a children array. Fixed
    // envelope/schema overhead leaves room for bounded nested node metadata,
    // and this remains below serde_json's recursion ceiling.
    usize_limit(limits.max_tree_depth)
        .saturating_mul(2)
        .saturating_add(16)
}

pub(crate) fn enforce_json_nesting_limit(bytes: &[u8], limit: usize) -> Result<(), ProtocolError> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > limit {
                    return Err(ProtocolError::JsonNestingTooDeep {
                        actual: depth,
                        limit,
                    });
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

impl CollabMessage {
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Welcome(_) => "welcome",
            Self::Submit(_) => "submit",
            Self::Commit(_) => "commit",
            Self::Reject(_) => "reject",
            Self::CatchUp(_) => "catch_up",
            Self::Snapshot(_) => "snapshot",
            Self::Applied(_) => "applied",
            Self::RenewTicket(_) => "renew_ticket",
            Self::UndoRequest(_) => "undo_request",
            Self::UndoResult(_) => "undo_result",
            Self::PresenceUpdate(_) => "presence_update",
            Self::PresenceChanged(_) => "presence_changed",
            Self::ParticipantJoined(_) => "participant_joined",
            Self::ParticipantLeft(_) => "participant_left",
            Self::Bye(_) => "bye",
        }
    }

    fn txn(&self) -> Option<&CollabTxn> {
        match self {
            Self::Submit(message) => Some(&message.txn),
            Self::Commit(message) => Some(&message.txn),
            _ => None,
        }
    }
}

fn validate_txn_limits(message: &CollabMessage, limits: WireLimits) -> Result<(), ProtocolError> {
    let Some(txn) = message.txn() else {
        return Ok(());
    };
    validate_txn(txn, limits)
}

fn validate_txn(txn: &CollabTxn, limits: WireLimits) -> Result<(), ProtocolError> {
    let max_ops = usize_limit(limits.max_ops_per_txn);
    if txn.ops.len() > max_ops {
        return Err(ProtocolError::TooManyOperations {
            actual: txn.ops.len(),
            limit: max_ops,
        });
    }
    validate_txn_resource_shape(txn, &ApplyLimits::from(limits))
        .map_err(ProtocolError::InvalidTransaction)?;
    validate_finite(txn).map_err(|_| ProtocolError::NonFiniteNumber)?;
    if typed_images::txn_has_external_image_ref(txn) {
        return Err(ProtocolError::ExternalImageReference);
    }
    let mut isolated = to_isolated_value(txn).map_err(ProtocolError::Encode)?;
    inline_txn(&mut isolated.value, &isolated.images);
    let encoded = serde_json::to_vec(&isolated.value).map_err(ProtocolError::Encode)?;
    let max_txn_bytes = usize_limit(limits.max_txn_bytes);
    if encoded.len() > max_txn_bytes {
        return Err(ProtocolError::TransactionTooLarge {
            actual: encoded.len(),
            limit: max_txn_bytes,
        });
    }
    Ok(())
}

pub(crate) fn enforce_envelope_limit(
    actual: usize,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    let limit = usize_limit(limits.max_envelope_bytes);
    if actual > limit {
        return Err(ProtocolError::EnvelopeTooLarge { actual, limit });
    }
    Ok(())
}

fn usize_limit(limit: u32) -> usize {
    usize::try_from(limit).unwrap_or(usize::MAX)
}

pub(crate) fn validate_envelope(
    envelope: &FrameEnvelope,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    if envelope.protocol_version != COLLAB_PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion {
            actual: envelope.protocol_version,
            expected: COLLAB_PROTOCOL_VERSION,
        });
    }
    require_identifier("session_id", envelope.session_id.as_ref(), limits)?;
    validate_message(&envelope.body, limits)
}

fn validate_message(message: &CollabMessage, limits: WireLimits) -> Result<(), ProtocolError> {
    match message {
        CollabMessage::Welcome(message) => {
            if message.hash_version != CANONICAL_HASH_VERSION {
                return Err(ProtocolError::UnsupportedHashVersion {
                    actual: message.hash_version,
                    expected: CANONICAL_HASH_VERSION,
                });
            }
            validate_wire_limits(message.limits)?;
            validate_participant("welcome.participant_id", &message.participant_id, limits)?;
            validate_peer("welcome.peer_id", &message.peer_id, limits)?;
            PeerNamespace::try_from(message.peer_namespace.as_str())?;
            for participant in &message.participants {
                validate_roster_participant(participant, limits)?;
            }
            require_identifier(
                "welcome.document_schema_version",
                &message.document_schema_version,
                limits,
            )
        }
        CollabMessage::Submit(message) => {
            validate_client_op("submit.client_op_id", &message.client_op_id, limits)
        }
        CollabMessage::Commit(message) => validate_commit(message, limits),
        CollabMessage::Reject(message) => {
            validate_client_op("reject.client_op_id", &message.client_op_id, limits)
        }
        CollabMessage::UndoRequest(message) => {
            validate_undo_request_id("undo_request.request_id", &message.request_id, limits)?;
            validate_client_op(
                "undo_request.target_client_op_id",
                &message.target_client_op_id,
                limits,
            )
        }
        CollabMessage::UndoResult(message) => {
            validate_undo_request_id("undo_result.request_id", &message.request_id, limits)?;
            validate_client_op(
                "undo_result.target_client_op_id",
                &message.target_client_op_id,
                limits,
            )?;
            if let Some(compensation) = &message.compensation_client_op_id {
                validate_client_op(
                    "undo_result.compensation_client_op_id",
                    compensation,
                    limits,
                )?;
            }
            Ok(())
        }
        CollabMessage::PresenceUpdate(message) => validate_presence(message, limits),
        CollabMessage::PresenceChanged(message) => {
            validate_participant(
                "presence_changed.participant_id",
                &message.participant_id,
                limits,
            )?;
            validate_peer("presence_changed.peer_id", &message.peer_id, limits)?;
            validate_presence(&message.presence, limits)
        }
        CollabMessage::ParticipantJoined(message) => validate_roster_participant(message, limits),
        CollabMessage::ParticipantLeft(message) => {
            validate_participant(
                "participant_left.participant_id",
                &message.participant_id,
                limits,
            )?;
            validate_peer("participant_left.peer_id", &message.peer_id, limits)
        }
        CollabMessage::Snapshot(snapshot) => {
            verify_snapshot(snapshot, &ApplyLimits::from(limits)).map_err(ProtocolError::from)
        }
        CollabMessage::CatchUp(_)
        | CollabMessage::Applied(_)
        | CollabMessage::RenewTicket(_)
        | CollabMessage::Bye(_) => Ok(()),
    }
}

fn validate_commit(commit: &Commit, limits: WireLimits) -> Result<(), ProtocolError> {
    validate_client_op("commit.client_op_id", &commit.client_op_id, limits)?;
    validate_participant(
        "commit.author.participant_id",
        &commit.author.participant_id,
        limits,
    )?;
    validate_peer("commit.author.peer_id", &commit.author.peer_id, limits)
}

pub(crate) fn validate_wire_limits(limits: WireLimits) -> Result<(), ProtocolError> {
    for (field, actual, maximum) in [
        (
            "max_envelope_bytes",
            limits.max_envelope_bytes,
            MAX_ENVELOPE_BYTES,
        ),
        ("max_txn_bytes", limits.max_txn_bytes, MAX_TXN_BYTES),
        ("max_ops_per_txn", limits.max_ops_per_txn, MAX_OPS_PER_TXN),
        (
            "max_processed_subtree_nodes_per_op",
            limits.max_processed_subtree_nodes_per_op,
            MAX_PROCESSED_SUBTREE_NODES_PER_OP,
        ),
        (
            "max_document_nodes",
            limits.max_document_nodes,
            MAX_DOCUMENT_NODES,
        ),
        ("max_tree_depth", limits.max_tree_depth, MAX_TREE_DEPTH),
        (
            "max_identifier_bytes",
            limits.max_identifier_bytes,
            MAX_IDENTIFIER_BYTES,
        ),
        (
            "max_presence_bytes",
            limits.max_presence_bytes,
            MAX_PRESENCE_BYTES,
        ),
        (
            "max_validation_node_visits_per_txn",
            limits.max_validation_node_visits_per_txn,
            MAX_VALIDATION_NODE_VISITS_PER_TXN,
        ),
    ] {
        if actual == 0 || actual > maximum {
            return Err(ProtocolError::InvalidWireLimit {
                field,
                actual,
                maximum,
            });
        }
    }
    Ok(())
}

fn validate_client_op(
    field: &'static str,
    id: &ClientOpId,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    validate_peer(field, &id.peer_id, limits)
}

fn validate_undo_request_id(
    field: &'static str,
    id: &UndoRequestId,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    validate_peer(field, &id.peer_id, limits)
}

fn validate_participant(
    field: &'static str,
    id: &ParticipantId,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    require_identifier(field, id.as_ref(), limits)
}

fn validate_roster_participant(
    participant: &Participant,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    validate_participant(
        "participant.participant_id",
        &participant.participant_id,
        limits,
    )?;
    validate_peer("participant.peer_id", &participant.peer_id, limits)?;
    validate_profile(
        participant.display_name.as_deref(),
        participant.avatar_url.as_deref(),
        "participant.display_name",
        "participant.avatar_url",
    )
}

pub(crate) fn validate_profile(
    display_name: Option<&str>,
    avatar_url: Option<&str>,
    display_name_field: &'static str,
    avatar_url_field: &'static str,
) -> Result<(), ProtocolError> {
    if display_name.is_some_and(|value| !valid_profile_display_name(value)) {
        return Err(ProtocolError::InvalidParticipantProfile {
            field: display_name_field,
        });
    }
    if avatar_url.is_some_and(|value| !valid_profile_avatar_url(value)) {
        return Err(ProtocolError::InvalidParticipantProfile {
            field: avatar_url_field,
        });
    }
    Ok(())
}

fn validate_peer(
    field: &'static str,
    id: &PeerId,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    require_identifier(field, id.as_ref(), limits)
}

fn require_identifier(
    field: &'static str,
    value: &str,
    limits: WireLimits,
) -> Result<(), ProtocolError> {
    if value.is_empty() {
        return Err(ProtocolError::EmptyIdentifier { field });
    }
    let limit = usize_limit(limits.max_identifier_bytes);
    if value.len() > limit {
        return Err(ProtocolError::IdentifierTooLong {
            field,
            actual: value.len(),
            limit,
        });
    }
    Ok(())
}

fn validate_presence(presence: &Presence, limits: WireLimits) -> Result<(), ProtocolError> {
    if let Some(cursor) = presence.cursor {
        if !cursor.x.is_finite() {
            return Err(ProtocolError::InvalidPresence { field: "cursor.x" });
        }
        if !cursor.y.is_finite() {
            return Err(ProtocolError::InvalidPresence { field: "cursor.y" });
        }
    }
    if let Some(viewport) = presence.viewport {
        for (field, value) in [
            ("viewport.pan_x", viewport.pan_x),
            ("viewport.pan_y", viewport.pan_y),
        ] {
            if !value.is_finite() {
                return Err(ProtocolError::InvalidPresence { field });
            }
        }
        if !viewport.zoom.is_finite() || viewport.zoom <= 0.0 {
            return Err(ProtocolError::InvalidPresence {
                field: "viewport.zoom",
            });
        }
    }
    for selected in &presence.selection {
        require_identifier("presence.selection", selected, limits)?;
    }
    if let Some(editing_node) = &presence.editing_node {
        require_identifier("presence.editing_node", editing_node, limits)?;
    }
    let encoded = serde_json::to_vec(presence).map_err(ProtocolError::Encode)?;
    let limit = usize_limit(limits.max_presence_bytes);
    if encoded.len() > limit {
        return Err(ProtocolError::PresenceTooLarge {
            actual: encoded.len(),
            limit,
        });
    }
    Ok(())
}

/// Compatibility alias for early callers; new code should use `FrameEnvelope`.
pub type WireEnvelope = FrameEnvelope;
