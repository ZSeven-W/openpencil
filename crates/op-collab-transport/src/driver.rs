use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use op_collab::{FrameEnvelope, InboundFrameDirection};
use zeroize::Zeroizing;

use crate::driver_io::{
    consume_rate, rate_ready_at, IncrementalRecordDecoder, ProgressDeadline, SocketWaiter,
};
use crate::queue::{BoundedTransferQueue, ReservedQueueItem};
use crate::{
    decode_frame_transfer, encrypt_record, AdmissionError, AdmissionHello, AdmissionPhase,
    AdmissionState, ChunkHeader, RecordError, RuntimeError, SecureConnection, SharedQueueBudget,
    TransferClass, TransportTransfer, CHUNK_HEADER_BYTES, MAX_CHUNK_PAYLOAD,
    MAX_NOISE_CIPHERTEXT_BYTES, TRANSPORT_HEARTBEAT_PLAINTEXT,
};

#[path = "driver_queue.rs"]
mod driver_queue;

const MAX_IO_STEPS_PER_POLL: usize = 64;

#[derive(Debug)]
pub enum DriverEvent {
    Admission(AdmissionHello),
    Frame {
        frame: FrameEnvelope,
        encoded_len: usize,
    },
}
#[derive(Debug)]
pub struct DriverPoll {
    pub event: Option<DriverEvent>,
    pub made_progress: bool,
    pub has_pending_output: bool,
    pub waiting_for_rate: bool,
    pub rate_ready_at: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboundTransferPolicy {
    /// Authenticated guest-to-owner traffic. Snapshots are owner-originated,
    /// so reject their large declared allocation before reassembly begins.
    PeerToOwner,
    /// Authenticated owner-to-guest traffic, including snapshots.
    OwnerToGuest,
}

impl InboundTransferPolicy {
    const fn allows(self, class: TransferClass) -> bool {
        !matches!((self, class), (Self::PeerToOwner, TransferClass::Snapshot))
    }

    const fn frame_direction(self) -> InboundFrameDirection {
        match self {
            Self::PeerToOwner => InboundFrameDirection::GuestToOwner,
            Self::OwnerToGuest => InboundFrameDirection::OwnerToGuest,
        }
    }
}

struct OutboundTransfer {
    item: ReservedQueueItem,
    transfer_id: u64,
    chunk_count: u32,
    next_chunk: u32,
    pending_wire: Vec<u8>,
    written: usize,
    is_initial_admission: bool,
}

impl OutboundTransfer {
    fn new(
        item: ReservedQueueItem,
        transfer_id: u64,
        is_initial_admission: bool,
    ) -> Result<Self, RuntimeError> {
        let chunk_count = u32::try_from(item.encoded_len().div_ceil(MAX_CHUNK_PAYLOAD))
            .map_err(|_| crate::ChunkError::LengthOverflow)?;
        // Reuse the canonical outbound validator before retaining any state.
        crate::TransferChunkIter::new(item.class(), transfer_id, item.bytes())?;
        Ok(Self {
            item,
            transfer_id,
            chunk_count,
            next_chunk: 0,
            pending_wire: Vec::new(),
            written: 0,
            is_initial_admission,
        })
    }

    fn plaintext(&self) -> Zeroizing<Vec<u8>> {
        let bytes = self.item.bytes();
        let start = self.next_chunk as usize * MAX_CHUNK_PAYLOAD;
        let end = start.saturating_add(MAX_CHUNK_PAYLOAD).min(bytes.len());
        let header = ChunkHeader {
            class: self.item.class(),
            transfer_id: self.transfer_id,
            chunk_index: self.next_chunk,
            chunk_count: self.chunk_count,
            total_len: u32::try_from(bytes.len()).expect("validated transfer length"),
        };
        let mut plaintext = Vec::with_capacity(CHUNK_HEADER_BYTES + end - start);
        plaintext.extend_from_slice(&header.encode());
        plaintext.extend_from_slice(&bytes[start..end]);
        Zeroizing::new(plaintext)
    }

    fn complete(&self) -> bool {
        self.next_chunk == self.chunk_count && self.pending_wire.is_empty()
    }
}

#[derive(Clone, Copy)]
enum WriteStep {
    Idle,
    Progress,
    WouldBlock,
    WaitingForRate(Instant),
}

/// Single-owner nonblocking driver for both directions of a Noise connection.
///
/// Callers enqueue from their event-loop thread, wake that thread from GUI
/// channels, and invoke [`Self::poll`] on socket readiness or a rate-refill
/// timer. The driver never holds a lock across I/O.
pub struct ConnectionDriver {
    connection: SecureConnection<TcpStream>,
    inbound_policy: InboundTransferPolicy,
    io_waiter: SocketWaiter,
    decoder: IncrementalRecordDecoder,
    inbound_ciphertext: Option<Vec<u8>>,
    outbound_queue: BoundedTransferQueue,
    outbound: Option<OutboundTransfer>,
    local_admission_queued: bool,
    local_admission_sent: bool,
    remote_admission_received: bool,
    ticket_renewal_notified: bool,
    heartbeat_wire: Vec<u8>,
    heartbeat_written: usize,
    last_sent_at: Instant,
    last_received_at: Instant,
    read_progress: ProgressDeadline,
    write_progress: ProgressDeadline,
    read_rate_ready_at: Option<Instant>,
    write_rate_ready_at: Option<Instant>,
}

impl ConnectionDriver {
    pub fn new(
        connection: SecureConnection<TcpStream>,
        shared_budget: SharedQueueBudget,
        inbound_policy: InboundTransferPolicy,
    ) -> Result<Self, RuntimeError> {
        connection.stream.set_read_timeout(None)?;
        connection.stream.set_write_timeout(None)?;
        connection.stream.set_nonblocking(true)?;
        let already_admitted = connection.admission.phase() != AdmissionPhase::Encrypted;
        let outbound_queue = BoundedTransferQueue::with_shared_budget(
            connection.config.connections.outbound_queue_items,
            connection.config.connections.global_queued_bytes,
            shared_budget,
        )?;
        let io_waiter = SocketWaiter::new(&connection.stream)?;
        let initial_activity = connection.last_activity;
        Ok(Self {
            connection,
            inbound_policy,
            io_waiter,
            decoder: IncrementalRecordDecoder::new(),
            inbound_ciphertext: None,
            outbound_queue,
            outbound: None,
            local_admission_queued: already_admitted,
            local_admission_sent: already_admitted,
            remote_admission_received: already_admitted,
            ticket_renewal_notified: false,
            heartbeat_wire: Vec::new(),
            heartbeat_written: 0,
            last_sent_at: initial_activity,
            last_received_at: initial_activity,
            read_progress: ProgressDeadline::default(),
            write_progress: ProgressDeadline::default(),
            read_rate_ready_at: None,
            write_rate_ready_at: None,
        })
    }

    pub fn remote_static(&self) -> &[u8; 32] {
        self.connection.remote_static()
    }

    pub fn admission_state(&self) -> &AdmissionState {
        self.connection.admission_state()
    }

    pub fn queued_items(&self) -> usize {
        self.outbound_queue.len() + usize::from(self.outbound.is_some())
    }

    /// How many items the outbound queue holds before it refuses one.
    ///
    /// Callers that mix droppable and undroppable traffic on one connection
    /// need this to keep the droppable half from consuming the last slot: a
    /// refused presence update is a stale cursor, a refused commit is a dead
    /// connection.
    pub fn outbound_queue_capacity(&self) -> usize {
        self.connection.config.connections.outbound_queue_items
    }

    pub fn queued_bytes(&self) -> usize {
        self.outbound_queue.queued_bytes()
            + self
                .outbound
                .as_ref()
                .map_or(0, |outbound| outbound.item.encoded_len())
    }

    pub fn has_pending_output(&self) -> bool {
        !self.heartbeat_wire.is_empty()
            || self.outbound.is_some()
            || !self.outbound_queue.is_empty()
    }

    pub const fn ticket_renewal_at(&self) -> Option<Instant> {
        self.connection.ticket_renewal_at
    }

    pub const fn ticket_expiry_at(&self) -> Option<Instant> {
        self.connection.ticket_deadline
    }

    /// Returns `true` once when the peer ticket reaches its proactive renewal
    /// point. A successful renewal rearms the notification for the new ticket.
    pub fn ticket_renewal_due(&mut self, now: Instant) -> bool {
        if self.ticket_renewal_notified
            || self.connection.admission.phase() != AdmissionPhase::Active
            || self
                .connection
                .ticket_renewal_at
                .is_none_or(|renewal_at| now < renewal_at)
        {
            return false;
        }
        self.ticket_renewal_notified = true;
        true
    }

    /// Earliest monotonic deadline at which the event loop should poll again.
    pub fn next_deadline(&self) -> Option<Instant> {
        let lifecycle = match self.connection.admission.phase() {
            AdmissionPhase::Active if !self.ticket_renewal_notified => self
                .connection
                .ticket_renewal_at
                .or(self.connection.ticket_deadline),
            AdmissionPhase::Active => self.connection.ticket_deadline,
            _ => self
                .connection
                .connected_at
                .checked_add(self.connection.config.timeouts.admission),
        };
        let heartbeat = if self.has_pending_output() {
            None
        } else {
            self.last_sent_at
                .checked_add(self.connection.config.timeouts.heartbeat)
        };
        let idle = self
            .last_received_at
            .checked_add(self.connection.config.timeouts.idle);
        let read = self
            .decoder
            .has_partial_record()
            .then(|| {
                self.read_progress
                    .next_deadline(self.connection.config.timeouts.read_write)
            })
            .flatten();
        let write = self
            .has_partial_write()
            .then(|| {
                self.write_progress
                    .next_deadline(self.connection.config.timeouts.read_write)
            })
            .flatten();
        [
            lifecycle,
            heartbeat,
            idle,
            self.connection.reassembler.next_deadline(),
            read,
            write,
            self.read_rate_ready_at,
            self.write_rate_ready_at,
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Blocks until the socket can make useful progress or `timeout` elapses.
    ///
    /// Command channels are intentionally not owned by the transport, so hosts
    /// should cap `timeout` to their desired command-delivery latency.
    pub fn wait_for_io(&mut self, now: Instant, timeout: Duration) -> Result<(), RuntimeError> {
        self.connection.ensure_operational()?;
        let readable = self.inbound_ciphertext.is_none()
            && self
                .read_rate_ready_at
                .is_none_or(|deadline| now >= deadline);
        let writable = self.has_pending_output()
            && self
                .write_rate_ready_at
                .is_none_or(|deadline| now >= deadline);
        if let Err(error) =
            self.io_waiter
                .wait(&self.connection.stream, readable, writable, timeout)
        {
            self.connection.failed = true;
            return Err(error.into());
        }
        Ok(())
    }

    pub fn poll(&mut self, now: Instant) -> Result<DriverPoll, RuntimeError> {
        let result = self.poll_inner(now);
        if result.is_err() {
            self.connection.failed = true;
        }
        result
    }

    fn poll_inner(&mut self, now: Instant) -> Result<DriverPoll, RuntimeError> {
        self.connection.ensure_operational()?;
        self.check_progress_deadlines(now)?;
        self.connection.reassembler.check_timeout(now)?;
        if now.saturating_duration_since(self.last_received_at)
            >= self.connection.config.timeouts.idle
        {
            return Err(RuntimeError::IdleTimeout);
        }
        match self.connection.admission.phase() {
            AdmissionPhase::Active => self.connection.check_ticket_expiry(now)?,
            _ => self.connection.ensure_admission_deadline(now)?,
        }

        let mut made_progress = false;
        let mut rate_ready_at = None;
        self.read_rate_ready_at = None;
        self.write_rate_ready_at = None;
        for _ in 0..MAX_IO_STEPS_PER_POLL {
            let write = self.poll_write_once(now)?;
            made_progress |= matches!(write, WriteStep::Progress);
            if let WriteStep::WaitingForRate(deadline) = write {
                self.write_rate_ready_at = Some(deadline);
                rate_ready_at = earlier(rate_ready_at, deadline);
            }

            let (read_progress, event, read_rate) = self.poll_read_once(now)?;
            if let Some(deadline) = read_rate {
                self.read_rate_ready_at = Some(deadline);
                rate_ready_at = earlier(rate_ready_at, deadline);
            }
            made_progress |= read_progress;
            if event.is_some() {
                return Ok(DriverPoll {
                    event,
                    made_progress,
                    has_pending_output: self.has_pending_output(),
                    waiting_for_rate: rate_ready_at.is_some(),
                    rate_ready_at,
                });
            }
            if !matches!(write, WriteStep::Progress) && !read_progress {
                break;
            }
        }
        Ok(DriverPoll {
            event: None,
            made_progress,
            has_pending_output: self.has_pending_output(),
            waiting_for_rate: rate_ready_at.is_some(),
            rate_ready_at,
        })
    }

    fn poll_write_once(&mut self, now: Instant) -> Result<WriteStep, RuntimeError> {
        if !self.heartbeat_wire.is_empty() {
            return self.poll_heartbeat_write(now);
        }
        if self.outbound.is_none() {
            let Some(item) = self.outbound_queue.take_reserved() else {
                return self.maybe_start_heartbeat(now);
            };
            let transfer_id = self
                .connection
                .next_send_id
                .ok_or(RuntimeError::TransferIdExhausted)?;
            let is_initial_admission = self.local_admission_queued && !self.local_admission_sent;
            self.outbound = Some(OutboundTransfer::new(
                item,
                transfer_id,
                is_initial_admission,
            )?);
        }

        if self
            .outbound
            .as_ref()
            .is_some_and(|outbound| outbound.pending_wire.is_empty())
        {
            let plaintext = self
                .outbound
                .as_ref()
                .expect("initialized above")
                .plaintext();
            let plaintext_len =
                u64::try_from(plaintext.len()).map_err(|_| RuntimeError::RateLimited)?;
            let ready = rate_ready_at(
                &mut self.connection.outbound_records,
                &mut self.connection.outbound_bytes,
                plaintext_len,
                now,
            )?;
            if ready > now {
                return Ok(WriteStep::WaitingForRate(ready));
            }
            consume_rate(
                &mut self.connection.outbound_records,
                &mut self.connection.outbound_bytes,
                plaintext_len,
                now,
            )?;
            let ciphertext = encrypt_record(self.connection.noise.transport_mut(), &plaintext)?;
            let length =
                u16::try_from(ciphertext.len()).map_err(|_| RecordError::InvalidLength {
                    actual: ciphertext.len(),
                    maximum: MAX_NOISE_CIPHERTEXT_BYTES,
                })?;
            let outbound = self.outbound.as_mut().expect("initialized above");
            outbound.pending_wire.reserve(ciphertext.len() + 2);
            outbound
                .pending_wire
                .extend_from_slice(&length.to_be_bytes());
            outbound.pending_wire.extend_from_slice(&ciphertext);
            outbound.written = 0;
            self.write_progress.arm(now);
        }

        let write_result = {
            let outbound = self.outbound.as_ref().expect("initialized above");
            self.connection
                .stream
                .write(&outbound.pending_wire[outbound.written..])
        };
        match write_result {
            Ok(0) => Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "nonblocking TCP write returned zero",
            )
            .into()),
            Ok(written) => {
                let outbound = self.outbound.as_mut().expect("initialized above");
                outbound.written += written;
                self.write_progress.note_progress(now);
                self.connection.last_activity = now;
                self.last_sent_at = now;
                if outbound.written == outbound.pending_wire.len() {
                    outbound.pending_wire.clear();
                    outbound.written = 0;
                    outbound.next_chunk += 1;
                    self.write_progress.clear();
                }
                if outbound.complete() {
                    let completed = self.outbound.take().expect("checked complete above");
                    if completed.is_initial_admission {
                        self.local_admission_sent = true;
                    }
                    self.connection.next_send_id = completed.transfer_id.checked_add(1);
                    drop(completed);
                }
                Ok(WriteStep::Progress)
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                Ok(WriteStep::Progress)
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                Ok(WriteStep::WouldBlock)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn maybe_start_heartbeat(&mut self, now: Instant) -> Result<WriteStep, RuntimeError> {
        if self.connection.admission.phase() != AdmissionPhase::Active
            || now.saturating_duration_since(self.last_sent_at)
                < self.connection.config.timeouts.heartbeat
        {
            return Ok(WriteStep::Idle);
        }
        let plaintext_len = TRANSPORT_HEARTBEAT_PLAINTEXT.len() as u64;
        let ready = rate_ready_at(
            &mut self.connection.outbound_records,
            &mut self.connection.outbound_bytes,
            plaintext_len,
            now,
        )?;
        if ready > now {
            return Ok(WriteStep::WaitingForRate(ready));
        }
        consume_rate(
            &mut self.connection.outbound_records,
            &mut self.connection.outbound_bytes,
            plaintext_len,
            now,
        )?;
        let ciphertext = encrypt_record(
            self.connection.noise.transport_mut(),
            TRANSPORT_HEARTBEAT_PLAINTEXT,
        )?;
        let length = u16::try_from(ciphertext.len()).map_err(|_| RecordError::InvalidLength {
            actual: ciphertext.len(),
            maximum: MAX_NOISE_CIPHERTEXT_BYTES,
        })?;
        self.heartbeat_wire.reserve(ciphertext.len() + 2);
        self.heartbeat_wire.extend_from_slice(&length.to_be_bytes());
        self.heartbeat_wire.extend_from_slice(&ciphertext);
        self.heartbeat_written = 0;
        self.write_progress.arm(now);
        self.poll_heartbeat_write(now)
    }

    fn poll_heartbeat_write(&mut self, now: Instant) -> Result<WriteStep, RuntimeError> {
        match self
            .connection
            .stream
            .write(&self.heartbeat_wire[self.heartbeat_written..])
        {
            Ok(0) => Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "nonblocking TCP heartbeat write returned zero",
            )
            .into()),
            Ok(written) => {
                self.heartbeat_written += written;
                self.write_progress.note_progress(now);
                self.connection.last_activity = now;
                self.last_sent_at = now;
                if self.heartbeat_written == self.heartbeat_wire.len() {
                    self.heartbeat_wire.clear();
                    self.heartbeat_written = 0;
                    self.write_progress.clear();
                }
                Ok(WriteStep::Progress)
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {
                Ok(WriteStep::Progress)
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                Ok(WriteStep::WouldBlock)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn poll_read_once(
        &mut self,
        now: Instant,
    ) -> Result<(bool, Option<DriverEvent>, Option<Instant>), RuntimeError> {
        let mut progressed = false;
        if self.inbound_ciphertext.is_none() {
            let before = self.decoder.buffered_len();
            self.inbound_ciphertext = self.decoder.poll(&mut self.connection.stream)?;
            progressed = self.inbound_ciphertext.is_some() || self.decoder.buffered_len() != before;
            if self.decoder.has_partial_record() {
                if progressed {
                    self.read_progress.note_progress(now);
                }
            } else {
                self.read_progress.clear();
            }
        }
        let Some(ciphertext) = self.inbound_ciphertext.as_ref() else {
            return Ok((progressed, None, None));
        };
        let expected_plaintext = ciphertext
            .len()
            .saturating_sub(crate::config::NOISE_AEAD_TAG_BYTES)
            as u64;
        let ready = rate_ready_at(
            &mut self.connection.inbound_records,
            &mut self.connection.inbound_bytes,
            expected_plaintext,
            now,
        )?;
        if ready > now {
            return Ok((progressed, None, Some(ready)));
        }
        consume_rate(
            &mut self.connection.inbound_records,
            &mut self.connection.inbound_bytes,
            expected_plaintext,
            now,
        )?;
        let ciphertext = self.inbound_ciphertext.take().expect("checked above");
        let plaintext = crate::decrypt_record(self.connection.noise.transport_mut(), &ciphertext)?;
        if plaintext.as_slice() == TRANSPORT_HEARTBEAT_PLAINTEXT {
            if self.connection.admission.phase() != AdmissionPhase::Active {
                return Err(AdmissionError::InvalidState.into());
            }
            self.connection.last_activity = now;
            self.last_received_at = now;
            return Ok((true, None, None));
        }
        let encoded_header = plaintext
            .get(..CHUNK_HEADER_BYTES)
            .ok_or(crate::ChunkError::HeaderLength(plaintext.len()))?;
        let header = ChunkHeader::decode(encoded_header)?;
        self.connection.ensure_transfer_allowed(header.class, now)?;
        if !self.inbound_policy.allows(header.class) {
            return Err(RuntimeError::ForbiddenInboundClass(header.class));
        }
        self.connection.last_activity = now;
        self.last_received_at = now;
        let Some(completed) = self.connection.reassembler.push(now, &plaintext)? else {
            return Ok((true, None, None));
        };
        let transfer = TransportTransfer {
            class: completed.class,
            transfer_id: completed.transfer_id,
            bytes: completed.bytes,
            _reservation: completed._reservation,
        };
        let event = self.decode_event(transfer)?;
        Ok((true, Some(event), None))
    }

    fn has_partial_write(&self) -> bool {
        !self.heartbeat_wire.is_empty()
            || self
                .outbound
                .as_ref()
                .is_some_and(|outbound| !outbound.pending_wire.is_empty())
    }

    fn check_progress_deadlines(&self, now: Instant) -> Result<(), RuntimeError> {
        let timeout = self.connection.config.timeouts.read_write;
        if self.decoder.has_partial_record() && self.read_progress.expired(now, timeout) {
            return Err(RuntimeError::ReadTimeout);
        }
        if self.has_partial_write() && self.write_progress.expired(now, timeout) {
            return Err(RuntimeError::WriteTimeout);
        }
        Ok(())
    }

    fn decode_event(&mut self, transfer: TransportTransfer) -> Result<DriverEvent, RuntimeError> {
        match self.connection.admission.phase() {
            AdmissionPhase::Active => {
                let encoded_len = transfer.bytes.len();
                let frame = decode_frame_transfer(
                    transfer.class,
                    &transfer.bytes,
                    self.connection.config.wire_limits,
                    self.inbound_policy.frame_direction(),
                )?;
                Ok(DriverEvent::Frame { frame, encoded_len })
            }
            _ => {
                if self.remote_admission_received || transfer.class != TransferClass::Ticket {
                    return Err(AdmissionError::InvalidState.into());
                }
                let hello = AdmissionHello::decode(&transfer.bytes)?;
                self.remote_admission_received = true;
                Ok(DriverEvent::Admission(hello))
            }
        }
    }
}

impl Drop for ConnectionDriver {
    fn drop(&mut self) {
        self.io_waiter.delete(&self.connection.stream);
    }
}

fn earlier(current: Option<Instant>, candidate: Instant) -> Option<Instant> {
    Some(current.map_or(candidate, |existing| existing.min(candidate)))
}
