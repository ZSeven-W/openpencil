use crate::auth::{SmokeAuth, GUEST_AVATAR_URL, GUEST_DEVICE_ID, GUEST_DISPLAY_NAME};
use crate::fault_transport::{self, GUEST_TICKET_ID};
use crate::fixtures;
use crate::scenario::Scenario;
use anyhow::{bail, Context, Result};
use op_collab::{
    canonical_document_hash, Bye, ByeReason, CollabMessage, CommitSeq, FrameEnvelope,
    GuestConnectionState, GuestEffect, GuestError, GuestSessionConfig, GuestSessionCore,
    PendingEditStatus, RejectCode, Welcome,
};
use op_collab_transport::{DeviceStaticKey, JoinIntent, ResumeHint, SecureConnection};
use std::collections::VecDeque;
use std::net::{SocketAddr, TcpStream};

#[derive(Default)]
struct GuestSignals {
    session_ended: Option<ByeReason>,
    pending_cancelled: usize,
    suppressed_applied: usize,
}

pub fn run(scenario: Scenario, address: SocketAddr) -> Result<String> {
    let guest_key = DeviceStaticKey::from_private([0x41; 32])?;
    let auth = SmokeAuth::for_device(
        &guest_key,
        GUEST_DEVICE_ID,
        GUEST_TICKET_ID,
        GUEST_DISPLAY_NAME,
        GUEST_AVATAR_URL,
    )?;
    match scenario {
        Scenario::RetryExactlyOnce => retry_exactly_once(address, &guest_key, &auth),
        Scenario::StaleRebase => stale_rebase(address, &guest_key, &auth),
        Scenario::AtomicTxnFailure => atomic_txn_failure(address, &guest_key, &auth),
        Scenario::ReconnectCatchUp => reconnect(address, &guest_key, &auth, false),
        Scenario::ReconnectSnapshot => reconnect(address, &guest_key, &auth, true),
        Scenario::EpochChange => epoch_change(address, &guest_key, &auth),
        Scenario::OwnerLeft => owner_left(address, &guest_key, &auth),
    }
}

fn retry_exactly_once(
    address: SocketAddr,
    guest_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link = fault_transport::connect_owner(address, guest_key, auth, JoinIntent::New)?;
    let (mut core, mut document) = join(&mut link)?;
    let first_desired = crate::scenario::with_position(&document, 10.0, 0.0)?;
    let first_submit = take_submit(core.begin_local_edit(&first_desired)?)?;
    fault_transport::send(
        &mut link.connection,
        link.epoch,
        CollabMessage::Submit(first_submit),
    )?;

    require_transport_closed(&mut link.connection)?;
    core.disconnect();
    drop(link);

    let mut resumed = fault_transport::connect_owner(
        address,
        guest_key,
        auth,
        JoinIntent::Resume(resume_hint(&core)),
    )?;
    let welcome = take_welcome(resumed.connection.receive_frame()?)?;
    let effects = core.resume(fixtures::session_id(), resumed.epoch, welcome)?;
    if !matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::CatchUp(catch_up))]
            if catch_up.after_seq == CommitSeq(0)
    ) {
        bail!("lost Commit did not recover through same-epoch CatchUp");
    }
    drive_effects(
        &mut core,
        &mut document,
        &mut resumed.connection,
        resumed.epoch,
        effects,
    )?;
    let effects = core.accept_frame(resumed.connection.receive_frame()?)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut resumed.connection,
        resumed.epoch,
        effects,
    )?;
    require_converged(&core, &document, CommitSeq(1))?;

    let second_desired = crate::scenario::with_position(&document, 10.0, 5.0)?;
    let unsent = take_submit(core.begin_local_edit(&second_desired)?)?;
    core.disconnect();
    drop(resumed);

    let mut retried = fault_transport::connect_owner(
        address,
        guest_key,
        auth,
        JoinIntent::Resume(resume_hint(&core)),
    )?;
    let welcome = take_welcome(retried.connection.receive_frame()?)?;
    let effects = core.resume(fixtures::session_id(), retried.epoch, welcome)?;
    let replay = effects.iter().find_map(|effect| match effect {
        GuestEffect::Send(CollabMessage::Submit(submit)) => Some(submit),
        _ => None,
    });
    if replay != Some(&unsent) || effects.len() != 1 {
        bail!("same-epoch recovery did not resend the exact retained Submit");
    }
    drive_effects(
        &mut core,
        &mut document,
        &mut retried.connection,
        retried.epoch,
        effects,
    )?;
    let effects = core.accept_frame(retried.connection.receive_frame()?)?;
    let signals = drive_effects_with_applied_fault(
        &mut core,
        &mut document,
        &mut retried.connection,
        retried.epoch,
        effects,
        true,
    )?;
    if signals.suppressed_applied != 1 {
        bail!("Applied-loss fault did not suppress the committed acknowledgement");
    }
    require_converged(&core, &document, CommitSeq(2))?;
    core.disconnect();
    drop(retried);

    let mut acknowledged = fault_transport::connect_owner(
        address,
        guest_key,
        auth,
        JoinIntent::Resume(resume_hint(&core)),
    )?;
    let welcome = take_welcome(acknowledged.connection.receive_frame()?)?;
    let effects = core.resume(fixtures::session_id(), acknowledged.epoch, welcome)?;
    if !matches!(
        effects.as_slice(),
        [GuestEffect::Send(CollabMessage::CatchUp(catch_up))]
            if catch_up.after_seq == CommitSeq(2)
    ) {
        bail!("lost Applied did not preserve the exact catch-up boundary");
    }
    drive_effects(
        &mut core,
        &mut document,
        &mut acknowledged.connection,
        acknowledged.epoch,
        effects,
    )?;
    let effects = core.accept_frame(acknowledged.connection.receive_frame()?)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut acknowledged.connection,
        acknowledged.epoch,
        effects,
    )?;
    require_converged(&core, &document, CommitSeq(3))?;
    let expected = crate::scenario::with_name(
        &crate::scenario::with_position(&crate::scenario::initial_document()?, 10.0, 5.0)?,
        "Ack recovery owner",
    )?;
    if canonical_document_hash(&document)? != canonical_document_hash(&expected)? {
        bail!("recovered retries and lost Applied did not converge");
    }
    Ok(canonical_document_hash(&document)?.to_string())
}

fn stale_rebase(
    address: SocketAddr,
    guest_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link = fault_transport::connect_owner(address, guest_key, auth, JoinIntent::New)?;
    let (mut core, mut document) = join(&mut link)?;
    let desired = crate::scenario::with_position(&document, 10.0, 0.0)?;
    let submit = take_submit(core.begin_local_edit(&desired)?)?;
    fault_transport::send(
        &mut link.connection,
        link.epoch,
        CollabMessage::Submit(submit),
    )?;

    let reject_effects = core.accept_frame(link.connection.receive_frame()?)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut link.connection,
        link.epoch,
        reject_effects,
    )?;
    let pending = core
        .pending_edit()
        .context("stale reject retains pending edit")?;
    if pending.status() != PendingEditStatus::AwaitingCatchUp {
        bail!("stale reject did not move pending edit into catch-up");
    }

    let catch_up_effects = core.accept_frame(link.connection.receive_frame()?)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut link.connection,
        link.epoch,
        catch_up_effects,
    )?;
    let pending = core
        .pending_edit()
        .context("remote commit preserves rebased pending edit")?;
    if pending.client_op_id().local_counter != 2 || pending.base_seq() != CommitSeq(1) {
        bail!("stale edit was not recomputed under a fresh client operation id");
    }
    let rebased = crate::scenario::with_position(&crate::scenario::initial_document()?, 10.0, 5.0)?;
    if canonical_document_hash(core.displayed_document().context("displayed document")?)?
        != canonical_document_hash(&rebased)?
    {
        bail!("remote commit did not cross and rebase the pending edit");
    }

    let committed_effects = core.accept_frame(link.connection.receive_frame()?)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut link.connection,
        link.epoch,
        committed_effects,
    )?;
    require_converged(&core, &document, CommitSeq(2))?;
    Ok(canonical_document_hash(&document)?.to_string())
}

fn atomic_txn_failure(
    address: SocketAddr,
    guest_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link = fault_transport::connect_owner(address, guest_key, auth, JoinIntent::New)?;
    let (mut core, mut document) = join(&mut link)?;
    let initial_hash = canonical_document_hash(&document)?;
    fault_transport::send(
        &mut link.connection,
        link.epoch,
        CollabMessage::Submit(crate::scenario::invalid_atomic_submit()?),
    )?;
    let frame = link.connection.receive_frame()?;
    let CollabMessage::Reject(reject) = frame.body() else {
        bail!("atomic transaction failure did not return Reject");
    };
    if reject.code != RejectCode::PreconditionFailed || reject.owner_seq != CommitSeq(0) {
        bail!("atomic transaction failure returned the wrong owner verdict");
    }
    let effects = core.accept_frame(frame)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut link.connection,
        link.epoch,
        effects,
    )?;
    if canonical_document_hash(&document)? != initial_hash
        || core.confirmed_seq() != Some(CommitSeq(0))
    {
        bail!("failed transaction left a partial mutation");
    }
    Ok(initial_hash.to_string())
}

fn reconnect(
    address: SocketAddr,
    guest_key: &DeviceStaticKey,
    auth: &SmokeAuth,
    expect_snapshot: bool,
) -> Result<String> {
    let mut first = fault_transport::connect_owner(address, guest_key, auth, JoinIntent::New)?;
    let (mut core, mut document) = join(&mut first)?;
    let resume_hint = ResumeHint {
        participant_id: core.participant_id().clone(),
        peer_id: core.peer_id().clone(),
        peer_namespace: core.peer_namespace().clone(),
        role: core.role(),
    };
    fault_transport::send(
        &mut first.connection,
        first.epoch,
        CollabMessage::Bye(Bye {
            reason: ByeReason::Normal,
        }),
    )?;
    core.disconnect();
    drop(first);

    let mut resumed =
        fault_transport::connect_owner(address, guest_key, auth, JoinIntent::Resume(resume_hint))?;
    if resumed.epoch != fixtures::EPOCH {
        bail!("same-session reconnect changed epoch");
    }
    let welcome = take_welcome(resumed.connection.receive_frame()?)?;
    let effects = core.resume(fixtures::session_id(), resumed.epoch, welcome)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut resumed.connection,
        resumed.epoch,
        effects,
    )?;

    let first_recovery = resumed.connection.receive_frame()?;
    let recovered_with_snapshot = matches!(first_recovery.body(), CollabMessage::Snapshot(_));
    if recovered_with_snapshot != expect_snapshot {
        bail!(
            "reconnect recovery kind mismatch: expected snapshot={expect_snapshot}, actual={recovered_with_snapshot}"
        );
    }
    let effects = core.accept_frame(first_recovery)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut resumed.connection,
        resumed.epoch,
        effects,
    )?;
    while core.confirmed_seq().is_none_or(|seq| seq < CommitSeq(2)) {
        let effects = core.accept_frame(resumed.connection.receive_frame()?)?;
        drive_effects(
            &mut core,
            &mut document,
            &mut resumed.connection,
            resumed.epoch,
            effects,
        )?;
    }
    require_converged(&core, &document, CommitSeq(2))?;
    let expected = crate::scenario::with_name(
        &crate::scenario::with_position(&crate::scenario::initial_document()?, 21.0, 0.0)?,
        "Offline owner final",
    )?;
    let hash = canonical_document_hash(&document)?;
    if hash != canonical_document_hash(&expected)? {
        bail!("same-epoch recovery converged to the wrong document semantics");
    }
    Ok(hash.to_string())
}

fn epoch_change(
    address: SocketAddr,
    guest_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut first = fault_transport::connect_owner(address, guest_key, auth, JoinIntent::New)?;
    let (mut old_core, mut old_document) = join(&mut first)?;
    let desired = crate::scenario::with_position(&old_document, 77.0, 0.0)?;
    let unsent = old_core.begin_local_edit(&desired)?;
    if !matches!(unsent, GuestEffect::Send(CollabMessage::Submit(_))) {
        bail!("old epoch did not retain a pending Submit");
    }
    fault_transport::send(
        &mut first.connection,
        first.epoch,
        CollabMessage::Bye(Bye {
            reason: ByeReason::Normal,
        }),
    )?;
    old_core.disconnect();
    drop(first);

    let mut replacement =
        fault_transport::connect_owner(address, guest_key, auth, JoinIntent::New)?;
    let welcome = take_welcome(replacement.connection.receive_frame()?)?;
    if replacement.epoch == fixtures::EPOCH {
        bail!("replacement owner did not advance the epoch");
    }
    let resume_error = old_core
        .resume(fixtures::session_id(), replacement.epoch, welcome.clone())
        .expect_err("new epoch must reject old-core resume");
    if !matches!(resume_error, GuestError::WrongEpoch)
        || old_core.state() != GuestConnectionState::Ended
        || old_core.pending_edit().is_none()
    {
        bail!("new epoch did not terminate and quarantine the old pending edit");
    }

    let mut new_core = GuestSessionCore::new(
        fixtures::session_id(),
        replacement.epoch,
        welcome,
        GuestSessionConfig::default(),
    )?;
    let effects = new_core.accept_frame(replacement.connection.receive_frame()?)?;
    drive_effects(
        &mut new_core,
        &mut old_document,
        &mut replacement.connection,
        replacement.epoch,
        effects,
    )?;
    require_converged(&new_core, &old_document, CommitSeq(0))?;
    let expected = crate::scenario::replacement_epoch_document()?;
    if canonical_document_hash(&old_document)? != canonical_document_hash(&expected)? {
        bail!("old pending edit leaked into the replacement epoch");
    }
    Ok(canonical_document_hash(&old_document)?.to_string())
}

fn owner_left(
    address: SocketAddr,
    guest_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link = fault_transport::connect_owner(address, guest_key, auth, JoinIntent::New)?;
    let (mut core, mut document) = join(&mut link)?;
    let effects = core.accept_frame(link.connection.receive_frame()?)?;
    let signals = drive_effects(
        &mut core,
        &mut document,
        &mut link.connection,
        link.epoch,
        effects,
    )?;
    if signals.session_ended != Some(ByeReason::OwnerLeft)
        || core.state() != GuestConnectionState::Ended
    {
        bail!("owner departure did not end the guest session");
    }
    let desired = crate::scenario::with_position(&document, 12.0, 0.0)?;
    if !matches!(
        core.begin_local_edit(&desired),
        Err(GuestError::SessionEnded)
    ) {
        bail!("ended guest session remained writable");
    }
    let fork = crate::scenario::with_name(&document, "Saved as local fork")?;
    if canonical_document_hash(&fork)? == canonical_document_hash(&document)? {
        bail!("Save As fork did not create an independent document");
    }
    Ok(canonical_document_hash(&document)?.to_string())
}

fn join(
    link: &mut fault_transport::GuestLink,
) -> Result<(GuestSessionCore, jian_ops_schema::PenDocument)> {
    if link.epoch != fixtures::EPOCH {
        bail!("new fault-matrix join used an unexpected epoch");
    }
    let welcome = take_welcome(link.connection.receive_frame()?)?;
    let mut core = GuestSessionCore::new(
        fixtures::session_id(),
        link.epoch,
        welcome,
        GuestSessionConfig::default(),
    )?;
    let mut document = crate::scenario::initial_document()?;
    let effects = core.accept_frame(link.connection.receive_frame()?)?;
    drive_effects(
        &mut core,
        &mut document,
        &mut link.connection,
        link.epoch,
        effects,
    )?;
    require_converged(&core, &document, CommitSeq(0))?;
    Ok((core, document))
}

fn take_welcome(frame: FrameEnvelope) -> Result<Welcome> {
    match frame.into_body() {
        CollabMessage::Welcome(welcome) => Ok(welcome),
        _ => bail!("guest expected Welcome"),
    }
}

fn take_submit(effect: GuestEffect) -> Result<op_collab::Submit> {
    match effect {
        GuestEffect::Send(CollabMessage::Submit(submit)) => Ok(submit),
        _ => bail!("guest local edit did not emit Submit"),
    }
}

fn resume_hint(core: &GuestSessionCore) -> ResumeHint {
    ResumeHint {
        participant_id: core.participant_id().clone(),
        peer_id: core.peer_id().clone(),
        peer_namespace: core.peer_namespace().clone(),
        role: core.role(),
    }
}

fn require_transport_closed(connection: &mut SecureConnection<TcpStream>) -> Result<()> {
    match connection.receive_frame() {
        Err(
            op_collab_transport::RuntimeError::Io(_)
            | op_collab_transport::RuntimeError::Record(op_collab_transport::RecordError::Io(_))
            | op_collab_transport::RuntimeError::ConnectionClosed,
        ) => Ok(()),
        Err(error) => Err(error).context("unexpected transport failure while simulating link loss"),
        Ok(_) => bail!("fault injection expected the TCP connection to close"),
    }
}

fn drive_effects(
    core: &mut GuestSessionCore,
    document: &mut jian_ops_schema::PenDocument,
    connection: &mut SecureConnection<TcpStream>,
    epoch: op_collab::Epoch,
    effects: Vec<GuestEffect>,
) -> Result<GuestSignals> {
    drive_effects_with_applied_fault(core, document, connection, epoch, effects, false)
}

fn drive_effects_with_applied_fault(
    core: &mut GuestSessionCore,
    document: &mut jian_ops_schema::PenDocument,
    connection: &mut SecureConnection<TcpStream>,
    epoch: op_collab::Epoch,
    effects: Vec<GuestEffect>,
    suppress_applied: bool,
) -> Result<GuestSignals> {
    let mut pending = VecDeque::from(effects);
    let mut signals = GuestSignals::default();
    while let Some(effect) = pending.pop_front() {
        match effect {
            GuestEffect::Send(CollabMessage::Applied(_)) if suppress_applied => {
                signals.suppressed_applied += 1;
            }
            GuestEffect::Send(message) => fault_transport::send(connection, epoch, message)?,
            GuestEffect::PrepareInstall(mut prepared) => {
                *document = prepared
                    .take_candidate_document()
                    .context("guest install candidate")?;
                let installed_hash = canonical_document_hash(document)?;
                pending.extend(core.finalize_install(*prepared, installed_hash)?);
            }
            GuestEffect::PendingCancelled { .. } => signals.pending_cancelled += 1,
            GuestEffect::SessionEnded { reason } => signals.session_ended = Some(reason),
            GuestEffect::ParticipantJoined(_)
            | GuestEffect::ParticipantLeft(_)
            | GuestEffect::PresenceChanged(_)
            | GuestEffect::UndoResult(_) => {}
            GuestEffect::VerifyRenewal { .. } => {
                bail!("unexpected ticket renewal in fault-matrix smoke")
            }
        }
    }
    Ok(signals)
}

fn require_converged(
    core: &GuestSessionCore,
    document: &jian_ops_schema::PenDocument,
    expected_seq: CommitSeq,
) -> Result<()> {
    let hash = canonical_document_hash(document)?;
    if core.confirmed_seq() != Some(expected_seq)
        || core.confirmed_hash() != Some(hash)
        || core.confirmed_document().is_none()
        || canonical_document_hash(core.confirmed_document().expect("checked"))? != hash
    {
        bail!("guest did not converge at sequence {}", expected_seq.0);
    }
    Ok(())
}
