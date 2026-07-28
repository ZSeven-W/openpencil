use crate::auth::{expected_issuer, expected_subject, SmokeAuth};
use crate::fixtures;
use anyhow::{bail, Context, Result};
use op_collab::{CollabMessage, Commit, Epoch, FrameEnvelope, Role};
use op_collab_transport::{
    accept_secure_tcp, connect_secure_tcp, m1_wire_limits, ActiveConnectionGuard, AdmissionHello,
    AdmissionIdentity, ConnectionLimiter, DeviceStaticKey, EncodedFrameTransfer, JoinIntent,
    SecureConnection, ServerPrelude, TransportConfig,
};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::time::Instant;

pub const OWNER_TICKET_ID: &str = "ZmF1bHQtb3duZXItdGlja2V0LTAwMDE";
pub const GUEST_TICKET_ID: &str = "ZmF1bHQtZ3Vlc3QtdGlja2V0LTAwMDE";

pub struct OwnerLink {
    pub connection: SecureConnection<TcpStream>,
    pub guest_identity: AdmissionIdentity,
    pub guest_intent: JoinIntent,
    _active: ActiveConnectionGuard,
}

pub struct GuestLink {
    pub connection: SecureConnection<TcpStream>,
    pub epoch: Epoch,
}

pub fn owner_listener(port_file: &Path) -> Result<TcpListener> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind fault-matrix owner listener")?;
    publish_address(port_file, listener.local_addr()?)?;
    Ok(listener)
}

pub fn accept_guest(
    listener: &TcpListener,
    limiter: &ConnectionLimiter,
    owner_key: &DeviceStaticKey,
    owner_auth: &SmokeAuth,
    epoch: Epoch,
) -> Result<OwnerLink> {
    let config = TransportConfig::default().validate()?;
    let (stream, peer_address) = listener.accept().context("accept fault-matrix guest")?;
    let pending = limiter.try_begin_handshake(peer_address.ip())?;
    let prelude = ServerPrelude::new(
        fixtures::DISCOVERY_ID.to_owned(),
        fixtures::session_id(),
        epoch,
    )?;
    let mut connection = accept_secure_tcp(stream, owner_key, &prelude, config)?;
    let local_hello = AdmissionHello::new(owner_auth.ticket().to_vec(), JoinIntent::New)?;
    let (guest_hello, guest_identity) = connection.exchange_admission_responder(
        &local_hello,
        owner_auth.verifier(),
        expected_issuer(),
        expected_subject(),
        owner_auth.now_unix_ms(),
        Instant::now(),
    )?;
    connection.authorize_remote(Role::Editor)?;
    connection.activate(Instant::now())?;
    let active = pending.activate()?;
    Ok(OwnerLink {
        connection,
        guest_identity,
        guest_intent: guest_hello.intent().clone(),
        _active: active,
    })
}

pub fn connect_owner(
    address: SocketAddr,
    guest_key: &DeviceStaticKey,
    guest_auth: &SmokeAuth,
    intent: JoinIntent,
) -> Result<GuestLink> {
    let config = TransportConfig::default().validate()?;
    let (prelude, mut connection) =
        connect_secure_tcp(address, guest_key, Some(fixtures::DISCOVERY_ID), config)?;
    let hello = AdmissionHello::new(guest_auth.ticket().to_vec(), intent)?;
    let _ = connection.exchange_admission_initiator(
        &hello,
        guest_auth.verifier(),
        expected_issuer(),
        expected_subject(),
        guest_auth.now_unix_ms(),
        Instant::now(),
    )?;
    connection.authorize_remote(Role::Owner)?;
    connection.activate(Instant::now())?;
    if prelude.prelude().session_id() != &fixtures::session_id() {
        bail!("fault-matrix prelude changed the collaboration session");
    }
    Ok(GuestLink {
        connection,
        epoch: prelude.prelude().epoch(),
    })
}

pub fn send(
    connection: &mut SecureConnection<TcpStream>,
    epoch: Epoch,
    message: CollabMessage,
) -> Result<()> {
    connection.send_frame(
        &FrameEnvelope::new(fixtures::session_id(), epoch, message),
        Instant::now(),
    )?;
    Ok(())
}

pub fn send_commit(
    connection: &mut SecureConnection<TcpStream>,
    epoch: Epoch,
    commit: &Commit,
) -> Result<()> {
    let encoded = EncodedFrameTransfer::encode_commit(
        &fixtures::session_id(),
        epoch,
        commit,
        m1_wire_limits(),
    )?;
    connection.send_encoded_frame(&encoded, Instant::now())?;
    Ok(())
}

fn publish_address(path: &Path, address: SocketAddr) -> Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, address.to_string()).context("write fault-matrix port file")?;
    std::fs::rename(&temporary, path).context("publish fault-matrix port file")
}
