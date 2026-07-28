use std::collections::VecDeque;
use std::io::{Cursor, Read};

use op_collab_transport::{IncrementalRecordDecoder, RecordError, MAX_NOISE_CIPHERTEXT_BYTES};

struct FragmentedReader {
    fragments: VecDeque<Result<Vec<u8>, std::io::ErrorKind>>,
    current: Cursor<Vec<u8>>,
}

impl FragmentedReader {
    fn new(fragments: Vec<Result<Vec<u8>, std::io::ErrorKind>>) -> Self {
        Self {
            fragments: fragments.into(),
            current: Cursor::new(Vec::new()),
        }
    }
}

impl Read for FragmentedReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.current.position() < self.current.get_ref().len() as u64 {
            return self.current.read(buffer);
        }
        match self.fragments.pop_front() {
            Some(Ok(fragment)) => {
                self.current = Cursor::new(fragment);
                self.current.read(buffer)
            }
            Some(Err(kind)) => Err(std::io::Error::from(kind)),
            None => Err(std::io::Error::from(std::io::ErrorKind::WouldBlock)),
        }
    }
}

#[test]
fn incremental_decoder_keeps_partial_prefix_and_body() {
    let body = vec![7_u8; 40];
    let mut wire = Vec::new();
    wire.extend_from_slice(&(body.len() as u16).to_be_bytes());
    wire.extend_from_slice(&body);
    let mut reader = FragmentedReader::new(vec![
        Ok(wire[..1].to_vec()),
        Err(std::io::ErrorKind::WouldBlock),
        Ok(wire[1..9].to_vec()),
        Err(std::io::ErrorKind::WouldBlock),
        Ok(wire[9..].to_vec()),
    ]);
    let mut decoder = IncrementalRecordDecoder::new();

    assert!(decoder.poll(&mut reader).unwrap().is_none());
    assert_eq!(decoder.buffered_len(), 1);
    assert!(decoder.poll(&mut reader).unwrap().is_none());
    assert_eq!(decoder.buffered_len(), 9);
    assert_eq!(decoder.poll(&mut reader).unwrap().unwrap(), body);
    assert!(!decoder.has_partial_record());
}

#[test]
fn incremental_decoder_rejects_length_before_body_allocation() {
    let oversized = u16::try_from(MAX_NOISE_CIPHERTEXT_BYTES + 1)
        .unwrap()
        .to_be_bytes();
    let mut decoder = IncrementalRecordDecoder::new();
    let error = decoder.poll(&mut Cursor::new(oversized)).unwrap_err();
    assert!(matches!(
        error,
        RecordError::InvalidLength {
            actual,
            maximum: MAX_NOISE_CIPHERTEXT_BYTES
        } if actual == MAX_NOISE_CIPHERTEXT_BYTES + 1
    ));
}
