use jian_ops_schema::PenDocument;
use op_collab::{
    AdmissionGrant, CollabMessage, CommitSeq, ConnectionKey, ConnectionPrincipal, Epoch,
    FrameEnvelope, OwnerSessionConfig, OwnerSessionCore, Participant, ParticipantId, PeerId,
    PeerNamespace, ProtocolError, Role, SessionError, SessionId, VerifiedAuthMetadata,
    MAX_COLLAB_PROFILE_AVATAR_URL_BYTES, MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES,
    MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS,
};

const SESSION: &str = "profile-session";
const EPOCH: u64 = 5;

fn connection(value: u64) -> ConnectionKey {
    ConnectionKey::new(value).unwrap()
}

fn document() -> PenDocument {
    serde_json::from_str(r#"{"version":"1.0","children":[]}"#).unwrap()
}

fn auth(
    peer: &str,
    expiry: u64,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> VerifiedAuthMetadata {
    VerifiedAuthMetadata {
        issuer: "https://issuer.example".into(),
        subject: format!("subject-{peer}"),
        device_id: format!("device-{peer}"),
        proof_binding: format!("proof-{peer}"),
        expires_at_unix_ms: expiry,
        display_name: display_name.map(str::to_owned),
        avatar_url: avatar_url.map(str::to_owned),
    }
}

fn principal(
    peer: &str,
    role: Role,
    expiry: u64,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> ConnectionPrincipal {
    ConnectionPrincipal::from_verified(
        auth(peer, expiry, display_name, avatar_url),
        ParticipantId::from(format!("participant-{peer}")),
        PeerId::from(peer),
        role,
    )
}

fn grant(
    peer: &str,
    role: Role,
    expiry: u64,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> AdmissionGrant {
    AdmissionGrant::new(
        principal(peer, role, expiry, display_name, avatar_url),
        PeerNamespace::try_from(format!("{peer}-ns")).unwrap(),
    )
}

fn frame(participant: Participant) -> FrameEnvelope {
    FrameEnvelope::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        CollabMessage::ParticipantJoined(participant),
    )
}

#[test]
fn optional_profile_fields_round_trip_without_identity_leaks() {
    let principal = principal(
        "guest",
        Role::Editor,
        100,
        Some("Kay 沈"),
        Some("https://profiles.example/avatar.png?size=80&sig=public"),
    );
    assert_eq!(principal.display_name(), Some("Kay 沈"));
    assert_eq!(
        principal.avatar_url(),
        Some("https://profiles.example/avatar.png?size=80&sig=public")
    );

    let participant = principal.roster_participant();
    let encoded = frame(participant.clone()).to_json_vec().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(json["body"]["payload"]["displayName"], "Kay 沈");
    assert_eq!(
        json["body"]["payload"]["avatarUrl"],
        "https://profiles.example/avatar.png?size=80&sig=public"
    );
    let text = String::from_utf8(encoded.clone()).unwrap();
    assert!(!text.contains("subject-guest"));
    assert!(!text.contains("device-guest"));
    assert_eq!(
        FrameEnvelope::from_json_slice(&encoded).unwrap(),
        frame(participant.clone())
    );

    let mut legacy = json;
    let payload = legacy["body"]["payload"].as_object_mut().unwrap();
    payload.remove("displayName");
    payload.remove("avatarUrl");
    let decoded = FrameEnvelope::from_json_slice(&serde_json::to_vec(&legacy).unwrap()).unwrap();
    let CollabMessage::ParticipantJoined(decoded) = decoded.into_body() else {
        panic!("fixture remains a participant_joined frame");
    };
    assert_eq!(decoded.display_name, None);
    assert_eq!(decoded.avatar_url, None);

    let absent = frame(Participant {
        participant_id: ParticipantId::from("participant-anonymous"),
        peer_id: PeerId::from("anonymous"),
        role: Role::Viewer,
        display_name: None,
        avatar_url: None,
    });
    let absent: serde_json::Value = serde_json::from_slice(&absent.to_json_vec().unwrap()).unwrap();
    let payload = absent["body"]["payload"].as_object().unwrap();
    assert!(!payload.contains_key("displayName"));
    assert!(!payload.contains_key("avatarUrl"));

    let principal_debug = format!("{principal:?}");
    let participant_debug = format!("{participant:?}");
    for secret in [
        "subject-guest",
        "device-guest",
        "proof-guest",
        "Kay 沈",
        "profiles.example",
    ] {
        assert!(!principal_debug.contains(secret));
        assert!(!participant_debug.contains(secret));
    }
}

#[test]
fn participant_wire_rejects_unverified_identity_fields() {
    let participant = principal("guest", Role::Editor, 100, None, None).roster_participant();
    let mut json: serde_json::Value =
        serde_json::from_slice(&frame(participant).to_json_vec().unwrap()).unwrap();
    json["body"]["payload"]["subject"] = serde_json::json!("must-not-cross-wire");
    assert!(matches!(
        FrameEnvelope::from_json_slice(&serde_json::to_vec(&json).unwrap()),
        Err(ProtocolError::Decode(_))
    ));
}

#[test]
fn fixed_profile_bounds_and_https_rules_apply_on_encode_and_decode() {
    assert_eq!(MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES, 320);
    assert_eq!(MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS, 80);
    assert_eq!(MAX_COLLAB_PROFILE_AVATAR_URL_BYTES, 2_048);

    for invalid in [
        String::new(),
        " leading".into(),
        "trailing ".into(),
        "line\nbreak".into(),
        "界".repeat(MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS + 1),
        "a".repeat(MAX_COLLAB_PROFILE_DISPLAY_NAME_BYTES + 1),
    ] {
        assert_invalid_profile(Some(invalid), None, "participant.display_name", true);
    }
    for invalid in [
        String::new(),
        "http://profiles.example/avatar.png".into(),
        "https://user@profiles.example/avatar.png".into(),
        "https://profiles.example/avatar.png#fragment".into(),
        "https://profiles.example/a vatar.png".into(),
        "https://profiles.example:0/avatar.png".into(),
        "https://profiles.example/头像.png".into(),
        format!(
            "https://profiles.example/{}",
            "a".repeat(MAX_COLLAB_PROFILE_AVATAR_URL_BYTES)
        ),
    ] {
        assert_invalid_profile(None, Some(invalid), "participant.avatar_url", true);
    }

    let valid = frame(Participant {
        participant_id: ParticipantId::from("participant-valid"),
        peer_id: PeerId::from("valid"),
        role: Role::Editor,
        display_name: Some("Kay 沈".into()),
        avatar_url: Some("https://[::1]:8443/avatar.png?size=80".into()),
    });
    assert!(valid.to_json_vec().is_ok());
}

#[test]
fn admission_and_resume_publish_latest_verified_profile() {
    let document = document();
    let mut owner = OwnerSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        CommitSeq(0),
        connection(1),
        grant(
            "owner",
            Role::Owner,
            100,
            Some("Owner"),
            Some("https://profiles.example/owner.png"),
        ),
        &document,
        OwnerSessionConfig::default(),
    )
    .unwrap();

    let activated = owner
        .activate_peer(
            connection(2),
            grant(
                "guest",
                Role::Editor,
                100,
                Some("Guest Old"),
                Some("https://profiles.example/guest-old.png"),
            ),
            &document,
        )
        .unwrap();
    assert_eq!(activated.joined.display_name.as_deref(), Some("Guest Old"));
    assert_eq!(
        activated.joined.avatar_url.as_deref(),
        Some("https://profiles.example/guest-old.png")
    );
    assert_eq!(
        activated
            .welcome
            .participants
            .iter()
            .find(|participant| participant.peer_id.as_ref() == "guest")
            .and_then(|participant| participant.display_name.as_deref()),
        Some("Guest Old")
    );

    owner.disconnect(connection(2)).unwrap();
    let resumed = owner
        .resume_peer(
            connection(3),
            grant(
                "guest",
                Role::Editor,
                200,
                Some("Guest New"),
                Some("https://profiles.example/guest-new.png"),
            ),
        )
        .unwrap();
    assert_eq!(resumed.joined.display_name.as_deref(), Some("Guest New"));
    assert_eq!(
        resumed
            .welcome
            .participants
            .iter()
            .find(|participant| participant.peer_id.as_ref() == "guest")
            .and_then(|participant| participant.display_name.as_deref()),
        Some("Guest New")
    );

    owner
        .complete_renewal(
            connection(3),
            auth(
                "guest",
                300,
                Some("Guest Renewed"),
                Some("https://profiles.example/guest-renewed.png"),
            ),
        )
        .unwrap();
    let renewed = owner
        .active_participants()
        .into_iter()
        .find(|participant| participant.peer_id.as_ref() == "guest")
        .unwrap();
    assert_eq!(renewed.display_name.as_deref(), Some("Guest New"));
    assert_eq!(
        renewed.avatar_url.as_deref(),
        Some("https://profiles.example/guest-new.png")
    );
}

#[test]
fn admission_and_renewal_reject_invalid_verified_profiles() {
    let document = document();
    let invalid_owner = OwnerSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        CommitSeq(0),
        connection(1),
        grant("owner", Role::Owner, 100, Some(" leading"), None),
        &document,
        OwnerSessionConfig::default(),
    );
    assert!(matches!(
        invalid_owner,
        Err(SessionError::InvalidAdmissionProfile {
            field: "display_name"
        })
    ));

    let mut owner = OwnerSessionCore::new(
        SessionId::from(SESSION),
        Epoch(EPOCH),
        CommitSeq(0),
        connection(1),
        grant("owner", Role::Owner, 100, Some("Owner"), None),
        &document,
        OwnerSessionConfig::default(),
    )
    .unwrap();
    owner
        .activate_peer(
            connection(2),
            grant("guest", Role::Editor, 100, Some("Guest"), None),
            &document,
        )
        .unwrap();
    assert!(matches!(
        owner.complete_renewal(
            connection(2),
            auth(
                "guest",
                200,
                Some("Guest"),
                Some("http://profiles.example/avatar.png"),
            ),
        ),
        Err(SessionError::InvalidAdmissionProfile {
            field: "avatar_url"
        })
    ));
    let guest = owner
        .active_participants()
        .into_iter()
        .find(|participant| participant.peer_id.as_ref() == "guest")
        .unwrap();
    assert_eq!(guest.display_name.as_deref(), Some("Guest"));
    assert_eq!(guest.avatar_url, None);
}

fn assert_invalid_profile(
    display_name: Option<String>,
    avatar_url: Option<String>,
    field: &'static str,
    check_decode: bool,
) {
    let participant = Participant {
        participant_id: ParticipantId::from("participant-invalid"),
        peer_id: PeerId::from("invalid"),
        role: Role::Editor,
        display_name,
        avatar_url,
    };
    let raw_payload = serde_json::to_value(&participant).unwrap();
    let invalid = frame(participant);
    assert!(matches!(
        invalid.to_json_vec(),
        Err(ProtocolError::InvalidParticipantProfile { field: actual }) if actual == field
    ));
    if check_decode {
        let raw = serde_json::to_vec(&serde_json::json!({
            "protocolVersion": 1,
            "sessionId": SESSION,
            "epoch": EPOCH,
            "body": {
                "type": "participant_joined",
                "payload": raw_payload,
            }
        }))
        .unwrap();
        assert!(matches!(
            FrameEnvelope::from_json_slice(&raw),
            Err(ProtocolError::InvalidParticipantProfile { field: actual }) if actual == field
        ));
    }
}
