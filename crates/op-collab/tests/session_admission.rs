use jian_ops_schema::PenDocument;
use op_collab::{
    canonical_document_hash, AdmissionGrant, CollabMessage, CommitSeq, ConnectionKey,
    ConnectionPrincipal, Epoch, FrameEnvelope, OpaqueTicket, OwnerEffect, OwnerSessionConfig,
    OwnerSessionCore, ParticipantId, PeerId, PeerNamespace, RenewTicket, Role, SessionError,
    SessionId, VerifiedAuthMetadata,
};

const SESSION: &str = "session-a";
const EPOCH: u64 = 7;
const OWNER_PARTICIPANT: &str = "participant-owner";
const OWNER_PEER: &str = "peer-owner";
const OWNER_NAMESPACE: &str = "owner-ns";
const EDITOR_PARTICIPANT: &str = "participant-editor";
const EDITOR_PEER: &str = "peer-editor";
const EDITOR_NAMESPACE: &str = "editor-ns";
const VIEWER_PARTICIPANT: &str = "participant-viewer";
const VIEWER_PEER: &str = "peer-viewer";
const VIEWER_NAMESPACE: &str = "viewer-ns";
const INITIAL_EXPIRY: u64 = 100;

fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).expect("non-zero test connection")
}

fn document() -> PenDocument {
    serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap()
}

fn verified(peer_id: &str, expires_at_unix_ms: u64) -> VerifiedAuthMetadata {
    VerifiedAuthMetadata {
        issuer: "https://issuer.example".into(),
        subject: format!("subject-{peer_id}"),
        device_id: format!("device-{peer_id}"),
        proof_binding: format!("proof-{peer_id}"),
        expires_at_unix_ms,
        display_name: None,
        avatar_url: None,
    }
}

fn grant(role: Role, participant_id: &str, peer_id: &str, namespace: &str) -> AdmissionGrant {
    AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            verified(peer_id, INITIAL_EXPIRY),
            ParticipantId::from(participant_id),
            PeerId::from(peer_id),
            role,
        ),
        PeerNamespace::try_from(namespace).unwrap(),
    )
}

fn setup_peer(role: Role) -> (OwnerSessionCore, PenDocument) {
    let document = document();
    let mut core = OwnerSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        CommitSeq(0),
        connection(1),
        grant(Role::Owner, OWNER_PARTICIPANT, OWNER_PEER, OWNER_NAMESPACE),
        &document,
        OwnerSessionConfig::default(),
    )
    .unwrap();
    let (participant, peer, namespace) = match role {
        Role::Editor => (EDITOR_PARTICIPANT, EDITOR_PEER, EDITOR_NAMESPACE),
        Role::Viewer => (VIEWER_PARTICIPANT, VIEWER_PEER, VIEWER_NAMESPACE),
        Role::Owner => panic!("owner is installed by OwnerSessionCore::new"),
    };
    core.activate_peer(
        connection(2),
        grant(role, participant, peer, namespace),
        &document,
    )
    .unwrap();
    (core, document)
}

fn frame(message: CollabMessage) -> FrameEnvelope {
    FrameEnvelope::new(SessionId::from(SESSION), Epoch(EPOCH), message)
}

#[test]
fn same_epoch_resume_requires_retained_identity_namespace_and_role() {
    let (mut core, document) = setup_peer(Role::Viewer);
    let before = canonical_document_hash(&document).unwrap();
    core.disconnect(connection(2)).unwrap();

    assert!(matches!(
        core.resume_peer(
            connection(3),
            grant(
                Role::Editor,
                VIEWER_PARTICIPANT,
                VIEWER_PEER,
                VIEWER_NAMESPACE,
            ),
        ),
        Err(SessionError::ResumeBindingMismatch)
    ));

    let mut changed_identity = verified(VIEWER_PEER, INITIAL_EXPIRY + 1);
    changed_identity.subject = "different-subject".into();
    let changed_identity_grant = AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            changed_identity,
            ParticipantId::from(VIEWER_PARTICIPANT),
            PeerId::from(VIEWER_PEER),
            Role::Viewer,
        ),
        PeerNamespace::try_from(VIEWER_NAMESPACE).unwrap(),
    );
    assert!(matches!(
        core.resume_peer(connection(3), changed_identity_grant),
        Err(SessionError::ResumeBindingMismatch)
    ));
    assert_eq!(canonical_document_hash(&document).unwrap(), before);
    assert!(
        !core
            .peer_progress(&PeerId::from(VIEWER_PEER))
            .unwrap()
            .active
    );

    let resumed = core
        .resume_peer(
            connection(3),
            grant(
                Role::Viewer,
                VIEWER_PARTICIPANT,
                VIEWER_PEER,
                VIEWER_NAMESPACE,
            ),
        )
        .unwrap();
    assert_eq!(resumed.welcome.role, Role::Viewer);
    assert!(resumed.snapshot.is_none());
}

#[test]
fn renewal_requires_the_same_auth_binding_and_a_later_expiry() {
    let (mut core, document) = setup_peer(Role::Editor);
    let ticket = OpaqueTicket::new("renewal-ticket".into()).unwrap();
    let effects = core
        .accept_frame(
            connection(2),
            frame(CollabMessage::RenewTicket(RenewTicket {
                opaque_ticket: ticket,
            })),
            &document,
        )
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [OwnerEffect::VerifyRenewal { connection: target, ticket }]
            if *target == connection(2) && ticket.expose() == "renewal-ticket"
    ));

    core.complete_renewal(connection(2), verified(EDITOR_PEER, INITIAL_EXPIRY + 1))
        .unwrap();

    assert!(matches!(
        core.complete_renewal(connection(2), verified(EDITOR_PEER, INITIAL_EXPIRY + 1)),
        Err(SessionError::RenewalDidNotExtend)
    ));
    let mut changed_binding = verified(EDITOR_PEER, INITIAL_EXPIRY + 2);
    changed_binding.proof_binding = "different-proof".into();
    assert!(matches!(
        core.complete_renewal(connection(2), changed_binding),
        Err(SessionError::RenewalBindingMismatch)
    ));
}
