use std::fmt;
use std::sync::Arc;

use op_collab::{
    decode_renew_ticket_frame_from_json_slice, encode_commit_frame_to_json_vec,
    encode_renew_ticket_frame_to_zeroizing_json, CollabMessage, Commit, Epoch, FrameEnvelope,
    OpaqueTicket, SensitiveFrameJson, SessionId, WireLimits,
};

use crate::queue::QueueItem;
use crate::{FrameTransportError, TransferClass};

enum EncodedFrameBytes {
    Shared(Arc<[u8]>),
    Sensitive(SensitiveFrameJson),
}

/// A validated frame transfer ready for a connection driver's bounded queue.
///
/// Non-ticket bytes are reference counted so one encoded Commit can be queued
/// for multiple peers without duplicating its transaction payload. Ticket
/// bytes remain uniquely owned and zeroize on every drop path.
pub struct EncodedFrameTransfer {
    class: TransferClass,
    bytes: EncodedFrameBytes,
    wire_limits: WireLimits,
}

impl fmt::Debug for EncodedFrameTransfer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncodedFrameTransfer")
            .field("class", &self.class)
            .field("encoded_len", &self.encoded_len())
            .finish()
    }
}

impl EncodedFrameTransfer {
    pub fn encode(frame: &FrameEnvelope, limits: WireLimits) -> Result<Self, FrameTransportError> {
        if let CollabMessage::RenewTicket(renewal) = frame.body() {
            return Self::encode_renew_ticket(
                frame.session_id(),
                frame.epoch(),
                &renewal.opaque_ticket,
                limits,
            );
        }
        let (class, bytes) = encode_frame_transfer(frame, limits)?;
        Ok(Self::from_validated_shared(class, bytes, limits))
    }

    /// Encodes a borrowed credential without cloning it into a message tree.
    pub fn encode_renew_ticket(
        session_id: &SessionId,
        epoch: Epoch,
        opaque_ticket: &OpaqueTicket,
        limits: WireLimits,
    ) -> Result<Self, FrameTransportError> {
        let bytes =
            encode_renew_ticket_frame_to_zeroizing_json(session_id, epoch, opaque_ticket, limits)?;
        enforce_class_limit(TransferClass::Ticket, bytes.as_bytes().len())?;
        Ok(Self {
            class: TransferClass::Ticket,
            bytes: EncodedFrameBytes::Sensitive(bytes),
            wire_limits: limits,
        })
    }

    /// Encodes an authoritative Commit without cloning its transaction.
    pub fn encode_commit(
        session_id: &SessionId,
        epoch: Epoch,
        commit: &Commit,
        limits: WireLimits,
    ) -> Result<Self, FrameTransportError> {
        let bytes = encode_commit_frame_to_json_vec(session_id, epoch, commit, limits)?;
        enforce_class_limit(TransferClass::Txn, bytes.len())?;
        Ok(Self::from_validated_shared(
            TransferClass::Txn,
            bytes,
            limits,
        ))
    }

    pub const fn class(&self) -> TransferClass {
        self.class
    }

    pub fn encoded_len(&self) -> usize {
        self.bytes().len()
    }

    /// Clones only non-sensitive shared storage.
    pub fn try_clone_shared(&self) -> Result<Self, FrameTransportError> {
        let EncodedFrameBytes::Shared(bytes) = &self.bytes else {
            return Err(FrameTransportError::SensitiveTransferNotShareable);
        };
        Ok(Self {
            class: self.class,
            bytes: EncodedFrameBytes::Shared(bytes.clone()),
            wire_limits: self.wire_limits,
        })
    }

    /// Reports whether two transfers reference the same encoded allocation.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        match (&self.bytes, &other.bytes) {
            (EncodedFrameBytes::Shared(left), EncodedFrameBytes::Shared(right)) => {
                Arc::ptr_eq(left, right)
            }
            _ => false,
        }
    }

    /// Decodes this validated transfer under the same wire limits.
    pub fn decode(&self, limits: WireLimits) -> Result<FrameEnvelope, FrameTransportError> {
        self.validate_for(limits)?;
        decode_frame_transfer(self.class, self.bytes(), limits)
    }

    pub(crate) fn validate_for(
        &self,
        expected_limits: WireLimits,
    ) -> Result<(), FrameTransportError> {
        if self.wire_limits != expected_limits {
            return Err(FrameTransportError::WireLimitsMismatch);
        }
        enforce_class_limit(self.class, self.encoded_len())
    }

    pub(crate) fn into_queue_item(
        self,
        expected_limits: WireLimits,
        coalesce_key: Option<u64>,
    ) -> Result<QueueItem, FrameTransportError> {
        self.validate_for(expected_limits)?;
        match self.bytes {
            EncodedFrameBytes::Shared(bytes) => match coalesce_key {
                Some(key) => QueueItem::coalescing(self.class, key, bytes),
                None => QueueItem::reliable(self.class, bytes),
            },
            EncodedFrameBytes::Sensitive(bytes) => {
                if self.class != TransferClass::Ticket || coalesce_key.is_some() {
                    return Err(FrameTransportError::SensitiveTransferNotShareable);
                }
                Ok(QueueItem::sensitive_ticket_frame(bytes))
            }
        }
    }

    fn from_validated_shared(
        class: TransferClass,
        bytes: Vec<u8>,
        wire_limits: WireLimits,
    ) -> Self {
        debug_assert_ne!(class, TransferClass::Ticket);
        Self {
            class,
            bytes: EncodedFrameBytes::Shared(Arc::from(bytes)),
            wire_limits,
        }
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        match &self.bytes {
            EncodedFrameBytes::Shared(bytes) => bytes,
            EncodedFrameBytes::Sensitive(bytes) => bytes.as_bytes(),
        }
    }
}

pub fn frame_transfer_class(frame: &FrameEnvelope) -> TransferClass {
    match frame.body() {
        CollabMessage::Submit(_) | CollabMessage::Commit(_) => TransferClass::Txn,
        CollabMessage::Snapshot(_) => TransferClass::Snapshot,
        CollabMessage::RenewTicket(_) => TransferClass::Ticket,
        _ => TransferClass::Control,
    }
}

pub fn encode_frame_transfer(
    frame: &FrameEnvelope,
    limits: WireLimits,
) -> Result<(TransferClass, Vec<u8>), FrameTransportError> {
    let class = frame_transfer_class(frame);
    if class == TransferClass::Ticket {
        return Err(op_collab::ProtocolError::SensitiveCredentialRequiresDedicatedCodec.into());
    }
    let encoded = frame.to_json_vec_with_limits(limits)?;
    enforce_class_limit(class, encoded.len())?;
    Ok((class, encoded))
}

pub fn decode_frame_transfer(
    declared_class: TransferClass,
    encoded: &[u8],
    limits: WireLimits,
) -> Result<FrameEnvelope, FrameTransportError> {
    enforce_class_limit(declared_class, encoded.len())?;
    let frame = if declared_class == TransferClass::Ticket {
        decode_renew_ticket_frame_from_json_slice(encoded, limits)?
    } else {
        FrameEnvelope::from_json_slice_with_limits(encoded, limits)?
    };
    let actual = frame_transfer_class(&frame);
    if actual != declared_class {
        return Err(FrameTransportError::ClassMismatch {
            declared: declared_class,
            actual,
        });
    }
    Ok(frame)
}

fn enforce_class_limit(class: TransferClass, actual: usize) -> Result<(), FrameTransportError> {
    let maximum = class.max_transfer_bytes();
    if actual == 0 || actual > maximum {
        return Err(FrameTransportError::TransferTooLarge {
            class,
            actual,
            maximum,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use op_collab::{
        Bye, ByeReason, CanonicalHash, ClientOpId, CollabMessage, CollabOp, CollabTxn, Commit,
        CommitAuthor, CommitSeq, Epoch, FrameEnvelope, OpaqueTicket, PageRef, ParticipantId,
        PeerId, RenewTicket, SessionId,
    };

    use super::*;
    use crate::m1_wire_limits;
    use static_assertions::assert_not_impl_any;

    assert_not_impl_any!(EncodedFrameTransfer: Clone);

    fn envelope(message: CollabMessage) -> FrameEnvelope {
        FrameEnvelope::new(SessionId::from("session"), Epoch(1), message)
    }

    #[test]
    fn final_encoded_frame_is_checked_against_its_transport_class() {
        let control = envelope(CollabMessage::Bye(Bye {
            reason: ByeReason::Normal,
        }));
        let (class, bytes) = encode_frame_transfer(&control, m1_wire_limits()).unwrap();
        assert_eq!(class, TransferClass::Control);
        assert_eq!(
            decode_frame_transfer(class, &bytes, m1_wire_limits())
                .unwrap()
                .body()
                .kind(),
            "bye"
        );

        let ticket = envelope(CollabMessage::RenewTicket(RenewTicket {
            opaque_ticket: OpaqueTicket::new("header.payload.signature".to_owned()).unwrap(),
        }));
        assert!(matches!(
            encode_frame_transfer(&ticket, m1_wire_limits()),
            Err(FrameTransportError::Protocol(
                op_collab::ProtocolError::SensitiveCredentialRequiresDedicatedCodec
            ))
        ));
    }

    #[test]
    fn authenticated_class_cannot_disagree_with_decoded_body() {
        let control = envelope(CollabMessage::Bye(Bye {
            reason: ByeReason::Normal,
        }));
        let bytes = control.to_json_vec_with_limits(m1_wire_limits()).unwrap();
        assert!(matches!(
            decode_frame_transfer(TransferClass::Txn, &bytes, m1_wire_limits()),
            Err(FrameTransportError::ClassMismatch { .. })
        ));
    }

    #[test]
    fn mislabeled_renewal_never_reaches_generic_payload_deserialization() {
        let malformed_ticket_payload = br#"{
            "protocolVersion":1,
            "sessionId":"session",
            "epoch":1,
            "body":{
                "type":"renew_ticket",
                "payload":{"opaqueTicket":{"mustNotBeDeserialized":"secret"}}
            }
        }"#;
        for declared_class in [
            TransferClass::Control,
            TransferClass::Txn,
            TransferClass::Snapshot,
        ] {
            assert!(matches!(
                decode_frame_transfer(declared_class, malformed_ticket_payload, m1_wire_limits()),
                Err(FrameTransportError::Protocol(
                    op_collab::ProtocolError::SensitiveCredentialRequiresDedicatedCodec
                ))
            ));
        }
    }

    #[test]
    fn borrowed_commit_encoding_is_wire_identical_and_shared_clones_reuse_storage() {
        let commit = Commit {
            client_op_id: ClientOpId {
                peer_id: PeerId::from("peer-a"),
                local_counter: 1,
            },
            seq: CommitSeq(2),
            author: CommitAuthor {
                participant_id: ParticipantId::from("participant-a"),
                peer_id: PeerId::from("peer-a"),
            },
            txn: CollabTxn::new(vec![CollabOp::DeleteExact {
                page: PageRef::DocumentRoot,
                node_id: "node-a".to_owned(),
                expected_hash: CanonicalHash::from_bytes([3; 32]),
            }]),
            doc_hash: CanonicalHash::from_bytes([7; 32]),
        };
        let session_id = SessionId::from("session");
        let epoch = Epoch(1);
        let owned = envelope(CollabMessage::Commit(commit.clone()));
        let (_, owned_bytes) = encode_frame_transfer(&owned, m1_wire_limits()).unwrap();
        let encoded =
            EncodedFrameTransfer::encode_commit(&session_id, epoch, &commit, m1_wire_limits())
                .unwrap();
        assert_eq!(encoded.bytes(), owned_bytes);
        let clone = encoded.try_clone_shared().unwrap();
        assert!(encoded.shares_storage_with(&clone));
    }

    #[test]
    fn ticket_encoding_stays_sensitive_and_cannot_be_shared() {
        const SECRET: &str = "header.payload.signature";
        let ticket = envelope(CollabMessage::RenewTicket(RenewTicket {
            opaque_ticket: OpaqueTicket::new(SECRET.to_owned()).unwrap(),
        }));
        let encoded = EncodedFrameTransfer::encode(&ticket, m1_wire_limits()).unwrap();
        assert_eq!(encoded.class(), TransferClass::Ticket);
        assert!(matches!(
            encoded.try_clone_shared(),
            Err(FrameTransportError::SensitiveTransferNotShareable)
        ));
        let debug = format!("{encoded:?}");
        assert!(debug.contains("class: Ticket"));
        assert!(debug.contains(&format!("encoded_len: {}", encoded.encoded_len())));
        assert!(!debug.contains(SECRET));
        assert!(!debug.contains(&format!("{:?}", SECRET.as_bytes())));
        let decoded = encoded.decode(m1_wire_limits()).unwrap();
        let CollabMessage::RenewTicket(decoded) = decoded.into_body() else {
            panic!("dedicated ticket codec must retain the message kind");
        };
        assert_eq!(decoded.opaque_ticket.expose(), SECRET);
    }
}
