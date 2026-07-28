use std::net::SocketAddr;

use op_collab::{
    Bye, ByeReason, CollabMessage, ConnectionKey, Epoch, FrameEnvelope, OpaqueTicket, Role,
    SessionId, VerifiedAuthMetadata,
};
use op_collab_transport::{EncodedFrameTransfer, JoinIntent, SharedQueueReservation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CollabRuntimeFailure {
    AuthenticationUnavailable,
    TicketRejected,
    SecureKeyUnavailable,
    ClockUnavailable,
    InvalidAddress,
    InvalidSession,
    Transport,
    Protocol,
    ResourceLimit,
}

impl CollabRuntimeFailure {
    pub(crate) const fn is_authentication(self) -> bool {
        matches!(self, Self::AuthenticationUnavailable | Self::TicketRejected)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("desktop collaboration failed: {failure:?}")]
pub(crate) struct CollabRuntimeError {
    pub(crate) failure: CollabRuntimeFailure,
}

impl CollabRuntimeError {
    pub(crate) const fn new(failure: CollabRuntimeFailure) -> Self {
        Self { failure }
    }

    pub(crate) const fn invalid_session() -> Self {
        Self::new(CollabRuntimeFailure::InvalidSession)
    }

    pub(crate) const fn resource_limit() -> Self {
        Self::new(CollabRuntimeFailure::ResourceLimit)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CollabStatusEvent {
    OwnerStarted { endpoint: SocketAddr },
    JoinStarted { endpoint: SocketAddr },
    PeerAuthenticated { connection: ConnectionKey },
    SessionActive { role: Role },
    Reconnecting,
    PeerDisconnected { connection: ConnectionKey },
    SessionEnded,
    Failed(CollabRuntimeFailure),
}

#[derive(Debug)]
pub(super) enum NetworkEvent {
    OwnerReady {
        session_id: SessionId,
        epoch: Epoch,
        endpoint: SocketAddr,
        share_endpoint: Option<SocketAddr>,
        local_auth: VerifiedAuthMetadata,
    },
    PeerAuthenticated {
        connection: ConnectionKey,
        auth: VerifiedAuthMetadata,
        intent: JoinIntent,
    },
    GuestAuthenticated {
        connection: ConnectionKey,
        session_id: SessionId,
        epoch: Epoch,
        remote_static: [u8; 32],
    },
    Frame {
        connection: ConnectionKey,
        frame: FrameEnvelope,
    },
    RenewalVerified {
        connection: ConnectionKey,
        auth: VerifiedAuthMetadata,
    },
    LocalTicketReady {
        ticket: OpaqueTicket,
    },
    ConnectionClosed {
        connection: ConnectionKey,
        failure: Option<CollabRuntimeFailure>,
        remote_bye: Option<RemoteBye>,
    },
    Discovery {
        sessions: Vec<DiscoveredEndpoint>,
    },
    Failed(CollabRuntimeFailure),
    Stopped,
}

pub(super) enum TerminalNetworkEvent {
    ConnectionClosed {
        connection: ConnectionKey,
        failure: Option<CollabRuntimeFailure>,
        remote_bye: Option<RemoteBye>,
    },
    Failed(CollabRuntimeFailure),
    Stopped,
}

impl From<TerminalNetworkEvent> for NetworkEvent {
    fn from(event: TerminalNetworkEvent) -> Self {
        match event {
            TerminalNetworkEvent::ConnectionClosed {
                connection,
                failure,
                remote_bye,
            } => Self::ConnectionClosed {
                connection,
                failure,
                remote_bye,
            },
            TerminalNetworkEvent::Failed(failure) => Self::Failed(failure),
            TerminalNetworkEvent::Stopped => Self::Stopped,
        }
    }
}

#[derive(Debug)]
pub(super) struct RemoteBye {
    pub(super) session_id: SessionId,
    pub(super) epoch: Epoch,
    pub(super) reason: ByeReason,
}

impl RemoteBye {
    pub(super) fn into_frame(self) -> FrameEnvelope {
        FrameEnvelope::new(
            self.session_id,
            self.epoch,
            CollabMessage::Bye(Bye {
                reason: self.reason,
            }),
        )
    }
}

pub(super) struct TaggedNetworkEvent {
    pub(super) generation: u64,
    pub(super) event: NetworkEvent,
    pub(super) bridge_reservation: Option<SharedQueueReservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiscoveredEndpoint {
    pub(super) discovery_id: String,
    pub(super) addresses: Vec<SocketAddr>,
    pub(super) compatible: bool,
}

pub(super) enum OwnerNetworkCommand {
    Authorize {
        connection: ConnectionKey,
        role: Role,
    },
    Send {
        connection: ConnectionKey,
        frame: Box<BudgetedFrame>,
        coalesce_key: Option<u64>,
    },
    VerifyRenewal {
        connection: ConnectionKey,
        opaque_ticket: OpaqueTicket,
    },
    Close {
        connection: ConnectionKey,
    },
}

pub(super) enum GuestNetworkCommand {
    Send {
        frame: Box<BudgetedFrame>,
        coalesce_key: Option<u64>,
    },
    VerifyRenewal(OpaqueTicket),
}

pub(super) enum PeerNetworkCommand {
    Authorize(Role),
    Send {
        frame: Box<BudgetedFrame>,
        coalesce_key: Option<u64>,
    },
    VerifyRenewal(OpaqueTicket),
    Stop,
}

/// A validated encoded frame carrying its GUI↔network bridge reservation.
///
/// The reservation intentionally travels through every bounded handoff and is
/// released only after the socket driver accepts (or rejects) the frame.
pub(super) struct BudgetedFrame {
    encoded: EncodedFrameTransfer,
    lossy_presence: bool,
    reservation: SharedQueueReservation,
}

impl BudgetedFrame {
    pub(super) const fn new(
        encoded: EncodedFrameTransfer,
        lossy_presence: bool,
        reservation: SharedQueueReservation,
    ) -> Self {
        Self {
            encoded,
            lossy_presence,
            reservation,
        }
    }

    pub(super) fn into_parts(self) -> (EncodedFrameTransfer, SharedQueueReservation) {
        (self.encoded, self.reservation)
    }

    #[cfg(test)]
    pub(super) fn into_inner(self) -> FrameEnvelope {
        self.encoded
            .decode(op_collab_transport::m1_wire_limits())
            .expect("budgeted test frame remains valid")
    }

    #[cfg(test)]
    pub(super) fn decode_for_test(&self) -> FrameEnvelope {
        self.encoded
            .decode(op_collab_transport::m1_wire_limits())
            .expect("budgeted test frame remains valid")
    }

    pub(super) const fn is_lossy_presence(&self) -> bool {
        self.lossy_presence
    }

    #[cfg(test)]
    pub(super) fn shares_storage_with(&self, other: &Self) -> bool {
        self.encoded.shares_storage_with(&other.encoded)
    }
}

pub(super) fn is_lossy_presence_frame(frame: &FrameEnvelope) -> bool {
    matches!(
        frame.body(),
        CollabMessage::PresenceUpdate(_) | CollabMessage::PresenceChanged(_)
    )
}

#[cfg(test)]
mod ticket_ownership_tests {
    use super::*;
    use static_assertions::assert_not_impl_any;

    assert_not_impl_any!(OwnerNetworkCommand: Clone);
    assert_not_impl_any!(GuestNetworkCommand: Clone);
    assert_not_impl_any!(PeerNetworkCommand: Clone);
    assert_not_impl_any!(BudgetedFrame: Clone);

    fn ticket(value: &str) -> OpaqueTicket {
        OpaqueTicket::new(value.to_owned()).unwrap()
    }

    #[test]
    fn verification_commands_move_the_original_ticket_allocation() {
        let owner_ticket = ticket("owner-renewal");
        let owner_ptr = owner_ticket.expose().as_ptr();
        let owner = OwnerNetworkCommand::VerifyRenewal {
            connection: ConnectionKey::new(7).unwrap(),
            opaque_ticket: owner_ticket,
        };
        let OwnerNetworkCommand::VerifyRenewal { opaque_ticket, .. } = owner else {
            unreachable!();
        };
        assert_eq!(opaque_ticket.expose().as_ptr(), owner_ptr);

        let guest_ticket = ticket("guest-renewal");
        let guest_ptr = guest_ticket.expose().as_ptr();
        let GuestNetworkCommand::VerifyRenewal(guest_ticket) =
            GuestNetworkCommand::VerifyRenewal(guest_ticket)
        else {
            unreachable!();
        };
        assert_eq!(guest_ticket.expose().as_ptr(), guest_ptr);

        let peer_ticket = ticket("peer-renewal");
        let peer_ptr = peer_ticket.expose().as_ptr();
        let PeerNetworkCommand::VerifyRenewal(peer_ticket) =
            PeerNetworkCommand::VerifyRenewal(peer_ticket)
        else {
            unreachable!();
        };
        assert_eq!(peer_ticket.expose().as_ptr(), peer_ptr);
    }
}
