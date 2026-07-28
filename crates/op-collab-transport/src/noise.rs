use std::fmt;
use std::io::{Read, Write};

use snow::{params::NoiseParams, Builder, HandshakeState, TransportState};

use crate::{DeviceStaticKey, EncodedServerPrelude, NoiseTransportError};

pub const NOISE_PROTOCOL_NAME: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
const MAX_HANDSHAKE_MESSAGE_BYTES: usize = 4 * 1024;

pub struct NoiseSession {
    state: TransportState,
    remote_static: [u8; 32],
}

impl NoiseSession {
    pub fn remote_static(&self) -> &[u8; 32] {
        &self.remote_static
    }

    pub(crate) fn transport_mut(&mut self) -> &mut TransportState {
        &mut self.state
    }
}

impl fmt::Debug for NoiseSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NoiseSession")
            .field("remote_static", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

pub fn run_xx_initiator(
    stream: &mut (impl Read + Write),
    local_static: &DeviceStaticKey,
    prelude: &EncodedServerPrelude,
) -> Result<NoiseSession, NoiseTransportError> {
    let mut handshake = build_handshake(local_static, prelude, true)?;
    write_handshake_message(stream, &mut handshake)?;
    read_handshake_message(stream, &mut handshake)?;
    write_handshake_message(stream, &mut handshake)?;
    finish_handshake(handshake)
}

pub fn run_xx_responder(
    stream: &mut (impl Read + Write),
    local_static: &DeviceStaticKey,
    prelude: &EncodedServerPrelude,
) -> Result<NoiseSession, NoiseTransportError> {
    let mut handshake = build_handshake(local_static, prelude, false)?;
    read_handshake_message(stream, &mut handshake)?;
    write_handshake_message(stream, &mut handshake)?;
    read_handshake_message(stream, &mut handshake)?;
    finish_handshake(handshake)
}

fn build_handshake(
    local_static: &DeviceStaticKey,
    prelude: &EncodedServerPrelude,
    initiator: bool,
) -> Result<HandshakeState, NoiseTransportError> {
    let params: NoiseParams = NOISE_PROTOCOL_NAME
        .parse()
        .map_err(NoiseTransportError::Configuration)?;
    let builder = Builder::new(params)
        .local_private_key(local_static.private_key())
        .prologue(prelude.prologue_bytes());
    if initiator {
        builder
            .build_initiator()
            .map_err(NoiseTransportError::Configuration)
    } else {
        builder
            .build_responder()
            .map_err(NoiseTransportError::Configuration)
    }
}

fn write_handshake_message(
    stream: &mut (impl Write + ?Sized),
    handshake: &mut HandshakeState,
) -> Result<(), NoiseTransportError> {
    let mut message = [0_u8; MAX_HANDSHAKE_MESSAGE_BYTES];
    let length = handshake
        .write_message(&[], &mut message)
        .map_err(NoiseTransportError::Handshake)?;
    if length == 0 || length > MAX_HANDSHAKE_MESSAGE_BYTES {
        return Err(NoiseTransportError::HandshakeFrameLength(length));
    }
    let length_prefix = u16::try_from(length)
        .map_err(|_| NoiseTransportError::HandshakeFrameLength(length))?
        .to_be_bytes();
    stream.write_all(&length_prefix)?;
    stream.write_all(&message[..length])?;
    stream.flush()?;
    Ok(())
}

fn read_handshake_message(
    stream: &mut (impl Read + ?Sized),
    handshake: &mut HandshakeState,
) -> Result<(), NoiseTransportError> {
    let mut prefix = [0_u8; 2];
    stream.read_exact(&mut prefix)?;
    let length = usize::from(u16::from_be_bytes(prefix));
    if length == 0 || length > MAX_HANDSHAKE_MESSAGE_BYTES {
        return Err(NoiseTransportError::HandshakeFrameLength(length));
    }
    let mut message = [0_u8; MAX_HANDSHAKE_MESSAGE_BYTES];
    stream.read_exact(&mut message[..length])?;
    let mut payload = [0_u8; MAX_HANDSHAKE_MESSAGE_BYTES];
    let payload_len = handshake
        .read_message(&message[..length], &mut payload)
        .map_err(NoiseTransportError::Handshake)?;
    if payload_len != 0 {
        return Err(NoiseTransportError::HandshakeFrameLength(payload_len));
    }
    Ok(())
}

fn finish_handshake(handshake: HandshakeState) -> Result<NoiseSession, NoiseTransportError> {
    let remote = handshake
        .get_remote_static()
        .filter(|key| key.len() == 32)
        .ok_or(NoiseTransportError::MissingRemoteStatic)?;
    let mut remote_static = [0_u8; 32];
    remote_static.copy_from_slice(remote);
    let state = handshake
        .into_transport_mode()
        .map_err(NoiseTransportError::Handshake)?;
    Ok(NoiseSession {
        state,
        remote_static,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_collab::{Epoch, SessionId};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    fn prelude(discovery: &str) -> EncodedServerPrelude {
        crate::ServerPrelude::new(discovery.to_owned(), SessionId::from("session"), Epoch(7))
            .unwrap()
            .encode()
            .unwrap()
    }

    #[test]
    fn xx_handshake_authenticates_both_static_keys() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let initiator_key = DeviceStaticKey::from_private([3_u8; 32]).unwrap();
        let expected_initiator_public = *initiator_key.public_key();
        let responder_key = DeviceStaticKey::from_private([9_u8; 32]).unwrap();
        let expected_responder_public = *responder_key.public_key();
        let responder_prelude = prelude("discovery");

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            run_xx_responder(&mut stream, &responder_key, &responder_prelude).unwrap()
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let initiator =
            run_xx_initiator(&mut stream, &initiator_key, &prelude("discovery")).unwrap();
        let responder = server.join().unwrap();
        assert_eq!(initiator.remote_static(), &expected_responder_public);
        assert_eq!(responder.remote_static(), &expected_initiator_public);
    }

    #[test]
    fn changed_prelude_breaks_the_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            run_xx_responder(
                &mut stream,
                &DeviceStaticKey::from_private([5_u8; 32]).unwrap(),
                &prelude("server"),
            )
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let client = run_xx_initiator(
            &mut stream,
            &DeviceStaticKey::from_private([6_u8; 32]).unwrap(),
            &prelude("tampered"),
        );
        let server = server.join().unwrap();
        assert!(client.is_err() || server.is_err());
    }
}
