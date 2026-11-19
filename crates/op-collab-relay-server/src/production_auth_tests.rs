#![cfg(test)]

use std::{
    fs,
    num::NonZeroU64,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signer as _, SigningKey};
use op_auth_bridge::{
    CollabJwksCacheLimits, CollabJwksFetchError, CollabJwksFetchRequest, CollabJwksFetchResponse,
    CollabJwksFetcher, CollabTicketVerifier, OpaqueCollabTicket, StaticTestJwksFetcher,
    TestCollabIssuer, TestCollabTicketSpec,
};
use op_collab_relay_protocol::{
    CallerDeviceDhPublic, ExpectedDiscoveryId, LocatorKeyId, LocatorSignature, OwnerNoiseStatic,
    RelayAuthExtensionV1, RelayChallengeKeyId, RelayChallengeProofV2, RelayClientHello,
    RelayHelloAuthMode, RelayLocatorVerifier, RelayRegion, RelayRejectCode, RelayRole,
    RelayServerChallengeV1, RouteCapability, RouteId, UnsignedRelayLocatorV1, VerifiedRelayRoute,
};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crate::{
    auth::{RelayBearerCredential, RelayUpgradeChallenge},
    CollabTicketRelayAuthenticator, PinnedEd25519LocatorVerifier, PinnedPolicyFileFetcher,
    PinnedX25519ProofBoundary, RelayAuthenticator, RelayServerX25519ProofBoundary,
    RelayX25519BoundaryError,
};

const CLIENT_A_SECRET: [u8; 32] = [0x31; 32];
const CLIENT_B_SECRET: [u8; 32] = [0x32; 32];
const SERVER_SECRET: [u8; 32] = [0x61; 32];
const OTHER_SUBJECT: &str = "223e4567-e89b-12d3-a456-426614174000";
const TICKET_ID_A: &str = "323e4567-e89b-12d3-a456-426614174001";
const TICKET_ID_B: &str = "323e4567-e89b-12d3-a456-426614174002";

struct StrictTestLocatorVerifier;

impl RelayLocatorVerifier for StrictTestLocatorVerifier {
    fn verify(
        &self,
        key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        signature: &[u8; 64],
    ) -> bool {
        key_id.as_str() == "test-locator-key" && signature == &[0x55; 64]
    }
}

struct ConstructionOnlyLocatorVerifier;

impl RelayLocatorVerifier for ConstructionOnlyLocatorVerifier {
    fn verify(
        &self,
        _key_id: &LocatorKeyId,
        _canonical_signing_bytes: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        true
    }
}

struct TestX25519Boundary {
    key_id: RelayChallengeKeyId,
    secret: StaticSecret,
}

impl TestX25519Boundary {
    fn new() -> Self {
        Self {
            key_id: RelayChallengeKeyId::new("relay-pop-key").unwrap(),
            secret: StaticSecret::from(SERVER_SECRET),
        }
    }
}

impl RelayServerX25519ProofBoundary for TestX25519Boundary {
    fn active_key_id(&self) -> Result<RelayChallengeKeyId, RelayX25519BoundaryError> {
        Ok(self.key_id.clone())
    }

    fn verify(
        &self,
        challenge: &RelayServerChallengeV1,
        caller_public_key: &CallerDeviceDhPublic,
        bearer: &[u8],
        hello: &RelayClientHello,
        proof: &RelayChallengeProofV2,
    ) -> Result<bool, RelayX25519BoundaryError> {
        let shared = self
            .secret
            .diffie_hellman(&PublicKey::from(*caller_public_key.as_bytes()));
        if !shared.was_contributory() {
            return Err(RelayX25519BoundaryError::NonContributoryPeer);
        }
        let shared = Zeroizing::new(shared.to_bytes());
        Ok(proof.verify(&shared, challenge, bearer, hello).is_ok())
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock follows Unix epoch")
        .as_secs()
}

fn client_public(secret: [u8; 32]) -> [u8; 32] {
    PublicKey::from(&StaticSecret::from(secret)).to_bytes()
}

fn server_public() -> [u8; 32] {
    PublicKey::from(&StaticSecret::from(SERVER_SECRET)).to_bytes()
}

fn ticket_verifier(issuer: &TestCollabIssuer) -> CollabTicketVerifier<StaticTestJwksFetcher> {
    CollabTicketVerifier::new(
        TestCollabIssuer::verifier_config().expect("test verifier config"),
        StaticTestJwksFetcher::new(issuer.jwks_json().expect("test JWKS"), 300),
        CollabJwksCacheLimits::default(),
    )
    .expect("test ticket verifier")
}

fn issue_ticket(
    issuer: &TestCollabIssuer,
    caller_dh: [u8; 32],
    subject: &str,
    ticket_id: &str,
    now: u64,
) -> OpaqueCollabTicket {
    let mut spec = TestCollabTicketSpec::valid_at(now, caller_dh);
    spec.subject = subject.to_owned();
    spec.ticket_id = ticket_id.to_owned();
    issuer.issue(&spec).expect("signed test ticket")
}

fn credential(ticket: &OpaqueCollabTicket) -> RelayBearerCredential {
    RelayBearerCredential::new(ticket.expose().to_vec())
}

fn route(
    home_region: RelayRegion,
    signature_byte: u8,
    capability_byte: u8,
    now: u64,
) -> VerifiedRelayRoute {
    let unsigned = UnsignedRelayLocatorV1::new(
        home_region,
        RouteId::new([0x41; 16]).expect("route id"),
        NonZeroU64::new(1).expect("generation"),
        OwnerNoiseStatic::new(client_public(CLIENT_A_SECRET)).expect("owner static"),
        ExpectedDiscoveryId::new("production-auth-test").expect("discovery id"),
        now.saturating_sub(1),
        now + 600,
        LocatorKeyId::new("test-locator-key").expect("locator key id"),
    )
    .expect("locator claims");
    let locator = unsigned.attach_signature(
        LocatorSignature::new([signature_byte; 64]).expect("test locator signature"),
    );
    let verified_locator = locator
        .verify(&ConstructionOnlyLocatorVerifier, now)
        .expect("test-only locator construction");
    VerifiedRelayRoute::new(
        verified_locator,
        RouteCapability::new([capability_byte; 32]).expect("route capability"),
    )
}

fn v1_hello(role: RelayRole, caller_dh: [u8; 32], route: &VerifiedRelayRoute) -> RelayClientHello {
    RelayClientHello::new(
        role,
        route,
        RelayAuthExtensionV1::without_possession_proof(
            CallerDeviceDhPublic::new(caller_dh).expect("caller DH"),
        ),
    )
}

fn v2_hello(
    role: RelayRole,
    client_secret: [u8; 32],
    bearer: &[u8],
    challenge: &RelayServerChallengeV1,
    route: &VerifiedRelayRoute,
) -> RelayClientHello {
    let caller = CallerDeviceDhPublic::new(client_public(client_secret)).expect("caller DH");
    let template = RelayClientHello::new_challenge_bound_v2(
        role,
        route,
        RelayAuthExtensionV1::without_possession_proof(caller),
    )
    .expect("V2 proof template");
    let shared =
        StaticSecret::from(client_secret).diffie_hellman(&PublicKey::from(server_public()));
    assert!(shared.was_contributory());
    let shared = Zeroizing::new(shared.to_bytes());
    let proof = RelayChallengeProofV2::derive(&shared, challenge, bearer, &template)
        .expect("challenge proof");
    RelayClientHello::new_challenge_bound_v2(
        role,
        route,
        RelayAuthExtensionV1::new(caller, Some(proof.as_bytes().to_vec()))
            .expect("proof extension"),
    )
    .expect("V2 hello")
}

fn challenge(
    authenticator: &impl RelayAuthenticator,
) -> (RelayUpgradeChallenge, RelayServerChallengeV1) {
    let key_id = authenticator
        .challenge_key_id()
        .expect("challenge key")
        .expect("strict policy emits challenge");
    let state = RelayUpgradeChallenge::generate(key_id, Instant::now() + Duration::from_secs(5))
        .expect("fresh challenge");
    let public = state.challenge().clone();
    (state, public)
}

fn full_authenticator(
    issuer: &TestCollabIssuer,
) -> CollabTicketRelayAuthenticator<
    StaticTestJwksFetcher,
    StrictTestLocatorVerifier,
    crate::RequireRelayChallengeProof<TestX25519Boundary>,
> {
    CollabTicketRelayAuthenticator::new(
        ticket_verifier(issuer),
        RelayRegion::Cn,
        StrictTestLocatorVerifier,
        TestX25519Boundary::new(),
    )
}

#[test]
fn full_auth_requires_v2_fresh_challenge_and_exact_bearer() {
    let now = unix_now();
    let issuer = TestCollabIssuer::initial();
    let authenticator = full_authenticator(&issuer);
    let caller = client_public(CLIENT_A_SECRET);
    let ticket = issue_ticket(
        &issuer,
        caller,
        op_auth_bridge::TEST_SUBJECT,
        TICKET_ID_A,
        now,
    );
    let credential = credential(&ticket);
    let route = route(RelayRegion::Cn, 0x55, 0x43, now);
    let (challenge_state, public_challenge) = challenge(&authenticator);
    let hello = v2_hello(
        RelayRole::Owner,
        CLIENT_A_SECRET,
        ticket.expose(),
        &public_challenge,
        &route,
    );

    authenticator
        .authenticate(&hello, Some(&credential), Some(challenge_state))
        .expect("fresh challenge-bound proof authenticates");

    let v1 = v1_hello(RelayRole::Owner, caller, &route);
    assert!(matches!(
        authenticator.authenticate(&v1, Some(&credential), None),
        Err(RelayRejectCode::AuthenticationFailed)
    ));
}

#[path = "production_auth_max_ticket_tests.rs"]
mod max_ticket_tests;

#[path = "production_auth_relay_token_tests.rs"]
mod relay_token_tests;

#[test]
fn replayed_stale_wrong_bearer_and_wrong_route_proofs_fail() {
    let now = unix_now();
    let issuer = TestCollabIssuer::initial();
    let authenticator = full_authenticator(&issuer);
    let caller = client_public(CLIENT_B_SECRET);
    let first_ticket = issue_ticket(&issuer, caller, OTHER_SUBJECT, TICKET_ID_A, now);
    let second_ticket = issue_ticket(&issuer, caller, OTHER_SUBJECT, TICKET_ID_B, now);
    let first_credential = credential(&first_ticket);
    let second_credential = credential(&second_ticket);
    let original_route = route(RelayRegion::Cn, 0x55, 0x43, now);

    let (_, old_public_challenge) = challenge(&authenticator);
    let replayed_proof = v2_hello(
        RelayRole::Guest,
        CLIENT_B_SECRET,
        first_ticket.expose(),
        &old_public_challenge,
        &original_route,
    );
    let (fresh_state, _) = challenge(&authenticator);
    assert!(matches!(
        authenticator.authenticate(&replayed_proof, Some(&first_credential), Some(fresh_state),),
        Err(RelayRejectCode::AuthenticationFailed)
    ));

    let stale_key = authenticator
        .challenge_key_id()
        .unwrap()
        .expect("strict challenge key");
    let stale_state =
        RelayUpgradeChallenge::generate(stale_key, Instant::now()).expect("stale challenge state");
    let stale_public = stale_state.challenge().clone();
    let stale_hello = v2_hello(
        RelayRole::Guest,
        CLIENT_B_SECRET,
        first_ticket.expose(),
        &stale_public,
        &original_route,
    );
    assert!(matches!(
        authenticator.authenticate(&stale_hello, Some(&first_credential), Some(stale_state),),
        Err(RelayRejectCode::AuthenticationFailed)
    ));

    let (wrong_bearer_state, wrong_bearer_challenge) = challenge(&authenticator);
    let wrong_bearer_hello = v2_hello(
        RelayRole::Guest,
        CLIENT_B_SECRET,
        first_ticket.expose(),
        &wrong_bearer_challenge,
        &original_route,
    );
    assert!(matches!(
        authenticator.authenticate(
            &wrong_bearer_hello,
            Some(&second_credential),
            Some(wrong_bearer_state),
        ),
        Err(RelayRejectCode::AuthenticationFailed)
    ));

    let (wrong_route_state, wrong_route_challenge) = challenge(&authenticator);
    let bound = v2_hello(
        RelayRole::Guest,
        CLIENT_B_SECRET,
        first_ticket.expose(),
        &wrong_route_challenge,
        &original_route,
    );
    let wrong_route = route(RelayRegion::Cn, 0x55, 0x44, now);
    let wrong_route_hello = RelayClientHello::new_challenge_bound_v2(
        RelayRole::Guest,
        &wrong_route,
        bound.auth_extension().clone(),
    )
    .expect("well-formed hello with mismatched proof binding");
    assert!(matches!(
        authenticator.authenticate(
            &wrong_route_hello,
            Some(&first_credential),
            Some(wrong_route_state),
        ),
        Err(RelayRejectCode::AuthenticationFailed)
    ));
}

#[test]
fn locator_region_ticket_and_owner_bindings_remain_fail_closed() {
    let now = unix_now();
    let issuer = TestCollabIssuer::initial();
    let authenticator = full_authenticator(&issuer);
    let caller_a = client_public(CLIENT_A_SECRET);
    let ticket_a = issue_ticket(
        &issuer,
        caller_a,
        op_auth_bridge::TEST_SUBJECT,
        TICKET_ID_A,
        now,
    );
    let credential_a = credential(&ticket_a);

    for invalid_route in [
        route(RelayRegion::Global, 0x55, 0x43, now),
        route(RelayRegion::Cn, 0x54, 0x43, now),
    ] {
        let (state, public) = challenge(&authenticator);
        let hello = v2_hello(
            RelayRole::Guest,
            CLIENT_A_SECRET,
            ticket_a.expose(),
            &public,
            &invalid_route,
        );
        assert!(matches!(
            authenticator.authenticate(&hello, Some(&credential_a), Some(state)),
            Err(RelayRejectCode::AuthenticationFailed)
        ));
    }

    let route = route(RelayRegion::Cn, 0x55, 0x43, now);
    let (mismatch_state, mismatch_challenge) = challenge(&authenticator);
    let caller_b_hello = v2_hello(
        RelayRole::Guest,
        CLIENT_B_SECRET,
        ticket_a.expose(),
        &mismatch_challenge,
        &route,
    );
    assert!(matches!(
        authenticator.authenticate(&caller_b_hello, Some(&credential_a), Some(mismatch_state),),
        Err(RelayRejectCode::AuthenticationFailed)
    ));

    let caller_b = client_public(CLIENT_B_SECRET);
    let ticket_b = issue_ticket(&issuer, caller_b, OTHER_SUBJECT, TICKET_ID_B, now);
    let credential_b = credential(&ticket_b);
    let (owner_state, owner_challenge) = challenge(&authenticator);
    let owner_b = v2_hello(
        RelayRole::Owner,
        CLIENT_B_SECRET,
        ticket_b.expose(),
        &owner_challenge,
        &route,
    );
    assert!(matches!(
        authenticator.authenticate(&owner_b, Some(&credential_b), Some(owner_state)),
        Err(RelayRejectCode::AuthenticationFailed)
    ));
}

#[test]
fn different_users_still_share_the_invite_route_key() {
    let now = unix_now();
    let issuer = TestCollabIssuer::initial();
    let authenticator = full_authenticator(&issuer);
    let owner_ticket = issue_ticket(
        &issuer,
        client_public(CLIENT_A_SECRET),
        op_auth_bridge::TEST_SUBJECT,
        TICKET_ID_A,
        now,
    );
    let guest_ticket = issue_ticket(
        &issuer,
        client_public(CLIENT_B_SECRET),
        OTHER_SUBJECT,
        TICKET_ID_B,
        now,
    );
    let owner_credential = credential(&owner_ticket);
    let guest_credential = credential(&guest_ticket);
    let route = route(RelayRegion::Cn, 0x55, 0x43, now);
    let (owner_state, owner_challenge) = challenge(&authenticator);
    let (guest_state, guest_challenge) = challenge(&authenticator);
    let owner = authenticator
        .authenticate(
            &v2_hello(
                RelayRole::Owner,
                CLIENT_A_SECRET,
                owner_ticket.expose(),
                &owner_challenge,
                &route,
            ),
            Some(&owner_credential),
            Some(owner_state),
        )
        .expect("owner authenticates");
    let guest = authenticator
        .authenticate(
            &v2_hello(
                RelayRole::Guest,
                CLIENT_B_SECRET,
                guest_ticket.expose(),
                &guest_challenge,
                &route,
            ),
            Some(&guest_credential),
            Some(guest_state),
        )
        .expect("guest authenticates");

    assert_eq!(owner.route, guest.route);
    assert_ne!(owner.role(), guest.role());
}

#[test]
fn explicit_ticket_binding_only_requires_v1_without_proof_or_challenge() {
    let now = unix_now();
    let issuer = TestCollabIssuer::initial();
    let authenticator = CollabTicketRelayAuthenticator::new_ticket_binding_only(
        ticket_verifier(&issuer),
        RelayRegion::Cn,
        StrictTestLocatorVerifier,
    );
    let caller = client_public(CLIENT_A_SECRET);
    let ticket = issue_ticket(
        &issuer,
        caller,
        op_auth_bridge::TEST_SUBJECT,
        TICKET_ID_A,
        now,
    );
    let credential = credential(&ticket);
    let route = route(RelayRegion::Cn, 0x55, 0x43, now);
    authenticator
        .authenticate(
            &v1_hello(RelayRole::Owner, caller, &route),
            Some(&credential),
            None,
        )
        .expect("explicit reduced policy accepts V1 ticket-to-DH binding");
    assert_eq!(authenticator.challenge_key_id().unwrap(), None);

    let challenge =
        RelayServerChallengeV1::new(RelayChallengeKeyId::new("relay-pop-key").unwrap(), [7; 32])
            .unwrap();
    let v2 = v2_hello(
        RelayRole::Owner,
        CLIENT_A_SECRET,
        ticket.expose(),
        &challenge,
        &route,
    );
    assert_eq!(v2.auth_mode(), RelayHelloAuthMode::ChallengeBoundX25519V2);
    assert!(matches!(
        authenticator.authenticate(&v2, Some(&credential), None),
        Err(RelayRejectCode::AuthenticationFailed)
    ));
}

#[test]
fn authentication_debug_output_is_anonymous() {
    let now = unix_now();
    let issuer = TestCollabIssuer::initial();
    let authenticator = full_authenticator(&issuer);
    let ticket = issue_ticket(
        &issuer,
        client_public(CLIENT_A_SECRET),
        OTHER_SUBJECT,
        TICKET_ID_A,
        now,
    );
    let credential = credential(&ticket);
    let route = route(RelayRegion::Cn, 0x55, 0x43, now);
    let (state, public) = challenge(&authenticator);
    let route = authenticator
        .authenticate(
            &v2_hello(
                RelayRole::Owner,
                CLIENT_A_SECRET,
                ticket.expose(),
                &public,
                &route,
            ),
            Some(&credential),
            Some(state),
        )
        .expect("valid authentication");

    for debug in [
        format!("{authenticator:?}"),
        format!("{credential:?}"),
        format!("{route:?}"),
        format!("{public:?}"),
    ] {
        assert!(!debug.contains(OTHER_SUBJECT));
        assert!(!debug.contains(TICKET_ID_A));
        assert!(
            !debug.contains(std::str::from_utf8(ticket.expose()).expect("test ticket is ASCII"))
        );
    }
}

#[test]
fn pinned_policy_fetcher_reads_regular_files_with_a_strict_bound() {
    let fixture = TempFixture::new();
    let path = fixture.path().join("operator-secret-policy.json");
    write_public_fixture(&path, b"{\"keys\":[]}");
    let config = TestCollabIssuer::verifier_config().expect("test config");
    let fetcher = PinnedPolicyFileFetcher::new(
        &config,
        &path,
        NonZeroU64::new(60).expect("non-zero max age"),
    );

    let first = fetch(&fetcher, config.keyset_endpoint(), None, 64)
        .expect("regular bounded file is accepted");
    let etag = match first {
        CollabJwksFetchResponse::Modified { body, etag, .. } => {
            assert_eq!(body, b"{\"keys\":[]}");
            etag.expect("file fetcher returns a bounded ETag")
        }
        other => panic!("expected modified response, got {other:?}"),
    };
    assert!(matches!(
        fetch(&fetcher, config.keyset_endpoint(), Some(&etag), 64),
        Ok(CollabJwksFetchResponse::NotModified { .. })
    ));
    assert!(matches!(
        fetch(&fetcher, config.keyset_endpoint(), None, 4),
        Err(CollabJwksFetchError::ResponseTooLarge)
    ));
    let debug = format!("{fetcher:?}");
    assert!(!debug.contains("operator-secret-policy"));
    assert!(!debug.contains(config.keyset_endpoint()));
}

#[cfg(unix)]
#[test]
fn pinned_policy_fetcher_rejects_a_symlink() {
    use std::os::unix::fs::symlink;

    let fixture = TempFixture::new();
    let target = fixture.path().join("policy.json");
    let link = fixture.path().join("policy-link.json");
    fs::write(&target, b"{\"keys\":[]}").expect("write target");
    symlink(&target, &link).expect("create symlink");
    let config = TestCollabIssuer::verifier_config().expect("test config");
    let fetcher = PinnedPolicyFileFetcher::new(
        &config,
        link,
        NonZeroU64::new(60).expect("non-zero max age"),
    );

    assert!(matches!(
        fetch(&fetcher, config.keyset_endpoint(), None, 64),
        Err(CollabJwksFetchError::RejectedResponse)
    ));
}

#[test]
fn pinned_locator_and_x25519_key_files_verify_matching_material() {
    let fixture = TempFixture::new();
    let signing_key = SigningKey::from_bytes(&[0x71; 32]);
    let locator_file =
        write_locator_key_file(&fixture, "locator-keys.json", "locator-key", &signing_key);
    let x25519_file = write_x25519_key_file(&fixture, "relay-x25519-keys.json");
    let locator =
        PinnedEd25519LocatorVerifier::from_file(&locator_file).expect("pinned locator keys");
    let boundary = PinnedX25519ProofBoundary::from_file(&x25519_file).expect("sealed X25519 keys");

    let locator_message = b"signed locator canonical bytes";
    let locator_signature = signing_key.sign(locator_message).to_bytes();
    let key_id = LocatorKeyId::new("locator-key").expect("locator key id");
    assert!(locator.verify(&key_id, locator_message, &locator_signature));
    assert!(!locator.verify(&key_id, b"other locator", &locator_signature));
    assert_eq!(boundary.active_key_id().unwrap().as_str(), "relay-pop-key");

    for debug in [format!("{locator:?}"), format!("{boundary:?}")] {
        assert!(!debug.contains("locator-key"));
        assert!(!debug.contains("relay-pop-key"));
        assert!(!debug.contains(&URL_SAFE_NO_PAD.encode(SERVER_SECRET)));
    }
}

#[cfg(unix)]
#[test]
fn x25519_key_file_rejects_broad_permissions() {
    use std::os::unix::fs::PermissionsExt as _;

    let fixture = TempFixture::new();
    let path = write_x25519_key_file(&fixture, "unsafe-x25519.json");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(PinnedX25519ProofBoundary::from_file(path).is_err());
}

fn write_locator_key_file(
    fixture: &TempFixture,
    filename: &str,
    key_id: &str,
    signing_key: &SigningKey,
) -> PathBuf {
    let path = fixture.path().join(filename);
    let body = serde_json::json!({
        "version": 1,
        "keys": [{
            "kid": key_id,
            "public_key_ed25519": URL_SAFE_NO_PAD.encode(signing_key.verifying_key().to_bytes()),
        }],
    });
    write_public_fixture(
        &path,
        &serde_json::to_vec(&body).expect("serialize key file"),
    );
    path
}

fn write_public_fixture(path: &Path, body: &[u8]) {
    fs::write(path, body).expect("write public fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(path, fs::Permissions::from_mode(0o644))
            .expect("public fixture permissions");
    }
}

fn write_x25519_key_file(fixture: &TempFixture, filename: &str) -> PathBuf {
    let path = fixture.path().join(filename);
    let body = serde_json::json!({
        "version": 1,
        "active_kid": "relay-pop-key",
        "keys": [{
            "kid": "relay-pop-key",
            "private_key_x25519": URL_SAFE_NO_PAD.encode(SERVER_SECRET),
            "public_key_x25519": URL_SAFE_NO_PAD.encode(server_public()),
        }],
    });
    fs::write(
        &path,
        serde_json::to_vec(&body).expect("serialize X25519 key file"),
    )
    .expect("write X25519 key file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .expect("restrict X25519 key fixture");
    }
    path
}

fn fetch(
    fetcher: &PinnedPolicyFileFetcher,
    endpoint: &str,
    etag: Option<&str>,
    maximum_body_bytes: usize,
) -> Result<CollabJwksFetchResponse, CollabJwksFetchError> {
    CollabJwksFetcher::fetch(
        fetcher,
        CollabJwksFetchRequest {
            endpoint,
            etag,
            maximum_body_bytes,
        },
    )
}

struct TempFixture {
    path: PathBuf,
}

impl TempFixture {
    fn new() -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock follows Unix epoch")
            .as_nanos();
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "op-collab-relay-auth-{}-{nonce}-{sequence}",
            std::process::id(),
        ));
        fs::create_dir(&path).expect("create isolated temporary directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
