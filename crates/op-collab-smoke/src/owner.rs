use crate::auth::{
    expected_issuer, expected_subject, SmokeAuth, GUEST_AVATAR_URL, GUEST_DISPLAY_NAME,
    OWNER_AVATAR_URL, OWNER_DEVICE_ID, OWNER_DISPLAY_NAME,
};
use crate::fixtures;
use anyhow::{bail, Context, Result};
use op_collab::{
    canonical_document_hash, diff_supported, ClientOpId, CollabMessage, CommitSeq, ConnectionKey,
    DiffContext, FrameEnvelope, OwnerEffect, OwnerSessionConfig, OwnerSessionCore, PeerId,
    PeerNamespace, Role, Submit,
};
use op_collab_transport::{
    accept_secure_tcp, m1_wire_limits, AdmissionHello, ConnectionLimiter, DeviceStaticKey,
    EncodedFrameTransfer, JoinIntent, ServerPrelude, TransportConfig,
};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::time::Instant;

const OWNER_TICKET_ID: &str = "b3duZXItdGlja2V0LTAwMDE";

pub struct LanOwnerResult {
    pub bound_address: SocketAddr,
    pub canonical_hash: String,
}

pub fn run(port_file: &Path) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").context("bind owner listener")?;
    publish_address(port_file, listener.local_addr()?)?;
    serve_listener(listener)
}

pub fn run_lan(bind_address: SocketAddr) -> Result<LanOwnerResult> {
    validate_lan_bind_address(bind_address)?;
    let listener = TcpListener::bind(bind_address).context("bind LAN owner listener")?;
    let bound_address = listener
        .local_addr()
        .context("read LAN owner listener address")?;
    eprintln!("LAN owner ready on {bound_address}");
    let canonical_hash = serve_listener(listener)?;
    Ok(LanOwnerResult {
        bound_address,
        canonical_hash,
    })
}

fn serve_listener(listener: TcpListener) -> Result<String> {
    let config = TransportConfig::default().validate()?;
    let limiter = ConnectionLimiter::new(config.connections)?;
    let (stream, peer_address) = listener.accept().context("accept guest")?;
    let pending = limiter.try_begin_handshake(peer_address.ip())?;
    serve(stream, pending, config)
}

fn validate_lan_bind_address(address: SocketAddr) -> Result<()> {
    let ip = address.ip();
    if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
        bail!("LAN owner requires an explicit unicast interface address");
    }
    Ok(())
}

fn serve(
    stream: TcpStream,
    pending: op_collab_transport::PendingHandshakeGuard,
    config: TransportConfig,
) -> Result<String> {
    let owner_key = DeviceStaticKey::from_private([0x31; 32])?;
    let auth = SmokeAuth::for_device(
        &owner_key,
        OWNER_DEVICE_ID,
        OWNER_TICKET_ID,
        OWNER_DISPLAY_NAME,
        OWNER_AVATAR_URL,
    )?;
    let prelude = ServerPrelude::new(
        fixtures::DISCOVERY_ID.to_owned(),
        fixtures::session_id(),
        fixtures::EPOCH,
    )?;
    let mut connection = accept_secure_tcp(stream, &owner_key, &prelude, config)?;
    let local_hello = AdmissionHello::new(auth.ticket().to_vec(), JoinIntent::New)?;
    let (_, guest_identity) = connection.exchange_admission_responder(
        &local_hello,
        auth.verifier(),
        expected_issuer(),
        expected_subject(),
        auth.now_unix_ms(),
        Instant::now(),
    )?;
    connection.authorize_remote(Role::Editor)?;
    connection.activate(Instant::now())?;
    let _active = pending.activate()?;

    let mut document = fixtures::initial_document()?;
    let owner_grant = fixtures::grant(
        auth.local_auth().clone(),
        Role::Owner,
        fixtures::OWNER_PARTICIPANT,
        fixtures::OWNER_PEER,
        fixtures::OWNER_NAMESPACE,
    )?;
    let owner_connection = ConnectionKey::new(1).context("owner connection key")?;
    let guest_connection = ConnectionKey::new(2).context("guest connection key")?;
    let mut core = OwnerSessionCore::new(
        fixtures::session_id(),
        fixtures::EPOCH,
        CommitSeq(0),
        owner_connection,
        owner_grant,
        &document,
        OwnerSessionConfig::default(),
    )?;
    let guest_grant = fixtures::grant(
        guest_identity.to_auth_metadata(),
        Role::Editor,
        fixtures::GUEST_PARTICIPANT,
        fixtures::GUEST_PEER,
        fixtures::GUEST_NAMESPACE,
    )?;
    let activation = core.activate_peer(guest_connection, guest_grant, &document)?;
    if activation.joined.display_name.as_deref() != Some(GUEST_DISPLAY_NAME)
        || activation.joined.avatar_url.as_deref() != Some(GUEST_AVATAR_URL)
    {
        bail!("owner roster did not preserve the guest's signed profile");
    }
    send(&mut connection, CollabMessage::Welcome(activation.welcome))?;
    let snapshot = activation.snapshot.context("new guest receives snapshot")?;
    send(&mut connection, CollabMessage::Snapshot(Box::new(snapshot)))?;

    let mut owner_edit_committed = false;
    loop {
        let frame = connection.receive_frame()?;
        let effects = core.accept_frame(guest_connection, frame, &document)?;
        let Some(mut candidate) = take_prepared(&mut connection, effects)? else {
            continue;
        };
        document = candidate
            .take_candidate_document()
            .context("owner candidate document")?;
        let installed_hash = canonical_document_hash(&document)?;
        let finalized = core.finalize_install(*candidate, installed_hash)?;
        route_effect(&mut connection, finalized)?;

        if core.seq() == CommitSeq(1) && !owner_edit_committed {
            let desired = fixtures::desired_owner_document(&document)?;
            let supported = diff_supported(
                &document,
                &desired,
                &DiffContext::new(
                    PeerNamespace::try_from(fixtures::OWNER_NAMESPACE)?,
                    Role::Owner,
                    Some(0),
                ),
            )?;
            let submit = Submit {
                client_op_id: ClientOpId {
                    peer_id: PeerId::from(fixtures::OWNER_PEER),
                    local_counter: 1,
                },
                base_seq: core.seq(),
                txn: supported.txn,
            };
            let effects = core.accept_frame(
                owner_connection,
                FrameEnvelope::new(
                    fixtures::session_id(),
                    fixtures::EPOCH,
                    CollabMessage::Submit(submit),
                ),
                &document,
            )?;
            let mut owner_candidate = take_prepared(&mut connection, effects)?
                .context("owner local edit receives install candidate")?;
            document = owner_candidate
                .take_candidate_document()
                .context("owner local candidate document")?;
            let hash = canonical_document_hash(&document)?;
            let finalized = core.finalize_install(*owner_candidate, hash)?;
            route_effect(&mut connection, finalized)?;
            owner_edit_committed = true;
        }

        if core.seq() == CommitSeq(3) {
            let hash = canonical_document_hash(&document)?;
            if hash != canonical_document_hash(&fixtures::expected_alternating_document()?)? {
                bail!("alternating smoke converged to the wrong document semantics");
            }
            return Ok(hash.to_string());
        }
    }
}

fn take_prepared(
    connection: &mut op_collab_transport::SecureConnection<TcpStream>,
    effects: Vec<OwnerEffect>,
) -> Result<Option<Box<op_collab::PreparedCommit>>> {
    let mut prepared = None;
    for effect in effects {
        match effect {
            OwnerEffect::PrepareInstall(candidate) => {
                if prepared.replace(candidate).is_some() {
                    bail!("owner produced multiple install candidates");
                }
            }
            effect => route_effect(connection, effect)?,
        }
    }
    Ok(prepared)
}

fn route_effect(
    connection: &mut op_collab_transport::SecureConnection<TcpStream>,
    effect: OwnerEffect,
) -> Result<()> {
    match effect {
        OwnerEffect::Reply { message, .. } | OwnerEffect::Broadcast { message } => {
            send(connection, message)
        }
        OwnerEffect::ReplyCommit { commit, .. } | OwnerEffect::BroadcastCommit { commit } => {
            send_commit(connection, &commit)
        }
        OwnerEffect::CommitBatch { commits, .. } => {
            for commit in commits {
                send_commit(connection, &commit)?;
            }
            Ok(())
        }
        OwnerEffect::Snapshot { snapshot, .. } => {
            send(connection, CollabMessage::Snapshot(snapshot))
        }
        OwnerEffect::UndoCommitted { result, commit, .. } => {
            send(connection, CollabMessage::UndoResult(result))?;
            send_commit(connection, &commit)
        }
        OwnerEffect::PrepareInstall(_) => bail!("nested owner install candidate"),
        OwnerEffect::VerifyRenewal { .. } | OwnerEffect::UndoRequested(_) => {
            bail!("unexpected owner control effect in smoke")
        }
        OwnerEffect::Close { reason, .. } => {
            send(connection, CollabMessage::Bye(op_collab::Bye { reason }))
        }
    }
}

fn send(
    connection: &mut op_collab_transport::SecureConnection<TcpStream>,
    message: CollabMessage,
) -> Result<()> {
    connection.send_frame(
        &FrameEnvelope::new(fixtures::session_id(), fixtures::EPOCH, message),
        Instant::now(),
    )?;
    Ok(())
}

fn send_commit(
    connection: &mut op_collab_transport::SecureConnection<TcpStream>,
    commit: &op_collab::Commit,
) -> Result<()> {
    let encoded = EncodedFrameTransfer::encode_commit(
        &fixtures::session_id(),
        fixtures::EPOCH,
        commit,
        m1_wire_limits(),
    )?;
    connection.send_encoded_frame(&encoded, Instant::now())?;
    Ok(())
}

fn publish_address(path: &Path, address: std::net::SocketAddr) -> Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, address.to_string()).context("write owner port file")?;
    std::fs::rename(&temporary, path).context("publish owner port file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_owner_requires_an_explicit_unicast_interface() {
        for address in [
            "0.0.0.0:45123",
            "127.0.0.1:45123",
            "224.0.0.251:45123",
            "[::]:45123",
            "[::1]:45123",
            "[ff02::fb]:45123",
        ] {
            assert!(validate_lan_bind_address(address.parse().unwrap()).is_err());
        }
        assert!(validate_lan_bind_address("192.168.1.20:0".parse().unwrap()).is_ok());
        assert!(validate_lan_bind_address("[fd00::20]:45123".parse().unwrap()).is_ok());
    }
}
