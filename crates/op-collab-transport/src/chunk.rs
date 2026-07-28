use std::fmt;
use std::time::Instant;

use zeroize::Zeroizing;

use crate::{
    ChunkError, TimeoutConfig, MAX_CONTROL_TRANSFER_BYTES, MAX_NOISE_PLAINTEXT_BYTES,
    MAX_SNAPSHOT_TRANSFER_BYTES, MAX_TXN_TRANSFER_BYTES,
};

pub const CHUNK_HEADER_BYTES: usize = 24;
pub const MAX_CHUNK_PAYLOAD: usize = MAX_NOISE_PLAINTEXT_BYTES - CHUNK_HEADER_BYTES;
const CHUNK_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TransferClass {
    Control = 1,
    Ticket = 2,
    Txn = 3,
    Snapshot = 4,
}

impl TransferClass {
    pub const fn max_transfer_bytes(self) -> usize {
        match self {
            Self::Control | Self::Ticket => MAX_CONTROL_TRANSFER_BYTES,
            Self::Txn => MAX_TXN_TRANSFER_BYTES,
            Self::Snapshot => MAX_SNAPSHOT_TRANSFER_BYTES,
        }
    }

    pub const fn timeout(self, timeouts: TimeoutConfig) -> std::time::Duration {
        match self {
            Self::Snapshot => timeouts.snapshot_transfer,
            Self::Control | Self::Ticket | Self::Txn => timeouts.ordinary_transfer,
        }
    }
}

impl TryFrom<u8> for TransferClass {
    type Error = ChunkError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Control),
            2 => Ok(Self::Ticket),
            3 => Ok(Self::Txn),
            4 => Ok(Self::Snapshot),
            unknown => Err(ChunkError::UnknownClass(unknown)),
        }
    }
}

impl From<TransferClass> for u8 {
    fn from(value: TransferClass) -> Self {
        value as Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    pub class: TransferClass,
    pub transfer_id: u64,
    pub chunk_index: u32,
    pub chunk_count: u32,
    pub total_len: u32,
}

impl ChunkHeader {
    pub fn encode(self) -> [u8; CHUNK_HEADER_BYTES] {
        let mut encoded = [0_u8; CHUNK_HEADER_BYTES];
        encoded[0] = CHUNK_VERSION;
        encoded[1] = self.class.into();
        encoded[2..4].copy_from_slice(&0_u16.to_be_bytes());
        encoded[4..12].copy_from_slice(&self.transfer_id.to_be_bytes());
        encoded[12..16].copy_from_slice(&self.chunk_index.to_be_bytes());
        encoded[16..20].copy_from_slice(&self.chunk_count.to_be_bytes());
        encoded[20..24].copy_from_slice(&self.total_len.to_be_bytes());
        encoded
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, ChunkError> {
        if encoded.len() != CHUNK_HEADER_BYTES {
            return Err(ChunkError::HeaderLength(encoded.len()));
        }
        if encoded[0] != CHUNK_VERSION {
            return Err(ChunkError::UnsupportedVersion(encoded[0]));
        }
        if encoded[2] != 0 || encoded[3] != 0 {
            return Err(ChunkError::ReservedBits);
        }

        let class = TransferClass::try_from(encoded[1])?;
        let transfer_id = u64::from_be_bytes(encoded[4..12].try_into().expect("fixed slice"));
        if transfer_id == 0 {
            return Err(ChunkError::ZeroTransferId);
        }
        let chunk_index = u32::from_be_bytes(encoded[12..16].try_into().expect("fixed slice"));
        let chunk_count = u32::from_be_bytes(encoded[16..20].try_into().expect("fixed slice"));
        let total_len = u32::from_be_bytes(encoded[20..24].try_into().expect("fixed slice"));
        validate_transfer_shape(class, total_len as usize, chunk_count)?;

        Ok(Self {
            class,
            transfer_id,
            chunk_index,
            chunk_count,
            total_len,
        })
    }
}

#[derive(PartialEq, Eq)]
pub struct CompletedTransfer {
    pub(crate) class: TransferClass,
    pub(crate) transfer_id: u64,
    pub(crate) bytes: Zeroizing<Vec<u8>>,
}

impl CompletedTransfer {
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

impl fmt::Debug for CompletedTransfer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompletedTransfer")
            .field("class", &self.class)
            .field("transfer_id", &self.transfer_id)
            .field("encoded_len", &self.encoded_len())
            .finish()
    }
}

pub struct TransferChunkIter<'a> {
    class: TransferClass,
    transfer_id: u64,
    bytes: &'a [u8],
    chunk_count: u32,
    next_index: u32,
}

pub struct TransferChunk(Zeroizing<Vec<u8>>);

impl std::ops::Deref for TransferChunk {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for TransferChunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TransferChunk")
            .field("encoded_len", &self.0.len())
            .finish()
    }
}

impl fmt::Debug for TransferChunkIter<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TransferChunkIter")
            .field("class", &self.class)
            .field("transfer_id", &self.transfer_id)
            .field("encoded_len", &self.bytes.len())
            .field("chunk_count", &self.chunk_count)
            .field("next_index", &self.next_index)
            .finish()
    }
}

impl<'a> TransferChunkIter<'a> {
    pub fn new(
        class: TransferClass,
        transfer_id: u64,
        bytes: &'a [u8],
    ) -> Result<Self, ChunkError> {
        if transfer_id == 0 {
            return Err(ChunkError::ZeroTransferId);
        }
        let chunk_count = expected_chunk_count(bytes.len())?;
        validate_transfer_shape(class, bytes.len(), chunk_count)?;

        Ok(Self {
            class,
            transfer_id,
            bytes,
            chunk_count,
            next_index: 0,
        })
    }

    pub const fn chunk_count(&self) -> u32 {
        self.chunk_count
    }
}

impl Iterator for TransferChunkIter<'_> {
    type Item = TransferChunk;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.chunk_count {
            return None;
        }

        let start = (self.next_index as usize) * MAX_CHUNK_PAYLOAD;
        let end = start
            .saturating_add(MAX_CHUNK_PAYLOAD)
            .min(self.bytes.len());
        let total_len = u32::try_from(self.bytes.len()).expect("validated by constructor");
        let header = ChunkHeader {
            class: self.class,
            transfer_id: self.transfer_id,
            chunk_index: self.next_index,
            chunk_count: self.chunk_count,
            total_len,
        };
        self.next_index += 1;

        let mut plaintext = Zeroizing::new(Vec::with_capacity(CHUNK_HEADER_BYTES + end - start));
        plaintext.extend_from_slice(&header.encode());
        plaintext.extend_from_slice(&self.bytes[start..end]);
        Some(TransferChunk(plaintext))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.chunk_count - self.next_index) as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for TransferChunkIter<'_> {}

struct InFlightTransfer {
    header: ChunkHeader,
    next_index: u32,
    started_at: Instant,
    bytes: Zeroizing<Vec<u8>>,
}

impl fmt::Debug for InFlightTransfer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InFlightTransfer")
            .field("header", &self.header)
            .field("next_index", &self.next_index)
            .field("started_at", &self.started_at)
            .field("encoded_len", &self.bytes.len())
            .finish()
    }
}

#[derive(Debug)]
pub struct Reassembler {
    timeouts: TimeoutConfig,
    in_flight: Option<InFlightTransfer>,
    last_started_id: Option<u64>,
}

impl Reassembler {
    pub const fn new(timeouts: TimeoutConfig) -> Self {
        Self {
            timeouts,
            in_flight: None,
            last_started_id: None,
        }
    }

    pub fn push(
        &mut self,
        now: Instant,
        plaintext: &[u8],
    ) -> Result<Option<CompletedTransfer>, ChunkError> {
        let result = self.push_inner(now, plaintext);
        if result.is_err() {
            self.in_flight = None;
        }
        result
    }

    pub fn reset(&mut self) {
        self.in_flight = None;
    }

    /// Returns the absolute deadline for the current logical transfer.
    pub fn next_deadline(&self) -> Option<Instant> {
        let in_flight = self.in_flight.as_ref()?;
        in_flight
            .started_at
            .checked_add(in_flight.header.class.timeout(self.timeouts))
    }

    /// Expires an incomplete transfer even when no subsequent chunk arrives.
    pub fn check_timeout(&mut self, now: Instant) -> Result<(), ChunkError> {
        let Some(in_flight) = self.in_flight.as_ref() else {
            return Ok(());
        };
        let timeout = in_flight.header.class.timeout(self.timeouts);
        if now.saturating_duration_since(in_flight.started_at) < timeout {
            return Ok(());
        }
        self.in_flight = None;
        Err(ChunkError::TimedOut(timeout))
    }

    fn push_inner(
        &mut self,
        now: Instant,
        plaintext: &[u8],
    ) -> Result<Option<CompletedTransfer>, ChunkError> {
        self.check_timeout(now)?;
        if plaintext.len() < CHUNK_HEADER_BYTES {
            return Err(ChunkError::HeaderLength(plaintext.len()));
        }

        let header = ChunkHeader::decode(&plaintext[..CHUNK_HEADER_BYTES])?;
        let payload = &plaintext[CHUNK_HEADER_BYTES..];

        if let Some(in_flight) = &self.in_flight {
            if header.class != in_flight.header.class
                || header.transfer_id != in_flight.header.transfer_id
                || header.chunk_count != in_flight.header.chunk_count
                || header.total_len != in_flight.header.total_len
            {
                return Err(ChunkError::InFlightMismatch);
            }
            if header.chunk_index != in_flight.next_index {
                return Err(ChunkError::UnexpectedChunkIndex {
                    actual: header.chunk_index,
                    expected: in_flight.next_index,
                });
            }
        } else {
            if let Some(previous) = self.last_started_id {
                if header.transfer_id <= previous {
                    return Err(ChunkError::ReplayedTransferId {
                        actual: header.transfer_id,
                        previous,
                    });
                }
            }
            if header.chunk_index != 0 {
                return Err(ChunkError::UnexpectedChunkIndex {
                    actual: header.chunk_index,
                    expected: 0,
                });
            }
            self.last_started_id = Some(header.transfer_id);
            self.in_flight = Some(InFlightTransfer {
                header,
                next_index: 0,
                started_at: now,
                bytes: Zeroizing::new(Vec::with_capacity(header.total_len as usize)),
            });
        }

        let in_flight = self.in_flight.as_mut().expect("initialized above");
        let expected_payload =
            expected_payload_len(in_flight.header.total_len as usize, header.chunk_index)?;
        if payload.len() != expected_payload {
            return Err(ChunkError::InvalidPayloadLength {
                actual: payload.len(),
                expected: expected_payload,
            });
        }

        in_flight.bytes.extend_from_slice(payload);
        in_flight.next_index = in_flight
            .next_index
            .checked_add(1)
            .ok_or(ChunkError::LengthOverflow)?;

        if in_flight.next_index != in_flight.header.chunk_count {
            return Ok(None);
        }

        let completed = self.in_flight.take().expect("present until completion");
        if completed.bytes.len() != completed.header.total_len as usize {
            return Err(ChunkError::InvalidPayloadLength {
                actual: completed.bytes.len(),
                expected: completed.header.total_len as usize,
            });
        }
        Ok(Some(CompletedTransfer {
            class: completed.header.class,
            transfer_id: completed.header.transfer_id,
            bytes: completed.bytes,
        }))
    }
}

fn validate_transfer_shape(
    class: TransferClass,
    total_len: usize,
    chunk_count: u32,
) -> Result<(), ChunkError> {
    if total_len == 0 {
        return Err(ChunkError::EmptyTransfer);
    }
    let maximum = class.max_transfer_bytes();
    if total_len > maximum {
        return Err(ChunkError::TransferTooLarge {
            class,
            actual: total_len,
            maximum,
        });
    }
    let expected = expected_chunk_count(total_len)?;
    if chunk_count != expected {
        return Err(ChunkError::InvalidChunkCount {
            actual: chunk_count,
            expected,
        });
    }
    Ok(())
}

fn expected_chunk_count(total_len: usize) -> Result<u32, ChunkError> {
    if total_len == 0 {
        return Err(ChunkError::EmptyTransfer);
    }
    let count = total_len.div_ceil(MAX_CHUNK_PAYLOAD);
    u32::try_from(count).map_err(|_| ChunkError::LengthOverflow)
}

fn expected_payload_len(total_len: usize, chunk_index: u32) -> Result<usize, ChunkError> {
    let offset = (chunk_index as usize)
        .checked_mul(MAX_CHUNK_PAYLOAD)
        .ok_or(ChunkError::LengthOverflow)?;
    let remaining = total_len
        .checked_sub(offset)
        .ok_or(ChunkError::LengthOverflow)?;
    Ok(remaining.min(MAX_CHUNK_PAYLOAD))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use static_assertions::assert_not_impl_any;

    assert_not_impl_any!(CompletedTransfer: Clone);
    assert_not_impl_any!(TransferChunk: Clone);
    assert_not_impl_any!(TransferChunkIter<'static>: Clone);

    fn mutate_header(mut chunk: Vec<u8>, offset: usize, value: u8) -> Vec<u8> {
        chunk[offset] = value;
        chunk
    }

    #[test]
    fn header_encoding_is_exact_and_big_endian() {
        let header = ChunkHeader {
            class: TransferClass::Txn,
            transfer_id: 0x0102_0304_0506_0708,
            chunk_index: 0x1112_1314,
            chunk_count: 0x0000_0002,
            total_len: 0x0000_f001,
        };
        let encoded = header.encode();
        assert_eq!(
            encoded,
            [
                1, 3, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 0x11, 0x12, 0x13, 0x14, 0, 0, 0, 2, 0, 0, 0xf0,
                1,
            ]
        );
        assert_eq!(ChunkHeader::decode(&encoded), Ok(header));
    }

    #[test]
    fn iterator_and_reassembler_cover_chunk_boundaries() {
        let timeouts = TimeoutConfig::default();
        let now = Instant::now();
        for (class, len) in [
            (TransferClass::Control, 1),
            (TransferClass::Control, MAX_CHUNK_PAYLOAD),
            (TransferClass::Control, MAX_CHUNK_PAYLOAD + 1),
            (TransferClass::Txn, MAX_TXN_TRANSFER_BYTES),
        ] {
            let source = vec![0xa5; len];
            let chunks = TransferChunkIter::new(class, 7, &source)
                .unwrap()
                .collect::<Vec<_>>();
            assert_eq!(chunks.len(), len.div_ceil(MAX_CHUNK_PAYLOAD));
            assert!(chunks
                .iter()
                .all(|chunk| chunk.len() <= MAX_NOISE_PLAINTEXT_BYTES));

            let mut reassembler = Reassembler::new(timeouts);
            let mut completed = None;
            for chunk in chunks {
                completed = reassembler.push(now, &chunk).unwrap();
            }
            assert_eq!(completed.unwrap().bytes.as_slice(), source);
        }
    }

    #[test]
    fn ticket_chunk_debug_only_reports_class_and_lengths() {
        let source = vec![211_u8; MAX_CHUNK_PAYLOAD + 1];
        let mut chunks = TransferChunkIter::new(TransferClass::Ticket, 7, &source).unwrap();
        let iter_debug = format!("{chunks:?}");
        assert!(iter_debug.contains("class: Ticket"));
        assert!(iter_debug.contains(&format!("encoded_len: {}", source.len())));
        assert!(!iter_debug.contains("211, 211"));

        let now = Instant::now();
        let mut reassembler = Reassembler::new(TimeoutConfig::default());
        assert_eq!(
            reassembler.push(now, &chunks.next().unwrap()).unwrap(),
            None
        );
        let in_flight_debug = format!("{reassembler:?}");
        assert!(in_flight_debug.contains("class: Ticket"));
        assert!(!in_flight_debug.contains("211, 211"));

        let completed = reassembler
            .push(now, &chunks.next().unwrap())
            .unwrap()
            .unwrap();
        let completed_debug = format!("{completed:?}");
        assert!(completed_debug.contains("class: Ticket"));
        assert!(completed_debug.contains(&format!("encoded_len: {}", source.len())));
        assert!(!completed_debug.contains("211, 211"));
    }

    #[test]
    fn class_caps_are_enforced_before_splitting_or_allocating() {
        for class in [
            TransferClass::Control,
            TransferClass::Ticket,
            TransferClass::Txn,
            TransferClass::Snapshot,
        ] {
            let oversized = vec![0; class.max_transfer_bytes() + 1];
            let exact =
                TransferChunkIter::new(class, 1, &oversized[..class.max_transfer_bytes()]).unwrap();
            assert_eq!(
                exact.chunk_count() as usize,
                class.max_transfer_bytes().div_ceil(MAX_CHUNK_PAYLOAD)
            );
            assert_eq!(
                TransferChunkIter::new(class, 1, &oversized).unwrap_err(),
                ChunkError::TransferTooLarge {
                    class,
                    actual: oversized.len(),
                    maximum: class.max_transfer_bytes(),
                }
            );
        }
    }

    #[test]
    fn malformed_headers_fail_closed() {
        let valid = TransferChunkIter::new(TransferClass::Control, 1, b"x")
            .unwrap()
            .next()
            .unwrap();
        let mut reassembler = Reassembler::new(TimeoutConfig::default());
        let now = Instant::now();

        assert_eq!(
            reassembler.push(now, &[0; 23]),
            Err(ChunkError::HeaderLength(23))
        );
        assert_eq!(
            reassembler.push(now, &mutate_header(valid.to_vec(), 0, 2)),
            Err(ChunkError::UnsupportedVersion(2))
        );
        assert_eq!(
            reassembler.push(now, &mutate_header(valid.to_vec(), 2, 1)),
            Err(ChunkError::ReservedBits)
        );
        assert_eq!(
            reassembler.push(now, &mutate_header(valid.to_vec(), 1, 99)),
            Err(ChunkError::UnknownClass(99))
        );
    }

    #[test]
    fn order_mismatch_clears_in_flight_state() {
        let source = vec![9; MAX_CHUNK_PAYLOAD + 1];
        let chunks = TransferChunkIter::new(TransferClass::Control, 4, &source)
            .unwrap()
            .collect::<Vec<_>>();
        let now = Instant::now();
        let mut reassembler = Reassembler::new(TimeoutConfig::default());

        assert_eq!(reassembler.push(now, &chunks[0]).unwrap(), None);
        assert_eq!(
            reassembler.push(now, &chunks[0]),
            Err(ChunkError::UnexpectedChunkIndex {
                actual: 0,
                expected: 1,
            })
        );
        assert_eq!(
            reassembler.push(now, &chunks[1]),
            Err(ChunkError::ReplayedTransferId {
                actual: 4,
                previous: 4,
            })
        );
        let next = TransferChunkIter::new(TransferClass::Control, 5, &source)
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(reassembler.push(now, &next[0]).unwrap(), None);
        assert!(reassembler.push(now, &next[1]).unwrap().is_some());
    }

    #[test]
    fn completed_transfer_ids_must_increase() {
        let now = Instant::now();
        let mut reassembler = Reassembler::new(TimeoutConfig::default());
        let transfer = |id| {
            TransferChunkIter::new(TransferClass::Control, id, b"x")
                .unwrap()
                .next()
                .unwrap()
        };

        assert!(reassembler.push(now, &transfer(10)).unwrap().is_some());
        assert_eq!(
            reassembler.push(now, &transfer(10)),
            Err(ChunkError::ReplayedTransferId {
                actual: 10,
                previous: 10,
            })
        );
        assert_eq!(
            reassembler.push(now, &transfer(9)),
            Err(ChunkError::ReplayedTransferId {
                actual: 9,
                previous: 10,
            })
        );
        assert!(reassembler.push(now, &transfer(11)).unwrap().is_some());
    }

    #[test]
    fn transfer_timeout_uses_class_specific_deadline_and_clears_state() {
        let timeouts = TimeoutConfig {
            ordinary_transfer: Duration::from_secs(2),
            snapshot_transfer: Duration::from_secs(8),
            ..TimeoutConfig::default()
        };
        let now = Instant::now();
        let mut reassembler = Reassembler::new(timeouts);
        let source = vec![1; MAX_CHUNK_PAYLOAD + 1];
        let control = TransferChunkIter::new(TransferClass::Control, 1, &source)
            .unwrap()
            .collect::<Vec<_>>();

        assert_eq!(reassembler.push(now, &control[0]).unwrap(), None);
        assert_eq!(
            reassembler.next_deadline(),
            Some(now + Duration::from_secs(2))
        );
        assert_eq!(
            reassembler.push(now + Duration::from_secs(2), &control[1]),
            Err(ChunkError::TimedOut(Duration::from_secs(2)))
        );
        let retry = TransferChunkIter::new(TransferClass::Control, 2, &source)
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(reassembler.push(now, &retry[0]).unwrap(), None);
        assert!(reassembler.push(now, &retry[1]).unwrap().is_some());

        let snapshot = TransferChunkIter::new(TransferClass::Snapshot, 3, &source)
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(reassembler.push(now, &snapshot[0]).unwrap(), None);
        assert_eq!(
            reassembler
                .push(now + Duration::from_secs(2), &snapshot[1])
                .unwrap()
                .unwrap()
                .transfer_id,
            3
        );
    }

    #[test]
    fn transfer_timeout_fires_without_another_chunk() {
        let timeouts = TimeoutConfig {
            ordinary_transfer: Duration::from_secs(2),
            snapshot_transfer: Duration::from_secs(8),
            ..TimeoutConfig::default()
        };
        let start = Instant::now();
        let source = vec![1; MAX_CHUNK_PAYLOAD + 1];
        let first = TransferChunkIter::new(TransferClass::Snapshot, 1, &source)
            .unwrap()
            .next()
            .unwrap();
        let mut reassembler = Reassembler::new(timeouts);

        assert_eq!(reassembler.push(start, &first).unwrap(), None);
        assert_eq!(
            reassembler.check_timeout(start + Duration::from_secs(7)),
            Ok(())
        );
        assert_eq!(
            reassembler.check_timeout(start + Duration::from_secs(8)),
            Err(ChunkError::TimedOut(Duration::from_secs(8)))
        );
        assert_eq!(reassembler.next_deadline(), None);
    }

    #[test]
    fn exact_payload_lengths_are_required() {
        let source = vec![3; MAX_CHUNK_PAYLOAD + 1];
        let mut chunks = TransferChunkIter::new(TransferClass::Control, 3, &source)
            .unwrap()
            .collect::<Vec<_>>();
        chunks[0].0.pop();
        let mut reassembler = Reassembler::new(TimeoutConfig::default());

        assert_eq!(
            reassembler.push(Instant::now(), &chunks[0]),
            Err(ChunkError::InvalidPayloadLength {
                actual: MAX_CHUNK_PAYLOAD - 1,
                expected: MAX_CHUNK_PAYLOAD,
            })
        );
    }

    #[test]
    fn forged_count_or_total_is_rejected_and_clears_state() {
        let source = vec![4; MAX_CHUNK_PAYLOAD + 1];
        let chunks = TransferChunkIter::new(TransferClass::Control, 12, &source)
            .unwrap()
            .collect::<Vec<_>>();
        let now = Instant::now();
        let mut reassembler = Reassembler::new(TimeoutConfig::default());

        let mut forged_count = chunks[0].to_vec();
        forged_count[16..20].copy_from_slice(&3_u32.to_be_bytes());
        assert_eq!(
            reassembler.push(now, &forged_count),
            Err(ChunkError::InvalidChunkCount {
                actual: 3,
                expected: 2,
            })
        );

        assert_eq!(reassembler.push(now, &chunks[0]).unwrap(), None);
        let mut forged_total = chunks[1].to_vec();
        forged_total[20..24].copy_from_slice(&(source.len() as u32 - 1).to_be_bytes());
        assert_eq!(
            reassembler.push(now, &forged_total),
            Err(ChunkError::InvalidChunkCount {
                actual: 2,
                expected: 1,
            })
        );

        let retry = TransferChunkIter::new(TransferClass::Control, 13, &source)
            .unwrap()
            .collect::<Vec<_>>();
        assert_eq!(reassembler.push(now, &retry[0]).unwrap(), None);
        assert!(reassembler.push(now, &retry[1]).unwrap().is_some());
    }
}
