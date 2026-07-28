use std::fmt;
use std::io::Read;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use polling::{Event, Events, Poller};

use crate::{RecordError, RuntimeError, TokenBucket, MAX_NOISE_CIPHERTEXT_BYTES};

const SOCKET_EVENT_KEY: usize = 1;

/// Incremental `u16` ciphertext record decoder.
///
/// Partial prefixes and bodies remain buffered when a nonblocking reader
/// returns `WouldBlock`; allocation happens only after the bounded prefix has
/// been validated.
#[derive(Default)]
pub struct IncrementalRecordDecoder {
    prefix: [u8; 2],
    prefix_len: usize,
    body: Vec<u8>,
    body_len: usize,
}

impl fmt::Debug for IncrementalRecordDecoder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IncrementalRecordDecoder")
            .field("buffered_len", &self.buffered_len())
            .field("expected_body_len", &self.body.len())
            .finish()
    }
}

impl IncrementalRecordDecoder {
    pub const fn new() -> Self {
        Self {
            prefix: [0; 2],
            prefix_len: 0,
            body: Vec::new(),
            body_len: 0,
        }
    }

    pub fn buffered_len(&self) -> usize {
        self.prefix_len + self.body_len
    }

    pub fn has_partial_record(&self) -> bool {
        self.buffered_len() != 0
    }

    pub fn poll<R: Read>(&mut self, reader: &mut R) -> Result<Option<Vec<u8>>, RecordError> {
        loop {
            if self.prefix_len < self.prefix.len() {
                match reader.read(&mut self.prefix[self.prefix_len..]) {
                    Ok(0) => return Err(unexpected_eof().into()),
                    Ok(read) => {
                        self.prefix_len += read;
                        if self.prefix_len < self.prefix.len() {
                            continue;
                        }
                        let length = usize::from(u16::from_be_bytes(self.prefix));
                        if length == 0 || length > MAX_NOISE_CIPHERTEXT_BYTES {
                            return Err(RecordError::InvalidLength {
                                actual: length,
                                maximum: MAX_NOISE_CIPHERTEXT_BYTES,
                            });
                        }
                        self.body.resize(length, 0);
                        self.body_len = 0;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        return Ok(None);
                    }
                    Err(error) => return Err(error.into()),
                }
            }

            match reader.read(&mut self.body[self.body_len..]) {
                Ok(0) => return Err(unexpected_eof().into()),
                Ok(read) => {
                    self.body_len += read;
                    if self.body_len < self.body.len() {
                        continue;
                    }
                    self.prefix_len = 0;
                    self.body_len = 0;
                    return Ok(Some(std::mem::take(&mut self.body)));
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
                Err(error) => return Err(error.into()),
            }
        }
    }
}

pub(crate) struct SocketWaiter {
    poller: Poller,
}

impl SocketWaiter {
    pub(crate) fn new(stream: &TcpStream) -> std::io::Result<Self> {
        let poller = Poller::new()?;
        // SAFETY: ConnectionDriver unregisters this stream in Drop before the
        // owned TcpStream is dropped.
        unsafe {
            poller.add(stream, Event::none(SOCKET_EVENT_KEY))?;
        }
        Ok(Self { poller })
    }

    pub(crate) fn wait(
        &self,
        stream: &TcpStream,
        readable: bool,
        writable: bool,
        timeout: Duration,
    ) -> std::io::Result<()> {
        if timeout.is_zero() {
            return Ok(());
        }
        if !readable && !writable {
            std::thread::sleep(timeout);
            return Ok(());
        }
        self.poller
            .modify(stream, Event::new(SOCKET_EVENT_KEY, readable, writable))?;
        let mut events = Events::new();
        self.poller.wait(&mut events, Some(timeout))?;
        Ok(())
    }

    pub(crate) fn delete(&self, stream: &TcpStream) {
        let _ = self.poller.delete(stream);
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProgressDeadline {
    last_progress: Option<Instant>,
}

impl ProgressDeadline {
    pub(crate) fn arm(&mut self, now: Instant) {
        self.last_progress.get_or_insert(now);
    }

    pub(crate) fn note_progress(&mut self, now: Instant) {
        self.last_progress = Some(now);
    }

    pub(crate) fn clear(&mut self) {
        self.last_progress = None;
    }

    pub(crate) fn next_deadline(&self, timeout: Duration) -> Option<Instant> {
        self.last_progress?.checked_add(timeout)
    }

    pub(crate) fn expired(&self, now: Instant, timeout: Duration) -> bool {
        self.last_progress
            .is_some_and(|last| now.saturating_duration_since(last) >= timeout)
    }
}

pub(crate) fn rate_ready_at(
    records: &mut TokenBucket,
    bytes: &mut TokenBucket,
    byte_count: u64,
    now: Instant,
) -> Result<Instant, RuntimeError> {
    let record_delay = records
        .time_until_available(1, now)
        .ok_or(RuntimeError::RateLimited)?;
    let byte_delay = bytes
        .time_until_available(byte_count, now)
        .ok_or(RuntimeError::RateLimited)?;
    now.checked_add(record_delay.max(byte_delay))
        .ok_or(RuntimeError::RateLimited)
}

pub(crate) fn consume_rate(
    records: &mut TokenBucket,
    bytes: &mut TokenBucket,
    byte_count: u64,
    now: Instant,
) -> Result<(), RuntimeError> {
    if !records.try_take(1, now) || !bytes.try_take(byte_count, now) {
        return Err(RuntimeError::RateLimited);
    }
    Ok(())
}

fn unexpected_eof() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::UnexpectedEof,
        "peer closed inside a ciphertext record",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_deadline_is_monotonic_and_clearable() {
        let start = Instant::now();
        let timeout = Duration::from_secs(3);
        let mut deadline = ProgressDeadline::default();
        assert_eq!(deadline.next_deadline(timeout), None);
        deadline.arm(start);
        deadline.arm(start + Duration::from_secs(1));
        assert_eq!(deadline.next_deadline(timeout), Some(start + timeout));
        deadline.note_progress(start);
        assert!(!deadline.expired(start + Duration::from_secs(2), timeout));
        assert!(deadline.expired(start + timeout, timeout));
        deadline.note_progress(start + Duration::from_secs(2));
        assert_eq!(
            deadline.next_deadline(timeout),
            Some(start + Duration::from_secs(5))
        );
        deadline.clear();
        assert_eq!(deadline.next_deadline(timeout), None);
    }

    #[test]
    fn decoder_debug_does_not_expose_buffered_ciphertext() {
        let decoder = IncrementalRecordDecoder {
            prefix: 4_u16.to_be_bytes(),
            prefix_len: 2,
            body: vec![211; 4],
            body_len: 1,
        };

        let debug = format!("{decoder:?}");
        assert!(debug.contains("buffered_len: 3"));
        assert!(debug.contains("expected_body_len: 4"));
        assert!(!debug.contains("211"));
    }
}
