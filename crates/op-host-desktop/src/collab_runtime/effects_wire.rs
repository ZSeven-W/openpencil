use op_collab::{CollabMessage, Commit, ConnectionKey, FrameEnvelope, OpaqueTicket};
use op_collab_transport::{m1_wire_limits, EncodedFrameTransfer};

use super::actor::{GuestActor, OwnerActor};
use super::effects::command_send_error;
use super::types::{
    is_lossy_presence_frame, BudgetedFrame, CollabRuntimeError, CollabRuntimeFailure,
    GuestNetworkCommand, OwnerNetworkCommand,
};
use super::DesktopCollabRuntime;

impl DesktopCollabRuntime {
    pub(super) fn send_owner_actor_message(
        &self,
        owner: &OwnerActor,
        connection: ConnectionKey,
        message: CollabMessage,
        coalesce_key: Option<u64>,
    ) -> Result<(), CollabRuntimeError> {
        let frame = FrameEnvelope::new(
            owner.session.core().session_id().clone(),
            owner.session.core().epoch(),
            message,
        );
        self.send_owner_frame(connection, frame, coalesce_key)
    }

    pub(super) fn send_owner_actor_commit(
        &self,
        owner: &OwnerActor,
        connection: ConnectionKey,
        commit: &Commit,
    ) -> Result<(), CollabRuntimeError> {
        let encoded = EncodedFrameTransfer::encode_commit(
            owner.session.core().session_id(),
            owner.session.core().epoch(),
            commit,
            m1_wire_limits(),
        )
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
        self.send_encoded_owner_frame(connection, encoded, false, None)
    }

    pub(super) fn send_owner_renew_ticket(
        &self,
        owner: &OwnerActor,
        connection: ConnectionKey,
        ticket: &OpaqueTicket,
    ) -> Result<(), CollabRuntimeError> {
        let encoded = EncodedFrameTransfer::encode_renew_ticket(
            owner.session.core().session_id(),
            owner.session.core().epoch(),
            ticket,
            m1_wire_limits(),
        )
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
        self.send_encoded_owner_frame(connection, encoded, false, None)
    }

    pub(super) fn broadcast_owner_renew_ticket(
        &self,
        owner: &OwnerActor,
        ticket: &OpaqueTicket,
    ) -> Result<(), CollabRuntimeError> {
        for connection in &owner.connections {
            self.send_owner_renew_ticket(owner, *connection, ticket)?;
        }
        Ok(())
    }

    pub(super) fn broadcast_owner_message(
        &self,
        owner: &OwnerActor,
        message: CollabMessage,
        coalesce_key: Option<u64>,
    ) -> Result<(), CollabRuntimeError> {
        if owner.connections.is_empty() {
            return Ok(());
        }
        let frame = FrameEnvelope::new(
            owner.session.core().session_id().clone(),
            owner.session.core().epoch(),
            message,
        );
        let lossy = is_lossy_presence_frame(&frame);
        let encoded = EncodedFrameTransfer::encode(&frame, m1_wire_limits())
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
        if encoded.class() == op_collab_transport::TransferClass::Ticket {
            for connection in &owner.connections {
                let encoded = EncodedFrameTransfer::encode(&frame, m1_wire_limits())
                    .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
                self.send_encoded_owner_frame(*connection, encoded, false, coalesce_key)?;
            }
            return Ok(());
        }
        for connection in &owner.connections {
            let peer_encoded = encoded
                .try_clone_shared()
                .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
            let result =
                self.send_encoded_owner_frame(*connection, peer_encoded, lossy, coalesce_key);
            if result.is_err()
                && lossy
                && result
                    .as_ref()
                    .is_err_and(|error| error.failure == CollabRuntimeFailure::ResourceLimit)
            {
                continue;
            }
            result?;
        }
        Ok(())
    }

    pub(super) fn broadcast_owner_commit(
        &self,
        owner: &OwnerActor,
        commit: &Commit,
    ) -> Result<(), CollabRuntimeError> {
        if owner.connections.is_empty() {
            return Ok(());
        }
        let encoded = EncodedFrameTransfer::encode_commit(
            owner.session.core().session_id(),
            owner.session.core().epoch(),
            commit,
            m1_wire_limits(),
        )
        .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
        for connection in &owner.connections {
            self.send_encoded_owner_frame(
                *connection,
                encoded
                    .try_clone_shared()
                    .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?,
                false,
                None,
            )?;
        }
        Ok(())
    }

    pub(super) fn send_guest_message(
        &self,
        guest: &GuestActor,
        message: CollabMessage,
        coalesce_key: Option<u64>,
    ) -> Result<(), CollabRuntimeError> {
        let frame = FrameEnvelope::new(
            guest.session.core().session_id().clone(),
            guest.session.core().epoch(),
            message,
        );
        self.send_guest_frame(frame, coalesce_key)
    }

    pub(super) fn send_owner_frame(
        &self,
        connection: ConnectionKey,
        frame: FrameEnvelope,
        coalesce_key: Option<u64>,
    ) -> Result<(), CollabRuntimeError> {
        let lossy = is_lossy_presence_frame(&frame);
        let encoded = EncodedFrameTransfer::encode(&frame, m1_wire_limits())
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
        self.send_encoded_owner_frame(connection, encoded, lossy, coalesce_key)
    }

    fn send_encoded_owner_frame(
        &self,
        connection: ConnectionKey,
        encoded: EncodedFrameTransfer,
        lossy: bool,
        coalesce_key: Option<u64>,
    ) -> Result<(), CollabRuntimeError> {
        let Some(frame) = self.reserve_outbound_frame(encoded, lossy)? else {
            return Ok(());
        };
        let result = self.send_owner(OwnerNetworkCommand::Send {
            connection,
            frame,
            coalesce_key,
        });
        if lossy
            && result
                .as_ref()
                .is_err_and(|error| error.failure == CollabRuntimeFailure::ResourceLimit)
        {
            return Ok(());
        }
        result
    }

    fn send_guest_frame(
        &self,
        frame: FrameEnvelope,
        coalesce_key: Option<u64>,
    ) -> Result<(), CollabRuntimeError> {
        let lossy = is_lossy_presence_frame(&frame);
        let encoded = EncodedFrameTransfer::encode(&frame, m1_wire_limits())
            .map_err(|_| CollabRuntimeError::new(CollabRuntimeFailure::Protocol))?;
        let Some(frame) = self.reserve_outbound_frame(encoded, lossy)? else {
            return Ok(());
        };
        let network = self
            .network
            .as_ref()
            .ok_or_else(CollabRuntimeError::invalid_session)?;
        let result = network
            .send_guest(GuestNetworkCommand::Send {
                frame,
                coalesce_key,
            })
            .map_err(command_send_error);
        if lossy
            && result
                .as_ref()
                .is_err_and(|error| error.failure == CollabRuntimeFailure::ResourceLimit)
        {
            return Ok(());
        }
        result
    }

    fn reserve_outbound_frame(
        &self,
        encoded: EncodedFrameTransfer,
        lossy: bool,
    ) -> Result<Option<Box<BudgetedFrame>>, CollabRuntimeError> {
        match self.bridge_budget.reserve(encoded.encoded_len()) {
            Ok(reservation) => Ok(Some(Box::new(BudgetedFrame::new(
                encoded,
                lossy,
                reservation,
            )))),
            Err(_) if lossy => Ok(None),
            Err(_) => Err(CollabRuntimeError::resource_limit()),
        }
    }
}
