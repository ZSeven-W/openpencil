use std::time::Instant;

use op_collab::{FrameEnvelope, Role};

use super::ConnectionDriver;
use crate::queue::QueueItem;
use crate::{
    verify_initial_ticket, AdmissionError, AdmissionHello, AdmissionIdentity, AdmissionPhase,
    EncodedFrameTransfer, QueueError, RuntimeError, TicketVerifier,
};

impl ConnectionDriver {
    pub fn queue_admission(
        &mut self,
        hello: &AdmissionHello,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        self.connection.ensure_operational()?;
        self.connection.check_idle(now)?;
        self.connection.ensure_admission_deadline(now)?;
        if !matches!(
            self.connection.admission.phase(),
            AdmissionPhase::Encrypted | AdmissionPhase::IdentityVerified
        ) {
            return Err(AdmissionError::InvalidState.into());
        }
        if self.local_admission_queued {
            return Err(AdmissionError::InvalidState.into());
        }
        self.queue_item(QueueItem::sensitive_admission(hello.encode()?))?;
        self.local_admission_queued = true;
        Ok(())
    }

    pub fn verify_remote_admission(
        &mut self,
        hello: &AdmissionHello,
        verifier: &dyn TicketVerifier,
        expected_issuer: &str,
        expected_subject: &str,
        now_unix_ms: u64,
        now: Instant,
    ) -> Result<AdmissionIdentity, RuntimeError> {
        if !self.remote_admission_received
            || self.connection.admission.phase() != AdmissionPhase::Encrypted
        {
            return Err(AdmissionError::InvalidState.into());
        }
        let identity = verify_initial_ticket(
            verifier,
            hello.ticket(),
            self.connection.remote_static(),
            expected_issuer,
            expected_subject,
            now_unix_ms,
        )
        .inspect_err(|_| self.connection.failed = true)?;
        self.connection
            .install_verified(identity.clone(), now_unix_ms, now)?;
        Ok(identity)
    }

    pub fn authorize_remote(&mut self, role: Role) -> Result<(), RuntimeError> {
        self.connection.authorize_remote(role)
    }

    pub fn activate(&mut self, now: Instant) -> Result<(), RuntimeError> {
        if !self.local_admission_queued || !self.remote_admission_received {
            return Err(AdmissionError::InvalidState.into());
        }
        self.connection.activate(now)
    }

    pub fn renew_ticket(
        &mut self,
        verifier: &dyn TicketVerifier,
        opaque_ticket: &[u8],
        now_unix_ms: u64,
        now: Instant,
    ) -> Result<&AdmissionIdentity, RuntimeError> {
        self.connection
            .renew_ticket(verifier, opaque_ticket, now_unix_ms, now)?;
        self.ticket_renewal_notified = false;
        Ok(self
            .connection
            .admission_state()
            .identity()
            .expect("renewed admission has an identity"))
    }

    pub fn queue_frame(&mut self, frame: &FrameEnvelope, now: Instant) -> Result<(), RuntimeError> {
        let encoded = EncodedFrameTransfer::encode(frame, self.connection.config.wire_limits)?;
        self.queue_encoded_frame(encoded, now)
    }

    /// Coalesces a queued latest-value frame, intended for presence updates.
    pub fn queue_coalescing_frame(
        &mut self,
        key: u64,
        frame: &FrameEnvelope,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        let encoded = EncodedFrameTransfer::encode(frame, self.connection.config.wire_limits)?;
        self.queue_coalescing_encoded_frame(key, encoded, now)
    }

    /// Queues a frame that was already encoded by the validated transport
    /// encoder. Ticket bytes remain uniquely owned and zeroizing.
    pub fn queue_encoded_frame(
        &mut self,
        encoded: EncodedFrameTransfer,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        self.queue_preencoded_frame(encoded, None, now)
    }

    /// Coalesces a validated, already-encoded latest-value frame.
    pub fn queue_coalescing_encoded_frame(
        &mut self,
        key: u64,
        encoded: EncodedFrameTransfer,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        self.queue_preencoded_frame(encoded, Some(key), now)
    }

    fn queue_preencoded_frame(
        &mut self,
        encoded: EncodedFrameTransfer,
        coalesce_key: Option<u64>,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        self.connection.ensure_active(now)?;
        let item = encoded.into_queue_item(self.connection.config.wire_limits, coalesce_key)?;
        self.queue_item(item)
    }

    fn queue_item(&mut self, item: QueueItem) -> Result<(), RuntimeError> {
        let limit = self.connection.config.connections;
        if !self.outbound_queue.replaces_queued_item(&item)
            && self.queued_items() >= limit.outbound_queue_items
        {
            return Err(QueueError::Full.into());
        }
        self.outbound_queue.push(item)?;
        Ok(())
    }
}
