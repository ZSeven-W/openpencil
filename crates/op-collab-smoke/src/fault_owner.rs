use crate::auth::{SmokeAuth, OWNER_AVATAR_URL, OWNER_DEVICE_ID, OWNER_DISPLAY_NAME};
use crate::fault_transport::{self, OWNER_TICKET_ID};
use crate::fixtures;
use crate::scenario::Scenario;
use anyhow::{bail, Context, Result};
use op_collab::{
    canonical_document_hash, diff_supported, AdmissionGrant, ClientOpId, CollabMessage, Commit,
    CommitSeq, ConnectionKey, DiffContext, Epoch, FrameEnvelope, OwnerEffect, OwnerSessionConfig,
    OwnerSessionCore, PeerId, PeerNamespace, RejectCode, Role, Submit,
};
use op_collab_transport::{
    AdmissionIdentity, ConnectionLimiter, DeviceStaticKey, JoinIntent, SecureConnection,
    TransportConfig,
};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

const OWNER_CONNECTION: u64 = 1;
const GUEST_CONNECTION: u64 = 2;
const RESUMED_GUEST_CONNECTION: u64 = 3;
const RETRIED_GUEST_CONNECTION: u64 = 4;
const ACK_RECOVERY_GUEST_CONNECTION: u64 = 5;

pub fn run(scenario: Scenario, port_file: &Path) -> Result<String> {
    let listener = fault_transport::owner_listener(port_file)?;
    let owner_key = DeviceStaticKey::from_private([0x31; 32])?;
    let auth = SmokeAuth::for_device(
        &owner_key,
        OWNER_DEVICE_ID,
        OWNER_TICKET_ID,
        OWNER_DISPLAY_NAME,
        OWNER_AVATAR_URL,
    )?;
    let config = TransportConfig::default().validate()?;
    let limiter = ConnectionLimiter::new(config.connections)?;
    match scenario {
        Scenario::RetryExactlyOnce => retry_exactly_once(&listener, &limiter, &owner_key, &auth),
        Scenario::StaleRebase => stale_rebase(&listener, &limiter, &owner_key, &auth),
        Scenario::AtomicTxnFailure => atomic_txn_failure(&listener, &limiter, &owner_key, &auth),
        Scenario::ReconnectCatchUp => reconnect(&listener, &limiter, &owner_key, &auth, false),
        Scenario::ReconnectSnapshot => reconnect(&listener, &limiter, &owner_key, &auth, true),
        Scenario::EpochChange => epoch_change(&listener, &limiter, &owner_key, &auth),
        Scenario::OwnerLeft => owner_left(&listener, &limiter, &owner_key, &auth),
    }
}

fn retry_exactly_once(
    listener: &TcpListener,
    limiter: &ConnectionLimiter,
    owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_new_intent(&link.guest_intent)?;
    let (mut core, mut document) =
        activate_new(&mut link, owner_key, auth, OwnerSessionConfig::default())?;
    receive_applied(
        &mut core,
        &document,
        guest_connection(),
        &mut link.connection,
    )?;

    let first = link.connection.receive_frame()?;
    let effects = core.accept_frame(guest_connection(), first, &document)?;
    let first_commit = finalize_prepare(&mut core, &mut document, effects)?;

    mark_guest_disconnected(&mut core, guest_connection())?;
    drop(link);

    let mut resumed =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_resume_intent(&resumed.guest_intent)?;
    let activation = core.resume_peer(
        resumed_guest_connection(),
        guest_grant(&resumed.guest_identity)?,
    )?;
    if activation.welcome.seq != CommitSeq(1) || activation.snapshot.is_some() {
        bail!("lost Commit recovery changed the retained owner state");
    }
    fault_transport::send(
        &mut resumed.connection,
        fixtures::EPOCH,
        CollabMessage::Welcome(activation.welcome),
    )?;
    let catch_up = resumed.connection.receive_frame()?;
    let effects = core.accept_frame(resumed_guest_connection(), catch_up, &document)?;
    match effects.as_slice() {
        [OwnerEffect::CommitBatch { commits, .. }]
            if commits.len() == 1 && commits[0].as_ref() == &first_commit => {}
        _ => bail!("lost Commit was not recovered from the retained log"),
    }
    route_effects(&mut resumed.connection, fixtures::EPOCH, effects)?;
    receive_applied(
        &mut core,
        &document,
        resumed_guest_connection(),
        &mut resumed.connection,
    )?;
    receive_transport_disconnect(
        &mut core,
        &document,
        resumed_guest_connection(),
        &mut resumed.connection,
    )?;
    drop(resumed);

    let mut retried =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_resume_intent(&retried.guest_intent)?;
    let activation = core.resume_peer(
        retried_guest_connection(),
        guest_grant(&retried.guest_identity)?,
    )?;
    if activation.welcome.seq != CommitSeq(1) || activation.snapshot.is_some() {
        bail!("lost Submit recovery changed the retained owner state");
    }
    fault_transport::send(
        &mut retried.connection,
        fixtures::EPOCH,
        CollabMessage::Welcome(activation.welcome),
    )?;
    let replay = retried.connection.receive_frame()?;
    let replayed_submit = match replay.body() {
        CollabMessage::Submit(submit) => submit,
        _ => bail!("same-epoch resume did not resend the retained Submit"),
    };
    if replayed_submit.client_op_id.local_counter != 2 || replayed_submit.base_seq != CommitSeq(1) {
        bail!("same-epoch resume changed the retained Submit identity");
    }
    let effects = core.accept_frame(retried_guest_connection(), replay, &document)?;
    let second_commit = finalize_prepare(&mut core, &mut document, effects)?;
    fault_transport::send(
        &mut retried.connection,
        fixtures::EPOCH,
        CollabMessage::Commit(second_commit),
    )?;
    receive_transport_disconnect(
        &mut core,
        &document,
        retried_guest_connection(),
        &mut retried.connection,
    )?;
    drop(retried);

    let progress = core
        .peer_progress(&PeerId::from(fixtures::GUEST_PEER))
        .context("guest progress after lost Applied")?;
    if core.seq() != CommitSeq(2)
        || progress.next_counter != Some(3)
        || progress.retained_results != 2
        || progress.applied_through != CommitSeq(1)
        || progress.active
    {
        bail!("lost Applied changed owner sequencing or dedupe progress");
    }

    let third_desired = crate::scenario::with_name(&document, "Ack recovery owner")?;
    let third_commit = commit_owner_edit(&mut core, &mut document, &third_desired, 1)?;

    let mut acknowledged =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_resume_intent(&acknowledged.guest_intent)?;
    let activation = core.resume_peer(
        ack_recovery_guest_connection(),
        guest_grant(&acknowledged.guest_identity)?,
    )?;
    if activation.welcome.seq != CommitSeq(3) || activation.snapshot.is_some() {
        bail!("lost Applied recovery changed the retained owner state");
    }
    fault_transport::send(
        &mut acknowledged.connection,
        fixtures::EPOCH,
        CollabMessage::Welcome(activation.welcome),
    )?;
    let catch_up = acknowledged.connection.receive_frame()?;
    if !matches!(
        catch_up.body(),
        CollabMessage::CatchUp(request) if request.after_seq == CommitSeq(2)
    ) {
        bail!("lost Applied recovery requested the wrong catch-up boundary");
    }
    let effects = core.accept_frame(ack_recovery_guest_connection(), catch_up, &document)?;
    match effects.as_slice() {
        [OwnerEffect::CommitBatch { commits, .. }]
            if commits.len() == 1 && commits[0].as_ref() == &third_commit => {}
        _ => bail!("lost Applied recovery did not replay only the missing Commit"),
    }
    route_effects(&mut acknowledged.connection, fixtures::EPOCH, effects)?;
    receive_applied(
        &mut core,
        &document,
        ack_recovery_guest_connection(),
        &mut acknowledged.connection,
    )?;
    let progress = core
        .peer_progress(&PeerId::from(fixtures::GUEST_PEER))
        .context("guest progress after Applied recovery")?;
    if core.seq() != CommitSeq(3)
        || progress.next_counter != Some(3)
        || progress.retained_results != 2
        || progress.applied_through != CommitSeq(3)
        || !progress.active
    {
        bail!("Applied recovery changed exactly-once owner progress");
    }
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
    listener: &TcpListener,
    limiter: &ConnectionLimiter,
    owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_new_intent(&link.guest_intent)?;
    let (mut core, mut document) =
        activate_new(&mut link, owner_key, auth, OwnerSessionConfig::default())?;
    receive_applied(
        &mut core,
        &document,
        guest_connection(),
        &mut link.connection,
    )?;

    let owner_desired = crate::scenario::with_position(&document, 0.0, 5.0)?;
    let owner_commit = commit_owner_edit(&mut core, &mut document, &owner_desired, 1)?;
    let stale_frame = link.connection.receive_frame()?;
    let stale_effects = core.accept_frame(guest_connection(), stale_frame, &document)?;
    let reply = take_reply(stale_effects)?;
    let CollabMessage::Reject(reject) = &reply else {
        bail!("stale submission did not return Reject");
    };
    if reject.code != RejectCode::StaleBase || reject.owner_seq != CommitSeq(1) {
        bail!("stale submission returned the wrong owner verdict");
    }
    fault_transport::send(&mut link.connection, fixtures::EPOCH, reply)?;

    let catch_up = link.connection.receive_frame()?;
    if !matches!(catch_up.body(), CollabMessage::CatchUp(_)) {
        bail!("stale guest did not request catch-up");
    }
    let catch_up_effects = core.accept_frame(guest_connection(), catch_up, &document)?;
    match catch_up_effects.as_slice() {
        [OwnerEffect::CommitBatch { commits, .. }]
            if commits.len() == 1 && commits[0].as_ref() == &owner_commit => {}
        _ => bail!("stale catch-up did not replay the retained owner Commit"),
    }
    route_effects(&mut link.connection, fixtures::EPOCH, catch_up_effects)?;

    while core.seq() < CommitSeq(2) {
        let frame = link.connection.receive_frame()?;
        let effects = core.accept_frame(guest_connection(), frame, &document)?;
        if effects
            .iter()
            .any(|effect| matches!(effect, OwnerEffect::PrepareInstall(_)))
        {
            let commit = finalize_prepare(&mut core, &mut document, effects)?;
            fault_transport::send(
                &mut link.connection,
                fixtures::EPOCH,
                CollabMessage::Commit(commit),
            )?;
        } else {
            route_effects(&mut link.connection, fixtures::EPOCH, effects)?;
        }
    }
    let expected =
        crate::scenario::with_position(&crate::scenario::initial_document()?, 10.0, 5.0)?;
    if canonical_document_hash(&document)? != canonical_document_hash(&expected)? {
        bail!("stale rebase did not preserve both authors' property edits");
    }
    Ok(canonical_document_hash(&document)?.to_string())
}

fn atomic_txn_failure(
    listener: &TcpListener,
    limiter: &ConnectionLimiter,
    owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_new_intent(&link.guest_intent)?;
    let (mut core, document) =
        activate_new(&mut link, owner_key, auth, OwnerSessionConfig::default())?;
    receive_applied(
        &mut core,
        &document,
        guest_connection(),
        &mut link.connection,
    )?;
    let initial_hash = canonical_document_hash(&document)?;

    let frame = link.connection.receive_frame()?;
    let effects = core.accept_frame(guest_connection(), frame, &document)?;
    let reply = take_reply(effects)?;
    let CollabMessage::Reject(reject) = &reply else {
        bail!("invalid atomic transaction did not return Reject");
    };
    if reject.code != RejectCode::PreconditionFailed || core.seq() != CommitSeq(0) {
        bail!("invalid atomic transaction changed owner sequencing");
    }
    if canonical_document_hash(&document)? != initial_hash || document.children.len() != 1 {
        bail!("invalid atomic transaction left its valid prefix installed");
    }
    fault_transport::send(&mut link.connection, fixtures::EPOCH, reply)?;
    Ok(initial_hash.to_string())
}

fn reconnect(
    listener: &TcpListener,
    limiter: &ConnectionLimiter,
    owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
    force_log_gap: bool,
) -> Result<String> {
    let mut first =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_new_intent(&first.guest_intent)?;
    let mut session_config = OwnerSessionConfig::default();
    if force_log_gap {
        session_config.session_limits.commit_log_entries = 1;
    }
    let (mut core, mut document) = activate_new(&mut first, owner_key, auth, session_config)?;
    receive_applied(
        &mut core,
        &document,
        guest_connection(),
        &mut first.connection,
    )?;
    receive_disconnect(
        &mut core,
        &document,
        guest_connection(),
        &mut first.connection,
    )?;
    drop(first);

    let first_desired = crate::scenario::with_position(&document, 21.0, 0.0)?;
    let _ = commit_owner_edit(&mut core, &mut document, &first_desired, 1)?;
    let second_desired = crate::scenario::with_name(&document, "Offline owner final")?;
    let _ = commit_owner_edit(&mut core, &mut document, &second_desired, 2)?;

    let mut resumed =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_resume_intent(&resumed.guest_intent)?;
    let activation = core.resume_peer(
        resumed_guest_connection(),
        guest_grant(&resumed.guest_identity)?,
    )?;
    if activation.snapshot.is_some() || activation.welcome.seq != CommitSeq(2) {
        bail!("same-epoch resume changed retained owner state");
    }
    fault_transport::send(
        &mut resumed.connection,
        fixtures::EPOCH,
        CollabMessage::Welcome(activation.welcome),
    )?;

    let catch_up = resumed.connection.receive_frame()?;
    if !matches!(catch_up.body(), CollabMessage::CatchUp(_)) {
        bail!("resumed guest did not request catch-up");
    }
    let effects = core.accept_frame(resumed_guest_connection(), catch_up, &document)?;
    let is_snapshot = matches!(effects.as_slice(), [OwnerEffect::Snapshot { .. }]);
    if is_snapshot != force_log_gap {
        bail!(
            "owner recovery kind mismatch: expected snapshot={force_log_gap}, actual={is_snapshot}"
        );
    }
    route_effects(&mut resumed.connection, fixtures::EPOCH, effects)?;
    while core
        .peer_progress(&PeerId::from(fixtures::GUEST_PEER))
        .is_some_and(|progress| progress.applied_through < CommitSeq(2))
    {
        let frame = resumed.connection.receive_frame()?;
        let effects = core.accept_frame(resumed_guest_connection(), frame, &document)?;
        route_effects(&mut resumed.connection, fixtures::EPOCH, effects)?;
    }
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
    listener: &TcpListener,
    limiter: &ConnectionLimiter,
    owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut first =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_new_intent(&first.guest_intent)?;
    let (mut old_core, old_document) =
        activate_new(&mut first, owner_key, auth, OwnerSessionConfig::default())?;
    receive_applied(
        &mut old_core,
        &old_document,
        guest_connection(),
        &mut first.connection,
    )?;
    receive_disconnect(
        &mut old_core,
        &old_document,
        guest_connection(),
        &mut first.connection,
    )?;
    drop(first);

    let next_epoch = Epoch(fixtures::EPOCH.0 + 1);
    let mut replacement =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, next_epoch)?;
    require_new_intent(&replacement.guest_intent)?;
    let document = crate::scenario::replacement_epoch_document()?;
    let (mut new_core, document) = activate_new_at_epoch(
        &mut replacement,
        owner_key,
        auth,
        OwnerSessionConfig::default(),
        next_epoch,
        document,
    )?;
    receive_applied(
        &mut new_core,
        &document,
        guest_connection(),
        &mut replacement.connection,
    )?;
    if new_core.seq() != CommitSeq(0) {
        bail!("old pending edit advanced the replacement owner sequence");
    }
    Ok(canonical_document_hash(&document)?.to_string())
}

fn owner_left(
    listener: &TcpListener,
    limiter: &ConnectionLimiter,
    owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
) -> Result<String> {
    let mut link =
        fault_transport::accept_guest(listener, limiter, owner_key, auth, fixtures::EPOCH)?;
    require_new_intent(&link.guest_intent)?;
    let (mut core, document) =
        activate_new(&mut link, owner_key, auth, OwnerSessionConfig::default())?;
    receive_applied(
        &mut core,
        &document,
        guest_connection(),
        &mut link.connection,
    )?;
    let effects = core.disconnect(owner_connection())?;
    if !matches!(
        effects.as_slice(),
        [OwnerEffect::Broadcast {
            message: CollabMessage::Bye(op_collab::Bye {
                reason: op_collab::ByeReason::OwnerLeft
            })
        }]
    ) {
        bail!("owner disconnect did not emit OwnerLeft");
    }
    route_effects(&mut link.connection, fixtures::EPOCH, effects)?;
    Ok(canonical_document_hash(&document)?.to_string())
}

fn activate_new(
    link: &mut fault_transport::OwnerLink,
    owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
    config: OwnerSessionConfig,
) -> Result<(OwnerSessionCore, jian_ops_schema::PenDocument)> {
    activate_new_at_epoch(
        link,
        owner_key,
        auth,
        config,
        fixtures::EPOCH,
        crate::scenario::initial_document()?,
    )
}

fn activate_new_at_epoch(
    link: &mut fault_transport::OwnerLink,
    _owner_key: &DeviceStaticKey,
    auth: &SmokeAuth,
    config: OwnerSessionConfig,
    epoch: Epoch,
    document: jian_ops_schema::PenDocument,
) -> Result<(OwnerSessionCore, jian_ops_schema::PenDocument)> {
    let owner_grant = fixtures::grant(
        auth.local_auth().clone(),
        Role::Owner,
        fixtures::OWNER_PARTICIPANT,
        fixtures::OWNER_PEER,
        fixtures::OWNER_NAMESPACE,
    )?;
    let mut core = OwnerSessionCore::new(
        fixtures::session_id(),
        epoch,
        CommitSeq(0),
        owner_connection(),
        owner_grant,
        &document,
        config,
    )?;
    let activation = core.activate_peer(
        guest_connection(),
        guest_grant(&link.guest_identity)?,
        &document,
    )?;
    fault_transport::send(
        &mut link.connection,
        epoch,
        CollabMessage::Welcome(activation.welcome),
    )?;
    let snapshot = activation.snapshot.context("new guest receives snapshot")?;
    fault_transport::send(
        &mut link.connection,
        epoch,
        CollabMessage::Snapshot(Box::new(snapshot)),
    )?;
    Ok((core, document))
}

fn guest_grant(identity: &AdmissionIdentity) -> Result<AdmissionGrant> {
    fixtures::grant(
        identity.to_auth_metadata(),
        Role::Editor,
        fixtures::GUEST_PARTICIPANT,
        fixtures::GUEST_PEER,
        fixtures::GUEST_NAMESPACE,
    )
}

fn commit_owner_edit(
    core: &mut OwnerSessionCore,
    document: &mut jian_ops_schema::PenDocument,
    desired: &jian_ops_schema::PenDocument,
    counter: u64,
) -> Result<Commit> {
    let supported = diff_supported(
        document,
        desired,
        &DiffContext::new(
            PeerNamespace::try_from(fixtures::OWNER_NAMESPACE)?,
            Role::Owner,
            Some(0),
        ),
    )?;
    let submit = Submit {
        client_op_id: ClientOpId {
            peer_id: PeerId::from(fixtures::OWNER_PEER),
            local_counter: counter,
        },
        base_seq: core.seq(),
        txn: supported.txn,
    };
    let effects = core.accept_frame(
        owner_connection(),
        FrameEnvelope::new(
            fixtures::session_id(),
            core.epoch(),
            CollabMessage::Submit(submit),
        ),
        document,
    )?;
    finalize_prepare(core, document, effects)
}

fn finalize_prepare(
    core: &mut OwnerSessionCore,
    document: &mut jian_ops_schema::PenDocument,
    effects: Vec<OwnerEffect>,
) -> Result<Commit> {
    match effects.as_slice() {
        [OwnerEffect::PrepareInstall(_)] => {}
        _ => bail!("owner did not produce one atomic install candidate"),
    }
    let OwnerEffect::PrepareInstall(prepared) = effects.into_iter().next().expect("checked") else {
        unreachable!("checked PrepareInstall")
    };
    let mut prepared = *prepared;
    *document = prepared
        .take_candidate_document()
        .context("owner install candidate")?;
    let installed_hash = canonical_document_hash(document)?;
    let effect = core.finalize_install(prepared, installed_hash)?;
    match effect {
        OwnerEffect::BroadcastCommit { commit } => Ok(commit.as_ref().clone()),
        OwnerEffect::Broadcast {
            message: CollabMessage::Commit(commit),
        } => Ok(commit),
        _ => bail!("owner finalization did not produce Commit"),
    }
}

fn receive_applied(
    core: &mut OwnerSessionCore,
    document: &jian_ops_schema::PenDocument,
    connection_key: ConnectionKey,
    connection: &mut SecureConnection<TcpStream>,
) -> Result<()> {
    let frame = connection.receive_frame()?;
    if !matches!(frame.body(), CollabMessage::Applied(_)) {
        bail!("owner expected guest Applied acknowledgement");
    }
    let effects = core.accept_frame(connection_key, frame, document)?;
    if !effects.is_empty() {
        bail!("Applied acknowledgement unexpectedly produced owner effects");
    }
    Ok(())
}

fn receive_disconnect(
    core: &mut OwnerSessionCore,
    document: &jian_ops_schema::PenDocument,
    connection_key: ConnectionKey,
    connection: &mut SecureConnection<TcpStream>,
) -> Result<()> {
    let frame = connection.receive_frame()?;
    if !matches!(frame.body(), CollabMessage::Bye(_)) {
        bail!("owner expected guest disconnect");
    }
    let effects = core.accept_frame(connection_key, frame, document)?;
    if !matches!(
        effects.as_slice(),
        [OwnerEffect::Broadcast {
            message: CollabMessage::ParticipantLeft(_)
        }]
    ) {
        bail!("guest disconnect did not retain resumable roster state");
    }
    Ok(())
}

fn receive_transport_disconnect(
    core: &mut OwnerSessionCore,
    document: &jian_ops_schema::PenDocument,
    connection_key: ConnectionKey,
    connection: &mut SecureConnection<TcpStream>,
) -> Result<()> {
    match connection.receive_frame() {
        Err(
            op_collab_transport::RuntimeError::Io(_)
            | op_collab_transport::RuntimeError::Record(op_collab_transport::RecordError::Io(_))
            | op_collab_transport::RuntimeError::ConnectionClosed,
        ) => mark_guest_disconnected(core, connection_key),
        Err(error) => Err(error).context("unexpected transport failure while awaiting link loss"),
        Ok(frame) => {
            let _ = core.accept_frame(connection_key, frame, document)?;
            bail!("fault injection expected the TCP connection to close")
        }
    }
}

fn mark_guest_disconnected(
    core: &mut OwnerSessionCore,
    connection_key: ConnectionKey,
) -> Result<()> {
    let effects = core.disconnect(connection_key)?;
    if !matches!(
        effects.as_slice(),
        [OwnerEffect::Broadcast {
            message: CollabMessage::ParticipantLeft(_)
        }]
    ) {
        bail!("transport disconnect did not retain resumable roster state");
    }
    Ok(())
}

fn route_effects(
    connection: &mut SecureConnection<TcpStream>,
    epoch: Epoch,
    effects: Vec<OwnerEffect>,
) -> Result<()> {
    for effect in effects {
        match effect {
            OwnerEffect::Reply { message, .. } | OwnerEffect::Broadcast { message } => {
                fault_transport::send(connection, epoch, message)?;
            }
            OwnerEffect::ReplyCommit { commit, .. } | OwnerEffect::BroadcastCommit { commit } => {
                fault_transport::send_commit(connection, epoch, &commit)?;
            }
            OwnerEffect::CommitBatch { commits, .. } => {
                for commit in commits {
                    fault_transport::send_commit(connection, epoch, &commit)?;
                }
            }
            OwnerEffect::Snapshot { snapshot, .. } => {
                fault_transport::send(connection, epoch, CollabMessage::Snapshot(snapshot))?;
            }
            OwnerEffect::Close { reason, .. } => {
                fault_transport::send(
                    connection,
                    epoch,
                    CollabMessage::Bye(op_collab::Bye { reason }),
                )?;
            }
            OwnerEffect::UndoCommitted { result, commit, .. } => {
                fault_transport::send(connection, epoch, CollabMessage::UndoResult(result))?;
                fault_transport::send_commit(connection, epoch, &commit)?;
            }
            OwnerEffect::PrepareInstall(_)
            | OwnerEffect::VerifyRenewal { .. }
            | OwnerEffect::UndoRequested(_) => {
                bail!("unexpected owner effect in fault-matrix routing")
            }
        }
    }
    Ok(())
}

fn take_reply(effects: Vec<OwnerEffect>) -> Result<CollabMessage> {
    match effects.as_slice() {
        [OwnerEffect::Reply { .. }] => {}
        _ => bail!("owner did not produce one direct reply"),
    }
    match effects.into_iter().next().expect("checked") {
        OwnerEffect::Reply { message, .. } => Ok(message),
        _ => unreachable!("checked Reply"),
    }
}

fn require_new_intent(intent: &JoinIntent) -> Result<()> {
    if matches!(intent, JoinIntent::New) {
        Ok(())
    } else {
        bail!("new guest advertised a resume intent")
    }
}

fn require_resume_intent(intent: &JoinIntent) -> Result<()> {
    let JoinIntent::Resume(hint) = intent else {
        bail!("same-epoch guest did not advertise resume intent");
    };
    if hint.participant_id.as_ref() != fixtures::GUEST_PARTICIPANT
        || hint.peer_id.as_ref() != fixtures::GUEST_PEER
        || hint.peer_namespace.as_str() != fixtures::GUEST_NAMESPACE
        || hint.role != Role::Editor
    {
        bail!("same-epoch resume hint changed retained binding");
    }
    Ok(())
}

fn owner_connection() -> ConnectionKey {
    ConnectionKey::new(OWNER_CONNECTION).expect("non-zero owner connection")
}

fn guest_connection() -> ConnectionKey {
    ConnectionKey::new(GUEST_CONNECTION).expect("non-zero guest connection")
}

fn resumed_guest_connection() -> ConnectionKey {
    ConnectionKey::new(RESUMED_GUEST_CONNECTION).expect("non-zero resumed guest connection")
}

fn retried_guest_connection() -> ConnectionKey {
    ConnectionKey::new(RETRIED_GUEST_CONNECTION).expect("non-zero retried guest connection")
}

fn ack_recovery_guest_connection() -> ConnectionKey {
    ConnectionKey::new(ACK_RECOVERY_GUEST_CONNECTION)
        .expect("non-zero Applied-recovery guest connection")
}
