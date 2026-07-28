use std::io::{Read, Write};

use op_collab::{Epoch, SessionId, COLLAB_PROTOCOL_VERSION, MAX_IDENTIFIER_BYTES};

use crate::PreludeError;

pub const TRANSPORT_PROTOCOL_VERSION: u16 = 1;
pub const MAX_DISCOVERY_ID_BYTES: usize = 128;
pub const MAX_SERVER_PRELUDE_BYTES: usize = 2 * 1024;

const PRELUDE_MAGIC: &[u8; 8] = b"OPCOLLAB";
const FIXED_PRELUDE_BYTES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerPrelude {
    discovery_id: String,
    session_id: SessionId,
    epoch: Epoch,
}

impl ServerPrelude {
    pub fn new(
        discovery_id: String,
        session_id: SessionId,
        epoch: Epoch,
    ) -> Result<Self, PreludeError> {
        validate_identifier("discovery_id", &discovery_id, MAX_DISCOVERY_ID_BYTES)?;
        validate_identifier(
            "session_id",
            session_id.as_ref(),
            MAX_IDENTIFIER_BYTES as usize,
        )?;
        Ok(Self {
            discovery_id,
            session_id,
            epoch,
        })
    }

    pub fn discovery_id(&self) -> &str {
        &self.discovery_id
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub const fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn encode(&self) -> Result<EncodedServerPrelude, PreludeError> {
        let discovery = self.discovery_id.as_bytes();
        let session = self.session_id.as_ref().as_bytes();
        let total = FIXED_PRELUDE_BYTES
            .checked_add(discovery.len())
            .and_then(|length| length.checked_add(session.len()))
            .ok_or(PreludeError::TooLarge {
                actual: usize::MAX,
                maximum: MAX_SERVER_PRELUDE_BYTES,
            })?;
        if total > MAX_SERVER_PRELUDE_BYTES {
            return Err(PreludeError::TooLarge {
                actual: total,
                maximum: MAX_SERVER_PRELUDE_BYTES,
            });
        }
        let discovery_len = u16::try_from(discovery.len()).map_err(|_| PreludeError::Malformed)?;
        let session_len = u16::try_from(session.len()).map_err(|_| PreludeError::Malformed)?;
        let mut raw = Vec::with_capacity(total);
        raw.extend_from_slice(PRELUDE_MAGIC);
        raw.extend_from_slice(&TRANSPORT_PROTOCOL_VERSION.to_be_bytes());
        raw.extend_from_slice(&COLLAB_PROTOCOL_VERSION.to_be_bytes());
        raw.extend_from_slice(&self.epoch.0.to_be_bytes());
        raw.extend_from_slice(&discovery_len.to_be_bytes());
        raw.extend_from_slice(&session_len.to_be_bytes());
        raw.extend_from_slice(discovery);
        raw.extend_from_slice(session);
        Ok(EncodedServerPrelude {
            prelude: self.clone(),
            raw,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedServerPrelude {
    prelude: ServerPrelude,
    raw: Vec<u8>,
}

impl EncodedServerPrelude {
    pub fn prelude(&self) -> &ServerPrelude {
        &self.prelude
    }

    /// Exact bytes mixed into the Noise prologue.
    pub fn prologue_bytes(&self) -> &[u8] {
        &self.raw
    }

    pub fn into_parts(self) -> (ServerPrelude, Vec<u8>) {
        (self.prelude, self.raw)
    }

    pub fn decode(raw: &[u8]) -> Result<Self, PreludeError> {
        if raw.len() < FIXED_PRELUDE_BYTES {
            return Err(PreludeError::Malformed);
        }
        if raw.len() > MAX_SERVER_PRELUDE_BYTES {
            return Err(PreludeError::TooLarge {
                actual: raw.len(),
                maximum: MAX_SERVER_PRELUDE_BYTES,
            });
        }
        if &raw[..PRELUDE_MAGIC.len()] != PRELUDE_MAGIC {
            return Err(PreludeError::Magic);
        }
        let transport_version = read_u16(raw, 8)?;
        if transport_version != TRANSPORT_PROTOCOL_VERSION {
            return Err(PreludeError::UnsupportedTransportVersion {
                actual: transport_version,
                expected: TRANSPORT_PROTOCOL_VERSION,
            });
        }
        let collab_version = read_u16(raw, 10)?;
        if collab_version != COLLAB_PROTOCOL_VERSION {
            return Err(PreludeError::UnsupportedCollabVersion {
                actual: collab_version,
                expected: COLLAB_PROTOCOL_VERSION,
            });
        }
        let epoch = Epoch(read_u64(raw, 12)?);
        let discovery_len = usize::from(read_u16(raw, 20)?);
        let session_len = usize::from(read_u16(raw, 22)?);
        let expected = FIXED_PRELUDE_BYTES
            .checked_add(discovery_len)
            .and_then(|length| length.checked_add(session_len))
            .ok_or(PreludeError::Malformed)?;
        if expected != raw.len() {
            return Err(PreludeError::Malformed);
        }
        let discovery_end = FIXED_PRELUDE_BYTES + discovery_len;
        let discovery_id = std::str::from_utf8(&raw[FIXED_PRELUDE_BYTES..discovery_end])
            .map_err(|_| PreludeError::Utf8)?
            .to_owned();
        let session_id = std::str::from_utf8(&raw[discovery_end..])
            .map_err(|_| PreludeError::Utf8)?
            .to_owned();
        let prelude = ServerPrelude::new(discovery_id, SessionId::from(session_id), epoch)?;
        Ok(Self {
            prelude,
            raw: raw.to_vec(),
        })
    }
}

pub fn write_server_prelude(
    writer: &mut impl Write,
    prelude: &ServerPrelude,
) -> Result<EncodedServerPrelude, PreludeError> {
    let encoded = prelude.encode()?;
    writer.write_all(encoded.prologue_bytes())?;
    writer.flush()?;
    Ok(encoded)
}

pub fn read_server_prelude(reader: &mut impl Read) -> Result<EncodedServerPrelude, PreludeError> {
    let mut fixed = [0_u8; FIXED_PRELUDE_BYTES];
    reader.read_exact(&mut fixed)?;
    let discovery_len = usize::from(u16::from_be_bytes([fixed[20], fixed[21]]));
    let session_len = usize::from(u16::from_be_bytes([fixed[22], fixed[23]]));
    let total = FIXED_PRELUDE_BYTES
        .checked_add(discovery_len)
        .and_then(|length| length.checked_add(session_len))
        .ok_or(PreludeError::Malformed)?;
    if total > MAX_SERVER_PRELUDE_BYTES {
        return Err(PreludeError::TooLarge {
            actual: total,
            maximum: MAX_SERVER_PRELUDE_BYTES,
        });
    }
    let mut raw = Vec::with_capacity(total);
    raw.extend_from_slice(&fixed);
    raw.resize(total, 0);
    reader.read_exact(&mut raw[FIXED_PRELUDE_BYTES..])?;
    EncodedServerPrelude::decode(&raw)
}

fn validate_identifier(
    field: &'static str,
    value: &str,
    maximum: usize,
) -> Result<(), PreludeError> {
    if value.is_empty() || value.len() > maximum {
        return Err(PreludeError::InvalidIdentifier { field });
    }
    Ok(())
}

fn read_u16(raw: &[u8], offset: usize) -> Result<u16, PreludeError> {
    let bytes = raw.get(offset..offset + 2).ok_or(PreludeError::Malformed)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn read_u64(raw: &[u8], offset: usize) -> Result<u64, PreludeError> {
    let bytes = raw.get(offset..offset + 8).ok_or(PreludeError::Malformed)?;
    let mut value = [0_u8; 8];
    value.copy_from_slice(bytes);
    Ok(u64::from_be_bytes(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ServerPrelude {
        ServerPrelude::new(
            "discover_a".to_owned(),
            SessionId::from("session_a"),
            Epoch(42),
        )
        .unwrap()
    }

    #[test]
    fn exact_shape_round_trips_and_is_the_prologue() {
        let encoded = fixture().encode().unwrap();
        assert_eq!(&encoded.prologue_bytes()[..8], b"OPCOLLAB");
        assert_eq!(
            &encoded.prologue_bytes()[8..10],
            &TRANSPORT_PROTOCOL_VERSION.to_be_bytes()
        );
        assert_eq!(
            EncodedServerPrelude::decode(encoded.prologue_bytes()).unwrap(),
            encoded
        );
    }

    #[test]
    fn stream_read_uses_bounded_lengths() {
        let encoded = fixture().encode().unwrap();
        let mut cursor = std::io::Cursor::new(encoded.prologue_bytes().to_vec());
        assert_eq!(read_server_prelude(&mut cursor).unwrap(), encoded);

        let mut malicious = vec![0_u8; FIXED_PRELUDE_BYTES];
        malicious[..8].copy_from_slice(PRELUDE_MAGIC);
        malicious[8..10].copy_from_slice(&TRANSPORT_PROTOCOL_VERSION.to_be_bytes());
        malicious[10..12].copy_from_slice(&COLLAB_PROTOCOL_VERSION.to_be_bytes());
        malicious[20..22].copy_from_slice(&u16::MAX.to_be_bytes());
        let error = read_server_prelude(&mut std::io::Cursor::new(malicious)).unwrap_err();
        assert!(matches!(error, PreludeError::TooLarge { .. }));
    }

    #[test]
    fn version_and_trailing_bytes_fail_closed() {
        let encoded = fixture().encode().unwrap();
        let mut wrong = encoded.prologue_bytes().to_vec();
        wrong[9] = 2;
        assert!(matches!(
            EncodedServerPrelude::decode(&wrong),
            Err(PreludeError::UnsupportedTransportVersion { .. })
        ));

        let mut trailing = encoded.prologue_bytes().to_vec();
        trailing.push(0);
        assert!(matches!(
            EncodedServerPrelude::decode(&trailing),
            Err(PreludeError::Malformed)
        ));
    }
}
