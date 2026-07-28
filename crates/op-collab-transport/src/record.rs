use std::io::{Read, Write};
use std::time::Instant;

use zeroize::Zeroizing;

use crate::{RecordError, MAX_NOISE_CIPHERTEXT_BYTES, MAX_NOISE_PLAINTEXT_BYTES};

/// Authenticated transport-level liveness marker, never a business envelope.
pub const TRANSPORT_HEARTBEAT_PLAINTEXT: &[u8; 8] = b"OPHB\0\0\0\x01";

pub fn encrypt_record(
    transport: &mut snow::TransportState,
    plaintext: &[u8],
) -> Result<Vec<u8>, RecordError> {
    validate_record_length(plaintext.len(), MAX_NOISE_PLAINTEXT_BYTES)?;
    let capacity = plaintext
        .len()
        .checked_add(crate::config::NOISE_AEAD_TAG_BYTES)
        .ok_or(RecordError::InvalidLength {
            actual: plaintext.len(),
            maximum: MAX_NOISE_PLAINTEXT_BYTES,
        })?;
    let mut ciphertext = vec![0_u8; capacity];
    let written = transport
        .write_message(plaintext, &mut ciphertext)
        .map_err(RecordError::Encrypt)?;
    ciphertext.truncate(written);
    validate_record_length(ciphertext.len(), MAX_NOISE_CIPHERTEXT_BYTES)?;
    Ok(ciphertext)
}

pub fn decrypt_record(
    transport: &mut snow::TransportState,
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, RecordError> {
    validate_record_length(ciphertext.len(), MAX_NOISE_CIPHERTEXT_BYTES)?;
    let mut plaintext = Zeroizing::new(vec![0_u8; ciphertext.len()]);
    let written = transport
        .read_message(ciphertext, &mut plaintext)
        .map_err(RecordError::Decrypt)?;
    plaintext.truncate(written);
    validate_record_length(plaintext.len(), MAX_NOISE_PLAINTEXT_BYTES)?;
    Ok(plaintext)
}

pub fn read_ciphertext_record<R: Read>(reader: &mut R) -> Result<Vec<u8>, RecordError> {
    let mut prefix = [0_u8; 2];
    reader.read_exact(&mut prefix)?;
    let length = u16::from_be_bytes(prefix) as usize;
    validate_record_length(length, MAX_NOISE_CIPHERTEXT_BYTES)?;

    let mut ciphertext = vec![0_u8; length];
    reader.read_exact(&mut ciphertext)?;
    Ok(ciphertext)
}

pub(crate) fn read_ciphertext_record_until<R: Read>(
    reader: &mut R,
    deadline: Instant,
) -> Result<Vec<u8>, RecordError> {
    let mut prefix = [0_u8; 2];
    read_exact_until(reader, &mut prefix, deadline)?;
    let length = u16::from_be_bytes(prefix) as usize;
    validate_record_length(length, MAX_NOISE_CIPHERTEXT_BYTES)?;

    let mut ciphertext = vec![0_u8; length];
    read_exact_until(reader, &mut ciphertext, deadline)?;
    Ok(ciphertext)
}

pub fn write_ciphertext_record<W: Write>(
    writer: &mut W,
    ciphertext: &[u8],
) -> Result<(), RecordError> {
    validate_record_length(ciphertext.len(), MAX_NOISE_CIPHERTEXT_BYTES)?;
    let length = u16::try_from(ciphertext.len()).map_err(|_| RecordError::InvalidLength {
        actual: ciphertext.len(),
        maximum: MAX_NOISE_CIPHERTEXT_BYTES,
    })?;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(ciphertext)?;
    Ok(())
}

fn validate_record_length(actual: usize, maximum: usize) -> Result<(), RecordError> {
    if actual == 0 || actual > maximum {
        return Err(RecordError::InvalidLength { actual, maximum });
    }
    Ok(())
}

fn read_exact_until<R: Read>(
    reader: &mut R,
    mut buffer: &mut [u8],
    deadline: Instant,
) -> Result<(), RecordError> {
    while !buffer.is_empty() {
        ensure_before(deadline)?;
        match reader.read(buffer) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "peer closed inside a ciphertext record",
                )
                .into())
            }
            Ok(read) => {
                ensure_before(deadline)?;
                buffer = &mut buffer[read..];
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn ensure_before(deadline: Instant) -> Result<(), RecordError> {
    if Instant::now() < deadline {
        return Ok(());
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "ciphertext record read deadline expired",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::time::Duration;

    use snow::{params::NoiseParams, Builder, TransportState};

    use super::*;

    fn transport_pair() -> (TransportState, TransportState) {
        let params: NoiseParams = "Noise_NN_25519_ChaChaPoly_BLAKE2s".parse().unwrap();
        let mut initiator = Builder::new(params.clone()).build_initiator().unwrap();
        let mut responder = Builder::new(params).build_responder().unwrap();
        let mut handshake = vec![0_u8; 65_535];
        let mut payload = vec![0_u8; 65_535];

        let first = initiator.write_message(&[], &mut handshake).unwrap();
        responder
            .read_message(&handshake[..first], &mut payload)
            .unwrap();
        let second = responder.write_message(&[], &mut handshake).unwrap();
        initiator
            .read_message(&handshake[..second], &mut payload)
            .unwrap();

        (
            initiator.into_transport_mode().unwrap(),
            responder.into_transport_mode().unwrap(),
        )
    }

    #[test]
    fn record_prefix_is_big_endian_and_round_trips() {
        let ciphertext = vec![7_u8; 0x0102];
        let mut encoded = Vec::new();
        write_ciphertext_record(&mut encoded, &ciphertext).unwrap();
        assert_eq!(&encoded[..2], &[0x01, 0x02]);
        assert_eq!(
            read_ciphertext_record(&mut Cursor::new(encoded)).unwrap(),
            ciphertext
        );
    }

    #[test]
    fn exact_ciphertext_boundary_round_trips() {
        let ciphertext = vec![3_u8; MAX_NOISE_CIPHERTEXT_BYTES];
        let mut encoded = Vec::new();
        write_ciphertext_record(&mut encoded, &ciphertext).unwrap();
        assert_eq!(encoded.len(), MAX_NOISE_CIPHERTEXT_BYTES + 2);
        assert_eq!(
            read_ciphertext_record(&mut Cursor::new(encoded)).unwrap(),
            ciphertext
        );
    }

    #[test]
    fn expired_monotonic_deadline_fails_before_reading() {
        let mut encoded = Cursor::new(vec![0, 1, 7]);
        let deadline = Instant::now() - Duration::from_millis(1);
        assert!(matches!(
            read_ciphertext_record_until(&mut encoded, deadline),
            Err(RecordError::Io(error)) if error.kind() == std::io::ErrorKind::TimedOut
        ));
        assert_eq!(encoded.position(), 0);
    }

    #[test]
    fn reader_rejects_length_before_allocating_or_reading_body() {
        assert!(matches!(
            read_ciphertext_record(&mut Cursor::new([0_u8, 0])),
            Err(RecordError::InvalidLength {
                actual: 0,
                maximum: MAX_NOISE_CIPHERTEXT_BYTES,
            })
        ));

        let oversized = u16::try_from(MAX_NOISE_CIPHERTEXT_BYTES + 1)
            .unwrap()
            .to_be_bytes();
        assert!(matches!(
            read_ciphertext_record(&mut Cursor::new(oversized)),
            Err(RecordError::InvalidLength {
                actual,
                maximum: MAX_NOISE_CIPHERTEXT_BYTES,
            }) if actual == MAX_NOISE_CIPHERTEXT_BYTES + 1
        ));
    }

    #[test]
    fn writer_rejects_empty_and_oversized_records() {
        let mut output = Vec::new();
        assert!(matches!(
            write_ciphertext_record(&mut output, &[]),
            Err(RecordError::InvalidLength { actual: 0, .. })
        ));
        assert!(matches!(
            write_ciphertext_record(
                &mut output,
                &vec![0; MAX_NOISE_CIPHERTEXT_BYTES + 1]
            ),
            Err(RecordError::InvalidLength { actual, .. })
                if actual == MAX_NOISE_CIPHERTEXT_BYTES + 1
        ));
        assert!(output.is_empty());
    }

    #[test]
    fn noise_record_accepts_exact_plaintext_boundary() {
        let (mut initiator, mut responder) = transport_pair();
        let plaintext = vec![0x5a; MAX_NOISE_PLAINTEXT_BYTES];
        let ciphertext = encrypt_record(&mut initiator, &plaintext).unwrap();
        assert_eq!(ciphertext.len(), MAX_NOISE_CIPHERTEXT_BYTES);
        assert_eq!(
            decrypt_record(&mut responder, &ciphertext)
                .unwrap()
                .as_slice(),
            plaintext
        );
    }

    #[test]
    fn noise_record_rejects_plaintext_over_boundary_without_advancing_nonce() {
        let (mut initiator, mut responder) = transport_pair();
        assert!(matches!(
            encrypt_record(
                &mut initiator,
                &vec![0; MAX_NOISE_PLAINTEXT_BYTES + 1]
            ),
            Err(RecordError::InvalidLength {
                actual,
                maximum: MAX_NOISE_PLAINTEXT_BYTES,
            }) if actual == MAX_NOISE_PLAINTEXT_BYTES + 1
        ));

        let ciphertext = encrypt_record(&mut initiator, b"still nonce zero").unwrap();
        assert_eq!(
            decrypt_record(&mut responder, &ciphertext)
                .unwrap()
                .as_slice(),
            b"still nonce zero"
        );
    }

    #[test]
    fn tampering_is_authenticated() {
        let (mut initiator, mut responder) = transport_pair();
        let mut ciphertext = encrypt_record(&mut initiator, b"authenticated").unwrap();
        *ciphertext.last_mut().unwrap() ^= 1;
        assert!(matches!(
            decrypt_record(&mut responder, &ciphertext),
            Err(RecordError::Decrypt(_))
        ));
    }
}
