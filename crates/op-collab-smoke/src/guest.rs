use crate::auth::{
    expected_issuer, expected_subject, SmokeAuth, GUEST_AVATAR_URL, GUEST_DEVICE_ID,
    GUEST_DISPLAY_NAME, OWNER_AVATAR_URL, OWNER_DISPLAY_NAME,
};
use crate::fixtures;
use anyhow::{bail, Context, Result};
use op_collab::{
    canonical_document_hash, CollabMessage, FrameEnvelope, GuestEffect, GuestSessionConfig,
    GuestSessionCore, Role,
};
use op_collab_transport::{
    connect_secure_tcp, AdmissionHello, DeviceStaticKey, JoinIntent, TransportConfig,
};
use std::collections::VecDeque;
use std::net::{SocketAddr, TcpStream};
use std::time::Instant;

const GUEST_TICKET_ID: &str = "Z3Vlc3QtdGlja2V0LTAwMDE";

pub fn run(address: SocketAddr) -> Result<String> {
    let guest_key = DeviceStaticKey::from_private([0x41; 32])?;
    let auth = SmokeAuth::for_device(
        &guest_key,
        GUEST_DEVICE_ID,
        GUEST_TICKET_ID,
        GUEST_DISPLAY_NAME,
        GUEST_AVATAR_URL,
    )?;
    let config = TransportConfig::default().validate()?;
    let (prelude, mut connection) =
        connect_secure_tcp(address, &guest_key, Some(fixtures::DISCOVERY_ID), config)?;
    let hello = AdmissionHello::new(auth.ticket().to_vec(), JoinIntent::New)?;
    let _ = connection.exchange_admission_initiator(
        &hello,
        auth.verifier(),
        expected_issuer(),
        expected_subject(),
        auth.now_unix_ms(),
        Instant::now(),
    )?;
    connection.authorize_remote(Role::Owner)?;
    connection.activate(Instant::now())?;

    let welcome = match connection.receive_frame()?.into_body() {
        CollabMessage::Welcome(welcome) => welcome,
        _ => bail!("guest expected welcome"),
    };
    if prelude.prelude().session_id() != &fixtures::session_id()
        || prelude.prelude().epoch() != fixtures::EPOCH
    {
        bail!("prelude and collaboration session differ");
    }
    let owner_profile = welcome
        .participants
        .iter()
        .find(|participant| participant.role == Role::Owner)
        .context("welcome owner roster entry")?;
    let guest_profile = welcome
        .participants
        .iter()
        .find(|participant| participant.peer_id.as_ref() == fixtures::GUEST_PEER)
        .context("welcome guest roster entry")?;
    if owner_profile.display_name.as_deref() != Some(OWNER_DISPLAY_NAME)
        || owner_profile.avatar_url.as_deref() != Some(OWNER_AVATAR_URL)
        || guest_profile.display_name.as_deref() != Some(GUEST_DISPLAY_NAME)
        || guest_profile.avatar_url.as_deref() != Some(GUEST_AVATAR_URL)
    {
        bail!("welcome roster did not preserve signed participant profiles");
    }
    let namespace = welcome.peer_namespace.clone();
    let mut core = GuestSessionCore::new(
        fixtures::session_id(),
        fixtures::EPOCH,
        welcome,
        GuestSessionConfig::default(),
    )?;
    let mut document = fixtures::initial_document()?;
    let snapshot = connection.receive_frame()?;
    let effects = core.accept_frame(snapshot)?;
    drive_effects(&mut core, &mut document, &mut connection, effects)?;

    let desired = fixtures::desired_guest_document(&namespace)?;
    let effects = vec![core.begin_local_edit(&desired)?];
    drive_effects(&mut core, &mut document, &mut connection, effects)?;

    let mut followup_submitted = false;
    loop {
        let effects = core.accept_frame(connection.receive_frame()?)?;
        drive_effects(&mut core, &mut document, &mut connection, effects)?;
        if core.pending_edit().is_none()
            && core.confirmed_seq().is_some_and(|seq| seq.0 == 2)
            && !followup_submitted
        {
            let desired = fixtures::desired_guest_followup(&document)?;
            let effects = vec![core.begin_local_edit(&desired)?];
            drive_effects(&mut core, &mut document, &mut connection, effects)?;
            followup_submitted = true;
        }
        if core.pending_edit().is_none()
            && core.confirmed_document().is_some()
            && core.confirmed_seq().is_some_and(|seq| seq.0 == 3)
        {
            let hash = canonical_document_hash(&document)?;
            if core.confirmed_hash() != Some(hash) {
                bail!("guest confirmed hash differs from installed document");
            }
            if hash != canonical_document_hash(&fixtures::expected_alternating_document()?)? {
                bail!("alternating smoke converged to the wrong document semantics");
            }
            return Ok(hash.to_string());
        }
    }
}

fn drive_effects(
    core: &mut GuestSessionCore,
    document: &mut jian_ops_schema::PenDocument,
    connection: &mut op_collab_transport::SecureConnection<TcpStream>,
    effects: Vec<GuestEffect>,
) -> Result<()> {
    let mut pending = VecDeque::from(effects);
    while let Some(effect) = pending.pop_front() {
        match effect {
            GuestEffect::Send(message) => send(connection, message)?,
            GuestEffect::PrepareInstall(mut prepared) => {
                *document = prepared
                    .take_candidate_document()
                    .context("guest candidate document")?;
                let installed_hash = canonical_document_hash(document)?;
                pending.extend(core.finalize_install(*prepared, installed_hash)?);
            }
            GuestEffect::ParticipantJoined(_)
            | GuestEffect::ParticipantLeft(_)
            | GuestEffect::PresenceChanged(_)
            | GuestEffect::PendingCancelled { .. }
            | GuestEffect::VerifyRenewal { .. }
            | GuestEffect::UndoResult(_) => {}
            GuestEffect::SessionEnded { reason } => {
                bail!("guest session ended during smoke: {reason:?}")
            }
        }
    }
    Ok(())
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
