use jian_ops_schema::PenDocument;
use op_collab::{
    AdmissionGrant, ClientOpId, CollabMessage, CommitSeq, ConnectionKey, ConnectionPrincipal,
    Epoch, FrameEnvelope, OwnerSessionConfig, OwnerSessionCore, ParticipantId, PeerId,
    PeerNamespace, Presence, ProtocolError, Role, SessionError, SessionId, UndoRequest,
    UndoRequestId, VerifiedAuthMetadata, WireLimits,
};

fn connection(raw: u64) -> ConnectionKey {
    ConnectionKey::new(raw).unwrap()
}

fn repeated_id(byte: char) -> String {
    byte.to_string().repeat(1_024)
}

fn grant(role: Role, participant: char, peer: char, namespace: &str) -> AdmissionGrant {
    AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            VerifiedAuthMetadata {
                issuer: "test-issuer".into(),
                subject: format!("subject-{peer}"),
                device_id: format!("device-{peer}"),
                proof_binding: format!("binding-{peer}"),
                expires_at_unix_ms: 10_000,
                display_name: None,
                avatar_url: None,
            },
            ParticipantId::from(repeated_id(participant)),
            PeerId::from(repeated_id(peer)),
            role,
        ),
        PeerNamespace::try_from(namespace).unwrap(),
    )
}

fn large_presence() -> Presence {
    Presence {
        cursor: None,
        selection: vec!["s".repeat(1_000); 20],
        viewport: None,
        editing_node: None,
    }
}

fn short_grant(role: Role, participant: &str, peer: &str, namespace: &str) -> AdmissionGrant {
    AdmissionGrant::new(
        ConnectionPrincipal::from_verified(
            VerifiedAuthMetadata {
                issuer: "test-issuer".into(),
                subject: format!("subject-{peer}"),
                device_id: format!("device-{peer}"),
                proof_binding: format!("binding-{peer}"),
                expires_at_unix_ms: 10_000,
                display_name: None,
                avatar_url: None,
            },
            ParticipantId::from(participant),
            PeerId::from(peer),
            role,
        ),
        PeerNamespace::try_from(namespace).unwrap(),
    )
}

#[test]
fn presence_payload_limit_applies_to_encode_and_decode() {
    let frame = FrameEnvelope::new(
        SessionId::from("session"),
        Epoch(1),
        CollabMessage::PresenceUpdate(large_presence()),
    );
    let encoded = frame.to_json_vec().unwrap();
    let limits = WireLimits {
        max_presence_bytes: 32,
        ..WireLimits::default()
    };

    assert!(matches!(
        frame.to_json_vec_with_limits(limits),
        Err(ProtocolError::PresenceTooLarge { .. })
    ));
    assert!(matches!(
        FrameEnvelope::from_json_slice_with_limits(&encoded, limits),
        Err(ProtocolError::PresenceTooLarge { .. })
    ));
}

#[test]
fn identity_injection_cannot_emit_an_oversized_presence_broadcast() {
    let document: PenDocument = serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap();
    let inbound = FrameEnvelope::new(
        SessionId::from("s"),
        Epoch(1),
        CollabMessage::PresenceUpdate(large_presence()),
    );
    let inbound_size = inbound.to_json_vec().unwrap().len();
    let mut config = OwnerSessionConfig::default();
    config.wire_limits.max_envelope_bytes = u32::try_from(inbound_size).unwrap();
    let mut core = OwnerSessionCore::new(
        SessionId::from("s"),
        Epoch(1),
        CommitSeq(0),
        connection(1),
        grant(Role::Owner, 'a', 'o', "owner"),
        &document,
        config,
    )
    .unwrap();
    core.activate_peer(
        connection(2),
        grant(Role::Editor, 'b', 'e', "editor"),
        &document,
    )
    .unwrap();

    assert!(matches!(
        core.accept_frame(connection(2), inbound, &document),
        Err(SessionError::InvalidFrame(
            ProtocolError::EnvelopeTooLarge { .. }
        ))
    ));
}

#[test]
fn viewer_undo_rejection_is_checked_after_adding_owner_fields() {
    let document: PenDocument = serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap();
    let inbound = FrameEnvelope::new(
        SessionId::from("s"),
        Epoch(1),
        CollabMessage::UndoRequest(UndoRequest {
            request_id: UndoRequestId {
                peer_id: PeerId::from("viewer"),
                local_counter: 1,
            },
            target_client_op_id: ClientOpId {
                peer_id: PeerId::from("t".repeat(1_024)),
                local_counter: 1,
            },
        }),
    );
    let inbound_size = inbound.to_json_vec().unwrap().len();
    let mut config = OwnerSessionConfig::default();
    config.wire_limits.max_envelope_bytes = u32::try_from(inbound_size).unwrap();
    let mut core = OwnerSessionCore::new(
        SessionId::from("s"),
        Epoch(1),
        CommitSeq(0),
        connection(1),
        short_grant(Role::Owner, "owner-participant", "owner", "owner"),
        &document,
        config,
    )
    .unwrap();
    core.activate_peer(
        connection(2),
        short_grant(Role::Viewer, "viewer-participant", "viewer", "viewer"),
        &document,
    )
    .unwrap();

    assert!(matches!(
        core.accept_frame(connection(2), inbound, &document),
        Err(SessionError::InvalidFrame(
            ProtocolError::EnvelopeTooLarge { .. }
        ))
    ));
}
