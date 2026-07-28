use super::*;

struct FixtureVerifier {
    claims: VerifiedTicketClaims,
    fail: bool,
}

impl TicketVerifier for FixtureVerifier {
    fn verify(
        &self,
        _opaque_ticket: &[u8],
        _expected_dh_pub_x25519: &[u8; 32],
        _now_unix_ms: u64,
    ) -> Result<VerifiedTicketClaims, AdmissionError> {
        if self.fail {
            Err(AdmissionError::Verification)
        } else {
            Ok(self.claims.clone())
        }
    }
}

fn claims(static_key: [u8; 32], expiry: u64) -> VerifiedTicketClaims {
    VerifiedTicketClaims::new(
        "https://issuer.example".to_owned(),
        "00000000-0000-0000-0000-000000000001".to_owned(),
        "00000000-0000-0000-0000-000000000002".to_owned(),
        static_key,
        expiry,
    )
    .unwrap()
}

fn claims_with_profile(static_key: [u8; 32], expiry: u64) -> VerifiedTicketClaims {
    VerifiedTicketClaims::new_with_profile(
        "https://issuer.example".to_owned(),
        "00000000-0000-0000-0000-000000000001".to_owned(),
        "00000000-0000-0000-0000-000000000002".to_owned(),
        static_key,
        expiry,
        Some("Kay 沈".to_owned()),
        Some("https://cdn.example/avatar.png?size=128".to_owned()),
    )
    .unwrap()
}

#[test]
fn admission_hello_round_trips_new_and_resume_without_logging_ticket() {
    let new = AdmissionHello::new(b"secret-ticket".to_vec(), JoinIntent::New).unwrap();
    let decoded = AdmissionHello::decode(&new.encode().unwrap()).unwrap();
    assert_eq!(decoded.ticket(), b"secret-ticket");
    assert_eq!(decoded.intent(), &JoinIntent::New);
    assert!(!format!("{new:?}").contains("secret-ticket"));

    let resume = AdmissionHello::new(
        b"renewed".to_vec(),
        JoinIntent::Resume(ResumeHint {
            participant_id: ParticipantId::from("participant"),
            peer_id: PeerId::from("peer"),
            peer_namespace: PeerNamespace::try_from("abcdef0123456789").unwrap(),
            role: Role::Editor,
        }),
    )
    .unwrap();
    let decoded = AdmissionHello::decode(&resume.encode().unwrap()).unwrap();
    assert_eq!(decoded.intent(), resume.intent());
}

#[test]
fn initial_ticket_binds_identity_profile_expiry_and_remote_static() {
    let static_key = [7_u8; 32];
    let verifier = FixtureVerifier {
        claims: claims_with_profile(static_key, 2_000),
        fail: false,
    };
    let identity = verify_initial_ticket(
        &verifier,
        b"ticket",
        &static_key,
        "https://issuer.example",
        "00000000-0000-0000-0000-000000000001",
        1_000,
    )
    .unwrap();
    assert_eq!(identity.remote_static(), &static_key);
    assert_eq!(identity.claims().display_name(), Some("Kay 沈"));
    assert_eq!(
        identity.claims().avatar_url(),
        Some("https://cdn.example/avatar.png?size=128")
    );
    let metadata = identity.to_auth_metadata();
    assert_eq!(metadata.proof_binding, URL_SAFE_NO_PAD.encode(static_key));
    assert_eq!(metadata.display_name.as_deref(), Some("Kay 沈"));
    assert_eq!(
        metadata.avatar_url.as_deref(),
        Some("https://cdn.example/avatar.png?size=128")
    );

    let debug = format!("{identity:?}");
    assert!(!debug.contains("Kay"));
    assert!(!debug.contains("avatar.png"));

    assert!(matches!(
        verify_initial_ticket(
            &verifier,
            b"ticket",
            &[8_u8; 32],
            "https://issuer.example",
            "00000000-0000-0000-0000-000000000001",
            1_000
        ),
        Err(AdmissionError::StaticKeyMismatch)
    ));
    assert!(matches!(
        verify_initial_ticket(
            &verifier,
            b"ticket",
            &static_key,
            "https://issuer.example",
            "other",
            1_000
        ),
        Err(AdmissionError::WrongSubject)
    ));
}

#[test]
fn profile_constructor_rejects_unbounded_or_non_https_values() {
    let static_key = [7_u8; 32];
    let build = |display_name, avatar_url| {
        VerifiedTicketClaims::new_with_profile(
            "https://issuer.example".to_owned(),
            "00000000-0000-0000-0000-000000000001".to_owned(),
            "00000000-0000-0000-0000-000000000002".to_owned(),
            static_key,
            2_000,
            display_name,
            avatar_url,
        )
    };
    assert!(build(Some(" leading".to_owned()), None).is_err());
    assert!(build(
        Some("x".repeat(MAX_COLLAB_PROFILE_DISPLAY_NAME_CHARS + 1)),
        None
    )
    .is_err());
    assert!(build(None, Some("http://cdn.example/avatar.png".to_owned())).is_err());
    assert!(build(None, Some("https://user@cdn.example/avatar.png".to_owned())).is_err());
}

#[test]
fn renewal_requires_same_identity_and_strictly_later_expiry() {
    let static_key = [4_u8; 32];
    let original_verifier = FixtureVerifier {
        claims: claims(static_key, 2_000),
        fail: false,
    };
    let original = verify_initial_ticket(
        &original_verifier,
        b"ticket",
        &static_key,
        "https://issuer.example",
        "00000000-0000-0000-0000-000000000001",
        1_000,
    )
    .unwrap();
    let renewed = FixtureVerifier {
        claims: claims_with_profile(static_key, 3_000),
        fail: false,
    };
    let renewed = verify_renewal_ticket(&renewed, b"renewed", &original, 1_500).unwrap();
    assert_eq!(renewed.claims().expires_at_unix_ms(), 3_000);
    assert_eq!(renewed.claims().display_name(), Some("Kay 沈"));

    let not_extended = FixtureVerifier {
        claims: claims(static_key, 2_000),
        fail: false,
    };
    assert!(matches!(
        verify_renewal_ticket(&not_extended, b"same", &original, 1_500),
        Err(AdmissionError::RenewalDidNotExtend)
    ));

    let changed = FixtureVerifier {
        claims: VerifiedTicketClaims::new(
            "https://issuer.example".into(),
            "00000000-0000-0000-0000-000000000001".into(),
            "00000000-0000-0000-0000-000000000003".into(),
            static_key,
            4_000,
        )
        .unwrap(),
        fail: false,
    };
    assert!(matches!(
        verify_renewal_ticket(&changed, b"changed", &original, 1_500),
        Err(AdmissionError::RenewalIdentityChanged)
    ));
}

#[test]
fn ticket_bounds_are_enforced_before_encoding() {
    assert!(AdmissionHello::new(Vec::new(), JoinIntent::New).is_err());
    assert!(AdmissionHello::new(vec![0; MAX_TICKET_BYTES + 1], JoinIntent::New).is_err());
}
