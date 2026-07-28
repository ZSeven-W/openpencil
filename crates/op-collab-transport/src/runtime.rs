use std::fmt;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use op_collab::{FrameEnvelope, Role};
use zeroize::Zeroizing;

use crate::record::read_ciphertext_record_until;
use crate::{
    decode_frame_transfer, encrypt_record, verify_initial_ticket, write_ciphertext_record,
    AdmissionError, AdmissionHello, AdmissionIdentity, AdmissionPhase, AdmissionState, ChunkHeader,
    EncodedFrameTransfer, NoiseSession, Reassembler, RecordError, RuntimeError, TicketVerifier,
    TokenBucket, TransferChunkIter, TransferClass, TransportConfig, CHUNK_HEADER_BYTES,
    TRANSPORT_HEARTBEAT_PLAINTEXT,
};

pub struct TransportTransfer {
    pub(crate) class: TransferClass,
    pub(crate) transfer_id: u64,
    pub(crate) bytes: Zeroizing<Vec<u8>>,
}

impl TransportTransfer {
    pub const fn class(&self) -> TransferClass {
        self.class
    }

    pub const fn transfer_id(&self) -> u64 {
        self.transfer_id
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn encoded_len(&self) -> usize {
        self.bytes.len()
    }
}

impl fmt::Debug for TransportTransfer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TransportTransfer")
            .field("class", &self.class)
            .field("transfer_id", &self.transfer_id)
            .field("encoded_len", &self.encoded_len())
            .finish()
    }
}

pub struct SecureConnection<S> {
    pub(crate) stream: S,
    pub(crate) noise: NoiseSession,
    pub(crate) config: TransportConfig,
    pub(crate) reassembler: Reassembler,
    pub(crate) next_send_id: Option<u64>,
    pub(crate) outbound_bytes: TokenBucket,
    pub(crate) outbound_records: TokenBucket,
    pub(crate) inbound_bytes: TokenBucket,
    pub(crate) inbound_records: TokenBucket,
    pub(crate) admission: AdmissionState,
    pub(crate) connected_at: Instant,
    pub(crate) last_activity: Instant,
    pub(crate) ticket_renewal_at: Option<Instant>,
    pub(crate) ticket_deadline: Option<Instant>,
    pub(crate) failed: bool,
}

impl<S: Read + Write> SecureConnection<S> {
    pub fn new(
        stream: S,
        noise: NoiseSession,
        config: TransportConfig,
        now: Instant,
    ) -> Result<Self, RuntimeError> {
        let config = config.validate()?;
        let outbound_bytes =
            TokenBucket::new(config.rate.bytes_per_second, config.rate.byte_burst, now)
                .map_err(|_| RuntimeError::RateLimited)?;
        let outbound_records = TokenBucket::new(
            config.rate.records_per_second,
            config.rate.record_burst,
            now,
        )
        .map_err(|_| RuntimeError::RateLimited)?;
        let inbound_bytes =
            TokenBucket::new(config.rate.bytes_per_second, config.rate.byte_burst, now)
                .map_err(|_| RuntimeError::RateLimited)?;
        let inbound_records = TokenBucket::new(
            config.rate.records_per_second,
            config.rate.record_burst,
            now,
        )
        .map_err(|_| RuntimeError::RateLimited)?;
        let remote_static = *noise.remote_static();
        Ok(Self {
            stream,
            noise,
            config,
            reassembler: Reassembler::new(config.timeouts),
            next_send_id: Some(1),
            outbound_bytes,
            outbound_records,
            inbound_bytes,
            inbound_records,
            admission: AdmissionState::Encrypted { remote_static },
            connected_at: now,
            last_activity: now,
            ticket_renewal_at: None,
            ticket_deadline: None,
            failed: false,
        })
    }

    pub fn remote_static(&self) -> &[u8; 32] {
        self.noise.remote_static()
    }

    pub fn config(&self) -> TransportConfig {
        self.config
    }

    pub fn admission_state(&self) -> &AdmissionState {
        &self.admission
    }

    pub const fn last_activity(&self) -> Instant {
        self.last_activity
    }

    pub fn check_idle(&self, now: Instant) -> Result<(), RuntimeError> {
        self.ensure_operational()?;
        if now.saturating_duration_since(self.last_activity) >= self.config.timeouts.idle {
            return Err(RuntimeError::IdleTimeout);
        }
        Ok(())
    }

    pub fn check_ticket_expiry(&self, now: Instant) -> Result<(), RuntimeError> {
        self.ensure_operational()?;
        if self.admission.phase() != AdmissionPhase::Active {
            return Err(AdmissionError::InvalidState.into());
        }
        match self.ticket_deadline {
            Some(deadline) if now < deadline => Ok(()),
            Some(_) => Err(AdmissionError::TicketExpired.into()),
            None => Err(AdmissionError::InvalidState.into()),
        }
    }

    pub fn authorize_remote(&mut self, role: Role) -> Result<(), RuntimeError> {
        self.ensure_operational()?;
        self.admission = self.admission.clone().authorize(role)?;
        Ok(())
    }

    pub fn activate(&mut self, now: Instant) -> Result<(), RuntimeError> {
        self.ensure_operational()?;
        self.ensure_ticket_deadline(now)?;
        self.admission = self.admission.clone().activate()?;
        Ok(())
    }

    pub fn renew_ticket(
        &mut self,
        verifier: &dyn TicketVerifier,
        opaque_ticket: &[u8],
        now_unix_ms: u64,
        now: Instant,
    ) -> Result<&AdmissionIdentity, RuntimeError> {
        self.check_ticket_expiry(now)?;
        let renewed = self
            .admission
            .clone()
            .renew(verifier, opaque_ticket, now_unix_ms)?;
        let identity = renewed.identity().ok_or(AdmissionError::InvalidState)?;
        let (renewal_at, deadline) = ticket_deadlines(identity, now_unix_ms, now)?;
        self.admission = renewed;
        self.ticket_renewal_at = Some(renewal_at);
        self.ticket_deadline = Some(deadline);
        Ok(self
            .admission
            .identity()
            .expect("renewed admission has an identity"))
    }

    pub fn send_transfer(
        &mut self,
        class: TransferClass,
        bytes: &[u8],
        now: Instant,
    ) -> Result<u64, RuntimeError> {
        self.ensure_transfer_allowed(class, now)?;
        let transfer_id = self.next_send_id.ok_or(RuntimeError::TransferIdExhausted)?;
        let chunks = TransferChunkIter::new(class, transfer_id, bytes)?;
        let record_count = u64::from(chunks.chunk_count());
        let wire_bytes = bytes
            .len()
            .checked_add(
                usize::try_from(record_count)
                    .unwrap_or(usize::MAX)
                    .saturating_mul(CHUNK_HEADER_BYTES),
            )
            .ok_or(RuntimeError::RateLimited)?;
        let wire_bytes = u64::try_from(wire_bytes).map_err(|_| RuntimeError::RateLimited)?;
        if self.outbound_records.available(now) < record_count
            || self.outbound_bytes.available(now) < wire_bytes
        {
            return Err(RuntimeError::RateLimited);
        }
        if !self.outbound_records.try_take(record_count, now)
            || !self.outbound_bytes.try_take(wire_bytes, now)
        {
            return Err(RuntimeError::RateLimited);
        }
        let send_result = (|| {
            for plaintext in chunks {
                let ciphertext = encrypt_record(self.noise.transport_mut(), &plaintext)?;
                write_ciphertext_record(&mut self.stream, &ciphertext)?;
            }
            self.stream.flush()?;
            Ok::<(), RuntimeError>(())
        })();
        if let Err(error) = send_result {
            self.failed = true;
            return Err(error);
        }
        self.next_send_id = transfer_id.checked_add(1);
        self.last_activity = now;
        Ok(transfer_id)
    }

    pub fn receive_transfer(&mut self) -> Result<TransportTransfer, RuntimeError> {
        self.ensure_operational()?;
        let result = self.receive_transfer_inner();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn receive_transfer_inner(&mut self) -> Result<TransportTransfer, RuntimeError> {
        self.check_idle(Instant::now())?;
        loop {
            let now = Instant::now();
            self.reassembler.check_timeout(now)?;
            let record_deadline = now
                .checked_add(self.config.timeouts.read_write)
                .ok_or(RuntimeError::ReadTimeout)?;
            let deadline = self
                .reassembler
                .next_deadline()
                .map_or(record_deadline, |transfer| transfer.min(record_deadline));
            let ciphertext = match read_ciphertext_record_until(&mut self.stream, deadline) {
                Ok(ciphertext) => ciphertext,
                Err(RecordError::Io(error))
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    self.reassembler.check_timeout(Instant::now())?;
                    return Err(RuntimeError::ReadTimeout);
                }
                Err(error) => return Err(error.into()),
            };
            let plaintext = crate::decrypt_record(self.noise.transport_mut(), &ciphertext)?;
            let now = Instant::now();
            self.reassembler.check_timeout(now)?;
            if !self.inbound_records.try_take(1, now)
                || !self.inbound_bytes.try_take(plaintext.len() as u64, now)
            {
                return Err(RuntimeError::RateLimited);
            }
            if plaintext.as_slice() == TRANSPORT_HEARTBEAT_PLAINTEXT {
                self.ensure_active(now)?;
                self.last_activity = now;
                continue;
            }
            let header = match plaintext.get(..CHUNK_HEADER_BYTES) {
                Some(encoded) => ChunkHeader::decode(encoded),
                None => Err(crate::ChunkError::HeaderLength(plaintext.len())),
            };
            let header = match header {
                Ok(header) => header,
                Err(error) => {
                    self.reassembler.reset();
                    return Err(error.into());
                }
            };
            if let Err(error) = self.ensure_transfer_allowed(header.class, now) {
                self.reassembler.reset();
                return Err(error);
            }
            self.last_activity = now;
            if let Some(completed) = self.reassembler.push(now, &plaintext)? {
                return Ok(TransportTransfer {
                    class: completed.class,
                    transfer_id: completed.transfer_id,
                    bytes: completed.bytes,
                });
            }
        }
    }

    pub fn send_frame(&mut self, frame: &FrameEnvelope, now: Instant) -> Result<u64, RuntimeError> {
        let encoded = EncodedFrameTransfer::encode(frame, self.config.wire_limits)?;
        self.send_encoded_frame(&encoded, now)
    }

    /// Sends an already validated frame without re-encoding its payload.
    pub fn send_encoded_frame(
        &mut self,
        encoded: &EncodedFrameTransfer,
        now: Instant,
    ) -> Result<u64, RuntimeError> {
        self.ensure_active(now)?;
        encoded.validate_for(self.config.wire_limits)?;
        self.send_transfer(encoded.class(), encoded.bytes(), now)
    }

    pub fn receive_frame(&mut self) -> Result<FrameEnvelope, RuntimeError> {
        self.ensure_active(Instant::now())?;
        let transfer = self.receive_transfer()?;
        self.ensure_active(Instant::now())?;
        decode_frame_transfer(transfer.class, &transfer.bytes, self.config.wire_limits)
            .map_err(RuntimeError::from)
    }

    pub fn send_admission(
        &mut self,
        hello: &AdmissionHello,
        now: Instant,
    ) -> Result<u64, RuntimeError> {
        self.ensure_initial_admission(now)?;
        let encoded = hello.encode()?;
        self.send_transfer(TransferClass::Ticket, &encoded, now)
    }

    pub fn receive_admission(&mut self) -> Result<AdmissionHello, RuntimeError> {
        self.ensure_initial_admission(Instant::now())?;
        let transfer = self.receive_transfer()?;
        if transfer.class != TransferClass::Ticket {
            return Err(RuntimeError::UnexpectedClass {
                expected: TransferClass::Ticket,
                actual: transfer.class,
            });
        }
        AdmissionHello::decode(&transfer.bytes).map_err(RuntimeError::from)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn exchange_admission_initiator(
        &mut self,
        local: &AdmissionHello,
        verifier: &dyn TicketVerifier,
        expected_issuer: &str,
        expected_subject: &str,
        now_unix_ms: u64,
        now: Instant,
    ) -> Result<(AdmissionHello, AdmissionIdentity), RuntimeError> {
        self.send_admission(local, now)?;
        let remote = self.receive_admission()?;
        let identity = verify_initial_ticket(
            verifier,
            remote.ticket(),
            self.remote_static(),
            expected_issuer,
            expected_subject,
            now_unix_ms,
        )
        .inspect_err(|_| {
            self.failed = true;
        })?;
        self.install_verified(identity.clone(), now_unix_ms, now)?;
        Ok((remote, identity))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn exchange_admission_responder(
        &mut self,
        local: &AdmissionHello,
        verifier: &dyn TicketVerifier,
        expected_issuer: &str,
        expected_subject: &str,
        now_unix_ms: u64,
        now: Instant,
    ) -> Result<(AdmissionHello, AdmissionIdentity), RuntimeError> {
        let remote = self.receive_admission()?;
        let identity = verify_initial_ticket(
            verifier,
            remote.ticket(),
            self.remote_static(),
            expected_issuer,
            expected_subject,
            now_unix_ms,
        )
        .inspect_err(|_| {
            self.failed = true;
        })?;
        self.send_admission(local, now)?;
        self.install_verified(identity.clone(), now_unix_ms, now)?;
        Ok((remote, identity))
    }

    pub(crate) fn install_verified(
        &mut self,
        identity: AdmissionIdentity,
        now_unix_ms: u64,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        if self.admission.phase() != AdmissionPhase::Encrypted {
            return Err(AdmissionError::InvalidState.into());
        }
        let (renewal_at, deadline) = ticket_deadlines(&identity, now_unix_ms, now)?;
        self.admission = AdmissionState::IdentityVerified(identity);
        self.ticket_renewal_at = Some(renewal_at);
        self.ticket_deadline = Some(deadline);
        Ok(())
    }

    pub(crate) fn ensure_operational(&self) -> Result<(), RuntimeError> {
        if self.failed {
            return Err(RuntimeError::ConnectionClosed);
        }
        Ok(())
    }

    pub(crate) fn ensure_admission_deadline(&self, now: Instant) -> Result<(), RuntimeError> {
        if now.saturating_duration_since(self.connected_at) >= self.config.timeouts.admission {
            return Err(AdmissionError::AdmissionTimedOut.into());
        }
        Ok(())
    }

    pub(crate) fn ensure_initial_admission(&self, now: Instant) -> Result<(), RuntimeError> {
        self.ensure_operational()?;
        self.check_idle(now)?;
        self.ensure_admission_deadline(now)?;
        if self.admission.phase() != AdmissionPhase::Encrypted {
            return Err(AdmissionError::InvalidState.into());
        }
        Ok(())
    }

    fn ensure_ticket_deadline(&self, now: Instant) -> Result<(), RuntimeError> {
        match self.ticket_deadline {
            Some(deadline) if now < deadline => Ok(()),
            Some(_) => Err(AdmissionError::TicketExpired.into()),
            None => Err(AdmissionError::InvalidState.into()),
        }
    }

    pub(crate) fn ensure_active(&self, now: Instant) -> Result<(), RuntimeError> {
        self.ensure_operational()?;
        self.check_idle(now)?;
        if self.admission.phase() != AdmissionPhase::Active {
            return Err(AdmissionError::InvalidState.into());
        }
        self.ensure_ticket_deadline(now)
    }

    pub(crate) fn ensure_transfer_allowed(
        &self,
        class: TransferClass,
        now: Instant,
    ) -> Result<(), RuntimeError> {
        self.ensure_operational()?;
        self.check_idle(now)?;
        match self.admission.phase() {
            AdmissionPhase::Active => self.ensure_ticket_deadline(now),
            _ if class == TransferClass::Ticket => self.ensure_admission_deadline(now),
            _ => Err(AdmissionError::InvalidState.into()),
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

fn ticket_deadlines(
    identity: &AdmissionIdentity,
    now_unix_ms: u64,
    now: Instant,
) -> Result<(Instant, Instant), AdmissionError> {
    let remaining_ms = identity
        .claims()
        .expires_at_unix_ms()
        .checked_sub(now_unix_ms)
        .filter(|remaining| *remaining > 0)
        .ok_or(AdmissionError::TicketExpired)?;
    let renewal_delay_ms = u64::try_from(u128::from(remaining_ms) * 4 / 5)
        .map_err(|_| AdmissionError::TicketExpired)?;
    let renewal_at = now
        .checked_add(Duration::from_millis(renewal_delay_ms))
        .ok_or(AdmissionError::TicketExpired)?;
    let deadline = now
        .checked_add(Duration::from_millis(remaining_ms))
        .ok_or(AdmissionError::TicketExpired)?;
    Ok((renewal_at, deadline))
}

#[cfg(test)]
mod tests {
    use super::*;
    use static_assertions::assert_not_impl_any;

    assert_not_impl_any!(TransportTransfer: Clone);

    #[test]
    fn ticket_transfer_debug_does_not_expose_bytes() {
        let transfer = TransportTransfer {
            class: TransferClass::Ticket,
            transfer_id: 9,
            bytes: Zeroizing::new(vec![211; 17]),
        };
        let debug = format!("{transfer:?}");
        assert!(debug.contains("class: Ticket"));
        assert!(debug.contains("transfer_id: 9"));
        assert!(debug.contains("encoded_len: 17"));
        assert!(!debug.contains("211, 211"));
    }

    #[test]
    fn ticket_deadlines_are_monotonic_and_use_eighty_percent_for_renewal() {
        let identity = verify_initial_ticket(
            &|_: &[u8], expected: &[u8; 32], _: u64| {
                crate::VerifiedTicketClaims::new(
                    "https://issuer.example".into(),
                    "00000000-0000-0000-0000-000000000001".into(),
                    "00000000-0000-0000-0000-000000000002".into(),
                    *expected,
                    1_250,
                )
            },
            b"ticket",
            &[7; 32],
            "https://issuer.example",
            "00000000-0000-0000-0000-000000000001",
            1_000,
        )
        .unwrap();
        let now = Instant::now();
        assert_eq!(
            ticket_deadlines(&identity, 1_000, now).unwrap(),
            (
                now + Duration::from_millis(200),
                now + Duration::from_millis(250)
            )
        );
        assert!(matches!(
            ticket_deadlines(&identity, 1_250, now),
            Err(AdmissionError::TicketExpired)
        ));
    }
}
